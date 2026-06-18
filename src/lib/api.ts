import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ZoomModel } from "./zoom";

export interface SourceOption {
  id: string;
  name: string;
  kind: string;
  rect: [number, number, number, number];
}

export interface MicOption {
  id: string;
  name: string;
}

export interface SourcesPayload {
  displays: SourceOption[];
  windows: SourceOption[];
}

export interface RecordingResult {
  video_path: string;
  metadata_path: string;
  duration_ms: number;
}

export const listSources = () => invoke<SourcesPayload>("list_sources");
export const listMicrophones = () => invoke<MicOption[]>("list_microphones");
export const startRecording = (source: SourceOption, micId: string | null) =>
  invoke<void>("start_recording", { source, micId });
export const stopRecording = () => invoke<RecordingResult>("stop_recording");
export const revealInFolder = (path: string) => invoke<void>("reveal_in_folder", { path });

export interface InputEventDTO {
  t_ms: number;
  type: string;
  x: number;
  y: number;
  button?: string;
}

export interface RecordingMetadata {
  version: number;
  recording: { width: number; height: number; fps: number; duration_ms: number };
  source: { type: string; id: string; rect: [number, number, number, number] };
  events: InputEventDTO[];
}

export interface LoadedRecording {
  metadata: RecordingMetadata;
  zoom: ZoomModel;
}

export const loadRecording = (videoPath: string) =>
  invoke<LoadedRecording>("load_recording", { videoPath });

export const saveZoom = (videoPath: string, zoom: ZoomModel) =>
  invoke<void>("save_zoom", { videoPath, zoom });

export const exportWithZoom = (
  videoPath: string,
  zoom: ZoomModel,
  outPath: string,
  fps: number,
  totalMs: number,
) => invoke<void>("export_with_zoom", { videoPath, zoom, outPath, fps, totalMs });

export const onExportProgress = (cb: (p: number) => void) =>
  listen<number>("export-progress", (e) => cb(e.payload));
