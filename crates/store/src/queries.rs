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

/// List sessions with optional source filter
pub const LIST_SESSIONS_FILTERED: &str = r#"
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
    WHERE (?1 = '' OR source = ?1)
    ORDER BY updated_at DESC
    LIMIT ?2 OFFSET ?3
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

/// Insert or update session metrics
pub const UPSERT_SESSION_METRICS: &str = r#"
    INSERT INTO session_metrics (
        session_id, total_events, message_count, tool_call_count, tool_result_count,
        error_count, user_messages, assistant_messages, duration_seconds,
        files_touched, lines_added, lines_removed, computed_at,
        model, provider, input_tokens, output_tokens, estimated_cost,
        total_latency_ms, avg_latency_ms, p50_latency_ms, p95_latency_ms
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)
    ON CONFLICT(session_id) DO UPDATE SET
        total_events = excluded.total_events,
        message_count = excluded.message_count,
        tool_call_count = excluded.tool_call_count,
        tool_result_count = excluded.tool_result_count,
        error_count = excluded.error_count,
        user_messages = excluded.user_messages,
        assistant_messages = excluded.assistant_messages,
        duration_seconds = excluded.duration_seconds,
        files_touched = excluded.files_touched,
        lines_added = excluded.lines_added,
        lines_removed = excluded.lines_removed,
        computed_at = excluded.computed_at,
        model = excluded.model,
        provider = excluded.provider,
        input_tokens = excluded.input_tokens,
        output_tokens = excluded.output_tokens,
        estimated_cost = excluded.estimated_cost,
        total_latency_ms = excluded.total_latency_ms,
        avg_latency_ms = excluded.avg_latency_ms,
        p50_latency_ms = excluded.p50_latency_ms,
        p95_latency_ms = excluded.p95_latency_ms
"#;

