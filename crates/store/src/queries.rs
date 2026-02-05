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

/// Search events using FTS5 with faceted filtering
pub const SEARCH_EVENTS_FILTERED: &str = r#"
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
    JOIN sessions s ON e.session_id = s.id
    WHERE events_fts MATCH ?1
        AND (?2 = '' OR s.source = ?2)
        AND (?3 = '' OR s.project = ?3)
        AND (?4 = '' OR e.kind = ?4)
        AND (?5 = '' OR e.timestamp >= ?5)
    ORDER BY rank
    LIMIT ?6 OFFSET ?7
"#;

/// Search sessions using FTS5 with faceted filtering
pub const SEARCH_SESSIONS_FILTERED: &str = r#"
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
        AND (?2 = '' OR s.source = ?2)
        AND (?3 = '' OR s.project = ?3)
        AND (?4 = '' OR s.created_at >= ?4)
    ORDER BY rank
    LIMIT ?5 OFFSET ?6
"#;

/// Get activity stats by day
pub const ACTIVITY_BY_DAY: &str = r#"
    SELECT
        DATE(timestamp) as day,
        COUNT(*) as event_count,
        COUNT(DISTINCT session_id) as session_count
    FROM events
    WHERE (?1 = '' OR timestamp >= ?1)
        AND (?2 = '' OR timestamp < ?2)
        AND (?3 = '' OR kind = ?3)
    GROUP BY DATE(timestamp)
    ORDER BY day DESC
"#;

/// Get error stats by day
pub const ERRORS_BY_DAY: &str = r#"
    SELECT
        DATE(timestamp) as day,
        COUNT(*) as error_count,
        content
    FROM events
    WHERE kind = 'error'
        AND (?1 = '' OR timestamp >= ?1)
        AND (?2 = '' OR timestamp < ?2)
    GROUP BY DATE(timestamp), content
    ORDER BY day DESC, error_count DESC
"#;

/// Get top error signatures
pub const TOP_ERROR_SIGNATURES: &str = r#"
    SELECT
        COALESCE(content, 'Unknown error') as signature,
        COUNT(*) as count
    FROM events
    WHERE kind = 'error'
        AND (?1 = '' OR timestamp >= ?1)
        AND (?2 = '' OR timestamp < ?2)
    GROUP BY content
    ORDER BY count DESC
    LIMIT ?3
"#;

/// Get stats by source
pub const STATS_BY_SOURCE: &str = r#"
    SELECT
        source,
        COUNT(*) as session_count,
        MIN(created_at) as earliest,
        MAX(updated_at) as latest
    FROM sessions
    GROUP BY source
    ORDER BY session_count DESC
"#;

/// Get stats by project
pub const STATS_BY_PROJECT: &str = r#"
    SELECT
        COALESCE(project, 'Unknown') as project,
        COUNT(*) as session_count,
        MIN(created_at) as earliest,
        MAX(updated_at) as latest
    FROM sessions
    WHERE (?1 = '' OR source = ?1)
    GROUP BY project
    ORDER BY session_count DESC
"#;

/// Get stats by tool kind
pub const STATS_BY_TOOL: &str = r#"
    SELECT
        kind,
        COUNT(*) as count,
        COUNT(DISTINCT session_id) as sessions
    FROM events
    WHERE (?1 = '' OR timestamp >= ?1)
        AND (?2 = '' OR timestamp < ?2)
    GROUP BY kind
    ORDER BY count DESC
"#;

/// Get distinct sources for faceting
pub const GET_SOURCES: &str = r#"
    SELECT DISTINCT source FROM sessions ORDER BY source
"#;

/// Get distinct projects for faceting
pub const GET_PROJECTS: &str = r#"
    SELECT DISTINCT project FROM sessions WHERE project IS NOT NULL ORDER BY project
"#;

/// Get distinct event kinds for faceting
pub const GET_EVENT_KINDS: &str = r#"
    SELECT DISTINCT kind FROM events ORDER BY kind
"#;
