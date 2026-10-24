#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - instrumented tests of :pos-core on the emulator (plan P3.2, PD81)
#
# Usage: scripts/android/run_android_tests.sh
#
# Runs :pos-core:connectedDebugAndroidTest on sunmi_d3_pro_api33 (DeviceLoadTest,
# DeviceLifecycleTest, DeviceEndToEndTest). The device reaches the host FDM and printer mocks
# only through adb reverse (PD81): device ports 19280 (FDM), 19290 (printer ESC/POS) and 19291
# (printer HTTP jobs), passed to the tests as instrumentation arguments fdmPort, printerPort and
# printerHttpPort. The mocks run on free host ports; logs go to target/android-tests/.
#
# Prints one line per test class and one total line, from the JUnit XML files that AGP writes
# under android/pos-core/build/outputs/androidTest-results/connected:
#     ANDROIDTEST | <class> | PASS|FAIL | <n> tests
#     ANDROIDTEST | total | PASS|FAIL | <n> tests
# Exit 0 only when gradle succeeded, at least 12 tests ran and none failed, errored or was skipped.
#
# On exit (also on error or signal) only what this script created is removed: its three reverse
# rules, its two mock processes and the :pos-core test package when it is still installed. The
# emulator is reused or started by scripts/android/emulator.sh and is never stopped.
# Environment: ANDROID_HOME, ANDROID_NDK_HOME, JAVA_HOME (JDK 17), cargo on PATH.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    sed -n '2,/^# ──*$/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
    exit 0
fi
[[ $# -eq 0 ]] || { echo "usage: run_android_tests.sh" >&2; exit 2; }

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
cd "$ROOT" || exit 2

ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
export ANDROID_HOME
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.1.12297006}"
if [[ -z "${JAVA_HOME:-}" ]] && [[ -x /usr/libexec/java_home ]]; then
    JAVA_HOME="$(/usr/libexec/java_home -v 17 2>/dev/null || true)"
    [[ -z "$JAVA_HOME" ]] || export JAVA_HOME
fi
ADB="$ANDROID_HOME/platform-tools/adb"
[[ -x "$ADB" ]] || ADB="$(command -v adb || true)"

FDM_DEV_PORT=19280
PRINTER_DEV_PORT=19290
PRINTER_HTTP_DEV_PORT=19291
MIN_TESTS=12
TEST_PKG_PATTERN="pilipos.core"
MOCKS="$ROOT/target/android-mocks/debug"
FDM_BIN="$MOCKS/pos-fdm-mock"
PRINTER_BIN="$MOCKS/pos-printer-mock"
OUTDIR="$ROOT/target/android-tests"
RESULTS="$ROOT/android/pos-core/build/outputs/androidTest-results/connected"

FDM_PID=""
PRINTER_PID=""
SERIAL=""
REVERSED=0

adbd() { "$ADB" -s "$SERIAL" "$@"; }

note() { printf 'run_android_tests: %s\n' "$*" >&2; }

cleanup() {
    local rc=$?
    trap - EXIT INT TERM
    if [[ -n "$SERIAL" ]]; then
        # The test package, when gradle did not get to remove it (kill, crash).
        local pkg
        for pkg in $(adbd shell pm list packages 2>/dev/null | tr -d '\r' | sed -n 's/^package://p' | grep "$TEST_PKG_PATTERN"); do
            adbd uninstall "$pkg" >/dev/null 2>&1
        done
        if [[ "$REVERSED" -eq 1 ]]; then
            adbd reverse --remove "tcp:$FDM_DEV_PORT" >/dev/null 2>&1
            adbd reverse --remove "tcp:$PRINTER_DEV_PORT" >/dev/null 2>&1
            adbd reverse --remove "tcp:$PRINTER_HTTP_DEV_PORT" >/dev/null 2>&1
        fi
    fi
    [[ -z "$FDM_PID" ]] || { kill "$FDM_PID" 2>/dev/null; wait "$FDM_PID" 2>/dev/null; }
    [[ -z "$PRINTER_PID" ]] || { kill "$PRINTER_PID" 2>/dev/null; wait "$PRINTER_PID" 2>/dev/null; }
    exit "$rc"
}
trap cleanup EXIT
trap 'exit 130' INT TERM

fail() { # <message>
    printf 'ANDROIDTEST | total | FAIL | %s\n' "$1"
    exit 1
}

free_port() { python3 -I -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])'; }

