use agent_v_store::Database;
use owo_colors::OwoColorize;
use tracing::info;

/// Recompute metrics for all sessions
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open_default().await?;
    db.migrate().await?;

    info!("Recomputing session metrics...");
    println!("{}", "Recomputing Session Metrics".bold().underline());
    println!();

    let mut total_sessions = 0;
    let mut offset = 0;
    let batch_size = 100;

    loop {
        let sessions = db.list_sessions(batch_size, offset).await?;
        if sessions.is_empty() {
            break;
        }
        total_sessions += sessions.len();
        offset += batch_size;
    }

    if total_sessions == 0 {
        println!("  No sessions found");
        return Ok(());
    }

    let milestone = (total_sessions as f64 / 10.0).ceil() as usize;
    let mut next_milestone = milestone;
    let mut processed = 0;
    offset = 0;

    loop {
        let sessions = db.list_sessions(batch_size, offset).await?;
        if sessions.is_empty() {
            break;
        }

        for session in &sessions {
            match db.compute_session_metrics(&session.id).await {
                Ok(_) => {
                    processed += 1;
                    if processed >= next_milestone {
                        let percent = (processed as f64 / total_sessions as f64 * 100.0) as usize;
                        print!(
                            "\r  Processed: {}% ({}/{} sessions)",
                            percent.to_string().cyan(),
                            processed.to_string().cyan(),
                            total_sessions
                        );
                        next_milestone += milestone;
                    }
                }
                Err(e) => {
                    eprintln!("\n  {} Error processing {}: {}", "✗".red(), session.external_id, e);
                }
            }
        }

        offset += batch_size;
    }

    print!("\r  Processed: 100% ({}/{} sessions)", processed, total_sessions);
    println!();
    println!("  {} Recomputed metrics for {} sessions", "✓".green(), processed);

    Ok(())
}
