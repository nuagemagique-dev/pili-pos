#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - end-to-end run of the :app foundation on the emulator (P4-T12, PD113)
#
# Usage: scripts/android/app_e2e.sh [--keep]
#
# Proves on sunmi_d3_pro_api33 that the debug app can be made device owner, provisioned from a
# development bundle through the debug inbox, opened (engine Ready), locked in Lock Task, that a
# manager can log in against the host FDM mock, and that a force-stop comes back Ready and relocked
# without a second import. One line per check, then one total line:
#     APPE2E | <check> | PASS|FAIL | <detail>
#     APPE2E | total | PASS|FAIL | <n> failed (<m> checks)
# Exit 1 on any FAIL. Checks, in order: emulator, precheck, build, mocks, bundle, install, launch,
# push, instrumented, restart, teardown. A check whose precondition failed earlier is not run
# (nothing is half done on top of a failure), but teardown always runs.
#
# The instrumented test (ProvisioningE2eTest) runs with `am instrument`, not with a Gradle connected
# task: Gradle uninstalls the app, which a device owner forbids (PD113). The bundle is made by
# make_dev_bundle.sh into a fresh temporary directory outside the repository and pushed to the app's
# debug inbox. The mocks run on free host ports; the device reaches them on 19380 (FDM), 19390
# (printer) and 19391 (printer HTTP jobs) through adb reverse.
#
# Teardown (an EXIT trap, so also after an error or a signal; skipped with --keep): the debug
# teardown broadcast releases the kiosk and clears the device owner, then both packages are
# uninstalled, this script's three reverse rules are removed, the mocks are stopped and the temporary
# bundle is deleted. It only acts on what this run did: a device that already had an owner or more
# than one user is refused by the precheck and left alone. The emulator is never stopped or wiped.
#
# Environment: ANDROID_HOME, ANDROID_NDK_HOME, JAVA_HOME (JDK 17), cargo on PATH. PILI_GRADLE_OFFLINE=0
# lets Gradle go online (default: --offline). Logs: target/android-app-e2e/.
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
*) echo "usage: app_e2e.sh [--keep]" >&2; exit 2 ;;
esac
[[ $# -le 1 ]] || { echo "usage: app_e2e.sh [--keep]" >&2; exit 2; }

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

PKG=dev.nuagemagique.pilipos.debug
TEST_PKG=dev.nuagemagique.pilipos.debug.test
ADMIN="$PKG/dev.nuagemagique.pilipos.provisioning.PiliAdminReceiver"
MAIN="$PKG/dev.nuagemagique.pilipos.MainActivity"
TEARDOWN_ACTION=dev.nuagemagique.pilipos.debug.TEARDOWN
TEARDOWN_COMPONENT="$PKG/dev.nuagemagique.pilipos.debugtools.DebugTeardownReceiver"
INBOX="/sdcard/Android/data/$PKG/files/provisioning-inbox"
FDM_DEV_PORT=19380
PRINTER_DEV_PORT=19390
PRINTER_HTTP_DEV_PORT=19391
MOCKS="$ROOT/target/android-mocks/debug"
OUTDIR="$ROOT/target/android-app-e2e"
APP_APK="$ROOT/android/app/build/outputs/apk/debug/app-debug.apk"
TEST_APK="$ROOT/android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk"
JVMARGS="${PILI_GRADLE_JVMARGS:--Xmx2g -XX:MaxMetaspaceSize=1g -Dfile.encoding=UTF-8}"

SERIAL=""
FDM_PID=""
PRINTER_PID=""
BUNDLE_DIR=""
OWNER_SET=0
INSTALLED=0
REVERSED=0
FAILED=0
CHECKS=0

adbd() { "$ADB" -s "$SERIAL" "$@"; }

note() { printf 'app_e2e: %s\n' "$*" >&2; }

pass() { # <check> <detail>
    CHECKS=$((CHECKS + 1))
    printf 'APPE2E | %s | PASS | %s\n' "$1" "$2"
}

fail() { # <check> <detail>
    CHECKS=$((CHECKS + 1))
    FAILED=$((FAILED + 1))
    printf 'APPE2E | %s | FAIL | %s\n' "$1" "$2"
}

# ── teardown ────────────────────────────────────────────────────
owners() { adbd shell dpm list-owners 2>/dev/null | tr -d '\r'; }

has_owner_ours() { owners | grep -q "$PKG"; }

# no_owner: dpm list-owners names no admin at all ('no owners').
no_owner() { ! owners | grep -qE 'admin=|DeviceOwner|ProfileOwner'; }

do_teardown() {
    local problems=() i
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
        if [[ "$INSTALLED" -eq 1 ]]; then
            for pkg in "$TEST_PKG" "$PKG"; do
                if adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -qx "package:$pkg"; then
                    out="$(adbd uninstall "$pkg" 2>&1 | tr -d '\r')"
                    [[ "$out" == *Success* ]] || problems+=("uninstall $pkg: $out")
                fi
            done
        fi
        if [[ "$REVERSED" -eq 1 ]]; then
            for port in "$FDM_DEV_PORT" "$PRINTER_DEV_PORT" "$PRINTER_HTTP_DEV_PORT"; do
                adbd reverse --remove "tcp:$port" >/dev/null 2>&1
            done
        fi
    fi
    [[ -z "$FDM_PID" ]] || { kill "$FDM_PID" 2>/dev/null; wait "$FDM_PID" 2>/dev/null; }
    [[ -z "$PRINTER_PID" ]] || { kill "$PRINTER_PID" 2>/dev/null; wait "$PRINTER_PID" 2>/dev/null; }
    [[ -z "$BUNDLE_DIR" ]] || rm -rf "$BUNDLE_DIR"
    if [[ -n "$SERIAL" && "$OWNER_SET" -eq 1 ]]; then
        no_owner || problems+=("dpm list-owners still names an owner: $(owners | tr '\n' ' ')")
    fi
    if [[ -n "$SERIAL" ]] && adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -q 'pilipos'; then
        if [[ "$INSTALLED" -eq 1 ]]; then problems+=("a pilipos package is still installed"); fi
    fi
    if [[ -n "$SERIAL" && "$REVERSED" -eq 1 ]]; then
        if adbd reverse --list 2>/dev/null | grep -qE "tcp:($FDM_DEV_PORT|$PRINTER_DEV_PORT|$PRINTER_HTTP_DEV_PORT)\b"; then
            problems+=("an adb reverse rule is still set")
        fi
    fi
    if [[ "${#problems[@]}" -eq 0 ]]; then
        pass teardown "owner cleared, packages uninstalled, reverse rules removed, mocks stopped, bundle deleted"
    else
        fail teardown "$(printf '%s; ' "${problems[@]}")"
    fi
}

finish() {
    local rc=$?
    trap - EXIT INT TERM
    if [[ "$KEEP" -eq 1 ]]; then
        printf 'APPE2E | teardown | SKIP | --keep: owner, packages, reverse rules and mocks stay (mocks stop with this script)\n'
    else
        do_teardown
    fi
    if [[ "$FAILED" -eq 0 && "$rc" -eq 0 ]]; then
        printf 'APPE2E | total | PASS | 0 failed (%d checks)\n' "$CHECKS"
        exit 0
    fi
    [[ "$FAILED" -gt 0 ]] || FAILED=1
    printf 'APPE2E | total | FAIL | %d failed (%d checks)\n' "$FAILED" "$CHECKS"
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

mkdir -p "$OUTDIR"
[[ -n "$ADB" && -x "$ADB" ]] || abort emulator "adb not found (ANDROID_HOME=$ANDROID_HOME)"

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
note "gradle :app:assembleDebug :app:assembleDebugAndroidTest (log $OUTDIR/gradle.log)"
(
    cd android &&
        bash ../scripts/android/heavy.sh ./gradlew --no-daemon --console=plain "${offline[@]}" \
            "-Dorg.gradle.jvmargs=$JVMARGS" :app:assembleDebug :app:assembleDebugAndroidTest
) >"$OUTDIR/gradle.log" 2>&1 || {
    grep -E "FAILED|What went wrong|error:|e: " "$OUTDIR/gradle.log" | head -n 20 >&2
    abort build "gradle failed, see $OUTDIR/gradle.log"
}
[[ -f "$APP_APK" && -f "$TEST_APK" ]] || abort build "APK missing: $APP_APK or $TEST_APK"
pass build "app-debug.apk and app-debug-androidTest.apk"

# ── mocks ───────────────────────────────────────────────────────
if [[ ! -x "$MOCKS/pos-fdm-mock" || ! -x "$MOCKS/pos-printer-mock" ]]; then
    bash scripts/android/heavy.sh cargo build --locked --manifest-path "$ROOT/Cargo.toml" -p pos-fdm-mock -p pos-printer-mock \
        --target-dir "$ROOT/target/android-mocks" >"$OUTDIR/mocks-build.log" 2>&1 ||
        abort mocks "cargo build of the mocks failed, see $OUTDIR/mocks-build.log"
fi
FDM_PORT="$(free_port)"
PRINTER_PORT="$(free_port)"
PRINTER_HTTP="$(free_port)"
PORT="$FDM_PORT" "$MOCKS/pos-fdm-mock" >"$OUTDIR/fdm-mock.log" 2>&1 &
FDM_PID=$!
PORT="$PRINTER_PORT" HTTP_PORT="$PRINTER_HTTP" "$MOCKS/pos-printer-mock" >"$OUTDIR/printer-mock.log" 2>&1 &
PRINTER_PID=$!
wait_port "$FDM_PORT" 20 && wait_port "$PRINTER_PORT" 20 && wait_port "$PRINTER_HTTP" 20 ||
    abort mocks "a mock does not accept connections, see $OUTDIR/*-mock.log"
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

# ── bundle ──────────────────────────────────────────────────────
BUNDLE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/pili-app-e2e-bundle.XXXXXX")" || abort bundle "mktemp failed"
BUNDLE_DIR="$(cd "$BUNDLE_DIR" && pwd -P)"
bundle_out="$(bash scripts/android/make_dev_bundle.sh --seed --out "$BUNDLE_DIR/bundle" \
    --fdm-url "http://127.0.0.1:$FDM_DEV_PORT/graphql" --printer "127.0.0.1:$PRINTER_DEV_PORT" 2>&1)" ||
    abort bundle "make_dev_bundle.sh failed: $(echo "$bundle_out" | tail -n 3 | tr '\n' ' ')"
pass bundle "$(echo "$bundle_out" | tail -n 1)"

# ── install ─────────────────────────────────────────────────────
INSTALLED=1
for apk in "$APP_APK" "$TEST_APK"; do
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
since="$(adbd shell date +'%m-%d\ %H:%M:%S.000' | tr -d '\r')"
adbd shell am start -n "$MAIN" >"$OUTDIR/launch.log" 2>&1
deadline=$(($(date +%s) + 60))
found=0
while [[ "$(date +%s)" -lt "$deadline" ]]; do
    if adbd logcat -d -s PiliShell:I -T "$since" 2>/dev/null | grep -q 'state=NotProvisioned'; then
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
for f in "$BUNDLE_DIR"/bundle/*; do
    out="$(adbd push "$f" "$INBOX/" 2>&1 | tr -d '\r')"
    [[ "$out" == *"1 file pushed"* ]] || abort push "adb push $(basename "$f"): $out"
    pushed=$((pushed + 1))
done
[[ "$pushed" -ge 3 ]] || abort push "only $pushed files in the bundle"
pass push "$pushed files in $INBOX"

# ── instrumented ────────────────────────────────────────────────
note "am instrument ProvisioningE2eTest (log $OUTDIR/instrument.log)"
adbd shell am instrument -w -r \
    -e class dev.nuagemagique.pilipos.e2e.ProvisioningE2eTest \
    -e fdmPort "$FDM_DEV_PORT" \
    "$TEST_PKG/androidx.test.runner.AndroidJUnitRunner" >"$OUTDIR/instrument.log" 2>&1
instr="$(tr -d '\r' <"$OUTDIR/instrument.log")"
if echo "$instr" | grep -q '^OK (1 test)' && ! echo "$instr" | grep -qE 'INSTRUMENTATION_STATUS_CODE: -[0-9]|FAILURES!!!|shortMsg='; then
    pass instrumented "OK (1 test)"
else
    echo "$instr" | grep -E "stack=|Failed|AssertionError|Exception|FAILURES|Tests run" | head -n 12 >&2
    abort instrumented "ProvisioningE2eTest did not pass, see $OUTDIR/instrument.log"
fi

# ── restart ─────────────────────────────────────────────────────
adbd shell am force-stop "$PKG"
sleep 1
since="$(adbd shell date +'%m-%d\ %H:%M:%S.000' | tr -d '\r')"
adbd shell am start -n "$MAIN" >"$OUTDIR/restart.log" 2>&1
deadline=$(($(date +%s) + 30))
ready=0
while [[ "$(date +%s)" -lt "$deadline" ]]; do
    if adbd logcat -d -s PiliShell:I -T "$since" 2>/dev/null | grep -q 'state=Ready'; then
        ready=1
        break
    fi
    sleep 1
done
shell_log="$(adbd logcat -d -s PiliShell:I -T "$since" 2>/dev/null | grep 'state=' | sed 's/.*state=/state=/' | tr '\n' ' ')"
[[ "$ready" -eq 1 ]] || abort restart "no state=Ready within 30 s of the restart (saw: $shell_log)"
if echo "$shell_log" | grep -q 'NotProvisioned'; then
    abort restart "the restart went through NotProvisioned, the bundle was not kept (saw: $shell_log)"
fi
locked=0
for _ in $(seq 1 20); do
    if adbd shell dumpsys activity activities 2>/dev/null | grep -q 'mLockTaskModeState=LOCKED'; then
        locked=1
        break
    fi
    sleep 1
done
[[ "$locked" -eq 1 ]] || abort restart "Ready without import, but mLockTaskModeState is not LOCKED (saw: $shell_log)"
pass restart "Ready again without import ($shell_log), mLockTaskModeState=LOCKED"

# teardown and the total line come from the EXIT trap
exit 0
