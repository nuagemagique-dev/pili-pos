#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - the soak of the Android port (P7-T5, P7D12)
#
# Usage:
#   scripts/android/p7_soak.sh start  --hours H [--day-minutes M] [--sales-per-day N] [--run-id ID]
#   scripts/android/p7_soak.sh report [--run ID | --latest] [--min-hours H]
#   scripts/android/p7_soak.sh stop   [--run ID]
#   scripts/android/p7_soak.sh status
#
# start    takes the P7 emulator lock, makes sure the emulator runs (emulator.sh ensure, never
#          wiped), prechecks it (no device owner, one user, no pilipos package), builds the debug
#          and androidTest APKs, builds and starts pos-fdm-mock and pos-printer-mock, provisions
#          the app exactly as app_e2e.sh does (device owner, development bundle, adb reverse on
#          19380, 19390, 19391, ProvisioningE2eTest), keeps the screen awake (svc power stayon
#          true, restored at the end) and starts the driver (SoakDriverTest) with `am instrument`
#          WITHOUT -w, so that it survives an adb disconnect. A detached supervisor (its own
#          process group, wrapped in caffeinate -i -w <pid> so the Mac stays awake) keeps the lock,
#          runs soak_sampler.py and tears down at the end. start returns after the first sample.
#          H is decimal (0.34 = 20 minutes). Defaults: 60 minutes per compressed day, 100 sales a
#          day. The driver plays: login (clock-in), the sales (70 percent takeaway paid in cash,
#          20 percent table with order, pre-bill and card, 5 percent split payment, 5 percent
#          refund of an earlier sale), an X report at mid-day, one idle gap of 6 minutes in which
#          the engine locks the session, login again, work-out and the Z at the end of each day.
# report   prints 'SOAK | <check> | PASS|FAIL | <detail>' lines and 'SOAK | total | PASS|FAIL |
#          <n> failed (<m> checks)' for the run so far (it works while the run goes on). Checks:
#          pid-stable, no-crash, sales-signed, counters-continuous, z-done, memory, threads, fds,
#          beats, storage, min-hours (soak_sampler.py has the rules). Exit 1 on any FAIL.
#          --latest (the default) is the newest run; --min-hours H fails a run shorter than H.
# stop     writes the stop file, waits for the driver to wind down (its day is worked out and
#          closed with a Z), for the supervisor to tear down, and prints the final report.
# status   says whether a soak runs: its run id, elapsed time and the last checkpoint; else
#          'no soak runs'. Exit 0.
#
# Teardown (end of the run, a stop, or a failed start): the debug teardown broadcast releases the
# kiosk and the device owner, both packages are uninstalled, this run's three reverse rules are
# removed, the mocks are stopped, the temporary bundle is deleted, svc power stayon is put back to
# its value before the run, and the lock is released when the supervisor exits. The emulator is
# never stopped or wiped.
#
# State under target/android-soak/<run-id>/: state.json (serial, pids, ports, start, hours),
# samples.jsonl, checkpoints.jsonl, fdm_journal.jsonl, logcat_app.txt, report.txt, driver.jsonl (a
# copy of the driver's own file), journal_meta.json, and the build and mock logs.
#
# Environment: ANDROID_HOME, ANDROID_NDK_HOME, JAVA_HOME (JDK 17), cargo on PATH.
# PILI_GRADLE_OFFLINE=0 lets Gradle go online (default: --offline). PILI_SOAK_SEED fixes the
# random seed of the sale mix. PILI_SOAK_IDLE_GAP_MINUTES changes the idle gap (default 6).
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

SELF="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)/$(basename "${BASH_SOURCE[0]}")"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
SOAKROOT="$ROOT/target/android-soak"
SAMPLER="$ROOT/scripts/android/soak_sampler.py"

usage() { echo "usage: p7_soak.sh start --hours H [--day-minutes M] [--sales-per-day N] [--run-id ID] | report [--run ID | --latest] [--min-hours H] | stop [--run ID] | status" >&2; exit 2; }

CMD="${1:-}"
case "$CMD" in
-h | --help)
    sed -n '2,/^# ──*$/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
    exit 0
    ;;
start | report | stop | status | _supervise) shift ;;
*) usage ;;
esac

cd "$ROOT" || exit 2

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
export ADB

