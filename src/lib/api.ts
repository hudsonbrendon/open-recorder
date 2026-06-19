import { invoke } from "@tauri-apps/api/core";

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
