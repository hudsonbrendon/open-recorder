import { useRef } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { geometry, type WebcamOverlay } from "../lib/webcam";

export function WebcamBubble(props: {
  overlay: WebcamOverlay;
  webcamPath: string;
  previewW: number;
  previewH: number;
  onChange: (ov: WebcamOverlay) => void;
}) {
  const dragging = useRef<{ dx: number; dy: number } | null>(null);
  const resizing = useRef<{ startX: number; startSize: number } | null>(null);
  const g = geometry(props.overlay, props.previewW, props.previewH);
  const radius = props.overlay.shape === "circle" ? "50%" : "16%";

  const onDown = (e: React.MouseEvent) => {
    e.preventDefault();
    dragging.current = { dx: e.clientX - g.x, dy: e.clientY - g.y };
    const move = (ev: MouseEvent) => {
      if (!dragging.current) return;
      const nx = (ev.clientX - dragging.current.dx) / props.previewW;
      const ny = (ev.clientY - dragging.current.dy) / props.previewH;
      props.onChange({
        ...props.overlay,
        x: Math.min(1, Math.max(0, nx)),
        y: Math.min(1, Math.max(0, ny)),
      });
    };
    const up = () => {
      dragging.current = null;
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", up);
    };
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
  };

  const onResizeDown = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    resizing.current = { startX: e.clientX, startSize: props.overlay.size };
    const move = (ev: MouseEvent) => {
      if (!resizing.current) return;
      const delta = (ev.clientX - resizing.current.startX) / props.previewW;
      const newSize = Math.min(1, Math.max(0.05, resizing.current.startSize + delta));
      props.onChange({ ...props.overlay, size: newSize });
    };
    const up = () => {
      resizing.current = null;
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", up);
    };
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
  };

  return (
    <div
      onMouseDown={onDown}
      style={{
        position: "absolute",
        left: g.x,
        top: g.y,
        width: g.s,
        height: g.s,
        borderRadius: radius,
        overflow: "hidden",
        cursor: "move",
        border: `${props.overlay.border_width}px solid ${props.overlay.border_color}`,
        boxSizing: "border-box",
      }}
    >
      <video
        src={convertFileSrc(props.webcamPath)}
        autoPlay
        muted
        loop
        style={{
          width: "100%",
          height: "100%",
          objectFit: "cover",
          transform: props.overlay.mirror ? "scaleX(-1)" : "none",
        }}
      />
      {/* Resize handle in bottom-right corner */}
      <div
        onMouseDown={onResizeDown}
        style={{
          position: "absolute",
          bottom: 4,
          right: 4,
          width: 12,
          height: 12,
          background: "rgba(255,255,255,0.8)",
          borderRadius: "50%",
          cursor: "se-resize",
        }}
      />
    </div>
  );
}
