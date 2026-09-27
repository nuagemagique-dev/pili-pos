# pili-pos

Desktop kiosk POS for Tokyo Sushi Bar.

`pili-pos` is a Tauri v2 application with a Rust backend and a Vue 3 touchscreen UI, designed for on-site restaurant operations and fiscal workflows.

## Stack

- Tauri v2
- Rust workspace (domain, fiscal, print, database, integrations)
- Vue 3 + TypeScript (Vite)
- SQLite local persistence
- ESC/POS printing

## Workspace layout

- `src-tauri/` - Tauri shell and command handlers
- `crates/pos-core` - core business domain
- `crates/pos-fiscal` - fiscal events, counters, QR/signature flows
- `crates/pos-print` - receipt and ticket printing
- `crates/pos-db` - SQLite schema and query layer
- `crates/pos-fdm-mock` - local FDM mock server
- `crates/pos-hubrise*` - HubRise integrations and mock tooling
- `ui/` - Vue app and Playwright tests
- `deploy/` - install, update, service, and kiosk hardening scripts

## Local setup

### Prerequisites

- Rust toolchain (edition 2024 workspace)
- `cargo-tauri`
- Node.js (for `ui/`)
- Linux build dependencies for Tauri/WebKitGTK when building native Linux binaries

### Environment

The app loads environment variables from `.env` at startup (if present).

HubRise-related variables used by the Tauri layer include:

- `HUBRISE_OAUTH_BASE_URL`
- `HUBRISE_BASE_URL`
- `HUBRISE_POS_CLIENT_ID`
- `HUBRISE_POS_CLIENT_SECRET`
- `HUBRISE_POS_REDIRECT_URI`
- `HUBRISE_POS_SCOPE` (optional)

### Run in development

```bash
# Install UI dependencies
cd ui && npm install

# Start desktop app in dev mode
cd ..
cargo tauri dev
```

## Build and test

```bash
# Rust tests
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all

# UI end-to-end tests
cd ui && npm run test:e2e
```

## Release packaging and install

Build a release package with bundled deployment artifacts:

```bash
./deploy/build-release.sh
# or from macOS via containerized Linux build
./deploy/build-release.sh --docker
```

Install on target terminal:

```bash
sudo ./deploy/install.sh --binary /path/to/pili-pos --seed /path/to/seed.sql
```

Update an installed terminal:

```bash
sudo ./deploy/update.sh /path/to/pili-pos [--seed /path/to/seed.sql]
```

## Operations

- Service unit: `deploy/pili-pos.service`
- Remote troubleshooting: `deploy/REMOTE-OPS.md`
- Update/rollback strategy: `deploy/UPDATE-STRATEGY.md`
- Kiosk lockdown files: `deploy/kiosk/`
