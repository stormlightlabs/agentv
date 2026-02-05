use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Source of the agent session (e.g., claude, codex, opencode, crush)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Claude,
    Codex,
    OpenCode,
    Crush,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Claude => write!(f, "claude"),
            Source::Codex => write!(f, "codex"),
            Source::OpenCode => write!(f, "opencode"),
            Source::Crush => write!(f, "crush"),
        }
    }
}

impl std::str::FromStr for Source {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(Source::Claude),
            "codex" => Ok(Source::Codex),
            "opencode" => Ok(Source::OpenCode),
            "crush" => Ok(Source::Crush),
            _ => Err(format!("Unknown source: {}", s)),
        }
    }
}

/// Type of event within a session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Message,
    ToolCall,
    ToolResult,
    Error,
    System,
}

impl std::fmt::Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventKind::Message => write!(f, "message"),
            EventKind::ToolCall => write!(f, "tool_call"),
            EventKind::ToolResult => write!(f, "tool_result"),
            EventKind::Error => write!(f, "error"),
            EventKind::System => write!(f, "system"),
        }
    }
}

/// Role of a message sender
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::System => write!(f, "system"),
        }
    }
}

/// A normalized session from any agent source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub source: Source,
    pub external_id: String,
    pub project: Option<String>,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub raw_payload: serde_json::Value,
}

/// A normalized event within a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub session_id: Uuid,
    pub kind: EventKind,
    pub role: Option<Role>,
    pub content: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub raw_payload: serde_json::Value,
}

/// Health status of an adapter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Health check result for a data source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceHealth {
    pub source: Source,
    pub status: HealthStatus,
    pub path: Option<String>,
    pub message: Option<String>,
}
