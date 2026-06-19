import type { ZoomModel } from "../lib/zoom";
import type { RecordingMetadata } from "../lib/api";
import { msToX, xToMs } from "../lib/timeline";

const W = 800;

export function Timeline(props: {
  model: ZoomModel;
  meta: RecordingMetadata;
  currentMs: number;
  selected: number | null;
  onSelect: (i: number) => void;
  onSeek: (ms: number) => void;
}) {
  const dur = props.meta.recording.duration_ms;

  return (
    <div
      style={{
        position: "relative",
        height: 56,
        width: W,
        background: "#1c1c1c",
        marginTop: 12,
        cursor: "pointer",
        userSelect: "none",
      }}
      onClick={(e) => {
        const rect = (e.currentTarget as HTMLDivElement).getBoundingClientRect();
        props.onSeek(xToMs(e.clientX - rect.left, dur, W));
      }}
    >
      {/* Click event markers */}
      {props.meta.events
        .filter((ev) => ev.type === "click")
        .map((ev, i) => (
          <div
            key={`c${i}`}
            style={{
              position: "absolute",
              left: msToX(ev.t_ms, dur, W),
              top: 0,
              width: 2,
              height: 10,
              background: "#888",
            }}
          />
        ))}

      {/* Zoom segment bars */}
      {props.model.segments.map((s, i) => (
        <div
          key={`s${i}`}
          onClick={(e) => {
            e.stopPropagation();
            props.onSelect(i);
          }}
          style={{
            position: "absolute",
            left: msToX(s.start_ms, dur, W),
            width: Math.max(4, msToX(s.end_ms, dur, W) - msToX(s.start_ms, dur, W)),
            top: 16,
            height: 28,
            borderRadius: 4,
            background: props.selected === i ? "#3b82f6" : "#2563eb88",
            cursor: "pointer",
          }}
        />
      ))}

      {/* Playhead */}
      <div
        style={{
          position: "absolute",
          left: msToX(props.currentMs, dur, W),
          top: 0,
          width: 1,
          height: 56,
          background: "red",
          pointerEvents: "none",
        }}
      />
    </div>
  );
}
