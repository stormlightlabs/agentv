use agent_viz_core::Session;
use agent_viz_store::Database;

pub async fn get_sessions(db: &Database) -> Result<Vec<Session>, Box<dyn std::error::Error>> {
    let _ = db;
    Ok(vec![])
}
