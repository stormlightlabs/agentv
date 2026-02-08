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
                content='events',
                content_rowid='rowid',
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
                content='sessions',
                content_rowid='rowid',
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
    Migration {
        name: "003_session_metrics_and_tool_calls",
        sql: r#"
            -- Session metrics table for computed analytics
            CREATE TABLE IF NOT EXISTS session_metrics (
                session_id TEXT PRIMARY KEY,
                total_events INTEGER NOT NULL DEFAULT 0,
                message_count INTEGER NOT NULL DEFAULT 0,
                tool_call_count INTEGER NOT NULL DEFAULT 0,
                tool_result_count INTEGER NOT NULL DEFAULT 0,
                error_count INTEGER NOT NULL DEFAULT 0,
                user_messages INTEGER NOT NULL DEFAULT 0,
                assistant_messages INTEGER NOT NULL DEFAULT 0,
                duration_seconds INTEGER,
                files_touched INTEGER NOT NULL DEFAULT 0,
                lines_added INTEGER NOT NULL DEFAULT 0,
                lines_removed INTEGER NOT NULL DEFAULT 0,
                computed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            -- Tool calls table for detailed tool usage analytics
            CREATE TABLE IF NOT EXISTS tool_calls (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                event_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                started_at TIMESTAMP NOT NULL,
                completed_at TIMESTAMP,
                duration_ms INTEGER,
                success BOOLEAN,
                error_message TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
                FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE
            );

            -- Files touched table for file operation tracking
            CREATE TABLE IF NOT EXISTS files_touched (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                operation TEXT NOT NULL,
                lines_added INTEGER NOT NULL DEFAULT 0,
                lines_removed INTEGER NOT NULL DEFAULT 0,
                touched_at TIMESTAMP NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            -- Indexes for performance
            CREATE INDEX IF NOT EXISTS idx_session_metrics_session ON session_metrics(session_id);
            CREATE INDEX IF NOT EXISTS idx_tool_calls_session ON tool_calls(session_id);
            CREATE INDEX IF NOT EXISTS idx_tool_calls_name ON tool_calls(tool_name);
            CREATE INDEX IF NOT EXISTS idx_tool_calls_duration ON tool_calls(duration_ms);
            CREATE INDEX IF NOT EXISTS idx_files_touched_session ON files_touched(session_id);
            CREATE INDEX IF NOT EXISTS idx_files_touched_path ON files_touched(file_path);
        "#,
    },
    Migration {
        name: "004_add_events_composite_index",
        sql: r#"
            -- Composite index for activity queries filtering by kind and timestamp
            CREATE INDEX IF NOT EXISTS idx_events_kind_timestamp ON events(kind, timestamp);
        "#,
    },
    Migration {
        name: "005_add_cost_and_latency_metrics",
        sql: r#"
            -- Add cost and latency metrics to session_metrics table
            ALTER TABLE session_metrics ADD COLUMN model TEXT;
            ALTER TABLE session_metrics ADD COLUMN provider TEXT;
            ALTER TABLE session_metrics ADD COLUMN input_tokens INTEGER;
            ALTER TABLE session_metrics ADD COLUMN output_tokens INTEGER;
            ALTER TABLE session_metrics ADD COLUMN estimated_cost REAL;
            ALTER TABLE session_metrics ADD COLUMN total_latency_ms INTEGER;
            ALTER TABLE session_metrics ADD COLUMN avg_latency_ms REAL;
            ALTER TABLE session_metrics ADD COLUMN p50_latency_ms INTEGER;
            ALTER TABLE session_metrics ADD COLUMN p95_latency_ms INTEGER;

            -- Index for cost queries
            CREATE INDEX IF NOT EXISTS idx_session_metrics_cost ON session_metrics(estimated_cost);
            CREATE INDEX IF NOT EXISTS idx_session_metrics_model ON session_metrics(model);
            CREATE INDEX IF NOT EXISTS idx_session_metrics_provider ON session_metrics(provider);
        "#,
    },
];
