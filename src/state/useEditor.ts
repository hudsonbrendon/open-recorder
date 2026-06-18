import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "../lib/api";
import type { ZoomModel } from "../lib/zoom";

export function useEditor(videoPath: string) {
  const [metadata, setMetadata] = useState<api.RecordingMetadata | null>(null);
  const [model, setModelState] = useState<ZoomModel>({ version: 1, segments: [] });
  const [selected, setSelected] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const saveTimer = useRef<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .loadRecording(videoPath)
      .then((r) => {
        if (!cancelled) {
          setMetadata(r.metadata);
          setModelState(r.zoom);
        }
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [videoPath]);

  const setModel = useCallback(
    (m: ZoomModel) => {
      setModelState(m);
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = window.setTimeout(() => {
        api.saveZoom(videoPath, m).catch((e) => setError(String(e)));
      }, 500);
    },
    [videoPath],
  );

  useEffect(() => {
    return () => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
    };
  }, []);

  return { metadata, model, setModel, selected, setSelected, error };
}
