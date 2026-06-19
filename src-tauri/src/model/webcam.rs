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

    pub fn geometry(&self, out_w: u32, out_h: u32) -> (u32, u32, u32) {
        let max_s = out_w.min(out_h);
        let s = ((self.size * out_w as f64).round() as i64).clamp(1, max_s as i64) as u32;
        let max_x = out_w.saturating_sub(s);
        let max_y = out_h.saturating_sub(s);
        let xpx = ((self.x * out_w as f64).round() as i64).clamp(0, max_x as i64) as u32;
        let ypx = ((self.y * out_h as f64).round() as i64).clamp(0, max_y as i64) as u32;
        (s, xpx, ypx)
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

    #[test]
    fn geometry_maps_and_clamps() {
        let mut o = WebcamOverlay::default_overlay();
        o.x = 0.5; o.y = 0.5; o.size = 0.2;
        // out 1000x500: s = 200, x = 500, y = 250
        assert_eq!(o.geometry(1000, 500), (200, 500, 250));
        // clamp: x near right edge
        o.x = 0.99;
        let (s, xpx, _) = o.geometry(1000, 500);
        assert_eq!(s, 200);
        assert_eq!(xpx, 800); // clamped to out_w - s
    }
}
