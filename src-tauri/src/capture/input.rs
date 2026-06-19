/// Cross-platform input listener.
///
/// On macOS: uses a CGEventTap (mouse-only; no keyboard → no crash on macOS 26).
/// On other platforms: uses rdev (full input).
///
/// `InputListener` is spawned once per process at the first `start()` call and
/// kept alive between recordings. Event ingestion is gated by an AtomicBool so
/// the background thread is idle when not recording.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::capture::input_recorder::InputRecorder;

/// Milliseconds since UNIX_EPOCH, for use from both input.rs and input_mac.rs.
pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// (x, y, kind, button, now_ms)
pub type InputMsg = (i64, i64, String, Option<String>, u64);

pub struct InputListener {
    recording: Arc<AtomicBool>,
    rx: Receiver<InputMsg>,
}

impl InputListener {
    /// Spawn the native listener thread (once per process).
    /// Returns immediately; the thread blocks on its own run-loop.
    pub fn start() -> InputListener {
        let recording = Arc::new(AtomicBool::new(false));
        let (tx, rx): (Sender<InputMsg>, Receiver<InputMsg>) = mpsc::channel();

        #[cfg(target_os = "macos")]
        crate::capture::input_mac::spawn(recording.clone(), tx);

        #[cfg(not(target_os = "macos"))]
        spawn_rdev(recording.clone(), tx);

        InputListener { recording, rx }
    }

    /// Enable or disable event ingestion.
    /// When enabling, stale events buffered while idle are discarded.
    pub fn set_recording(&self, on: bool) {
        self.recording.store(on, Ordering::Relaxed);
        if on {
            // Drain stale events accumulated while recording was off.
            while self.rx.try_recv().is_ok() {}
        }
    }

    /// Forward all buffered events to the InputRecorder.
    pub fn drain(&self, rec: &mut InputRecorder) {
        while let Ok((x, y, kind, button, now_ms)) = self.rx.try_recv() {
            rec.ingest(x, y, &kind, button, now_ms);
        }
    }
}

/// rdev fallback for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
fn spawn_rdev(recording: Arc<AtomicBool>, tx: Sender<InputMsg>) {
    std::thread::spawn(move || {
        let _ = rdev::listen(move |event: rdev::Event| {
            if !recording.load(Ordering::Relaxed) {
                return;
            }
            let now = now_ms();
            match event.event_type {
                rdev::EventType::MouseMove { x, y } => {
                    let _ = tx.send((x as i64, y as i64, "move".into(), None, now));
                }
                rdev::EventType::ButtonPress(btn) => {
                    let label = match btn {
                        rdev::Button::Left => "left",
                        rdev::Button::Right => "right",
                        rdev::Button::Middle => "middle",
                        rdev::Button::Unknown(_) => "unknown",
                    };
                    // rdev ButtonPress carries no coordinates; (0,0) is a placeholder.
                    let _ = tx.send((0, 0, "click".into(), Some(label.into()), now));
                }
                _ => {}
            }
        });
    });
}
