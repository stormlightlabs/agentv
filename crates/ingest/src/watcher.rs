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

/// Progress information emitted during ingestion
#[derive(Debug, Clone, Serialize)]
pub struct IngestProgress {
    pub source: String,
    pub phase: String,
    pub current: usize,
    pub total: usize,
}

/// Callback type invoked when new events are detected
pub type EventCallback = Arc<dyn Fn(StreamingEvent) + Send + Sync>;

/// Callback type invoked to report ingestion progress
pub type ProgressCallback = Arc<dyn Fn(IngestProgress) + Send + Sync>;

/// Cursor for tracking incremental parse position per session
#[derive(Debug, Clone)]
enum SessionCursor {
    /// Byte offset for JSONL files (Claude, Codex)
    ByteOffset(u64),
    /// Known message keys/signatures (OpenCode)
    KnownFiles(HashSet<String>),
    /// Last message created_at timestamp (Crush)
    LastCreatedAt(i64),
}

/// Watcher for file system changes and database updates
pub struct Watcher {
    config: WatcherConfig,
    stats: Arc<Mutex<Vec<IngestStats>>>,
    cursors: Arc<Mutex<HashMap<String, SessionCursor>>>,
    file_mtimes: Arc<Mutex<HashMap<PathBuf, SystemTime>>>,
    dirty_sessions: Arc<Mutex<HashSet<String>>>,
    event_callback: Option<EventCallback>,
    progress_callback: Option<ProgressCallback>,
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
            file_mtimes: Arc::new(Mutex::new(HashMap::new())),
            dirty_sessions: Arc::new(Mutex::new(HashSet::new())),
            event_callback: None,
            progress_callback: None,
        }
    }

    /// Create a new watcher with a callback for streaming events
    pub fn with_callback(config: WatcherConfig, callback: EventCallback) -> Self {
        Self {
            config,
            stats: Arc::new(Mutex::new(Vec::new())),
            cursors: Arc::new(Mutex::new(HashMap::new())),
            file_mtimes: Arc::new(Mutex::new(HashMap::new())),
            dirty_sessions: Arc::new(Mutex::new(HashSet::new())),
            event_callback: Some(callback),
            progress_callback: None,
        }
    }

    /// Create a new watcher with callbacks for streaming events and progress
    pub fn with_callbacks(
        config: WatcherConfig, event_callback: EventCallback, progress_callback: ProgressCallback,
    ) -> Self {
        Self {
            config,
            stats: Arc::new(Mutex::new(Vec::new())),
            cursors: Arc::new(Mutex::new(HashMap::new())),
            file_mtimes: Arc::new(Mutex::new(HashMap::new())),
            dirty_sessions: Arc::new(Mutex::new(HashSet::new())),
            event_callback: Some(event_callback),
            progress_callback: Some(progress_callback),
        }
    }

    /// Start watching all sources
    pub async fn watch_all(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (tx, mut rx) = mpsc::channel(100);

        let claude_paths = self.get_claude_watch_paths().await;
        let codex_paths = self.get_codex_watch_paths().await;
        let opencode_paths = self.get_opencode_watch_paths().await;
        let source_roots: Vec<(String, PathBuf)> = claude_paths
            .iter()
            .cloned()
            .map(|p| ("claude".to_string(), p))
            .chain(codex_paths.iter().cloned().map(|p| ("codex".to_string(), p)))
            .chain(opencode_paths.iter().cloned().map(|p| ("opencode".to_string(), p)))
            .collect();

        let tx_clone = tx.clone();
        let roots_for_events = source_roots.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let mut matched_sources = HashSet::new();
                    for changed_path in &event.paths {
                        for (source, root) in &roots_for_events {
                            if changed_path.starts_with(root) {
                                matched_sources.insert(source.clone());
                            }
                        }
                    }

                    for source in matched_sources {
                        let _ = tx_clone.try_send((source, SystemTime::now()));
                    }
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
        let crush_progress = self.progress_callback.clone();
        let crush_mtimes = self.file_mtimes.clone();
        let crush_dirty = self.dirty_sessions.clone();
        let crush_handle = tokio::spawn(Self::watch_crush_streaming(
            crush_poll_interval,
            crush_stats,
            crush_cursors,
            crush_callback,
            crush_progress,
            crush_mtimes,
            crush_dirty,
        ));

        let metrics_dirty = self.dirty_sessions.clone();
        let _metrics_handle = tokio::spawn(Self::background_metrics_worker(metrics_dirty));

        let debounce_duration = self.config.debounce_duration;
        let stats = self.stats.clone();
        let cursors = self.cursors.clone();
        let callback = self.event_callback.clone();
        let progress = self.progress_callback.clone();
        let mtimes = self.file_mtimes.clone();
        let dirty = self.dirty_sessions.clone();
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
                                    progress.clone(),
                                    mtimes.clone(),
                                    dirty.clone(),
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
                                    Err(e) => log::error!("Failed to ingest from {:?}: {}", src, e),
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

                let metrics_dirty = self.dirty_sessions.clone();
                let _metrics_handle = tokio::spawn(Self::background_metrics_worker(metrics_dirty));

                let debounce_duration = self.config.debounce_duration;
                let stats = self.stats.clone();
                let cursors = self.cursors.clone();
                let callback = self.event_callback.clone();
                let progress = self.progress_callback.clone();
                let mtimes = self.file_mtimes.clone();
                let dirty = self.dirty_sessions.clone();
                let source_str = source.to_string();

                while let Some(_ts) = rx.recv().await {
                    tokio::time::sleep(debounce_duration).await;

                    while rx.try_recv().is_ok() {
                        log::debug!("Draining pending events");
                    }

                    log::info!("Ingesting from {:?} due to file change", source);
                    match Self::ingest_source_streaming(
                        source,
                        cursors.clone(),
                        callback.clone(),
                        progress.clone(),
                        mtimes.clone(),
                        dirty.clone(),
                    )
                    .await
                    {
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
                let metrics_dirty = self.dirty_sessions.clone();
                let _metrics_handle = tokio::spawn(Self::background_metrics_worker(metrics_dirty));

                Self::watch_crush_streaming(
                    self.config.crush_poll_interval,
                    self.stats.clone(),
                    self.cursors.clone(),
                    self.event_callback.clone(),
                    self.progress_callback.clone(),
                    self.file_mtimes.clone(),
                    self.dirty_sessions.clone(),
                )
                .await;
            }
        }

        Ok(())
    }

    /// Background worker that computes metrics for dirty sessions on a debounced interval
    async fn background_metrics_worker(dirty_sessions: Arc<Mutex<HashSet<String>>>) {
        let mut tick = interval(Duration::from_secs(5));
        loop {
            tick.tick().await;

            let sessions_to_compute: Vec<String> = {
                let mut dirty = dirty_sessions.lock().await;
                if dirty.is_empty() {
                    continue;
                }
                let batch: Vec<String> = dirty.drain().collect();
                batch
            };

            if sessions_to_compute.is_empty() {
                continue;
            }

            log::info!("Computing metrics for {} dirty sessions", sessions_to_compute.len());
            let db_ok = match Database::open_default().await {
                Ok(db) => Some(db),
                Err(e) => {
                    log::error!("Failed to open DB for metric computation: {}", e);
                    None
                }
            };
            match db_ok {
                Some(db) => {
                    for session_id in &sessions_to_compute {
                        if let Err(e) = db.compute_session_metrics(session_id).await {
                            log::warn!("Failed to compute metrics for {}: {}", session_id, e);
                        }
                    }
                }
                None => {
                    let mut dirty = dirty_sessions.lock().await;
                    for sid in sessions_to_compute {
                        dirty.insert(sid);
                    }
                }
            }
        }
    }

    /// Watch Crush database for changes with streaming support
    async fn watch_crush_streaming(
        poll_interval: Duration, stats: Arc<Mutex<Vec<IngestStats>>>,
        cursors: Arc<Mutex<HashMap<String, SessionCursor>>>, callback: Option<EventCallback>,
        progress: Option<ProgressCallback>, _mtimes: Arc<Mutex<HashMap<PathBuf, SystemTime>>>,
        dirty_sessions: Arc<Mutex<HashSet<String>>>,
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
                match Self::ingest_crush_streaming(
                    cursors.clone(),
                    callback.clone(),
                    progress.clone(),
                    dirty_sessions.clone(),
                )
                .await
                {
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
        vec![adapter.log_path().clone()]
    }

    /// Check if a file has been modified since last processing
    async fn file_changed(path: &PathBuf, mtimes: &Arc<Mutex<HashMap<PathBuf, SystemTime>>>) -> bool {
        let current_mtime = match tokio::fs::metadata(path).await {
            Ok(meta) => match meta.modified() {
                Ok(mtime) => mtime,
                Err(_) => return true,
            },
            Err(_) => return true,
        };

        let cached = {
            let cache = mtimes.lock().await;
            cache.get(path).copied()
        };

        match cached {
            Some(cached_mtime) => current_mtime != cached_mtime,
            None => true,
        }
    }

    /// Update the cached mtime for a file
    async fn update_mtime(path: &PathBuf, mtimes: &Arc<Mutex<HashMap<PathBuf, SystemTime>>>) {
        if let Ok(meta) = tokio::fs::metadata(path).await
            && let Ok(mtime) = meta.modified()
        {
            let mut cache = mtimes.lock().await;
            cache.insert(path.clone(), mtime);
        }
    }

    /// Ingest from a specific source with streaming callback support
    async fn ingest_source_streaming(
        source: Source, cursors: Arc<Mutex<HashMap<String, SessionCursor>>>, callback: Option<EventCallback>,
        progress: Option<ProgressCallback>, mtimes: Arc<Mutex<HashMap<PathBuf, SystemTime>>>,
        dirty_sessions: Arc<Mutex<HashSet<String>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match source {
            Source::Claude => Self::ingest_claude_streaming(cursors, callback, progress, mtimes, dirty_sessions).await,
            Source::Codex => Self::ingest_codex_streaming(cursors, callback, progress, mtimes, dirty_sessions).await,
            Source::OpenCode => {
                Self::ingest_opencode_streaming(cursors, callback, progress, mtimes, dirty_sessions).await
            }
            Source::Crush => Self::ingest_crush_streaming(cursors, callback, progress, dirty_sessions).await,
        }
    }

    /// Helper function to open database with proper error conversion
    async fn open_db() -> Result<Database, Box<dyn std::error::Error + Send + Sync>> {
        match Database::open_default().await {
            Ok(db) => Ok(db),
            Err(e) => Err(format!("Database error: {}", e).into()),
        }
    }

    /// Mark a session as needing metric recomputation
    async fn mark_dirty(dirty_sessions: &Arc<Mutex<HashSet<String>>>, session_id: &str) {
        let mut dirty = dirty_sessions.lock().await;
        dirty.insert(session_id.to_string());
    }

    /// Ingest Claude sessions with incremental parsing and callback
    async fn ingest_claude_streaming(
        cursors: Arc<Mutex<HashMap<String, SessionCursor>>>, callback: Option<EventCallback>,
        progress: Option<ProgressCallback>, mtimes: Arc<Mutex<HashMap<PathBuf, SystemTime>>>,
        dirty_sessions: Arc<Mutex<HashSet<String>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let db = Self::open_db().await?;
        let adapter = ClaudeAdapter::new();
        let session_files = adapter.discover_sessions().await;
        let total = session_files.len();

        for (idx, session_file) in session_files.into_iter().enumerate() {
            if let Some(ref pcb) = progress {
                pcb(IngestProgress {
                    source: "claude".to_string(),
                    phase: "Ingesting sessions".to_string(),
                    current: idx,
                    total,
                });
            }

            if !Self::file_changed(&session_file.path, &mtimes).await {
                log::debug!("Skipping unchanged file: {:?}", session_file.path);
                continue;
            }

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
                    let session_id = session.id.to_string();
                    let _ = db.insert_session_with_events(&session, &events).await;
                    Self::mark_dirty(&dirty_sessions, &session_id).await;

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
                            if let Ok(Some(session_id)) =
                                db.get_session_id_by_external("claude", &session_file.session_id).await
                            {
                                let _ = db.append_events(&session_id, &new_events).await;
                                let _ = db.update_session_timestamp(&session_id, &chrono::Utc::now()).await;
                                Self::mark_dirty(&dirty_sessions, &session_id).await;
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
                    Err(e) => log::warn!("Incremental parse failed for {}: {}", session_file.session_id, e),
                }
            }

            Self::update_mtime(&session_file.path, &mtimes).await;
        }

        if let Some(ref pcb) = progress {
            pcb(IngestProgress { source: "claude".to_string(), phase: "Complete".to_string(), current: total, total });
        }

        Ok(())
    }

    /// Ingest Codex sessions with incremental parsing and callback
    async fn ingest_codex_streaming(
        cursors: Arc<Mutex<HashMap<String, SessionCursor>>>, callback: Option<EventCallback>,
        progress: Option<ProgressCallback>, mtimes: Arc<Mutex<HashMap<PathBuf, SystemTime>>>,
        dirty_sessions: Arc<Mutex<HashSet<String>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let db = Self::open_db().await?;
        let adapter = CodexAdapter::new();
        let session_files = adapter.discover_sessions().await;
        let total = session_files.len();

        for (idx, session_file) in session_files.into_iter().enumerate() {
            if let Some(ref pcb) = progress {
                pcb(IngestProgress {
                    source: "codex".to_string(),
                    phase: "Ingesting sessions".to_string(),
                    current: idx,
                    total,
                });
            }

            if !Self::file_changed(&session_file.path, &mtimes).await {
                log::debug!("Skipping unchanged file: {:?}", session_file.path);
                continue;
            }

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
                    let session_id = session.id.to_string();
                    let _ = db.insert_session_with_events(&session, &events).await;
                    Self::mark_dirty(&dirty_sessions, &session_id).await;

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
                            if let Ok(Some(session_id)) =
                                db.get_session_id_by_external("codex", &session_file.session_id).await
                            {
                                let _ = db.append_events(&session_id, &new_events).await;
                                let _ = db.update_session_timestamp(&session_id, &chrono::Utc::now()).await;
                                Self::mark_dirty(&dirty_sessions, &session_id).await;
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
                    Err(e) => log::warn!("Incremental parse failed for {}: {}", session_file.session_id, e),
                }
            }

            Self::update_mtime(&session_file.path, &mtimes).await;
        }

        if let Some(ref pcb) = progress {
            pcb(IngestProgress { source: "codex".to_string(), phase: "Complete".to_string(), current: total, total });
        }

        Ok(())
    }

    /// Ingest OpenCode sessions with incremental parsing and callback
    async fn ingest_opencode_streaming(
        cursors: Arc<Mutex<HashMap<String, SessionCursor>>>, callback: Option<EventCallback>,
        progress: Option<ProgressCallback>, _mtimes: Arc<Mutex<HashMap<PathBuf, SystemTime>>>,
        dirty_sessions: Arc<Mutex<HashSet<String>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let db = Self::open_db().await?;
        let adapter = OpenCodeAdapter::new();
        let sessions_list = adapter.discover_sessions().await;
        let total = sessions_list.len();

        for (idx, session) in sessions_list.into_iter().enumerate() {
            if let Some(ref pcb) = progress {
                pcb(IngestProgress {
                    source: "opencode".to_string(),
                    phase: "Ingesting sessions".to_string(),
                    current: idx,
                    total,
                });
            }

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
                    let session_id = session_obj.id.to_string();
                    let _ = db.insert_session_with_events(&session_obj, &events).await;
                    Self::mark_dirty(&dirty_sessions, &session_id).await;

                    let files = adapter.collect_incremental_known_files(&session.id).await;

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
                            if let Ok(Some(session_id)) = db.get_session_id_by_external("opencode", &session.id).await {
                                let _ = db.append_events(&session_id, &new_events).await;
                                let _ = db.update_session_timestamp(&session_id, &chrono::Utc::now()).await;
                                Self::mark_dirty(&dirty_sessions, &session_id).await;
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
                    Err(e) => log::warn!("Incremental parse failed for {}: {}", session.id, e),
                }
            }
        }

        if let Some(ref pcb) = progress {
            pcb(IngestProgress {
                source: "opencode".to_string(),
                phase: "Complete".to_string(),
                current: total,
                total,
            });
        }

        Ok(())
    }

    /// Ingest Crush sessions with incremental parsing and callback
    async fn ingest_crush_streaming(
        cursors: Arc<Mutex<HashMap<String, SessionCursor>>>, callback: Option<EventCallback>,
        progress: Option<ProgressCallback>, dirty_sessions: Arc<Mutex<HashSet<String>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let db = Self::open_db().await?;
        let adapter = CrushAdapter::new();
        let session_files = adapter.discover_sessions().await;
        let total = session_files.len();

        for (idx, session_file) in session_files.into_iter().enumerate() {
            if let Some(ref pcb) = progress {
                pcb(IngestProgress {
                    source: "crush".to_string(),
                    phase: "Ingesting sessions".to_string(),
                    current: idx,
                    total,
                });
            }

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
                    let session_id = session.id.to_string();
                    let _ = db.insert_session_with_events(&session, &events).await;
                    Self::mark_dirty(&dirty_sessions, &session_id).await;

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
                            if let Ok(Some(session_id)) =
                                db.get_session_id_by_external("crush", &session_file.session_id).await
                            {
                                let _ = db.append_events(&session_id, &new_events).await;
                                let _ = db.update_session_timestamp(&session_id, &chrono::Utc::now()).await;
                                Self::mark_dirty(&dirty_sessions, &session_id).await;
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
                    Err(e) => log::warn!("Incremental parse failed for {}: {}", session_file.session_id, e),
                }
            }
        }

        if let Some(ref pcb) = progress {
            pcb(IngestProgress { source: "crush".to_string(), phase: "Complete".to_string(), current: total, total });
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
