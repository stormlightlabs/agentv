use agent_viz_core::{Event, EventKind, Role, Session, Source};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// A discovered Claude Code session file
#[derive(Debug, Clone)]
pub struct ClaudeSessionFile {
    pub path: PathBuf,
    pub project: String,
    pub session_id: String,
}

/// Claude Code JSONL entry types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeEntry {
    Summary {
        summary: String,
        leaf_uuid: String,
    },
    Message {
        uuid: String,
        parent_uuid: Option<String>,
        timestamp: String,
        #[serde(flatten)]
        content: serde_json::Value,
    },
    #[serde(other)]
    Other,
}

/// Adapter for Claude Code sessions
#[derive(Debug, Clone)]
pub struct ClaudeAdapter {
    projects_dir: PathBuf,
}

impl ClaudeAdapter {
    /// Create a new Claude adapter with default projects directory
    pub fn new() -> Self {
        let projects_dir = dirs::home_dir()
            .map(|h| h.join(".claude").join("projects"))
            .unwrap_or_else(|| PathBuf::from("."));

        Self { projects_dir }
    }

    /// Create a new Claude adapter with a custom projects directory
    pub fn with_projects_dir(projects_dir: PathBuf) -> Self {
        Self { projects_dir }
    }

    /// Get the projects directory path
    pub fn projects_dir(&self) -> &PathBuf {
        &self.projects_dir
    }

    /// Discover all session files in the projects directory
    pub async fn discover_sessions(&self) -> Vec<ClaudeSessionFile> {
        let mut sessions = Vec::new();

        if !self.projects_dir.exists() {
            tracing::warn!("Claude projects directory not found: {:?}", self.projects_dir);
            return sessions;
        }

        let mut entries = match tokio::fs::read_dir(&self.projects_dir).await {
            Ok(e) => e,
            Err(e) => {
                tracing::error!("Failed to read projects directory: {}", e);
                return sessions;
            }
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let project_path = entry.path();
            if !project_path.is_dir() {
                continue;
            }

            let project_name = project_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let mut jsonl_files = match tokio::fs::read_dir(&project_path).await {
                Ok(f) => f,
                Err(e) => {
                    tracing::warn!("Failed to read project directory {:?}: {}", project_path, e);
                    continue;
                }
            };

            while let Ok(Some(file_entry)) = jsonl_files.next_entry().await {
                let file_path = file_entry.path();
                if file_path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                    let session_id = file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    sessions.push(ClaudeSessionFile { path: file_path, project: project_name.clone(), session_id });
                }
            }
        }

