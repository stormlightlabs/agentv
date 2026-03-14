//! Session merge planning for duplicate session rows.
//!
//! This module contains a deterministic merge planner used to consolidate
//! duplicate sessions that share the same `(source, external_id)`.
//!
//! The planner is intentionally pure:
//! - It does not read/write the database.
//! - It returns a merge plan that DB code can execute transactionally.
//!
//! Merge strategy:
//! 1. Select a canonical "keep" session using:
//!    - highest event count
//!    - latest `updated_at`
//!    - earliest `created_at`
//!    - lexical `id` as final deterministic tiebreaker
//! 2. Merge donor session events into the keep session.
//! 3. De-duplicate semantically equivalent events by fingerprinting:
//!    - `kind`, `role`, `content`, `timestamp`
//!    - canonicalized `raw_payload` JSON (object keys sorted recursively)
//! 4. Preserve deterministic insertion order by sorting donor events by
//!    `timestamp`, then `id`.
//! 5. Assign new event IDs for merged rows to avoid collisions with donor IDs
//!    that still exist in the same transaction before donor session deletion.
//!
//! This is designed for historical data repair where prior schema versions
//! or import bugs may have produced duplicate session rows.

use std::collections::{HashMap, HashSet};

/// Session candidate metadata used by the merge planner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeSession {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub event_count: usize,
}

/// Event candidate metadata used by the merge planner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeEvent {
    pub id: String,
    pub kind: String,
    pub role: Option<String>,
    pub content: Option<String>,
    pub timestamp: String,
    pub raw_payload: String,
}

/// Event that should be inserted into the canonical session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeEventInsert {
    pub id: String,
    pub kind: String,
    pub role: Option<String>,
    pub content: Option<String>,
    pub timestamp: String,
    pub raw_payload: String,
}

/// Transaction-ready merge plan for one duplicate `(source, external_id)` group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionMergePlan {
    pub keep_session_id: String,
    pub donor_session_ids: Vec<String>,
    pub earliest_created_at: String,
    pub latest_updated_at: String,
    pub events_to_insert: Vec<MergeEventInsert>,
}

/// Build a merge plan for a duplicate session group.
///
/// Returns `None` when there are fewer than two sessions.
pub fn build_merge_plan(
    sessions: &[MergeSession], events_by_session: &HashMap<String, Vec<MergeEvent>>,
) -> Option<SessionMergePlan> {
    if sessions.len() < 2 {
        return None;
    }

    let mut ordered = sessions.to_vec();
    ordered.sort_by(compare_sessions_for_keep);

    let keep = ordered.first()?.clone();
    let donors: Vec<MergeSession> = ordered.into_iter().skip(1).collect();

    let earliest_created_at = sessions
        .iter()
        .map(|s| s.created_at.as_str())
        .min()
        .unwrap_or(keep.created_at.as_str())
        .to_string();
    let latest_updated_at = sessions
        .iter()
        .map(|s| s.updated_at.as_str())
        .max()
        .unwrap_or(keep.updated_at.as_str())
        .to_string();

    let keep_events = events_by_session.get(&keep.id).cloned().unwrap_or_default();
    let mut seen_fingerprints: HashSet<String> = keep_events.iter().map(event_fingerprint).collect();

    let mut events_to_insert: Vec<MergeEventInsert> = Vec::new();

    for donor in &donors {
        let mut donor_events = events_by_session.get(&donor.id).cloned().unwrap_or_default();

        donor_events.sort_by(|a, b| {
            let ts = a.timestamp.cmp(&b.timestamp);
            if ts == std::cmp::Ordering::Equal { a.id.cmp(&b.id) } else { ts }
        });

        for donor_event in donor_events {
            let fingerprint = event_fingerprint(&donor_event);
            if !seen_fingerprints.insert(fingerprint) {
                continue;
            }

            events_to_insert.push(MergeEventInsert {
                id: uuid::Uuid::new_v4().to_string(),
                kind: donor_event.kind,
                role: donor_event.role,
                content: donor_event.content,
                timestamp: donor_event.timestamp,
                raw_payload: donor_event.raw_payload,
            });
        }
    }

    Some(SessionMergePlan {
        keep_session_id: keep.id,
        donor_session_ids: donors.into_iter().map(|s| s.id).collect(),
        earliest_created_at,
        latest_updated_at,
        events_to_insert,
    })
}

fn compare_sessions_for_keep(a: &MergeSession, b: &MergeSession) -> std::cmp::Ordering {
    b.event_count
        .cmp(&a.event_count)
        .then_with(|| b.updated_at.cmp(&a.updated_at))
        .then_with(|| a.created_at.cmp(&b.created_at))
        .then_with(|| a.id.cmp(&b.id))
}

fn event_fingerprint(event: &MergeEvent) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        event.kind,
        event.role.as_deref().unwrap_or(""),
        event.content.as_deref().unwrap_or(""),
        event.timestamp,
        canonicalize_raw_payload(&event.raw_payload)
    )
}

fn canonicalize_raw_payload(raw_payload: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(raw_payload) {
        Ok(value) => canonicalize_json_value(&value),
        Err(_) => raw_payload.trim().to_string(),
    }
}

