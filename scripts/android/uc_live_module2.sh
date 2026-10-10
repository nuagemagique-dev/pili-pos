#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - UC scenarios of the Android till against the module2 online FDM simulator (P7D13)
#
# Usage: scripts/android/uc_live_module2.sh --help
#        scripts/android/uc_live_module2.sh --dry-run [--group G]... [--max-requests N] [--env-file PATH] [--out DIR]
#        PILI_LIVE_GO_AHEAD=yes scripts/android/uc_live_module2.sh --go-ahead [--group G]... [--max-requests N]
#                                                                 [--pause-ms N] [--env-file PATH] [--out DIR]
#   --help          print this text and exit 0.
#   --dry-run       print the plan (groups, scenarios, request estimate, files) and exit 0. It reads no
#                   environment file, touches no device, no network and no file.
#   --go-ahead      with the environment variable PILI_LIVE_GO_AHEAD=yes, the only way to a real run. Without
#                   BOTH the script refuses (exit 3) and does nothing. The chain never passes them: the user does,
#                   after deciding the three prerequisites of docs/android/LIVE_MODULE2.md.
#   --group G       scenario group to play (repeatable). Default: 00_smoke and 10_basic. 60_fdm_errors is refused
#                   (its scenarios flip the simulated errors, which only the Developer tab of the simulator can do).
#   --max-requests N  cap on the requests forwarded to the simulator, status calls included (default
#                   $PILI_LIVE_MAX_REQUESTS, else 60, like scripts/live_fdm.sh). Past the cap nothing is forwarded.
#   --pause-ms N    minimum pause between two forwarded requests (default $PILI_LIVE_PAUSE_MS, else 1000).
#   --env-file PATH file that holds PILI_LIVE_FDM_URL and PILI_LIVE_FDM_TOKEN (default .env.live at the
#                   repository root, git-ignored). Read at run time only, by the proxy below, never by --dry-run.
#   --out DIR       evidence directory (default target/android-uc-live/<date>-<time>, inside target/).
#
# What a real run does (the same device run as uc_e2e.sh, with the simulator in place of pos-fdm-mock):
#   1. checks the environment file ('token: set' or 'token: none', never the value) and the URL;
#   2. takes the P7 emulator lock (it waits while the soak runs: stop it first, docs/android/LIVE_MODULE2.md);
#   3. puts a live proxy where uc_e2e.sh starts pos-fdm-mock: a small Python server on the host port of the mock
#      that forwards every GraphQL call to the simulator one at a time (pause between calls), adds the bearer
#      token ITSELF, counts the requests and stops forwarding at the cap, and answers /mock/requests like the
#      mock does so that uc_e2e.sh builds its journals. The token therefore lives only in the memory of the proxy:
#      it is not in the provisioning bundle, not on the device, not in a log, not in a committed or temporary
#      file. The real pos-fdm-mock binary is put back when the run ends (also after an error or a signal);
#   4. runs uc_e2e.sh on the groups (it builds its bundle in a temporary directory outside the repository);
#   5. records the reference (pos-uc-check record: the engine on the host, against its own mock) and compares:
#      pos-uc-check compare --no-counters --fixtures (business content only, the simulator's counters are not
#      ours); 10_basic/140_Q110 is a documented, expected FAIL (module2 answers its multi-language Status query
#      with HTTP 500 and the simulated warning cannot be switched from here): it is shown, not counted;
#   6. deletes its temporary directory and prints one line per check and
#          ULIVE | total | PASS|FAIL | <n> failed (<m> checks)
#
# Nothing here is run from CI, by the chain, or without the user (P7D13).
# Exit codes: 0 ok, 1 a check failed, 2 usage error, 3 refused (no go-ahead), 4 environment file or token missing,
# 75 the emulator lock is busy.
#
# Test hooks (scripts/android/test_uc_live_module2.sh): PILI_LIVE_UC_E2E, PILI_LIVE_UCCHECK_BIN,
# PILI_LIVE_MOCKS_DIR, PILI_LIVE_ASSETS, PILI_LIVE_JOURNALS.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

