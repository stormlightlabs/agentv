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

/// Represents a tool call extracted from assistant message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Represents a content block in an assistant message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
        signature: Option<String>,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(other)]
    Other,
}

/// Parsed Claude message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: Option<serde_json::Value>,
    pub model: Option<String>,
    pub usage: Option<serde_json::Value>,
    pub stop_reason: Option<String>,
}

/// Represents a conversation node with parent-child relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationNode {
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub content: Option<String>,
    pub role: Option<Role>,
    pub tool_calls: Vec<ToolCall>,
    pub thinking: Option<String>,
    pub git_branch: Option<String>,
    pub cwd: Option<String>,
    pub raw: serde_json::Value,
}

/// Represents a reconstructed conversation thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationThread {
    pub root_uuid: String,
    pub nodes: Vec<ConversationNode>,
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
        let mut leaf_uuid: Option<String> = None;

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
                if let Some(leaf) = value.get("leafUuid").and_then(|l| l.as_str()) {
                    leaf_uuid = Some(leaf.to_string());
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
            "leaf_uuid": leaf_uuid,
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

    /// Rebuild conversation threads from events
    pub fn rebuild_conversations(&self, events: &[Event]) -> Vec<ConversationThread> {
        let mut nodes: Vec<ConversationNode> = events.iter().filter_map(|event| self.event_to_node(event)).collect();

        nodes.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        let uuid_set: std::collections::HashSet<_> = nodes.iter().map(|n| &n.uuid).collect();
        let roots: Vec<_> = nodes
            .iter()
            .filter(|n| n.parent_uuid.is_none() || !uuid_set.contains(n.parent_uuid.as_ref().unwrap()))
            .map(|n| n.uuid.clone())
            .collect();

        roots
            .into_iter()
            .map(|root_uuid| ConversationThread { root_uuid: root_uuid.clone(), nodes: nodes.clone() })
            .collect()
    }

    /// Convert an Event to a ConversationNode
    fn event_to_node(&self, event: &Event) -> Option<ConversationNode> {
        let raw = &event.raw_payload;
        let uuid = raw.get("uuid")?.as_str()?.to_string();
        let parent_uuid = raw.get("parentUuid").and_then(|p| p.as_str()).map(|s| s.to_string());
        let event_type = raw.get("type")?.as_str()?.to_string();
        let timestamp = event.timestamp;
        let git_branch = raw.get("gitBranch").and_then(|g| g.as_str()).map(|s| s.to_string());
        let cwd = raw.get("cwd").and_then(|c| c.as_str()).map(|s| s.to_string());

        let (content, role, tool_calls, thinking) = match event_type.as_str() {
            "user" => {
                let content = self.extract_user_content(raw);
                let role = Some(Role::User);
                (content, role, Vec::new(), None)
            }
            "assistant" => {
                let (content, tool_calls, thinking) = self.extract_assistant_content(raw);
                let role = Some(Role::Assistant);
                (content, role, tool_calls, thinking)
            }
            "system" => {
                let content = raw.get("content").and_then(|c| c.as_str()).map(|s| s.to_string());
                let role = Some(Role::System);
                (content, role, Vec::new(), None)
            }
            _ => (event.content.clone(), event.role, Vec::new(), None),
        };

        Some(ConversationNode {
            uuid,
            parent_uuid,
            event_type,
            timestamp,
            content,
            role,
            tool_calls,
            thinking,
            git_branch,
            cwd,
            raw: raw.clone(),
        })
    }

    /// Extract content from a user message entry
    fn extract_user_content(&self, value: &serde_json::Value) -> Option<String> {
        if let Some(message) = value.get("message")
            && let Some(content) = message.get("content").and_then(|c| c.as_str())
        {
            return Some(content.to_string());
        }

        value.get("content").and_then(|c| c.as_str()).map(|s| s.to_string())
    }

    /// Extract content, tool calls, and thinking from an assistant message entry
    fn extract_assistant_content(&self, value: &serde_json::Value) -> (Option<String>, Vec<ToolCall>, Option<String>) {
        let mut content_parts = Vec::new();
        let mut tool_calls = Vec::new();
        let mut thinking = None;

        if let Some(message) = value.get("message") {
            if let Some(content_array) = message.get("content").and_then(|c| c.as_array()) {
                for block in content_array {
                    match block.get("type").and_then(|t| t.as_str()) {
                        Some("text") => {
                            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                content_parts.push(text.to_string());
                            }
                        }
                        Some("thinking") => {
                            if let Some(t) = block.get("thinking").and_then(|t| t.as_str()) {
                                thinking = Some(t.to_string());
                            }
                        }
                        Some("tool_use") => {
                            if let (Some(id), Some(name)) = (
                                block.get("id").and_then(|i| i.as_str()),
                                block.get("name").and_then(|n| n.as_str()),
                            ) {
                                let input = block.get("input").cloned().unwrap_or(serde_json::Value::Null);
                                tool_calls.push(ToolCall {
                                    id: id.to_string(),
                                    name: name.to_string(),
                                    arguments: input,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            } else if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                content_parts.push(content.to_string());
            }
        }

        let content = if content_parts.is_empty() { None } else { Some(content_parts.join("\n")) };

        (content, tool_calls, thinking)
    }

    /// Parse a single JSONL line into an Event
    fn parse_event_line(&self, value: &serde_json::Value, _line_idx: usize) -> Option<Event> {
        let entry_type = value.get("type")?.as_str()?;

        let timestamp = value
            .get("timestamp")
            .and_then(|t| t.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let (kind, role, content) = match entry_type {
            "user" => {
                let content = self.extract_user_content(value);
                (EventKind::Message, Some(Role::User), content)
            }
            "assistant" => {
                let (content, _, _) = self.extract_assistant_content(value);
                (EventKind::Message, Some(Role::Assistant), content)
            }
            "system" => {
                let content = value.get("content").and_then(|c| c.as_str()).map(|s| s.to_string());
                (EventKind::System, Some(Role::System), content)
            }
            "progress" => {
                let content = value
                    .get("data")
                    .and_then(|d| d.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| value.get("data").map(|d| d.to_string()));
                (EventKind::System, None, content)
            }
            "file-history-snapshot" => {
                let content = value.get("snapshot").map(|s| s.to_string());
                (EventKind::System, None, content)
            }
            "queue-operation" => {
                let content = value.get("content").and_then(|c| c.as_str()).map(|s| s.to_string());
                (EventKind::System, None, content)
            }
            "error" => {
                let content = value.get("message").and_then(|m| m.as_str()).map(|s| s.to_string());
                (EventKind::Error, None, content)
            }
            _ => (EventKind::System, None, Some(format!("Unknown type: {}", entry_type))),
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

    /// Get statistics about a session file
    pub async fn get_session_stats(
        &self, session_file: &ClaudeSessionFile,
    ) -> Result<SessionStats, Box<dyn std::error::Error + Send + Sync>> {
        let content = tokio::fs::read_to_string(&session_file.path).await?;
        let lines: Vec<&str> = content.lines().collect();

        let mut stats = SessionStats { total_lines: lines.len(), ..SessionStats::default() };

        for line in &lines {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(entry_type) = value.get("type").and_then(|t| t.as_str()) {
                    match entry_type {
                        "user" => stats.user_messages += 1,
                        "assistant" => {
                            stats.assistant_messages += 1;
                            if let Some(message) = value.get("message")
                                && let Some(content_array) = message.get("content").and_then(|c| c.as_array())
                            {
                                for block in content_array {
                                    if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                                        stats.tool_calls += 1;
                                    }
                                }
                            }
                        }
                        "system" => stats.system_messages += 1,
                        "progress" => stats.progress_messages += 1,
                        "file-history-snapshot" => stats.file_snapshots += 1,
                        _ => {}
                    }
                }

                if let Some(branch) = value.get("gitBranch").and_then(|b| b.as_str()) {
                    stats.git_branches.insert(branch.to_string());
                }
                if let Some(cwd) = value.get("cwd").and_then(|c| c.as_str()) {
                    stats.working_directories.insert(cwd.to_string());
                }
            }
        }

        Ok(stats)
    }
}

/// Statistics about a Claude Code session
#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub total_lines: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub system_messages: usize,
    pub progress_messages: usize,
    pub tool_calls: usize,
    pub file_snapshots: usize,
    pub git_branches: std::collections::HashSet<String>,
    pub working_directories: std::collections::HashSet<String>,
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
    fn test_parse_event_line_user_message_with_message_object() {
        let adapter = ClaudeAdapter::new();
        let value = serde_json::json!({
            "type": "user",
            "uuid": "test-uuid",
            "timestamp": "2024-01-01T00:00:00Z",
            "message": {
                "role": "user",
                "content": "Message from object"
            }
        });

        let event = adapter.parse_event_line(&value, 0).unwrap();
        assert_eq!(event.kind, EventKind::Message);
        assert_eq!(event.role, Some(Role::User));
        assert_eq!(event.content, Some("Message from object".to_string()));
    }

    #[test]
    fn test_parse_event_line_assistant_message() {
        let adapter = ClaudeAdapter::new();
        let value = serde_json::json!({
            "type": "assistant",
            "uuid": "test-uuid",
            "timestamp": "2024-01-01T00:00:00Z",
            "message": {
                "role": "assistant",
                "content": [{"type": "text", "text": "How can I help?"}]
            }
        });

        let event = adapter.parse_event_line(&value, 0).unwrap();
        assert_eq!(event.kind, EventKind::Message);
        assert_eq!(event.role, Some(Role::Assistant));
        assert_eq!(event.content, Some("How can I help?".to_string()));
    }

    #[test]
    fn test_parse_event_line_assistant_with_tool_use() {
        let adapter = ClaudeAdapter::new();
        let value = serde_json::json!({
            "type": "assistant",
            "uuid": "test-uuid",
            "timestamp": "2024-01-01T00:00:00Z",
            "message": {
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Let me check that file."},
                    {"type": "tool_use", "id": "call_123", "name": "Read", "input": {"file_path": "/test/file.txt"}}
                ]
            }
        });

        let event = adapter.parse_event_line(&value, 0).unwrap();
        assert_eq!(event.kind, EventKind::Message);
        assert_eq!(event.role, Some(Role::Assistant));
        assert_eq!(event.content, Some("Let me check that file.".to_string()));

        let (_, tool_calls, _) = adapter.extract_assistant_content(&value);
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "Read");
    }

    #[test]
    fn test_parse_event_line_assistant_with_thinking() {
        let adapter = ClaudeAdapter::new();
        let value = serde_json::json!({
            "type": "assistant",
            "uuid": "test-uuid",
            "timestamp": "2024-01-01T00:00:00Z",
            "message": {
                "role": "assistant",
                "content": [
                    {"type": "thinking", "thinking": "I need to analyze this carefully."},
                    {"type": "text", "text": "Here's my analysis."}
                ]
            }
        });

        let event = adapter.parse_event_line(&value, 0).unwrap();
        assert_eq!(event.content, Some("Here's my analysis.".to_string()));

        let (_, _, thinking) = adapter.extract_assistant_content(&value);
        assert_eq!(thinking, Some("I need to analyze this carefully.".to_string()));
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
    fn test_parse_event_line_progress() {
        let adapter = ClaudeAdapter::new();
        let value = serde_json::json!({
            "type": "progress",
            "uuid": "test-uuid",
            "timestamp": "2024-01-01T00:00:00Z",
            "data": {
                "message": {
                    "content": "Processing..."
                }
            }
        });

        let event = adapter.parse_event_line(&value, 0).unwrap();
        assert_eq!(event.kind, EventKind::System);
        assert_eq!(event.content, Some("Processing...".to_string()));
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

    #[test]
    fn test_rebuild_conversations() {
        let adapter = ClaudeAdapter::new();
        let events = vec![
            Event {
                id: Uuid::new_v4(),
                session_id: Uuid::nil(),
                kind: EventKind::Message,
                role: Some(Role::User),
                content: Some("First message".to_string()),
                timestamp: Utc::now(),
                raw_payload: serde_json::json!({
                    "type": "user",
                    "uuid": "uuid-1",
                    "parentUuid": null,
                    "timestamp": "2024-01-01T00:00:00Z"
                }),
            },
            Event {
                id: Uuid::new_v4(),
                session_id: Uuid::nil(),
                kind: EventKind::Message,
                role: Some(Role::Assistant),
                content: Some("Response".to_string()),
                timestamp: Utc::now(),
                raw_payload: serde_json::json!({
                    "type": "assistant",
                    "uuid": "uuid-2",
                    "parentUuid": "uuid-1",
                    "timestamp": "2024-01-01T00:00:01Z"
                }),
            },
        ];

        let threads = adapter.rebuild_conversations(&events);
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].root_uuid, "uuid-1");
        assert_eq!(threads[0].nodes.len(), 2);
    }
}
