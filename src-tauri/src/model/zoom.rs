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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomAt {
    pub scale: f64,
    pub cx: f64,
    pub cy: f64,
}

fn target_at(seg: &ZoomSegment, t_ms: u64) -> (f64, f64) {
    let ts = &seg.targets;
    if ts.len() == 1 {
        return (ts[0].x, ts[0].y);
    }
    let first = ts[0].t_ms;
    let last = ts[ts.len() - 1].t_ms;
    let t = t_ms.clamp(first, last);
    for w in ts.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if t >= a.t_ms && t <= b.t_ms {
            let span = (b.t_ms - a.t_ms).max(1) as f64;
            let f = (t - a.t_ms) as f64 / span;
            return (a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f);
        }
    }
    (ts[ts.len() - 1].x, ts[ts.len() - 1].y)
}

pub fn zoom_at(model: &ZoomModel, t_ms: u64) -> ZoomAt {
    for seg in &model.segments {
        if t_ms < seg.start_ms || t_ms >= seg.end_ms {
            continue;
        }
        let rel = (t_ms - seg.start_ms) as f64;
        let dur = (seg.end_ms - seg.start_ms) as f64;
        let ein = seg.ease_in_ms as f64;
        let eout = seg.ease_out_ms as f64;
        let e_in = if ein > 0.0 {
            smoothstep(rel / ein)
        } else {
            1.0
        };
        let e_out = if eout > 0.0 {
            smoothstep((dur - rel) / eout)
        } else {
            1.0
        };
        let e = if rel < ein && rel > dur - eout {
            e_in.min(e_out) // overlapping ramps: take the smaller
        } else if rel < ein {
            e_in
        } else if rel > dur - eout {
            e_out
        } else {
            1.0
        };
        let scale_t = 1.0 + (seg.scale - 1.0) * e;
        let (tx, ty) = target_at(seg, t_ms);
        let m = 0.5 / scale_t;
        let cx = tx.clamp(m, 1.0 - m);
        let cy = ty.clamp(m, 1.0 - m);
        return ZoomAt {
            scale: scale_t,
            cx,
            cy,
        };
    }
    ZoomAt {
        scale: 1.0,
        cx: 0.5,
        cy: 0.5,
    }
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
                start_ms: 0,
                end_ms: 2000,
                ease_in_ms: 300,
                ease_out_ms: 400,
                scale: 2.0,
                targets: vec![ZoomTarget {
                    t_ms: 0,
                    x: 0.25,
                    y: 0.75,
                }],
            }],
        };
        let j = serde_json::to_string(&m).unwrap();
        let back: ZoomModel = serde_json::from_str(&j).unwrap();
        assert_eq!(m, back);
    }

    fn fixture() -> ZoomModel {
        ZoomModel {
            version: 1,
            segments: vec![ZoomSegment {
                start_ms: 0,
                end_ms: 2000,
                ease_in_ms: 300,
                ease_out_ms: 400,
                scale: 2.0,
                targets: vec![ZoomTarget {
                    t_ms: 0,
                    x: 0.25,
                    y: 0.75,
                }],
            }],
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn zoom_at_outside_is_identity() {
        let z = zoom_at(&fixture(), 2500);
        assert!(close(z.scale, 1.0) && close(z.cx, 0.5) && close(z.cy, 0.5));
    }

    #[test]
    fn zoom_at_ease_in_mid() {
        let z = zoom_at(&fixture(), 150); // smoothstep(0.5)=0.5 -> scale 1.5
        assert!(close(z.scale, 1.5), "{}", z.scale);
        assert!(close(z.cx, 1.0 / 3.0), "{}", z.cx); // clamp(0.25, 0.333..,0.666..)
        assert!(close(z.cy, 2.0 / 3.0), "{}", z.cy); // clamp(0.75, ...)
    }

    #[test]
    fn zoom_at_plateau_full_scale() {
        let z = zoom_at(&fixture(), 1000);
        assert!(close(z.scale, 2.0));
        assert!(close(z.cx, 0.25) && close(z.cy, 0.75)); // m=0.25
    }

    #[test]
    fn zoom_at_ease_out_mid() {
        let z = zoom_at(&fixture(), 1800); // (2000-1800)/400=0.5 -> 0.5 -> scale 1.5
        assert!(close(z.scale, 1.5));
    }
}
