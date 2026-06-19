use crate::model::coords::map_to_source;
use crate::model::metadata::InputEvent;

pub struct InputRecorder {
    rect: [i64; 4],
    start_ms: u64,
    events: Vec<InputEvent>,
}

impl InputRecorder {
    pub fn new(rect: [i64; 4], start_ms: u64) -> Self {
        Self { rect, start_ms, events: Vec::new() }
    }

    pub fn ingest(&mut self, x: i64, y: i64, kind: &str, button: Option<String>, now_ms: u64) {
        if let Some((rx, ry)) = map_to_source(self.rect, x, y) {
            let t_ms = now_ms.saturating_sub(self.start_ms);
            self.events.push(InputEvent {
                t_ms,
                kind: kind.to_string(),
                x: rx,
                y: ry,
                button,
            });
        }
    }

    pub fn take_events(self) -> Vec<InputEvent> {
        self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingest_stores_mapped_event_with_relative_time() {
        let mut rec = InputRecorder::new([100, 50, 800, 600], 1000);
        rec.ingest(150, 90, "click", Some("left".into()), 1200);
        let ev = rec.take_events();
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].t_ms, 200);
        assert_eq!((ev[0].x, ev[0].y), (50, 40));
        assert_eq!(ev[0].kind, "click");
        assert_eq!(ev[0].button.as_deref(), Some("left"));
    }

    #[test]
    fn ingest_drops_events_outside_source() {
        let mut rec = InputRecorder::new([0, 0, 100, 100], 0);
        rec.ingest(500, 500, "move", None, 50);
        assert_eq!(rec.take_events().len(), 0);
    }
}
