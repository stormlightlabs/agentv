use agent_v_core::{Event, EventKind, Role, Session, Source};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// OpenCode session storage format (from `~/.local/share/opencode/storage/session/`)
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

/// OpenCode message storage format (from `~/.local/share/opencode/storage/message/<session_id>/`)
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
    #[serde(default)]
    parent_id: Option<String>,
}

/// OpenCode part storage format (from `~/.local/share/opencode/storage/part/<message_id>/`)
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
    output: Option<String>,
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
    additions: u32,
    deletions: u32,
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

/// Adapter for OpenCode sessions
///
/// Discovers and parses sessions directly from OpenCode's storage directory
/// structure without requiring the CLI export command.
///
/// Storage structure:
/// - `~/.local/share/opencode/storage/session/<project_hash>/<session_id>.json`
/// - `~/.local/share/opencode/storage/message/<session_id>/<message_id>.json`
/// - `~/.local/share/opencode/storage/part/<message_id>/<part_id>.json`
#[derive(Debug, Clone)]
pub struct OpenCodeAdapter {
    storage_path: PathBuf,
    auth_path: PathBuf,
}

impl OpenCodeAdapter {
    /// Create a new OpenCode adapter with default paths
    pub fn new() -> Self {
        let storage_path = dirs::home_dir()
            .map(|h| h.join(".local/share/opencode/storage"))
            .unwrap_or_else(|| PathBuf::from("~/.local/share/opencode/storage"));

        let auth_path = dirs::home_dir()
            .map(|h| h.join(".local/share/opencode/auth.json"))
            .unwrap_or_else(|| PathBuf::from("~/.local/share/opencode/auth.json"));

        Self { storage_path, auth_path }
    }

    /// Create a new OpenCode adapter with custom paths
    pub fn with_paths(storage_path: PathBuf, auth_path: PathBuf) -> Self {
        Self { storage_path, auth_path }
    }

    /// Get the storage path
    pub fn storage_path(&self) -> &PathBuf {
        &self.storage_path
    }

    /// Check if storage directory exists (OpenCode data available)
    pub fn is_available(&self) -> bool {
        self.storage_path.exists()
    }

