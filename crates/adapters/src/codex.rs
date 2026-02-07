use agent_v_core::{Event, EventKind, Role, Session, Source};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A discovered Codex rollout session file
#[derive(Debug, Clone)]
pub struct CodexSessionFile {
    pub path: PathBuf,
    pub session_id: String,
    pub date: String,
}

/// Codex session metadata from session_meta event
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexSessionMeta {
    id: String,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    cli_version: Option<String>,
    #[serde(default)]
    model_provider: Option<String>,
    #[serde(default)]
    git: Option<CodexGitInfo>,
}

/// Git metadata from Codex
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexGitInfo {
    #[serde(default)]
    commit_hash: Option<String>,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    repository_url: Option<String>,
}

/// Codex event wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexEvent {
    timestamp: String,
    #[serde(rename = "type")]
    event_type: String,
    payload: serde_json::Value,
}

/// Response item payload (message, function_call, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResponseItem {
    #[serde(rename = "type")]
    item_type: String,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: Option<Vec<ContentBlock>>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
    #[serde(default)]
    call_id: Option<String>,
    #[serde(default)]
    output: Option<String>,
}

/// Content block in messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "output_text")]
    OutputText { text: String },
    #[serde(other)]
    Other,
}

/// Event message payload
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    message: Option<String>,
}

/// Turn context payload
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TurnContext {
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    model: Option<String>,
}

/// Adapter for Codex CLI rollout logs
#[derive(Debug, Clone)]
pub struct CodexAdapter {
    sessions_dir: PathBuf,
}

