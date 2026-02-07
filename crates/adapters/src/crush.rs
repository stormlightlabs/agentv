use agent_v_core::{Event, EventKind, Role, Session, Source};
use chrono::{DateTime, TimeZone, Utc};
use rayon::prelude::*;
use rusqlite::{Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

/// Check if sessions table exists
const CHECK_SESSIONS_TABLE: &str = r#"
    SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sessions'
"#;

/// List all root sessions
const LIST_SESSIONS: &str = r#"
    SELECT id FROM sessions WHERE parent_session_id IS NULL ORDER BY updated_at DESC
"#;

/// Get session with todos column
const GET_SESSION_WITH_TODOS: &str = r#"
    SELECT id, parent_session_id, title, message_count, prompt_tokens, completion_tokens, cost,
           updated_at, created_at, summary_message_id, todos
    FROM sessions WHERE id = ?
"#;

/// Get session without todos column
const GET_SESSION_WITHOUT_TODOS: &str = r#"
    SELECT id, parent_session_id, title, message_count, prompt_tokens, completion_tokens, cost,
           updated_at, created_at, summary_message_id, NULL as todos
    FROM sessions WHERE id = ?
"#;

/// Get messages with all optional columns
const GET_MESSAGES_FULL: &str = r#"
    SELECT id, session_id, role, parts, model, provider, created_at, updated_at, finished_at, is_summary_message
    FROM messages WHERE session_id = ? ORDER BY created_at ASC
"#;

/// Get messages without provider column
const GET_MESSAGES_NO_PROVIDER: &str = r#"
    SELECT id, session_id, role, parts, model, NULL as provider, created_at, updated_at, finished_at, is_summary_message
    FROM messages WHERE session_id = ? ORDER BY created_at ASC
"#;

/// Get messages without is_summary_message column
const GET_MESSAGES_NO_SUMMARY_FLAG: &str = r#"
    SELECT id, session_id, role, parts, model, provider, created_at, updated_at, finished_at, 0 as is_summary_message
    FROM messages WHERE session_id = ? ORDER BY created_at ASC
"#;

/// Get messages minimal (no provider, no is_summary_message)
const GET_MESSAGES_MINIMAL: &str = r#"
    SELECT id, session_id, role, parts, model, NULL as provider, created_at, updated_at, finished_at, 0 as is_summary_message
    FROM messages WHERE session_id = ? ORDER BY created_at ASC
"#;

/// Check for provider column
const CHECK_PROVIDER_COLUMN: &str = r#"
    SELECT COUNT(*) FROM pragma_table_info('messages') WHERE name='provider'
"#;

/// Check for is_summary_message column
const CHECK_SUMMARY_COLUMN: &str = r#"
    SELECT COUNT(*) FROM pragma_table_info('messages') WHERE name='is_summary_message'
"#;

/// Check for todos column
const CHECK_TODOS_COLUMN: &str = r#"
    SELECT COUNT(*) FROM pragma_table_info('sessions') WHERE name='todos'
"#;

/// Check for read_files table
const CHECK_READ_FILES_TABLE: &str = r#"
    SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='read_files'
"#;

/// Get read files for a session
const GET_READ_FILES: &str = r#"
    SELECT session_id, path, read_at FROM read_files WHERE session_id = ? ORDER BY read_at ASC
"#;

/// A discovered Crush database file
#[derive(Debug, Clone)]
pub struct CrushSessionFile {
    pub path: PathBuf,
    pub session_id: String,
}

/// Crush Message model from database
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CrushMessage {
    id: String,
    session_id: String,
    role: String,
    parts: String,
    model: Option<String>,
    provider: Option<String>,
    created_at: i64,
    updated_at: i64,
    finished_at: Option<i64>,
    is_summary_message: i64,
}

/// Crush ReadFile model from database
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CrushReadFile {
    session_id: String,
    path: String,
    read_at: i64,
}

/// Crush Session model from database
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CrushSession {
    id: String,
    parent_session_id: Option<String>,
    title: String,
    message_count: i64,
    prompt_tokens: i64,
    completion_tokens: i64,
    cost: f64,
    updated_at: i64,
    created_at: i64,
    summary_message_id: Option<String>,
    todos: Option<String>,
}

/// Represents a content part in a Crush message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentPart {
    #[serde(rename = "text")]
    Text { data: TextData },
    #[serde(rename = "reasoning")]
    Thinking { data: ThinkingData },
    #[serde(rename = "tool_use")]
    ToolUse { data: ToolUseData },
    #[serde(rename = "tool_result")]
    ToolResult { data: ToolResultData },
    #[serde(rename = "image")]
    Image { data: serde_json::Value },
    #[serde(rename = "finish")]
    Finish { data: FinishData },
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TextData {
    text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ThinkingData {
    thinking: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolUseData {
    id: String,
    name: String,
    #[serde(flatten)]
    input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolResultData {
    tool_use_id: String,
    content: String,
    is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FinishData {
    reason: String,
    time: u64,
}

/// Schema feature detection results
#[derive(Debug, Clone, Default)]
struct SchemaFeatures {
    has_provider_column: bool,
    has_is_summary_message: bool,
    has_todos_column: bool,
    has_read_files_table: bool,
}

/// Adapter for Crush databases
#[derive(Debug, Clone)]
pub struct CrushAdapter {
    db_path: PathBuf,
}

impl CrushAdapter {
    /// Create a new Crush adapter with default database path
    pub fn new() -> Self {
        let db_path = dirs::home_dir()
            .map(|h| h.join(".crush").join("crush.db"))
            .unwrap_or_else(|| PathBuf::from(".crush/crush.db"));

        Self { db_path }
    }

    /// Create a new Crush adapter with a custom database path
    pub fn with_db_path(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    /// Get the database path
    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }

    /// Discover all Crush databases and their sessions
    pub async fn discover_sessions(&self) -> Vec<CrushSessionFile> {
        let mut sessions = Vec::new();

        let home_dir = match dirs::home_dir() {
            Some(home) => home,
            None => {
                tracing::warn!("Could not determine home directory");
                return sessions;
            }
        };

        tracing::info!("Searching for Crush databases in: {:?}", home_dir);

        let db_paths: Vec<PathBuf> = WalkDir::new(&home_dir)
            .max_depth(6)
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();
                let is_common_skip = matches!(
                    name.as_ref(),
                    "node_modules" | "target" | "vendor" | "build" | "dist" | ".git" | "Cache"
                );
                !is_common_skip || entry.file_type().is_file()
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.file_name() == "crush.db"
                    && e.path()
                        .parent()
                        .and_then(|p| p.file_name())
                        .map(|n| n == ".crush")
                        .unwrap_or(false)
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        tracing::info!("Found {} Crush database(s)", db_paths.len());

        sessions = db_paths
            .into_par_iter()
            .flat_map(|db_path| {
                tracing::debug!("Processing Crush database: {:?}", db_path);
                match self.discover_sessions_in_db(&db_path) {
                    Ok(found) => {
                        tracing::debug!("Found {} sessions in {:?}", found.len(), db_path);
                        found
                    }
                    Err(e) => {
                        tracing::warn!("Failed to read database {:?}: {}", db_path, e);
                        Vec::new()
                    }
                }
            })
            .collect();

        tracing::info!("Discovered {} total Crush sessions", sessions.len());
        sessions
    }

    /// Discover sessions within a specific Crush database
    fn discover_sessions_in_db(
        &self, db_path: &Path,
    ) -> Result<Vec<CrushSessionFile>, Box<dyn std::error::Error + Send + Sync>> {
        let mut sessions = Vec::new();

        let conn = Connection::open(db_path)?;

        let has_sessions: i64 = conn.query_row(CHECK_SESSIONS_TABLE, [], |row| row.get(0)).unwrap_or(0);

        if has_sessions == 0 {
            tracing::warn!("No sessions table found in database: {:?}", db_path);
            return Ok(sessions);
        }

        let mut stmt = conn.prepare(LIST_SESSIONS)?;

        let session_ids = stmt.query_map([], |row| row.get::<_, String>(0))?;

        for session_id in session_ids {
            let session_id = session_id?;
            sessions.push(CrushSessionFile { path: db_path.to_path_buf(), session_id });
        }

        Ok(sessions)
    }

    /// Parse a session from the Crush database
    pub async fn parse_session(
        &self, session_file: &CrushSessionFile,
    ) -> Result<(Session, Vec<Event>), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!(
            "Parsing Crush session: {} from {:?}",
            session_file.session_id,
            session_file.path
        );

        let conn = Connection::open(&session_file.path)?;
        let features = self.detect_schema_features(&conn)?;
        let crush_session = self.get_session(&conn, &session_file.session_id, &features)?;
        let messages = self.get_session_messages(&conn, &session_file.session_id, &features)?;
        let read_files = self.get_read_files(&conn, &session_file.session_id, &features);

        let created_at = timestamp_to_datetime(crush_session.created_at);
        let updated_at = timestamp_to_datetime(crush_session.updated_at);

        let project = session_file
            .path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        let raw_payload = serde_json::json!({
            "source": "crush",
            "db_path": session_file.path.to_string_lossy(),
            "session_id": crush_session.id,
            "parent_session_id": crush_session.parent_session_id,
            "message_count": crush_session.message_count,
            "prompt_tokens": crush_session.prompt_tokens,
            "completion_tokens": crush_session.completion_tokens,
            "cost": crush_session.cost,
            "todos": crush_session.todos,
            "read_files": read_files,
        });

        let session = Session {
            id: Uuid::new_v4(),
            source: Source::Crush,
            external_id: crush_session.id,
            project,
            title: Some(crush_session.title),
            created_at,
            updated_at,
            raw_payload,
        };

        let events: Vec<Event> = messages
            .into_iter()
            .filter_map(|msg| self.message_to_event(msg, &features))
            .map(|mut event| {
                event.session_id = session.id;
                event
            })
            .collect();

        tracing::info!(
            "Parsed Crush session {} with {} events",
            session.external_id,
            events.len()
        );

        Ok((session, events))
    }

    /// Detect database schema features for graceful degradation
    fn detect_schema_features(&self, conn: &Connection) -> SqliteResult<SchemaFeatures> {
        let has_provider_column = conn
            .query_row(CHECK_PROVIDER_COLUMN, [], |row| row.get::<_, i64>(0))
            .unwrap_or(0)
            > 0;
        let has_is_summary_message = conn
            .query_row(CHECK_SUMMARY_COLUMN, [], |row| row.get::<_, i64>(0))
            .unwrap_or(0)
            > 0;
        let has_todos_column = conn
            .query_row(CHECK_TODOS_COLUMN, [], |row| row.get::<_, i64>(0))
            .unwrap_or(0)
            > 0;
        let has_read_files_table = conn
            .query_row(CHECK_READ_FILES_TABLE, [], |row| row.get::<_, i64>(0))
            .unwrap_or(0)
            > 0;

        Ok(SchemaFeatures { has_provider_column, has_is_summary_message, has_todos_column, has_read_files_table })
    }

    /// Get a session by ID from the database
    fn get_session(
        &self, conn: &Connection, session_id: &str, features: &SchemaFeatures,
    ) -> SqliteResult<CrushSession> {
        let query = if features.has_todos_column { GET_SESSION_WITH_TODOS } else { GET_SESSION_WITHOUT_TODOS };

        conn.query_row(query, [session_id], |row| {
            Ok(CrushSession {
                id: row.get(0)?,
                parent_session_id: row.get(1)?,
                title: row.get(2)?,
                message_count: row.get(3)?,
                prompt_tokens: row.get(4)?,
                completion_tokens: row.get(5)?,
                cost: row.get(6)?,
                updated_at: row.get(7)?,
                created_at: row.get(8)?,
                summary_message_id: row.get(9)?,
                todos: row.get(10)?,
            })
        })
    }

    /// Get all messages for a session
    fn get_session_messages(
        &self, conn: &Connection, session_id: &str, features: &SchemaFeatures,
    ) -> SqliteResult<Vec<CrushMessage>> {
        let query = match (features.has_provider_column, features.has_is_summary_message) {
            (true, true) => GET_MESSAGES_FULL,
            (true, false) => GET_MESSAGES_NO_SUMMARY_FLAG,
            (false, true) => GET_MESSAGES_NO_PROVIDER,
            (false, false) => GET_MESSAGES_MINIMAL,
        };

        let mut stmt = conn.prepare(query)?;
        let messages = stmt.query_map([session_id], |row| {
            Ok(CrushMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                parts: row.get(3)?,
                model: row.get(4)?,
                provider: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                finished_at: row.get(8)?,
                is_summary_message: row.get(9)?,
            })
        })?;

        messages.collect()
    }

    /// Get read files for a session (if read_files table exists)
    fn get_read_files(&self, conn: &Connection, session_id: &str, features: &SchemaFeatures) -> Vec<CrushReadFile> {
        if !features.has_read_files_table {
            return Vec::new();
        }

        match conn.prepare(GET_READ_FILES) {
            Ok(mut stmt) => stmt
                .query_map([session_id], |row| {
                    Ok(CrushReadFile { session_id: row.get(0)?, path: row.get(1)?, read_at: row.get(2)? })
                })
                .and_then(|rows| rows.collect())
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    /// Convert a Crush message to a canonical Event
    fn message_to_event(&self, msg: CrushMessage, _features: &SchemaFeatures) -> Option<Event> {
        if msg.is_summary_message > 0 {
            return None;
        }

        let timestamp = timestamp_to_datetime(msg.created_at);

        let parts: Vec<ContentPart> = serde_json::from_str(&msg.parts).unwrap_or_default();

        let role = match msg.role.as_str() {
            "user" => Some(Role::User),
            "assistant" => Some(Role::Assistant),
            "system" => Some(Role::System),
            _ => None,
        };

        let (kind, content) = self.extract_content_from_parts(&parts, &msg.role);

        let raw_payload = serde_json::json!({
            "id": msg.id,
            "session_id": msg.session_id,
            "role": msg.role,
            "parts": msg.parts,
            "model": msg.model,
            "provider": msg.provider,
            "created_at": msg.created_at,
            "updated_at": msg.updated_at,
            "finished_at": msg.finished_at,
            "is_summary_message": msg.is_summary_message,
        });

        Some(Event { id: Uuid::new_v4(), session_id: Uuid::nil(), kind, role, content, timestamp, raw_payload })
    }

    /// Extract content and event kind from message parts
    fn extract_content_from_parts(&self, parts: &[ContentPart], role: &str) -> (EventKind, Option<String>) {
        let mut content_parts = Vec::new();
        let mut tool_calls = Vec::new();

        for part in parts {
            match part {
                ContentPart::Text { data } => {
                    content_parts.push(data.text.clone());
                }
                ContentPart::Thinking { data } => {
                    content_parts.push(format!("[Thinking: {}]", data.thinking));
                }
                ContentPart::ToolUse { data } => {
                    tool_calls.push(data.name.clone());
                    content_parts.push(format!("[Tool: {}]", data.name));
                }
                ContentPart::ToolResult { data } => {
                    let prefix = if data.is_error.unwrap_or(false) { "[Error]" } else { "[Result]" };
                    content_parts.push(format!("{} {}", prefix, data.content));
                }
                ContentPart::Image { .. } => {
                    content_parts.push("[Image]".to_string());
                }
                ContentPart::Finish { data } => {
                    content_parts.push(format!("[Finished: {}]", data.reason));
                }
                ContentPart::Other => {}
            }
        }

        let kind = if role == "assistant" && !tool_calls.is_empty() { EventKind::ToolCall } else { EventKind::Message };

        let content = if content_parts.is_empty() { None } else { Some(content_parts.join("\n")) };

        (kind, content)
    }
}

impl Default for CrushAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert Crush Unix timestamp (milliseconds) to DateTime<Utc>
fn timestamp_to_datetime(ts_millis: i64) -> DateTime<Utc> {
    let secs = if ts_millis > 1_000_000_000_000 { ts_millis / 1000 } else { ts_millis };
    Utc.timestamp_opt(secs, 0).single().unwrap_or_else(Utc::now)
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_crush_adapter_new() {
        let adapter = CrushAdapter::new();
        let path_str = adapter.db_path().to_string_lossy();
        assert!(path_str.contains(".crush") || path_str.contains("crush.db"));
    }

    #[test]
    fn test_timestamp_conversion() {
        let ts = 1704067200;
        let dt = timestamp_to_datetime(ts);
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
    }

    #[test]
    fn test_extract_content_from_text() {
        let adapter = CrushAdapter::new();
        let parts = vec![ContentPart::Text { data: TextData { text: "Hello, world!".to_string() } }];
        let (kind, content) = adapter.extract_content_from_parts(&parts, "user");
        assert_eq!(kind, EventKind::Message);
        assert_eq!(content, Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_extract_content_from_tool_use() {
        let adapter = CrushAdapter::new();
        let parts = vec![
            ContentPart::Text { data: TextData { text: "Let me check that file.".to_string() } },
            ContentPart::ToolUse {
                data: ToolUseData {
                    id: "call_123".to_string(),
                    name: "read_file".to_string(),
                    input: serde_json::json!({"file_path": "/test/file.txt"}),
                },
            },
        ];
        let (kind, content) = adapter.extract_content_from_parts(&parts, "assistant");
        assert_eq!(kind, EventKind::ToolCall);
        assert!(content.is_some());
        assert!(content.unwrap().contains("read_file"));
    }

    #[test]
    fn test_content_part_serialization() {
        let part = ContentPart::Text { data: TextData { text: "Test content".to_string() } };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("text"));
        assert!(json.contains("Test content"));
    }
}
