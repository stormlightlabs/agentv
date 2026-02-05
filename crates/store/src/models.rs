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
