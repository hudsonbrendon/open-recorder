use std::sync::Mutex;
use tauri::State;

use crate::capture::audio_capture;
use crate::capture::webcam_capture;
use crate::capture::source_enum::{self, SourceOption};
use crate::recording::coordinator::{Coordinator, RecordingResult};
use crate::model::metadata::RecordingMetadata;
use crate::model::zoom::ZoomModel;
use crate::zoom::{store, generate::{generate, GenOpts}};

#[derive(serde::Serialize)]
pub struct LoadedRecording {
    pub metadata: RecordingMetadata,
    pub zoom: ZoomModel,
}

fn metadata_path(video_path: &str) -> std::path::PathBuf {
    std::path::Path::new(video_path).with_extension("metadata.json")
}

#[tauri::command]
pub fn load_recording(video_path: String) -> Result<LoadedRecording, String> {
    let mtxt = std::fs::read_to_string(metadata_path(&video_path))
        .map_err(|e| format!("metadata não encontrada: {e}"))?;
    let metadata: RecordingMetadata = serde_json::from_str(&mtxt).map_err(|e| e.to_string())?;
    let zoom = store::load(&video_path)
        .unwrap_or_else(|| generate(&metadata.events, metadata.source.rect, &GenOpts::default()));
    Ok(LoadedRecording { metadata, zoom })
}

#[tauri::command]
pub fn save_zoom(video_path: String, zoom: ZoomModel) -> Result<(), String> {
    store::save(&video_path, &zoom)
}

#[derive(serde::Serialize)]
pub struct SourcesPayload {
    pub displays: Vec<SourceOption>,
    pub windows: Vec<SourceOption>,
}

#[derive(serde::Serialize)]
pub struct MicOption {
    pub id: String,
    pub name: String,
}

#[derive(serde::Serialize)]
pub struct CameraOption {
    pub id: String,
    pub name: String,
}

#[tauri::command]
pub fn list_sources() -> Result<SourcesPayload, String> {
    Ok(SourcesPayload {
        displays: source_enum::list_displays()?,
        windows: source_enum::list_windows().unwrap_or_default(),
    })
}

#[tauri::command]
pub fn list_microphones() -> Vec<MicOption> {
    audio_capture::list_microphones()
        .into_iter()
        .map(|(id, name)| MicOption { id, name })
        .collect()
}

#[tauri::command]
pub fn list_cameras() -> Vec<CameraOption> {
    webcam_capture::list_cameras()
        .into_iter()
        .map(|(id, name)| CameraOption { id, name })
        .collect()
}

#[tauri::command]
pub fn start_recording(
    state: State<'_, Mutex<Coordinator>>,
    source: SourceOption,
    mic_id: Option<String>,
    camera_id: Option<String>,
) -> Result<(), String> {
    let cs = source_enum::to_capture_source(&source);
    state.lock().unwrap().start(cs, mic_id, camera_id, 30)
}

#[tauri::command]
pub fn stop_recording(state: State<'_, Mutex<Coordinator>>) -> Result<RecordingResult, String> {
    state.lock().unwrap().stop()
}

#[tauri::command]
pub fn reveal_in_folder(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let dir = p.parent().unwrap_or(p);
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(dir).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(dir).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
    Ok(())
}

#[tauri::command]
pub fn export_with_zoom(
    app: tauri::AppHandle,
    video_path: String,
    zoom: crate::model::zoom::ZoomModel,
    out_path: String,
    fps: u32,
    total_ms: u64,
) -> Result<(), String> {
    use tauri::Emitter;
    crate::zoom::export::export(&video_path, &zoom, &out_path, fps, total_ms, |p| {
        let _ = app.emit("export-progress", p);
    })
}
