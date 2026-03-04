use agent_v_adapters::{ClaudeAdapter, CodexAdapter, CrushAdapter, OpenCodeAdapter};
use agent_v_core::Source;
use agent_v_store::Database;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, mpsc};
use tokio::time::interval;

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

/// A streaming event pushed to the callback when new data is detected
#[derive(Debug, Clone, Serialize)]
pub struct StreamingEvent {
    pub session_external_id: String,
    pub source: String,
    pub project: Option<String>,
    pub new_events: Vec<agent_v_core::Event>,
    pub is_new_session: bool,
}

/// Callback type invoked when new events are detected
pub type EventCallback = Arc<dyn Fn(StreamingEvent) + Send + Sync>;

/// Cursor for tracking incremental parse position per session
#[derive(Debug, Clone)]
enum SessionCursor {
    /// Byte offset for JSONL files (Claude, Codex)
    ByteOffset(u64),
    /// Known message file names (OpenCode)
    KnownFiles(HashSet<String>),
    /// Last message created_at timestamp (Crush)
    LastCreatedAt(i64),
}

/// Watcher for file system changes and database updates
pub struct Watcher {
    config: WatcherConfig,
    stats: Arc<Mutex<Vec<IngestStats>>>,
    cursors: Arc<Mutex<HashMap<String, SessionCursor>>>,
    event_callback: Option<EventCallback>,
}

impl Watcher {
    /// Create a new watcher with default configuration
    pub fn new() -> Self {
        Self::with_config(WatcherConfig::default())
    }

    /// Create a new watcher with custom configuration
    pub fn with_config(config: WatcherConfig) -> Self {
        Self {
            config,
            stats: Arc::new(Mutex::new(Vec::new())),
            cursors: Arc::new(Mutex::new(HashMap::new())),
            event_callback: None,
        }
    }

