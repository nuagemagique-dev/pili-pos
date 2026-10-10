#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - run the Android app as a demo on the D3 Pro (GMS) emulator
#
# Usage: scripts/android/demo/demo.sh start [--no-build] | stop | reset | status
#   start   starts the emulator sunmi_d3_pro_gms_api33 WITH a window (reused when it runs),
#           builds the debug APK (skipped with --no-build when it exists), starts the FDM and
#           printer mocks on the host, installs the app as device owner, imports a development
#           bundle with the demo seed (Tokyo Sushi Bar menu) and opens the app. Running it again
#           keeps the data of the till (the app is updated in place).
#   stop    stops the mocks and the adb reverse rules; the emulator and the app stay.
#   reset   stop, then release the device owner and uninstall the app (fresh till on next start).
#   status  shows what runs.
# Tickets: the printer mock shows every printed ticket at http://127.0.0.1:19391/ (kitchen tickets:
# http://127.0.0.1:19393/).
# Nightly export: received files land in ~/.cache/pili-demo/exports (decrypt the backup with
# age -d -i ~/.cache/pili-demo/age-key.txt).
# PINs (demo seed): Gerant 1111, Elisa 2222, John 4444, Sammy 5555, Technicien 0097.
# Not a certification tool: the demo seed is not the SPF test set.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT" || exit 1
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
if [[ -z "${JAVA_HOME:-}" ]]; then JAVA_HOME="$(/usr/libexec/java_home -v 17 2>/dev/null || true)"; export JAVA_HOME; fi
ADB="$ANDROID_HOME/platform-tools/adb"
EMU="$ANDROID_HOME/emulator/emulator"
AVD="${PILI_DEMO_AVD:-sunmi_d3_pro_gms_api33}"
PKG=dev.nuagemagique.pilipos.debug
ADMIN="$PKG/dev.nuagemagique.pilipos.provisioning.PiliAdminReceiver"
MAIN="$PKG/dev.nuagemagique.pilipos.MainActivity"
TEARDOWN_ACTION=dev.nuagemagique.pilipos.debug.TEARDOWN
TEARDOWN_COMPONENT="$PKG/dev.nuagemagique.pilipos.debugtools.DebugTeardownReceiver"
INBOX="/sdcard/Android/data/$PKG/files/provisioning-inbox"
FDM_PORT=19380
PRINTER_PORT=19390
PRINTER_HTTP=19391
KITCHEN_PORT=19392
KITCHEN_HTTP=19393
EXPORT_PORT=9330
STATE="$HOME/.cache/pili-demo"
MOCKS="$ROOT/target/android-mocks/debug"
APK="$ROOT/android/app/build/outputs/apk/debug/app-debug.apk"
SEED="$ROOT/scripts/android/demo/seed_demo_tokyosushi.sql"
mkdir -p "$STATE"

say() { printf 'demo: %s\n' "$*"; }
die() { printf 'demo: ERROR: %s\n' "$*" >&2; exit 1; }
serial() { "$ADB" devices | awk '/^emulator-[0-9]+\tdevice$/{print $1}' | while read -r s; do
    [[ "$("$ADB" -s "$s" emu avd name 2>/dev/null | head -n 1 | tr -d '\r')" == "$AVD" ]] && echo "$s"; done | head -n 1; }
adbd() { "$ADB" -s "$SERIAL" "$@"; }
port_open() { nc -z 127.0.0.1 "$1" >/dev/null 2>&1; }

start_emulator() {
    SERIAL="$(serial)"
    if [[ -z "$SERIAL" ]]; then
        if pgrep -f "qemu-system.*-avd " >/dev/null && ! pgrep -f "qemu-system.*-avd $AVD" >/dev/null; then
            die "another emulator runs ($(pgrep -fl 'qemu-system' | grep -o '\-avd [^ ]*' | head -n 1)); stop it first (one emulator at a time)"
        fi
        say "starting the emulator $AVD (window)"
        nohup "$EMU" -avd "$AVD" -no-boot-anim -no-audio >"$STATE/emulator.log" 2>&1 &
        for _ in $(seq 1 90); do SERIAL="$(serial)"; [[ -n "$SERIAL" ]] && break; sleep 2; done
        [[ -n "$SERIAL" ]] || die "the emulator did not come up, see $STATE/emulator.log"
    fi
    for _ in $(seq 1 120); do
        [[ "$(adbd shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" == 1 ]] && break; sleep 2
    done
    adbd shell svc power stayon true >/dev/null 2>&1
    say "emulator $SERIAL ready"
}

