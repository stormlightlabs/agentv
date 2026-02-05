use owo_colors::OwoColorize;
use tracing::info;

pub async fn run(source: Option<String>, watch: bool) -> Result<(), Box<dyn std::error::Error>> {
    if watch {
        println!("{}", "Watch mode".bold().underline());
        println!("{}", "Continuously monitoring for new sessions...".dimmed());
        println!("{}", "(Not yet implemented - would run indefinitely)".yellow());
        return Ok(());
    }

    match source {
        Some(src) => {
            info!("Ingesting from source: {}", src);
            println!("{} {}", "Ingesting from:".bold(), src.cyan());
            println!("{}", "(Not yet implemented)".yellow());
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
