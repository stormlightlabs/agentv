use agent_v_store::Database;
use chrono::{DateTime, Duration, Utc};
use owo_colors::OwoColorize;

/// Run the stats command
pub async fn run(by: Option<String>, since: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open_default().await?;
    db.migrate().await?;

    let since_dt = parse_since(&since)?;
    let until_dt = Some(Utc::now());

    match by.as_deref() {
        Some("day") | Some("daily") => show_activity_by_day(&db, since_dt, until_dt).await?,
        Some("source") => show_stats_by_source(&db).await?,
        Some("project") => show_stats_by_project(&db, None).await?,
        Some("tool") => show_stats_by_tool(&db, since_dt, until_dt).await?,
        Some("error") | Some("errors") => show_error_stats(&db, since_dt, until_dt).await?,
        Some("tools") | Some("tool-calls") => show_tool_call_frequency(&db, since_dt, until_dt).await?,
        Some("files") => show_files_leaderboard(&db, since_dt, until_dt).await?,
        Some("churn") => show_patch_churn(&db, since_dt, until_dt).await?,
        Some("latency") | Some("slow") => show_long_running_tools(&db, since_dt, until_dt).await?,
        _ => show_summary(&db).await?,
    }

    Ok(())
}

async fn show_summary(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Statistics Summary".bold().underline());
    println!();

    let sources = db.get_stats_by_source().await?;
    println!("{}", "By Source:".bold());
    for stat in sources {
        println!(
            "  {:12} {:4} sessions  ({} - {})",
            stat.dimension.cyan(),
            stat.count,
            stat.earliest.as_deref().unwrap_or("?").dimmed(),
            stat.latest.as_deref().unwrap_or("?").dimmed()
        );
    }
    println!();

    let projects = db.get_stats_by_project(None).await?;
    println!("{}", "By Project:".bold());
    for stat in projects.iter().take(10) {
        println!(
            "  {:20} {:4} sessions  ({} - {})",
            stat.dimension.cyan(),
            stat.count,
            stat.earliest.as_deref().unwrap_or("?").dimmed(),
            stat.latest.as_deref().unwrap_or("?").dimmed()
        );
    }
    if projects.len() > 10 {
        println!("  ... and {} more", projects.len() - 10);
    }
    println!();

    let tools = db.get_stats_by_tool(None, None).await?;
    println!("{}", "By Event Kind:".bold());
    for stat in tools {
        println!(
            "  {:15} {:6} events  ({} sessions)",
            stat.dimension.cyan(),
            stat.count,
            stat.sessions.unwrap_or(0)
        );
    }

    Ok(())
}

async fn show_activity_by_day(
    db: &Database, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Activity by Day".bold().underline());
    println!();

    let stats = db.get_activity_by_day(since, until, None).await?;

    if stats.is_empty() {
        println!("{}", "No activity found.".yellow());
        return Ok(());
    }

    let max_events: i64 = stats.iter().map(|s| s.event_count).max().unwrap_or(1);
    let bar_width = 40u64;

    for stat in stats.iter().take(30) {
        let bar_len = ((stat.event_count as f64 / max_events as f64) * bar_width as f64) as usize;
        let bar = "█".repeat(bar_len);
        let padding = " ".repeat(bar_width as usize - bar_len);

        println!(
            "  {}  {:4} events  {}{}  ({} sessions)",
            stat.day.to_string().dimmed(),
            stat.event_count,
            bar.green(),
            padding,
            stat.session_count
        );
    }

    if stats.len() > 30 {
        println!("  ... and {} more days", stats.len() - 30);
    }

    let total_events: i64 = stats.iter().map(|s| s.event_count).sum();
    let total_sessions: i64 = stats.iter().map(|s| s.session_count).sum();

    println!();
    println!(
        "  {}: {} events, {} sessions",
        "Total".bold(),
        total_events,
        total_sessions
    );

    Ok(())
}

