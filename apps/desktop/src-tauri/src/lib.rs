mod commands;

use commands::{
    get_activity_stats, get_error_stats, get_event_kinds, get_projects, get_session_events, get_sources, ingest_source,
    list_sessions, search_events,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            get_session_events,
            ingest_source,
            search_events,
            get_activity_stats,
            get_error_stats,
            get_sources,
            get_projects,
            get_event_kinds
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