build_apk() {
    if [[ "${NO_BUILD:-0}" == 1 && -f "$APK" ]]; then say "using the existing APK"; return; fi
    say "building the debug APK (a few minutes the first time)"
    (cd android && ./gradlew --console=plain --offline -Dorg.gradle.jvmargs="-Xmx3g -XX:MaxMetaspaceSize=1g" :app:assembleDebug) \
        >"$STATE/gradle.log" 2>&1 || { grep -E "What went wrong|error:|e: " "$STATE/gradle.log" | head -n 15; die "gradle failed, see $STATE/gradle.log"; }
    [[ -f "$APK" ]] || die "APK missing: $APK"
}

start_mocks() {
    if [[ ! -x "$MOCKS/pos-fdm-mock" || ! -x "$MOCKS/pos-printer-mock" || ! -x "$MOCKS/pos-export-mock" ]]; then
        say "building the mocks"
        cargo build --locked -p pos-fdm-mock -p pos-printer-mock -p pos-export-mock --target-dir "$ROOT/target/android-mocks" \
            >"$STATE/mocks-build.log" 2>&1 || die "mock build failed, see $STATE/mocks-build.log"
    fi
    if ! port_open "$FDM_PORT"; then
        PORT="$FDM_PORT" nohup "$MOCKS/pos-fdm-mock" >"$STATE/fdm-mock.log" 2>&1 &
        echo $! >"$STATE/fdm-mock.pid"
    fi
    if ! port_open "$PRINTER_PORT"; then
        PORT="$PRINTER_PORT" HTTP_PORT="$PRINTER_HTTP" nohup "$MOCKS/pos-printer-mock" >"$STATE/printer-mock.log" 2>&1 &
        echo $! >"$STATE/printer-mock.pid"
    fi
    if ! port_open "$KITCHEN_PORT"; then
        PORT="$KITCHEN_PORT" HTTP_PORT="$KITCHEN_HTTP" nohup "$MOCKS/pos-printer-mock" >"$STATE/kitchen-mock.log" 2>&1 &
        echo $! >"$STATE/kitchen-mock.pid"
    fi
    # nightly export receiver; the demo key pair and token stay in $STATE (never in git)
    [[ -f "$STATE/age-key.txt" ]] || age-keygen -o "$STATE/age-key.txt" >/dev/null 2>&1 || die "age-keygen missing (brew install age)"
    [[ -f "$STATE/export-token" ]] || openssl rand -hex 16 >"$STATE/export-token"
    if ! port_open "$EXPORT_PORT"; then
        nohup "$MOCKS/pos-export-mock" --listen "127.0.0.1:$EXPORT_PORT" --token-file "$STATE/export-token" \
            --dir "$STATE/exports" >"$STATE/export-mock.log" 2>&1 &
        echo $! >"$STATE/export-mock.pid"
    fi
    for _ in $(seq 1 20); do port_open "$FDM_PORT" && port_open "$PRINTER_PORT" && port_open "$PRINTER_HTTP" && port_open "$KITCHEN_PORT" && break; sleep 1; done
    port_open "$FDM_PORT" && port_open "$PRINTER_PORT" || die "a mock does not listen, see $STATE/*-mock.log"
    adbd reverse "tcp:$FDM_PORT" "tcp:$FDM_PORT" >/dev/null &&
        adbd reverse "tcp:$PRINTER_PORT" "tcp:$PRINTER_PORT" >/dev/null &&
        adbd reverse "tcp:$PRINTER_HTTP" "tcp:$PRINTER_HTTP" >/dev/null &&
        adbd reverse "tcp:$KITCHEN_PORT" "tcp:$KITCHEN_PORT" >/dev/null &&
        adbd reverse "tcp:$EXPORT_PORT" "tcp:$EXPORT_PORT" >/dev/null || die "adb reverse failed"
    say "mocks up: FDM :$FDM_PORT, printer :$PRINTER_PORT, tickets http://127.0.0.1:$PRINTER_HTTP/ , kitchen http://127.0.0.1:$KITCHEN_HTTP/"
}

installed() { adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -qx "package:$PKG"; }
is_owner() { adbd shell dpm list-owners 2>/dev/null | tr -d '\r' | grep -q "$PKG"; }

tap_text() { # tap the first node whose text is one of the arguments; returns 1 when none is on screen
    adbd shell uiautomator dump /data/local/tmp/pili-ui.xml >/dev/null 2>&1 || return 1
    local b t xml
    xml="$(adbd shell cat /data/local/tmp/pili-ui.xml 2>/dev/null | tr '>' '\n')"
    for t in "$@"; do b="$(echo "$xml" | grep -F "text=\"$t\"" | head -n 1 |
        sed -nE 's/.*bounds="\[([0-9]+),([0-9]+)\]\[([0-9]+),([0-9]+)\]".*/\1 \2 \3 \4/p')"; [[ -n "$b" ]] && break; done
    [[ -n "$b" ]] || return 1
    read -r x1 y1 x2 y2 <<<"$b"
    adbd shell input tap $(((x1 + x2) / 2)) $(((y1 + y2) / 2))
}