PKG=dev.nuagemagique.pilipos.debug
TEST_PKG=dev.nuagemagique.pilipos.debug.test
RUNNER="$TEST_PKG/androidx.test.runner.AndroidJUnitRunner"
ADMIN="$PKG/dev.nuagemagique.pilipos.provisioning.PiliAdminReceiver"
MAIN="$PKG/dev.nuagemagique.pilipos.MainActivity"
TEARDOWN_ACTION=dev.nuagemagique.pilipos.debug.TEARDOWN
TEARDOWN_COMPONENT="$PKG/dev.nuagemagique.pilipos.debugtools.DebugTeardownReceiver"
INBOX="/sdcard/Android/data/$PKG/files/provisioning-inbox"
SOAKDIR="/sdcard/Android/data/$PKG/files/soak"
FDM_DEV_PORT=19380
PRINTER_DEV_PORT=19390
PRINTER_HTTP_DEV_PORT=19391
MOCKS="$ROOT/target/android-mocks/debug"
APP_APK="$ROOT/android/app/build/outputs/apk/debug/app-debug.apk"
TEST_APK="$ROOT/android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk"
JVMARGS="${PILI_GRADLE_JVMARGS:--Xmx2g -XX:MaxMetaspaceSize=1g -Dfile.encoding=UTF-8}"

note() { printf 'p7_soak: %s\n' "$*" >&2; }

# ── state.json helpers (the JSON is read and written by soak_sampler.py) ─────────
state_get() { python3 -B -I "$SAMPLER" state "$1" get "$2" 2>/dev/null; }
state_set() { local f="$1"; shift; python3 -B -I "$SAMPLER" state "$f" set "$@"; }

# detach <logfile> <command...>: starts the command in its own session (no controlling terminal, no
# SIGHUP), output to the log, and prints its pid. Open descriptors above 2 are closed unless
# DETACH_KEEP_FDS=1 (the supervisor keeps the descriptor that holds the P7 lock).
detach() {
    local log="$1"
    shift
    python3 -B -I -c '
import os, sys
log = sys.argv[1]
keep = os.environ.get("DETACH_KEEP_FDS") == "1"
pid = os.fork()
if pid:
    print(pid)
    sys.exit(0)
os.setsid()
fd = os.open(log, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o644)
os.dup2(fd, 1)
os.dup2(fd, 2)
os.dup2(os.open(os.devnull, os.O_RDONLY), 0)
if not keep:
    os.closerange(3, 1024)
os.execvp(sys.argv[2], sys.argv[2:])
' "$log" "$@"
}

# alive <pid>: true when the process exists and its command line mentions $2 (any when empty)
alive() {
    [[ "$1" =~ ^[0-9]+$ ]] || return 1
    kill -0 "$1" 2>/dev/null || return 1
    [[ -z "${2:-}" ]] || ps -o command= -p "$1" 2>/dev/null | grep -q -- "$2"
}

# The id of the soak that runs, if any.
running_run() {
    local d f sup
    for d in "$SOAKROOT"/*/; do
        f="${d}state.json"
        [[ -f "$f" ]] || continue
        [[ "$(state_get "$f" status)" == running ]] || continue
        sup="$(state_get "$f" supervisor_pid)"
        if alive "$sup" p7_soak; then
            basename "$d"
            return 0
        fi
    done
    return 1
}

latest_run() { ls -1dt "$SOAKROOT"/*/ 2>/dev/null | head -n 1 | sed 's#/$##' | xargs -n 1 basename 2>/dev/null; }

# ── device helpers ──────────────────────────────────────────────
SERIAL=""
adbd() { "$ADB" -s "$SERIAL" "$@"; }
owners() { adbd shell dpm list-owners 2>/dev/null | tr -d '\r'; }
has_owner_ours() { owners | grep -q "$PKG"; }
no_owner() { ! owners | grep -qE 'admin=|DeviceOwner|ProfileOwner'; }
free_port() { python3 -I -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])'; }
accepts() { (exec 3<>"/dev/tcp/127.0.0.1/$1") 2>/dev/null; }
wait_port() { # <port> <seconds>
    local deadline=$(($(date +%s) + $2))
    until accepts "$1"; do
        [[ "$(date +%s)" -lt "$deadline" ]] || return 1
        sleep 0.3
    done
}

