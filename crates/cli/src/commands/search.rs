use agent_v_store::{Database, SearchFacets};
use chrono::{DateTime, Duration, Utc};
use owo_colors::OwoColorize;

/// Run the search command
pub async fn run(
    query: String, source: Option<String>, since: Option<String>, kind: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open_default().await?;
    db.migrate().await?;

    let since_dt = parse_since(&since)?;

    let facets = SearchFacets { source, project: None, kind, since: since_dt };

    println!("{} {}", "Search:".bold().underline(), query.cyan());

    if let Some(ref s) = facets.source {
        println!("  {} {}", "Source:".dimmed(), s.cyan());
    }
    if let Some(ref k) = facets.kind {
        println!("  {} {}", "Kind:".dimmed(), k.cyan());
    }
    if let Some(ref s) = since {
        println!("  {} {}", "Since:".dimmed(), s.cyan());
    }
    println!();

    let results = db.search_events(&query, &facets, 50, 0).await?;

    if results.is_empty() {
        println!("{}", "No results found.".yellow());
        return Ok(());
    }

    println!(
        "{} {}",
        "Results:".bold().underline(),
        format!("({})", results.len()).dimmed()
    );
    println!();

    for result in results {
        let event = &result.event;

        let kind_label = match event.kind.as_str() {
            "message" => "MSG".blue().to_string(),
            "tool_call" => "TOOL".magenta().to_string(),
            "tool_result" => "RES".green().to_string(),
            "error" => "ERR".red().to_string(),
            _ => event.kind.to_uppercase().dimmed().to_string(),
        };

        let role_label = event
            .role
            .as_ref()
            .map(|r| match r.as_str() {
                "user" => "user".blue().to_string(),
                "assistant" => "asst".cyan().to_string(),
                "system" => "sys".dimmed().to_string(),
                _ => r.dimmed().to_string(),
            })
            .unwrap_or_else(|| "-".dimmed().to_string());

        let content_preview = event
            .content
            .as_ref()
            .map(|c| {
                let preview = if c.len() > 80 { &c[..80] } else { c };
                preview.replace('\n', " ")
            })
            .unwrap_or_else(|| "(no content)".dimmed().to_string());

        let timestamp = &event.timestamp[..19.min(event.timestamp.len())];

        println!(
            "  {} {} {} {} {}",
            timestamp.dimmed(),
            kind_label,
            role_label,
            "|".dimmed(),
            content_preview
        );

        println!(
            "     {} {} {}",
            "Session:".dimmed(),
            event.session_id[..8].to_string().cyan(),
            format!("(rank: {:.4})", result.rank).dimmed()
        );
        println!();
    }

    Ok(())
}

fn parse_since(since: &Option<String>) -> Result<Option<DateTime<Utc>>, Box<dyn std::error::Error>> {
    let Some(s) = since else {
        return Ok(None);
    };

    let duration = if s.ends_with('d') {
        let days: i64 = s[..s.len() - 1].parse()?;
        Duration::days(days)
    } else if s.ends_with('h') {
        let hours: i64 = s[..s.len() - 1].parse()?;
        Duration::hours(hours)
    } else if s.ends_with('w') {
        let weeks: i64 = s[..s.len() - 1].parse()?;
        Duration::weeks(weeks)
    } else if s.ends_with('m') && !s.ends_with("min") {
        let months: i64 = s[..s.len() - 1].parse()?;
        Duration::days(months * 30)
    } else {
        return Err(format!("Invalid duration format: {}. Use Nd, Nh, Nw, Nm", s).into());
    };

    Ok(Some(Utc::now() - duration))
}
