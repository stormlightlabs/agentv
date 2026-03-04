mod commands;

use agent_v_ingest::{IngestProgress, StreamingEvent, WatcherConfig};
use agent_v_store::Database;
use commands::{
    check_for_new_sessions, export_search, export_session, get_activity_stats, get_cost_stats_by_project,
    get_cost_stats_by_source, get_efficiency_stats, get_error_stats, get_event_kinds, get_files_leaderboard,
    get_latency_distribution, get_long_running_tools, get_model_usage_stats, get_patch_churn, get_projects,
    get_session_events, get_session_metrics, get_source_health, get_sources, get_tool_call_frequency,
    ingest_all_sources, ingest_source, list_sessions, recompute_all_metrics, search_events,
};
use commands::{EventData, StreamingEventPayload};
use std::sync::Arc;
use tauri::{Emitter, Manager};

fn streaming_event_to_payload(event: StreamingEvent) -> StreamingEventPayload {
    let events: Vec<EventData> = event
        .new_events
        .iter()
        .map(|e| EventData {
            id: e.id.to_string(),
            session_id: e.session_id.to_string(),
            kind: format!("{:?}", e.kind).to_lowercase(),
            role: e.role.map(|r| format!("{:?}", r).to_lowercase()),
            content: e.content.clone(),
            timestamp: e.timestamp.to_rfc3339(),
        })
        .collect();

    StreamingEventPayload {
        session_external_id: event.session_external_id,
        source: event.source,
        project: event.project,
        events,
        is_new_session: event.is_new_session,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_log::Builder::new().level(log::LevelFilter::Debug).build())
        .setup(|app| {
            tauri::async_runtime::block_on(async {
                let db = Database::open_default().await.expect("Failed to open database");
                db.migrate().await.expect("Failed to run database migrations");
                app.manage(db);
            });

            let app_handle = app.handle().clone();
            let progress_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let watcher = agent_v_ingest::Watcher::with_callbacks(
                    WatcherConfig::default(),
                    Arc::new(move |event: StreamingEvent| {
                        let payload = streaming_event_to_payload(event);
                        if let Err(e) = app_handle.emit("agent-events", &payload) {
                            log::error!("Failed to emit agent-events: {}", e);
                        }
                    }),
                    Arc::new(move |progress: IngestProgress| {
                        if let Err(e) = progress_handle.emit("ingest-progress", &progress) {
                            log::error!("Failed to emit ingest-progress: {}", e);
                        }
                    }),
                );
                if let Err(e) = watcher.watch_all().await {
                    log::error!("Watcher failed: {}", e);
                }
            });

            Ok(())
        })
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
            export_search,
            recompute_all_metrics,
            get_session_metrics,
            get_cost_stats_by_source,
            get_cost_stats_by_project,
            get_model_usage_stats,
            get_latency_distribution,
            get_efficiency_stats
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
