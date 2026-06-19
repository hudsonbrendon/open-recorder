import { useCallback, useEffect, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { useEditor } from "../state/useEditor";
import { PreviewCanvas } from "./PreviewCanvas";
import { Timeline } from "./Timeline";
import { SegmentInspector } from "./SegmentInspector";
import { WebcamBubble } from "./WebcamBubble";
import * as api from "../lib/api";
import type { WebcamOverlay } from "../lib/webcam";

const DEFAULT_WEBCAM_OVERLAY: WebcamOverlay = {
  enabled: true,
  shape: "circle",
  x: 0.75,
  y: 0.75,
  size: 0.2,
  border_width: 3,
  border_color: "#ffffff",
  mirror: true,
};

export function EditorView(props: { videoPath: string; onBack: () => void }) {
  const ed = useEditor(props.videoPath);
  const [currentMs, setCurrentMs] = useState(0);
  const [progress, setProgress] = useState<number | null>(null);
  const previewContainerRef = useRef<HTMLDivElement | null>(null);
  const [previewSize, setPreviewSize] = useState({ w: 0, h: 0 });

  // Store the video element so onSeek can reach it without stale closures
  const videoRef = useRef<HTMLVideoElement | null>(null);

  // Subscribe to export progress events; clean up on unmount
  useEffect(() => {
    const un = api.onExportProgress((p) => setProgress(p));
    return () => {
      un.then((f) => f());
    };
  }, []);

  // Measure preview container size for webcam bubble geometry
  useEffect(() => {
    const el = previewContainerRef.current;
    if (!el) return;
    const obs = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setPreviewSize({ w: entry.contentRect.width, h: entry.contentRect.height });
      }
    });
    obs.observe(el);
    // Initial measure
    setPreviewSize({ w: el.clientWidth, h: el.clientHeight });
    return () => obs.disconnect();
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

  // Ensure webcam overlay is initialized when has_webcam is true
  const hasWebcam = ed.metadata?.recording.has_webcam === true;
  const ov: WebcamOverlay | undefined =
    hasWebcam ? (ed.model.webcam ?? DEFAULT_WEBCAM_OVERLAY) : undefined;

  // Initialize model.webcam if needed (only once, when has_webcam and model.webcam is absent)
  useEffect(() => {
    if (hasWebcam && ed.metadata && !ed.model.webcam) {
      ed.setModel({ ...ed.model, webcam: DEFAULT_WEBCAM_OVERLAY });
    }
  }, [hasWebcam, ed.metadata, ed.model.webcam]);

  // Derive webcam video path: replace .mp4 with .webcam.mp4
  const webcamPath = props.videoPath.replace(/\.mp4$/, ".webcam.mp4");

  function setOverlay(next: WebcamOverlay) {
    ed.setModel({ ...ed.model, webcam: next });
  }

  return (
    <div className="editor">
      <button className="btn small" onClick={props.onBack}>
        ← Voltar
      </button>
      {ed.error && <p className="error">{ed.error}</p>}

      {/* Preview container: position relative so bubble is positioned within it */}
      <div ref={previewContainerRef} style={{ position: "relative" }}>
        <PreviewCanvas
          videoPath={props.videoPath}
          model={ed.model}
          onTime={onTime}
          playRef={playRef}
        />
        {/* Webcam bubble: sibling of PreviewCanvas's inner zoom element, outside zoom transform */}
        {ov && ov.enabled && previewSize.w > 0 && (
          <WebcamBubble
            overlay={ov}
            webcamPath={webcamPath}
            previewW={previewSize.w}
            previewH={previewSize.h}
            onChange={setOverlay}
          />
        )}
      </div>

      {/* Webcam overlay controls */}
      {hasWebcam && ov && (
        <div className="webcam-controls" style={{ display: "flex", gap: 12, alignItems: "center", flexWrap: "wrap", padding: "8px 0" }}>
          <label style={{ display: "flex", alignItems: "center", gap: 4 }}>
            <input
              type="checkbox"
              checked={ov.enabled}
              onChange={(e) => setOverlay({ ...ov, enabled: e.target.checked })}
            />
            Webcam
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 4 }}>
            <input
              type="checkbox"
              checked={ov.shape === "circle"}
              onChange={(e) => setOverlay({ ...ov, shape: e.target.checked ? "circle" : "rounded" })}
            />
            Círculo
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 4 }}>
            Borda
            <input
              type="number"
              min={0}
              max={20}
              value={ov.border_width}
              style={{ width: 48 }}
              onChange={(e) => setOverlay({ ...ov, border_width: Number(e.target.value) })}
            />
            px
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 4 }}>
            Cor
            <input
              type="color"
              value={ov.border_color}
              onChange={(e) => setOverlay({ ...ov, border_color: e.target.value })}
            />
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 4 }}>
            <input
              type="checkbox"
              checked={ov.mirror}
              onChange={(e) => setOverlay({ ...ov, mirror: e.target.checked })}
            />
            Espelho
          </label>
        </div>
      )}

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
