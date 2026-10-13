#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - end-to-end run of the platform features on the emulator (P6-T11, PD218)
#
# Usage: scripts/android/plat_e2e.sh [--keep]
#
# Proves on sunmi_d3_pro_api33 that the debug app applies its device-owner policies, exports the
# data for the tax authority (SPF) onto a removable volume, installs a declared update from a host
# registry mock, restarts after a fatal engine error, and writes the audit log; and measures the
# growth of the database per sale (PLATFORM.md section 12). One line per check, then one total line:
#     PLATE2E | <check> | PASS|FAIL | <detail>
#     PLATE2E | total | PASS|FAIL | <n> failed (<m> checks)
# Exit 1 on any FAIL. Checks, in order: emulator, precheck, removable-volume, build, newer-apk, mocks,
# bundle, install, launch, push, policies, sales, export, declared, update, refusals, watchdog, audit,
# teardown. A check whose precondition failed earlier is not run (nothing is half done on top of a
# failure), but teardown always runs.
#
# The instrumented steps (PlatformE2eTest, one `am instrument -e step <name>` per step) run with
# `am instrument`, never with a Gradle connected task (a device owner cannot be uninstalled, PD113). The
# install and the watchdog end the instrumented process on purpose, so the restart checks compare pids
# and read only the logcat of the NEW pid (PD118). `am force-stop` is never used.
#
# Ports. Host mocks run on free host ports; the device reaches them through adb reverse: 19380 (FDM),
# 19390 (printer), 19391 (printer HTTP jobs), 19392 (registry mock) and 19393 (the registry's redirect
# target, which stands for the CDN: the bearer token must not reach it).
#
# The newer APK of the update is the same debug build with versionCode + 1 and the next patch version
# (no -dev suffix, as the descriptor format wants). AGP ignores the injected version properties here, so it is
# built from a throwaway copy of HEAD under target/p6-newer of this tree (`git archive`, so not a registered
# work tree; build.gradle.kts is patched there only, never committed), removed at the end. The declared.json is signed with the development
# minisign key (the secret key is read in place, never copied).
#
# Teardown (an EXIT trap; skipped with --keep): the debug teardown broadcast releases the kiosk and
# clears the device owner, both packages are uninstalled, the export folder on the volume, this
# run's reverse rules, the mocks, the temporary directories and the newer-apk copy are removed.
# The emulator is never stopped or wiped.
#
# Environment: ANDROID_HOME, ANDROID_NDK_HOME, JAVA_HOME (JDK 17), cargo on PATH, minisign.
# PILI_GRADLE_OFFLINE=0 lets Gradle go online (default: --offline). PLAT_E2E_WAIT (seconds, default 2400) is how
# long the precheck waits for another end-to-end run on the shared emulator to end. PLAT_E2E_SALES (default 50) is
# the number of one-line sales, then PLAT_E2E_SALES2 (10) sales of PLAT_E2E_LINES2 (6) lines each. Logs: target/android-plat-e2e/.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    sed -n '2,/^# ──*$/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
    exit 0
