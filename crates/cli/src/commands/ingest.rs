use agent_viz_adapters::claude::ClaudeAdapter;
use agent_viz_core::Source;
use agent_viz_store::Database;
use owo_colors::OwoColorize;
use std::str::FromStr;
use tracing::{error, info};

pub async fn run(source: Option<String>, watch: bool) -> Result<(), Box<dyn std::error::Error>> {
    if watch {
        println!("{}", "Watch mode".bold().underline());
        println!("{}", "Continuously monitoring for new sessions...".dimmed());
        println!("{}", "(Not yet implemented - would run indefinitely)".yellow());
        return Ok(());
    }

    let db = Database::open_default().await?;
    db.migrate().await?;

    match source {
        Some(src) => {
            let source = Source::from_str(&src)?;
            info!("Ingesting from source: {}", source);
            println!("{} {}", "Ingesting from:".bold(), src.cyan());

            match source {
                Source::Claude => ingest_claude(&db).await?,
                _ => {
                    println!("{}", format!("Source '{}' not yet implemented", src).yellow());
                }
            }
        }
        None => {
            println!("{}", "Ingest Sessions".bold().underline());
            println!();
            println!("Usage: {}", "agent-viz ingest --source <SOURCE>".cyan());
            println!();
            println!("{}", "Available sources:".bold());
            println!("  {}    - Claude Code sessions", "claude".green());
            println!("  {}     - Codex CLI rollouts", "codex".green());
            println!("  {}  - OpenCode logs", "opencode".green());
            println!("  {}     - Crush database", "crush".green());
            println!();
            println!("{}", "Options:".bold());
            println!("  {}   Continuously watch for new sessions", "--watch".cyan());
        }
    }

    Ok(())
}

async fn ingest_claude(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = ClaudeAdapter::new();

    println!("  {} Discovering sessions...", "→".dimmed());
    let sessions = adapter.discover_sessions().await;

    if sessions.is_empty() {
        println!("  {} No Claude Code sessions found", "✗".red());
        println!();
        println!("{}", "Make sure Claude Code is installed and has sessions.".dimmed());
        return Ok(());
    }

    println!(
        "  {} Found {} session files",
        "✓".green(),
        sessions.len().to_string().bold()
    );
    println!();

    let mut imported = 0;
    let mut failed = 0;

    for session_file in sessions {
        print!(
            "  {} {} ... ",
            "→".dimmed(),
            session_file.session_id.cyan()
        );

        match adapter.parse_session(&session_file).await {
            Ok((session, events)) => {
                match db.insert_session_with_events(&session, &events).await {
                    Ok(_) => {
                        println!(
                            "{} ({} events)",
                            "✓".green(),
                            events.len().to_string().dimmed()
                        );
                        imported += 1;
                    }
                    Err(e) => {
                        println!("{} {}", "✗".red(), e.to_string().dimmed());
                        error!("Failed to insert session {}: {}", session.external_id, e);
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                println!("{} {}", "✗".red(), e.to_string().dimmed());
                error!("Failed to parse session {:?}: {}", session_file.path, e);
                failed += 1;
            }
        }
    }

    println!();
    println!("{}", "Ingest complete".bold().underline());
    println!("  {} Imported: {}", "✓".green(), imported.to_string().bold());
    if failed > 0 {
        println!("  {} Failed: {}", "✗".red(), failed.to_string().bold());
    }

    Ok(())
}
