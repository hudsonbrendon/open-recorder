/// Native macOS mouse event tap — mouse-only (no keyboard → no crash on macOS 26).
///
/// Uses CGEventTap with ListenOnly so we never modify the event stream.
/// The tap runs a CFRunLoop on its own thread; events are forwarded via mpsc.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
};

use crate::capture::input::InputMsg;

pub fn spawn(recording: Arc<AtomicBool>, tx: Sender<InputMsg>) {
    std::thread::spawn(move || {
        let events = vec![
            CGEventType::LeftMouseDown,
            CGEventType::RightMouseDown,
            CGEventType::MouseMoved,
            CGEventType::LeftMouseDragged,
        ];

        let tap = CGEventTap::new(
            CGEventTapLocation::Session,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            events,
            // Callback: must NOT block. Returns None (ListenOnly — event unchanged).
            move |_proxy, etype, event| {
                if recording.load(Ordering::Relaxed) {
                    let loc = event.location();
                    let now = crate::recording::coordinator::now_ms_pub();
                    let msg: Option<InputMsg> = match etype {
                        CGEventType::LeftMouseDown => Some((
                            loc.x as i64,
                            loc.y as i64,
                            "click".into(),
                            Some("left".into()),
                            now,
                        )),
                        CGEventType::RightMouseDown => Some((
                            loc.x as i64,
                            loc.y as i64,
                            "click".into(),
                            Some("right".into()),
                            now,
                        )),
                        CGEventType::MouseMoved | CGEventType::LeftMouseDragged => {
                            Some((loc.x as i64, loc.y as i64, "move".into(), None, now))
                        }
                        _ => None,
                    };
                    if let Some(m) = msg {
                        let _ = tx.send(m);
                    }
                }
                // ListenOnly: don't modify the event.
                None
            },
        );

        match tap {
            Ok(tap) => {
                let loop_source = tap
                    .mach_port
                    .create_runloop_source(0)
                    .expect("CGEventTap: failed to create runloop source");
                let current = CFRunLoop::get_current();
                // add_source is safe in core-foundation 0.10.
                current.add_source(&loop_source, unsafe { kCFRunLoopCommonModes });
                tap.enable();
                CFRunLoop::run_current(); // blocks this thread forever
            }
            Err(()) => {
                // Tap creation failed (e.g. missing Accessibility permission).
                // Log and exit the thread silently — recording continues without input events.
                eprintln!(
                    "[open-recorder] CGEventTap::new failed — \
                     grant Accessibility access in System Settings → Privacy & Security → Accessibility"
                );
            }
        }
    });
}
