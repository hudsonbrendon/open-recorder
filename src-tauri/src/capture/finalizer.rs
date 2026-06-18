use std::path::Path;
use crate::model::metadata::{RecordingMetadata, RecordingInfo, SourceInfo, InputEvent};
use crate::model::source::{CaptureSource, SourceKind};

pub fn build_metadata(
    source: &CaptureSource,
    fps: u32,
    duration_ms: u64,
    events: Vec<InputEvent>,
) -> RecordingMetadata {
    let [x, y, w, h] = source.rect;
    RecordingMetadata {
        version: 1,
        recording: RecordingInfo { width: w as u32, height: h as u32, fps, duration_ms },
        source: SourceInfo { kind: source.kind.as_str().to_string(), id: source.id.clone(), rect: [x, y, w, h] },
        events,
    }
}

pub fn write_metadata(meta: &RecordingMetadata, path: &Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(meta).expect("serialize metadata");
    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_metadata_from_source() {
        let src = CaptureSource { kind: SourceKind::Display, id: "1".into(), rect: [0, 0, 1920, 1080] };
        let meta = build_metadata(&src, 30, 5000, vec![]);
        assert_eq!(meta.version, 1);
        assert_eq!(meta.recording, RecordingInfo { width: 1920, height: 1080, fps: 30, duration_ms: 5000 });
        assert_eq!(meta.source, SourceInfo { kind: "display".into(), id: "1".into(), rect: [0, 0, 1920, 1080] });
    }

    #[test]
    fn writes_json_file_round_trip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("meta-{}.json", std::process::id()));
        let src = CaptureSource { kind: SourceKind::Window, id: "7".into(), rect: [5, 6, 100, 200] };
        let meta = build_metadata(&src, 60, 1234, vec![
            InputEvent { t_ms: 10, kind: "click".into(), x: 1, y: 2, button: Some("left".into()) },
        ]);
        write_metadata(&meta, &path).unwrap();
        let txt = std::fs::read_to_string(&path).unwrap();
        let back: RecordingMetadata = serde_json::from_str(&txt).unwrap();
        assert_eq!(back, meta);
        let _ = std::fs::remove_file(&path);
    }
}
