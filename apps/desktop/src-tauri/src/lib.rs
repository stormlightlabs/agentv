mod commands;

use commands::{
    check_for_new_sessions, export_search, export_session, get_activity_stats, get_error_stats, get_event_kinds,
    get_files_leaderboard, get_long_running_tools, get_patch_churn, get_projects, get_session_events,
    get_source_health, get_sources, get_tool_call_frequency, ingest_all_sources, ingest_source, list_sessions,
    search_events,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            get_session_events,
            ingest_source,
            ingest_all_sources,
            search_events,
            get_activity_stats,
            get_error_stats,
            get_sources,
            get_projects,
            get_event_kinds,
            get_source_health,
            check_for_new_sessions,
            get_tool_call_frequency,
            get_files_leaderboard,
            get_patch_churn,
            get_long_running_tools,
            export_session,
            export_search
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