    /// Discover all OpenCode sessions from storage
    pub async fn discover_sessions(&self) -> Vec<OpenCodeSession> {
        if !self.is_available() {
            tracing::debug!("OpenCode storage not found at {:?}", self.storage_path);
            return Vec::new();
        }

        let session_dir = self.storage_path.join("session");
        if !session_dir.exists() {
            return Vec::new();
        }

        let mut sessions = Vec::new();

        let Ok(project_dirs) = tokio::fs::read_dir(&session_dir).await else {
            return Vec::new();
        };

        let mut project_dirs = project_dirs;
        while let Ok(Some(project_entry)) = project_dirs.next_entry().await {
            if !project_entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }

            let Ok(session_files) = tokio::fs::read_dir(project_entry.path()).await else {
                continue;
            };

            let mut session_files = session_files;
            while let Ok(Some(session_entry)) = session_files.next_entry().await {
                let path = session_entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }

                match self.parse_session_file(&path).await {
                    Ok(session) => sessions.push(session),
                    Err(e) => tracing::warn!("Failed to parse session file {:?}: {}", path, e),
                }
            }
        }

        tracing::info!("Discovered {} OpenCode sessions", sessions.len());
        sessions
    }

    /// Parse a session file from storage
    async fn parse_session_file(
        &self, path: &PathBuf,
    ) -> Result<OpenCodeSession, Box<dyn std::error::Error + Send + Sync>> {
        let content = tokio::fs::read_to_string(path).await?;
        let data: OpenCodeSessionStorage = serde_json::from_str(&content)?;

        let created = DateTime::from_timestamp(data.time.created / 1000, 0).unwrap_or_else(Utc::now);
        let updated = DateTime::from_timestamp(data.time.updated / 1000, 0).unwrap_or(created);

        Ok(OpenCodeSession {
            id: data.id,
            title: data.title,
            directory: data.directory,
            project_id: data.project_id,
            created,
            updated,
        })
    }

    /// Parse a session from storage files
    pub async fn parse_session(
        &self, session: &OpenCodeSession,
    ) -> Result<(Session, Vec<Event>), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Parsing OpenCode session: {}", session.id);

        let session_path = self.find_session_file(&session.id).await?;
        let session_content = tokio::fs::read_to_string(&session_path).await?;
        let session_data: OpenCodeSessionStorage = serde_json::from_str(&session_content)?;

        let created_at = DateTime::from_timestamp(session_data.time.created / 1000, 0).unwrap_or_else(Utc::now);
        let updated_at = DateTime::from_timestamp(session_data.time.updated / 1000, 0).unwrap_or(created_at);

        let session_obj = Session {
            id: Uuid::new_v4(),
            source: Source::OpenCode,
            external_id: session_data.id.clone(),
            project: session_data.directory.clone(),
            title: Some(session_data.title.clone()),
            created_at,
            updated_at,
            raw_payload: serde_json::to_value(&session_data)?,
        };

        let messages = self.load_session_messages(&session.id).await?;
        let mut events = Vec::new();

        for message in messages {
            let timestamp = DateTime::from_timestamp(message.time.created / 1000, 0).unwrap_or_else(Utc::now);

            let role = match message.role.as_str() {
                "user" => Some(Role::User),
                "assistant" => Some(Role::Assistant),
                "system" => Some(Role::System),
                _ => None,
            };

            let event_kind = if role.is_some() { EventKind::Message } else { EventKind::System };

            let parts = self.load_message_parts(&message.id).await.unwrap_or_default();
            let content = self.format_message_content(&parts, &message);

            let event = Event {
                id: Uuid::new_v4(),
                session_id: session_obj.id,
                kind: event_kind,
                role,
                content: Some(content),
                timestamp,
                raw_payload: serde_json::to_value(&message)?,
            };

            events.push(event);

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

                    let tool_event = Event {
                        id: Uuid::new_v4(),
                        session_id: session_obj.id,
                        kind: EventKind::ToolCall,
                        role: Some(Role::Assistant),
                        content: tool_content.or_else(|| part.tool.clone()),
                        timestamp,
                        raw_payload: serde_json::to_value(part)?,
                    };

                    events.push(tool_event);
                }
            }
        }

        tracing::info!(
            "Parsed session {} with {} events",
            session_obj.external_id,
            events.len()
        );

        Ok((session_obj, events))
    }

    /// Find a session file by session ID
    async fn find_session_file(&self, session_id: &str) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let session_dir = self.storage_path.join("session");

        let Ok(project_dirs) = tokio::fs::read_dir(&session_dir).await else {
            return Err("Session directory not found".into());
        };

        let mut project_dirs = project_dirs;
        while let Ok(Some(project_entry)) = project_dirs.next_entry().await {
            if !project_entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }

            let session_path = project_entry.path().join(format!("{}.json", session_id));
            if session_path.exists() {
                return Ok(session_path);
            }
        }

        Err(format!("Session file not found for {}", session_id).into())
    }

    /// Load all messages for a session
    async fn load_session_messages(
        &self, session_id: &str,
    ) -> Result<Vec<OpenCodeMessageStorage>, Box<dyn std::error::Error + Send + Sync>> {
        let message_dir = self.storage_path.join("message").join(session_id);

        if !message_dir.exists() {
            return Ok(Vec::new());
        }

        let mut messages = Vec::new();

        let Ok(entries) = tokio::fs::read_dir(&message_dir).await else {
            return Ok(Vec::new());
        };

        let mut entries = entries;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            match tokio::fs::read_to_string(&path).await {
                Ok(content) => {
                    if let Ok(message) = serde_json::from_str::<OpenCodeMessageStorage>(&content) {
                        messages.push(message);
                    }
                }
                Err(e) => tracing::warn!("Failed to read message file {:?}: {}", path, e),
            }
        }

        messages.sort_by(|a, b| a.time.created.cmp(&b.time.created));

        Ok(messages)
    }

    /// Load all parts for a message
    async fn load_message_parts(
        &self, message_id: &str,
    ) -> Result<Vec<OpenCodePartStorage>, Box<dyn std::error::Error + Send + Sync>> {
        let part_dir = self.storage_path.join("part").join(message_id);

        if !part_dir.exists() {
            return Ok(Vec::new());
        }

        let mut parts = Vec::new();

        let Ok(entries) = tokio::fs::read_dir(&part_dir).await else {
            return Ok(Vec::new());
        };

        let mut entries = entries;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            match tokio::fs::read_to_string(&path).await {
                Ok(content) => {
                    if let Ok(part) = serde_json::from_str::<OpenCodePartStorage>(&content) {
                        parts.push(part);
                    }
                }
                Err(e) => tracing::warn!("Failed to read part file {:?}: {}", path, e),
            }
        }

        parts.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(parts)
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
