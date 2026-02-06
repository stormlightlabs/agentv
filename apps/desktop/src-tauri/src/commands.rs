use agent_viz_adapters::{ClaudeAdapter, CodexAdapter, CrushAdapter, OpenCodeAdapter};
use agent_viz_core::Source;
use agent_viz_store::{check_sources_health, Database};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Session data for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub id: String,
    pub source: String,
    pub external_id: String,
    pub project: Option<String>,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Event data for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    pub id: String,
    pub session_id: String,
    pub kind: String,
    pub role: Option<String>,
    pub content: Option<String>,
    pub timestamp: String,
}

/// Result of an ingestion operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub imported: usize,
    pub failed: usize,
    pub total: usize,
    pub source: String,
    pub duration_ms: u64,
}

/// Search result for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub event: EventData,
    pub rank: f64,
    pub snippet: Option<String>,
}

/// Search facets for filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacets {
    pub source: Option<String>,
    pub project: Option<String>,
    pub kind: Option<String>,
    pub since: Option<String>,
}

/// Activity stats for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityStats {
    pub day: String,
    pub event_count: i64,
    pub session_count: i64,
}

/// Error stats for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStats {
    pub day: String,
    pub error_count: i64,
    pub signature: Option<String>,
}

/// Grouped stats for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct GroupedStats {
    pub dimension: String,
    pub count: i64,
    pub sessions: Option<i64>,
    pub earliest: Option<String>,
    pub latest: Option<String>,
}