async fn show_stats_by_source(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Statistics by Source".bold().underline());
    println!();

    let stats = db.get_stats_by_source().await?;

    for stat in stats {
        println!(
            "  {:12} {:4} sessions  ({} - {})",
            stat.dimension.cyan(),
            stat.count,
            stat.earliest.as_deref().unwrap_or("?").dimmed(),
            stat.latest.as_deref().unwrap_or("?").dimmed()
        );
    }

    Ok(())
}

async fn show_stats_by_project(db: &Database, source: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Statistics by Project".bold().underline());
    println!();

    let stats = db.get_stats_by_project(source).await?;

    for stat in stats {
        println!(
            "  {:20} {:4} sessions  ({} - {})",
            stat.dimension.cyan(),
            stat.count,
            stat.earliest.as_deref().unwrap_or("?").dimmed(),
            stat.latest.as_deref().unwrap_or("?").dimmed()
        );
    }

    Ok(())
}

async fn show_stats_by_tool(
    db: &Database, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Statistics by Tool/Event Kind".bold().underline());
    println!();

    let stats = db.get_stats_by_tool(since, until).await?;

    for stat in stats {
        println!(
            "  {:15} {:6} events  ({} sessions)",
            stat.dimension.cyan(),
            stat.count,
            stat.sessions.unwrap_or(0)
        );
    }

    Ok(())
}

async fn show_error_stats(
    db: &Database, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Error Statistics".bold().underline());
    println!();

    let errors_by_day = db.get_errors_by_day(since, until).await?;

    if errors_by_day.is_empty() {
        println!("{}", "No errors found.".green());
        return Ok(());
    }

    println!("{}", "Errors by Day:".bold());

    let mut current_day = None;
    let mut day_count = 0;

    for stat in errors_by_day.iter().take(50) {
        if current_day != Some(stat.day) {
            current_day = Some(stat.day);
            day_count = 0;
            println!();
            println!("  {}", stat.day.to_string().dimmed());
        }

        let signature = stat
            .signature
            .as_deref()
            .unwrap_or("Unknown")
            .lines()
            .next()
            .unwrap_or("Unknown");
        let preview = if signature.len() > 60 { format!("{}...", &signature[..60]) } else { signature.to_string() };

        println!("    {:3} × {}", stat.error_count, preview.red());
        day_count += 1;

        if day_count >= 5 {
            println!("    ...");
            break;
        }
    }

    println!();
    println!("{}", "Top Error Signatures:".bold());

    let top_errors = db.get_top_errors(since, until, 10).await?;

    for (idx, (signature, count)) in top_errors.iter().enumerate() {
        let preview = if signature.len() > 60 { format!("{}...", &signature[..60]) } else { signature.clone() };
        println!("  {}. {:4} × {}", idx + 1, count, preview.red());
    }

    let total_errors: i64 = errors_by_day.iter().map(|e| e.error_count).sum();
    println!();
    println!("  {}: {} errors", "Total".bold(), total_errors);

    Ok(())
}

async fn show_tool_call_frequency(
    db: &Database, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Tool Call Frequency".bold().underline());
    println!();

    let stats = db.get_tool_call_frequency(since, until).await?;

    if stats.is_empty() {
        println!("{}", "No tool calls found.".yellow());
        return Ok(());
    }

    let max_calls: i64 = stats.iter().map(|s| s.call_count).max().unwrap_or(1);
    let bar_width = 30u64;

    for stat in &stats {
        let bar_len = ((stat.call_count as f64 / max_calls as f64) * bar_width as f64) as usize;
        let bar = "█".repeat(bar_len);
        let padding = " ".repeat(bar_width as usize - bar_len);

        let duration_str = match stat.avg_duration_ms {
            Some(avg) => format!("  avg {:.0}ms  max {}ms", avg, stat.max_duration_ms.unwrap_or(0)),
            None => String::new(),
        };

        println!(
            "  {:25} {:5} calls {}{}  ({} sessions){}",
            stat.tool_name.cyan(),
            stat.call_count,
            bar.green(),
            padding,
            stat.sessions,
            duration_str.dimmed()
        );
    }

    let total_calls: i64 = stats.iter().map(|s| s.call_count).sum();
    let total_tools = stats.len();

    println!();
    println!(
        "  {}: {} calls across {} tools",
        "Total".bold(),
        total_calls,
        total_tools
    );

    Ok(())
}

