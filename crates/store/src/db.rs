use agent_viz_core::{Event, HealthStatus, Session, Source, SourceHealth};
use std::path::PathBuf;
use tokio_rusqlite::Connection;
use tracing::{error, info};

use crate::migrations::MIGRATIONS;
use crate::models::{EventRow, SessionRow};
use crate::queries;

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

    /// Insert a session with all its events in a transaction
    pub async fn insert_session_with_events(
        &self, session: &Session, events: &[Event],
    ) -> Result<(), tokio_rusqlite::Error> {
        self.insert_session(session).await?;

        for event in events {
            self.insert_event(event).await?;
        }

        info!("Inserted session {} with {} events", session.external_id, events.len());

        Ok(())
    }
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
    let opencode_dir = dirs::data_local_dir().map(|d| d.join("opencode"));

    match opencode_dir {
        Some(path) if path.exists() => SourceHealth {
            source: Source::OpenCode,
            status: HealthStatus::Healthy,
            path: Some(path.to_string_lossy().to_string()),
            message: Some("OpenCode data found".to_string()),
        },
        Some(path) => SourceHealth {
            source: Source::OpenCode,
            status: HealthStatus::Unknown,
            path: Some(path.to_string_lossy().to_string()),
            message: Some("OpenCode data not found".to_string()),
        },
        None => SourceHealth {
            source: Source::OpenCode,
            status: HealthStatus::Unknown,
            path: None,
            message: Some("Could not determine local data directory".to_string()),
        },
    }
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