        tracing::info!("Discovered {} Claude Code sessions", sessions.len());
        sessions
    }

    /// Parse a session file and return a Session with its Events
    pub async fn parse_session(
        &self, session_file: &ClaudeSessionFile,
    ) -> Result<(Session, Vec<Event>), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Parsing session file: {:?}", session_file.path);

        let content = tokio::fs::read_to_string(&session_file.path).await?;
        let lines: Vec<&str> = content.lines().collect();

        let mut session_title = None;
        let mut events = Vec::new();
        let mut first_timestamp: Option<DateTime<Utc>> = None;
        let mut last_timestamp: Option<DateTime<Utc>> = None;

        for (idx, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Failed to parse line {} in {:?}: {}", idx, session_file.path, e);
                    continue;
                }
            };

            if let Some(ts_str) = value.get("timestamp").and_then(|t| t.as_str())
                && let Ok(ts) = DateTime::parse_from_rfc3339(ts_str)
            {
                let ts_utc = ts.with_timezone(&Utc);
                if first_timestamp.is_none() {
                    first_timestamp = Some(ts_utc);
                }
                last_timestamp = Some(ts_utc);
            }

            if value.get("type").and_then(|t| t.as_str()) == Some("summary") {
                if let Some(summary) = value.get("summary").and_then(|s| s.as_str()) {
                    session_title = Some(summary.to_string());
                }
                continue;
            }

            if let Some(event) = self.parse_event_line(&value, idx) {
                events.push(event);
            }
        }

        let created_at = first_timestamp.unwrap_or_else(Utc::now);
        let updated_at = last_timestamp.unwrap_or(created_at);

        let external_id = session_file.session_id.clone();

        let raw_payload = serde_json::json!({
            "source": "claude",
            "project": session_file.project,
            "session_id": external_id,
            "file_path": session_file.path.to_string_lossy().to_string(),
            "line_count": lines.len(),
        });

        let session = Session {
            id: Uuid::new_v4(),
            source: Source::Claude,
            external_id,
            project: Some(session_file.project.clone()),
            title: session_title,
            created_at,
            updated_at,
            raw_payload,
        };

        let events: Vec<Event> = events
            .into_iter()
            .map(|mut e| {
                e.session_id = session.id;
                e
            })
            .collect();

        tracing::info!("Parsed session {} with {} events", session.external_id, events.len());

        Ok((session, events))
    }

    /// Parse a single JSONL line into an Event
    fn parse_event_line(&self, value: &serde_json::Value, _line_idx: usize) -> Option<Event> {
        let entry_type = value.get("type")?.as_str()?;
        let _uuid = value
            .get("uuid")
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let timestamp = value
            .get("timestamp")
            .and_then(|t| t.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let (kind, role, content) = match entry_type {
            "user" => {
                let content = value.get("content").and_then(|c| c.as_str()).map(|s| s.to_string());
                (EventKind::Message, Some(Role::User), content)
            }
            "assistant" => {
                let content = value.get("content").and_then(|c| c.as_str()).map(|s| s.to_string());
                (EventKind::Message, Some(Role::Assistant), content)
            }
            "system" => {
                let content = value.get("content").and_then(|c| c.as_str()).map(|s| s.to_string());
                (EventKind::System, Some(Role::System), content)
            }
            "tool_call" => {
                let content = serde_json::to_string(&serde_json::json!({
                    "name": value.get("name"),
                    "arguments": value.get("arguments"),
                }))
                .ok();
                (EventKind::ToolCall, Some(Role::Assistant), content)
            }
            "tool_result" => {
                let content = value.get("content").and_then(|c| c.as_str()).map(|s| s.to_string());
                (EventKind::ToolResult, Some(Role::Assistant), content)
            }
            "error" => {
                let content = value.get("message").and_then(|m| m.as_str()).map(|s| s.to_string());
                (EventKind::Error, None, content)
            }
            _ => (EventKind::System, None, serde_json::to_string(value).ok()),
        };

        Some(Event {
            id: Uuid::new_v4(),
            session_id: Uuid::nil(),
            kind,
            role,
            content,
            timestamp,
            raw_payload: value.clone(),
        })
    }
}

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_adapter_new() {
        let adapter = ClaudeAdapter::new();
        assert!(adapter.projects_dir().to_string_lossy().contains(".claude/projects"));
    }

    #[test]
    fn test_parse_event_line_user_message() {
        let adapter = ClaudeAdapter::new();
        let value = serde_json::json!({
            "type": "user",
            "uuid": "test-uuid",
            "timestamp": "2024-01-01T00:00:00Z",
            "content": "Hello, world!"
        });

        let event = adapter.parse_event_line(&value, 0).unwrap();
        assert_eq!(event.kind, EventKind::Message);
        assert_eq!(event.role, Some(Role::User));
        assert_eq!(event.content, Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_parse_event_line_assistant_message() {
        let adapter = ClaudeAdapter::new();
        let value = serde_json::json!({
            "type": "assistant",
            "uuid": "test-uuid",
            "timestamp": "2024-01-01T00:00:00Z",
            "content": "How can I help?"
        });

        let event = adapter.parse_event_line(&value, 0).unwrap();
        assert_eq!(event.kind, EventKind::Message);
        assert_eq!(event.role, Some(Role::Assistant));
    }

    #[test]
    fn test_parse_event_line_system() {
        let adapter = ClaudeAdapter::new();
        let value = serde_json::json!({
            "type": "system",
            "uuid": "test-uuid",
            "timestamp": "2024-01-01T00:00:00Z",
            "content": "System message"
        });

        let event = adapter.parse_event_line(&value, 0).unwrap();
        assert_eq!(event.kind, EventKind::System);
        assert_eq!(event.role, Some(Role::System));
    }

    #[test]
    fn test_parse_event_line_error() {
        let adapter = ClaudeAdapter::new();
        let value = serde_json::json!({
            "type": "error",
            "uuid": "test-uuid",
            "timestamp": "2024-01-01T00:00:00Z",
            "message": "Something went wrong"
        });

        let event = adapter.parse_event_line(&value, 0).unwrap();
        assert_eq!(event.kind, EventKind::Error);
        assert_eq!(event.content, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_parse_event_line_unknown_type() {
        let adapter = ClaudeAdapter::new();
        let value = serde_json::json!({
            "type": "unknown_type",
            "uuid": "test-uuid",
            "timestamp": "2024-01-01T00:00:00Z",
            "data": "some data"
        });

        let event = adapter.parse_event_line(&value, 0).unwrap();
        assert_eq!(event.kind, EventKind::System);
        assert!(event.content.is_some());
    }
}
