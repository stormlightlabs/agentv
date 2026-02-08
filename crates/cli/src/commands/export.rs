use agent_v_store::{Database, EventRow, SearchFacets, SessionMetricsRow, SessionRow};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::io::Write;

/// Export a single session
pub async fn export_session(
    session_id: String, format: ExportFormat, output: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open_default().await?;
    db.migrate().await?;

    let mut session = None;
    let mut offset = 0;
    loop {
        let sessions = db.list_sessions(100, offset).await?;
        if sessions.is_empty() {
            break;
        }
        if let Some(found) = sessions
            .into_iter()
            .find(|s| s.id == session_id || s.external_id == session_id)
        {
            session = Some(found);
            break;
        }
        offset += 100;
    }

    let session = match session {
        Some(s) => s,
        None => {
            return Err(format!("Session not found: {}", session_id).into());
        }
    };

    let events = db.get_session_events(session.id.clone()).await?;
    let metrics = db.get_session_metrics(&session.id).await?;

    match format {
        ExportFormat::Markdown => {
            let md = export_session_to_markdown(&session, &events, metrics.as_ref()).await?;
            write_output(&md, output.as_deref())?;
        }
        ExportFormat::Json => {
            let json = export_session_to_json(&session, &events, metrics.as_ref()).await?;
            write_output(&json, output.as_deref())?;
        }
        ExportFormat::Jsonl => {
            let jsonl = export_session_to_jsonl(&session, &events).await?;
            write_output(&jsonl, output.as_deref())?;
        }
    }

    Ok(())
}

/// Export search results
pub async fn export_search(
    query: String, source: Option<String>, since: Option<String>, kind: Option<String>, format: ExportFormat,
    output: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open_default().await?;
    db.migrate().await?;

    let facets =
        SearchFacets { source, project: None, kind, since: since.and_then(|s| parse_since(&s).ok().flatten()) };

    let results = db.search_events(&query, &facets, 10000, 0).await?;

    match format {
        ExportFormat::Markdown => {
            let md = export_search_to_markdown(&query, &results).await?;
            write_output(&md, output.as_deref())?;
        }
        ExportFormat::Json => {
            let json = export_search_to_json(&query, &results).await?;
            write_output(&json, output.as_deref())?;
        }
        ExportFormat::Jsonl => {
            let jsonl = export_search_to_jsonl(&query, &results).await?;
            write_output(&jsonl, output.as_deref())?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Markdown,
    Json,
    Jsonl,
}

impl ExportFormat {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "md" | "markdown" => Ok(ExportFormat::Markdown),
            "json" => Ok(ExportFormat::Json),
            "jsonl" => Ok(ExportFormat::Jsonl),
            _ => Err(format!("Unknown format: {}. Use 'md', 'json', or 'jsonl'", s)),
        }
    }
}

fn write_output(content: &str, output_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    match output_path {
        Some(path) => {
            let mut file = std::fs::File::create(path)?;
            file.write_all(content.as_bytes())?;
            println!("Exported to: {}", path);
        }
        None => {
            println!("{}", content);
        }
    }
    Ok(())
}

