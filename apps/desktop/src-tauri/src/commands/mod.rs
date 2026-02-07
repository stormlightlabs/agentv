mod export;
mod ingest;
mod models;

use agent_v_core::Source;
use agent_v_store::SearchFacets as DbSearchFacets;
use agent_v_store::{check_sources_health, Database};
use chrono::{Duration, Utc};
use std::str::FromStr;
use tauri::State;

pub use models::*;

/// List all sessions
#[tauri::command]
pub async fn list_sessions(db: State<'_, Database>) -> Result<Vec<SessionData>, String> {
    let rows = db
        .list_sessions(1000, 0)
        .await
        .map_err(|e| format!("Failed to list sessions: {}", e))?;

    let sessions = rows
        .into_iter()
        .map(|row| SessionData {
            id: row.id,
            source: row.source,
            external_id: row.external_id,
            project: row.project,
            title: row.title,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect();

    Ok(sessions)
}

/// Get events for a session
#[tauri::command]
pub async fn get_session_events(db: State<'_, Database>, session_id: String) -> Result<Vec<EventData>, String> {
    let rows = db
        .get_session_events(session_id)
        .await
        .map_err(|e| format!("Failed to get session events: {}", e))?;

    let events = rows
        .into_iter()
        .map(|row| EventData {
            id: row.id,
            session_id: row.session_id,
            kind: row.kind,
            role: row.role,
            content: row.content,
            timestamp: row.timestamp,
        })
        .collect();

    Ok(events)
}

/// Trigger ingestion from a source
#[tauri::command]
pub async fn ingest_source(db: State<'_, Database>, source: String) -> Result<IngestResult, String> {
    let source = Source::from_str(&source)?;

    ingest::ingest_single_source(&db, source).await
}

/// Search events with FTS5 and faceted filtering
#[tauri::command]
pub async fn search_events(
    db: State<'_, Database>, query: String, facets: SearchFacets, limit: i64,
) -> Result<Vec<SearchResult>, String> {
    let since_dt = facets
        .since
        .and_then(|s| parse_duration(&s))
        .map(|dur| Utc::now() - dur);

    let db_facets =
        DbSearchFacets { source: facets.source, project: facets.project, kind: facets.kind, since: since_dt };

    let results = db
        .search_events(&query, &db_facets, limit, 0)
        .await
        .map_err(|e| format!("Failed to search events: {}", e))?;

    Ok(results
        .into_iter()
        .map(|r| SearchResult {
            event: EventData {
                id: r.event.id,
                session_id: r.event.session_id,
                kind: r.event.kind,
                role: r.event.role,
                content: r.event.content,
                timestamp: r.event.timestamp,
            },
            rank: r.rank,
            snippet: r.snippet,
        })
        .collect())
}

/// Get activity stats by day
#[tauri::command]
pub async fn get_activity_stats(
    db: State<'_, Database>, since: Option<String>, until: Option<String>,
) -> Result<Vec<ActivityStats>, String> {
    let since_dt = since.and_then(|s| parse_duration(&s)).map(|dur| Utc::now() - dur);
    let until_dt = until.and_then(|s| parse_duration(&s)).map(|dur| Utc::now() - dur);

    let stats = db
        .get_activity_by_day(since_dt, until_dt, None)
        .await
        .map_err(|e| format!("Failed to get activity stats: {}", e))?;

    Ok(stats
        .into_iter()
        .map(|s| ActivityStats { day: s.day.to_string(), event_count: s.event_count, session_count: s.session_count })
        .collect())
}

/// Get error stats
#[tauri::command]
pub async fn get_error_stats(
    db: State<'_, Database>, since: Option<String>, until: Option<String>,
) -> Result<Vec<ErrorStats>, String> {
    let since_dt = since.and_then(|s| parse_duration(&s)).map(|dur| Utc::now() - dur);
    let until_dt = until.and_then(|s| parse_duration(&s)).map(|dur| Utc::now() - dur);

    let stats = db
        .get_errors_by_day(since_dt, until_dt)
        .await
        .map_err(|e| format!("Failed to get error stats: {}", e))?;

    Ok(stats
        .into_iter()
        .map(|s| ErrorStats { day: s.day.to_string(), error_count: s.error_count, signature: s.signature })
        .collect())
}

/// Get available sources for faceting
#[tauri::command]
pub async fn get_sources(db: State<'_, Database>) -> Result<Vec<String>, String> {
    let sources = db
        .get_sources()
        .await
        .map_err(|e| format!("Failed to get sources: {}", e))?;

    Ok(sources)
}

/// Get available projects for faceting
#[tauri::command]
pub async fn get_projects(db: State<'_, Database>) -> Result<Vec<String>, String> {
    let projects = db
        .get_projects()
        .await
        .map_err(|e| format!("Failed to get projects: {}", e))?;

    Ok(projects)
}

/// Get available event kinds for faceting
#[tauri::command]
pub async fn get_event_kinds(db: State<'_, Database>) -> Result<Vec<String>, String> {
    let kinds = db
        .get_event_kinds()
        .await
        .map_err(|e| format!("Failed to get event kinds: {}", e))?;

    Ok(kinds)
}

fn parse_duration(s: &str) -> Option<chrono::Duration> {
    if s.ends_with('d') {
        s.strip_suffix('d')
            .and_then(|days| days.parse().ok())
            .map(Duration::days)
    } else if s.ends_with('h') {
        s.strip_suffix('h')
            .and_then(|hours| hours.parse().ok())
            .map(Duration::hours)
    } else if s.ends_with('w') {
        s.strip_suffix('w')
            .and_then(|weeks| weeks.parse().ok())
            .map(Duration::weeks)
    } else if s.ends_with('m') && !s.ends_with("min") {
        s[..s.len() - 1]
            .parse()
            .ok()
            .map(|months: i64| Duration::days(months * 30))
    } else {
        None
    }
}

/// Get health status for all data sources
#[tauri::command]
pub async fn get_source_health() -> Result<Vec<agent_v_core::SourceHealth>, String> {
    let health_results = check_sources_health().await;
    Ok(health_results)
}

/// Ingest from all available sources
#[tauri::command]
pub async fn ingest_all_sources(db: State<'_, Database>) -> Result<Vec<IngestResult>, String> {
    let mut results = Vec::new();
    let sources = vec![Source::Claude, Source::Codex, Source::OpenCode, Source::Crush];

    for source in sources {
        match ingest::ingest_single_source(&db, source).await {
            Ok(result) => results.push(result),
            Err(e) => {
                results.push(IngestResult {
                    imported: 0,
                    failed: 0,
                    total: 0,
                    source: source.to_string(),
                    duration_ms: 0,
                });
                eprintln!("Failed to ingest {}: {}", source, e);
            }
        }
    }

    Ok(results)
}

/// Check for new sessions available in source directories
#[tauri::command]
pub async fn check_for_new_sessions(db: State<'_, Database>) -> Result<bool, String> {
    ingest::check_new_sessions_available(&db).await
}

/// Get tool call frequency stats
#[tauri::command]
pub async fn get_tool_call_frequency(
    db: State<'_, Database>, since: Option<String>, _until: Option<String>,
) -> Result<Vec<ToolFrequencyStats>, String> {
    let since_dt = since.and_then(|s| parse_duration(&s)).map(|dur| Utc::now() - dur);

    let stats = db
        .get_tool_call_frequency(since_dt, None)
        .await
        .map_err(|e| format!("Failed to get tool call frequency: {}", e))?;

    Ok(stats
        .into_iter()
        .map(|s| ToolFrequencyStats {
            tool_name: s.tool_name,
            call_count: s.call_count,
            sessions: s.sessions,
            avg_duration_ms: s.avg_duration_ms,
            max_duration_ms: s.max_duration_ms,
        })
        .collect())
}

/// Get files touched leaderboard
#[tauri::command]
pub async fn get_files_leaderboard(
    db: State<'_, Database>, since: Option<String>, _until: Option<String>, limit: Option<i64>,
) -> Result<Vec<FileLeaderboardEntry>, String> {
    use chrono::Utc;

    let since_dt = since.and_then(|s| parse_duration(&s)).map(|dur| Utc::now() - dur);
    let limit = limit.unwrap_or(20);

    let stats = db
        .get_files_leaderboard(since_dt, None, limit)
        .await
        .map_err(|e| format!("Failed to get files leaderboard: {}", e))?;

    Ok(stats
        .into_iter()
        .map(|s| FileLeaderboardEntry {
            file_path: s.file_path,
            touch_count: s.touch_count,
            sessions: s.sessions,
            total_lines_added: s.total_lines_added,
            total_lines_removed: s.total_lines_removed,
        })
        .collect())
}

/// Get patch churn stats by day
#[tauri::command]
pub async fn get_patch_churn(
    db: State<'_, Database>, since: Option<String>, _until: Option<String>,
) -> Result<Vec<PatchChurnStats>, String> {
    let since_dt = since.and_then(|s| parse_duration(&s)).map(|dur| Utc::now() - dur);

    let stats = db
        .get_patch_churn_by_day(since_dt, None)
        .await
        .map_err(|e| format!("Failed to get patch churn: {}", e))?;

    Ok(stats
        .into_iter()
        .map(|s| PatchChurnStats {
            day: s.day.to_string(),
            lines_added: s.lines_added,
            lines_removed: s.lines_removed,
            files_changed: s.files_changed,
            sessions: s.sessions,
        })
        .collect())
}

/// Get long-running tool calls
#[tauri::command]
pub async fn get_long_running_tools(
    db: State<'_, Database>, since: Option<String>, _until: Option<String>, min_duration_ms: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<LongRunningToolCall>, String> {
    let since_dt = since.and_then(|s| parse_duration(&s)).map(|dur| Utc::now() - dur);
    let min_duration = min_duration_ms.unwrap_or(5000);
    let limit = limit.unwrap_or(20);

    let stats = db
        .get_long_running_tool_calls(since_dt, None, min_duration, limit)
        .await
        .map_err(|e| format!("Failed to get long running tools: {}", e))?;

    Ok(stats
        .into_iter()
        .map(|s| LongRunningToolCall {
            tool_name: s.tool_name,
            duration_ms: s.duration_ms,
            started_at: s.started_at,
            session_external_id: s.session_external_id,
            project: s.project,
            error_message: s.error_message,
        })
        .collect())
}

/// Export a session to various formats
#[tauri::command]
pub async fn export_session(db: State<'_, Database>, session_id: String, format: String) -> Result<String, String> {
    let mut session = None;
    let mut offset = 0;
    loop {
        let sessions = db.list_sessions(100, offset).await.map_err(|e| e.to_string())?;
        if sessions.is_empty() {
            break;
        }
        if let Some(found) = sessions
            .into_iter()
            .find(|s| s.id == session_id || s.external_id == session_id)
        {
            session = Some(found);
            break;
        }
        offset += 100;
    }

    let session = session.ok_or_else(|| format!("Session not found: {}", session_id))?;
    let events = db
        .get_session_events(session.id.clone())
        .await
        .map_err(|e| e.to_string())?;

    let export_format = ExportFormat::from_str(&format)?;

    match export_format {
        ExportFormat::Markdown => export::export_session_to_markdown(&session, &events).await,
        ExportFormat::Json => export::export_session_to_json(&session, &events).await,
        ExportFormat::Jsonl => export::export_session_to_jsonl(&session, &events).await,
    }
}

/// Export search results to various formats
#[tauri::command]
pub async fn export_search(
    db: State<'_, Database>, query: String, source: Option<String>, since: Option<String>, kind: Option<String>,
    format: String,
) -> Result<String, String> {
    let since_dt = since.and_then(|s| parse_duration(&s)).map(|dur| Utc::now() - dur);

    let db_facets = DbSearchFacets { source, project: None, kind, since: since_dt };

    let results = db
        .search_events(&query, &db_facets, 10000, 0)
        .await
        .map_err(|e| e.to_string())?;

    let export_format = ExportFormat::from_str(&format)?;

    match export_format {
        ExportFormat::Markdown => export::export_search_to_markdown(&query, &results).await,
        ExportFormat::Json => export::export_search_to_json(&query, &results).await,
        ExportFormat::Jsonl => export::export_search_to_jsonl(&query, &results).await,
    }
}
