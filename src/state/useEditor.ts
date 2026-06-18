import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "../lib/api";
import type { ZoomModel } from "../lib/zoom";

export function useEditor(videoPath: string) {
  const [metadata, setMetadata] = useState<api.RecordingMetadata | null>(null);
  const [model, setModelState] = useState<ZoomModel>({ version: 1, segments: [] });
  const [selected, setSelected] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const saveTimer = useRef<number | null>(null);
  const latestModelRef = useRef<ZoomModel>({ version: 1, segments: [] });

  useEffect(() => {
    let cancelled = false;
    api
      .loadRecording(videoPath)
      .then((r) => {
        if (!cancelled) {
          setMetadata(r.metadata);
          setModelState(r.zoom);
          latestModelRef.current = r.zoom;
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
      latestModelRef.current = m;
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = window.setTimeout(() => {
        api.saveZoom(videoPath, m).catch((e) => setError(String(e)));
      }, 500);
    },
    [videoPath],
  );

  useEffect(() => {
    return () => {
      if (saveTimer.current) {
        clearTimeout(saveTimer.current);
        // Flush the pending save so edits made <500ms before unmount are not lost.
        api.saveZoom(videoPath, latestModelRef.current).catch(() => {});
      }
    };
  }, [videoPath]);

  return { metadata, model, setModel, selected, setSelected, error };
}
