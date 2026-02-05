use agent_viz_store::Database;
use owo_colors::OwoColorize;

pub async fn session(session_id: String) -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open_default().await?;
    db.migrate().await?;

    let sessions = db.list_sessions(1000, 0).await?;

    let session = sessions
        .iter()
        .find(|s| s.id == session_id || s.external_id == session_id)
        .cloned();

    let session = match session {
        Some(s) => s,
        None => {
            println!("{} Session not found: {}", "✗".red(), session_id.cyan());
            println!();
            println!(
                "{}",
                "Run 'agent-viz list sessions' to see available sessions.".dimmed()
            );
            return Ok(());
        }
    };

    println!("{}", "Session Details".bold().underline());
    println!();
    println!("{} {}", "ID:".dimmed(), session.id);
    println!("{} {}", "External ID:".dimmed(), session.external_id.cyan());
    println!("{} {}", "Source:".dimmed(), session.source.cyan());
    if let Some(ref project) = session.project {
        println!("{} {}", "Project:".dimmed(), project.cyan());
    }
    if let Some(ref title) = session.title {
        println!("{} {}", "Title:".dimmed(), title.bold());
    }
    println!("{} {}", "Created:".dimmed(), session.created_at);
    println!("{} {}", "Updated:".dimmed(), session.updated_at);
    println!();

    let events = db.get_session_events(session.id.clone()).await?;

    if events.is_empty() {
        println!("{}", "No events found for this session.".yellow());
        return Ok(());
    }

    println!(
        "{} {} {}",
        "Timeline".bold().underline(),
        "(".dimmed(),
        format!("{} events", events.len()).dimmed()
    );
    println!();

    for (idx, event) in events.iter().enumerate() {
        let role_label = event.role.as_deref().unwrap_or("-");
        let role_colored = match role_label {
            "user" => "USER".green().to_string(),
            "assistant" => "ASSISTANT".blue().to_string(),
            "system" => "SYSTEM".dimmed().to_string(),
            _ => role_label.dimmed().to_string(),
        };

        let kind_label = match event.kind.as_str() {
            "message" => "MSG",
            "tool_call" => "TOOL",
            "tool_result" => "RESULT",
            "error" => "ERR",
            "system" => "SYS",
            _ => &event.kind.to_uppercase(),
        };
        let kind_colored = match event.kind.as_str() {
            "message" => kind_label.to_string(),
            "tool_call" => kind_label.yellow().to_string(),
            "tool_result" => kind_label.yellow().to_string(),
            "error" => kind_label.red().to_string(),
            "system" => kind_label.dimmed().to_string(),
            _ => kind_label.dimmed().to_string(),
        };

        print!(
            "{:>3} {} {} {} ",
            (idx + 1).to_string().dimmed(),
            event.timestamp.split('T').next().unwrap_or("").dimmed(),
            kind_colored,
            role_colored
        );

        if let Some(ref content) = event.content {
            let preview: String = content.lines().next().unwrap_or("").chars().take(60).collect();

            if preview.len() >= 60 {
                print!("{}...", preview);
            } else {
                print!("{}", preview);
            }
        }

        println!();
    }

    println!();
    println!(
        "{}",
        "Use 'agent-viz search \"<query>\"' to search across all sessions.".dimmed()
    );

    Ok(())
}
