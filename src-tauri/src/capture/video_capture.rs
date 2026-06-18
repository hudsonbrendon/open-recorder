// Task 9: Video capture via scap (BGRA frames) piped to ffmpeg stdin.
//
// Stride note: scap 0.0.8 BGRAFrame.data is already stride-clean.
// The mac engine's create_bgra_frame iterates row-by-row taking 4*width bytes,
// so data.len() == width * height * 4 with no padding. We can write it directly.
//
// Window target: matched by parsing source.id as u32 against scap Window.id.
// Display / Region target: uses target: None which defaults to the primary display.
// For non-primary display sources the implementation currently falls back to primary
// (scap 0.0.8 does not let us easily select a display by id from our numeric id
// without going through the internal mac engine directly).

use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread::JoinHandle;

use crate::capture::ffmpeg::{encode_args, ffmpeg_binary};
use crate::model::source::{CaptureSource, SourceKind};

pub struct VideoCapture {
    stop_tx: mpsc::Sender<()>,
    handle: Option<JoinHandle<()>>,
    ffmpeg: Child,
}

impl VideoCapture {
    /// Start capturing `source` at `fps`, encoding to `video_tmp` via ffmpeg.
    ///
    /// Uses scap BGRA frames piped to ffmpeg stdin as raw video.
    /// `source.rect[2]` = width, `source.rect[3]` = height (logical pixels).
    pub fn start(source: &CaptureSource, fps: u32, video_tmp: &Path) -> Result<Self, String> {
        let [_x, _y, w, h] = source.rect;
        let width = w as u32;
        let height = h as u32;

        let args = encode_args(width, height, fps, video_tmp.to_str().ok_or("invalid path")?);
        let mut ffmpeg = Command::new(ffmpeg_binary())
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("failed to start ffmpeg: {e}"))?;
        let mut stdin = ffmpeg.stdin.take().ok_or("ffmpeg has no stdin")?;

        // Resolve scap target: Window matched by id, Display/Region → primary (None).
        let scap_target = if source.kind == SourceKind::Window {
            if let Ok(window_id) = source.id.parse::<u32>() {
                scap::get_all_targets()
                    .into_iter()
                    .find_map(|t| match t {
                        scap::Target::Window(w) if w.id == window_id => {
                            Some(scap::Target::Window(w))
                        }
                        _ => None,
                    })
            } else {
                None
            }
        } else {
            None // primary display
        };

        let mut capturer = scap::capturer::Capturer::build(scap::capturer::Options {
            fps,
            target: scap_target,
            show_cursor: true,
            output_type: scap::frame::FrameType::BGRAFrame,
            output_resolution: scap::capturer::Resolution::Captured,
            ..Default::default()
        })
        .map_err(|e| format!("scap build error: {e}"))?;

        capturer.start_capture();

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let handle = std::thread::spawn(move || {
            loop {
                // Check for stop signal (non-blocking).
                if stop_rx.try_recv().is_ok() {
                    break;
                }
                match capturer.get_next_frame() {
                    Ok(scap::frame::Frame::BGRA(f)) => {
                        // f.data is already stride-clean: width * height * 4 bytes.
                        if stdin.write_all(&f.data).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {
                        // Unexpected frame type — skip.
                    }
                    Err(_) => break,
                }
            }
            capturer.stop_capture();
            // Dropping stdin signals EOF to ffmpeg so it can finalize the file.
            drop(stdin);
        });

        Ok(Self {
            stop_tx,
            handle: Some(handle),
            ffmpeg,
        })
    }

    /// Signal the capture thread to stop, wait for it, then wait for ffmpeg to finish.
    pub fn stop(mut self) -> Result<(), String> {
        let _ = self.stop_tx.send(());
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        self.ffmpeg
            .wait()
            .map_err(|e| format!("ffmpeg wait error: {e}"))?;
        Ok(())
    }
}
