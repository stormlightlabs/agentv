# Show available commands
default:
    @just --list

# Run Tauri desktop app in dev mode
dev:
    pnpm --filter agent-v-gui tauri dev

# Run CLI with arguments
cli *ARGS:
    cargo run -p agent-v-cli -- {{ ARGS }}

# Build all Rust crates
build:
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
    cargo clippy --workspace -- -D warnings

# Format code
fmt:
    cargo fmt --all
    pnpm -r format

# Type check all packages
check:
    cargo check --workspace
    pnpm -r check

# Deploy Cloudflare Worker
deploy-worker:
    pnpm --filter agent-v-updater deploy

# Clean build artifacts
clean:
    cargo clean
    rm -rf apps/desktop/build
    rm -rf apps/desktop/src-tauri/target

# Update dependencies
update:
    cargo update
    pnpm update
