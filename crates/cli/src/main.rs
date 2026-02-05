use clap::{Parser, Subcommand};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

mod commands;

use commands::{doctor, ingest, list, search, show, stats};

#[derive(Parser)]
#[command(name = "agent-viz")]
#[command(about = "Agent session visualization and analysis tool")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check system health and configuration
    Doctor,
    /// Ingest sessions from various sources
    Ingest {
        /// Source to ingest from (claude, codex, opencode, crush)
        #[arg(short, long)]
        source: Option<String>,
        /// Watch for new sessions continuously
        #[arg(short, long)]
        watch: bool,
    },
    /// List sessions
    List {
        /// What to list
        #[command(subcommand)]
        what: ListWhat,
    },
    /// Show session details
    Show {
        /// Session ID to show
        session_id: String,
    },
    /// Search across sessions
    Search {
        /// Search query
        query: String,
        /// Filter by source
        #[arg(short = 'S', long)]
        source: Option<String>,
        /// Filter by date range (e.g., "7d", "30d")
        #[arg(short = 's', long)]
        since: Option<String>,
        /// Filter by event kind (message, tool_call, tool_result, error)
        #[arg(short = 'k', long)]
        kind: Option<String>,
    },
    /// Show statistics and analytics
    Stats {
        /// Group by dimension (day, source, project, tool, error)
        #[arg(short, long)]
        by: Option<String>,
        /// Filter by date range (e.g., "7d", "30d")
        #[arg(short, long)]
        since: Option<String>,
    },
}

#[derive(Subcommand)]
enum ListWhat {
    /// List all sessions
    Sessions {
        /// Filter by source
        #[arg(short, long)]
        source: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor => {
            info!("Running doctor command");
            doctor::run().await?;
        }
        Commands::Ingest { source, watch } => {
            info!("Running ingest command");
            ingest::run(source, watch).await?;
        }
        Commands::List { what } => match what {
            ListWhat::Sessions { source } => {
                info!("Running list sessions command");
                list::sessions(source).await?;
            }
        },
        Commands::Show { session_id } => {
            info!("Showing session: {}", session_id);
            show::session(session_id).await?;
        }
        Commands::Search { query, source, since, kind } => {
            info!("Searching for: {}", query);
            search::run(query, source, since, kind).await?;
        }
        Commands::Stats { by, since } => {
            info!("Running stats command");
            stats::run(by, since).await?;
        }
    }

    Ok(())
}
