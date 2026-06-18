import { useCallback, useEffect, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { useEditor } from "../state/useEditor";
import { PreviewCanvas } from "./PreviewCanvas";
import { Timeline } from "./Timeline";
import { SegmentInspector } from "./SegmentInspector";
import * as api from "../lib/api";

export function EditorView(props: { videoPath: string; onBack: () => void }) {
  const ed = useEditor(props.videoPath);
  const [currentMs, setCurrentMs] = useState(0);
  const [progress, setProgress] = useState<number | null>(null);

  // Store the video element so onSeek can reach it without stale closures
  const videoRef = useRef<HTMLVideoElement | null>(null);

  // Subscribe to export progress events; clean up on unmount
  useEffect(() => {
    const un = api.onExportProgress((p) => setProgress(p));
    return () => {
      un.then((f) => f());
    };
  }, []);

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

  async function doExport() {
    if (!ed.metadata) return;
    const out = await save({
      defaultPath: "OpenRecorder-export.mp4",
      filters: [{ name: "MP4", extensions: ["mp4"] }],
    });
    if (!out) return;
    setProgress(0);
    try {
      await api.exportWithZoom(
        props.videoPath,
        ed.model,
        out,
        ed.metadata.recording.fps,
        ed.metadata.recording.duration_ms,
      );
    } finally {
      setProgress(null);
    }
  }

  function handleDelete(i: number) {
    ed.setModel({
      ...ed.model,
      segments: ed.model.segments.filter((_, j) => j !== i),
    });
    ed.setSelected(null);
  }

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
      <SegmentInspector
        model={ed.model}
        selected={ed.selected}
        onChange={ed.setModel}
        onDelete={handleDelete}
      />
      <div className="export-row">
        <button
          className="btn"
          onClick={doExport}
          disabled={progress !== null}
        >
          Exportar
        </button>
        {progress !== null && (
          <progress value={progress} max={1} style={{ flex: 1 }} />
        )}
      </div>
    </div>
  );
}
