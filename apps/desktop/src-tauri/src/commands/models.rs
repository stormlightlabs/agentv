use serde::{Deserialize, Serialize};

/// Session data for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub id: String,
    pub source: String,
    pub external_id: String,
    pub project: Option<String>,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Event data for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    pub id: String,
    pub session_id: String,
    pub kind: String,
    pub role: Option<String>,
    pub content: Option<String>,
    pub timestamp: String,
}

/// Result of an ingestion operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub imported: usize,
    pub failed: usize,
    pub total: usize,
    pub source: String,
    pub duration_ms: u64,
}

/// Search result for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub event: EventData,
    pub rank: f64,
    pub snippet: Option<String>,
}

/// Search facets for filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacets {
    pub source: Option<String>,
    pub project: Option<String>,
    pub kind: Option<String>,
    pub since: Option<String>,
}

/// Activity stats for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityStats {
    pub day: String,
    pub event_count: i64,
    pub session_count: i64,
}

/// Error stats for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStats {
    pub day: String,
    pub error_count: i64,
    pub signature: Option<String>,
}

/// Grouped stats for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct GroupedStats {
    pub dimension: String,
    pub count: i64,
    pub sessions: Option<i64>,
    pub earliest: Option<String>,
    pub latest: Option<String>,
}

/// Tool frequency stats for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFrequencyStats {
    pub tool_name: String,
    pub call_count: i64,
    pub sessions: i64,
    pub avg_duration_ms: Option<f64>,
    pub max_duration_ms: Option<i64>,
}

/// File leaderboard entry for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLeaderboardEntry {
    pub file_path: String,
    pub touch_count: i64,
    pub sessions: i64,
    pub total_lines_added: i64,
    pub total_lines_removed: i64,
}

/// Patch churn stats for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchChurnStats {
    pub day: String,
    pub lines_added: i64,
    pub lines_removed: i64,
    pub files_changed: i64,
    pub sessions: i64,
}

/// Long-running tool call for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongRunningToolCall {
    pub tool_name: String,
    pub duration_ms: i64,
    pub started_at: String,
    pub session_external_id: String,
    pub project: Option<String>,
    pub error_message: Option<String>,
}

/// Export format enum
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Markdown,
    Json,
    Jsonl,
}

impl ExportFormat {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "md" | "markdown" => Ok(ExportFormat::Markdown),
            "json" => Ok(ExportFormat::Json),
            "jsonl" => Ok(ExportFormat::Jsonl),
            _ => Err(format!("Unknown format: {}. Use 'md', 'json', or 'jsonl'", s)),
        }
    }
}
