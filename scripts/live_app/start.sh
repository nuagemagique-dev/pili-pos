#!/usr/bin/env bash
# Inside the container (scripts/live_app/run.sh): seed the database once, start
# Xvfb, the printer mock and tauri-driver.
set -euo pipefail
export MOYUN_POS_DATA_DIR=/data MOYUN_POS_CONFIG=/work/pos.json DISPLAY=:99 GDK_BACKEND=x11
mkdir -p /data/backups /shots
if [[ ! -f /data/pos.db ]]; then
  /cargo-target/release/moyun-pos --init-db --seed /src/src-tauri/seed_spf_test.sql
  # Ticket numbers go on after the live suite's (target/live-fdm/state.json);
  # scripts/live_app/save_state.sh writes them back after the session.
  python3 - <<'PY'
import json, sqlite3
saved = json.load(open("/src/target/live-fdm/state.json"))
db = sqlite3.connect("/data/pos.db")
for label, value in saved.items():
    db.execute("INSERT INTO fiscal_counters (event_label, current_value) VALUES (?, ?) "
               "ON CONFLICT(event_label) DO UPDATE SET current_value = excluded.current_value",
               (label, value))
db.commit()
PY
fi
Xvfb :99 -screen 0 1920x1080x24 >/tmp/xvfb.log 2>&1 &
/cargo-target/release/pos-printer-mock >/tmp/printer.log 2>&1 &
tauri-driver --port 4444 >/tmp/tauri-driver.log 2>&1 &
for i in $(seq 1 30); do curl -sf http://127.0.0.1:4444/status >/dev/null && break; sleep 1; done
echo ready
