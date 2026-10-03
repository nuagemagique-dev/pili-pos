#!/usr/bin/env bash
# Online orders end to end, offline: the real certification binary
# (HubRise switched on in pos.json) against pos-hubrise-mock,
# pos-fdm-mock and pos-printer-mock, driven over WebDriver in a Linux
# container (WebKitGTK + Xvfb + tauri-driver), from macOS or Linux.
# Nothing leaves the machine: no HubRise order, no FDM signature.
#
#   scripts/e2e_hubrise.sh          # build, run ui/e2e/real-binary/hubrise.test.mjs
#
# Logs of the mocks, the driver and the app: target/e2e-hubrise/logs/.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HERE="$ROOT/scripts/e2e_hubrise"
LOGS="$ROOT/target/e2e-hubrise/logs"
NAME=pili-e2e-hubrise
mkdir -p "$LOGS"

docker build -q -t "$NAME" "$HERE" >/dev/null
# The UI bundle the certification build embeds.
(cd "$ROOT/ui" && npm run build >/dev/null)

docker rm -f "$NAME" >/dev/null 2>&1 || true
trap 'docker rm -f "$NAME" >/dev/null 2>&1 || true' EXIT
docker run -d --name "$NAME" \
    -v "$ROOT":/src:ro \
    -v pili-e2e-target:/cargo-target \
    -v pili-e2e-cargo:/usr/local/cargo/registry \
    -v "$HERE":/work:ro \
    -v "$LOGS":/logs \
    -e CARGO_TARGET_DIR=/cargo-target -e CARGO_INCREMENTAL=0 \
    "$NAME" sleep infinity >/dev/null
docker exec -w /src/src-tauri "$NAME" \
    cargo build --release --no-default-features --features cert
docker exec -w /src "$NAME" \
    cargo build --release -p pos-hubrise-mock -p pos-fdm-mock -p pos-printer-mock
docker exec "$NAME" bash /work/inside.sh
