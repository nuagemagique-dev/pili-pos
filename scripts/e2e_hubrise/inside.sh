#!/usr/bin/env bash
# Inside the container (scripts/e2e_hubrise.sh): a fresh database, the
# three mocks, Xvfb and tauri-driver, then the suite.
set -euo pipefail
T=/cargo-target/release
export PILI_POS_DATA_DIR=/tmp/pili-hubrise PILI_POS_CONFIG=/work/pos.json
export DISPLAY=:99 GDK_BACKEND=x11
rm -rf "$PILI_POS_DATA_DIR" /logs/app && mkdir -p "$PILI_POS_DATA_DIR" /logs
# The app's database and its own log, kept for a look after the run.
trap 'cp -r "$PILI_POS_DATA_DIR" /logs/app' EXIT
"$T/pili-pos" --init-db --seed /src/ui/e2e/fixtures/seed_hubrise.sql

"$T/pos-hubrise-mock" >/logs/hubrise-mock.log 2>&1 &
"$T/pos-fdm-mock" >/logs/fdm-mock.log 2>&1 &
"$T/pos-printer-mock" >/logs/printer-mock.log 2>&1 &
Xvfb :99 -screen 0 1920x1080x24 >/logs/xvfb.log 2>&1 &
tauri-driver --port 4444 >/logs/tauri-driver.log 2>&1 &
for _ in $(seq 1 30); do
    curl -sf http://127.0.0.1:4444/status >/dev/null &&
        curl -sf http://127.0.0.1:3001/ >/dev/null &&
        curl -sf http://127.0.0.1:8081/mock/control >/dev/null &&
        curl -sf http://127.0.0.1:9180/jobs >/dev/null && break
    sleep 1
done

cd /src/ui
TAURI_DRIVER_URL=http://127.0.0.1:4444 PILI_POS_BINARY="$T/pili-pos" \
    HUBRISE_MOCK_URL=http://127.0.0.1:3001 PRINTER_JOBS_URL=http://127.0.0.1:9180 \
    PILI_POS_DB="$PILI_POS_DATA_DIR/pos.db" \
    node --test --test-reporter=spec e2e/real-binary/hubrise.test.mjs
