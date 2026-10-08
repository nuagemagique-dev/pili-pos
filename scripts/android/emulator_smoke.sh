#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - emulator smoke run of the :smoke app (PD80, PD81)
#
# Usage: scripts/android/emulator_smoke.sh [--no-build]
#
# Proves the Rust engine and the consumer R8 rules of :pos-core on the emulator: builds the
# minified release APK of :smoke, checks it, signs a copy with the development key, installs it
# on sunmi_d3_pro_api33, runs the scripted scenario against the host FDM and printer mocks and
# reads the result from logcat. Prints one line per check
#     SMOKE | <check> | PASS|FAIL | <detail>
# (the scenario steps of the device are printed with the same shape), and ends with exactly one
#     SMOKE | total | PASS|FAIL | <detail>
# Exit 0 only when everything passed.
#
# --no-build   use the APK and the mock binaries that exist (fails when one is missing)
#
# Device ports (PD81): 19180 for the FDM mock, 19190 for the printer mock (adb reverse). Only
# these two reverse rules, the package dev.nuagemagique.pilipos.smoke and the mock processes
# started here are removed on exit. The emulator is never stopped, wiped or provisioned.
# Environment: ANDROID_HOME, ANDROID_NDK_HOME, JAVA_HOME (JDK 17), cargo on PATH.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    sed -n '2,/^# ──*$/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
    exit 0
