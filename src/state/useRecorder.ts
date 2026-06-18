import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "../lib/api";

export function useRecorder() {
  const [displays, setDisplays] = useState<api.SourceOption[]>([]);
  const [windows, setWindows] = useState<api.SourceOption[]>([]);
  const [mics, setMics] = useState<api.MicOption[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedMic, setSelectedMic] = useState<string | null>(null);
  const [isRecording, setRecording] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const [recordings, setRecordings] = useState<api.RecordingResult[]>([]);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<number | null>(null);
  const startedAt = useRef<number>(0);

  const refresh = useCallback(async () => {
    try {
      const s = await api.listSources();
      setDisplays(s.displays); setWindows(s.windows);
      const m = await api.listMicrophones();
      setMics(m);
      if (!selectedId && s.displays[0]) setSelectedId(s.displays[0].id);
      if (!selectedMic && m[0]) setSelectedMic(m[0].id);
    } catch (e) { setError(String(e)); }
  }, [selectedId, selectedMic]);

  useEffect(() => { refresh(); }, [refresh]);

  const allSources = [...displays, ...windows];
  const selected = allSources.find((x) => x.id === selectedId) ?? null;

  const start = useCallback(async () => {
    if (!selected) return;
    try {
      await api.startRecording(selected, selectedMic);
      setRecording(true);
      startedAt.current = Date.now();
      timer.current = window.setInterval(() => setElapsed(Date.now() - startedAt.current), 100);
    } catch (e) { setError(String(e)); }
  }, [selected, selectedMic]);

  const stop = useCallback(async () => {
    try {
      const res = await api.stopRecording();
      setRecordings((r) => [res, ...r]);
    } catch (e) { setError(String(e)); }
    setRecording(false);
    if (timer.current) { clearInterval(timer.current); timer.current = null; }
    setElapsed(0);
  }, []);

  return { displays, windows, mics, selectedId, setSelectedId, selectedMic,
           setSelectedMic, isRecording, elapsed, recordings, error, start, stop };
}
