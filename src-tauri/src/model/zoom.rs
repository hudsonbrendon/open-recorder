use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ZoomTarget {
    pub t_ms: u64,
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ZoomSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub ease_in_ms: u64,
    pub ease_out_ms: u64,
    pub scale: f64,
    pub targets: Vec<ZoomTarget>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ZoomModel {
    pub version: u32,
    pub segments: Vec<ZoomSegment>,
}

/// Cubic smoothstep with input clamped to [0,1].
pub fn smoothstep(p: f64) -> f64 {
    let p = p.clamp(0.0, 1.0);
    p * p * (3.0 - 2.0 * p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothstep_endpoints_and_mid() {
        assert!((smoothstep(0.0) - 0.0).abs() < 1e-9);
        assert!((smoothstep(1.0) - 1.0).abs() < 1e-9);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-9);
        assert!((smoothstep(-3.0) - 0.0).abs() < 1e-9); // clamp
        assert!((smoothstep(7.0) - 1.0).abs() < 1e-9);  // clamp
    }

    #[test]
    fn model_round_trips() {
        let m = ZoomModel {
            version: 1,
            segments: vec![ZoomSegment {
                start_ms: 0, end_ms: 2000, ease_in_ms: 300, ease_out_ms: 400,
                scale: 2.0, targets: vec![ZoomTarget { t_ms: 0, x: 0.25, y: 0.75 }],
            }],
        };
        let j = serde_json::to_string(&m).unwrap();
        let back: ZoomModel = serde_json::from_str(&j).unwrap();
        assert_eq!(m, back);
    }
}
