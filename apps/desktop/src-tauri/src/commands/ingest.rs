use super::models::IngestResult;

use agent_v_adapters::{ClaudeAdapter, CodexAdapter, CrushAdapter, OpenCodeAdapter};
use agent_v_core::Source;
use agent_v_store::Database;

/// Ingest sessions from a single source
pub async fn ingest_single_source(db: &Database, source: Source) -> Result<IngestResult, String> {
    let start = std::time::Instant::now();

    let (imported, failed) = match source {
        Source::Claude => ingest_claude(db).await,
        Source::Codex => ingest_codex(db).await,
        Source::OpenCode => ingest_opencode(db).await,
        Source::Crush => ingest_crush(db).await,
    };

    let duration = start.elapsed().as_millis() as u64;

    Ok(IngestResult { imported, failed, total: imported + failed, source: source.to_string(), duration_ms: duration })
}

/// Ingest from Claude source
async fn ingest_claude(db: &Database) -> (usize, usize) {
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

/// Ingest from Codex source
async fn ingest_codex(db: &Database) -> (usize, usize) {
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

/// Ingest from OpenCode source
async fn ingest_opencode(db: &Database) -> (usize, usize) {
    let adapter = OpenCodeAdapter::new();
    let sessions = adapter.discover_sessions().await;
    let mut imported = 0;
    let mut failed = 0;

    for session in sessions {
        match adapter.parse_session(&session).await {
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

/// Ingest from Crush source
async fn ingest_crush(db: &Database) -> (usize, usize) {
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

/// Check if new sessions are available in any source
pub async fn check_new_sessions_available(db: &Database) -> Result<bool, String> {
    let existing_sessions = db
        .list_sessions(10000, 0)
        .await
        .map_err(|e| format!("Failed to list sessions: {}", e))?;
    let existing_ids: std::collections::HashSet<String> =
        existing_sessions.into_iter().map(|s| s.external_id).collect();

    let adapter = ClaudeAdapter::new();
    for session_file in adapter.discover_sessions().await {
        if !existing_ids.contains(&session_file.session_id) {
            return Ok(true);
        }
    }

    let adapter = CodexAdapter::new();
    for session_file in adapter.discover_sessions().await {
        if !existing_ids.contains(&session_file.session_id) {
            return Ok(true);
        }
    }

    let adapter = OpenCodeAdapter::new();
    for session in adapter.discover_sessions().await {
        if !existing_ids.contains(&session.id) {
            return Ok(true);
        }
    }

    let adapter = CrushAdapter::new();
    for session_file in adapter.discover_sessions().await {
        if !existing_ids.contains(&session_file.session_id) {
            return Ok(true);
        }
    }

    Ok(false)
}
