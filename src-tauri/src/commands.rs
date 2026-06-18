use std::sync::Mutex;
use tauri::State;

use crate::capture::audio_capture;
use crate::capture::source_enum::{self, SourceOption};
use crate::recording::coordinator::{Coordinator, RecordingResult};

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
pub fn start_recording(
    state: State<'_, Mutex<Coordinator>>,
    source: SourceOption,
    mic_id: Option<String>,
) -> Result<(), String> {
    let cs = source_enum::to_capture_source(&source);
    state.lock().unwrap().start(cs, mic_id, 30)
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
