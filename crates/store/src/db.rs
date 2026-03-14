use agent_v_core::{Event, EventKind, HealthStatus, ModelMetadata, Session, Source, SourceHealth};
use chrono::{DateTime, NaiveDate, Utc};
use log::{error, info};
use rusqlite::OptionalExtension;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio_rusqlite::Connection;

use crate::migrations::MIGRATIONS;
use crate::models::{EventRow, SessionMetricsRow, SessionRow};
use crate::queries;
use crate::session_merge::{MergeEvent, MergeSession, build_merge_plan};

/// Search result with highlighted snippet
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub event: EventRow,
    pub rank: f64,
    pub snippet: Option<String>,
}

/// Facets for filtering search results
#[derive(Debug, Clone, Default)]
pub struct SearchFacets {
    pub source: Option<String>,
    pub project: Option<String>,
    pub kind: Option<String>,
    pub since: Option<DateTime<Utc>>,
}

/// Activity stats for a day
#[derive(Debug, Clone)]
pub struct ActivityStats {
    pub day: NaiveDate,
    pub event_count: i64,
    pub session_count: i64,
}

/// Error stats for a day
#[derive(Debug, Clone)]
pub struct ErrorStats {
    pub day: NaiveDate,
    pub error_count: i64,
    pub signature: Option<String>,
}

/// Stats grouped by a dimension
#[derive(Debug, Clone)]
pub struct GroupedStats {
    pub dimension: String,
    pub count: i64,
    pub sessions: Option<i64>,
    pub earliest: Option<String>,
    pub latest: Option<String>,
}

