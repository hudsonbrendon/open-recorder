import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "../lib/api";

export type SourceKind = "display" | "window";

export function useRecorder() {
  const [displays, setDisplays] = useState<api.SourceOption[]>([]);
  const [windows, setWindows] = useState<api.SourceOption[]>([]);
  const [mics, setMics] = useState<api.MicOption[]>([]);
  const [cameras, setCameras] = useState<api.CameraOption[]>([]);
  const [sourceKind, setSourceKindState] = useState<SourceKind>("display");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [micOn, setMicOn] = useState(true);
  const [selectedMic, setSelectedMic] = useState<string | null>(null);
  const [camOn, setCamOn] = useState(false);
  const [selectedCamera, setSelectedCamera] = useState<string | null>(null);
  const [isRecording, setRecording] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const [recordings, setRecordings] = useState<api.RecordingResult[]>([]);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<number | null>(null);
  const startedAt = useRef<number>(0);

  const refresh = useCallback(async () => {
    try {
      const s = await api.listSources();
      setDisplays(s.displays);
      setWindows(s.windows);
      const m = await api.listMicrophones();
      setMics(m);
      const c = await api.listCameras();
      setCameras(c);
      setSelectedId((cur) => cur ?? s.displays[0]?.id ?? null);
      setSelectedMic((cur) => cur ?? m[0]?.id ?? null);
      setSelectedCamera((cur) => cur ?? c[0]?.id ?? null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(
    () => () => {
      if (timer.current) clearInterval(timer.current);
    },
    [],
  );

  const sources = sourceKind === "display" ? displays : windows;
  const selected = sources.find((x) => x.id === selectedId) ?? null;

  const setSourceKind = useCallback(
    (kind: SourceKind) => {
      setSourceKindState(kind);
      const list = kind === "display" ? displays : windows;
      setSelectedId(list[0]?.id ?? null);
    },
    [displays, windows],
  );

  const start = useCallback(async () => {
    if (!selected) return;
    try {
      await api.startRecording(
        selected,
        micOn ? selectedMic : null,
        camOn ? selectedCamera : null,
      );
      setRecording(true);
      startedAt.current = Date.now();
      timer.current = window.setInterval(
        () => setElapsed(Date.now() - startedAt.current),
        100,
      );
    } catch (e) {
      setError(String(e));
    }
  }, [selected, micOn, selectedMic, camOn, selectedCamera]);

  const stop = useCallback(async () => {
    try {
      const res = await api.stopRecording();
      setRecordings((r) => [res, ...r]);
    } catch (e) {
      setError(String(e));
    }
    setRecording(false);
    if (timer.current) {
      clearInterval(timer.current);
      timer.current = null;
    }
    setElapsed(0);
  }, []);

  return {
    displays,
    windows,
    mics,
    cameras,
    sourceKind,
    setSourceKind,
    sources,
    selectedId,
    setSelectedId,
    selected,
    micOn,
    setMicOn,
    selectedMic,
    setSelectedMic,
    camOn,
    setCamOn,
    selectedCamera,
    setSelectedCamera,
    isRecording,
    elapsed,
    recordings,
    error,
    start,
    stop,
  };
}