SELF="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)/$(basename "${BASH_SOURCE[0]}")"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
ASSETS="${PILI_LIVE_ASSETS:-$ROOT/android/app/src/androidTest/assets/uc}"
JOURNALS="${PILI_LIVE_JOURNALS:-$ROOT/docs/android/uc/journals/android}"
UC_E2E="${PILI_LIVE_UC_E2E:-$ROOT/scripts/android/uc_e2e.sh}"
UCCHECK_BIN="${PILI_LIVE_UCCHECK_BIN:-$ROOT/target/uccheck/release/pos-uc-check}"
MOCKS="${PILI_LIVE_MOCKS_DIR:-$ROOT/target/android-mocks/debug}"
DEFAULT_GROUPS=(00_smoke 10_basic)
REFUSED_GROUPS=(60_fdm_errors)
EXPECTED_FAIL=(10_basic/140_Q110)

usage_text() { sed -n '2,/^# ──*$/p' "$SELF" | sed '$d; s/^# \{0,1\}//'; }
usage() {
    echo "usage: uc_live_module2.sh --help | --dry-run [options] | --go-ahead [options] (with PILI_LIVE_GO_AHEAD=yes)" >&2
    exit 2
}

ORIG_ARGS=("$@")
DRY=0
GO=0
ENV_FILE="$ROOT/.env.live"
OUT=""
MAX_REQ_ARG=""
PAUSE_ARG=""
WANT_GROUPS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
    -h | --help) usage_text; exit 0 ;;
    --dry-run) DRY=1; shift ;;
    --go-ahead) GO=1; shift ;;
    --group) [[ $# -ge 2 && -n "$2" ]] || usage; WANT_GROUPS+=("$2"); shift 2 ;;
    --max-requests) [[ $# -ge 2 && -n "$2" ]] || usage; MAX_REQ_ARG="$2"; shift 2 ;;
    --pause-ms) [[ $# -ge 2 && -n "$2" ]] || usage; PAUSE_ARG="$2"; shift 2 ;;
    --env-file) [[ $# -ge 2 && -n "$2" ]] || usage; ENV_FILE="$2"; shift 2 ;;
    --out) [[ $# -ge 2 && -n "$2" ]] || usage; OUT="$2"; shift 2 ;;
    *) usage ;;
    esac
done

MAX_REQ="${MAX_REQ_ARG:-${PILI_LIVE_MAX_REQUESTS:-60}}"
PAUSE_MS="${PAUSE_ARG:-${PILI_LIVE_PAUSE_MS:-1000}}"
[[ "$MAX_REQ" =~ ^[0-9]+$ && "$MAX_REQ" -ge 1 && "$MAX_REQ" -le 1000 ]] || { echo "ULIVE | usage | FAIL | --max-requests must be a number from 1 to 1000" >&2; exit 2; }
[[ "$PAUSE_MS" =~ ^[0-9]+$ && "$PAUSE_MS" -le 60000 ]] || { echo "ULIVE | usage | FAIL | --pause-ms must be a number from 0 to 60000" >&2; exit 2; }
if [[ "${#WANT_GROUPS[@]}" -eq 0 ]]; then
    GROUP_LIST=("${DEFAULT_GROUPS[@]}")
else
    GROUP_LIST=("${WANT_GROUPS[@]}")
fi
for g in "${GROUP_LIST[@]}"; do
    [[ "$g" =~ ^[0-9]{2}_[a-z_]+$ ]] || { echo "ULIVE | usage | FAIL | bad group name '$g'" >&2; exit 2; }
    for r in "${REFUSED_GROUPS[@]}"; do
        if [[ "$g" == "$r" ]]; then
            echo "ULIVE | usage | FAIL | group $g cannot run against the simulator (its scenarios switch simulated errors on and off, which only the Developer tab of the simulator does; see docs/android/LIVE_MODULE2.md)" >&2
            exit 2
        fi
    done
done

# ── the number of scenarios and the estimate of requests, from the committed Android journals ──
# prints '<scenarios> <fdm calls> <status calls>' for the groups; 'unknown' when a journal is missing
estimate() {
    python3 -B -I - "$JOURNALS" "$ASSETS" "$@" <<'PY'
import glob, json, os, sys
journals, assets, groups = sys.argv[1], sys.argv[2], sys.argv[3:]
scenarios = fdm = status = 0
for g in groups:
    names = sorted(glob.glob(os.path.join(assets, g, "*.json")))
    if not names:
        print("missing-group", g)
        sys.exit(0)
    for path in names:
        scenarios += 1
        journal = os.path.join(journals, g, os.path.basename(path))
        try:
            with open(journal, encoding="utf-8") as handle:
                j = json.load(handle)
            fdm += len(j.get("fdm", []))
            status += len(j.get("statusCalls", []))
        except (OSError, ValueError):
            print("unknown", scenarios)
            sys.exit(0)
print(scenarios, fdm, status)
PY
}

EST="$(estimate "${GROUP_LIST[@]}")"
case "$EST" in
missing-group*) echo "ULIVE | usage | FAIL | no scenario files for group ${EST#missing-group }" >&2; exit 2 ;;
esac

if [[ "$EST" == unknown* ]]; then
    EST_TOTAL=0
    EST_TEXT="unknown (a committed journal is missing)"
    SCEN_COUNT="${EST#unknown }"
else
    read -r SCEN_COUNT EST_FDM EST_STATUS <<<"$EST"
    EST_TOTAL=$((EST_FDM + EST_STATUS))
    EST_TEXT="$EST_TOTAL requests ($EST_FDM signing calls and $EST_STATUS status calls, from the committed journals; the status monitor of the app adds a few)"
fi

# ── --dry-run: the plan only ────────────────────────────────────
if [[ "$DRY" -eq 1 ]]; then
    printf 'ULIVE | plan | INFO | groups: %s\n' "${GROUP_LIST[*]}"
    printf 'ULIVE | plan | INFO | scenarios: %s (expected live FAIL, shown and not counted: %s)\n' "$SCEN_COUNT" "${EXPECTED_FAIL[*]}"
    printf 'ULIVE | plan | INFO | request estimate: %s; cap %s; pause %s ms\n' "$EST_TEXT" "$MAX_REQ" "$PAUSE_MS"
    if [[ "$EST_TOTAL" -gt "$MAX_REQ" ]]; then
        printf 'ULIVE | plan | WARN | the estimate is above the cap: a real run would be refused, raise --max-requests or play fewer groups\n'
    elif [[ "$EST_TOTAL" -gt 0 && $((EST_TOTAL * 100)) -gt $((MAX_REQ * 80)) ]]; then
        printf 'ULIVE | plan | WARN | the estimate uses more than 80 percent of the cap: leave some room for the status monitor\n'
    fi
    printf 'ULIVE | plan | INFO | environment file: %s (not read by --dry-run; a real run needs PILI_LIVE_FDM_URL and PILI_LIVE_FDM_TOKEN in it)\n' "$ENV_FILE"
    printf 'ULIVE | plan | INFO | token: not read\n'
    printf 'ULIVE | plan | INFO | evidence: %s\n' "${OUT:-$ROOT/target/android-uc-live/<date>-<time>}"
    printf 'ULIVE | plan | INFO | temporary directory: a mktemp directory outside the repository, deleted at the end (live proxy, copy of the control files)\n'
    printf 'ULIVE | plan | INFO | commands: %s (device), pos-uc-check record then compare --no-counters --fixtures (host)\n' "$UC_E2E"
    printf 'ULIVE | dry-run | PASS | nothing was run, no file was read or written\n'
    exit 0
fi

# ── the two keys of the go-ahead ────────────────────────────────
if [[ "$GO" -ne 1 || "${PILI_LIVE_GO_AHEAD:-}" != "yes" ]]; then
    printf 'ULIVE | refused | FAIL | a real run needs both --go-ahead and PILI_LIVE_GO_AHEAD=yes; nothing was done (see --help and docs/android/LIVE_MODULE2.md)\n' >&2
    exit 3
fi

# ── from here on: a real run, started by the user ───────────────
# Reads the environment file the safe way (never sourced, never executed) and prints url, token state and
# simulator version, never the token.
read_env_summary() {
    python3 -B -I - "$1" <<'PY'
import re, sys
want = {"PILI_LIVE_FDM_URL": "", "PILI_LIVE_FDM_TOKEN": "", "PILI_LIVE_SIMULATOR_VERSION": ""}
try:
    with open(sys.argv[1], encoding="utf-8") as handle:
        lines = handle.read().splitlines()
except OSError:
    sys.exit(3)
for line in lines:
    line = line.strip()
    if not line or line.startswith("#"):
        continue
    if line.startswith("export "):
        line = line[7:].strip()
    key, sep, value = line.partition("=")
    key = key.strip()
    if sep and key in want:
        value = value.strip()
        if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
            value = value[1:-1]
        want[key] = value
print("url=" + want["PILI_LIVE_FDM_URL"])
print("token=" + ("set" if want["PILI_LIVE_FDM_TOKEN"] else "none"))
print("simulator=" + want["PILI_LIVE_SIMULATOR_VERSION"])
PY
}

fail_early() { # <code> <text>
    printf 'ULIVE | %s | FAIL | %s\n' "$2" "$3" >&2
    exit "$1"
}

[[ -f "$ENV_FILE" ]] || fail_early 4 env "environment file $ENV_FILE not found (PILI_LIVE_FDM_URL and PILI_LIVE_FDM_TOKEN)"
SUMMARY="$(read_env_summary "$ENV_FILE")" || fail_early 4 env "environment file $ENV_FILE cannot be read"
LIVE_URL="$(sed -n 's/^url=//p' <<<"$SUMMARY")"
TOKEN_STATE="$(sed -n 's/^token=//p' <<<"$SUMMARY")"
SIM_VERSION="$(sed -n 's/^simulator=//p' <<<"$SUMMARY")"
printf 'ULIVE | env | INFO | live FDM: %s\n' "${LIVE_URL:-none}"
printf 'ULIVE | env | INFO | token: %s\n' "$TOKEN_STATE"
[[ -n "$LIVE_URL" ]] || fail_early 4 env "PILI_LIVE_FDM_URL is not set in $ENV_FILE"
[[ "$LIVE_URL" =~ ^https?://[A-Za-z0-9._-]+(:[0-9]+)?(/[A-Za-z0-9._~/%?=\&-]*)?$ ]] || fail_early 4 env "PILI_LIVE_FDM_URL is not a plain http(s) URL"
[[ "$TOKEN_STATE" == "set" ]] || fail_early 4 env "PILI_LIVE_FDM_TOKEN is not set in $ENV_FILE (token: none)"
[[ -z "$SIM_VERSION" || "$SIM_VERSION" =~ ^[0-9.]+$ ]] || fail_early 4 env "PILI_LIVE_SIMULATOR_VERSION must look like 2.2.0.5"
if [[ "$EST_TOTAL" -gt "$MAX_REQ" ]]; then
    fail_early 1 budget "the estimate of $EST_TOTAL requests is above the cap of $MAX_REQ: raise --max-requests (or PILI_LIVE_MAX_REQUESTS) or play fewer groups"
fi

STAMP="$(date +%Y%m%d-%H%M%S)"
EVID="${OUT:-$ROOT/target/android-uc-live/$STAMP}"
case "$EVID" in
/*) ;;
*) EVID="$PWD/$EVID" ;;
esac
case "$EVID" in
"$ROOT"/target/*) ;;
"$ROOT"/*) fail_early 2 usage "--out must not lie inside the repository except under target/" ;;
esac

# ── the P7 emulator lock (before the first side effect and before our own traps) ──
LOCKLIB="$ROOT/scripts/android/p7_emulator_lock.sh"
[[ -f "$LOCKLIB" ]] || fail_early 1 lock "$LOCKLIB is absent: the P7 emulator lock is needed to share the emulator"
# shellcheck source=/dev/null
source "$LOCKLIB"
p7_emulator_lock uc_live "$SELF" ${ORIG_ARGS[@]+"${ORIG_ARGS[@]}"}
p7_emulator_lock_note uc_live

TMPBASE="${TMPDIR:-/tmp}"
TMP="$(mktemp -d "${TMPBASE%/}/pili-live.XXXXXX")" || fail_early 1 tmp "mktemp failed"
case "$TMP" in
"$ROOT"/*) rm -rf "$TMP"; fail_early 1 tmp "the temporary directory would lie inside the repository" ;;
esac
mkdir -p "$EVID"
STATE="$TMP/state"
mkdir -p "$STATE"
: >"$STATE/count"
echo 0 >"$STATE/count"

NFAIL=0
NCHECK=0
SWAPPED=0
pass() { NCHECK=$((NCHECK + 1)); printf 'ULIVE | %s | PASS | %s\n' "$1" "$2"; }
fail() { NCHECK=$((NCHECK + 1)); NFAIL=$((NFAIL + 1)); printf 'ULIVE | %s | FAIL | %s\n' "$1" "$2"; }

swap_out() {
    if [[ "$SWAPPED" -eq 1 ]]; then
        rm -f "$MOCKS/pos-fdm-mock"
        mv -f "$MOCKS/pos-fdm-mock.live-real" "$MOCKS/pos-fdm-mock" 2>/dev/null
        SWAPPED=0
    fi
}
finish() {
    local rc=$?
    swap_out
    rm -rf "$TMP"
    p7_emulator_lock_cleanup 2>/dev/null || true
    if [[ "${TOTAL_PRINTED:-0}" -ne 1 ]]; then
        # an early abort: say so, with a total line
        NFAIL=$((NFAIL + 1))
        NCHECK=$((NCHECK + 1))
        printf 'ULIVE | total | FAIL | %s failed (%s checks) - aborted\n' "$NFAIL" "$NCHECK"
        [[ "$rc" -ne 0 ]] || rc=1
    fi
    exit "$rc"
}
trap finish EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

pass env "token: set, live FDM ${LIVE_URL}, cap $MAX_REQ requests, pause $PAUSE_MS ms, groups ${GROUP_LIST[*]}"

# ── the live proxy, written next to nothing that is committed ──
cat >"$TMP/live_proxy.py" <<'PY'
"""Host-side live proxy that stands where uc_e2e.sh starts pos-fdm-mock (P7D13). Standard library only.

Forwards POST /graphql one at a time to PILI_LIVE_FDM_URL with the bearer token it reads itself from the
environment file, counts the requests in STATE/count, stops forwarding at the cap and answers the three routes
of the mock that uc_e2e.sh uses (GET /graphql as a liveness probe, GET and DELETE /mock/requests, POST
/mock/control, which is acknowledged and ignored). The token is never printed, logged or written.
"""
import json
import os
import sys
import threading
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

WANT = ("PILI_LIVE_FDM_URL", "PILI_LIVE_FDM_TOKEN", "PILI_LIVE_SIMULATOR_VERSION")
MUTATIONS = (
    "signSale", "signOrder", "signPreBill", "signWorkIn", "signWorkOut", "signCopy", "signInvoice",
    "signMoneyInOut", "signDrawerOpen", "signPaymentCorrection", "signCostCenterChange",
    "signReportTurnoverX", "signReportTurnoverZ", "signReportUserX", "signReportUserZ", "Status",
)


def read_env(path):
    values = dict.fromkeys(WANT, "")
    with open(path, encoding="utf-8") as handle:
        for line in handle.read().splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if line.startswith("export "):
                line = line[7:].strip()
            key, sep, value = line.partition("=")
            key = key.strip()
            if sep and key in values:
                value = value.strip()
                if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
                    value = value[1:-1]
                values[key] = value
    return values


ENV = read_env(os.environ["PILI_LIVE_ENV_FILE"])
UPSTREAM = ENV["PILI_LIVE_FDM_URL"]
TOKEN = ENV["PILI_LIVE_FDM_TOKEN"]
SIMULATOR = ENV["PILI_LIVE_SIMULATOR_VERSION"]
CAP = int(os.environ.get("PILI_LIVE_MAX_REQUESTS", "60"))
PAUSE = int(os.environ.get("PILI_LIVE_PAUSE_MS", "1000")) / 1000.0
STATE = os.environ["PILI_LIVE_STATE_DIR"]
PORT = int(os.environ["PORT"])
LOCK = threading.Lock()
LOG = []  # the last 100 calls, as pos-fdm-mock lists them
LAST = [0.0]


def operation_of(query):
    if "status(" in query:
        return "Status"
    for name in MUTATIONS:
        if name in query:
            return name
    return "unknown"


def read_count():
    try:
        with open(os.path.join(STATE, "count"), encoding="utf-8") as handle:
            return int(handle.read().strip() or "0")
    except (OSError, ValueError):
        return 0


def bump(name, value=None):
    path = os.path.join(STATE, name)
    if value is None:
        value = 0
        try:
            with open(path, encoding="utf-8") as handle:
                value = int(handle.read().strip() or "0")
        except (OSError, ValueError):
            value = 0
        value += 1
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(str(value) + "\n")


def note_forward(count, operation, status):
    stamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    with open(os.path.join(STATE, "forwarded.log"), "a", encoding="utf-8") as handle:
        handle.write("%s\t%d\t%s\t%s\n" % (stamp, count, operation, status))


def scrub(response):
    """What the log keeps: the answer without the visitor address, like the Linux live proxy."""
    if isinstance(response, dict) and isinstance(response.get("info"), dict):
        response = dict(response)
        info = dict(response["info"])
        info.pop("visitor_ip", None)
        response["info"] = info
    return response


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args):  # quiet: the run output must stay small
        pass

    def reply(self, status, payload):
        body = payload if isinstance(payload, bytes) else json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def body(self):
        length = int(self.headers.get("Content-Length") or "0")
        return self.rfile.read(length) if length else b""

    def do_GET(self):
        path = self.path.split("?", 1)[0]
        if path == "/mock/requests":
            with LOCK:
                self.reply(200, list(LOG))
        elif path == "/graphql":
            self.reply(200, {"live": True, "note": "live proxy of the module2 simulator, POST to use"})
        elif path == "/mock/control":
            self.reply(200, {"live": True, "note": "mock control is ignored by the live proxy"})
        else:
            self.reply(404, {"error": "not found"})

    def do_DELETE(self):
        if self.path.split("?", 1)[0] == "/mock/requests":
            with LOCK:
                del LOG[:]
            self.reply(200, {"ok": True})
        else:
            self.reply(404, {"error": "not found"})

    def do_POST(self):
        path = self.path.split("?", 1)[0]
        raw = self.body()
        if path == "/mock/control":
            bump("control_ignored")
            self.reply(200, {"ok": True, "ignored": True})
            return
        if path != "/graphql":
            self.reply(404, {"error": "not found"})
            return
        try:
            request = json.loads(raw.decode("utf-8"))
            query = request.get("query", "") if isinstance(request, dict) else ""
        except ValueError:
            request, query = {}, ""
        operation = operation_of(query if isinstance(query, str) else "")
        with LOCK:
            count = read_count()
            if count >= CAP:
                bump("blocked")
                answer = {"errors": [{"message": "live request cap reached (%d), nothing was forwarded" % CAP, "code": "LIVE_CAP_REACHED"}]}
                self.reply(429, answer)
                return
            wait = PAUSE - (time.monotonic() - LAST[0])
            if wait > 0:
                time.sleep(wait)
            with open(os.path.join(STATE, "count"), "w", encoding="utf-8") as handle:
                handle.write(str(count + 1) + "\n")
            headers = {"Content-Type": "application/json", "Authorization": "Bearer " + TOKEN}
            if SIMULATOR:
                headers["Simulator-Version"] = SIMULATOR
            call = urllib.request.Request(UPSTREAM, data=raw, headers=headers, method="POST")
            status = 502
            payload = b""
            try:
                with urllib.request.urlopen(call, timeout=30) as upstream:
                    status = upstream.status
                    payload = upstream.read()
            except urllib.error.HTTPError as err:
                status = err.code
                payload = err.read()
            except Exception as err:  # noqa: BLE001 - the app must get an answer, whatever happened
                payload = json.dumps({"errors": [{"message": "live proxy: upstream unreachable (%s)" % type(err).__name__}]}).encode("utf-8")
            LAST[0] = time.monotonic()
            note_forward(count + 1, operation, status)
            try:
                parsed = json.loads(payload.decode("utf-8")) if payload else {}
            except ValueError:
                parsed = {"raw": payload.decode("utf-8", "replace")[:500]}
            if len(LOG) >= 100:
                del LOG[0]
            LOG.append({
                "at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%fZ"),
                "operation": operation,
                "request": request,
                "response": scrub(parsed),
            })
            self.reply(status, payload or b"{}")


def main():
    server = ThreadingHTTPServer(("127.0.0.1", PORT), Handler)
    server.daemon_threads = True
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        sys.exit(0)


main()
PY

# ── the mocks: the live proxy takes the place of pos-fdm-mock for the length of this run ──
if [[ -e "$MOCKS/pos-fdm-mock.live-real" ]]; then
    # left by an interrupted earlier run: put the real binary back first (we hold the P7 lock)
    rm -f "$MOCKS/pos-fdm-mock"
    mv -f "$MOCKS/pos-fdm-mock.live-real" "$MOCKS/pos-fdm-mock"
    printf 'ULIVE | mocks | INFO | restored the pos-fdm-mock binary left by an interrupted run\n'
fi
if [[ ! -x "$MOCKS/pos-fdm-mock" || ! -x "$MOCKS/pos-printer-mock" ]]; then
    if [[ -z "${PILI_LIVE_MOCKS_DIR:-}" ]]; then
        mkdir -p "$ROOT/target"
        bash "$ROOT/scripts/android/heavy.sh" cargo build --locked --manifest-path "$ROOT/Cargo.toml" -p pos-fdm-mock -p pos-printer-mock \
            --target-dir "$ROOT/target/android-mocks" >"$TMP/mocks-build.log" 2>&1 || {
            fail mocks "cargo build of the mocks failed (see the output above in $TMP/mocks-build.log, deleted at exit)"
            tail -n 5 "$TMP/mocks-build.log" >&2
            exit 1
        }
    fi
fi
if [[ ! -x "$MOCKS/pos-fdm-mock" || ! -x "$MOCKS/pos-printer-mock" ]]; then
    fail mocks "$MOCKS does not hold pos-fdm-mock and pos-printer-mock"
    exit 1
fi
mv "$MOCKS/pos-fdm-mock" "$MOCKS/pos-fdm-mock.live-real" || { fail mocks "cannot move pos-fdm-mock aside"; exit 1; }
SWAPPED=1
cat >"$MOCKS/pos-fdm-mock" <<WRAP
#!/bin/sh
# pili-live-wrapper: the live proxy of uc_live_module2.sh, restored by that script at the end of the run
exec python3 -B -I "$TMP/live_proxy.py"
WRAP
chmod +x "$MOCKS/pos-fdm-mock"
pass mocks "live proxy in place of pos-fdm-mock (the real binary is put back at the end)"

export PILI_LIVE_ENV_FILE="$ENV_FILE" PILI_LIVE_MAX_REQUESTS="$MAX_REQ" PILI_LIVE_PAUSE_MS="$PAUSE_MS" PILI_LIVE_STATE_DIR="$STATE"
export PILI_P7_LOCK_HELD=1

# ── the device run ──────────────────────────────────────────────
E2E_OUT="$TMP/uc_e2e.out"
e2e_args=(--out "$EVID/journals")
for g in "${GROUP_LIST[@]}"; do e2e_args+=(--group "$g"); done
printf 'ULIVE | uc_e2e | INFO | %s %s\n' "$UC_E2E" "${e2e_args[*]}"
bash "$UC_E2E" "${e2e_args[@]}" 2>&1 | tee "$E2E_OUT"
E2E_RC=${PIPESTATUS[0]}

is_expected() { # <name>
    local x
    for x in "${EXPECTED_FAIL[@]}"; do [[ "$x" == "$1" ]] && return 0; done
    return 1
}
# the FAIL lines of a tool output that are not an expected live FAIL
unexpected_fails() { # <tag> <file>
    local line name
    while IFS= read -r line; do
        name="$(printf '%s' "$line" | awk -F' [|] ' '{print $2}')"
        [[ "$name" == "total" ]] && continue
        is_expected "$name" && continue
        printf '%s\n' "$name"
    done < <(grep -E "^$1 [|] .* [|] FAIL" "$2" 2>/dev/null)
}

E2E_BAD="$(unexpected_fails UCE2E "$E2E_OUT" | tr '\n' ' ')"
E2E_EXPECTED="$(grep -E '^UCE2E [|] .* [|] FAIL' "$E2E_OUT" 2>/dev/null | awk -F' [|] ' '{print $2}' | while read -r n; do is_expected "$n" && printf '%s ' "$n"; done)"
if [[ -n "$E2E_BAD" ]]; then
    fail uc_e2e "failed: $E2E_BAD(exit $E2E_RC)"
elif ! grep -q '^UCE2E [|] total [|]' "$E2E_OUT"; then
    fail uc_e2e "no UCE2E total line (exit $E2E_RC)"
else
    pass uc_e2e "every scenario of ${GROUP_LIST[*]} met its steps${E2E_EXPECTED:+; expected live FAIL: $E2E_EXPECTED}"
fi

# ── the request budget ──────────────────────────────────────────
FORWARDED="$(tr -d '[:space:]' <"$STATE/count" 2>/dev/null)"
BLOCKED="$(tr -d '[:space:]' <"$STATE/blocked" 2>/dev/null)"
IGNORED="$(tr -d '[:space:]' <"$STATE/control_ignored" 2>/dev/null)"
FORWARDED="${FORWARDED:-0}"
BLOCKED="${BLOCKED:-0}"
IGNORED="${IGNORED:-0}"
if [[ "$FORWARDED" -lt 1 ]]; then
    fail budget "no request reached the simulator: the run proves nothing"
elif [[ "$BLOCKED" -gt 0 ]]; then
    fail budget "$FORWARDED forwarded, $BLOCKED refused at the cap of $MAX_REQ: the journals are incomplete, raise --max-requests"
else
    pass budget "$FORWARDED requests forwarded of $MAX_REQ allowed, $IGNORED mock control calls ignored"
fi
cp "$STATE/forwarded.log" "$EVID/forwarded.log" 2>/dev/null || : >"$EVID/forwarded.log"

# ── reference and comparison ────────────────────────────────────
if [[ ! -x "$UCCHECK_BIN" && -z "${PILI_LIVE_UCCHECK_BIN:-}" ]]; then
    bash "$ROOT/scripts/android/heavy.sh" cargo build --manifest-path "$ROOT/android/uccheck/Cargo.toml" --locked --offline --release \
        --target-dir "$ROOT/target/uccheck" -p pos-uc-check >"$TMP/uccheck-build.log" 2>&1 || {
        fail compare "cargo build of pos-uc-check failed: $(tail -n 2 "$TMP/uccheck-build.log" | tr '\n' ' ')"
        UCCHECK_BIN=""
    }
fi
if [[ -n "$UCCHECK_BIN" && -x "$UCCHECK_BIN" ]]; then
    rec_args=(record --scenarios "$ASSETS" --out "$EVID/journals" --repo "$ROOT")
    for g in "${GROUP_LIST[@]}"; do rec_args+=(--group "$g"); done
    rm -rf "$EVID/journals/linux"
    "$UCCHECK_BIN" "${rec_args[@]}" >"$TMP/record.out" 2>&1 </dev/null
    rec_rc=$?
    if [[ "$rec_rc" -ne 0 ]] || ! grep -q '^UCREC [|] total [|] PASS' "$TMP/record.out"; then
        fail reference "the reference run failed (exit $rec_rc): $(grep '^UCREC | total' "$TMP/record.out" | tail -n 1)"
    else
        pass reference "$(grep '^UCREC | total' "$TMP/record.out" | tail -n 1 | sed 's/^UCREC | total | PASS | //') (engine on the host)"
        "$UCCHECK_BIN" compare --linux "$EVID/journals/linux" --android "$EVID/journals/android" --fixtures --no-counters \
            --repo "$ROOT" --report "$EVID/results.json" >"$TMP/compare.out" 2>&1 </dev/null
        cmp_rc=$?
        cp "$TMP/compare.out" "$EVID/compare.txt"
        cat "$TMP/compare.out"
        CMP_BAD="$(unexpected_fails UCCHECK "$TMP/compare.out" | tr '\n' ' ')"
        CMP_EXPECTED="$(grep -E '^UCCHECK [|] .* [|] FAIL' "$TMP/compare.out" 2>/dev/null | awk -F' [|] ' '{print $2}' | while read -r n; do is_expected "$n" && printf '%s ' "$n"; done)"
        if [[ -n "$CMP_BAD" ]]; then
            fail compare "differs from the reference on: $CMP_BAD(exit $cmp_rc), report $EVID/results.json"
        elif ! grep -q '^UCCHECK [|] total [|]' "$TMP/compare.out"; then
            fail compare "no UCCHECK total line (exit $cmp_rc)"
        else
            pass compare "business content equal to the reference, counters not compared${CMP_EXPECTED:+; expected live FAIL: $CMP_EXPECTED}, report $EVID/results.json"
        fi
    fi
elif [[ -z "$UCCHECK_BIN" ]]; then
    :
else
    fail compare "pos-uc-check not found at $UCCHECK_BIN"
fi

# ── the end ─────────────────────────────────────────────────────
swap_out
if [[ -e "$MOCKS/pos-fdm-mock.live-real" ]] || grep -q 'pili-live-wrapper' "$MOCKS/pos-fdm-mock" 2>/dev/null; then
    fail restore "the real pos-fdm-mock binary was not put back"
else
    pass restore "pos-fdm-mock restored, temporary directory deleted at exit, evidence in $EVID"
fi
if [[ "$NFAIL" -eq 0 ]]; then
    printf 'ULIVE | total | PASS | 0 failed (%s checks)\n' "$NCHECK"
    TOTAL_PRINTED=1
    exit 0
fi
printf 'ULIVE | total | FAIL | %s failed (%s checks)\n' "$NFAIL" "$NCHECK"
TOTAL_PRINTED=1
exit 1