fi
KEEP=0
case "${1:-}" in
"") ;;
--keep) KEEP=1 ;;
*) echo "usage: plat_e2e.sh [--keep]" >&2; exit 2 ;;
esac
[[ $# -le 1 ]] || { echo "usage: plat_e2e.sh [--keep]" >&2; exit 2; }

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
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
BUILD_TOOLS="$ANDROID_HOME/build-tools/35.0.0"
APKSIGNER="$BUILD_TOOLS/apksigner"
AAPT2="$BUILD_TOOLS/aapt2"
MINISIGN="$(command -v minisign || true)"
DEV_KEY="$HOME/.cache/pili-android/dev-minisign/minisign.key"
DEV_PUB="$ROOT/android/app/provisioning/dev-minisign.pub"

PKG=dev.nuagemagique.pilipos.debug
TEST_PKG=dev.nuagemagique.pilipos.debug.test
ADMIN="$PKG/dev.nuagemagique.pilipos.provisioning.PiliAdminReceiver"
MAIN="$PKG/dev.nuagemagique.pilipos.MainActivity"
TEARDOWN_ACTION=dev.nuagemagique.pilipos.debug.TEARDOWN
TEARDOWN_COMPONENT="$PKG/dev.nuagemagique.pilipos.debugtools.DebugTeardownReceiver"
TEST_CLASS=dev.nuagemagique.pilipos.platform.PlatformE2eTest
RUNNER="$TEST_PKG/androidx.test.runner.AndroidJUnitRunner"
INBOX="/sdcard/Android/data/$PKG/files/provisioning-inbox"
FDM_DEV_PORT=19380
PRINTER_DEV_PORT=19390
PRINTER_HTTP_DEV_PORT=19391
REGISTRY_DEV_PORT=19392
CDN_DEV_PORT=19393
DEV_PORTS=("$FDM_DEV_PORT" "$PRINTER_DEV_PORT" "$PRINTER_HTTP_DEV_PORT" "$REGISTRY_DEV_PORT" "$CDN_DEV_PORT")
SALES="${PLAT_E2E_SALES:-50}"
MOCKS="$ROOT/target/android-mocks/debug"
OUTDIR="$ROOT/target/android-plat-e2e"
APP_APK="$ROOT/android/app/build/outputs/apk/debug/app-debug.apk"
TEST_APK="$ROOT/android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk"
NEWER_WT="$ROOT/target/p6-newer"
JVMARGS="${PILI_GRADLE_JVMARGS:--Xmx2g -XX:MaxMetaspaceSize=1g -Dfile.encoding=UTF-8}"

SERIAL=""
FDM_PID=""
PRINTER_PID=""
REGISTRY_PID=""
WORK_DIR="" # holds the bundle, the copies of the APKs, the registry directory and the host copy of the export
OWNER_SET=0
INSTALLED=0
REVERSED=0
NEWER_WT_MADE=0
EXPORT_ROOT="" # device path of the export folder chosen on the volume
FAILED=0
CHECKS=0

adbd() { "$ADB" -s "$SERIAL" "$@"; }

note() { printf 'plat_e2e: %s\n' "$*" >&2; }

pass() { # <check> <detail>
    CHECKS=$((CHECKS + 1))
    printf 'PLATE2E | %s | PASS | %s\n' "$1" "$2"
}

fail() { # <check> <detail>
    CHECKS=$((CHECKS + 1))
    FAILED=$((FAILED + 1))
    printf 'PLATE2E | %s | FAIL | %s\n' "$1" "$2"
}

# ── teardown ────────────────────────────────────────────────────
owners() { adbd shell dpm list-owners 2>/dev/null | tr -d '\r'; }

has_owner_ours() { owners | grep -q "$PKG"; }

# no_owner: dpm list-owners names no admin at all ('no owners').
no_owner() { ! owners | grep -qE 'admin=|DeviceOwner|ProfileOwner'; }

do_teardown() {
    local problems=() i out pkg port
    if [[ -n "$SERIAL" ]]; then
        if [[ "$OWNER_SET" -eq 1 ]] && adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -qx "package:$PKG"; then
            # The app releases the kiosk (Lock Task, suspended launchers, persistent HOME) and clears the owner role.
            adbd shell am broadcast -a "$TEARDOWN_ACTION" -n "$TEARDOWN_COMPONENT" >"$OUTDIR/teardown-broadcast.log" 2>&1
            for i in 1 2 3 4 5 6 7 8 9 10; do
                has_owner_ours || break
                sleep 1
            done
            if has_owner_ours; then
                problems+=("device owner still set: $(owners | tr '\n' ' ')")
            fi
        fi
        if [[ -n "$EXPORT_ROOT" ]]; then
            adbd shell rm -rf "$EXPORT_ROOT" >/dev/null 2>&1
            if adbd shell "[ -e '$EXPORT_ROOT' ] && echo present" 2>/dev/null | grep -q present; then
                # the uninstall below removes the whole app-specific directory of the volume
                note "export folder $EXPORT_ROOT is still there, the uninstall removes it"
            fi
        fi
        if [[ "$INSTALLED" -eq 1 ]]; then
            for pkg in "$TEST_PKG" "$PKG"; do
                if adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -qx "package:$pkg"; then
                    # the policy manager may still hold the uninstall block for a moment after the owner is cleared
                    for i in 1 2 3 4 5 6; do
                        out="$(adbd uninstall "$pkg" 2>&1 | tr -d '\r')"
                        [[ "$out" == *Success* ]] && break
                        sleep 2
                    done
                    [[ "$out" == *Success* ]] || problems+=("uninstall $pkg: $out")
                fi
            done
        fi
        if [[ -n "$EXPORT_ROOT" ]] && adbd shell "[ -e '$EXPORT_ROOT' ] && echo present" 2>/dev/null | grep -q present; then
            problems+=("the export folder $EXPORT_ROOT is still on the volume")
        fi
        if [[ "$REVERSED" -eq 1 ]]; then
            for port in "${DEV_PORTS[@]}"; do
                adbd reverse --remove "tcp:$port" >/dev/null 2>&1
            done
        fi
    fi
    [[ -z "$FDM_PID" ]] || { kill "$FDM_PID" 2>/dev/null; wait "$FDM_PID" 2>/dev/null; }
    [[ -z "$PRINTER_PID" ]] || { kill "$PRINTER_PID" 2>/dev/null; wait "$PRINTER_PID" 2>/dev/null; }
    [[ -z "$REGISTRY_PID" ]] || { kill "$REGISTRY_PID" 2>/dev/null; wait "$REGISTRY_PID" 2>/dev/null; }
    [[ -z "$WORK_DIR" ]] || rm -rf "$WORK_DIR"
    if [[ "$NEWER_WT_MADE" -eq 1 ]]; then
        rm -rf "$NEWER_WT"
        if [[ -e "$NEWER_WT" ]] || git -C "$ROOT" worktree list | grep -q "p6-newer"; then
            problems+=("the newer-apk copy $NEWER_WT is still there")
        fi
    fi
    if [[ -n "$SERIAL" && "$OWNER_SET" -eq 1 ]]; then
        no_owner || problems+=("dpm list-owners still names an owner: $(owners | tr '\n' ' ')")
    fi
    if [[ -n "$SERIAL" ]] && adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -q 'pilipos'; then
        if [[ "$INSTALLED" -eq 1 ]]; then problems+=("a pilipos package is still installed"); fi
    fi
    if [[ -n "$SERIAL" && "$REVERSED" -eq 1 ]]; then
        if adbd reverse --list 2>/dev/null | grep -qE "tcp:($(IFS='|'; echo "${DEV_PORTS[*]}"))\b"; then
            problems+=("an adb reverse rule is still set")
        fi
    fi
    if [[ "${#problems[@]}" -eq 0 ]]; then
        pass teardown "owner cleared, packages uninstalled, export folder, reverse rules, mocks, temp dirs and newer-apk copy removed"
    else
        fail teardown "$(printf '%s; ' "${problems[@]}")"
    fi
}

finish() {
    local rc=$?
    trap - EXIT INT TERM
    if [[ "$KEEP" -eq 1 ]]; then
        printf 'PLATE2E | teardown | SKIP | --keep: owner, packages, reverse rules, mocks and the newer-apk copy stay\n'
    else
        do_teardown
    fi
    if [[ "$FAILED" -eq 0 && "$rc" -eq 0 ]]; then
        printf 'PLATE2E | total | PASS | 0 failed (%d checks)\n' "$CHECKS"
        exit 0
    fi
    [[ "$FAILED" -gt 0 ]] || FAILED=1
    printf 'PLATE2E | total | FAIL | %d failed (%d checks)\n' "$FAILED" "$CHECKS"
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

pid_of() { adbd shell pidof "$PKG" 2>/dev/null | tr -d '\r' | awk '{print $1}'; }

version_code_of() { adbd shell dumpsys package "$PKG" 2>/dev/null | tr -d '\r' | sed -n 's/.*versionCode=\([0-9][0-9]*\).*/\1/p' | head -n 1; }

is_locked() { adbd shell dumpsys activity activities 2>/dev/null | grep -q 'mLockTaskModeState=LOCKED'; }

# wait_new_pid <old pid> <seconds>: prints the first pid that differs from <old pid>, or nothing.
wait_new_pid() {
    local old="$1" deadline cand
    deadline=$(($(date +%s) + $2))
    while [[ "$(date +%s)" -lt "$deadline" ]]; do
        cand="$(pid_of)"
        if [[ -n "$cand" && "$cand" != "$old" ]]; then
            echo "$cand"
            return 0
        fi
        sleep 0.5
    done
    return 1
}

# wait_ready <pid> <seconds>: the engine of <pid> logged state=Ready (only that pid's lines count, PD118).
wait_ready() {
    local pid="$1" deadline
    deadline=$(($(date +%s) + $2))
    while [[ "$(date +%s)" -lt "$deadline" ]]; do
        adbd logcat -d --pid="$pid" -s PiliShell:I 2>/dev/null | grep -q 'state=Ready' && return 0
        sleep 1
    done
    return 1
}

# wait_locked <seconds>
wait_locked() {
    local deadline
    deadline=$(($(date +%s) + $1))
    while [[ "$(date +%s)" -lt "$deadline" ]]; do
        is_locked && return 0
        sleep 1
    done
    return 1
}

# run_step <name> <timeout seconds> [-e key value ...]: one `am instrument` of PlatformE2eTest. Sets STEP_OK (1 when
# the runner said OK (1 test) and no failure) and keeps the output in $OUTDIR/step-<name><suffix>.log and the
# PlatE2e logcat lines in the same name with .logcat. A step that ends its own process (update, fatal) is
# judged by the caller, not by STEP_OK.
STEP_OK=0
STEP_SUFFIX=""
run_step() {
    local step="$1" tmo="$2" ipid deadline log
    shift 2
    log="$OUTDIR/step-$step$STEP_SUFFIX"
    adbd logcat -c >/dev/null 2>&1
    adbd shell am instrument -w -r -e class "$TEST_CLASS" -e step "$step" -e managerInsz 94060242576 -e managerPin 1111 \
        -e sales "$SALES" -e sales2 "${PLAT_E2E_SALES2:-10}" -e lines2 "${PLAT_E2E_LINES2:-6}" -e registryPort "$REGISTRY_DEV_PORT" "$@" "$RUNNER" >"$log.log" 2>&1 &
    ipid=$!
    deadline=$(($(date +%s) + tmo))
    while kill -0 "$ipid" 2>/dev/null; do
        if [[ "$(date +%s)" -ge "$deadline" ]]; then
            kill "$ipid" 2>/dev/null
            echo "TIMEOUT after ${tmo}s" >>"$log.log"
            break
        fi
        sleep 1
    done
    wait "$ipid" 2>/dev/null
    adbd logcat -d -v threadtime -s PlatE2e:I >"$log.logcat" 2>&1
    adbd logcat -d -b crash -v threadtime >"$log.crash" 2>&1
    STEP_OK=0
    if tr -d '\r' <"$log.log" | grep -q '^OK (1 test)' && ! tr -d '\r' <"$log.log" | grep -qE 'INSTRUMENTATION_STATUS_CODE: -[0-9]|FAILURES!!!|shortMsg=|TIMEOUT after'; then
        STEP_OK=1
    else
        adbd logcat -d -t 600 -v threadtime >"$log.tail" 2>&1 # the last lines of every tag, for the post-mortem
    fi
}

# step_line <name> <prefix>: the last PLATE2E_<prefix> line of the step's logcat, without the logcat header.
step_line() { grep -h "PLATE2E_$2" "$OUTDIR/step-$1$STEP_SUFFIX.logcat" 2>/dev/null | tail -n 1 | sed 's/.*\(PLATE2E_\)/\1/'; }

# step_pid <name> <prefix>: the pid that logged the last PLATE2E_<prefix> line (the instrumented process).
step_pid() { grep -h "PLATE2E_$2" "$OUTDIR/step-$1$STEP_SUFFIX.logcat" 2>/dev/null | tail -n 1 | awk '{print $3}'; }

step_failure() { # <name>: the first lines of why a step failed
    tr -d '\r' <"$OUTDIR/step-$1$STEP_SUFFIX.log" | grep -E "stack=|AssertionError|Exception|shortMsg=|longMsg=|TIMEOUT|FAILURES" | head -n 6 | tr '\n' ' '
}

# ms_of <HH:MM:SS.mmm>: milliseconds since midnight (logcat threadtime stamps are device local time).
ms_of() { echo "$1" | awk -F'[:.]' '{ printf "%d", (($1 * 60 + $2) * 60 + $3) * 1000 + $4 }'; }

# step_time <name> <prefix>: the device time (HH:MM:SS.mmm) of the last PLATE2E_<prefix> line of the step.
step_time() { grep -h "PLATE2E_$2" "$OUTDIR/step-$1$STEP_SUFFIX.logcat" 2>/dev/null | tail -n 1 | awk '{print $2}'; }

# ready_time <pid>: the device time of the first 'state=Ready' line of that pid.
ready_time() { adbd logcat -d -v threadtime --pid="$1" -s PiliShell:I 2>/dev/null | grep 'state=Ready' | head -n 1 | awk '{print $2}'; }

field() { # <key> <line>: value of key=value in a result line
    echo "$2" | tr ' ' '\n' | sed -n "s/^$1=//p" | head -n 1
}

sha256_of() { shasum -a 256 "$1" | awk '{print $1}'; }

mkdir -p "$OUTDIR"
rm -f "$OUTDIR"/step-* "$OUTDIR"/registry.log
[[ -n "$ADB" && -x "$ADB" ]] || abort emulator "adb not found (ANDROID_HOME=$ANDROID_HOME)"
[[ -x "$APKSIGNER" && -x "$AAPT2" ]] || abort emulator "build-tools 35.0.0 (apksigner, aapt2) not found in $ANDROID_HOME"
[[ -n "$MINISIGN" ]] || abort emulator "minisign not found on PATH"
[[ -f "$DEV_KEY" ]] || abort emulator "the development minisign key is missing: $DEV_KEY (make_dev_bundle.sh creates it)"

# ── emulator ────────────────────────────────────────────────────
SERIAL="$(bash scripts/android/emulator.sh ensure 2>"$OUTDIR/emulator.err")" || {
    cat "$OUTDIR/emulator.err" >&2
    SERIAL=""
    abort emulator "emulator.sh ensure failed (exit 3 means another AVD runs)"
}
export ANDROID_SERIAL="$SERIAL"
pass emulator "$SERIAL"
# A sleeping headless emulator holds back the network: wake the screen (no setting is changed).
adbd shell "input keyevent KEYCODE_WAKEUP; wm dismiss-keyguard" >/dev/null 2>&1

# ── precheck ────────────────────────────────────────────────────
# The emulator is shared with the other lanes' gates. Another end-to-end run on it (app_e2e.sh, this script,
# the device tests) would install over this run and clear its device owner, so the run waits, at most
# PLAT_E2E_WAIT seconds (default 2400), until none is running. Nothing stops a run that starts later.
other_e2e() { pgrep -f 'scripts/android/(app_e2e|plat_e2e|device_engine_tests|emulator_smoke)[.]sh' | grep -vx "$$"; }
wait_idle() { # <check name>
    local deadline
    deadline=$(($(date +%s) + ${PLAT_E2E_WAIT:-2400}))
    while [[ -n "$(other_e2e)" ]]; do
        if [[ "$(date +%s)" -ge "$deadline" ]]; then
            abort "$1" "another end-to-end run still uses the emulator after ${PLAT_E2E_WAIT:-2400} s (pids $(other_e2e | tr '\n' ' '))"
        fi
        note "waiting for another end-to-end run on the emulator (pids $(other_e2e | tr '\n' ' '))"
        sleep 15
    done
}
wait_idle precheck
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

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/pili-plat-e2e.XXXXXX")" || abort precheck "mktemp failed"
WORK_DIR="$(cd "$WORK_DIR" && pwd -P)"

# ── removable-volume ────────────────────────────────────────────
# The removable volume is vold's virtual SD disk (S4-T4, Q20): a public vfat volume served by the same
# provider as a USB stick. The AVD keeps it; it is created only when missing.
mounted_volume() { adbd shell sm list-volumes public 2>/dev/null | tr -d '\r' | awk '$2 == "mounted" { print $3; exit }'; }
VOLUME_UUID="$(mounted_volume)"
created_volume=""
if [[ -z "$VOLUME_UUID" ]]; then
    adbd shell sm set-virtual-disk true >/dev/null 2>&1
    disk=""
    for _ in $(seq 1 30); do
        disk="$(adbd shell sm list-disks 2>/dev/null | tr -d '\r' | head -n 1)"
        [[ -n "$disk" ]] && break
        sleep 1
    done
    [[ -n "$disk" ]] || abort removable-volume "no virtual disk appeared after sm set-virtual-disk true"
    adbd shell sm partition "$disk" public >/dev/null 2>&1
    for _ in $(seq 1 60); do
        VOLUME_UUID="$(mounted_volume)"
        [[ -n "$VOLUME_UUID" ]] && break
        sleep 1
    done
    created_volume=" (created: sm set-virtual-disk true, sm partition $disk public)"
fi
[[ -n "$VOLUME_UUID" ]] || abort removable-volume "no mounted public volume (sm list-volumes public: $(adbd shell sm list-volumes public | tr -d '\r' | tr '\n' ' '))"
pass removable-volume "public volume $VOLUME_UUID mounted$created_volume"

# ── build ───────────────────────────────────────────────────────
offline=(--offline)
[[ "${PILI_GRADLE_OFFLINE:-1}" == "0" ]] && offline=()
# gradle_in <dir> <log> <gradle args...>: one Gradle build in <dir>/android through heavy.sh. Other agents on this
# Mac run `gradlew --stop`, which kills a build of ours in the middle ("Gradle build daemon has been stopped"):
# that failure alone is retried, twice.
gradle_in() {
    local dir="$1" log="$2" try rc=1
    shift 2
    for try in 1 2 3; do
        (
            cd "$dir/android" &&
                bash "$dir/scripts/android/heavy.sh" ./gradlew --no-daemon --console=plain "${offline[@]}" \
                    "-Dorg.gradle.jvmargs=$JVMARGS" "$@"
        ) >"$log" 2>&1
        rc=$?
        [[ "$rc" -ne 0 ]] && grep -q "Gradle build daemon has been stopped" "$log" || break
        note "the Gradle daemon was stopped by another process, retrying ($try)"
    done
    return "$rc"
}
gradle_run() { gradle_in "$ROOT" "$@"; } # <log> <gradle args...>
note "gradle :app:assembleDebug :app:assembleDebugAndroidTest (log $OUTDIR/gradle.log)"
gradle_run "$OUTDIR/gradle.log" :app:assembleDebug :app:assembleDebugAndroidTest || {
    grep -E "FAILED|What went wrong|error:|e: " "$OUTDIR/gradle.log" | head -n 20 >&2
    abort build "gradle failed, see $OUTDIR/gradle.log"
}
[[ -f "$APP_APK" && -f "$TEST_APK" ]] || abort build "APK missing: $APP_APK or $TEST_APK"
mkdir -p "$WORK_DIR/apk"
cp "$APP_APK" "$WORK_DIR/apk/app.apk"
cp "$TEST_APK" "$WORK_DIR/apk/test.apk"
badging="$("$AAPT2" dump badging "$WORK_DIR/apk/app.apk" 2>/dev/null)"
BASE_CODE="$(echo "$badging" | sed -n "s/.*versionCode='\([0-9]*\)'.*/\1/p" | head -n 1)"
BASE_NAME="$(echo "$badging" | sed -n "s/.*versionName='\([^']*\)'.*/\1/p" | head -n 1)"
[[ "$BASE_CODE" =~ ^[0-9]+$ && -n "$BASE_NAME" ]] || abort build "cannot read the version of the built APK"
pass build "app-debug.apk $BASE_NAME ($BASE_CODE) and app-debug-androidTest.apk"

# ── newer-apk ───────────────────────────────────────────────────
# The next patch version, without -dev (the descriptor wants X.Y.Z equal to the versionName of the APK).
IFS=. read -r v_major v_minor v_patch_raw <<<"${BASE_NAME%%-*}"
NEW_CODE=$((BASE_CODE + 1))
NEW_NAME="$v_major.$v_minor.$((v_patch_raw + 1))"
newer_ok() { # <apk>
    local b
    b="$("$AAPT2" dump badging "$1" 2>/dev/null)"
    [[ "$b" == *"versionCode='$NEW_CODE'"* && "$b" == *"versionName='$NEW_NAME'"* && "$b" == *"name='$PKG'"* ]]
}
# AGP 8.x ignores -Pandroid.injected.version.code and -Pandroid.injected.version.name here (tried: the APK keeps
# its version), so the newer APK is built from a throwaway copy of HEAD whose build.gradle.kts is patched there only.
rm -rf "$NEWER_WT"
mkdir -p "$NEWER_WT"
NEWER_WT_MADE=1
# A plain export of HEAD, not a `git worktree`, under this tree's own target directory (cargo output belongs
# there, and the disk guard of the shared Mac deletes `~/.cache/pili-android/*/target` while the disk is low:
# a build under the cache lost its target directory in the middle of the Gradle run).
git -C "$ROOT" archive HEAD | tar -x -C "$NEWER_WT" || abort newer-apk "git archive HEAD into $NEWER_WT failed"
sed -i '' -e "s/versionName = ffiVersion/versionName = \"$NEW_NAME\"/" -e "s/versionCode = computedVersionCode/versionCode = $NEW_CODE/" \
    "$NEWER_WT/android/app/build.gradle.kts"
grep -q "versionName = \"$NEW_NAME\"" "$NEWER_WT/android/app/build.gradle.kts" || abort newer-apk "could not patch build.gradle.kts in $NEWER_WT"
note "gradle :app:assembleDebug in $NEWER_WT (log $OUTDIR/gradle-newer.log)"
gradle_in "$NEWER_WT" "$OUTDIR/gradle-newer.log" :app:assembleDebug || {
    grep -E "FAILED|What went wrong|error:|e: " "$OUTDIR/gradle-newer.log" | head -n 20 >&2
    abort newer-apk "gradle failed in $NEWER_WT, see $OUTDIR/gradle-newer.log"
}
newer_ok "$NEWER_WT/android/app/build/outputs/apk/debug/app-debug.apk" || abort newer-apk "the APK built in $NEWER_WT is not $NEW_NAME ($NEW_CODE)"
cp "$NEWER_WT/android/app/build/outputs/apk/debug/app-debug.apk" "$WORK_DIR/apk/newer.apk"
newer_method="a throwaway export of HEAD in $NEWER_WT (removed at the end)"
pass newer-apk "$NEW_NAME ($NEW_CODE) built with $newer_method"

# ── mocks ───────────────────────────────────────────────────────
if [[ ! -x "$MOCKS/pos-fdm-mock" || ! -x "$MOCKS/pos-printer-mock" ]]; then
    bash scripts/android/heavy.sh cargo build --locked --manifest-path "$ROOT/Cargo.toml" -p pos-fdm-mock -p pos-printer-mock \
        --target-dir "$ROOT/target/android-mocks" >"$OUTDIR/mocks-build.log" 2>&1 ||
        abort mocks "cargo build of the mocks failed, see $OUTDIR/mocks-build.log"
fi
FDM_PORT="$(free_port)"
PRINTER_PORT="$(free_port)"
PRINTER_HTTP="$(free_port)"
REGISTRY_PORT="$(free_port)"
CDN_PORT="$(free_port)"
mkdir -p "$WORK_DIR/registry"
PORT="$FDM_PORT" "$MOCKS/pos-fdm-mock" >"$OUTDIR/fdm-mock.log" 2>&1 &
FDM_PID=$!
PORT="$PRINTER_PORT" HTTP_PORT="$PRINTER_HTTP" "$MOCKS/pos-printer-mock" >"$OUTDIR/printer-mock.log" 2>&1 &
PRINTER_PID=$!
python3 -I scripts/android/plat_registry_mock.py --dir "$WORK_DIR/registry" --port "$REGISTRY_PORT" \
    --redirect-port "$CDN_PORT" --redirect-location-port "$CDN_DEV_PORT" --log "$OUTDIR/registry.log" >"$OUTDIR/registry-mock.log" 2>&1 &
REGISTRY_PID=$!
wait_port "$FDM_PORT" 20 && wait_port "$PRINTER_PORT" 20 && wait_port "$PRINTER_HTTP" 20 && wait_port "$REGISTRY_PORT" 20 && wait_port "$CDN_PORT" 20 ||
    abort mocks "a mock does not accept connections, see $OUTDIR/*-mock.log"
REVERSED=1 # whatever of the rules exists is removed in the teardown
adbd reverse "tcp:$FDM_DEV_PORT" "tcp:$FDM_PORT" >/dev/null &&
    adbd reverse "tcp:$PRINTER_DEV_PORT" "tcp:$PRINTER_PORT" >/dev/null &&
    adbd reverse "tcp:$PRINTER_HTTP_DEV_PORT" "tcp:$PRINTER_HTTP" >/dev/null &&
    adbd reverse "tcp:$REGISTRY_DEV_PORT" "tcp:$REGISTRY_PORT" >/dev/null &&
    adbd reverse "tcp:$CDN_DEV_PORT" "tcp:$CDN_PORT" >/dev/null || abort mocks "adb reverse failed"
# The tunnels must carry data, not only accept a connection.
tunnel_ok=0
for _ in 1 2 3; do
    if adbd shell "(printf 'GET /graphql HTTP/1.0\\r\\n\\r\\n'; sleep 1) | nc -w 3 127.0.0.1 $FDM_DEV_PORT" 2>/dev/null | grep -q '^HTTP/'; then
        tunnel_ok=1
        break
    fi
done
[[ "$tunnel_ok" -eq 1 ]] || abort mocks "the FDM mock does not answer through the device port $FDM_DEV_PORT"
adbd shell "(printf 'GET /token HTTP/1.0\\r\\n\\r\\n'; sleep 1) | nc -w 3 127.0.0.1 $REGISTRY_DEV_PORT" 2>/dev/null | tr -d '\r' | grep -q '"token"' ||
    abort mocks "the registry mock does not answer through the device port $REGISTRY_DEV_PORT"
: >"$OUTDIR/registry.log" # the probe above is not part of the evidence
pass mocks "fdm $FDM_PORT, printer $PRINTER_PORT, jobs $PRINTER_HTTP, registry $REGISTRY_PORT (redirect target $CDN_PORT); device ports ${DEV_PORTS[*]}"

# ── bundle ──────────────────────────────────────────────────────
bundle_out="$(bash scripts/android/make_dev_bundle.sh --seed --out "$WORK_DIR/bundle" \
    --fdm-url "http://127.0.0.1:$FDM_DEV_PORT/graphql" --printer "127.0.0.1:$PRINTER_DEV_PORT" 2>&1)" ||
    abort bundle "make_dev_bundle.sh failed: $(echo "$bundle_out" | tail -n 3 | tr '\n' ' ')"
pass bundle "$(echo "$bundle_out" | tail -n 1)"

# ── install ─────────────────────────────────────────────────────
wait_idle install # the builds took minutes: another run may have started meanwhile
adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -q 'dev.nuagemagique.pilipos' && abort install "a pilipos package appeared during the build (another run?)"
no_owner || abort install "a device owner appeared during the build (another run?): $(owners | tr '\n' ' ')"
INSTALLED=1
for apk in "$WORK_DIR/apk/app.apk" "$WORK_DIR/apk/test.apk"; do
    out="$(adbd install -r "$apk" 2>&1 | tr -d '\r')"
    [[ "$out" == *Success* ]] || abort install "adb install $(basename "$apk"): $out"
done
# The exit code is 0 even when the OS refuses (S4): read the text. A just-installed package may need a moment.
owner_out=""
for _ in 1 2 3 4 5 6 7 8; do
    owner_out="$(adbd shell dpm set-device-owner "$ADMIN" 2>&1 | tr -d '\r')"
    [[ "$owner_out" == *Success* ]] && break
    sleep 3
done
if [[ "$owner_out" != *Success* ]]; then
    abort install "dpm set-device-owner refused: $(echo "$owner_out" | head -n 2 | tr '\n' ' ')"
fi
OWNER_SET=1
pass install "both APKs installed; $(echo "$owner_out" | head -n 1)"

# ── launch ──────────────────────────────────────────────────────
adbd logcat -c >/dev/null 2>&1
adbd shell am start -n "$MAIN" >"$OUTDIR/launch.log" 2>&1
deadline=$(($(date +%s) + 60))
found=0
while [[ "$(date +%s)" -lt "$deadline" ]]; do
    if adbd logcat -d -s PiliShell:I 2>/dev/null | grep -q 'state=NotProvisioned'; then
        found=1
        break
    fi
    sleep 1
done
[[ "$found" -eq 1 ]] || abort launch "no 'PiliShell state=NotProvisioned' within 60 s of the launch"
pass launch "PiliShell state=NotProvisioned"

# ── push ────────────────────────────────────────────────────────
adbd shell mkdir -p "$INBOX" || abort push "mkdir $INBOX failed"
pushed=0
for f in "$WORK_DIR"/bundle/*; do
    out="$(adbd push "$f" "$INBOX/" 2>&1 | tr -d '\r')"
    [[ "$out" == *"1 file pushed"* ]] || abort push "adb push $(basename "$f"): $out"
    pushed=$((pushed + 1))
done
[[ "$pushed" -ge 3 ]] || abort push "only $pushed files in the bundle"
pass push "$pushed files in $INBOX"

# ── policies ────────────────────────────────────────────────────
note "am instrument step=provision_and_policies (log $OUTDIR/step-provision_and_policies.log)"
run_step provision_and_policies 240
policy_line="$(step_line provision_and_policies POLICIES)"
if [[ "$STEP_OK" -eq 1 && "$policy_line" == "PLATE2E_POLICIES ok"* ]]; then
    pass policies "$policy_line; restrictions (install apps included), lock task features 0, auto time on, zone off, uninstall blocked, Google apps hidden, update window 03:00-05:00, keyguard"
else
    abort policies "step provision_and_policies did not pass: $(step_failure provision_and_policies)"
fi

# ── sales ───────────────────────────────────────────────────────
note "am instrument step=sales ($SALES sales, log $OUTDIR/step-sales.log)"
run_step sales 1500
logical_line="$(step_line sales DBSIZE_LOGICAL)"
lines_line="$(step_line sales DBSIZE_LINES)"
size_z_line="$(step_line sales DBSIZE_Z)"
backups_line="$(step_line sales BACKUPS)"
size_line="$(grep -h 'PLATE2E_DBSIZE sales=' "$OUTDIR/step-sales.logcat" 2>/dev/null | tail -n 1 | sed 's/.*\(PLATE2E_\)/\1/')"
if [[ "$STEP_OK" -eq 1 && "$size_line" == *bytes_per_sale=* ]]; then
    pass sales "$(echo "$size_line" | sed 's/^PLATE2E_DBSIZE //'); logical: $(echo "$logical_line" | sed 's/^PLATE2E_DBSIZE_LOGICAL //'); multi-line: $(echo "$lines_line" | sed 's/^PLATE2E_DBSIZE_LINES //'); z: $(echo "$size_z_line" | sed 's/^PLATE2E_DBSIZE_Z //'); backups: $(echo "$backups_line" | sed 's/^PLATE2E_BACKUPS //')"
else
    abort sales "step sales did not pass: $(step_failure sales)"
fi

# ── export ──────────────────────────────────────────────────────
note "am instrument step=export (log $OUTDIR/step-export.log)"
run_step export 300
export_line="$(step_line export EXPORT)"
if [[ "$STEP_OK" -ne 1 || "$export_line" != "PLATE2E_EXPORT ok"* ]]; then
    abort export "step export did not pass: $(step_failure export)"
fi
export_dir="$(field dir "$export_line")"
export_files="$(field files "$export_line")"
export_bytes="$(field bytes "$export_line")"
[[ "$export_dir" == /storage/* ]] || abort export "the export folder is not on a /storage volume: $export_dir"
EXPORT_ROOT="$(dirname "$export_dir")"
dev_check="$(adbd shell "cd '$export_dir' && sha256sum -c MANIFEST.sha256" 2>&1 | tr -d '\r')"
dev_bad="$(echo "$dev_check" | grep -vcE ': OK$')"
dev_ok="$(echo "$dev_check" | grep -cE ': OK$')"
[[ "$dev_bad" -eq 0 && "$dev_ok" -ge 1 ]] || abort export "device sha256sum -c MANIFEST.sha256 failed: $(echo "$dev_check" | head -n 4 | tr '\n' ' ')"
mkdir -p "$WORK_DIR/export-host"
pull_out="$(adbd pull "$export_dir" "$WORK_DIR/export-host/" 2>&1 | tr -d '\r' | tail -n 1)"
host_dir="$WORK_DIR/export-host/$(basename "$export_dir")"
[[ -f "$host_dir/MANIFEST.sha256" ]] || abort export "adb pull of $export_dir failed: $pull_out"
host_check="$(cd "$host_dir" && shasum -a 256 -c MANIFEST.sha256 2>&1)"
host_bad="$(echo "$host_check" | grep -vcE ': OK$')"
host_ok="$(echo "$host_check" | grep -cE ': OK$')"
[[ "$host_bad" -eq 0 && "$host_ok" -ge 1 ]] || abort export "host shasum -a 256 -c MANIFEST.sha256 failed: $(echo "$host_check" | head -n 4 | tr '\n' ' ')"
[[ "$host_ok" -eq "$dev_ok" && "$((host_ok + 1))" -eq "$export_files" ]] ||
    abort export "manifest lines differ: device $dev_ok, host $host_ok, files reported $export_files (manifest excluded)"
integrity="skipped (no sqlite3 on the host)"
if command -v sqlite3 >/dev/null 2>&1; then
    integrity="$(sqlite3 "$host_dir/pos.db" 'PRAGMA integrity_check;' 2>&1 | head -n 1)"
    [[ "$integrity" == ok ]] || abort export "PRAGMA integrity_check of the exported pos.db says: $integrity"
fi
pass export "$export_files files, $export_bytes bytes on volume $VOLUME_UUID; MANIFEST.sha256 verified on the device (sha256sum -c: $dev_ok OK) and on the host (shasum -a 256 -c: $host_ok OK); pos.db integrity_check $integrity"

# ── declared ────────────────────────────────────────────────────
# The host prepares the registry directory the way declare_android.sh would: the APK, declared.json (PD211 schema)
# and its minisign signature, made with the development key (read in place, never copied).
make_declared() { # <version> <versionCode> <apk file in registry> <sha256 to declare> <sha1> <signer> : writes declared.json and its signature
    local dir="$WORK_DIR/registry" ts
    ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    python3 -I - "$dir" "$1" "$2" "$3" "$4" "$5" "$6" "$ts" <<'PY' || return 1
import json, sys
d, version, code, asset, sha256, sha1, signer, ts = sys.argv[1:9]
doc = {"format": 1, "platform": "android", "package": "dev.nuagemagique.pilipos.debug", "version": version,
       "versionCode": int(code), "asset": asset, "sha1": sha1, "sha256": sha256, "signerSha256": signer, "declared_at": ts}
with open(d + "/declared.json", "w", encoding="utf-8", newline="\n") as f:
    f.write(json.dumps(doc, separators=(",", ":")) + "\n")
PY
    rm -f "$dir/declared.json.minisig"
    "$MINISIGN" -S -s "$DEV_KEY" -m "$dir/declared.json" -x "$dir/declared.json.minisig" -t "pili-pos-android $1 declared $ts" >/dev/null 2>&1 || return 1
    "$MINISIGN" -V -p "$DEV_PUB" -m "$dir/declared.json" -x "$dir/declared.json.minisig" >/dev/null 2>&1
}
NEW_ASSET="pili-pos-android-$NEW_NAME.apk"
cp "$WORK_DIR/apk/newer.apk" "$WORK_DIR/registry/$NEW_ASSET"
NEW_SHA1="$(shasum -a 1 "$WORK_DIR/apk/newer.apk" | awk '{print $1}')"
NEW_SHA256="$(sha256_of "$WORK_DIR/apk/newer.apk")"
NEW_SIGNER="$("$APKSIGNER" verify --print-certs "$WORK_DIR/apk/newer.apk" 2>/dev/null | sed -n 's/^Signer #1 certificate SHA-256 digest: //p' | head -n 1)"
[[ "$NEW_SIGNER" =~ ^[0-9a-f]{64}$ ]] || abort declared "apksigner verify --print-certs gave no signer SHA-256 for the newer APK"
OLD_SIGNER="$("$APKSIGNER" verify --print-certs "$WORK_DIR/apk/app.apk" 2>/dev/null | sed -n 's/^Signer #1 certificate SHA-256 digest: //p' | head -n 1)"
[[ "$NEW_SIGNER" == "$OLD_SIGNER" ]] || abort declared "the two APKs are not signed by the same key ($OLD_SIGNER vs $NEW_SIGNER)"
make_declared "$NEW_NAME" "$NEW_CODE" "$NEW_ASSET" "$NEW_SHA256" "$NEW_SHA1" "$NEW_SIGNER" || abort declared "declared.json could not be made or signed"
pass declared "$NEW_ASSET sha1 ${NEW_SHA1:0:12}.. sha256 ${NEW_SHA256:0:12}.. signer ${NEW_SIGNER:0:12}.., declared.json signed with the development key and verified with dev-minisign.pub"

# ── update ──────────────────────────────────────────────────────
# A failure here is recorded but does not stop the run: refusals, watchdog and audit do not need the update, and the
# audit check then reports the missing update rows by itself.
UPDATE_OK=0
check_update() {
    local old_code new_code old_pid new_pid update_end pid_seconds locked_seconds verified_line committed_line facts
    old_code="$(version_code_of)"
    if [[ "$old_code" != "$BASE_CODE" ]]; then
        fail update "the installed versionCode is $old_code, expected $BASE_CODE"
        return 1
    fi
    note "am instrument step=update (the install ends the process, log $OUTDIR/step-update.log)"
    run_step update 240
    update_end=$(date +%s)
    facts="$(step_line update PRECONDITIONS)"
    verified_line="$(step_line update 'UPDATE verified')"
    committed_line="$(step_line update 'UPDATE committed')"
    if [[ "$verified_line" != "PLATE2E_UPDATE verified"* ]]; then
        fail update "the update was not verified in the app: $(step_failure update) $facts"
        return 1
    fi
    if [[ "$committed_line" != "PLATE2E_UPDATE committed"* ]]; then
        fail update "the install was not committed: $(step_failure update) [$facts]"
        return 1
    fi
    old_pid="$(step_pid update 'UPDATE committed')" # the instrumented process that the install must end
    if ! [[ "$old_pid" =~ ^[0-9]+$ ]]; then
        fail update "cannot read the pid of the instrumented process"
        return 1
    fi
    if ! new_pid="$(wait_new_pid "$old_pid" 90)"; then
        fail update "no new process for $PKG within 90 s (old pid $old_pid)"
        return 1
    fi
    pid_seconds=$(($(date +%s) - update_end))
    new_code=""
    for _ in $(seq 1 30); do
        new_code="$(version_code_of)"
        [[ "$new_code" == "$NEW_CODE" ]] && break
        sleep 1
    done
    if [[ "$new_code" != "$NEW_CODE" ]]; then
        fail update "dumpsys package versionCode is '$new_code', expected $NEW_CODE (old pid $old_pid, new pid $new_pid)"
        return 1
    fi
    if ! wait_ready "$new_pid" 60; then
        fail update "pid $new_pid never logged state=Ready after the update"
        return 1
    fi
    if ! wait_locked 30; then
        fail update "pid $new_pid runs $new_code but mLockTaskModeState is not LOCKED within 30 s"
        return 1
    fi
    locked_seconds=$(($(date +%s) - update_end))
    # The bearer token must not reach the redirect target (the CDN): the registry log shows it.
    local cdn_hits cdn_with_auth main_blob_auth
    cdn_hits="$(grep -c '^REQ cdn ' "$OUTDIR/registry.log" 2>/dev/null)"
    cdn_with_auth="$(grep -c '^REQ cdn .* auth=yes' "$OUTDIR/registry.log" 2>/dev/null)"
    main_blob_auth="$(grep -c '^REQ main GET /v2/.*/blobs/.* auth=yes -> 307' "$OUTDIR/registry.log" 2>/dev/null)"
    if [[ "$cdn_hits" -lt 1 ]]; then
        fail update "the registry log shows no request on the redirect target: $(tail -n 5 "$OUTDIR/registry.log" | tr '\n' ' ')"
        return 1
    fi
    if [[ "$cdn_with_auth" -ne 0 ]]; then
        fail update "the Authorization header reached the redirect target $cdn_with_auth time(s)"
        return 1
    fi
    if [[ "$main_blob_auth" -lt 1 ]]; then
        fail update "no authenticated blob request was redirected by the registry"
        return 1
    fi
    local t_commit t_ready commit_to_ready=""
    t_commit="$(step_time update 'UPDATE committed')"
    t_ready="$(ready_time "$new_pid")"
    if [[ -n "$t_commit" && -n "$t_ready" ]]; then
        commit_to_ready="$(( $(ms_of "$t_ready") - $(ms_of "$t_commit") )) ms from the commit line to state=Ready of the new pid, "
    fi
    UPDATE_OK=1
    pass update "versionCode $old_code -> $new_code, pid $old_pid -> $new_pid, ${commit_to_ready}mLockTaskModeState=LOCKED within ${locked_seconds}s of the end of the instrumentation; registry: $main_blob_auth blob request(s) with the bearer redirected, $cdn_hits on the redirect target, 0 with Authorization"
}
check_update

