#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - UC scenarios on the emulator, Android journals (P7-T4, P7D6 to P7D8)
#
# Usage: scripts/android/uc_e2e.sh [--group G]... [--only STEM] [--out DIR] [--keep]
#   --group G    run the scenario group G (a directory of android/app/src/androidTest/assets/uc/, for
#                example 00_smoke); repeatable. Default: every group, in lexical order.
#   --only STEM  run only the scenario whose file stem is STEM (for example 001_SMOKE).
#   --out DIR    journal directory (default target/android-uc-e2e/journals); a journal is written to
#                DIR/android/<group>/<scenario>.json.
#   --keep       skip the teardown (owner, packages, reverse rules and mocks stay; for debugging).
#
# Runs the UC scenarios (P7.md section 4) against the real app on sunmi_d3_pro_api33 and the host mocks,
# and writes one Android journal per scenario (format 1, side android, scripts/android/uc_journal.py). One
# line per check and per scenario, then one total line:
#     UCE2E | <check or group/scenario> | PASS|FAIL | <detail>
#     UCE2E | total | PASS|FAIL | <n> failed (<m> checks)
# Exit 1 on any FAIL. Checks, in order: emulator, precheck, build, mocks, then per group <group>:provision and
# the scenarios of the group, and teardown. A scenario that fails does not stop the others; an infrastructure
# check that fails does (nothing is half done on top of a failure), but teardown always runs.
#
# The device is made device owner like app_e2e.sh does (the bundle import refuses a till that is not the
# device owner, and the hardware serial is only readable by it). Each group starts from a fresh install,
# a new development bundle (make_dev_bundle.sh --seed, in a temporary directory outside the repository),
# the import through UcProvisionTest, and a fresh pair of host mocks (the FDM counters start again with the
# empty database). Before each scenario the host clears the FDM request log and the printer jobs and
# resets both mock controls; UcScenarioTest runs it (`am instrument`, never a Gradle connected task: a device
# owner cannot be uninstalled by Gradle, PD113); the host pulls the steps file, reads GET /mock/requests
# and GET /jobs and assembles the journal.
#
# The script takes the P7 emulator lock (p7_emulator_lock.sh) and aborts when that file is absent. The mocks
# run on free host ports; the device reaches them on 19380 (FDM), 19390 (printer) and 19391 (printer HTTP jobs)
# through adb reverse. Teardown (an EXIT trap, skipped with --keep): the debug teardown broadcast releases the
# kiosk and clears the device owner, both packages are uninstalled, this script's three reverse rules are
# removed, the mocks are stopped, the temporary directories are deleted and the lock is released. It only acts
# on what this run did. The emulator is never stopped or wiped.
#
# Environment: ANDROID_HOME, ANDROID_NDK_HOME, JAVA_HOME (JDK 17), cargo on PATH. PILI_GRADLE_OFFLINE=0 lets
# Gradle go online (default: --offline). UC_E2E_SCENARIO_TIMEOUT (seconds, default 300) bounds one scenario.
# Logs: target/android-uc-e2e/.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

SELF="${BASH_SOURCE[0]}"
ROOT="$(cd "$(dirname "$SELF")/../.." && pwd -P)"
SELF="$ROOT/scripts/android/uc_e2e.sh"

usage_text() { sed -n '2,/^# ──*$/p' "$SELF" | sed '$d; s/^# \{0,1\}//'; }
usage() { echo "usage: uc_e2e.sh [--group G]... [--only STEM] [--out DIR] [--keep]" >&2; exit 2; }

