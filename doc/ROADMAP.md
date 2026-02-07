# Agent Viz Roadmap

A Rust workspace shipping a desktop app (SvelteKit + Tauri) and CLI (clap) for ingesting, normalizing, and visualizing local session artifacts from multiple AI agents.

## Data Sources

### Claude Code

- **Location:** `~/.claude/projects/<project>/<session-id>.jsonl`
- **Format:** JSONL with conversation entries, tool calls, approvals
- **Reference:** [claude-code-viewer](https://github.com/d-kimuson/claude-code-viewer)

### Codex CLI

- **Location:** `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl` (defaults to `~/.codex`)
- **Format:** JSONL "rollout" logs, date-sharded
- **Note:** No CLI option to specify custom log path; must discover by glob

### OpenCode

- **Logs:** `~/.local/share/opencode/log/YYYY-MM-DDTHHMMSS.log` (timestamp-named, keeps last 10)
- **Project Data:** `~/.local/share/opencode/project/<project-slug>/storage/` (Git repos) or `./global/storage/` (non-Git)
- **Auth:** `~/.local/share/opencode/auth.json`
- **CLI:** `opencode session list --format json`, `opencode export <sessionID>`
- **Reference:** [Troubleshooting docs](https://opencode.ai/docs/troubleshooting/)

### Crush

- **Location:** `.crush/crush.db` (project-local) or `~/.crush/crush.db` (global)
- **Format:** SQLite with published schema.json
- **Note:** Schema may change; use feature probes (table/column existence)
- **Reference:** [schema.json](https://github.com/charmbracelet/crush/blob/main/schema.json)

## Project Structure

```sh
.
  Cargo.toml                 # workspace
  crates/
    core/                    # canonical model + normalization types
    adapters/                # per-tool adapters + shared parsing utils
      claude_code/
      codex/
      opencode/
      crush/
    store/                   # SQLite schema, migrations, queries, FTS
    ingest/                  # scanning + incremental ingest + watchers
    api/                     # Tauri command handlers (query/search/stream)
    cli/                     # clap CLI (depends on store+ingest)
  apps/
    desktop/                 # Tauri + SvelteKit
      src-tauri/             # minimal Rust glue calling crates/api
      web/                   # SvelteKit app
```

## Milestones

### M0 — Skeleton + Shared DB

**Goal:** Workspace scaffolding with SQLite + FTS5. CLI commands work against empty DB.

**Tasks:**

- [x] Workspace layout (crates + apps)
- [x] SQLite migrations + FTS5 virtual tables
- [x] Canonical structs + serde JSON for lossless payloads
- [x] CLI: `doctor` (validate paths, DB migrations, adapter health)
- [x] CLI: `ingest --help` (stub)
- [x] CLI: `list sessions` (empty DB returns nothing)

**CLI Testing:**

```bash
# Build and test CLI
cargo build -p agent-viz-cli
./target/debug/agent-viz doctor
./target/debug/agent-viz list sessions
```

### M1 — Claude Code Adapter (Read-Only)

**Goal:** Ingest Claude Code JSONL sessions and render read-only timeline.

**Tasks:**

- [x] Adapter: discover `~/.claude/projects/.../*.jsonl`
- [x] Adapter: parse JSONL into canonical events (messages, tool calls, errors)
- [x] Store: lossless ingestion with raw JSON preservation
- [x] CLI: `ingest --source claude` (batch import)
- [x] CLI: `show session <id>` (timeline view)
- [x] Desktop: session list + session viewer (progressive disclosure)

**CLI Testing:**

```bash
# Ingest Claude Code sessions
./target/debug/agent-viz ingest --source claude

# List and view sessions
./target/debug/agent-viz list sessions --source claude
./target/debug/agent-viz show session <session-id>
./target/debug/agent-viz search "error"
```

### M2 — Search + Charts

**Goal:** FTS search with facets + two core charts.

**Tasks:**

- [x] FTS5 query layer with highlighting
- [x] Faceted filtering (source, project, date range, kind)
- [x] CLI: `search "query" --since 7d --source claude`
- [x] Desktop: search page with facets + result list
- [x] Analytics: activity chart (events/day)
- [x] Analytics: errors chart (errors/day + top signatures)

**CLI Testing:**

```bash
# Search with filters
./target/debug/agent-viz search "panic" --since 7d --source claude
./target/debug/agent-viz search "tool:call" --kind tool.call

# Stats
./target/debug/agent-viz stats --by day
./target/debug/agent-viz stats --by tool
```

### Note

For all adapters, verify with:

```sh
./target/debug/agent-viz show <session-id>
./target/debug/agent-viz stats --by source
./target/debug/agent-viz doctor
```

### M3 — Codex Adapter

**Goal:** Ingest Codex CLI rollout logs.

**Tasks:**

- [x] Adapter: discover `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl`
- [x] Adapter: parse rollout records into canonical events
- [x] CLI: `ingest --source codex`
- [x] Desktop: source filter + per-source health status

**CLI Testing:**

```bash
./target/debug/agent-viz ingest --source codex
./target/debug/agent-viz list sessions --source codex
./target/debug/agent-viz search "rollout" --source codex
```

### M4 — OpenCode Adapter

**Goal:** Ingest OpenCode logs and session data.

**Tasks:**

- [x] Adapter: discover `~/.local/share/opencode/log/` (timestamp-named log files)
- [x] Adapter: parse log files and session JSONs from `project/` storage
- [x] Adapter: read auth.json for provider/model context
- [x] CLI: `ingest --source opencode`
- [x] Cross-tool search: query across all sources

**CLI Testing:**

```bash
./target/debug/agent-viz ingest --source opencode
./target/debug/agent-viz list sessions
./target/debug/agent-viz search "error" --source opencode
```

### M5 — Crush Adapter

**Goal:** Read-only SQLite adapter for Crush database.

**Tasks:**

- [x] Adapter: discover `.crush/crush.db` (project-local) and `~/.crush/crush.db` (global)
- [x] Adapter: schema probing + graceful degradation (feature probes for table/column existence)
- [x] CLI: `ingest --source crush`
- [x] Desktop: sessions + messages with raw JSON fallback

**CLI Testing:**

```bash
./target/debug/agent-viz ingest --source crush
./target/debug/agent-viz list sessions --source crush
```

### M6 — Incremental Ingest + Watchers

**Goal:** Auto-detect new sessions and changes.

**Tasks:**

- [x] Filesystem watchers for Claude/Codex/OpenCode logs
- [x] DB polling for Crush (mtime + latest message probe)
- [x] CLI: `ingest --watch` (daemon mode)
- [x] Desktop: auto-refresh indicator

**CLI Testing:**

```bash
# Watch mode (runs continuously)
./target/debug/agent-viz ingest --watch

# Or one-time incremental
./target/debug/agent-viz ingest --incremental
```

### M7 — Analytics Polish

**Goal:** Complete Tier 1 + Tier 2 charts.

**Tasks:**

- [x] Session metrics table (computed at ingest)
- [x] Tool-call frequency chart (`stats --by tools`)
- [x] Files touched leaderboard (`stats --by files`)
- [x] Patch churn (lines added/removed) (`stats --by churn`)
- [x] Latency tracking (long-running tool calls) (`stats --by latency`)
- [x] Export: `md`, `json`, `jsonl` formats (`export --session <id> --format md`)

**CLI Testing:**

```bash
./target/debug/agent-viz stats --by project
./target/debug/agent-viz stats --by tools
./target/debug/agent-viz stats --by files
./target/debug/agent-viz stats --by churn
./target/debug/agent-viz stats --by latency
./target/debug/agent-viz export --session <id> --format md
./target/debug/agent-viz export --search "error" --format jsonl
```

### M8 — CCV Parity: Interactive Client

**Goal:** Start/resume/continue Claude Code sessions from UI.

**Tasks:**

- [ ] Configure Claude Code executable path
- [ ] Spawn Claude Code process with PTY
- [ ] Stream stdin/stdout into UI
- [ ] Resume by session-id
- [ ] Continue without session-id reassignment
- [ ] Pause/resume controls
- [ ] Tool approval flow

**CLI Testing:**

```bash
# Configure and test runtime
./target/debug/agent-viz config set claude.executable /usr/local/bin/claude
./target/debug/agent-viz runtime start --project <path>
./target/debug/agent-viz runtime list
./target/debug/agent-viz runtime attach <session-id>
```

### M9 — CCV Parity: Project Management

**Goal:** Create projects and auto-discovery.

**Tasks:**

- [ ] Create project UI (select directory, run `/init`)
- [ ] Auto-discover projects from `~/.claude/projects/`
- [ ] Setting: hide sessions without user messages
- [ ] Setting: unify sessions with same title

### M10 — CCV Parity: File Upload & Preview

**Goal:** Upload and preview attachments.

**Tasks:**

- [ ] Upload images (PNG, JPEG, GIF, WebP), PDFs, text files
- [ ] Inline preview components
- [ ] Store attachments in app data dir
- [ ] Reference in event stream

### M11 — CCV Parity: Browser Preview

**Goal:** Right-side browser panel.

**Tasks:**

- [ ] Detect URLs in messages
- [ ] Resizable right-side panel
- [ ] URL input + reload
- [ ] Track URL changes same-origin

### M12 — CCV Parity: Git Workflow

**Goal:** In-app git diff viewer + commit/push.

**Tasks:**

- [ ] Diff viewer (staged/unstaged, file tree + hunks)
- [ ] Commit UI
- [ ] Push UI (push-only, commit+push)
- [ ] Session context integration

### M13 — CCV Parity: Scheduler + MCP

**Goal:** Message scheduling and system monitoring.

**Tasks:**

- [ ] Scheduler: one-off datetime + cron recurring
- [ ] Concurrency policies (skip/run)
- [ ] Rate limit auto-continue
- [ ] MCP server viewer in sidebar
- [ ] System info monitor

### M14 — CCV Parity: i18n + Polish

**Goal:** Multi-language support and final UX.

**Tasks:**

- [ ] i18n framework (en/ja/zh-Hans)
- [ ] Theme: system/dark/light
- [ ] Audio notifications
- [ ] Mobile-optimized layouts

## Quick Start for Development

```bash
# Build entire workspace
cargo build

# Run CLI commands during development
cargo run -p agent-viz-cli -- doctor
cargo run -p agent-viz-cli -- list sessions
cargo run -p agent-viz-cli -- ingest --source claude

# Run desktop app (after M1)
cargo tauri dev
```

## Acceptance Criteria by Phase

- **M0-M1:** Can ingest Claude Code sessions and view them in CLI + Desktop
- **M2:** Can search across sessions with FTS and view activity/errors charts
- **M3-M5:** Can ingest all four agent sources and search across them
- **M6:** New sessions appear automatically without manual re-ingest
- **M7:** Can export sessions and view comprehensive analytics
- **M8-M14:** Feature parity with claude-code-viewer

## References

- [claude-code-viewer](https://github.com/d-kimuson/claude-code-viewer) - Gold standard for progressive disclosure UI + strict schema validation
- [Codex CLI Issue #2288](https://github.com/openai/codex/issues/2288) - JSON output feature request
- [OpenCode Troubleshooting](https://opencode.ai/docs/troubleshooting/) - Log locations and storage details
- [OpenCode CLI](https://opencode.ai/docs/cli/) - Session export commands
- [Crush schema.json](https://github.com/charmbracelet/crush/blob/main/schema.json) - Published schema for adapter development
- [Crush Issue #939](https://github.com/charmbracelet/crush/issues/939) - Context rebuild discussion