/// Database connection wrapper with async support
#[derive(Debug)]
pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl Database {
    /// Open or create a database at the given path
    pub async fn open(path: impl Into<PathBuf>) -> Result<Self, tokio_rusqlite::Error> {
        let path = path.into();
        let path_clone = path.clone();

        let conn = Connection::open(path_clone).await?;

        info!("Database opened at: {:?}", path);

        Ok(Self { conn, path })
    }

    /// Open the default database in the user's data directory
    pub async fn open_default() -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = dirs::data_dir()
            .ok_or("Could not determine data directory")?
            .join("agent-viz");

        std::fs::create_dir_all(&data_dir)?;

        let db_path = data_dir.join("agent-viz.db");

        Ok(Self::open(db_path).await?)
    }

    /// Run all pending migrations
    pub async fn migrate(&self) -> Result<(), tokio_rusqlite::Error> {
        self.conn
            .call(|conn| {
                let mut stmt = conn.prepare(
                    "CREATE TABLE IF NOT EXISTS _migrations (
                        id INTEGER PRIMARY KEY,
                        name TEXT NOT NULL UNIQUE,
                        applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                    )",
                )?;
                stmt.execute([])?;
                Ok(())
            })
            .await?;

        for migration in MIGRATIONS {
            let name = migration.name;
            let sql = migration.sql;

            let already_applied: bool = self
                .conn
                .call(move |conn| {
                    let mut stmt = conn.prepare("SELECT 1 FROM _migrations WHERE name = ?1")?;
                    let exists = stmt.exists([name])?;
                    Ok(exists)
                })
                .await?;

            if already_applied {
                continue;
            }

            info!("Applying migration: {}", name);

            self.conn
                .call(move |conn| {
                    conn.execute_batch(sql)?;
                    conn.execute("INSERT INTO _migrations (name) VALUES (?1)", [name])?;
                    Ok(())
                })
                .await?;

            info!("Migration applied: {}", name);
        }

        Ok(())
    }

    /// Get the database file path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Check database health
    pub async fn health_check(&self) -> HealthStatus {
        match self
            .conn
            .call(|conn| {
                conn.query_row("SELECT 1", [], |_| Ok(()))
                    .map_err(tokio_rusqlite::Error::from)
            })
            .await
        {
            Ok(_) => HealthStatus::Healthy,
            Err(e) => {
                error!("Database health check failed: {}", e);
                HealthStatus::Unhealthy
            }
        }
    }

    /// Get all sessions (paginated)
    pub async fn list_sessions(&self, limit: i64, offset: i64) -> Result<Vec<SessionRow>, tokio_rusqlite::Error> {
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::LIST_SESSIONS)?;
                let rows = stmt
                    .query_map([limit, offset], |row| {
                        Ok(SessionRow {
                            id: row.get(0)?,
                            source: row.get(1)?,
                            external_id: row.get(2)?,
                            project: row.get(3)?,
                            title: row.get(4)?,
                            created_at: row.get(5)?,
                            updated_at: row.get(6)?,
                            raw_payload: row.get(7)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Prune duplicate session rows by `(source, external_id)` and keep the richest record.
    pub async fn prune_duplicate_sessions(&self) -> Result<usize, tokio_rusqlite::Error> {
        self.conn
            .call(|conn| {
                let tx = conn.transaction()?;
                let mut pruned_sessions = 0usize;

                let duplicate_groups = {
                    let mut stmt = tx.prepare(
                        r#"
                        SELECT source, external_id
                        FROM sessions
                        GROUP BY source, external_id
                        HAVING COUNT(*) > 1
                        "#,
                    )?;

                    stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
                        .collect::<Result<Vec<_>, _>>()?
                };

                for (source, external_id) in duplicate_groups {
                    let sessions = {
                        let mut stmt = tx.prepare(
                            r#"
                            SELECT
                                s.id,
                                s.created_at,
                                s.updated_at,
                                (
                                    SELECT COUNT(*)
                                    FROM events e
                                    WHERE e.session_id = s.id
                                ) AS event_count
                            FROM sessions s
                            WHERE s.source = ?1 AND s.external_id = ?2
                            "#,
                        )?;

                        stmt.query_map([&source, &external_id], |row| {
                            Ok(MergeSession {
                                id: row.get(0)?,
                                created_at: row.get(1)?,
                                updated_at: row.get(2)?,
                                event_count: row.get::<_, i64>(3)?.max(0) as usize,
                            })
                        })?
                        .collect::<Result<Vec<_>, _>>()?
                    };

                    if sessions.len() < 2 {
                        continue;
                    }

                    let mut events_by_session: HashMap<String, Vec<MergeEvent>> = HashMap::new();
                    for session in &sessions {
                        let events = {
                            let mut stmt = tx.prepare(
                                r#"
                                SELECT id, kind, role, content, timestamp, raw_payload
                                FROM events
                                WHERE session_id = ?1
                                ORDER BY timestamp ASC, id ASC
                                "#,
                            )?;

                            stmt.query_map([&session.id], |row| {
                                Ok(MergeEvent {
                                    id: row.get(0)?,
                                    kind: row.get(1)?,
                                    role: row.get(2)?,
                                    content: row.get(3)?,
                                    timestamp: row.get(4)?,
                                    raw_payload: row.get(5)?,
                                })
                            })?
                            .collect::<Result<Vec<_>, _>>()?
                        };
                        events_by_session.insert(session.id.clone(), events);
                    }

                    let Some(plan) = build_merge_plan(&sessions, &events_by_session) else {
                        continue;
                    };

                    tx.execute(
                        r#"
                        UPDATE sessions
                        SET created_at = ?2, updated_at = ?3
                        WHERE id = ?1
                        "#,
                        rusqlite::params![
                            &plan.keep_session_id,
                            &plan.earliest_created_at,
                            &plan.latest_updated_at
                        ],
                    )?;

                    for event in plan.events_to_insert {
                        tx.execute(
                            r#"
                            INSERT INTO events (id, session_id, kind, role, content, timestamp, raw_payload)
                            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                            "#,
                            rusqlite::params![
                                event.id,
                                &plan.keep_session_id,
                                event.kind,
                                event.role.unwrap_or_default(),
                                event.content.unwrap_or_default(),
                                event.timestamp,
                                event.raw_payload,
                            ],
                        )?;
                    }

                    for donor_id in &plan.donor_session_ids {
                        tx.execute("DELETE FROM sessions WHERE id = ?1", [donor_id])?;
                    }

                    pruned_sessions += plan.donor_session_ids.len();
                }

                tx.commit()?;
                Ok(pruned_sessions)
            })
            .await
    }

    /// Get a session by ID
    pub async fn get_session(&self, id: String) -> Result<Option<SessionRow>, tokio_rusqlite::Error> {
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare("SELECT id, source, external_id, project, title, created_at, updated_at, raw_payload FROM sessions WHERE id = ?1")?;
                let row = stmt.query_row([id], |row| {
                    Ok(SessionRow {
                        id: row.get(0)?,
                        source: row.get(1)?,
                        external_id: row.get(2)?,
                        project: row.get(3)?,
                        title: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                        raw_payload: row.get(7)?,
                    })
                });

                match row {
                    Ok(r) => Ok(Some(r)),
                    Err(e) => {
                        if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                            Ok(None)
                        } else {
                            Err(e.into())
                        }
                    }
                }
            })
            .await
    }

    /// Get sessions with optional source filter (paginated)
    pub async fn list_sessions_filtered(
        &self, source_filter: Option<&str>, limit: i64, offset: i64,
    ) -> Result<Vec<SessionRow>, tokio_rusqlite::Error> {
        let source = source_filter.unwrap_or("").to_string();
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::LIST_SESSIONS_FILTERED)?;
                let rows = stmt
                    .query_map([&source, &limit.to_string(), &offset.to_string()], |row| {
                        Ok(SessionRow {
                            id: row.get(0)?,
                            source: row.get(1)?,
                            external_id: row.get(2)?,
                            project: row.get(3)?,
                            title: row.get(4)?,
                            created_at: row.get(5)?,
                            updated_at: row.get(6)?,
                            raw_payload: row.get(7)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get events for a session
    pub async fn get_session_events(&self, session_id: String) -> Result<Vec<EventRow>, tokio_rusqlite::Error> {
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::GET_SESSION_EVENTS)?;
                let rows = stmt
                    .query_map([session_id], |row| {
                        Ok(EventRow {
                            id: row.get(0)?,
                            session_id: row.get(1)?,
                            kind: row.get(2)?,
                            role: row.get(3)?,
                            content: row.get(4)?,
                            timestamp: row.get(5)?,
                            raw_payload: row.get(6)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Insert or update a session
    pub async fn insert_session(&self, session: &Session) -> Result<(), tokio_rusqlite::Error> {
        let id = session.id.to_string();
        let source = session.source.to_string();
        let external_id = session.external_id.clone();
        let project = session.project.clone();
        let title = session.title.clone();
        let created_at = session.created_at.to_rfc3339();
        let updated_at = session.updated_at.to_rfc3339();
        let raw_payload = serde_json::to_string(&session.raw_payload).unwrap_or_default();

        self.conn
            .call(move |conn| {
                conn.execute(
                    queries::INSERT_SESSION,
                    rusqlite::params![
                        id,
                        source,
                        external_id,
                        project,
                        title,
                        created_at,
                        updated_at,
                        raw_payload,
                    ],
                )?;
                Ok(())
            })
            .await
    }

    /// Insert an event
    pub async fn insert_event(&self, event: &Event) -> Result<(), tokio_rusqlite::Error> {
        let id = event.id.to_string();
        let session_id = event.session_id.to_string();
        let kind = event.kind.to_string();
        let role = event.role.map(|r| r.to_string()).unwrap_or_default();
        let content = event.content.clone().unwrap_or_default();
        let timestamp = event.timestamp.to_rfc3339();
        let raw_payload = serde_json::to_string(&event.raw_payload).unwrap_or_default();

        self.conn
            .call(move |conn| {
                conn.execute(
                    queries::INSERT_EVENT,
                    [id, session_id, kind, role, content, timestamp, raw_payload],
                )?;
                Ok(())
            })
            .await
    }

    /// Insert a session with all its events atomically in a transaction
    pub async fn insert_session_with_events(
        &self, session: &Session, events: &[Event],
    ) -> Result<(), tokio_rusqlite::Error> {
        let session = session.clone();
        let events: Vec<Event> = events.to_vec();
        let source = session.source.to_string();
        let external_id = session.external_id.clone();
        let external_id_for_log = external_id.clone();
        let event_count = events.len();

        self.conn
            .call(move |conn| {
                let tx = conn.transaction()?;

                let existing_id: Option<String> = {
                    let mut stmt = tx.prepare(queries::GET_SESSION_ID_BY_SOURCE_AND_EXTERNAL_ID)?;
                    stmt.query_row([&source, &external_id], |row| row.get(0)).ok()
                };

                let (session_id_to_use, is_update) = match existing_id {
                    Some(id) => (id, true),
                    None => (session.id.to_string(), false),
                };

                if is_update {
                    tx.execute(queries::DELETE_EVENTS_BY_SESSION_ID, [&session_id_to_use])?;
                }

                let id = session_id_to_use.clone();
                let source = session.source.to_string();
                let external_id = session.external_id.clone();
                let project = session.project.clone();
                let title = session.title.clone();
                let created_at = session.created_at.to_rfc3339();
                let updated_at = session.updated_at.to_rfc3339();
                let raw_payload = serde_json::to_string(&session.raw_payload).unwrap_or_default();

                tx.execute(
                    queries::INSERT_SESSION,
                    [
                        id,
                        source,
                        external_id,
                        project.unwrap_or_default(),
                        title.unwrap_or_default(),
                        created_at,
                        updated_at,
                        raw_payload,
                    ],
                )?;

                for event in &events {
                    let id = event.id.to_string();
                    let session_id = uuid::Uuid::parse_str(&session_id_to_use)
                        .map(|uuid| uuid.to_string())
                        .unwrap_or_else(|_| event.session_id.to_string());
                    let kind = event.kind.to_string();
                    let role = event.role.map(|r| r.to_string()).unwrap_or_default();
                    let content = event.content.clone().unwrap_or_default();
                    let timestamp = event.timestamp.to_rfc3339();
                    let raw_payload = serde_json::to_string(&event.raw_payload).unwrap_or_default();

                    tx.execute(
                        queries::INSERT_EVENT,
                        [id, session_id, kind, role, content, timestamp, raw_payload],
                    )?;
                }

                tx.commit()?;
                Ok(())
            })
            .await?;

        info!("Inserted session {} with {} events", external_id_for_log, event_count);

        Ok(())
    }

    /// Look up internal session ID by source and external_id
    pub async fn get_session_id_by_external(
        &self, source: &str, external_id: &str,
    ) -> Result<Option<String>, tokio_rusqlite::Error> {
        let source = source.to_string();
        let external_id = external_id.to_string();
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::GET_SESSION_ID_BY_SOURCE_AND_EXTERNAL_ID)?;
                let id: Option<String> = stmt.query_row([&source, &external_id], |row| row.get(0)).ok();
                Ok(id)
            })
            .await
    }

    /// Append new events to an existing session without deleting existing events
    pub async fn append_events(&self, session_id: &str, events: &[Event]) -> Result<(), tokio_rusqlite::Error> {
        let session_id_owned = session_id.to_string();
        let events: Vec<Event> = events.to_vec();
        let event_count = events.len();
        let session_id_for_log = session_id_owned.clone();

        self.conn
            .call(move |conn| {
                let tx = conn.transaction()?;

                for event in &events {
                    let id = event.id.to_string();
                    let sid = session_id_owned.clone();
                    let kind = event.kind.to_string();
                    let role = event.role.map(|r| r.to_string()).unwrap_or_default();
                    let content = event.content.clone().unwrap_or_default();
                    let timestamp = event.timestamp.to_rfc3339();
                    let raw_payload = serde_json::to_string(&event.raw_payload).unwrap_or_default();

                    tx.execute(
                        queries::APPEND_EVENTS,
                        [id, sid, kind, role, content, timestamp, raw_payload],
                    )?;
                }

                tx.commit()?;
                Ok(())
            })
            .await?;

        info!("Appended {} events to session {}", event_count, session_id_for_log);
        Ok(())
    }

    /// Update a session's updated_at timestamp
    pub async fn update_session_timestamp(
        &self, session_id: &str, updated_at: &chrono::DateTime<Utc>,
    ) -> Result<(), tokio_rusqlite::Error> {
        let id = session_id.to_string();
        let ts = updated_at.to_rfc3339();
        self.conn
            .call(move |conn| {
                conn.execute(queries::UPDATE_SESSION_TIMESTAMP, [&id, &ts])?;
                Ok(())
            })
            .await
    }

    /// Search events with FTS5 and faceted filtering
    pub async fn search_events(
        &self, query: &str, facets: &SearchFacets, limit: i64, offset: i64,
    ) -> Result<Vec<SearchResult>, tokio_rusqlite::Error> {
        let query = query.to_string();
        let source = facets.source.clone();
        let project = facets.project.clone();
        let kind = facets.kind.clone();
        let since = facets.since.map(|dt| dt.to_rfc3339());

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::SEARCH_EVENTS_FILTERED)?;
                let rows = stmt
                    .query_map(
                        [
                            query,
                            source.unwrap_or_default(),
                            project.unwrap_or_default(),
                            kind.unwrap_or_default(),
                            since.unwrap_or_default(),
                            limit.to_string(),
                            offset.to_string(),
                        ],
                        |row| {
                            Ok(SearchResult {
                                event: EventRow {
                                    id: row.get(0)?,
                                    session_id: row.get(1)?,
                                    kind: row.get(2)?,
                                    role: row.get(3)?,
                                    content: row.get(4)?,
                                    timestamp: row.get(5)?,
                                    raw_payload: row.get(6)?,
                                },
                                rank: row.get(7)?,
                                snippet: None,
                            })
                        },
                    )?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Search sessions with FTS5 and faceted filtering
    pub async fn search_sessions(
        &self, query: &str, facets: &SearchFacets, limit: i64, offset: i64,
    ) -> Result<Vec<(SessionRow, f64)>, tokio_rusqlite::Error> {
        let query = query.to_string();
        let source = facets.source.clone();
        let project = facets.project.clone();
        let since = facets.since.map(|dt| dt.to_rfc3339());

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::SEARCH_SESSIONS_FILTERED)?;
                let rows = stmt
                    .query_map(
                        [
                            query,
                            source.unwrap_or_default(),
                            project.unwrap_or_default(),
                            since.unwrap_or_default(),
                            limit.to_string(),
                            offset.to_string(),
                        ],
                        |row| {
                            let session = SessionRow {
                                id: row.get(0)?,
                                source: row.get(1)?,
                                external_id: row.get(2)?,
                                project: row.get(3)?,
                                title: row.get(4)?,
                                created_at: row.get(5)?,
                                updated_at: row.get(6)?,
                                raw_payload: row.get(7)?,
                            };
                            let rank: f64 = row.get(8)?;
                            Ok((session, rank))
                        },
                    )?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get activity stats by day
    pub async fn get_activity_by_day(
        &self, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>, kind: Option<EventKind>,
    ) -> Result<Vec<ActivityStats>, tokio_rusqlite::Error> {
        let since_str = since.map(|dt| dt.to_rfc3339());
        let until_str = until.map(|dt| dt.to_rfc3339());
        let kind_str = kind.map(|k| k.to_string());

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::ACTIVITY_BY_DAY)?;
                let rows = stmt
                    .query_map(
                        [
                            since_str.unwrap_or_default(),
                            until_str.unwrap_or_default(),
                            kind_str.unwrap_or_default(),
                        ],
                        |row| {
                            let day_str: String = row.get(0)?;
                            let day = NaiveDate::parse_from_str(&day_str, "%Y-%m-%d")
                                .unwrap_or_else(|_| Utc::now().date_naive());
                            Ok(ActivityStats { day, event_count: row.get(1)?, session_count: row.get(2)? })
                        },
                    )?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get error stats by day
    pub async fn get_errors_by_day(
        &self, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
    ) -> Result<Vec<ErrorStats>, tokio_rusqlite::Error> {
        let since_str = since.map(|dt| dt.to_rfc3339());
        let until_str = until.map(|dt| dt.to_rfc3339());

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::ERRORS_BY_DAY)?;
                let rows = stmt
                    .query_map([since_str.unwrap_or_default(), until_str.unwrap_or_default()], |row| {
                        let day_str: String = row.get(0)?;
                        let day =
                            NaiveDate::parse_from_str(&day_str, "%Y-%m-%d").unwrap_or_else(|_| Utc::now().date_naive());
                        Ok(ErrorStats { day, error_count: row.get(1)?, signature: row.get(2)? })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get top error signatures
    pub async fn get_top_errors(
        &self, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>, limit: i64,
    ) -> Result<Vec<(String, i64)>, tokio_rusqlite::Error> {
        let since_str = since.map(|dt| dt.to_rfc3339());
        let until_str = until.map(|dt| dt.to_rfc3339());

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::TOP_ERROR_SIGNATURES)?;
                let rows = stmt
                    .query_map(
                        [
                            since_str.unwrap_or_default(),
                            until_str.unwrap_or_default(),
                            limit.to_string(),
                        ],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get stats grouped by source
    pub async fn get_stats_by_source(&self) -> Result<Vec<GroupedStats>, tokio_rusqlite::Error> {
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::STATS_BY_SOURCE)?;
                let rows = stmt
                    .query_map([], |row| {
                        Ok(GroupedStats {
                            dimension: row.get(0)?,
                            count: row.get(1)?,
                            sessions: None,
                            earliest: row.get(2)?,
                            latest: row.get(3)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get stats grouped by project
    pub async fn get_stats_by_project(
        &self, source: Option<String>,
    ) -> Result<Vec<GroupedStats>, tokio_rusqlite::Error> {
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::STATS_BY_PROJECT)?;
                let rows = stmt
                    .query_map([source.unwrap_or_default()], |row| {
                        Ok(GroupedStats {
                            dimension: row.get(0)?,
                            count: row.get(1)?,
                            sessions: None,
                            earliest: row.get(2)?,
                            latest: row.get(3)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get stats grouped by tool kind
    pub async fn get_stats_by_tool(
        &self, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
    ) -> Result<Vec<GroupedStats>, tokio_rusqlite::Error> {
        let since_str = since.map(|dt| dt.to_rfc3339());
        let until_str = until.map(|dt| dt.to_rfc3339());

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::STATS_BY_TOOL)?;
                let rows = stmt
                    .query_map([since_str.unwrap_or_default(), until_str.unwrap_or_default()], |row| {
                        Ok(GroupedStats {
                            dimension: row.get(0)?,
                            count: row.get(1)?,
                            sessions: row.get(2)?,
                            earliest: None,
                            latest: None,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get available sources for faceting
    pub async fn get_sources(&self) -> Result<Vec<String>, tokio_rusqlite::Error> {
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::GET_SOURCES)?;
                let rows = stmt.query_map([], |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get available projects for faceting
    pub async fn get_projects(&self) -> Result<Vec<String>, tokio_rusqlite::Error> {
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::GET_PROJECTS)?;
                let rows = stmt.query_map([], |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get available event kinds for faceting
    pub async fn get_event_kinds(&self) -> Result<Vec<String>, tokio_rusqlite::Error> {
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::GET_EVENT_KINDS)?;
                let rows = stmt.query_map([], |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Insert or update session metrics
    pub async fn upsert_session_metrics(&self, metrics: &SessionMetricsRow) -> Result<(), tokio_rusqlite::Error> {
        let session_id = metrics.session_id.clone();
        let total_events = metrics.total_events;
        let message_count = metrics.message_count;
        let tool_call_count = metrics.tool_call_count;
        let tool_result_count = metrics.tool_result_count;
        let error_count = metrics.error_count;
        let user_messages = metrics.user_messages;
        let assistant_messages = metrics.assistant_messages;
        let duration_seconds = metrics.duration_seconds;
        let files_touched = metrics.files_touched;
        let lines_added = metrics.lines_added;
        let lines_removed = metrics.lines_removed;
        let computed_at = metrics.computed_at.clone();
        let model = metrics.model.clone();
        let provider = metrics.provider.clone();
        let input_tokens = metrics.input_tokens;
        let output_tokens = metrics.output_tokens;
        let estimated_cost = metrics.estimated_cost;
        let total_latency_ms = metrics.total_latency_ms;
        let avg_latency_ms = metrics.avg_latency_ms;
        let p50_latency_ms = metrics.p50_latency_ms;
        let p95_latency_ms = metrics.p95_latency_ms;

        self.conn
            .call(move |conn| {
                conn.execute(
                    queries::UPSERT_SESSION_METRICS,
                    rusqlite::params![
                        session_id,
                        total_events,
                        message_count,
                        tool_call_count,
                        tool_result_count,
                        error_count,
                        user_messages,
                        assistant_messages,
                        duration_seconds,
                        files_touched,
                        lines_added,
                        lines_removed,
                        computed_at,
                        model,
                        provider,
                        input_tokens,
                        output_tokens,
                        estimated_cost,
                        total_latency_ms,
                        avg_latency_ms,
                        p50_latency_ms,
                        p95_latency_ms,
                    ],
                )?;
                Ok(())
            })
            .await
    }

    /// Compute and store metrics for a session
    pub async fn compute_session_metrics(&self, session_id: &str) -> Result<(), tokio_rusqlite::Error> {
        let session_id_str = session_id.to_string();
        let session = self.get_session(session_id_str.clone()).await?;
        let events = self.get_session_events(session_id_str.clone()).await?;

        if session.is_none() {
            return Ok(());
        }
        let session = session.unwrap();

        let file_stats: (i64, i64, i64) = self
            .conn
            .call({
                let sid = session_id_str.clone();
                move |conn| {
                    let mut stmt = conn.prepare(
                        "SELECT COUNT(DISTINCT file_path), SUM(lines_added), SUM(lines_removed)
                     FROM files_touched WHERE session_id = ?1",
                    )?;
                    stmt.query_row([sid], |row| {
                        Ok((
                            row.get::<_, i64>(0).unwrap_or(0),
                            row.get::<_, i64>(1).unwrap_or(0),
                            row.get::<_, i64>(2).unwrap_or(0),
                        ))
                    })
                    .map_err(|e| e.into())
                }
            })
            .await
            .unwrap_or((0, 0, 0));

        let latency_stats: (Option<i64>, Option<f64>) = self
            .conn
            .call({
                let sid = session_id_str.clone();
                move |conn| {
                    let mut stmt = conn.prepare(
                        "SELECT SUM(duration_ms), AVG(duration_ms)
                     FROM tool_calls WHERE session_id = ?1 AND duration_ms IS NOT NULL",
                    )?;
                    stmt.query_row([sid], |row| Ok((row.get(0).ok(), row.get(1).ok())))
                        .map_err(|e| e.into())
                }
            })
            .await
            .unwrap_or((None, None));

        let mut metrics = SessionMetricsRow {
            session_id: session_id_str,
            total_events: events.len() as i64,
            message_count: 0,
            tool_call_count: 0,
            tool_result_count: 0,
            error_count: 0,
            user_messages: 0,
            assistant_messages: 0,
            duration_seconds: None,
            files_touched: file_stats.0,
            lines_added: file_stats.1,
            lines_removed: file_stats.2,
            computed_at: Utc::now().to_rfc3339(),
            model: None,
            provider: None,
            input_tokens: None,
            output_tokens: None,
            estimated_cost: None,
            total_latency_ms: latency_stats.0,
            avg_latency_ms: latency_stats.1,
            p50_latency_ms: None,
            p95_latency_ms: None,
        };

        let mut input_tokens = 0;
        let mut output_tokens = 0;
        let mut model_name: Option<String> = None;

        if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&session.raw_payload) {
            if let Some(m) = payload.get("model").and_then(|v| v.as_str()) {
                model_name = Some(m.to_string());
            } else if let Some(m) = payload
                .get("metadata")
                .and_then(|meta| meta.get("model"))
                .and_then(|v| v.as_str())
            {
                model_name = Some(m.to_string());
            }
        }

        for event in events {
            match event.kind.as_str() {
                "message" => {
                    metrics.message_count += 1;
                    if event.role.as_deref() == Some("user") {
                        metrics.user_messages += 1;
                        if let Some(content) = &event.content {
                            input_tokens += ModelMetadata::estimate_tokens(content);
                        }
                    } else if event.role.as_deref() == Some("assistant") {
                        metrics.assistant_messages += 1;
                        if let Some(content) = &event.content {
                            output_tokens += ModelMetadata::estimate_tokens(content);
                        }
                    }
                }
                "tool_call" => metrics.tool_call_count += 1,
                "tool_result" => metrics.tool_result_count += 1,
                "error" => metrics.error_count += 1,
                _ => {}
            }

            if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&event.raw_payload) {
                if model_name.is_none() {
                    if let Some(m) = payload.get("model").and_then(|v| v.as_str()) {
                        model_name = Some(m.to_string());
                    } else if let Some(m) = payload
                        .get("message")
                        .and_then(|msg| msg.get("model"))
                        .and_then(|v| v.as_str())
                    {
                        model_name = Some(m.to_string());
                    } else if let Some(model_obj) = payload.get("model") {
                        if let Some(m) = model_obj.get("modelID").and_then(|v| v.as_str()) {
                            model_name = Some(m.to_string());
                        } else if let Some(m) = model_obj.get("model_id").and_then(|v| v.as_str()) {
                            model_name = Some(m.to_string());
                        }
                    }
                }

                if let Some(usage) = payload.get("message").and_then(|msg| msg.get("usage")) {
                    if let Some(it) = usage.get("input_tokens").and_then(|v| v.as_i64()) {
                        input_tokens = it as usize;
                    }
                    if let Some(ot) = usage.get("output_tokens").and_then(|v| v.as_i64()) {
                        output_tokens = ot as usize;
                    }
                }

                if let Some(usage) = payload.get("usage") {
                    if let Some(it) = usage.get("prompt_tokens").and_then(|v| v.as_i64()) {
                        input_tokens = it as usize;
                    }
                    if let Some(ot) = usage.get("completion_tokens").and_then(|v| v.as_i64()) {
                        output_tokens = ot as usize;
                    }
                }

                if let Some(info) = payload.get("info")
                    && let Some(token_usage) = info.get("total_token_usage")
                {
                    if let Some(it) = token_usage.get("input_tokens").and_then(|v| v.as_i64()) {
                        input_tokens = it as usize;
                    }
                    if let Some(ot) = token_usage.get("output_tokens").and_then(|v| v.as_i64()) {
                        output_tokens = ot as usize;
                    }
                }
            }
        }

        metrics.model = model_name.clone();
        metrics.input_tokens = Some(input_tokens as i64);
        metrics.output_tokens = Some(output_tokens as i64);

        if let Some(m) = model_name
            && let Some(meta) = ModelMetadata::lookup(&m)
        {
            metrics.provider = Some(meta.provider.clone());
            metrics.estimated_cost = Some(meta.calculate_cost(input_tokens, output_tokens));
        }

        self.upsert_session_metrics(&metrics).await?;

        Ok(())
    }

    /// Get tool call frequency stats
    pub async fn get_tool_call_frequency(
        &self, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
    ) -> Result<Vec<ToolFrequencyStats>, tokio_rusqlite::Error> {
        let since_str = since.map(|dt| dt.to_rfc3339());
        let until_str = until.map(|dt| dt.to_rfc3339());

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::TOOL_CALL_FREQUENCY)?;
                let rows = stmt
                    .query_map([since_str.unwrap_or_default(), until_str.unwrap_or_default()], |row| {
                        Ok(ToolFrequencyStats {
                            tool_name: row.get(0)?,
                            call_count: row.get(1)?,
                            sessions: row.get(2)?,
                            avg_duration_ms: row.get(3)?,
                            max_duration_ms: row.get(4)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get files touched leaderboard
    pub async fn get_files_leaderboard(
        &self, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>, limit: i64,
    ) -> Result<Vec<FileLeaderboardEntry>, tokio_rusqlite::Error> {
        let since_str = since.map(|dt| dt.to_rfc3339());
        let until_str = until.map(|dt| dt.to_rfc3339());

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::FILES_TOUCHED_LEADERBOARD)?;
                let rows = stmt
                    .query_map(
                        [
                            since_str.unwrap_or_default(),
                            until_str.unwrap_or_default(),
                            limit.to_string(),
                        ],
                        |row| {
                            Ok(FileLeaderboardEntry {
                                file_path: row.get(0)?,
                                touch_count: row.get(1)?,
                                sessions: row.get(2)?,
                                total_lines_added: row.get(3)?,
                                total_lines_removed: row.get(4)?,
                            })
                        },
                    )?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get patch churn stats by day
    pub async fn get_patch_churn_by_day(
        &self, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
    ) -> Result<Vec<PatchChurnStats>, tokio_rusqlite::Error> {
        let since_str = since.map(|dt| dt.to_rfc3339());
        let until_str = until.map(|dt| dt.to_rfc3339());

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::PATCH_CHURN_BY_DAY)?;
                let rows = stmt
                    .query_map([since_str.unwrap_or_default(), until_str.unwrap_or_default()], |row| {
                        let day_str: String = row.get(0)?;
                        let day =
                            NaiveDate::parse_from_str(&day_str, "%Y-%m-%d").unwrap_or_else(|_| Utc::now().date_naive());
                        Ok(PatchChurnStats {
                            day,
                            lines_added: row.get(1)?,
                            lines_removed: row.get(2)?,
                            files_changed: row.get(3)?,
                            sessions: row.get(4)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get long-running tool calls
    pub async fn get_long_running_tool_calls(
        &self, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>, min_duration_ms: i64, limit: i64,
    ) -> Result<Vec<LongRunningToolCall>, tokio_rusqlite::Error> {
        let since_str = since.map(|dt| dt.to_rfc3339());
        let until_str = until.map(|dt| dt.to_rfc3339());

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::LONG_RUNNING_TOOL_CALLS)?;
                let rows = stmt
                    .query_map(
                        [
                            since_str.unwrap_or_default(),
                            until_str.unwrap_or_default(),
                            min_duration_ms.to_string(),
                            limit.to_string(),
                        ],
                        |row| {
                            Ok(LongRunningToolCall {
                                tool_name: row.get(0)?,
                                duration_ms: row.get(1)?,
                                started_at: row.get(2)?,
                                session_external_id: row.get(3)?,
                                project: row.get(4)?,
                                error_message: row.get(5)?,
                            })
                        },
                    )?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get session metrics
    pub async fn get_session_metrics(
        &self, session_id: &str,
    ) -> Result<Option<SessionMetricsRow>, tokio_rusqlite::Error> {
        let session_id = session_id.to_string();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::GET_SESSION_METRICS)?;
                let row = stmt
                    .query_row([session_id], |row| {
                        Ok(SessionMetricsRow {
                            session_id: row.get(0)?,
                            total_events: row.get(1)?,
                            message_count: row.get(2)?,
                            tool_call_count: row.get(3)?,
                            tool_result_count: row.get(4)?,
                            error_count: row.get(5)?,
                            user_messages: row.get(6)?,
                            assistant_messages: row.get(7)?,
                            duration_seconds: row.get(8)?,
                            files_touched: row.get(9)?,
                            lines_added: row.get(10)?,
                            lines_removed: row.get(11)?,
                            computed_at: row.get(12)?,
                            model: row.get(13)?,
                            provider: row.get(14)?,
                            input_tokens: row.get(15)?,
                            output_tokens: row.get(16)?,
                            estimated_cost: row.get(17)?,
                            total_latency_ms: row.get(18)?,
                            avg_latency_ms: row.get(19)?,
                            p50_latency_ms: row.get(20)?,
                            p95_latency_ms: row.get(21)?,
                        })
                    })
                    .optional()?;
                Ok(row)
            })
            .await
    }

    /// Get all sessions with their metrics for export
    pub async fn get_sessions_with_metrics(
        &self, limit: i64, offset: i64,
    ) -> Result<Vec<(SessionRow, Option<SessionMetricsRow>)>, tokio_rusqlite::Error> {
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::GET_SESSIONS_WITH_METRICS)?;
                let rows = stmt
                    .query_map([limit, offset], |row| {
                        let session = SessionRow {
                            id: row.get(0)?,
                            source: row.get(1)?,
                            external_id: row.get(2)?,
                            project: row.get(3)?,
                            title: row.get(4)?,
                            created_at: row.get(5)?,
                            updated_at: row.get(6)?,
                            raw_payload: row.get(7)?,
                        };
                        let metrics: Option<SessionMetricsRow> = if row.get::<_, Option<i64>>(8)?.is_some() {
                            Some(SessionMetricsRow {
                                session_id: session.id.clone(),
                                total_events: row.get(8)?,
                                message_count: row.get(9)?,
                                tool_call_count: row.get(10)?,
                                tool_result_count: row.get(11)?,
                                error_count: row.get(12)?,
                                user_messages: row.get(13)?,
                                assistant_messages: row.get(14)?,
                                duration_seconds: row.get(15)?,
                                files_touched: row.get(16)?,
                                lines_added: row.get(17)?,
                                lines_removed: row.get(18)?,
                                computed_at: row.get(19)?,
                                model: row.get(20)?,
                                provider: row.get(21)?,
                                input_tokens: row.get(22)?,
                                output_tokens: row.get(23)?,
                                estimated_cost: row.get(24)?,
                                total_latency_ms: row.get(25)?,
                                avg_latency_ms: row.get(26)?,
                                p50_latency_ms: row.get(27)?,
                                p95_latency_ms: row.get(28)?,
                            })
                        } else {
                            None
                        };
                        Ok((session, metrics))
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get cost stats by source
    pub async fn get_cost_stats_by_source(
        &self, source_filter: Option<String>, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
    ) -> Result<Vec<CostStats>, tokio_rusqlite::Error> {
        let source = source_filter.unwrap_or_default();
        let since_str = since.map(|dt| dt.to_rfc3339()).unwrap_or_default();
        let until_str = until.map(|dt| dt.to_rfc3339()).unwrap_or_default();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::COST_STATS_BY_SOURCE)?;
                let rows = stmt
                    .query_map([source, since_str, until_str], |row| {
                        Ok(CostStats {
                            dimension: row.get(0)?,
                            session_count: row.get(1)?,
                            total_cost: row.get(2)?,
                            avg_cost_per_session: row.get(3)?,
                            total_input_tokens: row.get(4)?,
                            total_output_tokens: row.get(5)?,
                            avg_latency_ms: row.get(6)?,
                            p50_latency_ms: row.get(7)?,
                            p95_latency_ms: row.get(8)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get cost stats by project
    pub async fn get_cost_stats_by_project(
        &self, source_filter: Option<String>, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
    ) -> Result<Vec<CostStats>, tokio_rusqlite::Error> {
        let source = source_filter.unwrap_or_default();
        let since_str = since.map(|dt| dt.to_rfc3339()).unwrap_or_default();
        let until_str = until.map(|dt| dt.to_rfc3339()).unwrap_or_default();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::COST_STATS_BY_PROJECT)?;
                let rows = stmt
                    .query_map([source, since_str, until_str], |row| {
                        Ok(CostStats {
                            dimension: row.get(0)?,
                            session_count: row.get(1)?,
                            total_cost: row.get(2)?,
                            avg_cost_per_session: row.get(3)?,
                            total_input_tokens: row.get(4)?,
                            total_output_tokens: row.get(5)?,
                            avg_latency_ms: row.get(6)?,
                            p50_latency_ms: row.get(7)?,
                            p95_latency_ms: row.get(8)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get cost stats by session
    pub async fn get_cost_stats_by_session(
        &self, source_filter: Option<String>, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>, limit: i64,
        offset: i64,
    ) -> Result<Vec<SessionCostStats>, tokio_rusqlite::Error> {
        let source = source_filter.unwrap_or_default();
        let since_str = since.map(|dt| dt.to_rfc3339()).unwrap_or_default();
        let until_str = until.map(|dt| dt.to_rfc3339()).unwrap_or_default();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::COST_STATS_BY_SESSION)?;
                let rows = stmt
                    .query_map(
                        [source, since_str, until_str, limit.to_string(), offset.to_string()],
                        |row| {
                            Ok(SessionCostStats {
                                session_id: row.get(0)?,
                                external_id: row.get(1)?,
                                project: row.get(2)?,
                                source: row.get(3)?,
                                estimated_cost: row.get(4)?,
                                input_tokens: row.get(5)?,
                                output_tokens: row.get(6)?,
                                avg_latency_ms: row.get(7)?,
                                p50_latency_ms: row.get(8)?,
                                p95_latency_ms: row.get(9)?,
                                duration_seconds: row.get(10)?,
                                computed_at: row.get(11)?,
                            })
                        },
                    )?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get latency distribution stats
    pub async fn get_latency_distribution(
        &self, source_filter: Option<String>, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
    ) -> Result<LatencyDistribution, tokio_rusqlite::Error> {
        let source = source_filter.unwrap_or_default();
        let since_str = since.map(|dt| dt.to_rfc3339()).unwrap_or_default();
        let until_str = until.map(|dt| dt.to_rfc3339()).unwrap_or_default();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::LATENCY_DISTRIBUTION)?;
                let row = stmt.query_row([source, since_str, until_str], |row| {
                    Ok(LatencyDistribution {
                        avg_latency: row.get(0)?,
                        p50_latency: row.get(1)?,
                        p95_latency: row.get(2)?,
                        max_p95: row.get(3)?,
                        session_count: row.get(4)?,
                    })
                })?;
                Ok(row)
            })
            .await
    }

    /// Get model usage stats
    pub async fn get_model_usage_stats(
        &self, source_filter: Option<String>, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
    ) -> Result<Vec<ModelUsageStats>, tokio_rusqlite::Error> {
        let source = source_filter.unwrap_or_default();
        let since_str = since.map(|dt| dt.to_rfc3339()).unwrap_or_default();
        let until_str = until.map(|dt| dt.to_rfc3339()).unwrap_or_default();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::MODEL_USAGE_STATS)?;
                let rows = stmt
                    .query_map([source, since_str, until_str], |row| {
                        Ok(ModelUsageStats {
                            model: row.get(0)?,
                            provider: row.get(1)?,
                            session_count: row.get(2)?,
                            total_input_tokens: row.get(3)?,
                            total_output_tokens: row.get(4)?,
                            total_cost: row.get(5)?,
                            avg_latency_ms: row.get(6)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .await
    }

    /// Get aggregate efficiency stats
    pub async fn get_efficiency_stats(
        &self, source_filter: Option<String>, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
    ) -> Result<EfficiencyStats, tokio_rusqlite::Error> {
        let source = source_filter.unwrap_or_default();
        let since_str = since.map(|dt| dt.to_rfc3339()).unwrap_or_default();
        let until_str = until.map(|dt| dt.to_rfc3339()).unwrap_or_default();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(queries::EFFICIENCY_STATS)?;
                let row = stmt.query_row([source, since_str, until_str], |row| {
                    Ok(EfficiencyStats {
                        total_sessions: row.get(0)?,
                        total_cost: row.get(1)?,
                        avg_cost_per_session: row.get(2)?,
                        tool_error_rate: row.get(3)?,
                        retry_loops: row.get(4)?,
                        p50_latency_ms: row.get(5)?,
                        p95_latency_ms: row.get(6)?,
                    })
                })?;
                Ok(row)
            })
            .await
    }
}

/// Stats for tool call frequency
#[derive(Debug, Clone)]
pub struct ToolFrequencyStats {
    pub tool_name: String,
    pub call_count: i64,
    pub sessions: i64,
    pub avg_duration_ms: Option<f64>,
    pub max_duration_ms: Option<i64>,
}

/// Entry in the files touched leaderboard
#[derive(Debug, Clone)]
pub struct FileLeaderboardEntry {
    pub file_path: String,
    pub touch_count: i64,
    pub sessions: i64,
    pub total_lines_added: i64,
    pub total_lines_removed: i64,
}

/// Patch churn stats for a day
#[derive(Debug, Clone)]
pub struct PatchChurnStats {
    pub day: NaiveDate,
    pub lines_added: i64,
    pub lines_removed: i64,
    pub files_changed: i64,
    pub sessions: i64,
}

/// Long-running tool call entry
#[derive(Debug, Clone)]
pub struct LongRunningToolCall {
    pub tool_name: String,
    pub duration_ms: i64,
    pub started_at: String,
    pub session_external_id: String,
    pub project: Option<String>,
    pub error_message: Option<String>,
}

/// Cost and latency stats by source or project
#[derive(Debug, Clone)]
pub struct CostStats {
    pub dimension: String,
    pub session_count: i64,
    pub total_cost: Option<f64>,
    pub avg_cost_per_session: Option<f64>,
    pub total_input_tokens: Option<i64>,
    pub total_output_tokens: Option<i64>,
    pub avg_latency_ms: Option<f64>,
    pub p50_latency_ms: Option<f64>,
    pub p95_latency_ms: Option<f64>,
}

/// Cost stats for a single session
#[derive(Debug, Clone)]
pub struct SessionCostStats {
    pub session_id: String,
    pub external_id: String,
    pub project: Option<String>,
    pub source: String,
    pub estimated_cost: Option<f64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub avg_latency_ms: Option<f64>,
    pub p50_latency_ms: Option<i64>,
    pub p95_latency_ms: Option<i64>,
    pub duration_seconds: Option<i64>,
    pub computed_at: String,
}

/// Latency distribution summary
#[derive(Debug, Clone)]
pub struct LatencyDistribution {
    pub avg_latency: Option<f64>,
    pub p50_latency: Option<f64>,
    pub p95_latency: Option<f64>,
    pub max_p95: Option<i64>,
    pub session_count: i64,
}

/// Model/provider usage stats
#[derive(Debug, Clone)]
pub struct ModelUsageStats {
    pub model: String,
    pub provider: String,
    pub session_count: i64,
    pub total_input_tokens: Option<i64>,
    pub total_output_tokens: Option<i64>,
    pub total_cost: Option<f64>,
    pub avg_latency_ms: Option<f64>,
}

/// Aggregate efficiency stats
#[derive(Debug, Clone)]
pub struct EfficiencyStats {
    pub total_sessions: i64,
    pub total_cost: f64,
    pub avg_cost_per_session: f64,
    pub tool_error_rate: f64,
    pub retry_loops: i64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
}

/// Check health of all configured data sources
pub async fn check_sources_health() -> Vec<SourceHealth> {
    let mut results = Vec::new();
    results.push(check_claude_health().await);
    results.push(check_codex_health().await);
    results.push(check_opencode_health().await);
    results.push(check_crush_health().await);
    results
}

async fn check_claude_health() -> SourceHealth {
    let claude_dir = dirs::home_dir().map(|h| h.join(".claude").join("projects"));

    match claude_dir {
        Some(path) if path.exists() => SourceHealth {
            source: Source::Claude,
            status: HealthStatus::Healthy,
            path: Some(path.to_string_lossy().to_string()),
            message: Some(format!("Found {} projects", count_projects(&path).await)),
        },
        Some(path) => SourceHealth {
            source: Source::Claude,
            status: HealthStatus::Unknown,
            path: Some(path.to_string_lossy().to_string()),
            message: Some("Claude projects directory not found".to_string()),
        },
        None => SourceHealth {
            source: Source::Claude,
            status: HealthStatus::Unknown,
            path: None,
            message: Some("Could not determine home directory".to_string()),
        },
    }
}

async fn check_codex_health() -> SourceHealth {
    let codex_home: Option<PathBuf> = std::env::var("CODEX_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".codex")));

    match codex_home {
        Some(path) if path.exists() => SourceHealth {
            source: Source::Codex,
            status: HealthStatus::Healthy,
            path: Some(path.to_string_lossy().to_string()),
            message: Some("Codex home found".to_string()),
        },
        Some(path) => SourceHealth {
            source: Source::Codex,
            status: HealthStatus::Unknown,
            path: Some(path.to_string_lossy().to_string()),
            message: Some("Codex home not found".to_string()),
        },
        None => SourceHealth {
            source: Source::Codex,
            status: HealthStatus::Unknown,
            path: None,
            message: Some("Could not determine Codex home".to_string()),
        },
    }
}

async fn check_opencode_health() -> SourceHealth {
    let candidates = opencode_candidate_paths();
    let existing = candidates.iter().find(|path| path.exists()).cloned();
    let primary = candidates.first().cloned();

    match existing {
        Some(path) => {
            let storage_path = path.join("storage");
            let log_path = path.join("log");
            SourceHealth {
                source: Source::OpenCode,
                status: HealthStatus::Healthy,
                path: Some(path.to_string_lossy().to_string()),
                message: Some(format!(
                    "OpenCode data found (storage: {}, log: {})",
                    storage_path.exists(),
                    log_path.exists()
                )),
            }
        }
        None => SourceHealth {
            source: Source::OpenCode,
            status: HealthStatus::Unknown,
            path: primary.map(|p| p.to_string_lossy().to_string()),
            message: Some(format!(
                "OpenCode data not found (checked: {})",
                candidates
                    .iter()
                    .map(|p| p.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        },
    }
}

fn opencode_candidate_paths() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".local/share/opencode"));
    }

    if let Some(local_data) = dirs::data_local_dir() {
        let fallback = local_data.join("opencode");
        if !candidates.iter().any(|p| p == &fallback) {
            candidates.push(fallback);
        }
    }

    if candidates.is_empty() {
        candidates.push(PathBuf::from("~/.local/share/opencode"));
    }

    candidates
}

async fn check_crush_health() -> SourceHealth {
    let crush_global = dirs::home_dir().map(|h| h.join(".crush"));

    match crush_global {
        Some(path) if path.exists() => SourceHealth {
            source: Source::Crush,
            status: HealthStatus::Healthy,
            path: Some(path.to_string_lossy().to_string()),
            message: Some("Global Crush database found".to_string()),
        },
        Some(path) => SourceHealth {
            source: Source::Crush,
            status: HealthStatus::Unknown,
            path: Some(path.to_string_lossy().to_string()),
            message: Some("Global Crush database not found".to_string()),
        },
        None => SourceHealth {
            source: Source::Crush,
            status: HealthStatus::Unknown,
            path: None,
            message: Some("Could not determine home directory".to_string()),
        },
    }
}

async fn count_projects(path: &std::path::Path) -> usize {
    match tokio::fs::read_dir(path).await {
        Ok(mut entries) => {
            let mut count = 0;
            while let Ok(Some(_)) = entries.next_entry().await {
                count += 1;
            }
            count
        }
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_v_core::{Event, EventKind, Role, Session, Source};
    use uuid::Uuid;

    async fn setup_test_db() -> Database {
        let db = Database::open(":memory:").await.unwrap();
        db.migrate().await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_compute_session_metrics() {
        let db = setup_test_db().await;
        let session_id = Uuid::new_v4();

        let session = Session {
            id: session_id,
            source: Source::Claude,
            external_id: "ext-1".to_string(),
            project: Some("test-project".to_string()),
            title: Some("Test Session".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            raw_payload: serde_json::json!({"model": "claude-4.5-sonnet"}),
        };

        let events = vec![
            Event {
                id: Uuid::new_v4(),
                session_id,
                kind: EventKind::Message,
                role: Some(Role::User),
                content: Some("Hello".to_string()),
                timestamp: Utc::now(),
                raw_payload: serde_json::json!({}),
            },
            Event {
                id: Uuid::new_v4(),
                session_id,
                kind: EventKind::Message,
                role: Some(Role::Assistant),
                content: Some("Hi there! I am a 2026 model.".to_string()),
                timestamp: Utc::now(),
                raw_payload: serde_json::json!({"usage": {"prompt_tokens": 10, "completion_tokens": 20}}),
            },
        ];

        db.insert_session_with_events(&session, &events).await.unwrap();

        let s = db.get_session(session_id.to_string()).await.unwrap();
        assert!(s.is_some(), "Session should be in DB");

        let evs = db.get_session_events(session_id.to_string()).await.unwrap();
        assert_eq!(evs.len(), 2, "Should have 2 events in DB");

        db.compute_session_metrics(&session_id.to_string()).await.unwrap();

        let metrics_opt = db.get_session_metrics(&session_id.to_string()).await.unwrap();
        if metrics_opt.is_none() {
            let exists: bool = db
                .conn
                .call({
                    let sid = session_id.to_string();
                    move |conn| {
                        let mut stmt = conn.prepare("SELECT COUNT(*) FROM session_metrics WHERE session_id = ?1")?;
                        let count: i64 = stmt.query_row([sid], |row| row.get(0))?;
                        Ok(count > 0)
                    }
                })
                .await
                .unwrap();
            panic!("Metrics not found for {}. Row exists: {}", session_id, exists);
        }
        let metrics = metrics_opt.unwrap();

        assert_eq!(metrics.total_events, 2);
        assert_eq!(metrics.message_count, 2);
        assert_eq!(metrics.user_messages, 1);
        assert_eq!(metrics.assistant_messages, 1);
        assert_eq!(metrics.model, Some("claude-4.5-sonnet".to_string()));
        assert_eq!(metrics.provider, Some("anthropic".to_string()));

        assert_eq!(metrics.input_tokens, Some(10));
        assert_eq!(metrics.output_tokens, Some(20));

        assert!(metrics.estimated_cost.unwrap() > 0.0);
    }

    #[tokio::test]
    async fn test_prune_duplicate_sessions_merges_unique_events_and_dedupes_overlaps() {
        let db = Database::open(":memory:").await.unwrap();

        db.conn
            .call(|conn| {
                conn.execute_batch(
                    r#"
                    CREATE TABLE sessions (
                        id TEXT PRIMARY KEY,
                        source TEXT NOT NULL,
                        external_id TEXT NOT NULL,
                        project TEXT,
                        title TEXT,
                        created_at TIMESTAMP NOT NULL,
                        updated_at TIMESTAMP NOT NULL,
                        raw_payload TEXT NOT NULL
                    );

                    CREATE TABLE events (
                        id TEXT PRIMARY KEY,
                        session_id TEXT NOT NULL,
                        kind TEXT NOT NULL,
                        role TEXT,
                        content TEXT,
                        timestamp TIMESTAMP NOT NULL,
                        raw_payload TEXT NOT NULL
                    );
                    "#,
                )?;
                Ok(())
            })
            .await
            .unwrap();

        db.migrate().await.unwrap();

        let keep_id = Uuid::new_v4().to_string();
        let duplicate_id = Uuid::new_v4().to_string();
        let source = "claude".to_string();
        let external_id = "dup-ext-id".to_string();

        db.conn
            .call({
                let keep_id = keep_id.clone();
                let duplicate_id = duplicate_id.clone();
                let source = source.clone();
                let external_id = external_id.clone();
                move |conn| {
                    conn.execute(
                        "INSERT INTO sessions (id, source, external_id, project, title, created_at, updated_at, raw_payload)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        rusqlite::params![
                            &duplicate_id,
                            &source,
                            &external_id,
                            "proj",
                            "Older",
                            "2026-03-01T00:00:00Z",
                            "2026-03-02T00:00:00Z",
                            "{}"
                        ],
                    )?;

                    conn.execute(
                        "INSERT INTO sessions (id, source, external_id, project, title, created_at, updated_at, raw_payload)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        rusqlite::params![
                            &keep_id,
                            "claude",
                            "dup-ext-id",
                            "proj",
                            "Richer",
                            "2026-03-01T00:00:00Z",
                            "2026-03-04T00:00:00Z",
                            "{}"
                        ],
                    )?;

                    conn.execute(
                        "INSERT INTO events (id, session_id, kind, role, content, timestamp, raw_payload)
                         VALUES (?1, ?2, 'message', 'user', 'a', '2026-03-01T00:00:00Z', '{}')",
                        rusqlite::params![Uuid::new_v4().to_string(), &duplicate_id],
                    )?;
                    conn.execute(
                        "INSERT INTO events (id, session_id, kind, role, content, timestamp, raw_payload)
                         VALUES (?1, ?2, 'message', 'assistant', 'donor-unique', '2026-03-01T00:02:00Z', '{}')",
                        rusqlite::params![Uuid::new_v4().to_string(), &duplicate_id],
                    )?;

                    conn.execute(
                        "INSERT INTO events (id, session_id, kind, role, content, timestamp, raw_payload)
                         VALUES (?1, ?2, 'message', 'user', 'a', '2026-03-01T00:00:00Z', '{}')",
                        rusqlite::params![Uuid::new_v4().to_string(), &keep_id],
                    )?;
                    conn.execute(
                        "INSERT INTO events (id, session_id, kind, role, content, timestamp, raw_payload)
                         VALUES (?1, ?2, 'message', 'assistant', 'b', '2026-03-01T00:01:00Z', '{}')",
                        rusqlite::params![Uuid::new_v4().to_string(), &keep_id],
                    )?;

                    Ok(())
                }
            })
            .await
            .unwrap();

        let pruned = db.prune_duplicate_sessions().await.unwrap();
        assert_eq!(pruned, 1);

        let sessions = db.list_sessions(10, 0).await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, keep_id);
        assert_eq!(sessions[0].created_at, "2026-03-01T00:00:00Z");
        assert_eq!(sessions[0].updated_at, "2026-03-04T00:00:00Z");

        let events = db.get_session_events(keep_id).await.unwrap();
        assert_eq!(events.len(), 3);
        assert!(events.iter().any(|e| e.content.as_deref() == Some("donor-unique")));
    }

    #[tokio::test]
    async fn test_prune_duplicate_sessions_does_not_merge_across_sources() {
        let db = setup_test_db().await;
        let shared_external_id = "shared-id";

        let claude_session = Session {
            id: Uuid::new_v4(),
            source: Source::Claude,
            external_id: shared_external_id.to_string(),
            project: Some("p1".to_string()),
            title: Some("Claude".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            raw_payload: serde_json::json!({}),
        };
        let codex_session = Session {
            id: Uuid::new_v4(),
            source: Source::Codex,
            external_id: shared_external_id.to_string(),
            project: Some("p2".to_string()),
            title: Some("Codex".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            raw_payload: serde_json::json!({}),
        };

        let claude_events = vec![Event {
            id: Uuid::new_v4(),
            session_id: claude_session.id,
            kind: EventKind::Message,
            role: Some(Role::User),
            content: Some("claude".to_string()),
            timestamp: Utc::now(),
            raw_payload: serde_json::json!({}),
        }];
        let codex_events = vec![Event {
            id: Uuid::new_v4(),
            session_id: codex_session.id,
            kind: EventKind::Message,
            role: Some(Role::User),
            content: Some("codex".to_string()),
            timestamp: Utc::now(),
            raw_payload: serde_json::json!({}),
        }];

        db.insert_session_with_events(&claude_session, &claude_events)
            .await
            .unwrap();
        db.insert_session_with_events(&codex_session, &codex_events)
            .await
            .unwrap();

        let pruned = db.prune_duplicate_sessions().await.unwrap();
        assert_eq!(pruned, 0);

        let sessions = db.list_sessions(10, 0).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }
}