fn canonicalize_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("{:?}", s),
        serde_json::Value::Array(items) => {
            let canonical_items: Vec<String> = items.iter().map(canonicalize_json_value).collect();
            format!("[{}]", canonical_items.join(","))
        }
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
            keys.sort_unstable();
            let pairs = keys
                .into_iter()
                .map(|key| {
                    let value = map.get(key).expect("key gathered from map.keys must exist");
                    format!("{:?}:{}", key, canonicalize_json_value(value))
                })
                .collect::<Vec<_>>();
            format!("{{{}}}", pairs.join(","))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, created_at: &str, updated_at: &str, event_count: usize) -> MergeSession {
        MergeSession {
            id: id.to_string(),
            created_at: created_at.to_string(),
            updated_at: updated_at.to_string(),
            event_count,
        }
    }

    fn event(id: &str, kind: &str, content: &str, ts: &str, raw: &str) -> MergeEvent {
        MergeEvent {
            id: id.to_string(),
            kind: kind.to_string(),
            role: Some("user".to_string()),
            content: Some(content.to_string()),
            timestamp: ts.to_string(),
            raw_payload: raw.to_string(),
        }
    }

    #[test]
    fn chooses_session_with_more_events_as_keep() {
        let sessions = vec![
            session("s1", "2026-03-01T00:00:00Z", "2026-03-01T10:00:00Z", 2),
            session("s2", "2026-03-01T00:00:00Z", "2026-03-01T09:00:00Z", 5),
        ];

        let mut events_by_session = HashMap::new();
        events_by_session.insert("s1".to_string(), vec![]);
        events_by_session.insert("s2".to_string(), vec![]);

        let plan = build_merge_plan(&sessions, &events_by_session).expect("plan should exist");
        assert_eq!(plan.keep_session_id, "s2");
        assert_eq!(plan.donor_session_ids, vec!["s1".to_string()]);
    }

    #[test]
    fn dedupes_same_semantic_event_even_with_different_event_ids() {
        let sessions = vec![
            session("keep", "2026-03-01T00:00:00Z", "2026-03-02T00:00:00Z", 1),
            session("dup", "2026-03-01T00:00:00Z", "2026-03-02T00:00:00Z", 1),
        ];

        let mut events_by_session = HashMap::new();
        events_by_session.insert(
            "keep".to_string(),
            vec![event(
                "event-a",
                "message",
                "hello",
                "2026-03-01T00:00:00Z",
                r#"{"b":2,"a":1}"#,
            )],
        );
        events_by_session.insert(
            "dup".to_string(),
            vec![event(
                "event-b",
                "message",
                "hello",
                "2026-03-01T00:00:00Z",
                r#"{"a":1,"b":2}"#,
            )],
        );

        let plan = build_merge_plan(&sessions, &events_by_session).expect("plan should exist");
        assert!(plan.events_to_insert.is_empty());
    }

    #[test]
    fn merges_unique_events_from_donor_session() {
        let sessions = vec![
            session("keep", "2026-03-01T00:00:00Z", "2026-03-04T00:00:00Z", 2),
            session("dup", "2026-02-28T00:00:00Z", "2026-03-03T00:00:00Z", 2),
        ];

        let mut events_by_session = HashMap::new();
        events_by_session.insert(
            "keep".to_string(),
            vec![event(
                "event-a",
                "message",
                "hello",
                "2026-03-01T00:00:00Z",
                r#"{"x":1}"#,
            )],
        );
        events_by_session.insert(
            "dup".to_string(),
            vec![
                event("event-b", "message", "hello", "2026-03-01T00:00:00Z", r#"{"x":1}"#),
                event("event-c", "message", "goodbye", "2026-03-01T00:01:00Z", r#"{"x":2}"#),
            ],
        );

        let plan = build_merge_plan(&sessions, &events_by_session).expect("plan should exist");

        assert_eq!(plan.keep_session_id, "keep");
        assert_eq!(plan.events_to_insert.len(), 1);
        assert_eq!(plan.events_to_insert[0].content.as_deref(), Some("goodbye"));
        assert_eq!(plan.earliest_created_at, "2026-02-28T00:00:00Z");
        assert_eq!(plan.latest_updated_at, "2026-03-04T00:00:00Z");
    }

    #[test]
    fn assigns_new_event_id_for_merged_event() {
        let sessions = vec![
            session("keep", "2026-03-01T00:00:00Z", "2026-03-02T00:00:00Z", 1),
            session("dup", "2026-03-01T00:00:00Z", "2026-03-02T00:00:00Z", 1),
        ];
        let donor_event_id = "event-donor-id";

        let mut events_by_session = HashMap::new();
        events_by_session.insert(
            "keep".to_string(),
            vec![event(
                "event-keep-id",
                "message",
                "hello",
                "2026-03-01T00:00:00Z",
                r#"{"x":1}"#,
            )],
        );
        events_by_session.insert(
            "dup".to_string(),
            vec![event(
                donor_event_id,
                "message",
                "different",
                "2026-03-01T00:01:00Z",
                r#"{"x":2}"#,
            )],
        );

        let plan = build_merge_plan(&sessions, &events_by_session).expect("plan should exist");
        assert_eq!(plan.events_to_insert.len(), 1);
        assert_ne!(plan.events_to_insert[0].id, donor_event_id);
    }

    #[test]
    fn falls_back_to_raw_payload_string_when_json_is_invalid() {
        let sessions = vec![
            session("keep", "2026-03-01T00:00:00Z", "2026-03-02T00:00:00Z", 1),
            session("dup", "2026-03-01T00:00:00Z", "2026-03-02T00:00:00Z", 1),
        ];

        let mut events_by_session = HashMap::new();
        events_by_session.insert(
            "keep".to_string(),
            vec![event("event-a", "message", "hello", "2026-03-01T00:00:00Z", "not-json")],
        );
        events_by_session.insert(
            "dup".to_string(),
            vec![event("event-b", "message", "hello", "2026-03-01T00:00:00Z", "not-json")],
        );

        let plan = build_merge_plan(&sessions, &events_by_session).expect("plan should exist");
        assert!(plan.events_to_insert.is_empty());
    }
}
