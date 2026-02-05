/// List all sessions ordered by updated_at desc
pub const LIST_SESSIONS: &str = r#"
    SELECT 
        id,
        source,
        external_id,
        project,
        title,
        created_at,
        updated_at,
        raw_payload
    FROM sessions
    ORDER BY updated_at DESC
    LIMIT ?1 OFFSET ?2
"#;

/// Get events for a specific session
pub const GET_SESSION_EVENTS: &str = r#"
    SELECT 
        id,
        session_id,
        kind,
        role,
        content,
        timestamp,
        raw_payload
    FROM events
    WHERE session_id = ?1
    ORDER BY timestamp ASC
"#;

/// Search events using FTS5
pub const SEARCH_EVENTS: &str = r#"
    SELECT 
        e.id,
        e.session_id,
        e.kind,
        e.role,
        e.content,
        e.timestamp,
        e.raw_payload,
        rank
    FROM events_fts f
    JOIN events e ON e.rowid = f.rowid
    WHERE events_fts MATCH ?1
    ORDER BY rank
    LIMIT ?2 OFFSET ?3
"#;

/// Search sessions using FTS5
pub const SEARCH_SESSIONS: &str = r#"
    SELECT 
        s.id,
        s.source,
        s.external_id,
        s.project,
        s.title,
        s.created_at,
        s.updated_at,
        s.raw_payload,
        rank
    FROM sessions_fts f
    JOIN sessions s ON s.rowid = f.rowid
    WHERE sessions_fts MATCH ?1
    ORDER BY rank
    LIMIT ?2 OFFSET ?3
"#;

/// Insert a new session
pub const INSERT_SESSION: &str = r#"
    INSERT INTO sessions (id, source, external_id, project, title, created_at, updated_at, raw_payload)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
    ON CONFLICT(source, external_id) DO UPDATE SET
        title = excluded.title,
        updated_at = excluded.updated_at,
        raw_payload = excluded.raw_payload
"#;

/// Insert a new event
pub const INSERT_EVENT: &str = r#"
    INSERT INTO events (id, session_id, kind, role, content, timestamp, raw_payload)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
"#;