# ── refusals ────────────────────────────────────────────────────
# The installed versionCode is read again (NEW_CODE after the update, BASE_CODE when the update failed). Case 1: a
# signed descriptor for that same versionCode (not_newer). Case 2: a newer one whose declared SHA-256 is wrong
# (sha256_mismatch, the APK is downloaded first).
installed_code="$(version_code_of)"
[[ "$installed_code" =~ ^[0-9]+$ ]] || abort refusals "cannot read the installed versionCode"
refusal_notes=()
make_declared "$NEW_NAME" "$installed_code" "$NEW_ASSET" "$NEW_SHA256" "$NEW_SHA1" "$NEW_SIGNER" || abort refusals "declared.json (not_newer) could not be made"
STEP_SUFFIX="-not_newer"
run_step update_refusals 180 -e refusal not_newer
r1="$(step_line update_refusals REFUSALS)"
[[ "$STEP_OK" -eq 1 && "$r1" == "PLATE2E_REFUSALS ok"* ]] || abort refusals "case not_newer failed: $(step_failure update_refusals)"
refusal_notes+=("$(echo "$r1" | sed 's/^PLATE2E_REFUSALS ok //')")
next_name="$v_major.$v_minor.$((v_patch_raw + 2))"
next_asset="pili-pos-android-$next_name.apk"
rm -f "$WORK_DIR/registry/$NEW_ASSET"
cp "$WORK_DIR/apk/newer.apk" "$WORK_DIR/registry/$next_asset"
wrong_sha256="$(printf '%064d' 0)"
make_declared "$next_name" "$((installed_code + 1))" "$next_asset" "$wrong_sha256" "$NEW_SHA1" "$NEW_SIGNER" || abort refusals "declared.json (sha256_mismatch) could not be made"
STEP_SUFFIX="-sha256_mismatch"
run_step update_refusals 180 -e refusal sha256_mismatch
r2="$(step_line update_refusals REFUSALS)"
[[ "$STEP_OK" -eq 1 && "$r2" == "PLATE2E_REFUSALS ok"* ]] || abort refusals "case sha256_mismatch failed: $(step_failure update_refusals)"
refusal_notes+=("$(echo "$r2" | sed 's/^PLATE2E_REFUSALS ok //')")
STEP_SUFFIX=""
pass refusals "installed versionCode $installed_code; ${refusal_notes[*]}; the refused download left no APK in filesDir/platform/update"