ORIG_ARGS=("$@")
KEEP=0
ONLY=""
OUT_ARG=""
WANT_GROUPS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
    -h | --help) usage_text; exit 0 ;;
    --keep) KEEP=1; shift ;;
    --group) [[ $# -ge 2 && -n "$2" ]] || usage; WANT_GROUPS+=("$2"); shift 2 ;;
    --only) [[ $# -ge 2 && -n "$2" ]] || usage; ONLY="$2"; shift 2 ;;
    --out) [[ $# -ge 2 && -n "$2" ]] || usage; OUT_ARG="$2"; shift 2 ;;
    *) usage ;;
    esac
done

cd "$ROOT" || exit 2
ASSETS="$ROOT/android/app/src/androidTest/assets/uc"

# ── the P7 lock (before any device access and before our own traps) ──
LOCKLIB="$ROOT/scripts/android/p7_emulator_lock.sh"
if [[ ! -f "$LOCKLIB" ]]; then
    printf 'UCE2E | lock | FAIL | %s is absent: the P7 emulator lock (P7-T1) is needed to share the emulator\n' "$LOCKLIB"
    printf 'UCE2E | total | FAIL | 1 failed (1 checks)\n'
    exit 1
fi
# shellcheck source=/dev/null
source "$LOCKLIB"
p7_emulator_lock uc_e2e "$SELF" ${ORIG_ARGS[@]+"${ORIG_ARGS[@]}"}
p7_emulator_lock_note uc_e2e

# Same build-speed settings as every other cargo or Gradle run of the chain.
export RUSTC_WRAPPER="${RUSTC_WRAPPER:-/opt/homebrew/bin/sccache}"
export CARGO_INCREMENTAL=0
export SCCACHE_CACHE_SIZE="${SCCACHE_CACHE_SIZE:-25G}"
unset CARGO_TARGET_DIR
ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
export ANDROID_HOME
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.1.12297006}"
if [[ -z "${JAVA_HOME:-}" ]] && [[ -x /usr/libexec/java_home ]]; then
    JAVA_HOME="$(/usr/libexec/java_home -v 17 2>/dev/null || true)"
    [[ -z "$JAVA_HOME" ]] || export JAVA_HOME
fi
ADB="$ANDROID_HOME/platform-tools/adb"
[[ -x "$ADB" ]] || ADB="$(command -v adb || true)"

PKG=dev.nuagemagique.pilipos.debug
TEST_PKG=dev.nuagemagique.pilipos.debug.test
DEFAULT_RUNNER=androidx.test.runner.AndroidJUnitRunner
ADMIN="$PKG/dev.nuagemagique.pilipos.provisioning.PiliAdminReceiver"
MAIN="$PKG/dev.nuagemagique.pilipos.MainActivity"
TEARDOWN_ACTION=dev.nuagemagique.pilipos.debug.TEARDOWN
TEARDOWN_COMPONENT="$PKG/dev.nuagemagique.pilipos.debugtools.DebugTeardownReceiver"
INBOX="/sdcard/Android/data/$PKG/files/provisioning-inbox"
STEPS_DIR="/sdcard/Android/data/$PKG/files/uc/out"
FDM_DEV_PORT=19380
PRINTER_DEV_PORT=19390
PRINTER_HTTP_DEV_PORT=19391
MOCKS="$ROOT/target/android-mocks/debug"
LOGDIR="$ROOT/target/android-uc-e2e"
APP_APK="$ROOT/android/app/build/outputs/apk/debug/app-debug.apk"
TEST_APK="$ROOT/android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk"
SCENARIO_TIMEOUT="${UC_E2E_SCENARIO_TIMEOUT:-300}"
JOURNALER="$ROOT/scripts/android/uc_journal.py"

case "$OUT_ARG" in
"") OUT="$LOGDIR/journals" ;;
/*) OUT="$OUT_ARG" ;;
*) OUT="$ROOT/$OUT_ARG" ;;
esac

SERIAL=""
FDM_PID=""
PRINTER_PID=""
FDM_PORT=""
PRINTER_PORT=""
PRINTER_HTTP=""
WORK_DIR=""
RUNNER="$TEST_PKG/$DEFAULT_RUNNER"
OWNER_SET=0
INSTALLED=0
STARTED_DEVICE=0 # 1 once this run changed the device (installed a package): the teardown then verifies a clean device
RESET_PROBLEMS=()
REVERSED=0
FAILED=0
CHECKS=0

adbd() { "$ADB" -s "$SERIAL" "$@"; }

note() { printf 'uc_e2e: %s\n' "$*" >&2; }

pass() { # <check> <detail>
    CHECKS=$((CHECKS + 1))
    printf 'UCE2E | %s | PASS | %s\n' "$1" "$2"
}

fail() { # <check> <detail>
    CHECKS=$((CHECKS + 1))
    FAILED=$((FAILED + 1))
    printf 'UCE2E | %s | FAIL | %s\n' "$1" "$2"
}

# ── device state ────────────────────────────────────────────────
owners() { adbd shell dpm list-owners 2>/dev/null | tr -d '\r'; }

has_owner_ours() { owners | grep -q "$PKG"; }

# no_owner: dpm list-owners names no admin at all ('no owners').
no_owner() { ! owners | grep -qE 'admin=|DeviceOwner|ProfileOwner'; }

has_package() { adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -qx "package:$1"; }

# device_reset: clears the device owner this run set and uninstalls both packages; what it could not do goes
# to RESET_PROBLEMS (this shell's variables change, so it is never called in a subshell).
device_reset() {
    local pkg out i
    RESET_PROBLEMS=()
    [[ -n "$SERIAL" ]] || return 0
    if [[ "$OWNER_SET" -eq 1 ]] && has_package "$PKG"; then
        # The app releases the kiosk (Lock Task, suspended launchers, persistent HOME) and clears the owner role.
        adbd shell am broadcast -a "$TEARDOWN_ACTION" -n "$TEARDOWN_COMPONENT" >"$LOGDIR/teardown-broadcast.log" 2>&1
        for i in 1 2 3 4 5 6 7 8 9 10; do
            has_owner_ours || break
            sleep 1
        done
        if has_owner_ours; then
            RESET_PROBLEMS+=("device owner still set: $(owners | tr '\n' ' ')")
        else
            OWNER_SET=0
        fi
    fi
    if [[ "$INSTALLED" -eq 1 ]]; then
        for pkg in "$TEST_PKG" "$PKG"; do
            if has_package "$pkg"; then
                out=""
                # the policy manager may still hold the uninstall block for a moment after the owner is cleared
                for i in 1 2 3 4 5 6; do
                    out="$(adbd uninstall "$pkg" 2>&1 | tr -d '\r')"
                    [[ "$out" == *Success* ]] && break
                    sleep 2
                done
                [[ "$out" == *Success* ]] || RESET_PROBLEMS+=("uninstall $pkg: $out")
            fi
        done
        if ! has_package "$PKG" && ! has_package "$TEST_PKG"; then INSTALLED=0; fi
    fi
    return 0
}

stop_mocks() {
    [[ -z "$FDM_PID" ]] || { kill "$FDM_PID" 2>/dev/null; wait "$FDM_PID" 2>/dev/null; }
    [[ -z "$PRINTER_PID" ]] || { kill "$PRINTER_PID" 2>/dev/null; wait "$PRINTER_PID" 2>/dev/null; }
    FDM_PID=""
    PRINTER_PID=""
}

# finish_teardown: the rest of the teardown after device_reset, then the pass or fail line.
finish_teardown() {
    local problems=() port
    [[ "${#RESET_PROBLEMS[@]}" -eq 0 ]] || problems=("${RESET_PROBLEMS[@]}")
    if [[ -n "$SERIAL" && "$REVERSED" -eq 1 ]]; then
        for port in "$FDM_DEV_PORT" "$PRINTER_DEV_PORT" "$PRINTER_HTTP_DEV_PORT"; do
            adbd reverse --remove "tcp:$port" >/dev/null 2>&1
        done
    fi
    stop_mocks
    if [[ -n "$SERIAL" && "$STARTED_DEVICE" -eq 1 ]]; then
        no_owner || problems+=("dpm list-owners still names an owner: $(owners | tr '\n' ' ')")
        if adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -q 'pilipos'; then
            problems+=("a pilipos package is still installed")
        fi
        if [[ "$REVERSED" -eq 1 ]] && adbd reverse --list 2>/dev/null | grep -qE "tcp:($FDM_DEV_PORT|$PRINTER_DEV_PORT|$PRINTER_HTTP_DEV_PORT)\b"; then
            problems+=("an adb reverse rule is still set")
        fi
    fi
    [[ -z "$WORK_DIR" ]] || rm -rf "$WORK_DIR"
    if [[ "${#problems[@]}" -eq 0 ]]; then
        pass teardown "owner cleared, packages uninstalled, reverse rules removed, mocks stopped, temporary files deleted"
    else
        fail teardown "$(printf '%s; ' "${problems[@]}")"
    fi
}

finish() {
    local rc=$?
    trap - EXIT INT TERM
    if [[ "$KEEP" -eq 1 ]]; then
        printf 'UCE2E | teardown | SKIP | --keep: owner, packages, reverse rules and mocks stay (mocks stop with this script)\n'
    else
        device_reset
        finish_teardown
    fi
    p7_emulator_lock_cleanup
    if [[ "$FAILED" -eq 0 && "$rc" -eq 0 ]]; then
        printf 'UCE2E | total | PASS | 0 failed (%d checks)\n' "$CHECKS"
        exit 0
    fi
    [[ "$FAILED" -gt 0 ]] || FAILED=1
    printf 'UCE2E | total | FAIL | %d failed (%d checks)\n' "$FAILED" "$CHECKS"
    exit 1
}

trap finish EXIT
trap 'exit 130' INT TERM

abort() { # <check> <detail>: record a FAIL and stop (the trap tears down what was done)
    fail "$1" "$2"
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

# run_timeout <seconds> <command...>: exit 124 when the command had to be killed.
run_timeout() {
    local secs="$1" pid end
    shift
    "$@" &
    pid=$!
    end=$(($(date +%s) + secs))
    while kill -0 "$pid" 2>/dev/null; do
        if [[ "$(date +%s)" -ge "$end" ]]; then
            kill "$pid" 2>/dev/null
            sleep 1
            kill -9 "$pid" 2>/dev/null
            wait "$pid" 2>/dev/null
            return 124
        fi
        sleep 1
    done
    wait "$pid"
}

mkdir -p "$LOGDIR" "$OUT"
[[ -n "$ADB" && -x "$ADB" ]] || abort emulator "adb not found (ANDROID_HOME=$ANDROID_HOME)"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/pili-uc-e2e.XXXXXX")" || abort emulator "mktemp failed"
WORK_DIR="$(cd "$WORK_DIR" && pwd -P)"

# ── which scenarios ─────────────────────────────────────────────
[[ -d "$ASSETS" ]] || abort emulator "no scenario directory: $ASSETS"
GROUP_LIST=()
if [[ "${#WANT_GROUPS[@]}" -gt 0 ]]; then
    for g in "${WANT_GROUPS[@]}"; do
        [[ -d "$ASSETS/$g" ]] || abort emulator "unknown group '$g' (not a directory of $ASSETS)"
        GROUP_LIST+=("$g")
    done
else
    while IFS= read -r g; do
        [[ -z "$g" ]] || GROUP_LIST+=("$g")
    done < <(cd "$ASSETS" && find . -mindepth 1 -maxdepth 1 -type d | sed 's#^\./##' | sort)
fi
[[ "${#GROUP_LIST[@]}" -gt 0 ]] || abort emulator "no scenario group in $ASSETS"

scenarios_of() { # <group>: file stems, lexical, filtered by --only
    local f stem
    while IFS= read -r f; do
        stem="$(basename "$f" .json)"
        if [[ -z "$ONLY" || "$ONLY" == "$stem" ]]; then echo "$stem"; fi
    done < <(find "$ASSETS/$1" -maxdepth 1 -type f -name '*.json' | sort)
}
total_scenarios=0
for g in "${GROUP_LIST[@]}"; do
    n="$(scenarios_of "$g" | grep -c .)"
    total_scenarios=$((total_scenarios + n))
done
[[ "$total_scenarios" -gt 0 ]] || abort emulator "no scenario selected (--only '$ONLY')"

# ── emulator ────────────────────────────────────────────────────
SERIAL="$(bash scripts/android/emulator.sh ensure 2>"$LOGDIR/emulator.err")" || {
    cat "$LOGDIR/emulator.err" >&2
    SERIAL=""
    abort emulator "emulator.sh ensure failed (exit 3 means another AVD runs)"
}
export ANDROID_SERIAL="$SERIAL"
pass emulator "$SERIAL"
# A sleeping headless emulator holds back the network: wake the screen (no setting is changed).
adbd shell "input keyevent KEYCODE_WAKEUP; wm dismiss-keyguard" >/dev/null 2>&1

# ── precheck ────────────────────────────────────────────────────
owners_now="$(owners)"
users_now="$(adbd shell pm list users 2>/dev/null | tr -d '\r' | grep -c 'UserInfo{')"
if ! no_owner; then
    abort precheck "the device already has an owner, left alone: $(echo "$owners_now" | tr '\n' ' ')"
elif [[ "$users_now" -ne 1 ]]; then
    abort precheck "$users_now users on the device, one expected; left alone"
elif adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -q 'dev.nuagemagique.pilipos'; then
    abort precheck "a pilipos package is already installed (an earlier run?): $(adbd shell pm list packages | grep pilipos | tr -d '\r' | tr '\n' ' ')"
fi
pass precheck "no device owner, one user, no pilipos package"

# ── build ───────────────────────────────────────────────────────
offline=(--offline)
[[ "${PILI_GRADLE_OFFLINE:-1}" == "0" ]] && offline=()
jvmargs=()
[[ -z "${PILI_GRADLE_JVMARGS:-}" ]] || jvmargs=("-Dorg.gradle.jvmargs=$PILI_GRADLE_JVMARGS")
note "gradle :app:assembleDebug :app:assembleDebugAndroidTest (log $LOGDIR/gradle.log)"
(
    cd android &&
        bash ../scripts/android/heavy.sh ./gradlew --console=plain ${offline[@]+"${offline[@]}"} ${jvmargs[@]+"${jvmargs[@]}"} \
            :app:assembleDebug :app:assembleDebugAndroidTest
) >"$LOGDIR/gradle.log" 2>&1 || {
    grep -E "FAILED|What went wrong|error:|e: " "$LOGDIR/gradle.log" | head -n 20 >&2
    abort build "gradle failed, see $LOGDIR/gradle.log"
}
[[ -f "$APP_APK" && -f "$TEST_APK" ]] || abort build "APK missing: $APP_APK or $TEST_APK"
pass build "app-debug.apk and app-debug-androidTest.apk"

# ── mocks ───────────────────────────────────────────────────────
if [[ ! -x "$MOCKS/pos-fdm-mock" || ! -x "$MOCKS/pos-printer-mock" ]]; then
    bash scripts/android/heavy.sh cargo build --locked --manifest-path "$ROOT/Cargo.toml" -p pos-fdm-mock -p pos-printer-mock \
        --target-dir "$ROOT/target/android-mocks" >"$LOGDIR/mocks-build.log" 2>&1 ||
        abort mocks "cargo build of the mocks failed, see $LOGDIR/mocks-build.log"
fi
FDM_PORT="$(free_port)"
PRINTER_PORT="$(free_port)"
PRINTER_HTTP="$(free_port)"

start_mocks() { # fresh mocks on the same host ports: the FDM counters and the printer jobs start again
    PORT="$FDM_PORT" "$MOCKS/pos-fdm-mock" >>"$LOGDIR/fdm-mock.log" 2>&1 &
    FDM_PID=$!
    PORT="$PRINTER_PORT" HTTP_PORT="$PRINTER_HTTP" "$MOCKS/pos-printer-mock" >>"$LOGDIR/printer-mock.log" 2>&1 &
    PRINTER_PID=$!
    wait_port "$FDM_PORT" 20 && wait_port "$PRINTER_PORT" 20 && wait_port "$PRINTER_HTTP" 20
}

: >"$LOGDIR/fdm-mock.log"
: >"$LOGDIR/printer-mock.log"
start_mocks || abort mocks "a mock does not accept connections, see $LOGDIR/*-mock.log"
REVERSED=1 # whatever of the three rules exists is removed in the teardown
adbd reverse "tcp:$FDM_DEV_PORT" "tcp:$FDM_PORT" >/dev/null &&
    adbd reverse "tcp:$PRINTER_DEV_PORT" "tcp:$PRINTER_PORT" >/dev/null &&
    adbd reverse "tcp:$PRINTER_HTTP_DEV_PORT" "tcp:$PRINTER_HTTP" >/dev/null || abort mocks "adb reverse failed"
# The tunnel must carry data, not only accept a connection.
tunnel_ok=0
for _ in 1 2 3; do
    if adbd shell "(printf 'GET /graphql HTTP/1.0\\r\\n\\r\\n'; sleep 1) | nc -w 3 127.0.0.1 $FDM_DEV_PORT" 2>/dev/null | grep -q '^HTTP/'; then
        tunnel_ok=1
        break
    fi
done
[[ "$tunnel_ok" -eq 1 ]] || abort mocks "the FDM mock does not answer through the device port $FDM_DEV_PORT"
pass mocks "fdm $FDM_PORT, printer $PRINTER_PORT, jobs $PRINTER_HTTP; device ports $FDM_DEV_PORT, $PRINTER_DEV_PORT, $PRINTER_HTTP_DEV_PORT"

host_http() { # <method> <url> [body]: the body on stdout, non-zero on a transport or HTTP error
    if [[ $# -ge 3 ]]; then
        curl -sS -f --max-time 15 -X "$1" -H 'Content-Type: application/json' -d "$3" "$2"
    else
        curl -sS -f --max-time 15 -X "$1" "$2"
    fi
}

reset_mocks() { # before a scenario: empty request log and job list, default controls
    host_http DELETE "http://127.0.0.1:$FDM_PORT/mock/requests" >/dev/null &&
        host_http POST "http://127.0.0.1:$FDM_PORT/mock/control" '{}' >/dev/null &&
        host_http DELETE "http://127.0.0.1:$PRINTER_HTTP/jobs" >/dev/null &&
        host_http POST "http://127.0.0.1:$PRINTER_HTTP/control" '{}' >/dev/null
}

install_fresh() { # <group>
    local g="$1" apk out owner_out
    if has_package "$PKG" || has_package "$TEST_PKG"; then
        INSTALLED=1
        device_reset
        [[ "${#RESET_PROBLEMS[@]}" -eq 0 ]] || abort "$g:provision" "the previous group could not be removed: ${RESET_PROBLEMS[*]}"
    fi
    INSTALLED=1
    STARTED_DEVICE=1
    for apk in "$APP_APK" "$TEST_APK"; do
        out="$(adbd install -r "$apk" 2>&1 | tr -d '\r')"
        [[ "$out" == *Success* ]] || abort "$g:provision" "adb install $(basename "$apk"): $out"
    done
    # The exit code is 0 even when the OS refuses (S4): read the text. A just-installed package may need a moment.
    owner_out=""
    for _ in 1 2 3 4 5 6 7 8; do
        owner_out="$(adbd shell dpm set-device-owner "$ADMIN" 2>&1 | tr -d '\r')"
        [[ "$owner_out" == *Success* ]] && break
        sleep 3
    done
    [[ "$owner_out" == *Success* ]] || abort "$g:provision" "dpm set-device-owner refused: $(echo "$owner_out" | head -n 2 | tr '\n' ' ')"
    OWNER_SET=1
    RUNNER="$(adbd shell pm list instrumentation 2>/dev/null | tr -d '\r' | sed -n "s#^instrumentation:\\($TEST_PKG/[^ ]*\\) .*#\\1#p" | head -n 1)"
    [[ -n "$RUNNER" ]] || RUNNER="$TEST_PKG/$DEFAULT_RUNNER"
}

provision() { # <group>: launch, bundle, import through UcProvisionTest
    local g="$1" bundle bundle_out deadline found pushed f out instr
    adbd logcat -c >/dev/null 2>&1
    adbd shell am start -n "$MAIN" >"$LOGDIR/launch.log" 2>&1
    deadline=$(($(date +%s) + 60))
    found=0
    while [[ "$(date +%s)" -lt "$deadline" ]]; do
        if adbd logcat -d -s PiliShell:I 2>/dev/null | grep -q 'state=NotProvisioned'; then
            found=1
            break
        fi
        sleep 1
    done
    [[ "$found" -eq 1 ]] || abort "$g:provision" "no 'PiliShell state=NotProvisioned' within 60 s of the launch"

    bundle="$WORK_DIR/bundle-$g"
    bundle_out="$(bash scripts/android/make_dev_bundle.sh --seed --out "$bundle" \
        --fdm-url "http://127.0.0.1:$FDM_DEV_PORT/graphql" --printer "127.0.0.1:$PRINTER_DEV_PORT" 2>&1)" ||
        abort "$g:provision" "make_dev_bundle.sh failed: $(echo "$bundle_out" | tail -n 3 | tr '\n' ' ')"
    adbd shell mkdir -p "$INBOX" || abort "$g:provision" "mkdir $INBOX failed"
    pushed=0
    for f in "$bundle"/*; do
        out="$(adbd push "$f" "$INBOX/" 2>&1 | tr -d '\r')"
        [[ "$out" == *"1 file pushed"* ]] || abort "$g:provision" "adb push $(basename "$f"): $out"
        pushed=$((pushed + 1))
    done
    [[ "$pushed" -ge 3 ]] || abort "$g:provision" "only $pushed files in the bundle"

    note "am instrument UcProvisionTest ($g, log $LOGDIR/provision-$g.log)"
    run_timeout 240 adbd shell am instrument -w -r -e class dev.nuagemagique.pilipos.uc.UcProvisionTest "$RUNNER" >"$LOGDIR/provision-$g.log" 2>&1
    instr="$(tr -d '\r' <"$LOGDIR/provision-$g.log")"
    if echo "$instr" | grep -q '^OK (1 test)' && ! echo "$instr" | grep -qE 'INSTRUMENTATION_STATUS_CODE: -[0-9]|FAILURES!!!|shortMsg='; then
        pass "$g:provision" "fresh install, device owner, $pushed bundle files imported, engine Ready ($(adbd logcat -d -s PiliUcE2e:I 2>/dev/null | tr -d '\r' | sed -n 's/.*UCE2E_PROVISIONED //p' | tail -n 1))"
    else
        echo "$instr" | grep -E "stack=|Failed|AssertionError|Exception|FAILURES|Tests run" | head -n 12 >&2
        abort "$g:provision" "UcProvisionTest did not pass, see $LOGDIR/provision-$g.log"
    fi
}

run_scenario() { # <group> <scenario>
    local g="$1" s="$2" tag="$1__$2" log steps requests jobs jdir jout rc instr_ok detail summary
    log="$LOGDIR/instrument-$tag.log"
    steps="$WORK_DIR/$tag.steps.json"
    requests="$WORK_DIR/$tag.requests.json"
    jobs="$WORK_DIR/$tag.jobs.json"
    jdir="$OUT/android/$g"
    jout="$jdir/$s.json"
    mkdir -p "$jdir"
    rm -f "$jout"
    reset_mocks || { fail "$g/$s" "the host mocks could not be reset (curl failed)"; return 0; }
    adbd shell rm -f "$STEPS_DIR/$tag.steps.json" >/dev/null 2>&1
    adbd logcat -c >/dev/null 2>&1
    run_timeout "$SCENARIO_TIMEOUT" adbd shell am instrument -w -r \
        -e class dev.nuagemagique.pilipos.uc.UcScenarioTest \
        -e group "$g" -e scenario "$s" -e fdmPort "$FDM_DEV_PORT" -e printerHttpPort "$PRINTER_HTTP_DEV_PORT" \
        "$RUNNER" >"$log" 2>&1
    rc=$?
    instr_ok=0
    if [[ "$rc" -eq 0 ]] && tr -d '\r' <"$log" | grep -q '^OK (1 test)' &&
        ! tr -d '\r' <"$log" | grep -qE 'INSTRUMENTATION_STATUS_CODE: -[0-9]|FAILURES!!!|shortMsg='; then
        instr_ok=1
    fi
    adbd logcat -d -s PiliUcE2e:I PiliUcE2e:W >"$WORK_DIR/$tag.logcat" 2>&1
    if ! adbd pull "$STEPS_DIR/$tag.steps.json" "$steps" >/dev/null 2>&1; then
        detail="no steps file on the device (instrument exit $rc): $(tr -d '\r' <"$log" | grep -E 'shortMsg=|Error|Exception|FAILURES' | head -n 2 | tr '\n' ' ')"
        fail "$g/$s" "$detail"
        return 0
    fi
    host_http GET "http://127.0.0.1:$FDM_PORT/mock/requests" >"$requests" || { fail "$g/$s" "GET /mock/requests failed"; return 0; }
    host_http GET "http://127.0.0.1:$PRINTER_HTTP/jobs" >"$jobs" || { fail "$g/$s" "GET /jobs of the printer mock failed"; return 0; }
    summary="$(python3 -I "$JOURNALER" --steps "$steps" --requests "$requests" --jobs "$jobs" --out "$jout" --detail 2>"$WORK_DIR/$tag.journal.err")"
    if [[ $? -ne 0 ]]; then
        fail "$g/$s" "uc_journal.py: $(tr '\n' ' ' <"$WORK_DIR/$tag.journal.err")"
        return 0
    fi
    if [[ "$instr_ok" -eq 1 && "$summary" == "JOURNAL PASS "* ]]; then
        pass "$g/$s" "${summary#JOURNAL PASS } -> ${jout#"$ROOT"/}"
    else
        fail "$g/$s" "${summary#JOURNAL }; $(tr '\n' ' ' <"$WORK_DIR/$tag.journal.err")$(grep UCE2E_STEP_FAILED "$WORK_DIR/$tag.logcat" | head -n 1 | sed 's/.*UCE2E_STEP_FAILED/step/')"
    fi
    return 0
}

# ── groups ──────────────────────────────────────────────────────
group_index=0
for g in "${GROUP_LIST[@]}"; do
    scenario_list="$(scenarios_of "$g")"
    [[ -n "$scenario_list" ]] || continue
    if [[ "$group_index" -gt 0 ]]; then
        # fresh FDM counters for the fresh database
        stop_mocks
        start_mocks || abort mocks "the mocks did not come back for group $g, see $LOGDIR/*-mock.log"
    fi
    group_index=$((group_index + 1))
    install_fresh "$g"
    provision "$g"
    while IFS= read -r s; do
        [[ -z "$s" ]] || run_scenario "$g" "$s"
    done <<<"$scenario_list"
done

# teardown and the total line come from the EXIT trap
exit 0
