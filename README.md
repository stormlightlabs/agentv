# Agent Viz

A local-first workspace for ingesting, normalizing, and visualizing session artifacts from multiple AI coding agents.

Agent Viz provides a unified view of interactions with AI coding assistants like **Claude Code**, **Codex CLI**, and **OpenCode**, allowing you to search across sessions, analyze tool usage, and visualize agent timelines.

## Key Features

- Multi-Agent Ingestion: Adapters for discovering and parsing session data from:
    - Claude Code: JSONL sessions from `~/.claude/projects/`
    - Codex CLI: Rollout logs from `~/.codex/sessions/`
    - OpenCode: Session exports and logs via `opencode` CLI
    - Crush: _(Planned)_ SQLite database ingestion
- Unified Data Model: All agent events are normalized into a canonical format
  (Messages, Tool Calls, Results, Errors, System Events) while preserving raw payloads for debugging.
- High-Performance Search: Full-text search (FTS5) across all session titles and message content.
- Dual Interfaces:
    - CLI: Powerful command-line tool for batch ingestion, searching, and statistics.
    - Desktop App: Modern, cross-platform interface built with **Tauri 2.x** and **Svelte 5**.
- Local-First: Your data stays on your machine in a local SQLite database.

## Stack

| Backend               | Database      | Frontend                      | CLI  |
| --------------------- | ------------- | ----------------------------- | ---- |
| Rust 2024 (Workspace) | SQLite + FTS5 | Svelte (TS) + Tauri, Tailwind | Clap |

## Getting Started (Development)

### Prerequisites

- [Rust](https://www.rust-lang.org/) (latest stable)
- [Node.js](https://nodejs.org/) & [pnpm](https://pnpm.io/) (for desktop app)

### CLI

Build the CLI:

```bash
cargo build -p agent-viz-cli
```

Common commands:

```bash
# System health check
./target/debug/agent-viz doctor

# Ingest sessions from a source
./target/debug/agent-viz ingest --source claude
./target/debug/agent-viz ingest --source codex
./target/debug/agent-viz ingest --source opencode

# List and search sessions
./target/debug/agent-viz list sessions
./target/debug/agent-viz search "panic" --since 7d
./target/debug/agent-viz show session <id>

# Analytics
./target/debug/agent-viz stats --by day
```

### Desktop Development

Start the desktop application in development mode:

```bash
pnpm tauri dev
```

## Project Structure

```sh
crates/
├── core/         # Canonical data models and shared types
├── adapters/     # Per-tool adapters for parsing different sources
├── store/        # SQLite schema, migrations, and FTS5 search
├── ingest/       # File scanning and incremental ingestion logic
├── api/          # Tauri command handlers (Rust <-> Frontend bridge)
└── cli/          # Command-line interface implementation
apps/
└── desktop/      # Svelte + Tauri desktop wrapper
```

## References

- Inspired by [claude-code-viewer](https://github.com/d-kimuson/claude-code-viewer).
- Designed to work with [Claude Code](https://claude.ai/code), [Codex](https://github.com/openai/codex), and [OpenCode](https://opencode.ai).