# ── watchdog ────────────────────────────────────────────────────
# The test marks the engine fatal; the watchdog audits it, waits 3 s and kills the process; the system restarts the
# persistent HOME app. Only the new pid's lines count (PD118).
note "am instrument step=fatal (the watchdog ends the process, log $OUTDIR/step-fatal.log)"
run_step fatal 120
fatal_end=$(date +%s)
sent_line="$(step_line fatal FATAL)"
[[ "$sent_line" == "PLATE2E_FATAL sent"* ]] || abort watchdog "the fatal step did not run: $(step_failure fatal)"
old_pid="$(step_pid fatal 'FATAL sent')" # the instrumented process that the watchdog must end
[[ "$old_pid" =~ ^[0-9]+$ ]] || abort watchdog "cannot read the pid of the instrumented process"
new_pid="$(wait_new_pid "$old_pid" 15)" || abort watchdog "no new process for $PKG within 15 s of the end of the fatal step (old pid $old_pid)"
restart_seconds=$(($(date +%s) - fatal_end))
wait_ready "$new_pid" 60 || abort watchdog "pid $new_pid never logged state=Ready after the watchdog restart"
ready_seconds=$(($(date +%s) - fatal_end))
shell_log="$(adbd logcat -d --pid="$new_pid" -s PiliShell:I 2>/dev/null | grep 'state=' | sed 's/.*state=/state=/' | tr '\n' ' ')"
if echo "$shell_log" | grep -q 'NotProvisioned'; then
    abort watchdog "pid $new_pid went through NotProvisioned ($shell_log)"