async fn show_files_leaderboard(
    db: &Database, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Files Touched Leaderboard".bold().underline());
    println!();

    let stats = db.get_files_leaderboard(since, until, 20).await?;

    if stats.is_empty() {
        println!("{}", "No files touched found.".yellow());
        return Ok(());
    }

    let max_touches: i64 = stats.iter().map(|s| s.touch_count).max().unwrap_or(1);
    let bar_width = 20u64;

    for stat in stats {
        let bar_len = ((stat.touch_count as f64 / max_touches as f64) * bar_width as f64) as usize;
        let bar = "█".repeat(bar_len);
        let padding = " ".repeat(bar_width as usize - bar_len);

        let changes = format!("+{} -{}", stat.total_lines_added, stat.total_lines_removed);

        println!(
            "  {:45} {:4} {}{}  {}  ({} sessions)",
            stat.file_path.cyan(),
            stat.touch_count,
            bar.green(),
            padding,
            changes.yellow(),
            stat.sessions
        );
    }

    Ok(())
}

async fn show_patch_churn(
    db: &Database, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Patch Churn by Day".bold().underline());
    println!();

    let stats = db.get_patch_churn_by_day(since, until).await?;

    if stats.is_empty() {
        println!("{}", "No patch churn data found.".yellow());
        return Ok(());
    }

    let max_net: i64 = stats
        .iter()
        .map(|s| (s.lines_added + s.lines_removed).max(1))
        .max()
        .unwrap_or(1);
    let bar_width = 25u64;

    for stat in stats.iter().take(30) {
        let net = stat.lines_added + stat.lines_removed;
        let bar_len = ((net as f64 / max_net as f64) * bar_width as f64) as usize;
        let bar = "█".repeat(bar_len);
        let padding = " ".repeat(bar_width as usize - bar_len);

        println!(
            "  {}  {:5} files  +{:4} -{:4} {}{}",
            stat.day.to_string().dimmed(),
            stat.files_changed,
            stat.lines_added.to_string().green(),
            stat.lines_removed.to_string().red(),
            bar.blue(),
            padding
        );
    }

    let total_added: i64 = stats.iter().map(|s| s.lines_added).sum();
    let total_removed: i64 = stats.iter().map(|s| s.lines_removed).sum();

    println!();
    println!(
        "  {}: +{} -{} (net +{})",
        "Total".bold(),
        total_added,
        total_removed,
        total_added - total_removed
    );

    Ok(())
}

async fn show_long_running_tools(
    db: &Database, since: Option<DateTime<Utc>>, until: Option<DateTime<Utc>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Long-Running Tool Calls".bold().underline());
    println!();

    let min_duration_ms = 5000;
    let stats = db
        .get_long_running_tool_calls(since, until, min_duration_ms, 20)
        .await?;

    if stats.is_empty() {
        println!("{}", "No long-running tool calls found.".green());
        return Ok(());
    }

    for stat in stats {
        let duration_secs = stat.duration_ms as f64 / 1000.0;
        let error_indicator = match stat.error_message {
            Some(_) => " [ERROR]".red().to_string(),
            None => String::new(),
        };

        println!(
            "  {:20} {:6.1}s  {:15}  {}{}",
            stat.tool_name.cyan(),
            duration_secs,
            stat.session_external_id.dimmed(),
            stat.project.as_deref().unwrap_or("no project"),
            error_indicator
        );
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
