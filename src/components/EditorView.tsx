import { useCallback, useRef, useState } from "react";
import { useEditor } from "../state/useEditor";
import { PreviewCanvas } from "./PreviewCanvas";
import { Timeline } from "./Timeline";

export function EditorView(props: { videoPath: string; onBack: () => void }) {
  const ed = useEditor(props.videoPath);
  const [currentMs, setCurrentMs] = useState(0);

  // Store the video element so onSeek can reach it without stale closures
  const videoRef = useRef<HTMLVideoElement | null>(null);

  // Stable callback: PreviewCanvas calls this once with the <video> element
  const playRef = useCallback((v: HTMLVideoElement) => {
    videoRef.current = v;
  }, []);

  // Stable callback: called every rAF tick by PreviewCanvas
  const onTime = useCallback((ms: number) => {
    setCurrentMs(ms);
  }, []);

  // Seek via the stored video ref — no closure over currentMs or model
  const onSeek = useCallback((ms: number) => {
    const v = videoRef.current;
    if (v) v.currentTime = ms / 1000;
  }, []);

  return (
    <div className="editor">
      <button className="btn small" onClick={props.onBack}>
        ← Voltar
      </button>
      {ed.error && <p className="error">{ed.error}</p>}
      <PreviewCanvas
        videoPath={props.videoPath}
        model={ed.model}
        onTime={onTime}
        playRef={playRef}
      />
      {ed.metadata && (
        <Timeline
          model={ed.model}
          meta={ed.metadata}
          currentMs={currentMs}
          selected={ed.selected}
          onSelect={ed.setSelected}
          onSeek={onSeek}
        />
      )}
    </div>
  );
}
