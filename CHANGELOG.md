# CHANGELOG

## [Unreleased]

## 2026-02-06

- Filesystem watchers for Claude, Codex, and OpenCode, and DB polling for Crush.
    - Allows for real-time session discovery and incremental ingestion via the new `--watch` mode.
- Detailed metrics for tool-call frequency, file usage leaderboards, and patch churn.
    - Supports session and search result exports to Markdown, JSON, and JSONL formats for external analysis.

## 2026-02-05

- Read-only adapters for Claude Code, Codex, OpenCode, and Crush, enabling unified parsing and storage of agent sessions into a canonical event stream.
- Faceted full-text search layer and visual dashboards across CLI and Desktop interfaces.
    - Allows filtering sessions by agent source, project, and error signatures with result highlighting and activity charts.
