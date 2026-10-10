#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of scripts/android/uc_live_module2.sh (P7-T12)
#
# Usage: bash scripts/android/test_uc_live_module2.sh
#
# No device, no emulator, no module2: a fake environment file with a canary token, stub commands in place of
# uc_e2e.sh and pos-uc-check, a fake pos-fdm-mock binary and a fake simulator on 127.0.0.1 that records the
# Authorization header it receives. Cases: --help, the refusals (no flag, flag only, variable only, the refused
# group, an estimate above the cap, a missing token, an unsafe URL), --dry-run (does not read the environment
# file, creates nothing, with the go-ahead too), a full stubbed run (the token reaches the simulator and
# nothing else: not the output, not the evidence, not a temporary file; the temporary directory is deleted; the
# mock binary is restored; pos-uc-check compare gets --no-counters and --fixtures), the cap (the simulator sees
# no more requests than the cap), a run that sends nothing, a failing scenario, the documented expected FAIL, and
# the recovery of a binary left aside by an interrupted run.
# Prints one line per case, then
#     TESTULIVE | total | PASS|FAIL | <n> tests
# and exits 1 when a case fails.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
SUT="$ROOT/scripts/android/uc_live_module2.sh"
TMP="$(mktemp -d)"
BGPIDS=()
cleanup() {
    local p
    for p in ${BGPIDS[@]+"${BGPIDS[@]}"}; do kill "$p" 2>/dev/null || true; done
    chmod -R u+rw "$TMP" 2>/dev/null || true
    rm -rf "$TMP"
}
trap cleanup EXIT
export HOME="$TMP/home"
mkdir -p "$HOME"
unset PILI_LIVE_GO_AHEAD PILI_LIVE_MAX_REQUESTS PILI_LIVE_PAUSE_MS PILI_P7_LOCK_HELD PILI_LIVE_FDM_URL PILI_LIVE_FDM_TOKEN
pass=0
fail=0

ok() { pass=$((pass + 1)); printf 'TESTULIVE | %s | PASS\n' "$1"; }
bad() { fail=$((fail + 1)); printf 'TESTULIVE | %s | FAIL | %s\n' "$1" "$2"; }
# check NAME RESULT(0 = true) DETAIL
check() { if [[ "$2" -eq 0 ]]; then ok "$1"; else bad "$1" "$3"; fi; }
has() { grep -qF -- "$2" "$1"; } # has FILE TEXT

# The canary is assembled here so that this file never holds the literal.
CANARY="CANARY_TOKEN_$(printf '%s' 9f3a7c1d)_$(printf '%s' zz81)"