install_and_provision() {
    local fresh=0
    if ! installed; then fresh=1; fi
    say "installing the app"
    out="$(adbd install -r "$APK" 2>&1 | tr -d '\r')"; [[ "$out" == *Success* ]] || die "adb install: $out"
    if ! is_owner; then
        for _ in 1 2 3 4 5 6; do
            o="$(adbd shell dpm set-device-owner "$ADMIN" 2>&1 | tr -d '\r')"; [[ "$o" == *Success* ]] && break; sleep 3
        done
        is_owner || die "device owner refused: $o (an account or a second user on the emulator? use reset, or wipe the AVD)"
    fi
    adbd shell cmd locale set-app-locales "$PKG" --locales fr-FR >/dev/null 2>&1
    adbd shell am start -n "$MAIN" >/dev/null 2>&1
    sleep 6
    if [[ "$fresh" == 1 ]] || adbd logcat -d -s PiliShell:I 2>/dev/null | tail -n 3 | grep -q 'state=NotProvisioned'; then
        say "provisioning the till with the demo seed (Tokyo Sushi Bar menu)"
        rm -rf "$STATE/bundle"
        PILI_BUNDLE_RESTAURANT_NAME="Tokyo Sushi Bar" PILI_BUNDLE_RESTAURANT_ADDRESS="Liège" \
            PILI_BUNDLE_KITCHEN_PRINTER="127.0.0.1:$KITCHEN_PORT" \
            PILI_BUNDLE_EXPORT_URL="http://127.0.0.1:$EXPORT_PORT" \
            PILI_BUNDLE_EXPORT_RECIPIENT="$(grep -o 'age1[a-z0-9]*' "$STATE/age-key.txt" | head -n 1)" \
            PILI_BUNDLE_EXPORT_TOKEN="$(cat "$STATE/export-token")" bash scripts/android/make_dev_bundle.sh --seed-file "$SEED" --out "$STATE/bundle" \
            --fdm-url "http://127.0.0.1:$FDM_PORT/graphql" --printer "127.0.0.1:$PRINTER_PORT" >"$STATE/bundle.log" 2>&1 ||
            die "make_dev_bundle failed, see $STATE/bundle.log"
        adbd shell mkdir -p "$INBOX"
        for f in "$STATE"/bundle/*; do adbd push "$f" "$INBOX/" >/dev/null 2>&1 || die "adb push $(basename "$f") failed"; done
        for _ in $(seq 1 20); do tap_text "Importer depuis la boîte de réception de test" "Import from the test inbox" && break; sleep 2; done
        sleep 8
    fi
    say "app open on the emulator"
}

stop_mocks() {
    for m in fdm-mock printer-mock kitchen-mock export-mock; do
        [[ -f "$STATE/$m.pid" ]] && kill "$(cat "$STATE/$m.pid")" 2>/dev/null; rm -f "$STATE/$m.pid"
    done
    SERIAL="$(serial)"; [[ -n "$SERIAL" ]] && adbd reverse --remove-all >/dev/null 2>&1
    say "mocks stopped"
}

case "${1:-}" in
    start)
        [[ "${2:-}" == "--no-build" ]] && NO_BUILD=1
        start_emulator; build_apk; start_mocks; install_and_provision
        say "ready. PIN gerant 1111 (cashiers 2222, 4444, 5555). Tickets: http://127.0.0.1:$PRINTER_HTTP/" ;;
    stop) stop_mocks ;;
    reset)
        stop_mocks; SERIAL="$(serial)"
        if [[ -n "$SERIAL" ]] && installed; then
            adbd shell am broadcast -a "$TEARDOWN_ACTION" -n "$TEARDOWN_COMPONENT" >/dev/null 2>&1; sleep 4
            adbd uninstall "$PKG" >/dev/null 2>&1
        fi
        say "app removed; the next start provisions a fresh till" ;;
    status)
        SERIAL="$(serial)"; say "emulator: ${SERIAL:-not running} ($AVD)"
        [[ -n "$SERIAL" ]] && { installed && say "app: installed$(is_owner && echo ', device owner')" || say "app: not installed"; }
        port_open "$FDM_PORT" && say "FDM mock: up" || say "FDM mock: down"
        port_open "$PRINTER_HTTP" && say "printer mock: up (http://127.0.0.1:$PRINTER_HTTP/)" || say "printer mock: down" ;;
    *) sed -n 2,17p "$0"; exit 2 ;;
esac