# teardown <run-dir>: undoes what start did, from the state file. Prints the problems (none: nothing)
# and returns 1 when something is left.
teardown_run() {
    local rd="$1" sf="$1/state.json" problems=() i out pkg port
    local serial owner_set installed reversed fdm_pid printer_pid bundle stayon
    serial="$(state_get "$sf" serial)"
    owner_set="$(state_get "$sf" owner_set)"
    installed="$(state_get "$sf" installed)"
    reversed="$(state_get "$sf" reversed)"
    fdm_pid="$(state_get "$sf" fdm_pid)"
    printer_pid="$(state_get "$sf" printer_pid)"
    bundle="$(state_get "$sf" bundle_dir)"
    stayon="$(state_get "$sf" stayon_before)"
    SERIAL="$serial"
    if [[ -n "$SERIAL" ]]; then
        if [[ "$owner_set" == "1" || "$owner_set" == "true" ]] && adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -qx "package:$PKG"; then
            adbd shell am broadcast -a "$TEARDOWN_ACTION" -n "$TEARDOWN_COMPONENT" >"$rd/teardown-broadcast.log" 2>&1
            for i in 1 2 3 4 5 6 7 8 9 10; do
                has_owner_ours || break
                sleep 1
            done
            has_owner_ours && problems+=("device owner still set: $(owners | tr '\n' ' ')")
        fi
        if [[ "$installed" == "1" || "$installed" == "true" ]]; then
            for pkg in "$TEST_PKG" "$PKG"; do
                if adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -qx "package:$pkg"; then
                    out=""
                    for i in 1 2 3 4 5 6; do
                        out="$(adbd uninstall "$pkg" 2>&1 | tr -d '\r')"
                        [[ "$out" == *Success* ]] && break
                        sleep 2
                    done
                    [[ "$out" == *Success* ]] || problems+=("uninstall $pkg: $out")
                fi
            done
        fi
        if [[ "$reversed" == "1" || "$reversed" == "true" ]]; then
            for port in "$FDM_DEV_PORT" "$PRINTER_DEV_PORT" "$PRINTER_HTTP_DEV_PORT"; do
                adbd reverse --remove "tcp:$port" >/dev/null 2>&1
            done
        fi
        if [[ -n "$stayon" ]]; then
            adbd shell settings put global stay_on_while_plugged_in "$stayon" >/dev/null 2>&1
            [[ "$(adbd shell settings get global stay_on_while_plugged_in 2>/dev/null | tr -d '\r')" == "$stayon" ]] ||
                problems+=("stay_on_while_plugged_in not restored to $stayon")
        fi
    fi
    [[ -z "$fdm_pid" ]] || { kill "$fdm_pid" 2>/dev/null; }
    [[ -z "$printer_pid" ]] || { kill "$printer_pid" 2>/dev/null; }
    [[ -z "$bundle" ]] || rm -rf "$bundle"
    if [[ -n "$SERIAL" && ("$owner_set" == "1" || "$owner_set" == "true") ]]; then
        no_owner || problems+=("dpm list-owners still names an owner: $(owners | tr '\n' ' ')")
    fi
    if [[ -n "$SERIAL" ]] && adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -q 'pilipos'; then
        [[ "$installed" == "1" || "$installed" == "true" ]] && problems+=("a pilipos package is still installed")
    fi
    if [[ -n "$SERIAL" && ("$reversed" == "1" || "$reversed" == "true") ]]; then
        if adbd reverse --list 2>/dev/null | grep -qE "tcp:($FDM_DEV_PORT|$PRINTER_DEV_PORT|$PRINTER_HTTP_DEV_PORT)\b"; then
            problems+=("an adb reverse rule is still set")
        fi
    fi
    if [[ "${#problems[@]}" -eq 0 ]]; then
        return 0
    fi
    printf '%s; ' "${problems[@]}"
    return 1
}

# ══ start ═════════════════════════════════════════════════════════
RUN_ID=""
RUN=""
HANDOFF=0
START_PROBLEM=""

start_fail() { # <detail>
    START_PROBLEM="$1"
    exit 1
}

start_finish() {
    local rc=$? problems
    trap - EXIT INT TERM
    if [[ "$HANDOFF" -eq 0 && -n "$RUN" && -f "$RUN/state.json" ]]; then
        problems="$(teardown_run "$RUN")"
        state_set "$RUN/state.json" status=failed "teardown=${problems:-ok}" >/dev/null 2>&1
        [[ -z "$problems" ]] || note "teardown left: $problems"
    fi
    if [[ "$rc" -ne 0 ]]; then
        printf 'SOAK | start | FAIL | %s\n' "${START_PROBLEM:-start failed (exit $rc)}"
        exit 1
    fi
    exit 0
}

