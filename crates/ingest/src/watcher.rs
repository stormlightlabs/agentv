use agent_v_adapters::{ClaudeAdapter, CodexAdapter, CrushAdapter, OpenCodeAdapter};
use agent_v_core::Source;
use agent_v_store::Database;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, mpsc};
use tokio::time::interval;
use tracing::{error, info, warn};

/// Watcher configuration
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Polling interval for Crush (SQLite files)
    pub crush_poll_interval: Duration,
    /// Debounce duration for file system events
    pub debounce_duration: Duration,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self { crush_poll_interval: Duration::from_secs(30), debounce_duration: Duration::from_secs(2) }
    }
}

/// Ingestion statistics
#[derive(Debug, Clone)]
pub struct IngestStats {
    pub source: String,
    pub imported: usize,
    pub failed: usize,
    pub timestamp: SystemTime,
}

/// Watcher for file system changes and database updates
pub struct Watcher {
    config: WatcherConfig,
    stats: Arc<Mutex<Vec<IngestStats>>>,
}

impl Watcher {
    /// Create a new watcher with default configuration
    pub fn new() -> Self {
        Self::with_config(WatcherConfig::default())
    }

    /// Create a new watcher with custom configuration
    pub fn with_config(config: WatcherConfig) -> Self {
        Self { config, stats: Arc::new(Mutex::new(Vec::new())) }
    }

