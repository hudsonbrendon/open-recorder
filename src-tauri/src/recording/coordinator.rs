// RecordingCoordinator: wires VideoCapture, AudioCapture, InputRecorder, ffmpeg mux,
// and a native input listener into a single start/stop lifecycle.
//
// Dims-override note: VideoCapture::dimensions() returns the REAL encoded
// frame size (queried from scap at start time). For window sources source.rect
// is [0,0,0,0], so we MUST use VideoCapture::dimensions() for metadata.width/height.
// After build_metadata() we overwrite recording.width and recording.height with
// the stashed dims before writing the file.
//
// Input capture: on macOS we use a CGEventTap (mouse-only; avoids the crash caused
// by rdev calling main-thread-only Text Services APIs from a background thread).
// On other platforms rdev is used. The listener is spawned once per process the
// first time start() is called and kept alive between recordings, gated by an
// AtomicBool flag.

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::capture::audio_capture::AudioCapture;
use crate::capture::video_capture::VideoCapture;
use crate::capture::{ffmpeg, finalizer};
use crate::capture::input_recorder::InputRecorder;
use crate::capture::input::InputListener;
use crate::model::source::CaptureSource;

#[derive(serde::Serialize, Clone, PartialEq, Debug)]
pub struct RecordingResult {
    pub video_path: String,
    pub metadata_path: String,
    pub duration_ms: u64,
}

struct Active {
    source: CaptureSource,
    fps: u32,
    start_ms: u64,
    video_tmp: PathBuf,
    audio_tmp: PathBuf,
    out_video: PathBuf,
    out_meta: PathBuf,
    has_audio: bool,
    video: Option<VideoCapture>,
    audio: Option<AudioCapture>,
    /// Real encoded dimensions from VideoCapture::dimensions().
    video_dims: (u32, u32),
    input: InputRecorder,
}

#[derive(Default)]
pub struct Coordinator {
    active: Option<Active>,
    /// Native input listener — spawned once per process on first start().
    input: Option<InputListener>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn timestamp() -> String {
    // Simple epoch-seconds timestamp, e.g. "1750000000".
    // Using epoch avoids chrono dependency; callers can re-format if needed.
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{secs}")
}

/// Output directory: ~/Movies/OpenRecorder on macOS, ~/Videos/OpenRecorder elsewhere.
fn output_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    #[cfg(target_os = "macos")]
    let base = home.join("Movies");
    #[cfg(not(target_os = "macos"))]
    let base = home.join("Videos");
    let dir = base.join("OpenRecorder");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

impl Coordinator {
    pub fn start(
        &mut self,
        source: CaptureSource,
        mic_id: Option<String>,
        fps: u32,
    ) -> Result<(), String> {
        ffmpeg::ensure_ffmpeg()?;
        if self.active.is_some() {
            return Err("recording already in progress".into());
        }

        // Screen-recording permission gate. On macOS this triggers the system
        // TCC prompt (and registers the app in the Screen Recording list) the
        // first time it is missing, so the user can grant it and relaunch.
        if !scap::has_permission() {
            scap::request_permission();
            return Err("Permissão de Gravação de Tela necessária. Abra Ajustes do Sistema → Privacidade e Segurança → Gravação de Tela, autorize o OpenRecorder e reabra o app.".into());
        }

        let ts = timestamp();
        let (vname, mname) = make_filenames(&ts);
        let dir = output_dir();
        let out_video = dir.join(&vname);
        let out_meta = dir.join(&mname);
        let video_tmp = dir.join(format!("{ts}.video.mp4"));
        let audio_tmp = dir.join(format!("{ts}.audio.wav"));

        let start_ms = now_ms();

        // Start video capture first so we can read real dimensions.
        let video = VideoCapture::start(&source, fps, &video_tmp)?;
        let video_dims = video.dimensions();

        let has_audio = mic_id.is_some();
        let audio = if has_audio {
            Some(AudioCapture::start(mic_id, &audio_tmp)?)
        } else {
            None
        };

        let input = InputRecorder::new(source.rect, start_ms);

        // Native input capture (mouse-only on macOS via CGEventTap; rdev elsewhere).
        // The listener is spawned once per process and reused across recordings.
        if self.input.is_none() {
            self.input = Some(InputListener::start());
        }
        if let Some(ref l) = self.input {
            l.set_recording(true);
        }

        self.active = Some(Active {
            source,
            fps,
            start_ms,
            video_tmp,
            audio_tmp,
            out_video,
            out_meta,
            has_audio,
            video: Some(video),
            audio,
            video_dims,
            input,
        });
        Ok(())
    }

