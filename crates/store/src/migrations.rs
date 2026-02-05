/// Migration definition
pub struct Migration {
    pub name: &'static str,
    pub sql: &'static str,
}

/// All database migrations in order
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        name: "001_initial_schema",
        sql: r#"
            -- Sessions table
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL,
                external_id TEXT NOT NULL,
                project TEXT,
                title TEXT,
                created_at TIMESTAMP NOT NULL,
                updated_at TIMESTAMP NOT NULL,
                raw_payload TEXT NOT NULL,
                UNIQUE(source, external_id)
            );

            -- Events table
            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                role TEXT,
                content TEXT,
                timestamp TIMESTAMP NOT NULL,
                raw_payload TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            -- Indexes for common queries
            CREATE INDEX IF NOT EXISTS idx_sessions_source ON sessions(source);
            CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project);
            CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at);
            CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(session_id);
            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind);
        "#,
    },
    Migration {
        name: "002_fts5_virtual_tables",
        sql: r#"
            -- FTS5 virtual table for full-text search on events
            CREATE VIRTUAL TABLE IF NOT EXISTS events_fts USING fts5(
                content,
                content_rowid=id,
                tokenize='porter'
            );

            -- Trigger to keep FTS index in sync
            CREATE TRIGGER IF NOT EXISTS events_fts_insert AFTER INSERT ON events BEGIN
                INSERT INTO events_fts(rowid, content) VALUES (new.rowid, new.content);
            END;

            CREATE TRIGGER IF NOT EXISTS events_fts_delete AFTER DELETE ON events BEGIN
                INSERT INTO events_fts(events_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
            END;

            CREATE TRIGGER IF NOT EXISTS events_fts_update AFTER UPDATE ON events BEGIN
                INSERT INTO events_fts(events_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
                INSERT INTO events_fts(rowid, content) VALUES (new.rowid, new.content);
            END;

            -- FTS5 virtual table for session titles
            CREATE VIRTUAL TABLE IF NOT EXISTS sessions_fts USING fts5(
                title,
                content_rowid=id,
                tokenize='porter'
            );

            -- Trigger to keep FTS index in sync for sessions
            CREATE TRIGGER IF NOT EXISTS sessions_fts_insert AFTER INSERT ON sessions BEGIN
                INSERT INTO sessions_fts(rowid, title) VALUES (new.rowid, new.title);
            END;

            CREATE TRIGGER IF NOT EXISTS sessions_fts_delete AFTER DELETE ON sessions BEGIN
                INSERT INTO sessions_fts(sessions_fts, rowid, title) VALUES ('delete', old.rowid, old.title);
            END;

            CREATE TRIGGER IF NOT EXISTS sessions_fts_update AFTER UPDATE ON sessions BEGIN
                INSERT INTO sessions_fts(sessions_fts, rowid, title) VALUES ('delete', old.rowid, old.title);
                INSERT INTO sessions_fts(rowid, title) VALUES (new.rowid, new.title);
            END;
        "#,
    },
];
