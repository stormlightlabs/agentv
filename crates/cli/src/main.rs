use clap::{Parser, Subcommand};
use tracing_subscriber::FmtSubscriber;

mod commands;

use commands::{doctor, export, ingest, list, search, show, stats, support};

#[derive(Parser)]
#[command(name = "agent-viz")]
#[command(about = "Agent session visualization and analysis tool")]
#[command(version = env!("CARGO_PKG_VERSION"))]
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
    /// Export sessions or search results
    Export {
        /// Export a specific session by ID
        #[arg(long, group = "export_target")]
        session: Option<String>,
        /// Export search results
        #[arg(long, group = "export_target")]
        search: Option<String>,
        /// Output format (md, json, jsonl)
        #[arg(short, long, default_value = "md")]
        format: String,
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
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
    /// Show support information and funding links
    Support,
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
        .with_max_level(tracing::Level::INFO)
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
            tracing::info!("Running doctor command");
            doctor::run().await?;
        }
        Commands::Ingest { source, watch } => {
            tracing::info!("Running ingest command");
            ingest::run(source, watch).await?;
        }
        Commands::List { what } => match what {
            ListWhat::Sessions { source } => {
                tracing::info!("Running list sessions command");
                list::sessions(source).await?;
            }
        },
        Commands::Show { session_id } => {
            tracing::info!("Showing session: {}", session_id);
            show::session(session_id).await?;
        }
        Commands::Search { query, source, since, kind } => {
            tracing::info!("Searching for: {}", query);
            search::run(query, source, since, kind).await?;
        }
        Commands::Stats { by, since } => {
            tracing::info!("Running stats command");
            stats::run(by, since).await?;
        }
        Commands::Export { session, search, format, output, source, since, kind } => {
            tracing::info!("Running export command");
            let export_format = export::ExportFormat::from_str(&format)?;
            if let Some(session_id) = session {
                export::export_session(session_id, export_format, output).await?;
            } else if let Some(query) = search {
                export::export_search(query, source, since, kind, export_format, output).await?;
            }
        }
        Commands::Support => {
            tracing::info!("Running support command");
            support::run().await?;
        }
    }

    Ok(())
}