async fn export_session_to_markdown(
    session: &SessionRow, events: &[EventRow], metrics: Option<&SessionMetricsRow>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut md = String::new();

    md.push_str(&format!(
        "# Session: {}\n\n",
        session.title.as_deref().unwrap_or("Untitled")
    ));
    md.push_str(&format!("- **ID**: {}\n", session.external_id));
    md.push_str(&format!("- **Source**: {}\n", session.source));
    md.push_str(&format!(
        "- **Project**: {}\n",
        session.project.as_deref().unwrap_or("N/A")
    ));
    md.push_str(&format!("- **Created**: {}\n", session.created_at));
    md.push_str(&format!("- **Updated**: {}\n", session.updated_at));

    if let Some(m) = metrics {
        md.push_str("\n## Cost & Efficiency\n\n");
        if let Some(cost) = m.estimated_cost {
            md.push_str(&format!("- **Estimated Cost**: ${:.4}\n", cost));
        }
        if let Some(model) = &m.model {
            md.push_str(&format!("- **Model**: {}\n", model));
        }
        if let Some(provider) = &m.provider {
            md.push_str(&format!("- **Provider**: {}\n", provider));
        }
        if let (Some(input), Some(output)) = (m.input_tokens, m.output_tokens) {
            md.push_str(&format!("- **Tokens**: {} input / {} output\n", input, output));
        }
        if let Some(duration) = m.duration_seconds {
            md.push_str(&format!("- **Duration**: {}s\n", duration));
        }
        if let (Some(p50), Some(p95)) = (m.p50_latency_ms, m.p95_latency_ms) {
            md.push_str(&format!("- **Latency**: p50={}ms, p95={}ms\n", p50, p95));
        }
        md.push_str(&format!("- **Total Events**: {}\n", m.total_events));
        md.push_str(&format!(
            "- **Messages**: {} user / {} assistant\n",
            m.user_messages, m.assistant_messages
        ));
        md.push_str(&format!("- **Tool Calls**: {}\n", m.tool_call_count));
        md.push_str(&format!("- **Errors**: {}\n", m.error_count));
    }

    md.push_str("\n## Events\n\n");

    for event in events {
        md.push_str(&format!("### {} - {}\n\n", event.timestamp, event.kind));
        if let Some(ref role) = event.role {
            md.push_str(&format!("**Role**: {}\n\n", role));
        }
        if let Some(ref content) = event.content {
            md.push_str("```\n");
            md.push_str(content);
            md.push_str("\n```\n\n");
        }
    }

    Ok(md)
}

#[derive(Serialize)]
struct SessionExport {
    id: String,
    source: String,
    external_id: String,
    project: Option<String>,
    title: Option<String>,
    created_at: String,
    updated_at: String,
    events: Vec<EventExport>,
    metrics: Option<SessionMetricsExport>,
}

#[derive(Serialize)]
struct SessionMetricsExport {
    total_events: i64,
    message_count: i64,
    tool_call_count: i64,
    tool_result_count: i64,
    error_count: i64,
    user_messages: i64,
    assistant_messages: i64,
    duration_seconds: Option<i64>,
    files_touched: i64,
    lines_added: i64,
    lines_removed: i64,
    model: Option<String>,
    provider: Option<String>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    estimated_cost: Option<f64>,
    total_latency_ms: Option<i64>,
    avg_latency_ms: Option<f64>,
    p50_latency_ms: Option<i64>,
    p95_latency_ms: Option<i64>,
}

#[derive(Serialize)]
struct EventExport {
    id: String,
    kind: String,
    role: Option<String>,
    content: Option<String>,
    timestamp: String,
    raw_payload: serde_json::Value,
}

async fn export_session_to_json(
    session: &SessionRow, events: &[EventRow], metrics: Option<&SessionMetricsRow>,
) -> Result<String, Box<dyn std::error::Error>> {
    let metrics_export = metrics.map(|m| SessionMetricsExport {
        total_events: m.total_events,
        message_count: m.message_count,
        tool_call_count: m.tool_call_count,
        tool_result_count: m.tool_result_count,
        error_count: m.error_count,
        user_messages: m.user_messages,
        assistant_messages: m.assistant_messages,
        duration_seconds: m.duration_seconds,
        files_touched: m.files_touched,
        lines_added: m.lines_added,
        lines_removed: m.lines_removed,
        model: m.model.clone(),
        provider: m.provider.clone(),
        input_tokens: m.input_tokens,
        output_tokens: m.output_tokens,
        estimated_cost: m.estimated_cost,
        total_latency_ms: m.total_latency_ms,
        avg_latency_ms: m.avg_latency_ms,
        p50_latency_ms: m.p50_latency_ms,
        p95_latency_ms: m.p95_latency_ms,
    });

    let export = SessionExport {
        id: session.id.clone(),
        source: session.source.clone(),
        external_id: session.external_id.clone(),
        project: session.project.clone(),
        title: session.title.clone(),
        created_at: session.created_at.clone(),
        updated_at: session.updated_at.clone(),
        events: events
            .iter()
            .map(|e| EventExport {
                id: e.id.clone(),
                kind: e.kind.clone(),
                role: e.role.clone(),
                content: e.content.clone(),
                timestamp: e.timestamp.clone(),
                raw_payload: serde_json::from_str(&e.raw_payload).unwrap_or(serde_json::Value::Null),
            })
            .collect(),
        metrics: metrics_export,
    };

    Ok(serde_json::to_string_pretty(&export)?)
}

