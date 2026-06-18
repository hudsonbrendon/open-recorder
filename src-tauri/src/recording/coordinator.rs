// RecordingCoordinator: wires VideoCapture, AudioCapture, InputRecorder, ffmpeg mux,
// and rdev global input listener into a single start/stop lifecycle.
//
// Dims-override note: VideoCapture::dimensions() returns the REAL encoded
// frame size (queried from scap at start time). For window sources source.rect
// is [0,0,0,0], so we MUST use VideoCapture::dimensions() for metadata.width/height.
// After build_metadata() we overwrite recording.width and recording.height with
// the stashed dims before writing the file.
//
// rdev::listen note: rdev::listen is GLOBAL and BLOCKING. It can only be called
// once per process. We spawn it once in a background thread the first time start()
// is called, and gate event ingestion via a shared AtomicBool flag. The thread
// cannot be cleanly joined (rdev does not expose a stop handle), so we leave it
// parked but idle when recording stops. Events are forwarded via a channel to an
// Arc<Mutex<Option<InputRecorder>>> that the coordinator owns.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::capture::audio_capture::AudioCapture;
use crate::capture::video_capture::VideoCapture;
use crate::capture::{ffmpeg, finalizer};
use crate::capture::input_recorder::InputRecorder;
use crate::model::source::CaptureSource;

#[derive(serde::Serialize, Clone, PartialEq, Debug)]
pub struct RecordingResult {
    pub video_path: String,
    pub metadata_path: String,
    pub duration_ms: u64,
}

/// Shared state between the rdev listener thread and the Coordinator.
/// The listener sends (x, y, kind, button, now_ms) tuples via channel.
type InputMsg = (i64, i64, String, Option<String>, u64);

struct RdevHandle {
    /// Set to true while a recording is active; the listener thread checks this.
    recording: Arc<AtomicBool>,
    /// Receiver for input events from the listener thread.
    rx: mpsc::Receiver<InputMsg>,
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
    /// rdev listener is spawned once per process; stored here after first start().
    rdev: Option<RdevHandle>,
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

/// Spawn the rdev listen thread (called at most once per process).
/// Returns an RdevHandle containing the recording flag and event receiver.
fn spawn_rdev_listener() -> RdevHandle {
    let recording = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel::<InputMsg>();

    let recording_clone = recording.clone();
    std::thread::spawn(move || {
        // rdev::listen is blocking and cannot be stopped; we gate via the flag.
        let _ = rdev::listen(move |event: rdev::Event| {
            if !recording_clone.load(Ordering::Relaxed) {
                return;
            }
            let now = now_ms();
            match event.event_type {
                rdev::EventType::MouseMove { x, y } => {
                    let _ = tx.send((x as i64, y as i64, "move".to_string(), None, now));
                }
                rdev::EventType::ButtonPress(btn) => {
                    let label = match btn {
                        rdev::Button::Left => "left",
                        rdev::Button::Right => "right",
                        rdev::Button::Middle => "middle",
                        rdev::Button::Unknown(_) => "unknown",
                    };
                    let _ = tx.send((0, 0, "click".to_string(), Some(label.to_string()), now));
                }
                _ => {}
            }
        });
    });

    RdevHandle { recording, rx }
}

/// Drain all pending messages from the rdev channel into the InputRecorder.
fn drain_rdev(handle: &RdevHandle, input: &mut InputRecorder) {
    while let Ok((x, y, kind, button, now_ms)) = handle.rx.try_recv() {
        input.ingest(x, y, &kind, button, now_ms);
    }
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

        // Spawn the rdev listener thread (only once per process).
        if self.rdev.is_none() {
            self.rdev = Some(spawn_rdev_listener());
        }
        // Enable event ingestion.
        if let Some(ref h) = self.rdev {
            h.recording.store(true, Ordering::Relaxed);
            // Drain any stale messages from a previous session.
            while h.rx.try_recv().is_ok() {}
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

        // Disable rdev ingestion and drain remaining messages.
        if let Some(ref h) = self.rdev {
            h.recording.store(false, Ordering::Relaxed);
            drain_rdev(h, &mut a.input);
        }

        // Stop captures.
        if let Some(v) = a.video.take() {
            v.stop()?;
        }
        if let Some(au) = a.audio.take() {
            au.stop()?;
        }

        // Mux or rename.
        if a.has_audio {
            let args = ffmpeg::mux_args(
                a.video_tmp.to_str().unwrap_or_default(),
                a.audio_tmp.to_str().unwrap_or_default(),
                a.out_video.to_str().unwrap_or_default(),
            );
            Command::new(ffmpeg::ffmpeg_binary())
                .args(&args)
                .output()
                .map_err(|e| format!("mux failed: {e}"))?;
            let _ = std::fs::remove_file(&a.video_tmp);
            let _ = std::fs::remove_file(&a.audio_tmp);
        } else {
            std::fs::rename(&a.video_tmp, &a.out_video).map_err(|e| e.to_string())?;
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
