use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RecordingMetadata {
    pub version: u32,
    pub recording: RecordingInfo,
    pub source: SourceInfo,
    pub events: Vec<InputEvent>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RecordingInfo {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SourceInfo {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: String,
    pub rect: [i64; 4],
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InputEvent {
    pub t_ms: u64,
    #[serde(rename = "type")]
    pub kind: String,
    pub x: i64,
    pub y: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_snake_case_with_type_field() {
        let meta = RecordingMetadata {
            version: 1,
            recording: RecordingInfo { width: 2560, height: 1440, fps: 30, duration_ms: 18450 },
            source: SourceInfo { kind: "display".into(), id: "1".into(), rect: [0, 0, 2560, 1440] },
            events: vec![
                InputEvent { t_ms: 1200, kind: "click".into(), x: 840, y: 410, button: Some("left".into()) },
                InputEvent { t_ms: 1200, kind: "move".into(), x: 840, y: 410, button: None },
            ],
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("\"duration_ms\":18450"), "{json}");
        assert!(json.contains("\"t_ms\":1200"), "{json}");
        assert!(json.contains("\"type\":\"display\""), "{json}");
        assert!(json.contains("\"type\":\"click\""), "{json}");
    }

    #[test]
    fn round_trip_preserves_values() {
        let meta = RecordingMetadata {
            version: 1,
            recording: RecordingInfo { width: 100, height: 200, fps: 60, duration_ms: 5000 },
            source: SourceInfo { kind: "window".into(), id: "abc".into(), rect: [10, 20, 30, 40] },
            events: vec![InputEvent { t_ms: 0, kind: "click".into(), x: 1, y: 2, button: Some("right".into()) }],
        };
        let json = serde_json::to_string(&meta).unwrap();
        let back: RecordingMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, back);
    }
}
