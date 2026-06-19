use crate::model::source::{CaptureSource, SourceKind};
use display_info::DisplayInfo;

/// A serialisable / deserialisable description of a capturable source that can
/// be sent to (and received from) the frontend.
///
/// `rect` is `[x, y, width, height]` in logical pixels.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SourceOption {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub rect: [i64; 4],
}

/// Return all capturable displays with real geometry via the `display-info` crate.
///
/// Uses `DisplayInfo::all()` which exposes: `id: u32`, `name: String`,
/// `x: i32`, `y: i32`, `width: u32`, `height: u32`, `scale_factor: f32`,
/// `is_primary: bool`, and others.
pub fn list_displays() -> Result<Vec<SourceOption>, String> {
    let displays =
        DisplayInfo::all().map_err(|e| format!("Failed to enumerate displays: {e}"))?;
    let mut out = Vec::new();
    for di in displays {
        out.push(SourceOption {
            id: di.id.to_string(),
            name: format!("Display {} ({}x{})", di.id, di.width, di.height),
            kind: "display".into(),
            rect: [di.x as i64, di.y as i64, di.width as i64, di.height as i64],
        });
    }
    Ok(out)
}

/// Return all capturable windows (those that have a title).
///
/// Returns `Err` when screen-recording permission has not been granted.
/// Uses `scap::get_all_targets()` and filters `Target::Window` variants.
/// Real scap 0.0.8 `Window` has fields `id: u32` and `title: String`.
///
/// Note: window geometry is not available in scap 0.0.8 (`get_target_dimensions`
/// is private). `rect` is set to `[0, 0, 0, 0]`; windows fall back to the
/// primary-display size at capture time (handled by the capture pipeline).
pub fn list_windows() -> Result<Vec<SourceOption>, String> {
    if !scap::has_permission() {
        return Err("Screen recording permission not granted".into());
    }
    let targets = scap::get_all_targets();
    let mut out = Vec::new();
    for t in targets {
        if let scap::Target::Window(w) = t {
            out.push(SourceOption {
                id: w.id.to_string(),
                name: w.title.clone(),
                kind: "window".into(),
                // scap 0.0.8 does not expose window dimensions; falls back to
                // primary-display size at capture time.
                rect: [0, 0, 0, 0],
            });
        }
    }
    Ok(out)
}

/// Convert a [`SourceOption`] (from the frontend) into a [`CaptureSource`]
/// (used internally by the capture pipeline).
pub fn to_capture_source(opt: &SourceOption) -> CaptureSource {
    let kind = match opt.kind.as_str() {
        "window" => SourceKind::Window,
        "region" => SourceKind::Region,
        _ => SourceKind::Display,
    };
    CaptureSource {
        kind,
        id: opt.id.clone(),
        rect: opt.rect,
    }
}
