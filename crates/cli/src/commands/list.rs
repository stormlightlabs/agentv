use agent_viz_store::Database;
use owo_colors::OwoColorize;

pub async fn sessions(source_filter: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open_default().await?;
    db.migrate().await?;

    let sessions = db.list_sessions(100, 0).await?;

    if sessions.is_empty() {
        println!("{}", "No sessions found.".yellow());
        println!();
        println!("To ingest sessions, run:");
        println!("  {}", "agent-viz ingest --source <SOURCE>".cyan());
        return Ok(());
    }

    println!("{}", "Sessions".bold().underline());
    println!("{}", "-".repeat(80).dimmed());
    println!(
        "{:<36} {:<10} {:<20} {}",
        "ID".dimmed(),
        "Source".dimmed(),
        "Project".dimmed(),
        "Title".dimmed()
    );
    println!("{}", "-".repeat(80).dimmed());

    for session in sessions {
        if let Some(ref filter) = source_filter
            && session.source != *filter
        {
            continue;
        }

        let project = session.project.as_deref().unwrap_or("-");
        let title = session.title.as_deref().unwrap_or("Untitled");

        println!(
            "{:<36} {:<10} {:<20} {}",
            session.id.dimmed(),
            session.source.cyan(),
            &project[..project.len().min(20)],
            title.bold()
        );
    }

    Ok(())
}
