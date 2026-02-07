use agent_v_store::Database;
use serde::{Deserialize, Serialize};

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
    pub raw_payload: serde_json::Value,
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
            raw_payload: serde_json::from_str(&row.raw_payload).unwrap_or(serde_json::Value::Null),
        })
        .collect();

    Ok(events)
}

/// Trigger ingestion from a source
#[tauri::command]
pub async fn ingest_source(source: String) -> Result<IngestResult, String> {
    use agent_v_adapters::{claude::ClaudeAdapter, crush::CrushAdapter};
    use agent_v_core::Source;
    use std::str::FromStr;

    let db = Database::open_default()
        .await
        .map_err(|e| format!("Failed to open database: {}", e))?;

    db.migrate()
        .await
        .map_err(|e| format!("Failed to migrate database: {}", e))?;

    let source = Source::from_str(&source).map_err(|e| e.to_string())?;

    match source {
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

            Ok(IngestResult { imported, failed, total: imported + failed })
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

            Ok(IngestResult { imported, failed, total: imported + failed })
        }
        _ => Err(format!("Source '{}' not yet implemented", source)),
    }
}

/// Result of an ingestion operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub imported: usize,
    pub failed: usize,
    pub total: usize,
}
