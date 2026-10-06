#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - capture the FDM request journal of the HubRise e2e flow
#
# Runs the same flow as scripts/e2e_hubrise.sh (the real cert binary vs
# pos-hubrise-mock, pos-fdm-mock and pos-printer-mock, driven over
# WebDriver in a Linux container) and saves what pos-fdm-mock logged
# (`GET /mock/requests`: operation, request and response of every call)
# before the container is torn down. Nothing leaves the machine.
#
# scripts/e2e_hubrise.sh and scripts/e2e_hubrise/* are NOT edited: the
# container gets its own copy of that directory in a temp dir, where
# inside.sh's EXIT trap also dumps the journal.
#
# Output, in --out-dir (default docs/android/baseline):
#   fdm_journal_<name>.json             raw journal, as the mock served it
#   fdm_journal_<name>.normalised.json  volatile fields removed, keys
#                                       sorted (jq -S), see NORMALISE below
#
# Usage: scripts/android/capture_fdm_journals.sh [--name NAME] [--out-dir DIR]
#                                                [--work-dir DIR]
# Own docker volumes (pili-e2e-p0-*) and container, so a concurrent
# scripts/e2e_hubrise.sh run is untouched. Needs docker, jq, node/npm.
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HERE="$ROOT/scripts/e2e_hubrise"

name=hubrise_e2e
out_dir="$ROOT/docs/android/baseline"
work_dir="${PILI_ANDROID_WORK_DIR:-$HOME/.cache/pili-android/P0.1}/e2e"

while [[ $# -gt 0 ]]; do
    case "$1" in
    --name)
        name="${2:?--name needs a value}"
        shift
        ;;
    --out-dir)
        out_dir="${2:?--out-dir needs a directory}"
        shift
        ;;
    --work-dir)
        work_dir="${2:?--work-dir needs a directory}"
        shift
        ;;
    -h | --help)
        sed -n '2,/^set -euo/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
        exit 0
        ;;
    *)
        echo "unknown argument: $1" >&2
        exit 2
        ;;
    esac
    shift
done

command -v jq >/dev/null || { echo "jq is required" >&2; exit 2; }
command -v docker >/dev/null || { echo "docker is required" >&2; exit 2; }

# What differs between two runs of the same flow, or is the mock's own
# bookkeeping rather than a request/response contract:
#   .at                          wall-clock time the mock logged the call
#   posDateTime, fdmDateTime     timestamps (any key ending in DateTime)
#   uuid-shaped strings          -> "<uuid>"
#   eventCounter, totalCounter   the FDM's running counters
#   digitalSignature, shortSignature, verificationUrl
#                                derived from the timestamps and counters
# A quoted string that is an RFC 3339 / ISO date-time is also replaced
# by "<datetime>" and a bare YYYY-MM-DD (bookingDate) by "<date>", so a
# value tied to the day of the run cannot make two runs differ.
NORMALISE='
def scrub:
  walk(
    if type == "object" then
      with_entries(select(
        (.key | test("DateTime$") | not)
        and (.key != "at")
        and (.key != "eventCounter")
        and (.key != "totalCounter")
        and (.key != "digitalSignature")
        and (.key != "shortSignature")
        and (.key != "verificationUrl")
      ))
    elif type == "string" then
      if test("^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$") then "<uuid>"
      elif test("^[0-9]{4}-[0-9]{2}-[0-9]{2}[T ][0-9]{2}:[0-9]{2}:[0-9]{2}") then "<datetime>"
      elif test("^[0-9]{4}-[0-9]{2}-[0-9]{2}$") then "<date>"
      else . end
    else . end
  );
scrub
'

mkdir -p "$out_dir"
rm -rf "$work_dir/ctx" "$work_dir/logs"
mkdir -p "$work_dir/ctx" "$work_dir/logs"

# Own copy of the e2e directory; inside.sh gets the journal dump.
cp -R "$HERE"/. "$work_dir/ctx/"
dump='curl -s http://127.0.0.1:8081/mock/requests > /logs/fdm_requests.json 2>/dev/null; '
sed "s|^trap 'cp -r|trap '${dump}cp -r|" "$HERE/inside.sh" >"$work_dir/ctx/inside.sh"
grep -q 'fdm_requests.json' "$work_dir/ctx/inside.sh" || {
    echo "x could not inject the journal dump into the inside.sh copy (its trap line changed?)" >&2
    exit 1
}

IMAGE=pili-e2e-hubrise
CONTAINER=pili-e2e-p0-hubrise
VOL_TARGET=pili-e2e-p0-target
VOL_CARGO=pili-e2e-p0-cargo

docker build -q -t "$IMAGE" "$work_dir/ctx" >/dev/null
# The UI bundle the certification build embeds.
(cd "$ROOT/ui" && npm run build >/dev/null)
[[ -f "$ROOT/ui/dist/index.html" ]] || { echo "x ui/dist is missing after the UI build" >&2; exit 1; }

docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
trap 'docker rm -f "$CONTAINER" >/dev/null 2>&1 || true' EXIT
docker run -d --name "$CONTAINER" \
    -v "$ROOT":/src:ro \
    -v "$VOL_TARGET":/cargo-target \
    -v "$VOL_CARGO":/usr/local/cargo/registry \
    -v "$work_dir/ctx":/work:ro \
    -v "$work_dir/logs":/logs \
    -e CARGO_TARGET_DIR=/cargo-target -e CARGO_INCREMENTAL=0 \
    "$IMAGE" sleep infinity >/dev/null
docker exec -w /src/src-tauri "$CONTAINER" \
    cargo build --release --no-default-features --features cert
docker exec -w /src "$CONTAINER" \
    cargo build --release -p pos-hubrise-mock -p pos-fdm-mock -p pos-printer-mock

status=0
docker exec "$CONTAINER" bash /work/inside.sh | tee "$work_dir/e2e.log" || status=$?
if [[ $status -ne 0 ]]; then
    echo "x the e2e suite failed (exit $status): no journal saved. Log: $work_dir/e2e.log" >&2
    exit "$status"
fi
[[ -s "$work_dir/logs/fdm_requests.json" ]] || { echo "x no FDM journal was dumped" >&2; exit 1; }
jq -e 'type == "array"' "$work_dir/logs/fdm_requests.json" >/dev/null || { echo "x the journal is not a JSON array" >&2; exit 1; }

raw="$out_dir/fdm_journal_$name.json"
norm="$out_dir/fdm_journal_$name.normalised.json"
jq . "$work_dir/logs/fdm_requests.json" >"$raw"
jq -S "$NORMALISE" "$raw" >"$norm"

calls="$(jq length "$raw")"
echo "ok $calls FDM calls journaled"
if [[ "$calls" -ge 100 ]]; then
    echo "! pos-fdm-mock keeps only its last 100 calls (LOG_SIZE): the journal may be truncated" >&2
fi
sha256sum "$raw" "$norm" 2>/dev/null || shasum -a 256 "$raw" "$norm"