    /// Start watching all sources
    pub async fn watch_all(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (tx, mut rx) = mpsc::channel(100);

        let claude_paths = self.get_claude_watch_paths().await;
        let codex_paths = self.get_codex_watch_paths().await;
        let opencode_paths = self.get_opencode_watch_paths().await;

        let _tx_clone = tx.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(_event) = res {
                    let _ = _tx_clone.try_send(("claude", SystemTime::now()));
                }
            },
            Config::default(),
        )?;

        for path in &claude_paths {
            if path.exists() {
                info!("Watching claude path: {:?}", path);
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    warn!("Failed to watch claude path {:?}: {}", path, e);
                }
            }
        }

        for path in &codex_paths {
            if path.exists() {
                info!("Watching codex path: {:?}", path);
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    warn!("Failed to watch codex path {:?}: {}", path, e);
                }
            }
        }

        for path in &opencode_paths {
            if path.exists() {
                info!("Watching opencode path: {:?}", path);
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    warn!("Failed to watch opencode path {:?}: {}", path, e);
                }
            }
        }

        let crush_poll_interval = self.config.crush_poll_interval;
        let crush_handle = tokio::spawn(Self::watch_crush(crush_poll_interval, self.stats.clone()));

        let debounce_duration = self.config.debounce_duration;
        let stats = self.stats.clone();
        let file_watcher = tokio::spawn(async move {
            let mut pending_events: HashMap<String, SystemTime> = HashMap::new();
            let mut debounce_interval = interval(Duration::from_millis(500));

            loop {
                tokio::select! {
                    Some((source, _ts)) = rx.recv() => {
                        pending_events.insert(source.to_string(), SystemTime::now());
                    }
                    _ = debounce_interval.tick() => {
                        let now = SystemTime::now();
                        let ready: Vec<_> = pending_events
                            .iter()
                            .filter(|(_, ts)| now.duration_since(**ts).unwrap_or_default() >= debounce_duration)
                            .map(|(s, _)| s.clone())
                            .collect();

                        for source_str in ready {
                            pending_events.remove(&source_str);
                            info!("Ingesting from {} due to file change", source_str);

                            let source = match source_str.as_str() {
                                "claude" => Some(Source::Claude),
                                "codex" => Some(Source::Codex),
                                "opencode" => Some(Source::OpenCode),
                                "crush" => Some(Source::Crush),
                                _ => None,
                            };

                            if let Some(src) = source {
                                if let Err(e) = Self::ingest_source(src).await {
                                    error!("Failed to ingest from {:?}: {}", src, e);
                                } else {
                                    let mut s = stats.lock().await;
                                    s.push(IngestStats {
                                        source: source_str.clone(),
                                        imported: 1,
                                        failed: 0,
                                        timestamp: SystemTime::now(),
                                    });
                                }
                            }
                        }
                    }
                    else => break,
                }
            }
        });

        tokio::select! {
            _ = file_watcher => {},
            _ = crush_handle => {},
        }

        Ok(())
    }

    /// Watch a specific source
    pub async fn watch_source(self, source: Source) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match source {
            Source::Claude | Source::Codex | Source::OpenCode => {
                let paths = match source {
                    Source::Claude => self.get_claude_watch_paths().await,
                    Source::Codex => self.get_codex_watch_paths().await,
                    Source::OpenCode => self.get_opencode_watch_paths().await,
                    _ => unreachable!(),
                };

                let (tx, mut rx) = mpsc::channel(100);
                let mut watcher = RecommendedWatcher::new(
                    move |res: Result<Event, notify::Error>| {
                        if let Ok(_event) = res {
                            let _ = tx.try_send(SystemTime::now());
                        }
                    },
                    Config::default(),
                )?;

                for path in paths {
                    if path.exists() {
                        watcher.watch(&path, RecursiveMode::Recursive)?;
                    }
                }

                let debounce_duration = self.config.debounce_duration;
                let stats = self.stats.clone();
                let source_str = source.to_string();

                while let Some(_ts) = rx.recv().await {
                    tokio::time::sleep(debounce_duration).await;

                    while rx.try_recv().is_ok() {
                        // TODO: Drain pending events
                    }

                    info!("Ingesting from {:?} due to file change", source);
                    match Self::ingest_source(source).await {
                        Ok(_) => {
                            let mut s = stats.lock().await;
                            s.push(IngestStats {
                                source: source_str.clone(),
                                imported: 1,
                                failed: 0,
                                timestamp: SystemTime::now(),
                            });
                        }
                        Err(e) => {
                            error!("Failed to ingest from {:?}: {}", source, e);
                        }
                    }
                }
            }
            Source::Crush => {
                let _ = Self::watch_crush(self.config.crush_poll_interval, self.stats.clone()).await;
            }
        }

        Ok(())
    }

    /// Watch Crush database for changes
    async fn watch_crush(poll_interval: Duration, stats: Arc<Mutex<Vec<IngestStats>>>) {
        let adapter = CrushAdapter::new();
        let mut last_check = SystemTime::now();
        let mut check_interval = interval(poll_interval);

        loop {
            check_interval.tick().await;

            let sessions = adapter.discover_sessions().await;
            let mut needs_ingest = false;

            for session in &sessions {
                if let Ok(metadata) = tokio::fs::metadata(&session.path).await
                    && let Ok(modified) = metadata.modified()
                    && modified > last_check
                {
                    needs_ingest = true;
                    break;
                }
            }

            if needs_ingest {
                info!("Crush database modified, re-ingesting");
                match Self::ingest_crush().await {
                    Ok(_) => {
                        let mut s = stats.lock().await;
                        s.push(IngestStats {
                            source: "crush".to_string(),
                            imported: 1,
                            failed: 0,
                            timestamp: SystemTime::now(),
                        });
                    }
                    Err(e) => {
                        error!("Failed to ingest Crush: {}", e);
                    }
                }
            }

            last_check = SystemTime::now();
        }
    }

    /// Get paths to watch for Claude
    async fn get_claude_watch_paths(&self) -> Vec<PathBuf> {
        let adapter = ClaudeAdapter::new();
        vec![adapter.projects_dir().clone()]
    }

    /// Get paths to watch for Codex
    async fn get_codex_watch_paths(&self) -> Vec<PathBuf> {
        let adapter = CodexAdapter::new();
        vec![adapter.sessions_dir().clone()]
    }

    /// Get paths to watch for OpenCode
    ///
    /// Limitation: OpenCode does not expose session files directly. Sessions are
    /// accessed via CLI export (`opencode export <session-id>`). File system
    /// watching is not applicable - `--watch` mode cannot detect new OpenCode
    /// sessions automatically. Manual ingestion or periodic polling via CLI
    /// would be required for live updates.
    async fn get_opencode_watch_paths(&self) -> Vec<PathBuf> {
        vec![]
    }

    /// Ingest from a specific source
    async fn ingest_source(source: Source) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match source {
            Source::Claude => Self::ingest_claude().await,
            Source::Codex => Self::ingest_codex().await,
            Source::OpenCode => Self::ingest_opencode().await,
            Source::Crush => Self::ingest_crush().await,
        }
    }

    /// Helper function to open database with proper error conversion
    async fn open_db() -> Result<Database, Box<dyn std::error::Error + Send + Sync>> {
        match Database::open_default().await {
            Ok(db) => Ok(db),
            Err(e) => Err(format!("Database error: {}", e).into()),
        }
    }

    /// Ingest Claude sessions
    async fn ingest_claude() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let db = Self::open_db().await?;
        let adapter = ClaudeAdapter::new();

        for session_file in adapter.discover_sessions().await {
            if let Ok((session, events)) = adapter.parse_session(&session_file).await {
                let _ = db.insert_session_with_events(&session, &events).await;
            }
        }

        Ok(())
    }

    /// Ingest Codex sessions
    async fn ingest_codex() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let db = Self::open_db().await?;
        let adapter = CodexAdapter::new();

        for session_file in adapter.discover_sessions().await {
            if let Ok((session, events)) = adapter.parse_session(&session_file).await {
                let _ = db.insert_session_with_events(&session, &events).await;
            }
        }

        Ok(())
    }

    /// Ingest OpenCode sessions
    async fn ingest_opencode() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let db = Self::open_db().await?;
        let adapter = OpenCodeAdapter::new();

        for session in adapter.discover_sessions().await {
            if let Ok((session_obj, events)) = adapter.parse_session(&session).await {
                let _ = db.insert_session_with_events(&session_obj, &events).await;
            }
        }

        Ok(())
    }

    /// Ingest Crush sessions
    async fn ingest_crush() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let db = Self::open_db().await?;
        let adapter = CrushAdapter::new();

        for session_file in adapter.discover_sessions().await {
            if let Ok((session, events)) = adapter.parse_session(&session_file).await {
                let _ = db.insert_session_with_events(&session, &events).await;
            }
        }

        Ok(())
    }

    /// Get recent ingestion statistics
    pub async fn get_stats(&self) -> Vec<IngestStats> {
        let stats = self.stats.lock().await;
        stats.clone()
    }
}

impl Default for Watcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_watcher_config_default() {
        let config = WatcherConfig::default();
        assert_eq!(config.crush_poll_interval, Duration::from_secs(30));
        assert_eq!(config.debounce_duration, Duration::from_secs(2));
    }

    #[tokio::test]
    async fn test_watcher_new() {
        let watcher = Watcher::new();
        let stats = watcher.get_stats().await;
        assert!(stats.is_empty());
    }
}