/// List all sessions
#[tauri::command]
pub async fn list_sessions() -> Result<Vec<SessionData>, String> {
    let db = Database::open_default()
        .await
        .map_err(|e| format!("Failed to open database: {}", e))?;

    db.migrate()
        .await
        .map_err(|e| format!("Failed to migrate database: {}", e))?;

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
pub async fn get_session_events(session_id: String) -> Result<Vec<EventData>, String> {
    let db = Database::open_default()
        .await
        .map_err(|e| format!("Failed to open database: {}", e))?;

    db.migrate()
        .await
        .map_err(|e| format!("Failed to migrate database: {}", e))?;

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
pub async fn ingest_source(source: String) -> Result<IngestResult, String> {
    let db = Database::open_default()
        .await
        .map_err(|e| format!("Failed to open database: {}", e))?;

    db.migrate()
        .await
        .map_err(|e| format!("Failed to migrate database: {}", e))?;

    let source = Source::from_str(&source).map_err(|e| e.to_string())?;

    let start = std::time::Instant::now();
    let (imported, failed) = match source {
        Source::Claude => {
            let adapter = ClaudeAdapter::new();
            let sessions = adapter.discover_sessions().await;
            let mut imported = 0;
            let mut failed = 0;
            for session_file in sessions {
                match adapter.parse_session(&session_file).await {
                    Ok((session, events)) => {
                        if db.insert_session_with_events(&session, &events).await.is_ok() {
                            imported += 1;
                        } else {
                            failed += 1;
                        }
                    }
                    Err(_) => failed += 1,
                }
            }
            (imported, failed)
        }
        Source::Codex => {
            let adapter = CodexAdapter::new();
            let sessions = adapter.discover_sessions().await;
            let mut imported = 0;
            let mut failed = 0;
            for session_file in sessions {
                match adapter.parse_session(&session_file).await {
                    Ok((session, events)) => {
                        if db.insert_session_with_events(&session, &events).await.is_ok() {
                            imported += 1;
                        } else {
                            failed += 1;
                        }
                    }
                    Err(_) => failed += 1,
                }
            }
            (imported, failed)
        }
        Source::OpenCode => {
            let adapter = OpenCodeAdapter::new();
            let sessions = adapter.discover_sessions().await;
            let mut imported = 0;
            let mut failed = 0;
            for session_file in sessions {
                match adapter.parse_session(&session_file).await {
                    Ok((session, events)) => {
                        if db.insert_session_with_events(&session, &events).await.is_ok() {
                            imported += 1;
                        } else {
                            failed += 1;
                        }
                    }
                    Err(_) => failed += 1,
                }
            }
            (imported, failed)
        }
        Source::Crush => {
            let adapter = CrushAdapter::new();
            let sessions = adapter.discover_sessions().await;
            let mut imported = 0;
            let mut failed = 0;
            for session_file in sessions {
                match adapter.parse_session(&session_file).await {
                    Ok((session, events)) => {
                        if db.insert_session_with_events(&session, &events).await.is_ok() {
                            imported += 1;
                        } else {
                            failed += 1;
                        }
                    }
                    Err(_) => failed += 1,
                }
            }
            (imported, failed)
        }
    };

    let duration = start.elapsed().as_millis() as u64;

    Ok(IngestResult { imported, failed, total: imported + failed, source: source.to_string(), duration_ms: duration })
}

/// Search events with FTS5 and faceted filtering
#[tauri::command]
pub async fn search_events(query: String, facets: SearchFacets, limit: i64) -> Result<Vec<SearchResult>, String> {
    use agent_viz_store::SearchFacets as DbSearchFacets;
    use chrono::Utc;

    let db = Database::open_default()
        .await
        .map_err(|e| format!("Failed to open database: {}", e))?;

    db.migrate()
        .await
        .map_err(|e| format!("Failed to migrate database: {}", e))?;

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
pub async fn get_activity_stats(since: Option<String>, until: Option<String>) -> Result<Vec<ActivityStats>, String> {
    use chrono::Utc;

    let db = Database::open_default()
        .await
        .map_err(|e| format!("Failed to open database: {}", e))?;

    db.migrate()
        .await
        .map_err(|e| format!("Failed to migrate database: {}", e))?;

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
pub async fn get_error_stats(since: Option<String>, until: Option<String>) -> Result<Vec<ErrorStats>, String> {
    use chrono::Utc;

    let db = Database::open_default()
        .await
        .map_err(|e| format!("Failed to open database: {}", e))?;

    db.migrate()
        .await
        .map_err(|e| format!("Failed to migrate database: {}", e))?;

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
pub async fn get_sources() -> Result<Vec<String>, String> {
    let db = Database::open_default()
        .await
        .map_err(|e| format!("Failed to open database: {}", e))?;

    db.migrate()
        .await
        .map_err(|e| format!("Failed to migrate database: {}", e))?;

    let sources = db
        .get_sources()
        .await
        .map_err(|e| format!("Failed to get sources: {}", e))?;

    Ok(sources)
}

/// Get available projects for faceting
#[tauri::command]
pub async fn get_projects() -> Result<Vec<String>, String> {
    let db = Database::open_default()
        .await
        .map_err(|e| format!("Failed to open database: {}", e))?;

    db.migrate()
        .await
        .map_err(|e| format!("Failed to migrate database: {}", e))?;

    let projects = db
        .get_projects()
        .await
        .map_err(|e| format!("Failed to get projects: {}", e))?;

    Ok(projects)
}

/// Get available event kinds for faceting
#[tauri::command]
pub async fn get_event_kinds() -> Result<Vec<String>, String> {
    let db = Database::open_default()
        .await
        .map_err(|e| format!("Failed to open database: {}", e))?;

    db.migrate()
        .await
        .map_err(|e| format!("Failed to migrate database: {}", e))?;

    let kinds = db
        .get_event_kinds()
        .await
        .map_err(|e| format!("Failed to get event kinds: {}", e))?;

    Ok(kinds)
}

fn parse_duration(s: &str) -> Option<chrono::Duration> {
    use chrono::Duration;

    if s.ends_with('d') {
        if let Some(days) = s.strip_suffix('d') {
            let days = days.parse().ok()?;
            Some(Duration::days(days))
        } else {
            None
        }
    } else if s.ends_with('h') {
        if let Some(hours) = s.strip_suffix('h') {
            let hours = hours.parse().ok()?;
            Some(Duration::hours(hours))
        } else {
            None
        }
    } else if s.ends_with('w') {
        if let Some(weeks) = s.strip_suffix('w') {
            let weeks = weeks.parse().ok()?;
            Some(Duration::weeks(weeks))
        } else {
            None
        }
    } else if s.ends_with('m') && !s.ends_with("min") {
        let months: i64 = s[..s.len() - 1].parse().ok()?;
        Some(Duration::days(months * 30))
    } else {
        None
    }
}

/// Get health status for all data sources
#[tauri::command]
pub async fn get_source_health() -> Result<Vec<agent_viz_core::SourceHealth>, String> {
    let health_results = check_sources_health().await;
    Ok(health_results)
}

/// Ingest from all available sources
#[tauri::command]
pub async fn ingest_all_sources() -> Result<Vec<IngestResult>, String> {
    let db = Database::open_default()
        .await
        .map_err(|e| format!("Failed to open database: {}", e))?;

    db.migrate()
        .await
        .map_err(|e| format!("Failed to migrate database: {}", e))?;

    let mut results = Vec::new();
    let sources = vec![Source::Claude, Source::Codex, Source::OpenCode, Source::Crush];

    for source in sources {
        let start = std::time::Instant::now();
        let (imported, failed) = match source {
            Source::Claude => {
                let adapter = ClaudeAdapter::new();
                let sessions = adapter.discover_sessions().await;
                let mut imported = 0;
                let mut failed = 0;
                for session_file in sessions {
                    match adapter.parse_session(&session_file).await {
                        Ok((session, events)) => {
                            if db.insert_session_with_events(&session, &events).await.is_ok() {
                                imported += 1;
                            } else {
                                failed += 1;
                            }
                        }
                        Err(_) => failed += 1,
                    }
                }
                (imported, failed)
            }
            Source::Codex => {
                let adapter = CodexAdapter::new();
                let sessions = adapter.discover_sessions().await;
                let mut imported = 0;
                let mut failed = 0;
                for session_file in sessions {
                    match adapter.parse_session(&session_file).await {
                        Ok((session, events)) => {
                            if db.insert_session_with_events(&session, &events).await.is_ok() {
                                imported += 1;
                            } else {
                                failed += 1;
                            }
                        }
                        Err(_) => failed += 1,
                    }
                }
                (imported, failed)
            }
            Source::OpenCode => {
                let adapter = OpenCodeAdapter::new();
                let sessions = adapter.discover_sessions().await;
                let mut imported = 0;
                let mut failed = 0;
                for session_file in sessions {
                    match adapter.parse_session(&session_file).await {
                        Ok((session, events)) => {
                            if db.insert_session_with_events(&session, &events).await.is_ok() {
                                imported += 1;
                            } else {
                                failed += 1;
                            }
                        }
                        Err(_) => failed += 1,
                    }
                }
                (imported, failed)
            }
            Source::Crush => {
                let adapter = CrushAdapter::new();
                let sessions = adapter.discover_sessions().await;
                let mut imported = 0;
                let mut failed = 0;
                for session_file in sessions {
                    match adapter.parse_session(&session_file).await {
                        Ok((session, events)) => {
                            if db.insert_session_with_events(&session, &events).await.is_ok() {
                                imported += 1;
                            } else {
                                failed += 1;
                            }
                        }
                        Err(_) => failed += 1,
                    }
                }
                (imported, failed)
            }
        };

        let duration = start.elapsed().as_millis() as u64;

        results.push(IngestResult {
            imported,
            failed,
            total: imported + failed,
            source: source.to_string(),
            duration_ms: duration,
        });
    }

    Ok(results)
}

/// Check for new sessions available in source directories
#[tauri::command]
pub async fn check_for_new_sessions() -> Result<bool, String> {
    use agent_viz_adapters::{ClaudeAdapter, CodexAdapter, CrushAdapter, OpenCodeAdapter};

    let db = Database::open_default()
        .await
        .map_err(|e| format!("Failed to open database: {}", e))?;

    db.migrate()
        .await
        .map_err(|e| format!("Failed to migrate database: {}", e))?;

    let existing_sessions = db
        .list_sessions(10000, 0)
        .await
        .map_err(|e| format!("Failed to list sessions: {}", e))?;
    let existing_ids: std::collections::HashSet<String> =
        existing_sessions.into_iter().map(|s| s.external_id).collect();

    let claude_adapter = ClaudeAdapter::new();
    for session_file in claude_adapter.discover_sessions().await {
        if !existing_ids.contains(&session_file.session_id) {
            return Ok(true);
        }
    }

    let codex_adapter = CodexAdapter::new();
    for session_file in codex_adapter.discover_sessions().await {
        if !existing_ids.contains(&session_file.session_id) {
            return Ok(true);
        }
    }

    let opencode_adapter = OpenCodeAdapter::new();
    for session in opencode_adapter.discover_sessions().await {
        if !existing_ids.contains(&session.id) {
            return Ok(true);
        }
    }

    let crush_adapter = CrushAdapter::new();
    for session_file in crush_adapter.discover_sessions().await {
        if !existing_ids.contains(&session_file.session_id) {
            return Ok(true);
        }
    }

    Ok(false)
}
