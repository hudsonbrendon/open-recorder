import { useEffect, useRef } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { zoomAt, type ZoomModel } from "../lib/zoom";

export function PreviewCanvas(props: {
  videoPath: string;
  model: ZoomModel;
  onTime: (ms: number) => void;
  playRef: (v: HTMLVideoElement) => void;
}) {
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const rafRef = useRef<number | null>(null);

  useEffect(() => {
    const v = videoRef.current;
    if (!v) return;
    props.playRef(v);
    const tick = () => {
      const ms = v.currentTime * 1000;
      const z = zoomAt(props.model, ms);
      v.style.transformOrigin = `${z.cx * 100}% ${z.cy * 100}%`;
      v.style.transform = `scale(${z.scale})`;
      props.onTime(ms);
      rafRef.current = requestAnimationFrame(tick);
    };
    rafRef.current = requestAnimationFrame(tick);
    return () => {
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
    };
  }, [props.model]);

  return (
    <div
      style={{
        overflow: "hidden",
        background: "#000",
        aspectRatio: "16/9",
        width: "100%",
      }}
    >
      <video
        ref={videoRef}
        src={convertFileSrc(props.videoPath)}
        controls
        style={{
          width: "100%",
          height: "100%",
          display: "block",
          willChange: "transform",
        }}
      />
    </div>
  );
}
