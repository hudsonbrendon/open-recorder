use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WebcamOverlay {
    pub enabled: bool,
    pub shape: String,
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub border_width: u32,
    pub border_color: String,
    pub mirror: bool,
}

impl WebcamOverlay {
    pub fn default_overlay() -> Self {
        WebcamOverlay {
            enabled: true,
            shape: "circle".into(),
            x: 0.76,
            y: 0.74,
            size: 0.22,
            border_width: 3,
            border_color: "#ffffff".into(),
            mirror: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_overlay_values() {
        let o = WebcamOverlay::default_overlay();
        assert!(o.enabled);
        assert_eq!(o.shape, "circle");
        assert!((o.x - 0.76).abs() < 1e-9 && (o.y - 0.74).abs() < 1e-9);
        assert!((o.size - 0.22).abs() < 1e-9);
        assert_eq!(o.border_width, 3);
        assert_eq!(o.border_color, "#ffffff");
        assert!(o.mirror);
    }

    #[test]
    fn round_trips() {
        let o = WebcamOverlay::default_overlay();
        let j = serde_json::to_string(&o).unwrap();
        let back: WebcamOverlay = serde_json::from_str(&j).unwrap();
        assert_eq!(o, back);
    }
}
