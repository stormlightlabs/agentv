use agent_v_core::{Event, EventKind, Role, Session, Source};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use tokio::task;
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

/// OpenCode session export format (from `opencode export`)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeExport {
    info: OpenCodeSessionInfo,
    messages: Vec<OpenCodeMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeSessionInfo {
    id: String,
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
struct OpenCodeMessage {
    info: OpenCodeMessageInfo,
    #[serde(default)]
    parts: Vec<OpenCodePart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeMessageInfo {
    id: String,
    #[serde(rename = "sessionID")]
    session_id: String,
    role: String,
    time: OpenCodeMessageTime,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum OpenCodePart {
    #[serde(rename = "text")]
    Text {
        id: String,
        text: String,
        #[serde(default)]
        synthetic: bool,
    },
    #[serde(rename = "file")]
    File {
        id: String,
        filename: String,
        #[serde(default)]
        url: Option<String>,
        #[serde(default)]
        mime: Option<String>,
    },
    #[serde(rename = "tool")]
    Tool {
        id: String,
        #[serde(rename = "callID")]
        call_id: String,
        tool: String,
        #[serde(default)]
        state: Option<ToolState>,
    },
    #[serde(rename = "step-start")]
    StepStart { id: String },
    #[serde(rename = "step-end")]
    StepEnd { id: String },
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolState {
    #[serde(default)]
    status: String,
    #[serde(default)]
    input: Option<serde_json::Value>,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

/// OpenCode session list format (from `opencode session list --format json`)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenCodeSessionListItem {
    id: String,
    title: String,
    created: i64,
    updated: i64,
    #[serde(rename = "projectId")]
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    directory: Option<String>,
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
#[derive(Debug, Clone)]
pub struct OpenCodeAdapter {
    auth_path: PathBuf,
}

impl OpenCodeAdapter {
    /// Create a new OpenCode adapter with default paths
    pub fn new() -> Self {
        let auth_path = dirs::home_dir()
            .map(|h| h.join(".local/share/opencode/auth.json"))
            .unwrap_or_else(|| PathBuf::from("~/.local/share/opencode/auth.json"));

        Self { auth_path }
    }

    /// Create a new OpenCode adapter with custom auth path
    pub fn with_auth_path(auth_path: PathBuf) -> Self {
        Self { auth_path }
    }

    /// Check if OpenCode CLI is available
    pub fn is_available(&self) -> bool {
        Command::new("opencode")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Discover all OpenCode sessions via CLI
    pub async fn discover_sessions(&self) -> Vec<OpenCodeSession> {
        if !self.is_available() {
            tracing::warn!("OpenCode CLI not available");
            return Vec::new();
        }

        let sessions = task::spawn_blocking(|| {
            Command::new("opencode")
                .args(["session", "list", "--format", "json"])
                .output()
        })
        .await;

        let Ok(Ok(output)) = sessions else {
            tracing::error!("Failed to execute opencode session list");
            return Vec::new();
        };

        if !output.status.success() {
            tracing::warn!(
                "opencode session list failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return Vec::new();
        }

        let json_str = match String::from_utf8(output.stdout) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to parse session list output: {}", e);
                return Vec::new();
            }
        };

        let items: Vec<OpenCodeSessionListItem> = match serde_json::from_str(&json_str) {
            Ok(items) => items,
            Err(e) => {
                tracing::error!("Failed to parse session list JSON: {}", e);
                return Vec::new();
            }
        };

        let sessions: Vec<OpenCodeSession> = items
            .into_iter()
            .map(|item| {
                let created = DateTime::from_timestamp(item.created, 0).unwrap_or_else(Utc::now);
                let updated = DateTime::from_timestamp(item.updated, 0).unwrap_or(created);

                OpenCodeSession {
                    id: item.id,
                    title: item.title,
                    directory: item.directory,
                    project_id: item.project_id,
                    created,
                    updated,
                }
            })
            .collect();

        tracing::info!("Discovered {} OpenCode sessions", sessions.len());
        sessions
    }

    /// Parse a session by exporting it via OpenCode CLI
    pub async fn parse_session(
        &self, session: &OpenCodeSession,
    ) -> Result<(Session, Vec<Event>), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Parsing OpenCode session: {}", session.id);

        let session_id_for_path = session.id.clone();
        let session_id = session.id.clone();
        let temp_file = std::env::temp_dir()
            .join(format!("opencode_export_{}.json", session_id_for_path))
            .clone();
        let temp_file_clone = temp_file.clone();

        let export_result = task::spawn_blocking(move || {
            Command::new("opencode")
                .args(["export", &session_id])
                .stdout(std::fs::File::create(&temp_file)?)
                .output()
        })
        .await??;

        if !export_result.status.success() {
            return Err(format!(
                "opencode export failed: {}",
                String::from_utf8_lossy(&export_result.stderr)
            )
            .into());
        }

        let temp_file = temp_file_clone.clone();
        let json_str = task::spawn_blocking(move || std::fs::read_to_string(temp_file)).await??;

        let _ = std::fs::remove_file(&temp_file_clone);

        let export_data: OpenCodeExport = serde_json::from_str(&json_str).map_err(|e| {
            tracing::error!("Failed to parse export JSON: {}", e);
            tracing::error!("JSON length: {} bytes", json_str.len());
            e
        })?;

        let created_at = DateTime::from_timestamp(export_data.info.time.created, 0).unwrap_or_else(Utc::now);
        let updated_at = DateTime::from_timestamp(export_data.info.time.updated, 0).unwrap_or(created_at);

        let external_id = export_data.info.id.clone();

        let raw_payload = serde_json::to_value(&export_data.info)?;

        let session_obj = Session {
            id: Uuid::new_v4(),
            source: Source::OpenCode,
            external_id,
            project: export_data.info.directory.clone(),
            title: Some(export_data.info.title.clone()),
            created_at,
            updated_at,
            raw_payload,
        };

        let mut events = Vec::new();

        for message in &export_data.messages {
            let timestamp = DateTime::from_timestamp(message.info.time.created, 0).unwrap_or_else(Utc::now);

            let role = match message.info.role.as_str() {
                "user" => Some(Role::User),
                "assistant" => Some(Role::Assistant),
                "system" => Some(Role::System),
                _ => None,
            };

            let event_kind = if role.is_some() { EventKind::Message } else { EventKind::System };

            let content = self.format_message_content(message);

            let event = Event {
                id: Uuid::new_v4(),
                session_id: session_obj.id,
                kind: event_kind,
                role,
                content: Some(content),
                timestamp,
                raw_payload: serde_json::to_value(message)?,
            };

            events.push(event);

            for part in &message.parts {
                if let OpenCodePart::Tool { tool, state, .. } = part {
                    let tool_content = if let Some(state) = state {
                        serde_json::to_string(&serde_json::json!({
                            "tool": tool,
                            "status": state.status,
                            "input": state.input,
                            "output": state.output,
                            "metadata": state.metadata,
                        }))
                        .ok()
                    } else {
                        Some(tool.clone())
                    };

                    let tool_event = Event {
                        id: Uuid::new_v4(),
                        session_id: session_obj.id,
                        kind: EventKind::ToolCall,
                        role: Some(Role::Assistant),
                        content: tool_content,
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

    /// Format message content from parts
    fn format_message_content(&self, message: &OpenCodeMessage) -> String {
        let mut content_parts = Vec::new();

        for part in &message.parts {
            match part {
                OpenCodePart::Text { text, .. } => {
                    content_parts.push(text.clone());
                }
                OpenCodePart::File { filename, .. } => {
                    content_parts.push(format!("[📎 {}]", filename));
                }
                OpenCodePart::Tool { tool, state, .. } => {
                    if let Some(state) = state {
                        content_parts.push(format!("🔧 {} (status: {})", tool, state.status));
                    } else {
                        content_parts.push(format!("🔧 {}", tool));
                    }
                }
                _ => {}
            }
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