    pub fn stop(&mut self) -> Result<RecordingResult, String> {
        let mut a = self.active.take().ok_or("no active recording")?;
        let duration_ms = now_ms().saturating_sub(a.start_ms);

        // Capture temp paths up front so we can always clean them up on error.
        let video_tmp = a.video_tmp.clone();
        let audio_tmp = a.audio_tmp.clone();

        // Helper: remove both temp files, ignoring errors (best-effort cleanup).
        let cleanup_temps = |vtmp: &PathBuf, atmp: &PathBuf| {
            let _ = std::fs::remove_file(vtmp);
            let _ = std::fs::remove_file(atmp);
        };

        // Disable input ingestion and drain remaining messages.
        if let Some(ref l) = self.input {
            l.set_recording(false);
            l.drain(&mut a.input);
        }

        // Stop captures; clean up temp files if either fails.
        if let Some(v) = a.video.take() {
            if let Err(e) = v.stop() {
                cleanup_temps(&video_tmp, &audio_tmp);
                return Err(e);
            }
        }
        if let Some(au) = a.audio.take() {
            if let Err(e) = au.stop() {
                cleanup_temps(&video_tmp, &audio_tmp);
                return Err(e);
            }
        }

        // Mux or rename; clean up temp files on any error.
        if a.has_audio {
            let args = ffmpeg::mux_args(
                video_tmp.to_str().unwrap_or_default(),
                audio_tmp.to_str().unwrap_or_default(),
                a.out_video.to_str().unwrap_or_default(),
            );
            let output = Command::new(ffmpeg::ffmpeg_binary())
                .args(&args)
                .output()
                .map_err(|e| {
                    cleanup_temps(&video_tmp, &audio_tmp);
                    format!("mux failed: {e}")
                })?;
            if !output.status.success() {
                cleanup_temps(&video_tmp, &audio_tmp);
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!(
                    "mux falhou: ffmpeg saiu com status {}; stderr: {}",
                    output.status, stderr
                ));
            }
            let _ = std::fs::remove_file(&video_tmp);
            let _ = std::fs::remove_file(&audio_tmp);
        } else {
            std::fs::rename(&video_tmp, &a.out_video).map_err(|e| {
                cleanup_temps(&video_tmp, &audio_tmp);
                e.to_string()
            })?;
        }

        let events = a.input.take_events();

        // Build metadata from source (source.rect may be [0,0,0,0] for windows).
        // Then override width/height with the REAL encoded dimensions from VideoCapture.
        let mut meta = finalizer::build_metadata(&a.source, a.fps, duration_ms, events);
        // Override: use real frame dimensions, not source.rect which is 0 for windows.
        meta.recording.width = a.video_dims.0;
        meta.recording.height = a.video_dims.1;

        finalizer::write_metadata(&meta, &a.out_meta).map_err(|e| e.to_string())?;

        Ok(RecordingResult {
            video_path: a.out_video.to_string_lossy().into_owned(),
            metadata_path: a.out_meta.to_string_lossy().into_owned(),
            duration_ms,
        })
    }
}

/// Build the output filenames from a timestamp string.
/// Returns `(video_filename, metadata_filename)`.
pub fn make_filenames(timestamp: &str) -> (String, String) {
    (
        format!("REC-{timestamp}.mp4"),
        format!("REC-{timestamp}.metadata.json"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filenames_share_timestamp() {
        let (v, m) = make_filenames("20260618-153000");
        assert_eq!(v, "REC-20260618-153000.mp4");
        assert_eq!(m, "REC-20260618-153000.metadata.json");
    }

    #[test]
    fn coordinator_default_has_no_active() {
        let c = Coordinator::default();
        assert!(c.active.is_none());
    }

    #[test]
    fn stop_without_start_returns_error() {
        let mut c = Coordinator::default();
        assert!(c.stop().is_err());
    }
}
