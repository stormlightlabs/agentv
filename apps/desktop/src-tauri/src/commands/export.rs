use serde::Serialize;

/// Export a session to Markdown format
pub async fn export_session_to_markdown(
    session: &agent_v_store::SessionRow, events: &[agent_v_store::EventRow],
    metrics: Option<&agent_v_store::SessionMetricsRow>,
) -> Result<String, String> {
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
    md.push_str(&format!("- **Updated**: {}\n\n", session.updated_at));

    if let Some(m) = metrics {
        md.push_str("## Cost & Efficiency\n\n");
        if let Some(cost) = m.estimated_cost {
            md.push_str(&format!("- **Estimated Cost**: ${:.4}\n", cost));
        }
        if let (Some(provider), Some(model)) = (&m.provider, &m.model) {
            md.push_str(&format!("- **Model**: {}/{}\n", provider, model));
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
        md.push_str(&format!("- **Files Touched**: {}\n\n", m.files_touched));
    }

    md.push_str("## Events\n\n");

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

/// Export a session to JSON format
pub async fn export_session_to_json(
    session: &agent_v_store::SessionRow, events: &[agent_v_store::EventRow],
    metrics: Option<&agent_v_store::SessionMetricsRow>,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct EventExport {
        id: String,
        kind: String,
        role: Option<String>,
        content: Option<String>,
        timestamp: String,
        raw_payload: serde_json::Value,
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

    serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
}

/// Export a session to JSONL format
pub async fn export_session_to_jsonl(
    session: &agent_v_store::SessionRow, events: &[agent_v_store::EventRow],
    metrics: Option<&agent_v_store::SessionMetricsRow>,
) -> Result<String, String> {
    let mut lines = Vec::new();

    let metadata = serde_json::json!({
        "type": "session_metadata",
        "id": session.id,
        "source": session.source,
        "external_id": session.external_id,
        "project": session.project,
        "title": session.title,
        "created_at": session.created_at,
        "updated_at": session.updated_at,
        "metrics": metrics.map(|m| serde_json::json!({
            "total_events": m.total_events,
            "message_count": m.message_count,
            "tool_call_count": m.tool_call_count,
            "tool_result_count": m.tool_result_count,
            "error_count": m.error_count,
            "user_messages": m.user_messages,
            "assistant_messages": m.assistant_messages,
            "duration_seconds": m.duration_seconds,
            "files_touched": m.files_touched,
            "lines_added": m.lines_added,
            "lines_removed": m.lines_removed,
            "model": m.model,
            "provider": m.provider,
            "input_tokens": m.input_tokens,
            "output_tokens": m.output_tokens,
            "estimated_cost": m.estimated_cost,
            "total_latency_ms": m.total_latency_ms,
            "avg_latency_ms": m.avg_latency_ms,
            "p50_latency_ms": m.p50_latency_ms,
            "p95_latency_ms": m.p95_latency_ms,
        })),
    });
    lines.push(serde_json::to_string(&metadata).map_err(|e| e.to_string())?);

    for event in events {
        let obj = serde_json::json!({
            "id": event.id,
            "session_id": event.session_id,
            "kind": event.kind,
            "role": event.role,
            "content": event.content,
            "timestamp": event.timestamp,
            "raw_payload": serde_json::from_str::<serde_json::Value>(&event.raw_payload)
                .unwrap_or(serde_json::Value::Null),
        });
        lines.push(serde_json::to_string(&obj).map_err(|e| e.to_string())?);
    }

    Ok(lines.join("\n"))
}

/// Export search results to Markdown format
pub async fn export_search_to_markdown(query: &str, results: &[agent_v_store::SearchResult]) -> Result<String, String> {
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

/// Export search results to JSON format
pub async fn export_search_to_json(query: &str, results: &[agent_v_store::SearchResult]) -> Result<String, String> {
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

    #[derive(Serialize)]
    struct SearchResultExport {
        query: String,
        results: Vec<SearchEventExport>,
        total: usize,
    }

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

    serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
}

/// Export search results to JSONL format
pub async fn export_search_to_jsonl(_query: &str, results: &[agent_v_store::SearchResult]) -> Result<String, String> {
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
        lines.push(serde_json::to_string(&obj).map_err(|e| e.to_string())?);
    }

    Ok(lines.join("\n"))
}