    /// Create a new watcher with a callback for streaming events
    pub fn with_callback(config: WatcherConfig, callback: EventCallback) -> Self {
        Self {
            config,
            stats: Arc::new(Mutex::new(Vec::new())),
            cursors: Arc::new(Mutex::new(HashMap::new())),
            event_callback: Some(callback),
        }
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
                log::info!("Watching claude path: {:?}", path);
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    log::warn!("Failed to watch claude path {:?}: {}", path, e);
                }
            }
        }

        for path in &codex_paths {
            if path.exists() {
                log::info!("Watching codex path: {:?}", path);
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    log::warn!("Failed to watch codex path {:?}: {}", path, e);
                }
            }
        }

        for path in &opencode_paths {
            if path.exists() {
                log::info!("Watching opencode path: {:?}", path);
                if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                    log::warn!("Failed to watch opencode path {:?}: {}", path, e);
                }
            }
        }

        let crush_poll_interval = self.config.crush_poll_interval;
        let crush_stats = self.stats.clone();
        let crush_cursors = self.cursors.clone();
        let crush_callback = self.event_callback.clone();
        let crush_handle = tokio::spawn(Self::watch_crush_streaming(
            crush_poll_interval,
            crush_stats,
            crush_cursors,
            crush_callback,
        ));

        let debounce_duration = self.config.debounce_duration;
        let stats = self.stats.clone();
        let cursors = self.cursors.clone();
        let callback = self.event_callback.clone();
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
                            log::info!("Ingesting from {} due to file change", source_str);

                            let source = match source_str.as_str() {
                                "claude" => Some(Source::Claude),
                                "codex" => Some(Source::Codex),
                                "opencode" => Some(Source::OpenCode),
                                "crush" => Some(Source::Crush),
                                _ => None,
                            };

                            if let Some(src) = source {
                                let result = Self::ingest_source_streaming(
                                    src,
                                    cursors.clone(),
                                    callback.clone(),
                                ).await;

                                match result {
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
                                        log::error!("Failed to ingest from {:?}: {}", src, e);
                                    }
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
                let cursors = self.cursors.clone();
                let callback = self.event_callback.clone();
                let source_str = source.to_string();

                while let Some(_ts) = rx.recv().await {
                    tokio::time::sleep(debounce_duration).await;

                    while rx.try_recv().is_ok() {
                        log::debug!("Draining pending events");
                    }

                    log::info!("Ingesting from {:?} due to file change", source);
                    match Self::ingest_source_streaming(source, cursors.clone(), callback.clone()).await {
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
                            log::error!("Failed to ingest from {:?}: {}", source, e);
                        }
                    }
                }
            }
            Source::Crush => {
                Self::watch_crush_streaming(
                    self.config.crush_poll_interval,
                    self.stats.clone(),
                    self.cursors.clone(),
                    self.event_callback.clone(),
                )
                .await;
            }
        }

        Ok(())
    }

    /// Watch Crush database for changes with streaming support
    async fn watch_crush_streaming(
        poll_interval: Duration, stats: Arc<Mutex<Vec<IngestStats>>>,
        cursors: Arc<Mutex<HashMap<String, SessionCursor>>>, callback: Option<EventCallback>,
    ) {
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
                log::info!("Crush database modified, re-ingesting");
                match Self::ingest_crush_streaming(cursors.clone(), callback.clone()).await {
                    Ok(_) => {
                        let mut s = stats.lock().await;
                        s.push(IngestStats {
                            source: "crush".to_string(),
                            imported: 1,
                            failed: 0,
                            timestamp: SystemTime::now(),
                        });
                    }
                    Err(e) => log::error!("Failed to ingest Crush: {}", e),
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
    async fn get_opencode_watch_paths(&self) -> Vec<PathBuf> {
        let adapter = OpenCodeAdapter::new();
        vec![adapter.storage_path().clone()]
    }

    /// Ingest from a specific source with streaming callback support
    async fn ingest_source_streaming(
        source: Source, cursors: Arc<Mutex<HashMap<String, SessionCursor>>>, callback: Option<EventCallback>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match source {
            Source::Claude => Self::ingest_claude_streaming(cursors, callback).await,
            Source::Codex => Self::ingest_codex_streaming(cursors, callback).await,
            Source::OpenCode => Self::ingest_opencode_streaming(cursors, callback).await,
            Source::Crush => Self::ingest_crush_streaming(cursors, callback).await,
        }
    }

    /// Helper function to open database with proper error conversion
    async fn open_db() -> Result<Database, Box<dyn std::error::Error + Send + Sync>> {
        match Database::open_default().await {
            Ok(db) => Ok(db),
            Err(e) => Err(format!("Database error: {}", e).into()),
        }
    }

    /// Ingest Claude sessions with incremental parsing and callback
    async fn ingest_claude_streaming(
        cursors: Arc<Mutex<HashMap<String, SessionCursor>>>, callback: Option<EventCallback>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let db = Self::open_db().await?;
        let adapter = ClaudeAdapter::new();

        for session_file in adapter.discover_sessions().await {
            let cursor_key = format!("claude:{}", session_file.session_id);

            let byte_offset = {
                let c = cursors.lock().await;
                match c.get(&cursor_key) {
                    Some(SessionCursor::ByteOffset(off)) => *off,
                    _ => 0,
                }
            };

            let is_new_session = byte_offset == 0;

            if is_new_session {
                if let Ok((session, events)) = adapter.parse_session(&session_file).await {
                    let _ = db.insert_session_with_events(&session, &events).await;
                    let _ = db.compute_session_metrics(&session.id.to_string()).await;

                    let file_len = tokio::fs::metadata(&session_file.path)
                        .await
                        .map(|m| m.len())
                        .unwrap_or(0);

                    {
                        let mut c = cursors.lock().await;
                        c.insert(cursor_key, SessionCursor::ByteOffset(file_len));
                    }

                    if let Some(ref cb) = callback
                        && !events.is_empty()
                    {
                        cb(StreamingEvent {
                            session_external_id: session_file.session_id.clone(),
                            source: "claude".to_string(),
                            project: Some(session_file.project.clone()),
                            new_events: events,
                            is_new_session: true,
                        });
                    }
                }
            } else {
                match adapter.parse_session_incremental(&session_file, byte_offset).await {
                    Ok((new_events, new_offset)) => {
                        if !new_events.is_empty() {
                            if let Ok((session, events)) = adapter.parse_session(&session_file).await {
                                let _ = db.insert_session_with_events(&session, &events).await;
                                let _ = db.compute_session_metrics(&session.id.to_string()).await;
                            }

                            if let Some(ref cb) = callback {
                                cb(StreamingEvent {
                                    session_external_id: session_file.session_id.clone(),
                                    source: "claude".to_string(),
                                    project: Some(session_file.project.clone()),
                                    new_events,
                                    is_new_session: false,
                                });
                            }
                        }

                        {
                            let mut c = cursors.lock().await;
                            c.insert(cursor_key, SessionCursor::ByteOffset(new_offset));
                        }
                    }
                    Err(e) => {
                        log::warn!("Incremental parse failed for {}: {}", session_file.session_id, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Ingest Codex sessions with incremental parsing and callback
    async fn ingest_codex_streaming(
        cursors: Arc<Mutex<HashMap<String, SessionCursor>>>, callback: Option<EventCallback>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let db = Self::open_db().await?;
        let adapter = CodexAdapter::new();

        for session_file in adapter.discover_sessions().await {
            let cursor_key = format!("codex:{}", session_file.session_id);

            let byte_offset = {
                let c = cursors.lock().await;
                match c.get(&cursor_key) {
                    Some(SessionCursor::ByteOffset(off)) => *off,
                    _ => 0,
                }
            };

            let is_new_session = byte_offset == 0;

            if is_new_session {
                if let Ok((session, events)) = adapter.parse_session(&session_file).await {
                    let _ = db.insert_session_with_events(&session, &events).await;
                    let _ = db.compute_session_metrics(&session.id.to_string()).await;

                    let file_len = tokio::fs::metadata(&session_file.path)
                        .await
                        .map(|m| m.len())
                        .unwrap_or(0);

                    {
                        let mut c = cursors.lock().await;
                        c.insert(cursor_key, SessionCursor::ByteOffset(file_len));
                    }

                    if let Some(ref cb) = callback
                        && !events.is_empty()
                    {
                        cb(StreamingEvent {
                            session_external_id: session_file.session_id.clone(),
                            source: "codex".to_string(),
                            project: None,
                            new_events: events,
                            is_new_session: true,
                        });
                    }
                }
            } else {
                match adapter.parse_session_incremental(&session_file, byte_offset).await {
                    Ok((new_events, new_offset)) => {
                        if !new_events.is_empty() {
                            if let Ok((session, events)) = adapter.parse_session(&session_file).await {
                                let _ = db.insert_session_with_events(&session, &events).await;
                                let _ = db.compute_session_metrics(&session.id.to_string()).await;
                            }

                            if let Some(ref cb) = callback {
                                cb(StreamingEvent {
                                    session_external_id: session_file.session_id.clone(),
                                    source: "codex".to_string(),
                                    project: None,
                                    new_events,
                                    is_new_session: false,
                                });
                            }
                        }

                        {
                            let mut c = cursors.lock().await;
                            c.insert(cursor_key, SessionCursor::ByteOffset(new_offset));
                        }
                    }
                    Err(e) => {
                        log::warn!("Incremental parse failed for {}: {}", session_file.session_id, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Ingest OpenCode sessions with incremental parsing and callback
    async fn ingest_opencode_streaming(
        cursors: Arc<Mutex<HashMap<String, SessionCursor>>>, callback: Option<EventCallback>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let db = Self::open_db().await?;
        let adapter = OpenCodeAdapter::new();

        for session in adapter.discover_sessions().await {
            let cursor_key = format!("opencode:{}", session.id);

            let known_files = {
                let c = cursors.lock().await;
                match c.get(&cursor_key) {
                    Some(SessionCursor::KnownFiles(files)) => files.clone(),
                    _ => HashSet::new(),
                }
            };

            let is_new_session = known_files.is_empty();

            if is_new_session {
                if let Ok((session_obj, events)) = adapter.parse_session(&session).await {
                    let _ = db.insert_session_with_events(&session_obj, &events).await;
                    let _ = db.compute_session_metrics(&session_obj.id.to_string()).await;

                    let msg_dir = adapter.storage_path().join("message").join(&session.id);
                    let mut files = HashSet::new();
                    if let Ok(mut entries) = tokio::fs::read_dir(&msg_dir).await {
                        while let Ok(Some(entry)) = entries.next_entry().await {
                            if let Some(name) = entry.file_name().to_str() {
                                files.insert(name.to_string());
                            }
                        }
                    }

                    {
                        let mut c = cursors.lock().await;
                        c.insert(cursor_key, SessionCursor::KnownFiles(files));
                    }

                    if let Some(ref cb) = callback
                        && !events.is_empty()
                    {
                        cb(StreamingEvent {
                            session_external_id: session.id.clone(),
                            source: "opencode".to_string(),
                            project: session.directory.clone(),
                            new_events: events,
                            is_new_session: true,
                        });
                    }
                }
            } else {
                match adapter.parse_session_incremental(&session, &known_files).await {
                    Ok((new_events, new_known)) => {
                        if !new_events.is_empty() {
                            if let Ok((session_obj, events)) = adapter.parse_session(&session).await {
                                let _ = db.insert_session_with_events(&session_obj, &events).await;
                                let _ = db.compute_session_metrics(&session_obj.id.to_string()).await;
                            }

                            if let Some(ref cb) = callback {
                                cb(StreamingEvent {
                                    session_external_id: session.id.clone(),
                                    source: "opencode".to_string(),
                                    project: session.directory.clone(),
                                    new_events,
                                    is_new_session: false,
                                });
                            }
                        }

                        {
                            let mut c = cursors.lock().await;
                            c.insert(cursor_key, SessionCursor::KnownFiles(new_known));
                        }
                    }
                    Err(e) => {
                        log::warn!("Incremental parse failed for {}: {}", session.id, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Ingest Crush sessions with incremental parsing and callback
    async fn ingest_crush_streaming(
        cursors: Arc<Mutex<HashMap<String, SessionCursor>>>, callback: Option<EventCallback>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let db = Self::open_db().await?;
        let adapter = CrushAdapter::new();

        for session_file in adapter.discover_sessions().await {
            let cursor_key = format!("crush:{}", session_file.session_id);

            let last_created_at = {
                let c = cursors.lock().await;
                match c.get(&cursor_key) {
                    Some(SessionCursor::LastCreatedAt(ts)) => *ts,
                    _ => 0,
                }
            };

            let is_new_session = last_created_at == 0;

            if is_new_session {
                if let Ok((session, events)) = adapter.parse_session(&session_file).await {
                    let _ = db.insert_session_with_events(&session, &events).await;
                    let _ = db.compute_session_metrics(&session.id.to_string()).await;

                    let max_ts = events
                        .iter()
                        .filter_map(|e| e.raw_payload.get("created_at").and_then(|v| v.as_i64()))
                        .max()
                        .unwrap_or(0);

                    {
                        let mut c = cursors.lock().await;
                        c.insert(cursor_key, SessionCursor::LastCreatedAt(max_ts));
                    }

                    if let Some(ref cb) = callback
                        && !events.is_empty()
                    {
                        cb(StreamingEvent {
                            session_external_id: session_file.session_id.clone(),
                            source: "crush".to_string(),
                            project: session_file
                                .path
                                .parent()
                                .and_then(|p| p.parent())
                                .and_then(|p| p.file_name())
                                .and_then(|n| n.to_str())
                                .map(|s| s.to_string()),
                            new_events: events,
                            is_new_session: true,
                        });
                    }
                }
            } else {
                match adapter.parse_session_incremental(&session_file, last_created_at).await {
                    Ok((new_events, new_last)) => {
                        if !new_events.is_empty() {
                            if let Ok((session, events)) = adapter.parse_session(&session_file).await {
                                let _ = db.insert_session_with_events(&session, &events).await;
                                let _ = db.compute_session_metrics(&session.id.to_string()).await;
                            }

                            if let Some(ref cb) = callback {
                                cb(StreamingEvent {
                                    session_external_id: session_file.session_id.clone(),
                                    source: "crush".to_string(),
                                    project: session_file
                                        .path
                                        .parent()
                                        .and_then(|p| p.parent())
                                        .and_then(|p| p.file_name())
                                        .and_then(|n| n.to_str())
                                        .map(|s| s.to_string()),
                                    new_events,
                                    is_new_session: false,
                                });
                            }
                        }

                        {
                            let mut c = cursors.lock().await;
                            c.insert(cursor_key, SessionCursor::LastCreatedAt(new_last));
                        }
                    }
                    Err(e) => {
                        log::warn!("Incremental parse failed for {}: {}", session_file.session_id, e);
                    }
                }
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

    #[tokio::test]
    async fn test_watcher_with_callback() {
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();
        let callback: EventCallback = Arc::new(move |_event| {
            let c = called_clone.clone();
            tokio::task::block_in_place(|| {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(async {
                    let mut called = c.lock().await;
                    *called = true;
                });
            });
        });

        let watcher = Watcher::with_callback(WatcherConfig::default(), callback);
        assert!(watcher.event_callback.is_some());
    }
}