fi
wait_locked 30 || abort watchdog "pid $new_pid is Ready but mLockTaskModeState is not LOCKED within 30 s"
locked_seconds=$(($(date +%s) - fatal_end))
t_sent="$(step_time fatal 'FATAL sent')"
t_ready="$(ready_time "$new_pid")"
sent_to_ready=""
if [[ -n "$t_sent" && -n "$t_ready" ]]; then
    sent_to_ready="$(( $(ms_of "$t_ready") - $(ms_of "$t_sent") )) ms from markFatal to state=Ready of the new pid (3000 ms of it is the watchdog's wait), "
fi
pass watchdog "fatal sent, pid $old_pid -> $new_pid, ${sent_to_ready}new pid seen ${restart_seconds}s after the process died, mLockTaskModeState=LOCKED within ${locked_seconds}s ($shell_log)"

# ── audit ───────────────────────────────────────────────────────
note "am instrument step=check_audit (log $OUTDIR/step-check_audit.log)"
run_step check_audit 180
audit_line="$(step_line check_audit AUDIT)"
if [[ "$STEP_OK" -eq 1 && "$audit_line" == "PLATE2E_AUDIT ok"* ]]; then
    pass audit "$(echo "$audit_line" | sed 's/^PLATE2E_AUDIT ok //'); policy, export, update (verified, refused, started, package_replaced, done) and watchdog (fatal, restart, restarted) rows"
else
    abort audit "step check_audit did not pass: $(step_failure check_audit)"
fi

# teardown and the total line come from the EXIT trap
exit 0