accepts() { (exec 3<>"/dev/tcp/127.0.0.1/$1") 2>/dev/null; }

wait_port() { # <port> <seconds>
    local deadline=$(($(date +%s) + $2))
    until accepts "$1"; do
        [[ "$(date +%s)" -lt "$deadline" ]] || return 1
        sleep 0.3
    done
}

mkdir -p "$OUTDIR"
[[ -n "$ADB" && -x "$ADB" ]] || fail "adb not found (ANDROID_HOME=$ANDROID_HOME)"

# ── mocks ───────────────────────────────────────────────────────
if [[ ! -x "$FDM_BIN" || ! -x "$PRINTER_BIN" ]]; then
    cargo build --locked --manifest-path "$ROOT/Cargo.toml" -p pos-fdm-mock -p pos-printer-mock \
        --target-dir "$ROOT/target/android-mocks" >"$OUTDIR/mocks-build.log" 2>&1 ||
        { tail -n 20 "$OUTDIR/mocks-build.log" >&2; fail "cargo build of the mocks failed, see $OUTDIR/mocks-build.log"; }
fi

SERIAL="$(bash scripts/android/emulator.sh ensure 2>"$OUTDIR/emulator.err")" || {
    cat "$OUTDIR/emulator.err" >&2
    SERIAL=""
    fail "emulator.sh ensure failed"
}
note "emulator $SERIAL"

FDM_PORT="$(free_port)"
PRINTER_PORT="$(free_port)"
PRINTER_HTTP="$(free_port)"
PORT="$FDM_PORT" "$FDM_BIN" >"$OUTDIR/fdm-mock.log" 2>&1 &
FDM_PID=$!
PORT="$PRINTER_PORT" HTTP_PORT="$PRINTER_HTTP" "$PRINTER_BIN" >"$OUTDIR/printer-mock.log" 2>&1 &
PRINTER_PID=$!
wait_port "$FDM_PORT" 20 && wait_port "$PRINTER_PORT" 20 && wait_port "$PRINTER_HTTP" 20 ||
    fail "a mock does not accept connections, see $OUTDIR/*-mock.log"
note "mocks: fdm $FDM_PORT, printer $PRINTER_PORT, jobs http $PRINTER_HTTP"

REVERSED=1 # whatever of the three rules exists is removed on exit
adbd reverse "tcp:$FDM_DEV_PORT" "tcp:$FDM_PORT" >/dev/null &&
    adbd reverse "tcp:$PRINTER_DEV_PORT" "tcp:$PRINTER_PORT" >/dev/null &&
    adbd reverse "tcp:$PRINTER_HTTP_DEV_PORT" "tcp:$PRINTER_HTTP" >/dev/null || fail "adb reverse failed"

# A sleeping headless emulator holds back the network: wake the screen (no setting is changed).
adbd shell "input keyevent KEYCODE_WAKEUP; wm dismiss-keyguard" >/dev/null 2>&1
sleep 1

# The tunnel must carry data, not only accept a connection.
tunnel_ok() { # <device port>
    adbd shell "(printf 'GET /jobs HTTP/1.0\\r\\n\\r\\n'; sleep 1) | nc -w 3 127.0.0.1 $1" 2>/dev/null | grep -q '^HTTP/'
}
tunnel_fdm_ok() {
    adbd shell "(printf 'GET /graphql HTTP/1.0\\r\\n\\r\\n'; sleep 1) | nc -w 3 127.0.0.1 $FDM_DEV_PORT" 2>/dev/null | grep -q '^HTTP/'
}
ok=0
for _ in 1 2 3; do
    if tunnel_fdm_ok && tunnel_ok "$PRINTER_HTTP_DEV_PORT"; then ok=1; break; fi
