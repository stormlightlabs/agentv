use serde::{Deserialize, Serialize};

/// Database row for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRow {
    pub id: String,
    pub source: String,
    pub external_id: String,
    pub project: Option<String>,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub raw_payload: String,
}

/// Database row for an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRow {
    pub id: String,
    pub session_id: String,
    pub kind: String,
    pub role: Option<String>,
    pub content: Option<String>,
    pub timestamp: String,
    pub raw_payload: String,
}

/// Computed metrics for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetricsRow {
    pub session_id: String,
    pub total_events: i64,
    pub message_count: i64,
    pub tool_call_count: i64,
    pub tool_result_count: i64,
    pub error_count: i64,
    pub user_messages: i64,
    pub assistant_messages: i64,
    pub duration_seconds: Option<i64>,
    pub files_touched: i64,
    pub lines_added: i64,
    pub lines_removed: i64,
    pub computed_at: String,
}

/// Database row for a tool call with latency tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRow {
    pub id: String,
    pub session_id: String,
    pub event_id: String,
    pub tool_name: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub success: Option<bool>,
    pub error_message: Option<String>,
}

/// Database row for a file that was touched during a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTouchedRow {
    pub id: String,
    pub session_id: String,
    pub file_path: String,
    pub operation: String,
    pub lines_added: i64,
    pub lines_removed: i64,
    pub touched_at: String,
}
