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
  const [micOn, setMicOn] = useState(false);
  const [selectedMic, setSelectedMic] = useState<string | null>(null);
  const [camOn, setCamOn] = useState(false);
  const [selectedCamera, setSelectedCamera] = useState<string | null>(null);
  const [isRecording, setRecording] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const [recordings, setRecordings] = useState<api.RecordingResult[]>([]);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<number | null>(null);
  const startedAt = useRef<number>(0);
  const micsTried = useRef(false);
  const camsTried = useRef(false);

  // Only sources are listed at startup — they need no permission (displays via
  // display-info, windows via CGWindowList). Microphone (cpal) and camera
  // (nokhwa) enumeration is deferred until the user enables that device, so the
  // app never triggers a mic/camera permission prompt just by launching.
  const refresh = useCallback(async () => {
    try {
      const s = await api.listSources();
      setDisplays(s.displays);
      setWindows(s.windows);
      setSelectedId((cur) => cur ?? s.displays[0]?.id ?? null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // Lazy device enumeration on first enable (this is where the OS permission
  // prompt appears — only when the user actually opts in to mic/camera).
  useEffect(() => {
    if (!micOn || micsTried.current) return;
    micsTried.current = true;
    api
      .listMicrophones()
      .then((m) => {
        setMics(m);
        setSelectedMic((cur) => cur ?? m[0]?.id ?? null);
      })
      .catch((e) => setError(String(e)));
  }, [micOn]);

  useEffect(() => {
    if (!camOn || camsTried.current) return;
    camsTried.current = true;
    api
      .listCameras()
      .then((c) => {
        setCameras(c);
        setSelectedCamera((cur) => cur ?? c[0]?.id ?? null);
      })
      .catch((e) => setError(String(e)));
  }, [camOn]);

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
