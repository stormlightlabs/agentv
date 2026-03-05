use agent_v_core::{Event, EventKind, Role, Session, Source};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use uuid::Uuid;

/// A discovered OpenCode session
#[derive(Debug, Clone)]
pub struct OpenCodeSession {
    pub id: String,
    pub title: String,
    pub directory: Option<String>,
    pub project_id: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

/// OpenCode session payload format reconstructed from DB rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeSessionStorage {
    id: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    version: String,
    #[serde(rename = "projectID")]
    project_id: Option<String>,
    directory: Option<String>,
    title: String,
    time: OpenCodeTime,
    #[serde(default)]
    summary: OpenCodeSummary,
}

/// OpenCode message payload format reconstructed from DB rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeMessageStorage {
    id: String,
    #[serde(rename = "sessionID")]
    session_id: String,
    role: String,
    time: OpenCodeMessageTime,
    #[serde(default)]
    summary: Option<OpenCodeMessageSummary>,
    #[serde(default)]
    model: Option<OpenCodeModel>,
    #[serde(rename = "providerID")]
    #[serde(default)]
    provider_id: Option<String>,
    #[serde(default)]
    agent: Option<String>,
    #[serde(rename = "parentID")]
    #[serde(default)]
    parent_id: Option<String>,
}

/// OpenCode part payload format reconstructed from DB rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodePartStorage {
    id: String,
    #[serde(rename = "sessionID")]
    session_id: String,
    #[serde(rename = "messageID")]
    message_id: String,
    #[serde(rename = "type")]
    part_type: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    filename: Option<String>,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    state: Option<OpenCodePartState>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodePartState {
    #[serde(default)]
    status: String,
    #[serde(default)]
    input: Option<serde_json::Value>,
    #[serde(default)]
    output: Option<serde_json::Value>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeMessageSummary {
    #[serde(default)]
    title: String,
    #[serde(default)]
    diffs: Vec<OpenCodeDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeDiff {
    file: String,
    #[serde(default)]
    before: String,
    #[serde(default)]
    after: String,
    #[serde(default)]
    additions: u32,
    #[serde(default)]
    deletions: u32,
    #[serde(default)]
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeTime {
    created: i64,
    updated: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct OpenCodeSummary {
    #[serde(default)]
    additions: u32,
    #[serde(default)]
    deletions: u32,
    #[serde(default)]
    files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeMessageTime {
    created: i64,
    #[serde(default)]
    completed: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeModel {
    #[serde(rename = "providerID")]
    provider_id: String,
    #[serde(rename = "modelID")]
    model_id: String,
}

/// OpenCode auth.json format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeAuth {
    #[serde(flatten)]
    providers: HashMap<String, AuthProvider>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum AuthProvider {
    #[serde(rename = "api")]
    Api { key: String },
    #[serde(rename = "oauth")]
    Oauth {
        #[serde(default)]
        access: Option<String>,
        #[serde(default)]
        expires: Option<i64>,
    },
}

#[derive(Debug, Clone)]
struct OpenCodeDbSessionRow {
    id: String,
    slug: Option<String>,
    version: Option<String>,
    project_id: Option<String>,
    directory: Option<String>,
    title: String,
    time_created: i64,
    time_updated: i64,
    summary_additions: Option<u32>,
    summary_deletions: Option<u32>,
    summary_files: Option<u32>,
}

/// Adapter for OpenCode sessions sourced from OpenCode's SQLite database.
#[derive(Debug, Clone)]
pub struct OpenCodeAdapter {
    storage_path: PathBuf,
    auth_path: PathBuf,
    log_path: PathBuf,
    db_path: PathBuf,
}

impl OpenCodeAdapter {
    fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
        if !paths.iter().any(|p| p == &path) {
            paths.push(path);
        }
    }

    /// Candidate OpenCode base directories from troubleshooting docs.
    /// - macOS/Linux: `~/.local/share/opencode`
    /// - Windows: `%USERPROFILE%\\.local\\share\\opencode`
    ///
    /// Older installs may also use platform local data dirs.
    pub fn candidate_base_paths() -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        if let Some(home) = dirs::home_dir() {
            Self::push_unique_path(&mut candidates, home.join(".local/share/opencode"));
        }

        if let Some(local_data) = dirs::data_local_dir() {
            Self::push_unique_path(&mut candidates, local_data.join("opencode"));
        }

        if candidates.is_empty() {
            candidates.push(PathBuf::from("~/.local/share/opencode"));
        }

        candidates
    }

    fn default_base_path() -> PathBuf {
        let candidates = Self::candidate_base_paths();
        candidates
            .iter()
            .find(|path| path.join("storage").exists() || path.exists())
            .cloned()
            .or_else(|| candidates.first().cloned())
            .unwrap_or_else(|| PathBuf::from("~/.local/share/opencode"))
    }

    /// Create a new OpenCode adapter with default paths
    pub fn new() -> Self {
        let base_path = Self::default_base_path();
        let storage_path = base_path.join("storage");
        let auth_path = base_path.join("auth.json");
        let log_path = base_path.join("log");
        let db_path = base_path.join("opencode.db");

        Self { storage_path, auth_path, log_path, db_path }
    }

    /// Create a new OpenCode adapter with custom paths
    pub fn with_paths(storage_path: PathBuf, auth_path: PathBuf) -> Self {
        let base_path = auth_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("~/.local/share/opencode"));
        let log_path = base_path.join("log");
        let db_path = base_path.join("opencode.db");

        Self { storage_path, auth_path, log_path, db_path }
    }

    /// Get the storage path
    pub fn storage_path(&self) -> &PathBuf {
        &self.storage_path
    }

    /// Get the OpenCode auth path
    pub fn auth_path(&self) -> &PathBuf {
        &self.auth_path
    }

    /// Get the OpenCode log path
    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }

    /// Get the OpenCode SQLite DB path
    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }

    fn has_db(&self) -> bool {
        self.db_path.exists()
    }

    /// Check if OpenCode data is available
    pub fn is_available(&self) -> bool {
        self.has_db()
    }

    fn timestamp_from_millis(ts: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(ts / 1000, 0).unwrap_or_else(Utc::now)
    }

    fn build_message_from_db_row(
        id: String, session_id: String, created_ms: i64, data_json: String,
    ) -> Option<OpenCodeMessageStorage> {
        let mut value: serde_json::Value = serde_json::from_str(&data_json).ok()?;
        let obj = value.as_object_mut()?;
        obj.insert("id".to_string(), serde_json::Value::String(id.clone()));
        obj.insert("sessionID".to_string(), serde_json::Value::String(session_id.clone()));

        if !obj.contains_key("time") {
            obj.insert(
                "time".to_string(),
                serde_json::json!({
                    "created": created_ms,
                }),
            );
        }

        if !obj.contains_key("model")
            && let (Some(provider_id), Some(model_id)) = (
                obj.get("providerID").and_then(|v| v.as_str()),
                obj.get("modelID").and_then(|v| v.as_str()),
            )
        {
            obj.insert(
                "model".to_string(),
                serde_json::json!({
                    "providerID": provider_id,
                    "modelID": model_id,
                }),
            );
        }

        serde_json::from_value(value).ok()
    }

    fn build_part_from_db_row(
        id: String, session_id: String, message_id: String, data_json: String,
    ) -> Option<OpenCodePartStorage> {
        let mut value: serde_json::Value = serde_json::from_str(&data_json).ok()?;
        let obj = value.as_object_mut()?;
        obj.insert("id".to_string(), serde_json::Value::String(id));
        obj.insert("sessionID".to_string(), serde_json::Value::String(session_id));
        obj.insert("messageID".to_string(), serde_json::Value::String(message_id));
        serde_json::from_value(value).ok()
    }

    async fn discover_sessions_from_db(&self) -> Vec<OpenCodeSession> {
        if !self.has_db() {
            return Vec::new();
        }

        let db_path = self.db_path.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<Vec<OpenCodeSession>, String> {
            let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| format!("open opencode db: {e}"))?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, title, directory, project_id, time_created, time_updated \
                     FROM session ORDER BY time_created ASC",
                )
                .map_err(|e| format!("prepare session query: {e}"))?;

            let rows = stmt
                .query_map([], |row| {
                    let created_ms: i64 = row.get(4)?;
                    let updated_ms: i64 = row.get(5)?;
                    let created = Self::timestamp_from_millis(created_ms);
                    let updated = Self::timestamp_from_millis(updated_ms);
                    Ok(OpenCodeSession {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        directory: row.get(2)?,
                        project_id: row.get(3)?,
                        created,
                        updated,
                    })
                })
                .map_err(|e| format!("query sessions: {e}"))?;

            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("collect sessions: {e}"))
        })
        .await;

        match result {
            Ok(Ok(sessions)) => sessions,
            Ok(Err(e)) => {
                log::warn!("Failed to discover OpenCode sessions from db: {e}");
                Vec::new()
            }
            Err(e) => {
                log::warn!("OpenCode db discover task failed: {e}");
                Vec::new()
            }
        }
    }

    /// Discover all OpenCode sessions from the SQLite DB.
    pub async fn discover_sessions(&self) -> Vec<OpenCodeSession> {
        if !self.has_db() {
            log::debug!("OpenCode db not found at {:?}", self.db_path);
            return Vec::new();
        }

        let sessions = self.discover_sessions_from_db().await;
        log::info!("Discovered {} OpenCode sessions from db", sessions.len());
        sessions
    }

    /// Parse a session from OpenCode DB rows.
    pub async fn parse_session(
        &self, session: &OpenCodeSession,
    ) -> Result<(Session, Vec<Event>), Box<dyn std::error::Error + Send + Sync>> {
        if !self.has_db() {
            return Err(format!("OpenCode db not found at {:?}", self.db_path).into());
        }
        self.parse_session_from_db(session).await
    }

    async fn load_db_session_row(
        &self, session_id: &str,
    ) -> Result<Option<OpenCodeDbSessionRow>, Box<dyn std::error::Error + Send + Sync>> {
        let db_path = self.db_path.clone();
        let sid = session_id.to_string();
        let row = tokio::task::spawn_blocking(move || -> Result<Option<OpenCodeDbSessionRow>, String> {
            let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| format!("open opencode db: {e}"))?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, slug, version, project_id, directory, title, time_created, time_updated, \
                     summary_additions, summary_deletions, summary_files \
                     FROM session WHERE id = ?1 LIMIT 1",
                )
                .map_err(|e| format!("prepare session row query: {e}"))?;

            stmt.query_row([sid], |r| {
                Ok(OpenCodeDbSessionRow {
                    id: r.get(0)?,
                    slug: r.get(1)?,
                    version: r.get(2)?,
                    project_id: r.get(3)?,
                    directory: r.get(4)?,
                    title: r.get(5)?,
                    time_created: r.get(6)?,
                    time_updated: r.get(7)?,
                    summary_additions: r.get(8)?,
                    summary_deletions: r.get(9)?,
                    summary_files: r.get(10)?,
                })
            })
            .optional()
            .map_err(|e| format!("query session row: {e}"))
        })
        .await
        .map_err(|e| format!("OpenCode db session row task failed: {e}"))?
        .map_err(|e| format!("OpenCode db session row query failed: {e}"))?;

        Ok(row)
    }

    async fn parse_session_from_db(
        &self, session: &OpenCodeSession,
    ) -> Result<(Session, Vec<Event>), Box<dyn std::error::Error + Send + Sync>> {
        let row = self
            .load_db_session_row(&session.id)
            .await?
            .ok_or_else(|| format!("Session not found in opencode db: {}", session.id))?;

        let created_at = Self::timestamp_from_millis(row.time_created);
        let updated_at = Self::timestamp_from_millis(row.time_updated);

        let summary = OpenCodeSummary {
            additions: row.summary_additions.unwrap_or(0),
            deletions: row.summary_deletions.unwrap_or(0),
            files: row.summary_files.unwrap_or(0),
        };

        let raw_session = OpenCodeSessionStorage {
            id: row.id.clone(),
            slug: row.slug.unwrap_or_default(),
            version: row.version.unwrap_or_default(),
            project_id: row.project_id.clone(),
            directory: row.directory.clone(),
            title: row.title.clone(),
            time: OpenCodeTime { created: row.time_created, updated: row.time_updated },
            summary,
        };

        let session_obj = Session {
            id: Uuid::new_v4(),
            source: Source::OpenCode,
            external_id: row.id.clone(),
            project: row.directory.clone(),
            title: Some(row.title.clone()),
            created_at,
            updated_at,
            raw_payload: serde_json::to_value(&raw_session)?,
        };

        let messages = self.load_session_messages(&session.id).await?;
        let mut events = Vec::new();

        for message in messages {
            let timestamp = Self::timestamp_from_millis(message.time.created);
            let role = match message.role.as_str() {
                "user" => Some(Role::User),
                "assistant" => Some(Role::Assistant),
                "system" => Some(Role::System),
                _ => None,
            };
            let event_kind = if role.is_some() { EventKind::Message } else { EventKind::System };
            let parts = self.load_message_parts(&message.id).await.unwrap_or_default();
            let content = self.format_message_content(&parts, &message);

            events.push(Event {
                id: Uuid::new_v4(),
                session_id: session_obj.id,
                kind: event_kind,
                role,
                content: Some(content),
                timestamp,
                raw_payload: serde_json::to_value(&message)?,
            });

            for part in &parts {
                if part.part_type == "tool" {
                    let tool_content = part.state.as_ref().map(|state| {
                        serde_json::to_string(&serde_json::json!({
                            "tool": part.tool,
                            "status": state.status,
                            "input": state.input,
                            "output": state.output,
                            "metadata": state.metadata,
                        }))
                        .unwrap_or_default()
                    });

                    events.push(Event {
                        id: Uuid::new_v4(),
                        session_id: session_obj.id,
                        kind: EventKind::ToolCall,
                        role: Some(Role::Assistant),
                        content: tool_content.or_else(|| part.tool.clone()),
                        timestamp,
                        raw_payload: serde_json::to_value(part)?,
                    });
                }
            }
        }

        let session_diffs = self.load_session_diffs(&session.id).await.unwrap_or_default();
        if let Some(mut diff_event) = self.build_session_diff_event(updated_at, &session_diffs) {
            diff_event.session_id = session_obj.id;
            events.push(diff_event);
        }

        log::info!(
            "Parsed session {} with {} events from db",
            session_obj.external_id,
            events.len()
        );

        Ok((session_obj, events))
    }

    /// Load all messages for a session
    async fn load_session_messages(
        &self, session_id: &str,
    ) -> Result<Vec<OpenCodeMessageStorage>, Box<dyn std::error::Error + Send + Sync>> {
        let db_path = self.db_path.clone();
        let sid = session_id.to_string();
        let messages = tokio::task::spawn_blocking(move || -> Result<Vec<OpenCodeMessageStorage>, String> {
            let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| format!("open opencode db: {e}"))?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, session_id, time_created, data \
                     FROM message WHERE session_id = ?1 ORDER BY time_created ASC",
                )
                .map_err(|e| format!("prepare message query: {e}"))?;

            let rows = stmt
                .query_map([sid], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(|e| format!("query messages: {e}"))?;

            let mut out = Vec::new();
            for row in rows {
                let (id, session_id, created_ms, data_json) = row.map_err(|e| format!("read message row: {e}"))?;
                if let Some(message) = Self::build_message_from_db_row(id, session_id, created_ms, data_json) {
                    out.push(message);
                }
            }
            Ok(out)
        })
        .await
        .map_err(|e| format!("OpenCode db message task failed: {e}"))?
        .map_err(|e| format!("OpenCode db message query failed: {e}"))?;

        Ok(messages)
    }

    /// Load all parts for a message
    async fn load_message_parts(
        &self, message_id: &str,
    ) -> Result<Vec<OpenCodePartStorage>, Box<dyn std::error::Error + Send + Sync>> {
        let db_path = self.db_path.clone();
        let mid = message_id.to_string();
        let parts = tokio::task::spawn_blocking(move || -> Result<Vec<OpenCodePartStorage>, String> {
            let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| format!("open opencode db: {e}"))?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, message_id, session_id, data \
                     FROM part WHERE message_id = ?1 ORDER BY time_created ASC",
                )
                .map_err(|e| format!("prepare part query: {e}"))?;

            let rows = stmt
                .query_map([mid], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(|e| format!("query parts: {e}"))?;

            let mut out = Vec::new();
            for row in rows {
                let (id, message_id, session_id, data_json) = row.map_err(|e| format!("read part row: {e}"))?;
                if let Some(part) = Self::build_part_from_db_row(id, session_id, message_id, data_json) {
                    out.push(part);
                }
            }
            Ok(out)
        })
        .await
        .map_err(|e| format!("OpenCode db part task failed: {e}"))?
        .map_err(|e| format!("OpenCode db part query failed: {e}"))?;

        Ok(parts)
    }

    /// Load session-level diffs from `session.summary_diffs` in OpenCode DB.
    async fn load_session_diffs(
        &self, session_id: &str,
    ) -> Result<Vec<OpenCodeDiff>, Box<dyn std::error::Error + Send + Sync>> {
        if !self.has_db() {
            return Ok(Vec::new());
        }

        let db_path = self.db_path.clone();
        let sid = session_id.to_string();
        let diffs = tokio::task::spawn_blocking(move || -> Result<Vec<OpenCodeDiff>, String> {
            let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| format!("open opencode db: {e}"))?;
            let mut stmt = conn
                .prepare("SELECT summary_diffs FROM session WHERE id = ?1 LIMIT 1")
                .map_err(|e| format!("prepare diff query: {e}"))?;

            let summary: Option<String> = stmt
                .query_row([sid], |row| row.get::<_, Option<String>>(0))
                .optional()
                .map_err(|e| format!("query summary_diffs: {e}"))?
                .flatten();

            let Some(summary) = summary else {
                return Ok(Vec::new());
            };

            if summary.trim().is_empty() || summary.trim() == "[]" {
                return Ok(Vec::new());
            }

            Ok(serde_json::from_str(&summary).unwrap_or_default())
        })
        .await
        .map_err(|e| format!("OpenCode db diff task failed for {session_id}: {e}"))?
        .map_err(|e| format!("OpenCode db diff query failed for {session_id}: {e}"))?;

        Ok(diffs)
    }

    fn format_diff_status(status: &str) -> &str {
        match status {
            "added" => "A",
            "removed" => "D",
            "deleted" => "D",
            "modified" => "M",
            "renamed" => "R",
            _ => "?",
        }
    }

    fn format_session_diff_content(&self, diffs: &[OpenCodeDiff]) -> String {
        if diffs.is_empty() {
            return "Session diffs: no file changes captured".to_string();
        }

        let total_additions: u32 = diffs.iter().map(|d| d.additions).sum();
        let total_deletions: u32 = diffs.iter().map(|d| d.deletions).sum();

        let mut lines = vec![format!(
            "Session diffs: {} files changed (+{} -{})",
            diffs.len(),
            total_additions,
            total_deletions
        )];

        for diff in diffs.iter().take(20) {
            lines.push(format!(
                "{} {} (+{} -{})",
                Self::format_diff_status(&diff.status),
                diff.file,
                diff.additions,
                diff.deletions
            ));
        }

        if diffs.len() > 20 {
            lines.push(format!("... {} more files", diffs.len() - 20));
        }

        lines.join("\n")
    }

    fn build_session_diff_event(&self, timestamp: DateTime<Utc>, diffs: &[OpenCodeDiff]) -> Option<Event> {
        if diffs.is_empty() {
            return None;
        }

        Some(Event {
            id: Uuid::new_v4(),
            session_id: Uuid::nil(),
            kind: EventKind::System,
            role: None,
            content: Some(self.format_session_diff_content(diffs)),
            timestamp,
            raw_payload: serde_json::to_value(diffs).unwrap_or_default(),
        })
    }

    async fn session_diff_signature(&self, session_id: &str) -> Option<String> {
        if !self.has_db() {
            return None;
        }

        let db_path = self.db_path.clone();
        let sid = session_id.to_string();
        tokio::task::spawn_blocking(move || -> Result<Option<String>, String> {
            let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| format!("open opencode db: {e}"))?;
            let mut stmt = conn
                .prepare("SELECT time_updated, summary_diffs FROM session WHERE id = ?1 LIMIT 1")
                .map_err(|e| format!("prepare diff signature query: {e}"))?;

            let row: Option<(i64, Option<String>)> = stmt
                .query_row([sid], |r| Ok((r.get(0)?, r.get(1)?)))
                .optional()
                .map_err(|e| format!("query diff signature: {e}"))?;

            let Some((updated_ms, diffs)) = row else {
                return Ok(None);
            };

            let diff_text = diffs.unwrap_or_default();
            if diff_text.trim().is_empty() || diff_text.trim() == "[]" {
                return Ok(None);
            }

            Ok(Some(format!("diff:{}:{}", diff_text.len(), updated_ms)))
        })
        .await
        .ok()
        .and_then(Result::ok)
        .flatten()
    }

    fn message_file_key(file_name: &str) -> String {
        format!("msg:{}", file_name)
    }

    fn is_diff_key(key: &str) -> bool {
        key.starts_with("diff:")
    }

    /// Build incremental cursor keys for a session.
    /// Keys are prefixed (`msg:`/`diff:`) to avoid collisions.
    pub async fn collect_incremental_known_files(&self, session_id: &str) -> HashSet<String> {
        let mut known = HashSet::new();
        if self.has_db() {
            let db_path = self.db_path.clone();
            let sid = session_id.to_string();
            if let Ok(Ok(keys)) = tokio::task::spawn_blocking(move || -> Result<Vec<String>, String> {
                let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .map_err(|e| format!("open opencode db: {e}"))?;
                let mut stmt = conn
                    .prepare("SELECT id FROM message WHERE session_id = ?1")
                    .map_err(|e| format!("prepare known message query: {e}"))?;
                let rows = stmt
                    .query_map([sid], |row| row.get::<_, String>(0))
                    .map_err(|e| format!("query known messages: {e}"))?;
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(|e| format!("collect known messages: {e}"))
            })
            .await
            {
                for key in keys {
                    known.insert(Self::message_file_key(&key));
                }
            }
        }

        if let Some(diff_signature) = self.session_diff_signature(session_id).await {
            known.insert(diff_signature);
        }

        known
    }

    /// Format message content from parts
    fn format_message_content(&self, parts: &[OpenCodePartStorage], message: &OpenCodeMessageStorage) -> String {
        let mut content_parts = Vec::new();

        for part in parts {
            match part.part_type.as_str() {
                "text" => {
                    if let Some(text) = &part.text {
                        content_parts.push(text.clone());
                    }
                }
                "file" => {
                    if let Some(filename) = &part.filename {
                        content_parts.push(format!("[📎 {}]", filename));
                    }
                }
                "tool" => {
                    if let Some(tool) = &part.tool {
                        if let Some(state) = &part.state {
                            content_parts.push(format!("🔧 {} (status: {})", tool, state.status));
                        } else {
                            content_parts.push(format!("🔧 {}", tool));
                        }
                    }
                }
                _ => {}
            }
        }

        if content_parts.is_empty()
            && let Some(summary) = &message.summary
            && !summary.title.is_empty()
        {
            content_parts.push(summary.title.clone());
        }

        content_parts.join("\n\n")
    }

    /// Parse only new events from a session by tracking known message keys.
    /// Returns new events and the updated key set.
    pub async fn parse_session_incremental(
        &self, session: &OpenCodeSession, known_files: &HashSet<String>,
    ) -> Result<(Vec<Event>, HashSet<String>), Box<dyn std::error::Error + Send + Sync>> {
        let mut new_known = known_files.clone();
        let mut new_events = Vec::new();

        let messages = self.load_session_messages(&session.id).await.unwrap_or_default();
        for message in messages {
            let file_key = Self::message_file_key(&message.id);
            if known_files.contains(&file_key) {
                continue;
            }

            new_known.insert(file_key);

            let timestamp = Self::timestamp_from_millis(message.time.created);
            let role = match message.role.as_str() {
                "user" => Some(Role::User),
                "assistant" => Some(Role::Assistant),
                "system" => Some(Role::System),
                _ => None,
            };
            let event_kind = if role.is_some() { EventKind::Message } else { EventKind::System };
            let parts = self.load_message_parts(&message.id).await.unwrap_or_default();
            let content_str = self.format_message_content(&parts, &message);

            new_events.push(Event {
                id: Uuid::new_v4(),
                session_id: Uuid::nil(),
                kind: event_kind,
                role,
                content: Some(content_str),
                timestamp,
                raw_payload: serde_json::to_value(&message).unwrap_or_default(),
            });

            for part in &parts {
                if part.part_type == "tool" {
                    let tool_content = part.state.as_ref().map(|state| {
                        serde_json::to_string(&serde_json::json!({
                            "tool": part.tool,
                            "status": state.status,
                            "input": state.input,
                            "output": state.output,
                        }))
                        .unwrap_or_default()
                    });

                    new_events.push(Event {
                        id: Uuid::new_v4(),
                        session_id: Uuid::nil(),
                        kind: EventKind::ToolCall,
                        role: Some(Role::Assistant),
                        content: tool_content.or_else(|| part.tool.clone()),
                        timestamp,
                        raw_payload: serde_json::to_value(part).unwrap_or_default(),
                    });
                }
            }
        }

        if let Some(diff_signature) = self.session_diff_signature(&session.id).await {
            let known_diff = known_files.iter().any(|key| Self::is_diff_key(key));
            let diff_changed = !known_files.contains(&diff_signature) || !known_diff;

            if diff_changed {
                let session_diffs = self.load_session_diffs(&session.id).await.unwrap_or_default();
                if let Some(diff_event) = self.build_session_diff_event(session.updated, &session_diffs) {
                    new_events.push(diff_event);
                }
            }

            new_known.retain(|key| !Self::is_diff_key(key));
            new_known.insert(diff_signature);
        } else {
            new_known.retain(|key| !Self::is_diff_key(key));
        }

        Ok((new_events, new_known))
    }

    /// Get provider information from auth.json
    pub fn get_providers(&self) -> Vec<String> {
        if !self.auth_path.exists() {
            return Vec::new();
        }

        let content = match std::fs::read_to_string(&self.auth_path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let auth: OpenCodeAuth = match serde_json::from_str(&content) {
            Ok(a) => a,
            Err(_) => return Vec::new(),
        };

        auth.providers.keys().cloned().collect()
    }
}

impl Default for OpenCodeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_opencode_adapter_new() {
        let adapter = OpenCodeAdapter::new();
        assert!(
            adapter
                .storage_path
                .to_string_lossy()
                .contains(".local/share/opencode/storage")
        );
        assert!(adapter.auth_path.to_string_lossy().contains(".local/share/opencode"));
        assert!(
            adapter
                .db_path
                .to_string_lossy()
                .contains(".local/share/opencode/opencode.db")
        );
    }

    #[test]
    fn test_timestamp_conversion() {
        let ts_millis = 1704067200000i64;
        let ts_seconds = ts_millis / 1000;
        let dt = DateTime::from_timestamp(ts_seconds, 0).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
    }
}
