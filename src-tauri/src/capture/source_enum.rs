use crate::model::source::{CaptureSource, SourceKind};

/// A serialisable / deserialisable description of a capturable source that can
/// be sent to (and received from) the frontend.
///
/// `rect` is `[x, y, width, height]` in logical pixels.  For displays we
/// cannot obtain dimensions through the public scap 0.0.8 API
/// (`get_target_dimensions` lives in the private `targets` module), so the
/// field is set to `[0, 0, 0, 0]` and can be filled in by the caller if
/// platform-specific geometry is required later.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SourceOption {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub rect: [i64; 4],
}

/// Return all capturable displays.
///
/// Returns `Err` when screen-recording permission has not been granted.
/// Uses `scap::get_all_targets()` and filters `Target::Display` variants.
/// Real scap 0.0.8 `Display` has fields `id: u32` and `title: String`
/// (no `width`/`height` — those live in the private `get_target_dimensions`
/// helper that is not re-exported from the crate root).
pub fn list_displays() -> Result<Vec<SourceOption>, String> {
    if !scap::has_permission() {
        return Err("Screen recording permission not granted".into());
    }
    let targets = scap::get_all_targets();
    let mut out = Vec::new();
    for t in targets {
        if let scap::Target::Display(d) = t {
            out.push(SourceOption {
                id: d.id.to_string(),
                name: d.title.clone(),
                kind: "display".into(),
                // scap 0.0.8 does not expose display dimensions through its
                // public API; rect is left as a zero rectangle.
                rect: [0, 0, 0, 0],
            });
        }
    }
    Ok(out)
}

/// Return all capturable windows (those that have a title).
///
/// Returns `Err` when screen-recording permission has not been granted.
/// Uses `scap::get_all_targets()` and filters `Target::Window` variants.
/// Real scap 0.0.8 `Window` has fields `id: u32` and `title: String`.
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