async fn export_session_to_jsonl(
    _session: &SessionRow, events: &[EventRow],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut lines = Vec::new();

    for event in events {
        let obj = serde_json::json!({
            "id": event.id,
            "session_id": event.session_id,
            "kind": event.kind,
            "role": event.role,
            "content": event.content,
            "timestamp": event.timestamp,
            "raw_payload": serde_json::from_str::<serde_json::Value>(&event.raw_payload).unwrap_or(serde_json::Value::Null),
        });
        lines.push(serde_json::to_string(&obj)?);
    }

    Ok(lines.join("\n"))
}

#[derive(Serialize)]
struct SearchResultExport {
    query: String,
    results: Vec<SearchEventExport>,
    total: usize,
}

#[derive(Serialize)]
struct SearchEventExport {
    event_id: String,
    session_id: String,
    kind: String,
    role: Option<String>,
    content: Option<String>,
    timestamp: String,
    rank: f64,
}

async fn export_search_to_markdown(
    query: &str, results: &[agent_v_store::SearchResult],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut md = String::new();

    md.push_str(&format!("# Search Results: \"{}\"\n\n", query));
    md.push_str(&format!("**{} results**\n\n", results.len()));

    for (idx, result) in results.iter().enumerate() {
        md.push_str(&format!("## Result {}\n\n", idx + 1));
        md.push_str(&format!("- **Event ID**: {}\n", result.event.id));
        md.push_str(&format!("- **Session ID**: {}\n", result.event.session_id));
        md.push_str(&format!("- **Kind**: {}\n", result.event.kind));
        md.push_str(&format!("- **Timestamp**: {}\n", result.event.timestamp));
        md.push_str(&format!("- **Rank**: {:.4}\n\n", result.rank));

        if let Some(ref content) = result.event.content {
            md.push_str("```\n");
            md.push_str(content);
            md.push_str("\n```\n\n");
        }
    }

    Ok(md)
}

async fn export_search_to_json(
    query: &str, results: &[agent_v_store::SearchResult],
) -> Result<String, Box<dyn std::error::Error>> {
    let export = SearchResultExport {
        query: query.to_string(),
        total: results.len(),
        results: results
            .iter()
            .map(|r| SearchEventExport {
                event_id: r.event.id.clone(),
                session_id: r.event.session_id.clone(),
                kind: r.event.kind.clone(),
                role: r.event.role.clone(),
                content: r.event.content.clone(),
                timestamp: r.event.timestamp.clone(),
                rank: r.rank,
            })
            .collect(),
    };

    Ok(serde_json::to_string_pretty(&export)?)
}

async fn export_search_to_jsonl(
    _query: &str, results: &[agent_v_store::SearchResult],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut lines = Vec::new();

    for result in results {
        let obj = serde_json::json!({
            "event_id": result.event.id,
            "session_id": result.event.session_id,
            "kind": result.event.kind,
            "role": result.event.role,
            "content": result.event.content,
            "timestamp": result.event.timestamp,
            "rank": result.rank,
        });
        lines.push(serde_json::to_string(&obj)?);
    }

    Ok(lines.join("\n"))
}

fn parse_since(since: &str) -> Result<Option<DateTime<Utc>>, String> {
    if since.is_empty() {
        return Ok(None);
    }

    let err = Err(format!("Invalid duration format: {}. Use Nd, Nh, Nw", since));

    let duration = if since.ends_with('d') {
        since
            .strip_suffix('d')
            .and_then(|days| days.parse().ok())
            .map(chrono::Duration::days)
    } else if since.ends_with('h') {
        since
            .strip_suffix('h')
            .and_then(|hours| hours.parse().ok())
            .map(chrono::Duration::hours)
    } else if since.ends_with('w') {
        since
            .strip_suffix('w')
            .and_then(|weeks| weeks.parse().ok())
            .map(chrono::Duration::weeks)
    } else {
        return err;
    };

    match duration {
        Some(duration) => Ok(Some(Utc::now() - duration)),
        None => Ok(None),
    }
}
