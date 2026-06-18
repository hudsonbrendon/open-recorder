use crate::model::metadata::InputEvent;
use crate::model::zoom::{ZoomModel, ZoomSegment, ZoomTarget};

pub struct GenOpts {
    pub scale: f64,
    pub ease_in_ms: u64,
    pub hold_ms: u64,
    pub ease_out_ms: u64,
}

impl Default for GenOpts {
    fn default() -> Self {
        GenOpts { scale: 2.0, ease_in_ms: 300, hold_ms: 1500, ease_out_ms: 400 }
    }
}

pub fn generate(events: &[InputEvent], source_rect: [i64; 4], opts: &GenOpts) -> ZoomModel {
    let w = source_rect[2].max(1) as f64;
    let h = source_rect[3].max(1) as f64;
    let merge_window = opts.hold_ms + opts.ease_out_ms;

    let mut clicks: Vec<&InputEvent> = events.iter().filter(|e| e.kind == "click").collect();
    clicks.sort_by_key(|e| e.t_ms);

    let mut segments: Vec<ZoomSegment> = Vec::new();
    let mut last_click_t: Option<u64> = None;

    for ev in clicks {
        let nx = (ev.x as f64 / w).clamp(0.0, 1.0);
        let ny = (ev.y as f64 / h).clamp(0.0, 1.0);
        let target = ZoomTarget { t_ms: ev.t_ms, x: nx, y: ny };

        let merge = matches!(last_click_t, Some(lt) if ev.t_ms <= lt + merge_window);
        if merge {
            let seg = segments.last_mut().unwrap();
            seg.targets.push(target);
            seg.end_ms = ev.t_ms + opts.hold_ms + opts.ease_out_ms;
        } else {
            segments.push(ZoomSegment {
                start_ms: ev.t_ms,
                end_ms: ev.t_ms + opts.hold_ms + opts.ease_out_ms,
                ease_in_ms: opts.ease_in_ms,
                ease_out_ms: opts.ease_out_ms,
                scale: opts.scale,
                targets: vec![target],
            });
        }
        last_click_t = Some(ev.t_ms);
    }

    ZoomModel { version: 1, segments, webcam: None }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn click(t: u64, x: i64, y: i64) -> InputEvent {
        InputEvent { t_ms: t, kind: "click".into(), x, y, button: Some("left".into()) }
    }

    #[test]
    fn single_click_one_segment() {
        let evs = vec![click(500, 250, 750)];
        let m = generate(&evs, [0, 0, 1000, 1000], &GenOpts::default());
        assert_eq!(m.segments.len(), 1);
        let s = &m.segments[0];
        assert_eq!(s.start_ms, 500);
        assert_eq!(s.end_ms, 500 + 1500 + 400);
        assert_eq!(s.targets.len(), 1);
        assert!((s.targets[0].x - 0.25).abs() < 1e-9);
        assert!((s.targets[0].y - 0.75).abs() < 1e-9);
    }

    #[test]
    fn nearby_clicks_merge() {
        let evs = vec![click(500, 250, 750), click(1000, 500, 500)];
        let m = generate(&evs, [0, 0, 1000, 1000], &GenOpts::default());
        assert_eq!(m.segments.len(), 1);
        let s = &m.segments[0];
        assert_eq!(s.start_ms, 500);
        assert_eq!(s.end_ms, 1000 + 1500 + 400);
        assert_eq!(s.targets.len(), 2);
    }

    #[test]
    fn distant_clicks_separate() {
        let evs = vec![click(500, 250, 750), click(5000, 500, 500)];
        let m = generate(&evs, [0, 0, 1000, 1000], &GenOpts::default());
        assert_eq!(m.segments.len(), 2);
    }

    #[test]
    fn moves_ignored() {
        let mut evs = vec![InputEvent { t_ms: 100, kind: "move".into(), x: 1, y: 2, button: None }];
        evs.push(click(500, 250, 750));
        let m = generate(&evs, [0, 0, 1000, 1000], &GenOpts::default());
        assert_eq!(m.segments.len(), 1);
        assert_eq!(m.segments[0].start_ms, 500);
    }
}
