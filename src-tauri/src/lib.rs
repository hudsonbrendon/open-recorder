pub mod model;
pub mod capture;
pub mod recording;
pub mod commands;
pub mod zoom;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(std::sync::Mutex::new(
            crate::recording::coordinator::Coordinator::default(),
        ))
        .invoke_handler(tauri::generate_handler![
            crate::commands::list_sources,
            crate::commands::list_microphones,
            crate::commands::start_recording,
            crate::commands::stop_recording,
            crate::commands::reveal_in_folder,
            crate::commands::load_recording,
            crate::commands::save_zoom,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