impl CodexAdapter {
    /// Create a new Codex adapter with default sessions directory
    pub fn new() -> Self {
        let sessions_dir = std::env::var("CODEX_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|h| h.join(".codex").join("sessions")))
            .unwrap_or_else(|| PathBuf::from(".codex/sessions"));

        Self { sessions_dir }
    }

    /// Create a new Codex adapter with a custom sessions directory
    pub fn with_sessions_dir(sessions_dir: PathBuf) -> Self {
        Self { sessions_dir }
    }

    /// Get the sessions directory path
    pub fn sessions_dir(&self) -> &PathBuf {
        &self.sessions_dir
    }

    /// Discover all rollout session files in the sessions directory
    pub async fn discover_sessions(&self) -> Vec<CodexSessionFile> {
        let mut sessions = Vec::new();

        if !self.sessions_dir.exists() {
            tracing::warn!("Codex sessions directory not found: {:?}", self.sessions_dir);
            return sessions;
        }

        let mut year_entries = match tokio::fs::read_dir(&self.sessions_dir).await {
            Ok(e) => e,
            Err(e) => {
                tracing::error!("Failed to read sessions directory: {}", e);
                return sessions;
            }
        };

        while let Ok(Some(year_entry)) = year_entries.next_entry().await {
            let year_path = year_entry.path();
            if !year_path.is_dir() {
                continue;
            }

            let year = year_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");

            let mut month_entries = match tokio::fs::read_dir(&year_path).await {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("Failed to read year directory {:?}: {}", year_path, e);
                    continue;
                }
            };

            while let Ok(Some(month_entry)) = month_entries.next_entry().await {
                let month_path = month_entry.path();
                if !month_path.is_dir() {
                    continue;
                }

                let mut day_entries = match tokio::fs::read_dir(&month_path).await {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!("Failed to read month directory {:?}: {}", month_path, e);
                        continue;
                    }
                };

                while let Ok(Some(day_entry)) = day_entries.next_entry().await {
                    let day_path = day_entry.path();
                    if !day_path.is_dir() {
                        continue;
                    }

                    let mut jsonl_files = match tokio::fs::read_dir(&day_path).await {
                        Ok(f) => f,
                        Err(e) => {
                            tracing::warn!("Failed to read day directory {:?}: {}", day_path, e);
                            continue;
                        }
                    };

                    while let Ok(Some(file_entry)) = jsonl_files.next_entry().await {
                        let file_path = file_entry.path();
                        if file_path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                            let file_name = file_path.file_name().and_then(|s| s.to_str()).unwrap_or("unknown");

                            if let Some(session_id) = file_name
                                .strip_prefix("rollout-")
                                .and_then(|s| s.strip_suffix(".jsonl"))
                            {
                                let date = format!(
                                    "{}/{}/{}",
                                    year,
                                    month_path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                                    day_path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
                                );
                                sessions.push(CodexSessionFile {
                                    path: file_path.clone(),
                                    session_id: session_id.to_string(),
                                    date,
                                });
                            }
                        }
                    }
                }
            }
        }

        tracing::info!("Discovered {} Codex sessions", sessions.len());
        sessions
    }

    /// Parse a rollout session file and return a Session with its Events
    pub async fn parse_session(
        &self, session_file: &CodexSessionFile,
    ) -> Result<(Session, Vec<Event>), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Parsing session file: {:?}", session_file.path);

        let content = tokio::fs::read_to_string(&session_file.path).await?;
        let lines: Vec<&str> = content.lines().collect();

        let mut session_meta: Option<CodexSessionMeta> = None;
        let mut events = Vec::new();
        let mut first_timestamp: Option<DateTime<Utc>> = None;
        let mut last_timestamp: Option<DateTime<Utc>> = None;
        let mut project: Option<String> = None;
        let session_title: Option<String> = None;

        for (idx, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            let codex_event: CodexEvent = match serde_json::from_str(line) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("Failed to parse line {} in {:?}: {}", idx, session_file.path, e);
                    continue;
                }
            };

            let timestamp = DateTime::parse_from_rfc3339(&codex_event.timestamp)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            if first_timestamp.is_none() {
                first_timestamp = Some(timestamp);
            }
            last_timestamp = Some(timestamp);

            match codex_event.event_type.as_str() {
                "session_meta" => {
                    if let Ok(meta) = serde_json::from_value::<CodexSessionMeta>(codex_event.payload.clone()) {
                        session_meta = Some(meta.clone());
                        project = meta.cwd.clone();

                        if let Some(git) = meta.git
                            && let Some(repo_url) = git.repository_url
                        {
                            let repo_name = repo_url.split('/').next_back().unwrap_or("unknown");
                            project = Some(format!(
                                "{}/{}",
                                repo_name,
                                git.branch.unwrap_or_else(|| "main".to_string())
                            ));
                        }
                    }
                }
                "response_item" => {
                    if let Some(event) = self.parse_response_item(&codex_event, timestamp) {
                        events.push(event);
                    }
                }
                "event_msg" => {
                    if let Some(event) = self.parse_event_msg(&codex_event, timestamp) {
                        events.push(event);
                    }
                }
                "turn_context" => {}
                _ => {
                    tracing::trace!("Unknown Codex event type: {}", codex_event.event_type);
                }
            }
        }

        let created_at = first_timestamp.unwrap_or_else(Utc::now);
        let updated_at = last_timestamp.unwrap_or(created_at);

        let external_id = session_file.session_id.clone();

        let raw_payload = serde_json::json!({
            "source": "codex",
            "session_id": external_id,
            "date": session_file.date,
            "file_path": session_file.path.to_string_lossy().to_string(),
            "line_count": lines.len(),
            "meta": session_meta,
        });

        let session = Session {
            id: uuid::Uuid::new_v4(),
            source: Source::Codex,
            external_id,
            project,
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

        tracing::info!(
            "Parsed Codex session {} with {} events",
            session.external_id,
            events.len()
        );

        Ok((session, events))
    }

    /// Parse a response_item event
    fn parse_response_item(&self, codex_event: &CodexEvent, timestamp: DateTime<Utc>) -> Option<Event> {
        let item = serde_json::from_value::<ResponseItem>(codex_event.payload.clone()).ok()?;

        match item.item_type.as_str() {
            "message" => {
                let role = match item.role.as_deref() {
                    Some("user") => Some(Role::User),
                    Some("assistant") => Some(Role::Assistant),
                    Some("system") => Some(Role::System),
                    _ => None,
                };

                let content = item.content.and_then(|blocks| {
                    blocks
                        .into_iter()
                        .filter_map(|block| match block {
                            ContentBlock::InputText { text } => Some(text),
                            ContentBlock::OutputText { text } => Some(text),
                            ContentBlock::Other => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                        .into()
                });

                Some(Event {
                    id: uuid::Uuid::new_v4(),
                    session_id: uuid::Uuid::nil(),
                    kind: EventKind::Message,
                    role,
                    content,
                    timestamp,
                    raw_payload: serde_json::to_value(codex_event).unwrap_or_default(),
                })
            }
            "function_call" => {
                let name = item.name.unwrap_or_else(|| "unknown".to_string());
                let arguments = item.arguments.unwrap_or_else(|| "{}".to_string());

                let content = Some(format!("Called {} with arguments: {}", name, arguments));

                Some(Event {
                    id: uuid::Uuid::new_v4(),
                    session_id: uuid::Uuid::nil(),
                    kind: EventKind::ToolCall,
                    role: Some(Role::Assistant),
                    content,
                    timestamp,
                    raw_payload: serde_json::to_value(codex_event).unwrap_or_default(),
                })
            }
            "function_call_output" => {
                let output = item.output.unwrap_or_else(|| "".to_string());

                Some(Event {
                    id: uuid::Uuid::new_v4(),
                    session_id: uuid::Uuid::nil(),
                    kind: EventKind::ToolResult,
                    role: None,
                    content: Some(output),
                    timestamp,
                    raw_payload: serde_json::to_value(codex_event).unwrap_or_default(),
                })
            }
            "reasoning" => Some(Event {
                id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::nil(),
                kind: EventKind::System,
                role: Some(Role::Assistant),
                content: Some("[Reasoning content encrypted by Codex]".to_string()),
                timestamp,
                raw_payload: serde_json::to_value(codex_event).unwrap_or_default(),
            }),
            _ => None,
        }
    }

    /// Parse an event_msg event
    fn parse_event_msg(&self, codex_event: &CodexEvent, timestamp: DateTime<Utc>) -> Option<Event> {
        let msg = serde_json::from_value::<EventMessage>(codex_event.payload.clone()).ok()?;

        match msg.msg_type.as_str() {
            "user_message" => Some(Event {
                id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::nil(),
                kind: EventKind::Message,
                role: Some(Role::User),
                content: msg.message,
                timestamp,
                raw_payload: serde_json::to_value(codex_event).unwrap_or_default(),
            }),
            "agent_reasoning" => Some(Event {
                id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::nil(),
                kind: EventKind::System,
                role: Some(Role::Assistant),
                content: msg.message.map(|m| format!("[Thinking] {}", m)),
                timestamp,
                raw_payload: serde_json::to_value(codex_event).unwrap_or_default(),
            }),
            "token_count" => None,
            _ => None,
        }
    }

    /// Get statistics about a rollout session file
    pub async fn get_session_stats(
        &self, session_file: &CodexSessionFile,
    ) -> Result<SessionStats, Box<dyn std::error::Error + Send + Sync>> {
        let content = tokio::fs::read_to_string(&session_file.path).await?;
        let lines: Vec<&str> = content.lines().collect();

        let mut stats = SessionStats { total_lines: lines.len(), ..SessionStats::default() };

        for line in &lines {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(codex_event) = serde_json::from_str::<CodexEvent>(line) {
                match codex_event.event_type.as_str() {
                    "response_item" => {
                        if let Ok(item) = serde_json::from_value::<ResponseItem>(codex_event.payload) {
                            match item.item_type.as_str() {
                                "message" => match item.role.as_deref() {
                                    Some("user") => stats.user_messages += 1,
                                    Some("assistant") => stats.assistant_messages += 1,
                                    Some("system") => stats.system_messages += 1,
                                    _ => {}
                                },
                                "function_call" => {
                                    stats.tool_calls += 1;
                                }
                                _ => {}
                            }
                        }
                    }
                    "event_msg" => {
                        if let Ok(msg) = serde_json::from_value::<EventMessage>(codex_event.payload) {
                            match msg.msg_type.as_str() {
                                "user_message" => stats.user_messages += 1,
                                "agent_reasoning" => stats.system_messages += 1,
                                _ => {}
                            }
                        }
                    }
                    "turn_context" => {
                        if let Ok(ctx) = serde_json::from_value::<TurnContext>(codex_event.payload)
                            && let Some(cwd) = ctx.cwd
                        {
                            stats.working_directories.insert(cwd);
                        }
                    }
                    "session_meta" => {
                        if let Ok(meta) = serde_json::from_value::<CodexSessionMeta>(codex_event.payload) {
                            if let Some(cwd) = meta.cwd {
                                stats.working_directories.insert(cwd);
                            }

                            if let Some(git) = meta.git
                                && let Some(branch) = git.branch
                            {
                                stats.git_branches.insert(branch);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(stats)
    }
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about a Codex rollout session
#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub total_lines: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub system_messages: usize,
    pub tool_calls: usize,
    pub git_branches: std::collections::HashSet<String>,
    pub working_directories: std::collections::HashSet<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codex_adapter_new() {
        let adapter = CodexAdapter::new();
        assert!(
            adapter.sessions_dir().to_string_lossy().contains(".codex")
                || adapter.sessions_dir().to_string_lossy().contains("CODEX_HOME")
        );
    }

    #[test]
    fn test_parse_response_item_message() {
        let adapter = CodexAdapter::new();
        let codex_event = CodexEvent {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            event_type: "response_item".to_string(),
            payload: serde_json::json!({
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "Hello, world!"}]
            }),
        };

        let event = adapter.parse_response_item(&codex_event, Utc::now());
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.kind, EventKind::Message);
        assert_eq!(event.role, Some(Role::User));
        assert_eq!(event.content, Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_parse_response_item_function_call() {
        let adapter = CodexAdapter::new();
        let codex_event = CodexEvent {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            event_type: "response_item".to_string(),
            payload: serde_json::json!({
                "type": "function_call",
                "name": "shell",
                "arguments": "{\"command\": [\"ls\"]}",
                "call_id": "call_123"
            }),
        };

        let event = adapter.parse_response_item(&codex_event, Utc::now());
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.kind, EventKind::ToolCall);
        assert!(event.content.is_some());
    }

    #[test]
    fn test_parse_event_msg_user_message() {
        let adapter = CodexAdapter::new();
        let codex_event = CodexEvent {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            event_type: "event_msg".to_string(),
            payload: serde_json::json!({
                "type": "user_message",
                "message": "Test message"
            }),
        };

        let event = adapter.parse_event_msg(&codex_event, Utc::now());
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.kind, EventKind::Message);
        assert_eq!(event.role, Some(Role::User));
        assert_eq!(event.content, Some("Test message".to_string()));
    }
}
