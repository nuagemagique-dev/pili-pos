#!/usr/bin/env bash
# The real certification binary against the module2 simulator, driven
# over WebDriver in a Linux container (WebKitGTK + Xvfb + tauri-driver,
# as the CI integration job), from macOS or Linux. By hand only: every
# action signs on module2.
#
#   scripts/live_app/run.sh            # build, start, launch the app
#   docker exec -w /work pili-live-app python3 wd.py click .user-btn 'Gerant Test'
#   docker exec -w /work pili-live-app python3 wd.py shot 01_login   # -> target/live-app/shots/
#   docker rm -f pili-live-app        # done
#
# Needs target/live-fdm/fdm.token (written by scripts/live_fdm.sh) and
# module2's certificates in target/live-fdm/mtls/. The printer is
# pos-printer-mock inside the container: jobs on http://127.0.0.1:9180/jobs.
# See docs/internal/LIVE_FDM_MODULE2.md, "The real app".
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HERE="$ROOT/scripts/live_app"
SHOTS="$ROOT/target/live-app/shots"
NAME=pili-live-app
mkdir -p "$SHOTS"

for f in fdm.token mtls/ca.crt mtls/client.crt mtls/client.key state.json; do
    [[ -f "$ROOT/target/live-fdm/$f" ]] || { echo "✗ target/live-fdm/$f is missing" >&2; exit 1; }
done

docker build -q -t "$NAME" "$HERE" >/dev/null
# The UI bundle the certification build embeds.
(cd "$ROOT/ui" && VITE_FEATURE_HUBRISE=false npm run build >/dev/null)

docker rm -f "$NAME" >/dev/null 2>&1 || true
docker run -d --name "$NAME" -p 127.0.0.1:9180:9180 \
    -v "$ROOT":/src:ro \
    -v pili-live-app-target:/cargo-target \
    -v pili-live-app-cargo:/usr/local/cargo/registry \
    -v "$HERE":/work:ro \
    -v "$SHOTS":/shots \
    -e CARGO_TARGET_DIR=/cargo-target \
    "$NAME" sleep infinity >/dev/null
docker exec -w /src/src-tauri "$NAME" \
    cargo build --release --no-default-features --features cert
docker exec -w /src "$NAME" cargo build --release -p pos-printer-mock
docker exec "$NAME" bash /work/start.sh
docker exec -w /work "$NAME" python3 wd.py start
echo "→ app running; drive it with: docker exec -w /work $NAME python3 wd.py ..."