free_port() { python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()'; }

# ── fixtures ──
ASSETS="$TMP/assets"
JOURNALS="$TMP/journals"
for g in 00_smoke 10_basic 60_fdm_errors; do
    mkdir -p "$ASSETS/$g" "$JOURNALS/$g"
done
echo '{}' >"$ASSETS/00_smoke/001_SMOKE.json"
echo '{"fdm":[{},{}],"statusCalls":[{}]}' >"$JOURNALS/00_smoke/001_SMOKE.json" # 3 requests
echo '{}' >"$ASSETS/10_basic/140_Q110.json"
echo '{"fdm":[],"statusCalls":[{}]}' >"$JOURNALS/10_basic/140_Q110.json" # 1 request
echo '{}' >"$ASSETS/60_fdm_errors/010_V.json"
echo '{"fdm":[],"statusCalls":[]}' >"$JOURNALS/60_fdm_errors/010_V.json"
export PILI_LIVE_ASSETS="$ASSETS" PILI_LIVE_JOURNALS="$JOURNALS"

MOCKS="$TMP/mocks"
mkdir -p "$MOCKS"
printf '#!/bin/sh\necho REAL-FDM-MOCK\n' >"$MOCKS/pos-fdm-mock"
printf '#!/bin/sh\necho REAL-PRINTER-MOCK\n' >"$MOCKS/pos-printer-mock"
chmod +x "$MOCKS/pos-fdm-mock" "$MOCKS/pos-printer-mock"
export PILI_LIVE_MOCKS_DIR="$MOCKS"

# The fake simulator: records the Authorization header of each request in $TMP/upstream.log.
UP_PORT="$(free_port)"
UP_LOG="$TMP/upstream.log"
: >"$UP_LOG"
cat >"$TMP/upstream.py" <<'PY'
import json, sys
from http.server import BaseHTTPRequestHandler, HTTPServer

class H(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass
    def do_POST(self):
        n = int(self.headers.get("Content-Length") or 0)
        self.rfile.read(n)
        with open(sys.argv[2], "a") as f:
            f.write("%s|%s\n" % (self.headers.get("Authorization", ""), self.headers.get("Simulator-Version", "")))
        body = json.dumps({"data": {"ok": 1}, "info": {"visitor_ip": "203.0.113.9", "version": "2.2.0.5"}}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

HTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
PY
python3 -B -I "$TMP/upstream.py" "$UP_PORT" "$UP_LOG" &
BGPIDS+=("$!")
for _ in 1 2 3 4 5 6 7 8 9 10; do
    (exec 3<>"/dev/tcp/127.0.0.1/$UP_PORT") 2>/dev/null && break
    sleep 0.3
done

ENVF="$TMP/env.live"
write_env() { # <token or empty> [url]
    {
        echo "# fake environment file of the self-test"
        echo "PILI_LIVE_FDM_URL=${2:-http://127.0.0.1:$UP_PORT/graphql}"
        [[ -z "$1" ]] || printf '%s=%s\n' PILI_LIVE_FDM_TOKEN "$1"
        echo "PILI_LIVE_SIMULATOR_VERSION=2.2.0.5"
    } >"$ENVF"
}
write_env "$CANARY"

# The stub of uc_e2e.sh: plays the part of the app against the pos-fdm-mock of the mocks directory (the live
# proxy when the script put it there) and writes a journal file where uc_e2e.sh would.
STUB_E2E="$TMP/stub_uc_e2e.sh"
cat >"$STUB_E2E" <<'STUB'
#!/usr/bin/env bash
out=""; groups=()
while [[ $# -gt 0 ]]; do
    case "$1" in
    --out) out="$2"; shift 2 ;;
    --group) groups+=("$2"); shift 2 ;;
    *) shift ;;
    esac
done
echo "stub uc_e2e: $*" >>"$STUB_MARK"
port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')"
PORT="$port" "$PILI_LIVE_MOCKS_DIR/pos-fdm-mock" >"$STUB_MOCK_OUT" 2>&1 &
mock=$!
for _ in $(seq 1 40); do
    (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null && break
    sleep 0.25
done
curl -s -o /dev/null -w 'probe %{http_code}\n' "http://127.0.0.1:$port/graphql" >>"$STUB_CODES"
curl -s -o /dev/null -w 'control %{http_code}\n' -X POST -d '{"warnings":["X"]}' "http://127.0.0.1:$port/mock/control" >>"$STUB_CODES"
i=0
while [[ "$i" -lt "${STUB_CALLS:-3}" ]]; do
    i=$((i + 1))
    if [[ "$i" -eq 1 ]]; then q='query { status(a:1) { x } }'; else q='mutation { signSale(input: {}) { x } }'; fi
    curl -s -o /dev/null -w 'call %{http_code}\n' -X POST -H 'Content-Type: application/json' \
        -d "{\"query\":\"$q\",\"variables\":{}}" "http://127.0.0.1:$port/graphql" >>"$STUB_CODES"
done
curl -s "http://127.0.0.1:$port/mock/requests" >"$STUB_REQS"
curl -s -o /dev/null -X DELETE "http://127.0.0.1:$port/mock/requests"
kill "$mock" 2>/dev/null; wait "$mock" 2>/dev/null
for g in "${groups[@]}"; do
    mkdir -p "$out/android/$g"
    echo '{}' >"$out/android/$g/001.json"
    if [[ "$g" == 10_basic && "${STUB_Q110:-}" == fail ]]; then
        echo "UCE2E | 10_basic/140_Q110 | FAIL | expected on the simulator"
    else
        echo "UCE2E | $g/001 | PASS | ok"
    fi
    [[ -z "${STUB_BAD:-}" ]] || echo "UCE2E | $g/002_BAD | FAIL | stub failure"
done
if [[ -n "${STUB_BAD:-}${STUB_Q110:-}" ]]; then echo "UCE2E | total | FAIL | 1 failed"; exit 1; fi
echo "UCE2E | total | PASS | 0 failed"
STUB
chmod +x "$STUB_E2E"
export PILI_LIVE_UC_E2E="$STUB_E2E"

STUB_UCC="$TMP/stub_pos_uc_check"
cat >"$STUB_UCC" <<'STUB'
#!/usr/bin/env bash
echo "$*" >>"$UCC_ARGS"
case "$1" in
record)
    out=""; while [[ $# -gt 0 ]]; do [[ "$1" == --out ]] && out="$2"; shift; done
    mkdir -p "$out/linux/00_smoke"
    echo "UCREC | total | PASS | 1 scenarios" ;;
compare)
    echo "UCCHECK | 00_smoke/001 | PASS | steps fdm-ops"
    if [[ "${STUB_Q110:-}" == fail ]]; then echo "UCCHECK | 10_basic/140_Q110 | FAIL | status-shape"; echo "UCCHECK | total | FAIL | 1 failed (2 scenarios)"; exit 1; fi
    echo "UCCHECK | total | PASS | 0 failed (1 scenarios)" ;;
esac
STUB
chmod +x "$STUB_UCC"
export PILI_LIVE_UCCHECK_BIN="$STUB_UCC"

export STUB_MARK="$TMP/stub.mark" STUB_CODES="$TMP/stub.codes" STUB_REQS="$TMP/stub.reqs" STUB_MOCK_OUT="$TMP/stub.mockout" UCC_ARGS="$TMP/ucc.args"

export TMPDIR="$TMP/tmpdir"
mkdir -p "$TMPDIR"
export PILI_P7_LOCK_DIR="$TMP/lock"
mkdir -p "$PILI_P7_LOCK_DIR"
export PILI_P7_LOCK_HELD=1 # the lock itself is proven by test_p7_emulator_lock.sh

reset_case() {
    : >"$STUB_MARK"; : >"$STUB_CODES"; : >"$STUB_REQS"; : >"$STUB_MOCK_OUT"; : >"$UCC_ARGS"; : >"$UP_LOG"
    rm -rf "$TMP/evid" "$TMPDIR"/* "$TMP/lock"/* 2>/dev/null
    unset STUB_CALLS STUB_BAD STUB_Q110 PILI_LIVE_GO_AHEAD PILI_LIVE_MAX_REQUESTS
    write_env "$CANARY"
    printf '#!/bin/sh\necho REAL-FDM-MOCK\n' >"$MOCKS/pos-fdm-mock"
    chmod +x "$MOCKS/pos-fdm-mock"
    rm -f "$MOCKS/pos-fdm-mock.live-real"
}
mocks_intact() { [[ ! -e "$MOCKS/pos-fdm-mock.live-real" ]] && grep -q REAL-FDM-MOCK "$MOCKS/pos-fdm-mock"; }
nothing_done() { [[ ! -s "$STUB_MARK" ]] && [[ -z "$(ls -A "$TMPDIR")" ]] && mocks_intact && [[ ! -e "$TMP/evid" ]]; }
upstream_count() { grep -c . "$UP_LOG" 2>/dev/null || echo 0; }
OUT="$TMP/run.out"

# ── 1 help ──
reset_case
bash "$SUT" --help >"$OUT" 2>&1
rc=$?
check help $((rc != 0 || !$(has "$OUT" -- '--go-ahead' && has "$OUT" 'PILI_LIVE_GO_AHEAD=yes' && echo 1 || echo 0))) "exit $rc"

# ── 2 refusals ──
reset_case
bash "$SUT" >"$OUT" 2>&1
rc=$?
check refuse-no-flags $((rc == 0)) "exit $rc"
check refuse-no-flags-does-nothing $(nothing_done && echo 0 || echo 1) "side effects"
reset_case
bash "$SUT" --go-ahead --env-file "$ENVF" --out "$TMP/evid" >"$OUT" 2>&1
rc=$?
check refuse-flag-only $((rc != 3 || $(nothing_done && echo 0 || echo 1))) "exit $rc"
reset_case
PILI_LIVE_GO_AHEAD=yes bash "$SUT" --env-file "$ENVF" --out "$TMP/evid" >"$OUT" 2>&1
rc=$?
check refuse-variable-only $((rc != 3 || $(nothing_done && echo 0 || echo 1))) "exit $rc"
reset_case
PILI_LIVE_GO_AHEAD=yes bash "$SUT" --go-ahead --env-file "$ENVF" --group 60_fdm_errors --out "$TMP/evid" >"$OUT" 2>&1
rc=$?
check refuse-fdm-errors-group $((rc != 2 || $(nothing_done && echo 0 || echo 1))) "exit $rc"
reset_case
PILI_LIVE_GO_AHEAD=yes bash "$SUT" --go-ahead --env-file "$ENVF" --group 00_smoke --max-requests 2 --out "$TMP/evid" >"$OUT" 2>&1
rc=$?
check refuse-estimate-above-cap $((rc != 1 || $(has "$OUT" 'above the cap' && echo 0 || echo 1) || $(nothing_done && echo 0 || echo 1))) "exit $rc"
reset_case
write_env ""
PILI_LIVE_GO_AHEAD=yes bash "$SUT" --go-ahead --env-file "$ENVF" --group 00_smoke --out "$TMP/evid" >"$OUT" 2>&1
rc=$?
check refuse-missing-token $((rc != 4 || $(has "$OUT" 'token: none' && echo 0 || echo 1) || $(nothing_done && echo 0 || echo 1))) "exit $rc"
reset_case
write_env "$CANARY" 'file:///etc/passwd'
PILI_LIVE_GO_AHEAD=yes bash "$SUT" --go-ahead --env-file "$ENVF" --group 00_smoke --out "$TMP/evid" >"$OUT" 2>&1
rc=$?
check refuse-unsafe-url $((rc != 4 || $(nothing_done && echo 0 || echo 1))) "exit $rc"
reset_case
PILI_LIVE_GO_AHEAD=yes bash "$SUT" --go-ahead --env-file "$TMP/does-not-exist" --group 00_smoke --out "$TMP/evid" >"$OUT" 2>&1
rc=$?
check refuse-missing-env-file $((rc != 4 || $(nothing_done && echo 0 || echo 1))) "exit $rc"

# ── 3 dry runs ──
reset_case
chmod 000 "$ENVF"
bash "$SUT" --dry-run --env-file "$ENVF" >"$OUT" 2>&1
rc=$?
chmod 600 "$ENVF"
check dry-run-plan $((rc != 0 || $(has "$OUT" 'groups: 00_smoke 10_basic' && has "$OUT" 'request estimate' && has "$OUT" 'token: not read' && echo 0 || echo 1))) "exit $rc"
check dry-run-no-token $(has "$OUT" "$CANARY" && echo 1 || echo 0) "canary in the plan"
check dry-run-does-nothing $(nothing_done && echo 0 || echo 1) "side effects"
reset_case
PILI_LIVE_GO_AHEAD=yes bash "$SUT" --dry-run --go-ahead --env-file "$ENVF" --out "$TMP/evid" >"$OUT" 2>&1
rc=$?
check dry-run-with-go-ahead $((rc != 0 || $(nothing_done && echo 0 || echo 1) || $(has "$OUT" 'ULIVE | dry-run | PASS' && echo 0 || echo 1))) "exit $rc"

# ── 4 a full stubbed run ──
reset_case
export STUB_CALLS=3
PILI_LIVE_GO_AHEAD=yes PILI_LIVE_PAUSE_MS=0 bash "$SUT" --go-ahead --env-file "$ENVF" --group 00_smoke --out "$TMP/evid" >"$OUT" 2>&1
rc=$?
check run-passes $((rc != 0 || $(has "$OUT" 'ULIVE | total | PASS' && echo 0 || echo 1))) "exit $rc: $(tail -n 4 "$OUT" | tr '\n' ' ')"
check run-says-token-set $(has "$OUT" 'token: set' && echo 0 || echo 1) "no 'token: set' line"
check run-token-reaches-simulator $((! $(has "$UP_LOG" "Bearer $CANARY" && echo 1 || echo 0))) "the simulator did not receive the bearer token"
check run-simulator-version $(has "$UP_LOG" '|2.2.0.5' && echo 0 || echo 1) "no Simulator-Version header"
check run-forwarded-count $(((($(upstream_count)) != 3))) "simulator saw $(upstream_count) requests, expected 3"
check run-probe-and-control-answered-locally $(has "$STUB_CODES" 'probe 200' && has "$STUB_CODES" 'control 200' && echo 0 || echo 1) "$(tr '\n' ' ' <"$STUB_CODES")"
check run-mock-requests-listed $(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); assert [c["operation"] for c in d]==["Status","signSale","signSale"], d; assert "visitor_ip" not in json.dumps(d)' "$STUB_REQS" 2>/dev/null && echo 0 || echo 1) "GET /mock/requests: $(head -c 300 "$STUB_REQS")"
check run-compare-flags $(has "$UCC_ARGS" 'compare' && has "$UCC_ARGS" '--no-counters' && has "$UCC_ARGS" '--fixtures' && echo 0 || echo 1) "$(tr '\n' ';' <"$UCC_ARGS")"
check run-mock-restored $(mocks_intact && echo 0 || echo 1) "pos-fdm-mock not restored"
check run-tmpdir-deleted $([[ -z "$(ls -A "$TMPDIR")" ]] && echo 0 || echo 1) "left: $(ls "$TMPDIR" | tr '\n' ' ')"
leak=0
grep -rlF -- "$CANARY" "$OUT" "$TMP/evid" "$TMPDIR" "$STUB_CODES" "$STUB_REQS" "$STUB_MOCK_OUT" "$UCC_ARGS" "$TMP/lock" 2>/dev/null | grep -q . && leak=1
check run-token-nowhere $leak "the token value appears in the output, the evidence or a temporary file"
repoleak=0
grep -rlF -- "$CANARY" "$ROOT/scripts" "$ROOT/docs" "$ROOT/target/android-uc-live" 2>/dev/null | grep -v test_uc_live_module2.sh | grep -q . && repoleak=1
check run-token-not-in-repo $repoleak "the token value appears in the work tree"
check run-evidence-written $([[ -s "$TMP/evid/forwarded.log" && -d "$TMP/evid/journals/android/00_smoke" ]] && echo 0 || echo 1) "no forwarded.log or journals"

# ── 5 the cap ──
reset_case
export STUB_CALLS=6
PILI_LIVE_GO_AHEAD=yes PILI_LIVE_PAUSE_MS=0 bash "$SUT" --go-ahead --env-file "$ENVF" --group 10_basic --max-requests 3 --out "$TMP/evid" >"$OUT" 2>&1
rc=$?
check cap-stops-forwarding $(((($(upstream_count)) != 3))) "simulator saw $(upstream_count) requests, cap 3"
check cap-reported $((rc != 1 || $(has "$OUT" 'refused at the cap' && echo 0 || echo 1))) "exit $rc"
check cap-answers-429 $(grep -c 'call 429' "$STUB_CODES" | grep -q '^3$' && echo 0 || echo 1) "$(tr '\n' ' ' <"$STUB_CODES")"
check cap-mock-restored $(mocks_intact && echo 0 || echo 1) "pos-fdm-mock not restored"

# ── 6 nothing sent, a failing scenario, the expected FAIL, a stale binary ──
reset_case
export STUB_CALLS=0
PILI_LIVE_GO_AHEAD=yes PILI_LIVE_PAUSE_MS=0 bash "$SUT" --go-ahead --env-file "$ENVF" --group 00_smoke --out "$TMP/evid" >"$OUT" 2>&1
rc=$?
check nothing-forwarded-fails $((rc != 1 || $(has "$OUT" 'no request reached the simulator' && echo 0 || echo 1))) "exit $rc"
reset_case
export STUB_CALLS=3 STUB_BAD=1
PILI_LIVE_GO_AHEAD=yes PILI_LIVE_PAUSE_MS=0 bash "$SUT" --go-ahead --env-file "$ENVF" --group 00_smoke --out "$TMP/evid" >"$OUT" 2>&1
rc=$?
check scenario-failure-fails $((rc != 1 || $(has "$OUT" 'ULIVE | total | FAIL' && echo 0 || echo 1) || $(mocks_intact && echo 0 || echo 1))) "exit $rc"
reset_case
export STUB_CALLS=3 STUB_Q110=fail
PILI_LIVE_GO_AHEAD=yes PILI_LIVE_PAUSE_MS=0 bash "$SUT" --go-ahead --env-file "$ENVF" --group 10_basic --out "$TMP/evid" >"$OUT" 2>&1
rc=$?
check expected-live-fail-not-counted $((rc != 0 || $(has "$OUT" 'expected live FAIL: 10_basic/140_Q110' && has "$OUT" 'ULIVE | total | PASS' && echo 0 || echo 1))) "exit $rc: $(tail -n 5 "$OUT" | tr '\n' ' ')"
reset_case
printf '#!/bin/sh\necho REAL-FDM-MOCK\n' >"$MOCKS/pos-fdm-mock.live-real"
chmod +x "$MOCKS/pos-fdm-mock.live-real"
printf '#!/bin/sh\n# pili-live-wrapper\nexit 0\n' >"$MOCKS/pos-fdm-mock"
export STUB_CALLS=3
PILI_LIVE_GO_AHEAD=yes PILI_LIVE_PAUSE_MS=0 bash "$SUT" --go-ahead --env-file "$ENVF" --group 00_smoke --out "$TMP/evid" >"$OUT" 2>&1
rc=$?
check stale-binary-recovered $((rc != 0 || $(mocks_intact && echo 0 || echo 1) || $(has "$OUT" 'restored the pos-fdm-mock binary left by an interrupted run' && echo 0 || echo 1))) "exit $rc"

printf 'TESTULIVE | total | %s | %s tests\n' "$([[ "$fail" -eq 0 ]] && echo PASS || echo FAIL)" "$((pass + fail))"
[[ "$fail" -eq 0 ]]
