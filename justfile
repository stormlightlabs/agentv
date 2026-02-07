# Agent V Justfile
# Quick commands for development workflow

# Show available commands
default:
    @just --list

# Start the SvelteKit dev server
dev-web:
    pnpm --filter agent-v-gui dev

# Run Tauri desktop app
dev-desktop:
    pnpm --filter agent-v-gui tauri dev

# Run CLI
cli *ARGS:
    cargo run -p agent-v-cli -- {{ ARGS }}

# Build all Rust crates
build-rust:
    cargo build --workspace

# Build release binaries
build-release:
    cargo build --workspace --release

# Build desktop app for production
build-desktop:
    pnpm --filter agent-v-gui tauri build

# Run all tests
test:
    cargo test --workspace

# Run Clippy lints
lint:
    cargo clippy --fix --allow-dirty

# Format code
fmt:
    cargo fmt --all
    pnpm --filter agent-v-gui format

# Type check SvelteKit
check-web:
    pnpm --filter agent-v-gui check

# Run all checks
check: lint test check-web

# CLI Operations
# ==============

# Run doctor command
doctor:
    cargo run -p agent-v-cli -- doctor

# Ingest from all sources
ingest:
    cargo run -p agent-v-cli -- ingest

# Ingest from specific source (claude|codex|opencode|crush)
ingest-source source:
    cargo run -p agent-v-cli -- ingest --source {{ source }}

# Watch for new sessions
watch:
    cargo run -p agent-v-cli -- ingest --watch

# List sessions
list:
    cargo run -p agent-v-cli -- list sessions

# Search sessions
search query:
    cargo run -p agent-v-cli -- search "{{ query }}"

# Show session details
show session_id:
    cargo run -p agent-v-cli -- show session {{ session_id }}

# Get statistics
stats *ARGS:
    cargo run -p agent-v-cli -- stats {{ ARGS }}

# Export session
export session_id format="md":
    cargo run -p agent-v-cli -- export --session {{ session_id }} --format {{ format }}

# Generate version from git describe
version:
    @node scripts/get-version.js

# Deploy Cloudflare Worker
deploy-worker:
    cd apps/worker && pnpm deploy

# Build release artifacts locally (unsigned)
build-local:
    pnpm --filter agent-v-gui tauri build --debug

# Clean build artifacts
clean:
    cargo clean
    rm -rf apps/desktop/build
    rm -rf apps/desktop/src-tauri/target

# Utilities
# =========

# Update dependencies
update-deps:
    cargo update
    pnpm update

# Open database (macOS)
open-db:
    open ~/Library/Application\ Support/org.stormlightlabs.agent-v-gui/agent_v.db

# Tail logs (macOS)
tail-logs:
    tail -f ~/Library/Application\ Support/org.stormlightlabs.agent-v-gui/logs/*.log

# Generate TypeScript types from Rust (requires ts-rs setup)
gen-types:
    cargo test --workspace export_bindings
