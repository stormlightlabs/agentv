use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
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

/// Metadata for AI models including pricing and provider info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub model_id: String,
    pub provider: String,
    pub input_price_per_1m: f64,
    pub output_price_per_1m: f64,
    pub extended_input_price_per_1m: Option<f64>,
    pub extended_output_price_per_1m: Option<f64>,
}

impl ModelMetadata {
    /// Average characters per token heuristic
    pub const CHARS_PER_TOKEN: usize = 4;

    /// Get the comprehensive model registry for early 2026
    pub fn get_registry() -> Vec<ModelMetadata> {
        static REGISTRY: OnceLock<Vec<ModelMetadata>> = OnceLock::new();
        REGISTRY
            .get_or_init(|| {
                let json = include_str!("models.json");
                serde_json::from_str(json).expect("Failed to parse models.json")
            })
            .clone()
    }

    /// Lookup metadata for a given model name
    pub fn lookup(model_name: &str) -> Option<ModelMetadata> {
        let registry = Self::get_registry();
        let name_lower = model_name.to_lowercase();

        if let Some(meta) = registry.iter().find(|m| m.model_id == name_lower) {
            Some(meta.clone())
        } else {
            registry
                .iter()
                .find(|m| name_lower.contains(&m.model_id) || m.model_id.contains(&name_lower))
                .cloned()
        }
    }

    /// Estimate token count from text using characters-per-token heuristic
    pub fn estimate_tokens(text: &str) -> usize {
        text.len().div_ceil(Self::CHARS_PER_TOKEN)
    }

    /// Calculate cost for a given number of tokens
    pub fn calculate_cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        let input_price = if input_tokens > 200_000 {
            self.extended_input_price_per_1m.unwrap_or(self.input_price_per_1m)
        } else {
            self.input_price_per_1m
        };

        let output_price = if input_tokens > 200_000 {
            self.extended_output_price_per_1m.unwrap_or(self.output_price_per_1m)
        } else {
            self.output_price_per_1m
        };

        let input_cost = (input_tokens as f64 / 1_000_000.0) * input_price;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * output_price;

        input_cost + output_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimation() {
        assert_eq!(ModelMetadata::estimate_tokens(""), 0);
        assert_eq!(ModelMetadata::estimate_tokens("abcd"), 1);
        assert_eq!(ModelMetadata::estimate_tokens("abcde"), 2);
    }

    #[test]
    fn test_lookup() {
        let meta = ModelMetadata::lookup("claude-4.5-sonnet");
        assert!(meta.is_some());
        assert_eq!(meta.unwrap().provider, "anthropic");

        let fuzzy = ModelMetadata::lookup("gpt-5.3");
        assert!(fuzzy.is_some());
        assert_eq!(fuzzy.unwrap().provider, "openai");
    }

    #[test]
    fn test_cost_calculation() {
        let meta = ModelMetadata {
            model_id: "test".to_string(),
            provider: "test".to_string(),
            input_price_per_1m: 10.0,
            output_price_per_1m: 30.0,
            extended_input_price_per_1m: None,
            extended_output_price_per_1m: None,
        };

        let cost = meta.calculate_cost(1_000_000, 1_000_000);
        assert_eq!(cost, 40.0);
    }

    #[test]
    fn test_extended_cost_calculation() {
        let meta = ModelMetadata {
            model_id: "test".to_string(),
            provider: "test".to_string(),
            input_price_per_1m: 1.0,
            output_price_per_1m: 1.0,
            extended_input_price_per_1m: Some(5.0),
            extended_output_price_per_1m: Some(5.0),
        };

        assert_eq!(meta.calculate_cost(100_000, 100_000), (200_000.0 / 1_000_000.0) * 1.0);

        let cost = meta.calculate_cost(250_000, 60_000);
        assert_eq!(cost, (250_000.0 / 1_000_000.0) * 5.0 + (60_000.0 / 1_000_000.0) * 5.0);
    }
}
