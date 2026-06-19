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

/// macOS implementation: enumerate on-screen windows via CGWindowListCopyWindowInfo.
///
/// This API does NOT require screen-recording permission for owner name and bounds.
/// Window titles (kCGWindowName) may be absent/empty without the permission — we
/// fall back to just the owner name in that case.
#[cfg(target_os = "macos")]
pub fn list_windows() -> Result<Vec<SourceOption>, String> {
    use core::ffi::c_void;
    use core_foundation::base::TCFType;
    use core_foundation::dictionary::{CFDictionaryGetValue, CFDictionaryRef};
    use core_foundation::number::{CFNumberGetValue, kCFNumberDoubleType, kCFNumberSInt32Type};
    use core_foundation::string::{
        kCFStringEncodingUTF8, CFString, CFStringGetCString, CFStringGetCStringPtr,
        CFStringGetLength, CFStringRef,
    };
    use core_graphics::window::{
        copy_window_info, kCGNullWindowID, kCGWindowBounds, kCGWindowLayer,
        kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly, kCGWindowName,
        kCGWindowNumber, kCGWindowOwnerName,
    };

    /// Look up a value from a raw CFDictionary using a CFString key (CFStringRef).
    ///
    /// The `key_ptr` is a `CFStringRef` (an opaque pointer). CoreFoundation
    /// dictionaries use CF equality for key matching, so passing the static
    /// `kCGWindowXxx` string refs directly works correctly.
    ///
    /// Returns the value as `*const c_void`, or null if not found.
    unsafe fn dict_get(dict_ref: CFDictionaryRef, key_ptr: CFStringRef) -> *const c_void {
        CFDictionaryGetValue(dict_ref, key_ptr as *const c_void)
    }

    /// Extract an i32 from a CFNumberRef void pointer.
    unsafe fn cf_number_i32(ptr: *const c_void) -> Option<i32> {
        if ptr.is_null() {
            return None;
        }
        let mut val: i32 = 0;
        if CFNumberGetValue(
            ptr as _,
            kCFNumberSInt32Type,
            &mut val as *mut i32 as *mut c_void,
        ) {
            Some(val)
        } else {
            None
        }
    }

    /// Extract a f64 from a CFNumberRef void pointer.
    unsafe fn cf_number_f64(ptr: *const c_void) -> Option<f64> {
        if ptr.is_null() {
            return None;
        }
        let mut val: f64 = 0.0;
        if CFNumberGetValue(
            ptr as _,
            kCFNumberDoubleType,
            &mut val as *mut f64 as *mut c_void,
        ) {
            Some(val)
        } else {
            None
        }
    }

    /// Convert a CFStringRef void pointer to a Rust String.
    unsafe fn cf_string_to_rust(ptr: *const c_void) -> Option<String> {
        if ptr.is_null() {
            return None;
        }
        let str_ref: CFStringRef = ptr as _;
        // Fast path: if CFStringGetCStringPtr returns non-null, use it directly.
        let c_ptr = CFStringGetCStringPtr(str_ref, kCFStringEncodingUTF8);
        if !c_ptr.is_null() {
            let cstr = std::ffi::CStr::from_ptr(c_ptr);
            return Some(cstr.to_string_lossy().into_owned());
        }
        // Slow path: allocate a buffer.
        let len = CFStringGetLength(str_ref);
        if len == 0 {
            return Some(String::new());
        }
        let buf_size = (len as usize) * 4 + 1;
        let mut buf = vec![0u8; buf_size];
        if CFStringGetCString(
            str_ref,
            buf.as_mut_ptr() as *mut i8,
            buf_size as _,
            kCFStringEncodingUTF8,
        ) != 0 {
            let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            Some(String::from_utf8_lossy(&buf[..end]).into_owned())
        } else {
            None
        }
    }

    let options = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;
    let window_list = match copy_window_info(options, kCGNullWindowID) {
        Some(arr) => arr,
        None => return Ok(Vec::new()),
    };

    let mut out = Vec::new();

    // get_all_values() returns Vec<*const c_void>. Each element is a CFDictionaryRef
    // retained by the array (create-rule). We can use the raw pointer directly
    // without additional retain since the array stays alive for the duration of the loop.
    for raw_ptr in window_list.get_all_values() {
        if raw_ptr.is_null() {
            continue;
        }
        // Treat raw_ptr as a CFDictionaryRef.
        let dict_ref: CFDictionaryRef = raw_ptr as _;

        // --- kCGWindowLayer: keep only layer 0 (normal application windows) ---
        let layer: i32 = unsafe {
            let val = dict_get(dict_ref, kCGWindowLayer);
            cf_number_i32(val).unwrap_or(1)
        };
        if layer != 0 {
            continue;
        }

        // --- kCGWindowNumber: CGWindowID (u32) ---
        let win_id: u32 = unsafe {
            let val = dict_get(dict_ref, kCGWindowNumber);
            cf_number_i32(val).map(|i| i as u32).unwrap_or(0)
        };
        if win_id == 0 {
            continue;
        }

        // --- kCGWindowOwnerName: application name ---
        let owner: String = unsafe {
            let val = dict_get(dict_ref, kCGWindowOwnerName);
            cf_string_to_rust(val).unwrap_or_default()
        };
        if owner.is_empty() {
            continue;
        }

        // --- kCGWindowName: window title (absent/empty without screen-recording permission) ---
        let title: String = unsafe {
            let val = dict_get(dict_ref, kCGWindowName);
            cf_string_to_rust(val).unwrap_or_default()
        };

        // --- kCGWindowBounds: {X, Y, Width, Height} as a nested CFDictionary ---
        let rect: [i64; 4] = unsafe {
            let bounds_ptr = dict_get(dict_ref, kCGWindowBounds);
            if bounds_ptr.is_null() {
                [0, 0, 0, 0]
            } else {
                let bounds_ref: CFDictionaryRef = bounds_ptr as _;
                // kCGWindowBounds keys are plain CFString literals: "X", "Y", "Width", "Height"
                let get_field = |name: &str| -> f64 {
                    let key = CFString::new(name);
                    let v = CFDictionaryGetValue(
                        bounds_ref,
                        key.as_concrete_TypeRef() as *const c_void,
                    );
                    cf_number_f64(v).unwrap_or(0.0)
                };
                [
                    get_field("X") as i64,
                    get_field("Y") as i64,
                    get_field("Width") as i64,
                    get_field("Height") as i64,
                ]
            }
        };

        // Filter out tiny utility/noise windows
        if rect[2] < 40 || rect[3] < 40 {
            continue;
        }

        let label = if title.is_empty() {
            owner.clone()
        } else {
            format!("{owner} \u{2014} {title}")
        };

        out.push(SourceOption {
            id: win_id.to_string(),
            name: label,
            kind: "window".into(),
            rect,
        });
    }

    Ok(out)
}

/// Non-macOS implementation: use scap to enumerate windows.
///
/// Returns `Err` when screen-recording permission has not been granted.
#[cfg(not(target_os = "macos"))]
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

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    /// Validates that CGWindowList-based window enumeration returns real windows.
    ///
    /// Run with: cargo test --ignored list_windows -- --nocapture
    #[test]
    #[ignore]
    fn test_list_windows_macos() {
        let windows = list_windows().expect("list_windows should not error on macOS");
        println!("Window count: {}", windows.len());
        for w in windows.iter().take(5) {
            println!("  id={} name={:?} rect={:?}", w.id, w.name, w.rect);
        }
        assert!(
            !windows.is_empty(),
            "Expected at least one on-screen window (e.g. Finder/Terminal)"
        );
        for w in &windows {
            assert!(!w.name.is_empty(), "Window name should not be empty");
            assert_eq!(w.kind, "window");
        }
    }
}
