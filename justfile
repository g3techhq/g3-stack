set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

# One-time setup after cloning.
setup:
    npm install
    lefthook install

# --- Run ------------------------------------------------------------------

# Web dev server on http://localhost:8080 (also runs the server build).
dev:
    dx serve

# Android emulator/device.
dev-android:
    dx serve --platform android

# iOS simulator (macOS only).
dev-ios:
    dx serve --platform ios

# Start SurrealDB in the background.
db-up:
    docker compose up -d db

db-down:
    docker compose down

# Stop SurrealDB and delete its data; the next `just dev` reapplies the schema.
[confirm("Delete all local database data?")]
db-reset:
    docker compose down
    node -e "require('fs').rmSync('data', { recursive: true, force: true })"

# Apply database/schema by hand (`dx serve` also does this in debug builds).
db-sync:
    surrealkit sync --fail-fast

# Load the demo data. Safe to re-run: every statement is an UPSERT.
db-seed:
    surrealkit seed

# --- Quality --------------------------------------------------------------

format:
    cargo fmt --all
    npm run format:web

format-check:
    cargo fmt --all -- --check
    npm run format:web:check

check: check-web check-server check-mobile

check-web:
    cargo check

check-server:
    cargo check --no-default-features --features server

check-mobile:
    cargo check --no-default-features --features mobile

lint:
    cargo clippy --all-targets --no-deps
    cargo clippy --all-targets --no-default-features --features server --no-deps
    npm run lint:web

lint-strict:
    cargo clippy --all-targets --no-deps -- -D warnings
    cargo clippy --all-targets --no-default-features --features server --no-deps -- -D warnings

test-rust:
    cargo test
    cargo test --no-default-features --features server

test-scripts:
    npm run test:scripts

test: test-rust test-scripts

# End-to-end browser tests (starts its own `dx serve` unless PLAYWRIGHT_BASE_URL is set).
test-ui:
    npm run test:ui

spell:
    typos

security:
    cargo deny check

# What the pre-push hook runs: fast, local-only.
pre-push: format-check check-web test-rust spell

# Everything CI runs.
quality: format-check check lint test spell

ci: quality security
