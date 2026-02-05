use agent_viz_core::HealthStatus;
use agent_viz_store::{Database, check_sources_health};
use owo_colors::OwoColorize;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Agent Viz Doctor".bold().underline());
    println!();

    println!("{}", "Checking database...".dimmed());
    match check_database().await {
        Ok(status) => {
            println!("  Database: {}", status);
        }
        Err(e) => {
            tracing::error!("Database check failed: {}", e);
            println!("  Database: {} - {}", "FAILED".red().bold(), e);
        }
    }
    println!();

    println!("{}", "Checking data sources...".dimmed());
    let health_results = check_sources_health().await;
    for health in health_results {
        print_source_health(&health);
    }
    println!();

    println!("{}", "Doctor check complete.".green().bold());
    Ok(())
}

async fn check_database() -> Result<String, Box<dyn std::error::Error>> {
    let db = Database::open_default().await?;

    db.migrate().await?;

    let health = db.health_check().await;

    let path = db.path().display().to_string();

    let status_str = match health {
        HealthStatus::Healthy => "healthy".green().bold().to_string(),
        HealthStatus::Degraded => "degraded".yellow().bold().to_string(),
        HealthStatus::Unhealthy => "unhealthy".red().bold().to_string(),
        HealthStatus::Unknown => "unknown".dimmed().to_string(),
    };

    Ok(format!("{} ({})", status_str, path.dimmed()))
}

type IconStyler = Box<dyn Fn(&str) -> String>;

fn print_source_health(health: &agent_viz_core::SourceHealth) {
    let (icon, icon_style): (&str, IconStyler) = match health.status {
        HealthStatus::Healthy => ("✓", Box::new(|s: &str| s.green().bold().to_string())),
        HealthStatus::Degraded => ("~", Box::new(|s: &str| s.yellow().bold().to_string())),
        HealthStatus::Unhealthy => ("✗", Box::new(|s: &str| s.red().bold().to_string())),
        HealthStatus::Unknown => ("?", Box::new(|s: &str| s.dimmed().to_string())),
    };

    let status_str: String = match health.status {
        HealthStatus::Healthy => "healthy".green().bold().to_string(),
        HealthStatus::Degraded => "degraded".yellow().bold().to_string(),
        HealthStatus::Unhealthy => "unhealthy".red().bold().to_string(),
        HealthStatus::Unknown => "unknown".dimmed().to_string(),
    };

    println!(
        "  [{}] {}: {}",
        icon_style(icon),
        health.source.to_string().cyan(),
        status_str
    );

    if let Some(path) = &health.path {
        println!("      Path: {}", path.dimmed());
    }

    if let Some(msg) = &health.message {
        println!("      {}", msg.italic());
    }
}
