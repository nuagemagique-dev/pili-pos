#!/usr/bin/env bash
# After a session: keep the app's ticket numbers in
# target/live-fdm/state.json, so the live suite and the next session
# go on after them.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
docker exec pili-live-app sqlite3 -json /data/pos.db \
    "SELECT event_label, current_value FROM fiscal_counters WHERE event_label != 'I'" |
python3 -c "
import json, sys
path = '$ROOT/target/live-fdm/state.json'
state = json.load(open(path))
for row in json.load(sys.stdin):
    state[row['event_label']] = max(state.get(row['event_label'], 0), row['current_value'])
json.dump(state, open(path, 'w'), indent=2, sort_keys=True)
print(state)"