fi
build=1
case "${1:-}" in
"") ;;
--no-build) build=0 ;;
*) echo "usage: emulator_smoke.sh [--no-build]" >&2; exit 2 ;;
esac
[[ $# -le 1 ]] || { echo "usage: emulator_smoke.sh [--no-build]" >&2; exit 2; }

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

PKG="dev.nuagemagique.pilipos.smoke"
FDM_DEV_PORT=19180
PRINTER_DEV_PORT=19190
APK_UNSIGNED="$ROOT/android/smoke/build/outputs/apk/release/smoke-release-unsigned.apk"
OUTDIR="$ROOT/target/smoke"
APK_SIGNED="$OUTDIR/smoke-release-dev-signed.apk"
MOCKS="$ROOT/target/android-mocks/debug"
FDM_BIN="$MOCKS/pos-fdm-mock"
PRINTER_BIN="$MOCKS/pos-printer-mock"
TOTAL_TIMEOUT=240

failed=0
checks=0
FDM_PID=""
PRINTER_PID=""
SERIAL=""
REVERSED=0
INSTALLED=0

# say <check> <PASS|FAIL> <detail>
say() {
    local d
    d="$(printf '%s' "$3" | tr '\n|' ' /' | sed 's/  */ /g; s/^ //; s/ $//' | cut -c1-200)"
    printf 'SMOKE | %s | %s | %s\n' "$1" "$2" "${d:-ok}"
    checks=$((checks + 1))
    [[ "$2" != "FAIL" ]] || failed=$((failed + 1))
}

adbd() { "$ADB" -s "$SERIAL" "$@"; }

cleanup() {
    local rc=$?
    trap - EXIT INT TERM
    if [[ -n "$SERIAL" ]]; then
        [[ "$INSTALLED" -eq 0 ]] || adbd uninstall "$PKG" >/dev/null 2>&1
        if [[ "$REVERSED" -eq 1 ]]; then
            adbd reverse --remove "tcp:$FDM_DEV_PORT" >/dev/null 2>&1
            adbd reverse --remove "tcp:$PRINTER_DEV_PORT" >/dev/null 2>&1
        fi
    fi
    [[ -z "$FDM_PID" ]] || { kill "$FDM_PID" 2>/dev/null; wait "$FDM_PID" 2>/dev/null; }
    [[ -z "$PRINTER_PID" ]] || { kill "$PRINTER_PID" 2>/dev/null; wait "$PRINTER_PID" 2>/dev/null; }
    exit "$rc"
}
trap cleanup EXIT
trap 'exit 130' INT TERM

finish() {
    if [[ "$failed" -eq 0 ]]; then
        printf 'SMOKE | total | PASS | 0 failed (%d checks)\n' "$checks"
        exit 0
    fi
    printf 'SMOKE | total | FAIL | %d failed (%d checks)\n' "$failed" "$checks"
    exit 1
}

# abort <check> <detail>: a FAIL that makes the rest pointless.
abort() {
    say "$1" FAIL "$2"
    finish
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
[[ -n "$ADB" && -x "$ADB" ]] || abort tools "adb not found (ANDROID_HOME=$ANDROID_HOME)"

# ── build ───────────────────────────────────────────────────────
if [[ "$build" -eq 1 ]]; then
    if [[ ! -x "$FDM_BIN" || ! -x "$PRINTER_BIN" ]]; then
        if cargo build --locked --manifest-path "$ROOT/Cargo.toml" -p pos-fdm-mock -p pos-printer-mock \
            --target-dir "$ROOT/target/android-mocks" >"$OUTDIR/mocks-build.log" 2>&1; then
            say build-mocks PASS "built into target/android-mocks"
        else
            tail -n 20 "$OUTDIR/mocks-build.log" >&2
            abort build-mocks "cargo build failed, see $OUTDIR/mocks-build.log"
        fi
    fi
    if (cd android && ./gradlew --no-daemon --console=plain :smoke:assembleRelease) >"$OUTDIR/gradle.log" 2>&1; then
        say build-apk PASS ":smoke:assembleRelease"
    else
        tail -n 30 "$OUTDIR/gradle.log" >&2
        abort build-apk "gradle failed, see $OUTDIR/gradle.log"
    fi
fi
[[ -f "$APK_UNSIGNED" ]] || abort apk "missing $APK_UNSIGNED (run without --no-build)"
[[ -x "$FDM_BIN" && -x "$PRINTER_BIN" ]] || abort mocks "missing mock binaries under $MOCKS (run without --no-build)"

# ── check and sign ──────────────────────────────────────────────
if bash scripts/android/check_apk.sh "$APK_UNSIGNED" >"$OUTDIR/check_apk.out" 2>&1; then
    say apk-check PASS "$(grep -c '| PASS |' "$OUTDIR/check_apk.out") checks passed"
else
    cat "$OUTDIR/check_apk.out" >&2
    abort apk-check "check_apk.sh failed"
fi
if bash scripts/android/sign_dev.sh "$APK_UNSIGNED" "$APK_SIGNED" >"$OUTDIR/sign.out" 2>&1; then
    say sign PASS "copy signed with the development key (v2, v3)"
else
    cat "$OUTDIR/sign.out" >&2
    abort sign "sign_dev.sh failed"
fi

# ── emulator and mocks ──────────────────────────────────────────
SERIAL="$(bash scripts/android/emulator.sh ensure 2>"$OUTDIR/emulator.err")" || {
    cat "$OUTDIR/emulator.err" >&2
    SERIAL=""
    abort emulator "emulator.sh ensure failed"
}
say emulator PASS "$SERIAL"

FDM_PORT="$(free_port)"
PRINTER_PORT="$(free_port)"
PRINTER_HTTP="$(free_port)"
PORT="$FDM_PORT" "$FDM_BIN" >"$OUTDIR/fdm-mock.log" 2>&1 &
FDM_PID=$!
PORT="$PRINTER_PORT" HTTP_PORT="$PRINTER_HTTP" "$PRINTER_BIN" >"$OUTDIR/printer-mock.log" 2>&1 &
PRINTER_PID=$!
if wait_port "$FDM_PORT" 20 && wait_port "$PRINTER_PORT" 20 && wait_port "$PRINTER_HTTP" 20; then
    say mocks PASS "fdm $FDM_PORT, printer $PRINTER_PORT, jobs http $PRINTER_HTTP"
else
    abort mocks "a mock does not accept connections, see $OUTDIR/*-mock.log"
fi

adbd reverse "tcp:$FDM_DEV_PORT" "tcp:$FDM_PORT" >/dev/null && adbd reverse "tcp:$PRINTER_DEV_PORT" "tcp:$PRINTER_PORT" >/dev/null &&
    REVERSED=1
if [[ "$REVERSED" -ne 1 ]]; then
    REVERSED=1 # remove whatever of the two rules exists
    abort reverse "adb reverse failed"
fi

# A sleeping headless emulator holds back the network of the tunnel and of the app: wake the screen
# (no setting is changed; the activity keeps it on).
adbd shell "input keyevent KEYCODE_WAKEUP; wm dismiss-keyguard" >/dev/null 2>&1
sleep 1

# The tunnel must carry data, not only accept a connection: ask the FDM mock for a plain HTTP answer
# through the device port (stdin stays open one second so that nc reads the answer).
tunnel_ok() {
    adbd shell "(printf 'GET /graphql HTTP/1.0\\r\\n\\r\\n'; sleep 1) | nc -w 3 127.0.0.1 $FDM_DEV_PORT" 2>/dev/null | grep -q '^HTTP/'
}
tunnel=0
for _ in 1 2 3; do
    if tunnel_ok; then tunnel=1; break; fi
done
[[ "$tunnel" -eq 1 ]] || abort reverse "the FDM mock does not answer through the device port $FDM_DEV_PORT"
say reverse PASS "device $FDM_DEV_PORT -> $FDM_PORT, $PRINTER_DEV_PORT -> $PRINTER_PORT, answered through the tunnel"

# ── install and run ─────────────────────────────────────────────
out="$(adbd install -r "$APK_SIGNED" 2>&1)"
if [[ "$out" == *INSTALL_FAILED_UPDATE_INCOMPATIBLE* ]]; then
    # An older build was signed by another key: uninstall only the smoke package and retry.
    adbd uninstall "$PKG" >/dev/null 2>&1
    out="$(adbd install -r "$APK_SIGNED" 2>&1)"
fi
if [[ "$out" == *Success* ]]; then
    INSTALLED=1
    say install PASS "$PKG"
else
    abort install "$out"
fi

start="$(adbd shell date +'%m-%d\ %H:%M:%S.000' | tr -d '\r')"
adbd shell am force-stop "$PKG" >/dev/null 2>&1
if ! adbd shell am start -W -n "$PKG/.SmokeActivity" --ez run true --ei fdmPort "$FDM_DEV_PORT" --ei printerPort "$PRINTER_DEV_PORT" \
    >"$OUTDIR/am-start.out" 2>&1 || grep -q "Error" "$OUTDIR/am-start.out"; then
    abort start "$(cat "$OUTDIR/am-start.out")"
fi
say start PASS "SmokeActivity launched (log since $start)"

PIDS=""
total_line=""
deadline=$(($(date +%s) + TOTAL_TIMEOUT))
while [[ "$(date +%s)" -lt "$deadline" ]]; do
    p="$(adbd shell pidof "$PKG" 2>/dev/null | tr -d '\r')"
    [[ -z "$p" || " $PIDS " == *" $p "* ]] || PIDS="$PIDS $p"
    adbd logcat -d -v raw -T "$start" -s PiliSmoke:I >"$OUTDIR/smoke.log" 2>/dev/null
    total_line="$(grep -E '^SMOKE \| total \| ' "$OUTDIR/smoke.log" | tail -n 1)"
    [[ -z "$total_line" ]] || break
    sleep 3
done

# Every line of the device, as printed by the app.
grep -E '^SMOKE \| ' "$OUTDIR/smoke.log" | grep -v '^SMOKE | total | '
if [[ -z "$total_line" ]]; then
    say scenario FAIL "no total line within $TOTAL_TIMEOUT s"
elif [[ "$total_line" == *"| PASS |"* ]]; then
    say scenario PASS "${total_line#SMOKE | total | PASS | }"
else
    say scenario FAIL "${total_line#SMOKE | total | FAIL | }"
fi
for s in ensure-initialized version logging install-db open events login fdm-status sale z-report shutdown; do
    if ! grep -qE "^SMOKE \| $s \| PASS \|" "$OUTDIR/smoke.log"; then
        say "step-$s" FAIL "no PASS line on the device"
    fi
done

# ── printer jobs and R8 problems ────────────────────────────────
jobs_json="$(curl -s --max-time 5 "http://127.0.0.1:$PRINTER_HTTP/jobs" 2>/dev/null || true)"
njobs="$(printf '%s' "$jobs_json" | grep -o '"text"' | wc -l | tr -d ' ')"
if [[ "${njobs:-0}" -ge 1 ]]; then
    say printer-jobs PASS "$njobs job(s) on the printer mock"
else
    say printer-jobs FAIL "the printer mock holds no job"
fi

p="$(adbd shell pidof "$PKG" 2>/dev/null | tr -d '\r')"
[[ -z "$p" || " $PIDS " == *" $p "* ]] || PIDS="$PIDS $p"
adbd logcat -d -v threadtime -T "$start" >"$OUTDIR/logcat-full.log" 2>/dev/null
bad="$(python3 -I - "$OUTDIR/logcat-full.log" "$PKG" "$PIDS" <<'PY'
import re
import sys

path, pkg, pids = sys.argv[1], sys.argv[2], set(sys.argv[3].split())
needles = ("UnsatisfiedLinkError", "ClassNotFoundException", "NoSuchMethodError")
lines = open(path, errors="replace").read().splitlines()
hits = []
for i, ln in enumerate(lines):
    m = re.match(r"^\S+\s+\S+\s+(\d+)\s+\d+\s+\w\s+(.*)$", ln)
    pid = m.group(1) if m else ""
    ours = pid in pids or pkg in ln
    if "FATAL EXCEPTION" in ln:
        window = "\n".join(lines[i : i + 4])
        if pid in pids or ("Process: " + pkg) in window:
            hits.append(ln.strip())
    elif ours and any(n in ln for n in needles):
        hits.append(ln.strip())
print("\n".join(hits[:5]))
PY
)"
if [[ -z "$bad" ]]; then
    say r8-crashes PASS "no FATAL EXCEPTION, UnsatisfiedLinkError, ClassNotFoundException or NoSuchMethodError in the smoke process"
else
    printf '%s\n' "$bad" >&2
    say r8-crashes FAIL "$(printf '%s' "$bad" | head -n 1)"
fi

finish