cmd_start() {
    local hours="" day_minutes=60 sales=100 seed gap warm
    while [[ $# -gt 0 ]]; do
        case "$1" in
        --hours) hours="${2:-}"; shift ;;
        --day-minutes) day_minutes="${2:-}"; shift ;;
        --sales-per-day) sales="${2:-}"; shift ;;
        --run-id) RUN_ID="${2:-}"; shift ;;
        *) usage ;;
        esac
        shift
    done
    [[ "$hours" =~ ^[0-9]+([.][0-9]+)?$ ]] || { echo "start: --hours H (decimal) is required" >&2; exit 2; }
    [[ "$day_minutes" =~ ^[0-9]+$ && "$sales" =~ ^[0-9]+$ ]] || { echo "start: --day-minutes and --sales-per-day are whole numbers" >&2; exit 2; }
    [[ -n "$RUN_ID" ]] || RUN_ID="soak-$(date +%Y%m%d-%H%M%S)"
    [[ "$RUN_ID" =~ ^[A-Za-z0-9._-]+$ ]] || { echo "start: --run-id may hold letters, digits, dot, dash and underscore" >&2; exit 2; }
    seed="${PILI_SOAK_SEED:-$((RANDOM * 32768 + RANDOM))}"
    gap="${PILI_SOAK_IDLE_GAP_MINUTES:-6}"

    local busy
    if busy="$(running_run)"; then
        printf 'SOAK | start | FAIL | a soak already runs: %s (p7_soak.sh stop)\n' "$busy"
        exit 1
    fi
    RUN="$SOAKROOT/$RUN_ID"
    if [[ -e "$RUN" ]]; then
        printf 'SOAK | start | FAIL | run %s exists already (%s), pick another --run-id\n' "$RUN_ID" "$RUN"
        exit 1
    fi

    # The lock is taken here, and kept by the supervisor after this process has returned.
    # shellcheck source=/dev/null
    source "$ROOT/scripts/android/p7_emulator_lock.sh"
    p7_emulator_lock p7_soak "$SELF" start --hours "$hours" --day-minutes "$day_minutes" --sales-per-day "$sales" --run-id "$RUN_ID"

    mkdir -p "$RUN" || exit 2
    RUN="$(cd "$RUN" && pwd -P)"
    trap start_finish EXIT
    trap 'exit 130' INT TERM
    [[ -n "$ADB" && -x "$ADB" ]] || start_fail "adb not found (ANDROID_HOME=$ANDROID_HOME)"
    state_set "$RUN/state.json" "run_id=\"$RUN_ID\"" status=starting "package=\"$PKG\"" "test_package=\"$TEST_PKG\"" \
        "hours=$hours" "day_minutes=$day_minutes" "sales_per_day=$sales" "seed=$seed" "start_pid=$$" >/dev/null || start_fail "cannot write state.json"
    # the memory slope needs an hour of warm-up on a long run; a short run keeps a quarter of its length
    warm="$(python3 -I -c "print(int(min(3600, $hours * 3600 / 4)))")"
    state_set "$RUN/state.json" "warmup_s=$warm" >/dev/null

    # emulator
    SERIAL="$(bash scripts/android/emulator.sh ensure 2>"$RUN/emulator.err")" || {
        cat "$RUN/emulator.err" >&2
        SERIAL=""
        start_fail "emulator.sh ensure failed (exit 3 means another AVD runs)"
    }
    export ANDROID_SERIAL="$SERIAL"
    state_set "$RUN/state.json" "serial=\"$SERIAL\"" >/dev/null
    adbd shell "input keyevent KEYCODE_WAKEUP; wm dismiss-keyguard" >/dev/null 2>&1
    note "emulator $SERIAL"

    # precheck
    local users_now
    users_now="$(adbd shell pm list users 2>/dev/null | tr -d '\r' | grep -c 'UserInfo{')"
    if ! no_owner; then
        start_fail "precheck: the device already has an owner, left alone: $(owners | tr '\n' ' ')"
    elif [[ "$users_now" -ne 1 ]]; then
        start_fail "precheck: $users_now users on the device, one expected; left alone"
    elif adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -q 'dev.nuagemagique.pilipos'; then
        start_fail "precheck: a pilipos package is already installed (an earlier run?)"
    fi

    # screen awake (restored in the teardown)
    local stayon
    stayon="$(adbd shell settings get global stay_on_while_plugged_in 2>/dev/null | tr -d '\r')"
    [[ "$stayon" =~ ^[0-9]+$ ]] || stayon=0
    state_set "$RUN/state.json" "stayon_before=$stayon" >/dev/null
    adbd shell svc power stayon true >/dev/null 2>&1

    # build
    local offline=(--offline)
    [[ "${PILI_GRADLE_OFFLINE:-1}" == "0" ]] && offline=()
    note "gradle :app:assembleDebug :app:assembleDebugAndroidTest (log $RUN/gradle.log)"
    (
        cd android &&
            bash ../scripts/android/heavy.sh ./gradlew --no-daemon --console=plain ${offline[@]+"${offline[@]}"} \
                "-Dorg.gradle.jvmargs=$JVMARGS" :app:assembleDebug :app:assembleDebugAndroidTest
    ) >"$RUN/gradle.log" 2>&1 || {
        grep -E "FAILED|What went wrong|error:|e: " "$RUN/gradle.log" | head -n 20 >&2
        start_fail "build: gradle failed, see $RUN/gradle.log"
    }
    [[ -f "$APP_APK" && -f "$TEST_APK" ]] || start_fail "build: APK missing: $APP_APK or $TEST_APK"

    # mocks
    if [[ ! -x "$MOCKS/pos-fdm-mock" || ! -x "$MOCKS/pos-printer-mock" ]]; then
        bash scripts/android/heavy.sh cargo build --locked --manifest-path "$ROOT/Cargo.toml" -p pos-fdm-mock -p pos-printer-mock \
            --target-dir "$ROOT/target/android-mocks" >"$RUN/mocks-build.log" 2>&1 ||
            start_fail "mocks: cargo build failed, see $RUN/mocks-build.log"
    fi
    local fdm_port printer_port printer_http fdm_pid printer_pid
    fdm_port="$(free_port)"
    printer_port="$(free_port)"
    printer_http="$(free_port)"
    fdm_pid="$(detach "$RUN/fdm-mock.log" env PORT="$fdm_port" "$MOCKS/pos-fdm-mock")"
    printer_pid="$(detach "$RUN/printer-mock.log" env PORT="$printer_port" HTTP_PORT="$printer_http" "$MOCKS/pos-printer-mock")"
    state_set "$RUN/state.json" "fdm_pid=$fdm_pid" "printer_pid=$printer_pid" "fdm_port=$fdm_port" "printer_port=$printer_port" \
        "printer_http_port=$printer_http" >/dev/null
    wait_port "$fdm_port" 20 && wait_port "$printer_port" 20 && wait_port "$printer_http" 20 ||
        start_fail "mocks: a mock does not accept connections, see $RUN/*-mock.log"
    state_set "$RUN/state.json" reversed=1 >/dev/null # whatever of the three rules exists is removed in the teardown
    adbd reverse "tcp:$FDM_DEV_PORT" "tcp:$fdm_port" >/dev/null &&
        adbd reverse "tcp:$PRINTER_DEV_PORT" "tcp:$printer_port" >/dev/null &&
        adbd reverse "tcp:$PRINTER_HTTP_DEV_PORT" "tcp:$printer_http" >/dev/null || start_fail "mocks: adb reverse failed"
    local tunnel_ok=0 _
    for _ in 1 2 3; do
        if adbd shell "(printf 'GET /graphql HTTP/1.0\\r\\n\\r\\n'; sleep 1) | nc -w 3 127.0.0.1 $FDM_DEV_PORT" 2>/dev/null | grep -q '^HTTP/'; then
            tunnel_ok=1
            break
        fi
    done
    [[ "$tunnel_ok" -eq 1 ]] || start_fail "mocks: the FDM mock does not answer through the device port $FDM_DEV_PORT"

    # bundle (outside the repository)
    local bundle bundle_out
    bundle="$(mktemp -d "${TMPDIR:-/tmp}/pili-soak-bundle.XXXXXX")" || start_fail "bundle: mktemp failed"
    bundle="$(cd "$bundle" && pwd -P)"
    state_set "$RUN/state.json" "bundle_dir=\"$bundle\"" >/dev/null
    bundle_out="$(bash scripts/android/make_dev_bundle.sh --seed --out "$bundle/bundle" \
        --fdm-url "http://127.0.0.1:$FDM_DEV_PORT/graphql" --printer "127.0.0.1:$PRINTER_DEV_PORT" 2>&1)" ||
        start_fail "bundle: make_dev_bundle.sh failed: $(echo "$bundle_out" | tail -n 3 | tr '\n' ' ')"

    # install and device owner
    state_set "$RUN/state.json" installed=1 >/dev/null
    local apk out owner_out
    for apk in "$APP_APK" "$TEST_APK"; do
        out="$(adbd install -r "$apk" 2>&1 | tr -d '\r')"
        [[ "$out" == *Success* ]] || start_fail "install: adb install $(basename "$apk"): $out"
    done
    owner_out=""
    for _ in 1 2 3 4 5 6 7 8; do
        owner_out="$(adbd shell dpm set-device-owner "$ADMIN" 2>&1 | tr -d '\r')"
        [[ "$owner_out" == *Success* ]] && break
        sleep 3
    done
    [[ "$owner_out" == *Success* ]] || start_fail "install: dpm set-device-owner refused: $(echo "$owner_out" | head -n 2 | tr '\n' ' ')"
    state_set "$RUN/state.json" owner_set=1 >/dev/null

    # launch, then push the bundle
    local since found deadline pushed f
    since="$(adbd shell date +'%m-%d\ %H:%M:%S.000' | tr -d '\r')"
    adbd shell am start -n "$MAIN" >"$RUN/launch.log" 2>&1
    deadline=$(($(date +%s) + 60))
    found=0
    while [[ "$(date +%s)" -lt "$deadline" ]]; do
        if adbd logcat -d -s PiliShell:I -T "$since" 2>/dev/null | grep -q 'state=NotProvisioned'; then
            found=1
            break
        fi
        sleep 1
    done
    [[ "$found" -eq 1 ]] || start_fail "launch: no 'PiliShell state=NotProvisioned' within 60 s of the launch"
    adbd shell mkdir -p "$INBOX" || start_fail "push: mkdir $INBOX failed"
    pushed=0
    for f in "$bundle"/bundle/*; do
        out="$(adbd push "$f" "$INBOX/" 2>&1 | tr -d '\r')"
        [[ "$out" == *"1 file pushed"* ]] || start_fail "push: adb push $(basename "$f"): $out"
        pushed=$((pushed + 1))
    done
    [[ "$pushed" -ge 3 ]] || start_fail "push: only $pushed files in the bundle"

    # provisioning, as app_e2e.sh does
    note "am instrument ProvisioningE2eTest (log $RUN/provision.log)"
    adbd shell am instrument -w -r -e class dev.nuagemagique.pilipos.e2e.ProvisioningE2eTest -e fdmPort "$FDM_DEV_PORT" "$RUNNER" \
        >"$RUN/provision.log" 2>&1
    local instr
    instr="$(tr -d '\r' <"$RUN/provision.log")"
    if ! { echo "$instr" | grep -q '^OK (1 test)' && ! echo "$instr" | grep -qE 'INSTRUMENTATION_STATUS_CODE: -[0-9]|FAILURES!!!|shortMsg='; }; then
        echo "$instr" | grep -E "stack=|Failed|AssertionError|Exception|FAILURES|Tests run" | head -n 12 >&2
        start_fail "provisioning: ProvisioningE2eTest did not pass, see $RUN/provision.log"
    fi

    # the system restarts the app (persistent HOME) when the instrumentation ends: let it settle and reach Ready
    local pid=""
    for _ in $(seq 1 30); do
        pid="$(adbd shell pidof "$PKG" | tr -d '\r' | awk '{print $1}')"
        [[ -n "$pid" ]] && break
        sleep 1
    done
    sleep 3
    pid="$(adbd shell pidof "$PKG" | tr -d '\r' | awk '{print $1}')"
    [[ -n "$pid" ]] || start_fail "provisioning: no running process for $PKG after the provisioning test"
    deadline=$(($(date +%s) + 40))
    found=0
    while [[ "$(date +%s)" -lt "$deadline" ]]; do
        if adbd logcat -d --pid="$pid" -s PiliShell:I 2>/dev/null | grep -q 'state=Ready'; then
            found=1
            break
        fi
        sleep 1
    done
    [[ "$found" -eq 1 ]] || start_fail "provisioning: pid $pid never logged state=Ready"

    # the driver: am instrument WITHOUT -w, it keeps running when adb goes away
    adbd shell rm -rf "$SOAKDIR" >/dev/null 2>&1
    local start_epoch
    start_epoch="$(date +%s)"
    note "starting the driver (hours $hours, day $day_minutes min, $sales sales a day, seed $seed)"
    adbd shell am instrument -r -e class dev.nuagemagique.pilipos.soak.SoakDriverTest -e hours "$hours" -e dayMinutes "$day_minutes" \
        -e salesPerDay "$sales" -e seed "$seed" -e idleGapMinutes "$gap" "$RUNNER" >"$RUN/instrument.log" 2>&1
    deadline=$(($(date +%s) + 150))
    found=0
    while [[ "$(date +%s)" -lt "$deadline" ]]; do
        if adbd exec-out "cat $SOAKDIR/driver.jsonl 2>/dev/null" | grep -q '"type":"start"'; then
            found=1
            break
        fi
        sleep 2
    done
    if [[ "$found" -ne 1 ]]; then
        adbd logcat -d -s PiliSoak:I AndroidRuntime:E 2>/dev/null | tail -n 20 >&2
        start_fail "driver: no start record in driver.jsonl within 150 s (am instrument: $(tr -d '\r' <"$RUN/instrument.log" | head -n 3 | tr '\n' ' '))"
    fi
    state_set "$RUN/state.json" "start=$start_epoch" >/dev/null

    # the supervisor: own session, keeps the lock descriptor, tears down at the end
    local sup
    sup="$(DETACH_KEEP_FDS=1 detach "$RUN/supervisor.log" bash "$SELF" _supervise --run-id "$RUN_ID" --start-pid "$$")"
    [[ "$sup" =~ ^[0-9]+$ ]] || start_fail "supervisor: could not start it"
    state_set "$RUN/state.json" "supervisor_pid=$sup" status=running >/dev/null
    detach /dev/null caffeinate -i -w "$sup" >/dev/null
    HANDOFF=1

    # start returns after the first sample
    deadline=$(($(date +%s) + 180))
    while [[ ! -s "$RUN/samples.jsonl" && "$(date +%s)" -lt "$deadline" ]]; do
        alive "$sup" p7_soak || break
        sleep 2
    done
    if [[ ! -s "$RUN/samples.jsonl" ]]; then
        printf 'SOAK | start | FAIL | no first sample within 180 s; the supervisor (pid %s) tears the run down, see %s/supervisor.log\n' "$sup" "$RUN"
        kill -TERM "$sup" 2>/dev/null
        trap - EXIT INT TERM
        exit 1
    fi
    trap - EXIT INT TERM
    printf 'SOAK | start | PASS | run %s on %s: %s h, %s min a day, %s sales a day; supervisor pid %s, state %s\n' \
        "$RUN_ID" "$SERIAL" "$hours" "$day_minutes" "$sales" "$sup" "$RUN/state.json"
    exit 0
}

# ══ supervisor ═════════════════════════════════════════════════════
cmd_supervise() {
    local run_id="" start_pid=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
        --run-id) run_id="${2:-}"; shift ;;
        --start-pid) start_pid="${2:-}"; shift ;;
        *) usage ;;
        esac
        shift
    done
    RUN="$SOAKROOT/$run_id"
    [[ -f "$RUN/state.json" ]] || { echo "supervisor: no state for $run_id" >&2; exit 2; }
    # shellcheck source=/dev/null
    source "$ROOT/scripts/android/p7_emulator_lock.sh"
    # wait for `start` to be gone: the lock wrapper removes its holder line then, and ours replaces it
    local i
    for i in $(seq 1 60); do
        alive "$start_pid" || break
        sleep 1
    done
    sleep 2
    p7_emulator_lock_note p7_soak
    local sf="$RUN/state.json" sampler_pid="" rc=0 problems stop_requested=0
    SERIAL="$(state_get "$sf" serial)"
    state_set "$sf" "supervisor_pid=$$" status=running >/dev/null
    on_term() { stop_requested=1; [[ -z "$sampler_pid" ]] || kill -TERM "$sampler_pid" 2>/dev/null; }
    trap on_term TERM INT
    python3 -B -I "$SAMPLER" run --dir "$RUN" >"$RUN/sampler.log" 2>&1 &
    sampler_pid=$!
    state_set "$sf" "sampler_pid=$sampler_pid" >/dev/null
    while kill -0 "$sampler_pid" 2>/dev/null; do
        wait "$sampler_pid"
        rc=$?
    done
    # if the sampler stopped before the driver did, ask the driver to wind down and give it a minute
    if [[ "$stop_requested" -eq 0 && -n "$SERIAL" ]] && ! grep -q '"type":"end"' "$RUN/driver.jsonl" 2>/dev/null; then
        adbd shell "mkdir -p $SOAKDIR && touch $SOAKDIR/stop" >/dev/null 2>&1
        sleep 20
    fi
    [[ -s "$RUN/report.txt" ]] || python3 -B -I "$SAMPLER" report --dir "$RUN" >"$RUN/report.txt" 2>&1
    problems="$(teardown_run "$RUN")"
    state_set "$sf" status=ended "end=$(date +%s)" "sampler_rc=$rc" "teardown=${problems:-ok}" >/dev/null
    p7_emulator_lock_cleanup
    exit 0
}

# ══ report ═════════════════════════════════════════════════════════
cmd_report() {
    local run="" min=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
        --run) run="${2:-}"; shift ;;
        --latest) run="" ;;
        --min-hours) min="${2:-}"; shift ;;
        *) usage ;;
        esac
        shift
    done
    [[ -n "$run" ]] || run="$(latest_run)"
    if [[ -z "$run" || ! -d "$SOAKROOT/$run" ]]; then
        printf 'SOAK | total | FAIL | no soak run found under %s\n' "$SOAKROOT"
        exit 1
    fi
    local args=(report --dir "$SOAKROOT/$run")
    [[ -z "$min" ]] || args+=(--min-hours "$min")
    python3 -B -I "$SAMPLER" "${args[@]}"
}

# ══ status ═════════════════════════════════════════════════════════
cmd_status() {
    local run sf last started now
    if run="$(running_run)"; then
        sf="$SOAKROOT/$run/state.json"
        started="$(state_get "$sf" start)"
        now="$(date +%s)"
        last="$(python3 -B -I "$SAMPLER" status --dir "$SOAKROOT/$run")"
        printf 'a soak runs: run %s, elapsed %s of %s h, %s min a day; %s\n' "$run" \
            "$(python3 -I -c "s=max(0,$now-int('${started:-$now}')); print('%d:%02d:%02d' % (s//3600, s%3600//60, s%60))")" \
            "$(state_get "$sf" hours)" "$(state_get "$sf" day_minutes)" "$last"
        return 0
    fi
    run="$(latest_run)"
    if [[ -n "$run" ]]; then
        sf="$SOAKROOT/$run/state.json"
        printf 'no soak runs (last run: %s, %s, teardown %s)\n' "$run" "$(state_get "$sf" status)" "$(state_get "$sf" teardown)"
    else
        echo "no soak runs"
    fi
    return 0
}

# ══ stop ═══════════════════════════════════════════════════════════
cmd_stop() {
    local run="" sf sup deadline rc
    while [[ $# -gt 0 ]]; do
        case "$1" in
        --run) run="${2:-}"; shift ;;
        *) usage ;;
        esac
        shift
    done
    if [[ -z "$run" ]]; then
        run="$(running_run)" || { echo "no soak runs"; exit 0; }
    fi
    sf="$SOAKROOT/$run/state.json"
    [[ -f "$sf" ]] || { printf 'SOAK | stop | FAIL | no run %s\n' "$run"; exit 1; }
    sup="$(state_get "$sf" supervisor_pid)"
    if alive "$sup" p7_soak; then
        SERIAL="$(state_get "$sf" serial)"
        if ! adbd shell "mkdir -p $SOAKDIR && touch $SOAKDIR/stop" >/dev/null 2>&1; then
            note "could not write the stop file through adb: asking the supervisor to stop"
            kill -TERM "$sup" 2>/dev/null
        fi
        deadline=$(($(date +%s) + 600))
        while alive "$sup" p7_soak && [[ "$(date +%s)" -lt "$deadline" ]]; do
            sleep 3
        done
        if alive "$sup" p7_soak; then
            printf 'SOAK | stop | FAIL | the supervisor (pid %s) is still tearing down after 600 s\n' "$sup"
            exit 1
        fi
    fi
    python3 -B -I "$SAMPLER" report --dir "$SOAKROOT/$run"
    rc=$?
    printf 'SOAK | teardown | %s | %s\n' "$([[ "$(state_get "$sf" teardown)" == ok ]] && echo PASS || echo FAIL)" "$(state_get "$sf" teardown)"
    exit "$rc"
}

case "$CMD" in
start) cmd_start "$@" ;;
_supervise) cmd_supervise "$@" ;;
report) cmd_report "$@" ;;
stop) cmd_stop "$@" ;;
status) cmd_status ;;
esac