/// Insert a tool call record
pub const INSERT_TOOL_CALL: &str = r#"
    INSERT INTO tool_calls (
        id, session_id, event_id, tool_name, started_at, completed_at, duration_ms, success, error_message
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
    ON CONFLICT(id) DO UPDATE SET
        completed_at = excluded.completed_at,
        duration_ms = excluded.duration_ms,
        success = excluded.success,
        error_message = excluded.error_message
"#;

/// Insert a file touched record
pub const INSERT_FILE_TOUCHED: &str = r#"
    INSERT INTO files_touched (
        id, session_id, file_path, operation, lines_added, lines_removed, touched_at
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
    ON CONFLICT(id) DO UPDATE SET
        lines_added = excluded.lines_added,
        lines_removed = excluded.lines_removed
"#;

/// Get tool call frequency stats
pub const TOOL_CALL_FREQUENCY: &str = r#"
    SELECT
        tool_name,
        COUNT(*) as call_count,
        COUNT(DISTINCT session_id) as sessions,
        AVG(CASE WHEN duration_ms IS NOT NULL THEN duration_ms END) as avg_duration_ms,
        MAX(duration_ms) as max_duration_ms
    FROM tool_calls
    WHERE (?1 = '' OR started_at >= ?1)
        AND (?2 = '' OR started_at < ?2)
    GROUP BY tool_name
    ORDER BY call_count DESC
"#;

/// Get files touched leaderboard
pub const FILES_TOUCHED_LEADERBOARD: &str = r#"
    SELECT
        file_path,
        COUNT(*) as touch_count,
        COUNT(DISTINCT session_id) as sessions,
        SUM(lines_added) as total_lines_added,
        SUM(lines_removed) as total_lines_removed
    FROM files_touched
    WHERE (?1 = '' OR touched_at >= ?1)
        AND (?2 = '' OR touched_at < ?2)
    GROUP BY file_path
    ORDER BY touch_count DESC
    LIMIT ?3
"#;

/// Get patch churn stats (lines added/removed) by day
pub const PATCH_CHURN_BY_DAY: &str = r#"
    SELECT
        DATE(touched_at) as day,
        SUM(lines_added) as lines_added,
        SUM(lines_removed) as lines_removed,
        COUNT(DISTINCT file_path) as files_changed,
        COUNT(DISTINCT session_id) as sessions
    FROM files_touched
    WHERE (?1 = '' OR touched_at >= ?1)
        AND (?2 = '' OR touched_at < ?2)
    GROUP BY DATE(touched_at)
    ORDER BY day DESC
"#;

/// Get long-running tool calls (latency tracking)
pub const LONG_RUNNING_TOOL_CALLS: &str = r#"
    SELECT
        tc.tool_name,
        tc.duration_ms,
        tc.started_at,
        s.external_id as session_external_id,
        s.project,
        tc.error_message
    FROM tool_calls tc
    JOIN sessions s ON tc.session_id = s.id
    WHERE tc.duration_ms IS NOT NULL
        AND (?1 = '' OR tc.started_at >= ?1)
        AND (?2 = '' OR tc.started_at < ?2)
        AND tc.duration_ms >= ?3
    ORDER BY tc.duration_ms DESC
    LIMIT ?4
"#;

/// Get session metrics for a specific session
pub const GET_SESSION_METRICS: &str = r#"
    SELECT
        session_id, total_events, message_count, tool_call_count, tool_result_count,
        error_count, user_messages, assistant_messages, duration_seconds,
        files_touched, lines_added, lines_removed, computed_at,
        model, provider, input_tokens, output_tokens, estimated_cost,
        total_latency_ms, avg_latency_ms, p50_latency_ms, p95_latency_ms
    FROM session_metrics
    WHERE session_id = ?1
"#;

/// Get aggregated session metrics summary
pub const GET_SESSION_METRICS_SUMMARY: &str = r#"
    SELECT
        COUNT(DISTINCT session_id) as total_sessions,
        SUM(total_events) as total_events,
        SUM(tool_call_count) as total_tool_calls,
        SUM(error_count) as total_errors,
        SUM(files_touched) as total_files_touched,
        SUM(lines_added) as total_lines_added,
        SUM(lines_removed) as total_lines_removed,
        AVG(duration_seconds) as avg_duration_seconds
    FROM session_metrics
"#;

/// Get session ID by source and external_id
pub const GET_SESSION_ID_BY_SOURCE_AND_EXTERNAL_ID: &str = r#"
    SELECT id FROM sessions WHERE source = ?1 AND external_id = ?2
"#;

/// Delete events for a session
pub const DELETE_EVENTS_BY_SESSION_ID: &str = r#"
    DELETE FROM events WHERE session_id = ?1
"#;

/// Get sessions with their metrics for export
pub const GET_SESSIONS_WITH_METRICS: &str = r#"
    SELECT
        s.id, s.source, s.external_id, s.project, s.title, s.created_at, s.updated_at, s.raw_payload,
        m.total_events, m.message_count, m.tool_call_count, m.tool_result_count,
        m.error_count, m.user_messages, m.assistant_messages, m.duration_seconds,
        m.files_touched, m.lines_added, m.lines_removed, m.computed_at,
        m.model, m.provider, m.input_tokens, m.output_tokens, m.estimated_cost,
        m.total_latency_ms, m.avg_latency_ms, m.p50_latency_ms, m.p95_latency_ms
    FROM sessions s
    LEFT JOIN session_metrics m ON s.id = m.session_id
    ORDER BY s.updated_at DESC
    LIMIT ?1 OFFSET ?2
"#;

/// Get cost stats by source
pub const COST_STATS_BY_SOURCE: &str = r#"
    SELECT
        s.source,
        COUNT(DISTINCT m.session_id) as session_count,
        SUM(m.estimated_cost) as total_cost,
        AVG(m.estimated_cost) as avg_cost_per_session,
        SUM(m.input_tokens) as total_input_tokens,
        SUM(m.output_tokens) as total_output_tokens,
        AVG(m.avg_latency_ms) as avg_latency_ms,
        AVG(m.p50_latency_ms) as p50_latency_ms,
        AVG(m.p95_latency_ms) as p95_latency_ms
    FROM session_metrics m
    JOIN sessions s ON m.session_id = s.id
    WHERE (?1 = '' OR s.source = ?1)
        AND (?2 = '' OR m.computed_at >= ?2)
        AND (?3 = '' OR m.computed_at < ?3)
    GROUP BY s.source
    ORDER BY total_cost DESC
"#;

/// Get cost stats by project
pub const COST_STATS_BY_PROJECT: &str = r#"
    SELECT
        COALESCE(s.project, 'Unknown') as project,
        COUNT(DISTINCT m.session_id) as session_count,
        SUM(m.estimated_cost) as total_cost,
        AVG(m.estimated_cost) as avg_cost_per_session,
        SUM(m.input_tokens) as total_input_tokens,
        SUM(m.output_tokens) as total_output_tokens,
        AVG(m.avg_latency_ms) as avg_latency_ms,
        AVG(m.p50_latency_ms) as p50_latency_ms,
        AVG(m.p95_latency_ms) as p95_latency_ms
    FROM session_metrics m
    JOIN sessions s ON m.session_id = s.id
    WHERE (?1 = '' OR s.source = ?1)
        AND (?2 = '' OR m.computed_at >= ?2)
        AND (?3 = '' OR m.computed_at < ?3)
    GROUP BY s.project
    ORDER BY total_cost DESC
"#;

/// Get cost stats by session
pub const COST_STATS_BY_SESSION: &str = r#"
    SELECT
        s.id,
        s.external_id,
        s.project,
        s.source,
        m.estimated_cost,
        m.input_tokens,
        m.output_tokens,
        m.avg_latency_ms,
        m.p50_latency_ms,
        m.p95_latency_ms,
        m.duration_seconds,
        m.computed_at
    FROM session_metrics m
    JOIN sessions s ON m.session_id = s.id
    WHERE (?1 = '' OR s.source = ?1)
        AND (?2 = '' OR m.computed_at >= ?2)
        AND (?3 = '' OR m.computed_at < ?3)
    ORDER BY m.estimated_cost DESC NULLS LAST
    LIMIT ?4 OFFSET ?5
"#;

/// Get latency distribution stats (p50, p95, p99)
pub const LATENCY_DISTRIBUTION: &str = r#"
    SELECT
        AVG(m.avg_latency_ms) as avg_latency,
        AVG(m.p50_latency_ms) as p50_latency,
        AVG(m.p95_latency_ms) as p95_latency,
        MAX(m.p95_latency_ms) as max_p95,
        COUNT(*) as session_count
    FROM session_metrics m
    JOIN sessions s ON m.session_id = s.id
    WHERE (?1 = '' OR s.source = ?1)
        AND (?2 = '' OR m.computed_at >= ?2)
        AND (?3 = '' OR m.computed_at < ?3)
        AND m.p95_latency_ms IS NOT NULL
"#;

/// Get model/provider usage stats
pub const MODEL_USAGE_STATS: &str = r#"
    SELECT
        COALESCE(m.model, 'unknown') as model,
        COALESCE(m.provider, 'unknown') as provider,
        COUNT(DISTINCT m.session_id) as session_count,
        SUM(m.input_tokens) as total_input_tokens,
        SUM(m.output_tokens) as total_output_tokens,
        SUM(m.estimated_cost) as total_cost,
        AVG(m.avg_latency_ms) as avg_latency_ms
    FROM session_metrics m
    JOIN sessions s ON m.session_id = s.id
    WHERE (?1 = '' OR s.source = ?1)
        AND (?2 = '' OR m.computed_at >= ?2)
        AND (?3 = '' OR m.computed_at < ?3)
    GROUP BY m.model, m.provider
    ORDER BY total_cost DESC NULLS LAST
"#;

/// Get aggregate efficiency stats
pub const EFFICIENCY_STATS: &str = r#"
    SELECT
        COUNT(DISTINCT m.session_id) as total_sessions,
        COALESCE(SUM(m.estimated_cost), 0.0) as total_cost,
        COALESCE(AVG(m.estimated_cost), 0.0) as avg_cost_per_session,
        COALESCE(
            CAST((SELECT COUNT(*) FROM tool_calls tc JOIN sessions s2 ON tc.session_id = s2.id
                  WHERE tc.success = 0
                  AND (?1 = '' OR s2.source = ?1)
                  AND (?2 = '' OR tc.started_at >= ?2)
                  AND (?3 = '' OR tc.started_at < ?3)) AS REAL) /
            NULLIF((SELECT COUNT(*) FROM tool_calls tc JOIN sessions s2 ON tc.session_id = s2.id
                  WHERE (?1 = '' OR s2.source = ?1)
                  AND (?2 = '' OR tc.started_at >= ?2)
                  AND (?3 = '' OR tc.started_at < ?3)), 0),
            0.0
        ) as tool_error_rate,
        (SELECT COUNT(DISTINCT m2.session_id) FROM session_metrics m2
         JOIN sessions s2 ON m2.session_id = s2.id
         WHERE m2.error_count > 2
         AND (?1 = '' OR s2.source = ?1)
         AND (?2 = '' OR m2.computed_at >= ?2)
         AND (?3 = '' OR m2.computed_at < ?3)) as retry_loops,
        COALESCE(AVG(m.p50_latency_ms), 0.0) as p50_latency_ms,
        COALESCE(AVG(m.p95_latency_ms), 0.0) as p95_latency_ms
    FROM session_metrics m
    JOIN sessions s ON m.session_id = s.id
    WHERE (?1 = '' OR s.source = ?1)
        AND (?2 = '' OR m.computed_at >= ?2)
        AND (?3 = '' OR m.computed_at < ?3)
"#;
