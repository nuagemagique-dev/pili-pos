# pili-pos

Point-of-sale system for Belgian restaurants, built for the SCE 2.0 certification (fiscal data module, signed events, fiscal tickets and reports).

One Rust fiscal engine (`crates/pos-engine`) runs behind two front ends:

| Product | Platform | UI | Status |
|---|---|---|---|
| `pili-pos-android` 1.0.0 | Android 13 (API 33), Sunmi D3 Pro, arm64 | Kotlin + Jetpack Compose, engine through UniFFI (`libpos_ffi.so`) | **Submitted for the v1 certification.** Dossier `docs/android/dossier`, freeze `android-v1.0.0-rc3` (`docs/android/ANDROID_FREEZE.md`) |
| `pili-pos` 1.0.0 | Linux x86_64 kiosk terminal (Debian minimal, Cage) | Tauri v2 + Vue 3 | Builds and passes its tests, **not submitted for v1**. What it still needs: `src-tauri/LINUX_RELEASE_GAP.md` |

Every fiscal and monetary rule lives in the engine; neither UI computes totals, counters or tickets. Parity scripts keep both UIs and their command layers in step (`scripts/android/check_command_parity.py`, `check_ui_parity.py`, `check_ipc_wrappers.py`).

## Features

- Sales, refunds, pro formas, invoices, payment corrections, copies and staff presence signed by the FDM; fiscal tickets, X, Z and user reports, training mode, automatic Z
- Tables and takeaway, table plan, table transfer (M122), payment correction (M132), discounts, kitchen notes and slips, courses
- Payments: cash, card through a linked terminal (Worldline, CCV), meal vouchers (chèques-repas), multi-purpose gift vouchers
- B2B invoices (event I, M150) with Peppol delivery
- Menu editor, sold-out items, the 14 EU allergens, sales reports
- Online orders from HubRise (switched per installation), digital tickets by email
- Staff badges (USB reader, or NFC on Android), owner alerts on Telegram, nightly encrypted export, accountant export
- UI in French, English or Chinese; tickets in French, Dutch or German, set per installation

## Repository layout

**Engine and shared crates** (`crates/`)

- `pos-engine`: the fiscal engine without any UI: commands, state, background tasks (automatic Z, mail outbox, HubRise poller, alerts, presence, idle lock, FDM status), events. Integration tests in `crates/pos-engine/tests`
- `pos-core`: business domain. `pos-fiscal`: fiscal events, counters, signing. `pos-db`: SQLite schema and queries. `pos-print`: ESC/POS tickets and slips
- `pos-catalog`: the menu as a mirror of an external catalog. `pos-hubrise`, `pos-hubrise-types`: HubRise client
- `pos-eft`: card terminal drivers. `pos-peppol`: Peppol e-invoicing. `pos-mail`: digital tickets. `pos-export`: nightly export
- `pos-accounting`: stand-alone accountant export, outside the certified build
- `pos-engine-testkit`: test harness of the engine tests
- Development mocks: `pos-fdm-mock`, `pos-printer-mock`, `pos-hubrise-mock`, `pos-eft-mock`, `pos-export-mock`; on-site card terminal check: `pos-eft-probe`

**Android** (`android/`)

- `app/`: the till app (Compose, device owner, Lock Task, declared-only updater)
- `pos-core/`: Android library with the Kotlin bindings and `libpos_ffi.so`
- `rust/`: separate Cargo workspace with `pos-ffi` (UniFFI facade over the engine) and `uniffi-bindgen`
- `smoke/`: test app that is never shipped; `uccheck/`: host replay of the official UC journals; `docker/`: reproducible build image

**Linux** (not submitted for v1)

- `src-tauri/`: thin Tauri shell; `src-tauri/src/ipc/` wraps each engine command in one call
- `ui/`: Vue 3 app and Playwright tests
- `deploy/`: release build, install, update, kiosk image and hardening

**Docs and scripts**

- `docs/android/`: Android build, decisions (`DECISIONS.md`), freeze records, UC matrix, dossier and its evidence
- `docs/SUBMISSION_ARTIFACTS/`: Linux dossier (kept, not submitted)
- `docs/` (dated folders): official SPF specifications and UC fixtures
- `scripts/android/`: gates (`p*_gate.sh`), emulator, captures, device tests, reproducibility; `scripts/`: dossier and evidence tools

`CLAUDE.md` holds the detailed architecture, SCE 2.0 rules, schema and decisions.

## Prerequisites

- Rust from `rust-toolchain.toml` (rustup first in `PATH`), `cargo-ndk`
- Android SDK with NDK 27.1.12297006, JDK 17
- Node.js for `ui/`; `cargo-tauri` and the WebKitGTK build dependencies for native Linux builds
- Docker for the reproducible builds

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/27.1.12297006"
export JAVA_HOME="$(/usr/libexec/java_home -v 17)"
```

## Build and test

```bash
# Engine and workspace
cargo test --workspace
cargo test -p pos-engine --no-default-features --features cert
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

# Android: FFI crate, then the app
(cd android/rust && cargo test --workspace --locked)
(cd android && ./gradlew :app:assembleDebug :app:testDebugUnitTest)

# Android gates and device runs (see docs/android/BUILD.md)
scripts/android/p6_gate.sh
scripts/android/emulator.sh ensure
scripts/android/device_engine_tests.sh --check

# Linux shell and UI
cargo test -p pili-pos --no-default-features --features cert
cd ui && npm ci && npm run test:e2e
cargo tauri dev
```

Development mocks: `cargo run -p pos-fdm-mock` (FDM and email API on :8081) and `cargo run -p pos-printer-mock` (printer on :9100). The Android dev bundle and screen captures use `scripts/android/make_dev_bundle.sh` and `scripts/android/capture_screens.sh`.

## Release and reproducibility

- **Android**: `scripts/android/check_repro.sh` builds the unsigned release APK twice in clean offline containers and compares them byte for byte; `scripts/android/check_android_frozen.sh` checks that the frozen inputs match the freeze tag. Signing happens outside the build with the release key (`docs/android/REPRO.md`).
- **Linux**: `./deploy/build-release.sh --docker` builds the release, `scripts/check_repro.sh` checks it twice. Install and update with `deploy/install.sh` and `deploy/update.sh`; operations in `deploy/REMOTE-OPS.md` and `deploy/UPDATE-STRATEGY.md`.

Live calls to the FDM simulator, HubRise or the card terminals are run by hand only, never in CI.

## Pili repositories

Every Pili repository lives in the GitHub org `nuagemagique-dev`. Local folders kept their earlier names.

| Repository | Local folder | What it is |
|---|---|---|
| pili-api | `tsb-service` | GraphQL API (Go) and the `pili-mcp` tool server |
| pili-shop | `tsb-core` | Customer web shop, one Nuxt app per brand |
| pili-dashboard | `tsb-dashboard` | Admin dashboard (Nuxt) |
| pili-mobile | `tsb-mobile` | Customer iOS and Android app (Expo) |
| pili-handheld | `moyun-handheld` | Staff handheld app for the Sunmi V3H (Kotlin) |
| **pili-pos** (this repository) | `moyun-pos` | SCE 2.0 cash register (Rust engine, Android and Linux) |
| pili-agent | `tsb-agent` | Owner and staff Telegram agent |
| pili-infra | `tsb-infra` | Helm charts, Terraform, SOPS secrets, runbooks |

## License

Proprietary (nuagemagique.dev).