done
[[ "$ok" -eq 1 ]] || fail "the mocks do not answer through the device ports $FDM_DEV_PORT and $PRINTER_HTTP_DEV_PORT"
note "adb reverse: $FDM_DEV_PORT -> $FDM_PORT, $PRINTER_DEV_PORT -> $PRINTER_PORT, $PRINTER_HTTP_DEV_PORT -> $PRINTER_HTTP"

# ── gradle ──────────────────────────────────────────────────────
# Stale results of an earlier run must not be read as this run's.
rm -rf "$RESULTS"
export ANDROID_SERIAL="$SERIAL"
note "connectedDebugAndroidTest (log: $OUTDIR/gradle.log)"
(
    cd android &&
        ./gradlew --no-daemon --console=plain :pos-core:connectedDebugAndroidTest \
            -Pandroid.testInstrumentationRunnerArguments.fdmPort="$FDM_DEV_PORT" \
            -Pandroid.testInstrumentationRunnerArguments.printerPort="$PRINTER_DEV_PORT" \
            -Pandroid.testInstrumentationRunnerArguments.printerHttpPort="$PRINTER_HTTP_DEV_PORT"
) >"$OUTDIR/gradle.log" 2>&1
gradle_rc=$?
if [[ "$gradle_rc" -ne 0 ]]; then
    grep -E "FAILED|AssertionError|Exception|What went wrong" "$OUTDIR/gradle.log" | head -n 30 >&2
    tail -n 15 "$OUTDIR/gradle.log" >&2
fi

# ── result files ────────────────────────────────────────────────
# The folder layout varies by AGP version: every *.xml below $RESULTS with a <testsuite> root counts.
python3 -I - "$RESULTS" "$MIN_TESTS" "$gradle_rc" <<'PY'
import glob
import os
import sys
import xml.etree.ElementTree as ET

root, minimum, gradle_rc = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
per_class = {}
files = sorted(glob.glob(os.path.join(root, "**", "*.xml"), recursive=True))
for path in files:
    try:
        tree = ET.parse(path).getroot()
    except ET.ParseError:
        continue
    suites = [tree] if tree.tag == "testsuite" else list(tree.iter("testsuite"))
    for suite in suites:
        for case in suite.iter("testcase"):
            cls = case.get("classname", "?").rsplit(".", 1)[-1]
            entry = per_class.setdefault(cls, {"tests": 0, "bad": 0})
            entry["tests"] += 1
            if case.find("failure") is not None or case.find("error") is not None or case.find("skipped") is not None:
                entry["bad"] += 1
                print("ANDROIDTEST | detail | FAIL | %s.%s" % (cls, case.get("name")), file=sys.stderr)

total = sum(e["tests"] for e in per_class.values())
bad = sum(e["bad"] for e in per_class.values())
for cls in sorted(per_class):
    e = per_class[cls]
    print("ANDROIDTEST | %s | %s | %d tests" % (cls, "FAIL" if e["bad"] else "PASS", e["tests"]))
problems = []
if gradle_rc != 0:
    problems.append("gradle exit %d" % gradle_rc)
if not files:
    problems.append("no result file under %s" % root)
if total < minimum:
    problems.append("%d tests, at least %d expected" % (total, minimum))
if bad:
    problems.append("%d failed or skipped" % bad)
if problems:
    print("ANDROIDTEST | total | FAIL | %d tests (%s)" % (total, "; ".join(problems)))
    sys.exit(1)
print("ANDROIDTEST | total | PASS | %d tests" % total)
PY
exit $?
