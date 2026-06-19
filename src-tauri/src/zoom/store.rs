use std::path::{Path, PathBuf};
use crate::model::zoom::ZoomModel;

pub fn zoom_path(video_path: &str) -> PathBuf {
    let p = Path::new(video_path);
    // with_extension replaces only the last extension component.
    // "REC-123.mp4" -> "REC-123.zoom.json"
    p.with_extension("zoom.json")
}

pub fn webcam_path(video_path: &str) -> PathBuf {
    let p = Path::new(video_path);
    p.with_extension("webcam.mp4")
}

pub fn load(video_path: &str) -> Option<ZoomModel> {
    let path = zoom_path(video_path);
    let txt = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&txt).ok()
}

pub fn save(video_path: &str, model: &ZoomModel) -> Result<(), String> {
    let path = zoom_path(video_path);
    let json = serde_json::to_string_pretty(model).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::zoom::{ZoomSegment, ZoomTarget};

    #[test]
    fn zoom_path_swaps_extension() {
        let p = zoom_path("/x/REC-123.mp4");
        assert_eq!(p, PathBuf::from("/x/REC-123.zoom.json"));
    }

    #[test]
    fn webcam_path_swaps_suffix() {
        assert_eq!(webcam_path("/x/REC-1.mp4"), PathBuf::from("/x/REC-1.webcam.mp4"));
    }

    #[test]
    fn save_then_load_round_trip() {
        let dir = std::env::temp_dir();
        let video = dir.join(format!("REC-test-{}.mp4", std::process::id()));
        let model = ZoomModel { version: 1, segments: vec![ZoomSegment {
            start_ms: 0, end_ms: 1000, ease_in_ms: 100, ease_out_ms: 100,
            scale: 2.0, targets: vec![ZoomTarget { t_ms: 0, x: 0.5, y: 0.5 }],
        }], webcam: None};
        save(video.to_str().unwrap(), &model).unwrap();
        let loaded = load(video.to_str().unwrap()).unwrap();
        assert_eq!(loaded, model);
        let _ = std::fs::remove_file(zoom_path(video.to_str().unwrap()));
    }
}
