use agent_viz_adapters::{claude::ClaudeAdapter, codex::CodexAdapter, opencode::OpenCodeAdapter};
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
                Source::Codex => ingest_codex(&db).await?,
                Source::OpenCode => ingest_opencode(&db).await?,
                Source::Crush => {
                    println!("{}", format!("Source '{}' not yet implemented", src).yellow())
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
        print!("  {} {} ... ", "→".dimmed(), session_file.session_id.cyan());

        match adapter.parse_session(&session_file).await {
            Ok((session, events)) => match db.insert_session_with_events(&session, &events).await {
                Ok(_) => {
                    println!("{} ({} events)", "✓".green(), events.len().to_string().dimmed());
                    imported += 1;
                }
                Err(e) => {
                    println!("{} {}", "✗".red(), e.to_string().dimmed());
                    error!("Failed to insert session {}: {}", session.external_id, e);
                    failed += 1;
                }
            },
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

async fn ingest_codex(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = CodexAdapter::new();

    println!("  {} Discovering sessions...", "→".dimmed());
    let sessions = adapter.discover_sessions().await;

    if sessions.is_empty() {
        println!("  {} No Codex rollouts found", "✗".red());
        println!();
        println!("{}", "Make sure Codex is installed and has sessions.".dimmed());
        println!(
            "{}",
            "Sessions should be in $CODEX_HOME/sessions/ or ~/.codex/sessions/".dimmed()
        );
        return Ok(());
    }

    println!(
        "  {} Found {} rollout files",
        "✓".green(),
        sessions.len().to_string().bold()
    );
    println!();

    let mut imported = 0;
    let mut failed = 0;

    for session_file in sessions {
        print!("  {} {} ... ", "→".dimmed(), session_file.session_id.cyan());

        match adapter.parse_session(&session_file).await {
            Ok((session, events)) => match db.insert_session_with_events(&session, &events).await {
                Ok(_) => {
                    println!("{} ({} events)", "✓".green(), events.len().to_string().dimmed());
                    imported += 1;
                }
                Err(e) => {
                    println!("{} {}", "✗".red(), e.to_string().dimmed());
                    error!("Failed to insert session {}: {}", session.external_id, e);
                    failed += 1;
                }
            },
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

async fn ingest_opencode(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = OpenCodeAdapter::new();

    if !adapter.is_available() {
        println!("  {} OpenCode CLI not found", "✗".red());
        println!();
        println!("{}", "Make sure OpenCode is installed and in PATH.".dimmed());
        return Ok(());
    }

    println!("  {} Discovering sessions...", "→".dimmed());
    let sessions = adapter.discover_sessions().await;

    if sessions.is_empty() {
        println!("  {} No OpenCode sessions found", "✗".red());
        println!();
        println!("{}", "Make sure OpenCode has sessions.".dimmed());
        return Ok(());
    }

    println!("  {} Found {} sessions", "✓".green(), sessions.len().to_string().bold());
    println!();

    let providers = adapter.get_providers();
    if !providers.is_empty() {
        println!(
            "  {} Configured providers: {}",
            "→".dimmed(),
            providers.join(", ").dimmed()
        );
        println!();
    }

    let mut imported = 0;
    let mut failed = 0;

    for session in sessions {
        print!(
            "  {} {} ... ",
            "→".dimmed(),
            session.title.chars().take(50).collect::<String>().cyan()
        );

        match adapter.parse_session(&session).await {
            Ok((session_obj, events)) => match db.insert_session_with_events(&session_obj, &events).await {
                Ok(_) => {
                    println!("{} ({} events)", "✓".green(), events.len().to_string().dimmed());
                    imported += 1;
                }
                Err(e) => {
                    println!("{} {}", "✗".red(), e.to_string().dimmed());
                    error!("Failed to insert session {}: {}", session_obj.external_id, e);
                    failed += 1;
                }
            },
            Err(e) => {
                println!("{} {}", "✗".red(), e.to_string().dimmed());
                error!("Failed to parse session {}: {}", session.id, e);
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
