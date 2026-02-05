mod commands;

use commands::{get_session_events, ingest_source, list_sessions};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            get_session_events,
            ingest_source
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
