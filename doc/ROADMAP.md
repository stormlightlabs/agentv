# Agent V Roadmap

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
cargo build -p agent-v-cli
./target/debug/agent-v doctor
./target/debug/agent-v list sessions
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
./target/debug/agent-v ingest --source claude

# List and view sessions
./target/debug/agent-v list sessions --source claude
./target/debug/agent-v show session <session-id>
./target/debug/agent-v search "error"
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
./target/debug/agent-v search "panic" --since 7d --source claude
./target/debug/agent-v search "tool:call" --kind tool.call

# Stats
./target/debug/agent-v stats --by day
./target/debug/agent-v stats --by tool
```

### Note

For all adapters, verify with:

```sh
./target/debug/agent-v show <session-id>
./target/debug/agent-v stats --by source
./target/debug/agent-v doctor
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
./target/debug/agent-v ingest --source codex
./target/debug/agent-v list sessions --source codex
./target/debug/agent-v search "rollout" --source codex
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
./target/debug/agent-v ingest --source opencode
./target/debug/agent-v list sessions
./target/debug/agent-v search "error" --source opencode
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
./target/debug/agent-v ingest --source crush
./target/debug/agent-v list sessions --source crush
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
./target/debug/agent-v ingest --watch

# Or one-time incremental
./target/debug/agent-v ingest --incremental
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
./target/debug/agent-v stats --by project
./target/debug/agent-v stats --by tools
./target/debug/agent-v stats --by files
./target/debug/agent-v stats --by churn
./target/debug/agent-v stats --by latency
./target/debug/agent-v export --session <id> --format md
./target/debug/agent-v export --search "error" --format jsonl
```

### M8 — Alpha Release: Installable App + Updates

**Goal:** Ship a downloadable alpha that users can install, update, and evaluate without local dev setup.

**Tasks:**

- [ ] Enable Tauri updater in desktop app with signed update verification
- [ ] Deploy Cloudflare Worker updater endpoint (serving platform-specific manifests/artifacts)
- [ ] Add CI release pipeline to build + sign bundles and publish update metadata
- [ ] Implement in-app update UX (check/download/install/restart + release notes)
- [ ] Add first-run onboarding (source health check, one-click ingest, empty-state guidance)
- [ ] Add diagnostics flow (log file location, copy debug bundle, actionable error messages)
- [ ] Publish alpha install docs (macOS/Windows/Linux), known limits, and rollback steps

**Release Testing:**

```bash
# Build release artifacts
pnpm --filter agent-v-gui tauri build

# Verify local health + baseline functionality
./target/debug/agent-v doctor
./target/debug/agent-v ingest --source claude
./target/debug/agent-v list sessions
```

### M9 — Log Browser Core

**Goal:** Make high-volume agent event logs fast to browse and filter.

**Tasks:**

- [ ] Add global event browser with virtualization (100k+ events)
- [ ] Add unified filter bar (source/project/kind/role/tool/date/full-text)
- [ ] Add deep-linkable URL state for active query + selected session/event
- [ ] Add keyboard-first navigation and quick actions (copy id/payload/open session)
- [ ] Add saved filter presets for recurring investigations

### M10 — Event Inspector + Correlation

**Goal:** Make each event inspectable, explainable, and traceable.

**Tasks:**

- [ ] Side-by-side normalized fields + raw payload JSON view
- [ ] Schema-aware renderers for message, tool call, tool result, error, and system events
- [ ] Correlation graph navigation (tool_call -> tool_result, parent/child chains)
- [ ] Adjacent-event diff mode for prompt/response and tool output changes
- [ ] Sensitive-field redaction toggles for copy/export safety

### M11 — Visual Analytics Workbench

**Goal:** Turn logs into interactive operational insights.

**Tasks:**

- [ ] Reusable chart primitives for timeseries, histogram, and leaderboard views
- [ ] Multi-series throughput charts (events/sessions by source/project/model)
- [ ] Latency distributions (p50/p95/p99) with outlier drill-down
- [ ] Error signature trends and top-regression views
- [ ] Click-through from chart points into pre-filtered event browser results

### M12 — Compare Modes + Regression Signals

**Goal:** Explain what changed between runs, projects, or date windows.

**Tasks:**

- [ ] Side-by-side compare mode for two time windows or project scopes
- [ ] Delta panels for volume, latency, tool usage, churn, and error rates
- [ ] Configurable anomaly detection for spikes/drops with threshold controls
- [ ] Timeline annotations (release markers, ingest incidents, manual notes)
- [ ] Export comparison reports (`md`/`json`) with reproducible query parameters

### M13 — Ingestion Freshness + Reliability

**Goal:** Ensure near-real-time ingest is trustworthy at scale.

**Tasks:**

- [ ] Add per-source ingest history (last success, duration, failures, lag)
- [ ] Complete OpenCode watcher support and close current watch-path gap
- [ ] Add retry/backoff + dead-letter handling for parse/ingest failures
- [ ] Add stale-source alerts in desktop status panel
- [ ] Add idempotency and duplicate-ingest verification checks

### M14 — Daily Driver Polish for Analysts

**Goal:** Make the app a default workflow for log triage and performance analysis.

**Tasks:**

- [ ] Command palette for search/filter/navigation actions
- [ ] Favorites + investigation bookmarks for sessions, filters, and chart states
- [ ] Workspace profiles (path sets, defaults, and per-profile source preferences)
- [ ] One-click export bundles (charts + stats + referenced sessions)
- [ ] Performance targets on reference dataset (startup, query p95, memory budget)

## Acceptance Criteria by Phase

- **M0-M1:** Can ingest Claude Code sessions and view them in CLI + Desktop
- **M2:** Can search across sessions with FTS and view activity/errors charts
- **M3-M5:** Can ingest all four agent sources and search across them
- **M6:** New sessions appear automatically without manual re-ingest
- **M7:** Can export sessions and view comprehensive analytics
- **M8:** Users can download an alpha build, install it, and receive updates via Cloudflare Worker-backed Tauri updater
- **M9-M10:** Users can browse large log volumes and deeply inspect correlated event details
- **M11-M12:** Users can visualize trends and compare windows/projects to detect regressions
- **M13:** Continuous ingest is observable, resilient, and freshness-aware across all sources
- **M14:** The tool is polished enough for daily investigation workflows

## References

- [claude-code-viewer](https://github.com/d-kimuson/claude-code-viewer) - Progressive disclosure UI inspiration
- [Codex CLI Issue #2288](https://github.com/openai/codex/issues/2288) - JSON output feature request
- [OpenCode Troubleshooting](https://opencode.ai/docs/troubleshooting/) - Log locations and storage details
- [OpenCode CLI](https://opencode.ai/docs/cli/) - Session export commands
- [Crush schema.json](https://github.com/charmbracelet/crush/blob/main/schema.json) - Published schema for adapter development
- [Crush Issue #939](https://github.com/charmbracelet/crush/issues/939) - Context rebuild discussion
