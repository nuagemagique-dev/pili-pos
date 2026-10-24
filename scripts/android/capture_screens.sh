#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - bench helper for the screen captures of the Android dossier (P8b)
#
# Usage: scripts/android/capture_screens.sh up [--terminal] | down | reset
#                                           | shot NAME | tap TEXT | tapat X Y | text STRING | key KEYCODE
#                                           | ui | has TEXT | fdm KEY=VALUE... | card approve|decline|unknown [brand]
#   up         starts the emulator sunmi_d3_pro_gms_api33 (headless), the FDM, printer, kitchen printer, export
#              receiver and HubRise mocks of the host, installs the debug APK as device owner and imports the
#              signed development bundle with the SPF test seed (Gerant Test 1111, Elisa 2222, Apprenti Test 3333).
#              Running it again on a provisioned till keeps its data. With --terminal the bundle also links the
#              card terminal mock pos-eft-mock (CCV, OPI), as scripts/android/demo/demo.sh does.
#   down       stops the mocks and removes the adb rules (the emulator and the app stay).
#   reset      down, then releases the device owner and uninstalls the app (a fresh till at the next up).
#   shot NAME  full-screen PNG of the emulator, never cropped, into $PILI_SHOTS (default
#              docs/android/dossier/screenshots) as NAME.png.
#   tap TEXT   taps the first node whose text (else content description) is TEXT; fails when none is on screen.
#   tapat X Y  taps a pixel. text STRING types into the focused field. key sends an Android key code.
#   ui         prints the visible texts with their bounds. has TEXT succeeds when TEXT is on screen.
#   fdm ...    POST /mock/control on the FDM mock, e.g. fdm errors=FDM_LOCKED, fdm warnings=BUFFER_NEAR_FULL,
#              fdm errors= warnings= (clears), fdm initialized=false.
#   printer off|on  stops or restarts the ticket printer mock AND removes or restores its adb reverse rules: the device
#              must refuse the connection, otherwise a dead host behind adb reverse looks like a printer that accepts the job.
#   fdmproc off|on  stops or restarts the FDM mock process and its adb reverse rule (the till then sees a refused connection).
#   card ...   sets how the NEXT card terminal payment ends (pos-eft-mock /mock/next), as demo.sh card.
# Environment: PILI_UP_IMPORT=0 (up stops before the bundle import: the "Mise en service" screen), PILI_UP_SEED_FILE (a seed
#   SQL file instead of the SPF test seed), PILI_BUNDLE_* (read by make_dev_bundle.sh, see its header), PILI_SHOTS, ANDROID_HOME, JAVA_HOME (not needed here). The APK is android/app/build/outputs/apk/debug/app-debug.apk
# and the mocks target/android-mocks/debug (built by demo.sh or by `cargo build -p pos-fdm-mock ... --target-dir target/android-mocks`).
# State, pids and logs: $HOME/.cache/pili-capture. Mocks only: no call to a real vendor.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT" || exit 1
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
ADB="$ANDROID_HOME/platform-tools/adb"
AVD=sunmi_d3_pro_gms_api33
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
HUBRISE_PORT=3001
EFT_PORT=19400
EFT_OPI_PORT=19410
EFT_DEVICE_PORT=19412
STATE="$HOME/.cache/pili-capture"
MOCKS="$ROOT/target/android-mocks/debug"
APK="$ROOT/android/app/build/outputs/apk/debug/app-debug.apk"
SHOTS="${PILI_SHOTS:-$ROOT/docs/android/dossier/screenshots}"
mkdir -p "$STATE"

say() { printf 'capture: %s\n' "$*"; }
die() { printf 'capture: ERROR: %s\n' "$*" >&2; exit 1; }
serial() { "$ADB" devices | awk '/^emulator-[0-9]+\tdevice$/{print $1}' | head -n 1; }
SERIAL="$(serial)"
adbd() { "$ADB" -s "$SERIAL" "$@"; }
port_open() { nc -z 127.0.0.1 "$1" >/dev/null 2>&1; }

ui_dump() { # prints "text<TAB>desc<TAB>x1 y1 x2 y2" for every node that has a text or a description
    adbd shell uiautomator dump /data/local/tmp/pili-ui.xml >/dev/null 2>&1 || return 1
    adbd exec-out cat /data/local/tmp/pili-ui.xml | python3 -I -c '
import re, sys, html
x = sys.stdin.read()
for m in re.finditer(r"<node ([^>]*)>", x):
    a = dict(re.findall(r"([\w-]+)=\"([^\"]*)\"", m.group(1)))
    t, d = html.unescape(a.get("text", "")), html.unescape(a.get("content-desc", ""))
    b = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", a.get("bounds", ""))
    if (t or d) and b:
        print("%s\t%s\t%s %s %s %s" % (t.replace("\n", " / "), d.replace("\n", " / "), *b.groups()))
'
}

tap_text() {
    local line b
    line="$(ui_dump | awk -F'\t' -v t="$1" '$1 == t || ($1 == "" && $2 == t) {print; exit}')"
    [[ -n "$line" ]] || return 1
    b="${line##*$'\t'}"
    read -r x1 y1 x2 y2 <<<"$b"
    adbd shell input tap $(((x1 + x2) / 2)) $(((y1 + y2) / 2))
}

start_emulator() {
    if [[ -z "$SERIAL" ]]; then
        PILI_AVD="$AVD" bash scripts/android/emulator.sh ensure >/dev/null || die "emulator.sh ensure failed"
        SERIAL="$(serial)"
    fi
    [[ -n "$SERIAL" ]] || die "no emulator"
    adbd shell svc power stayon true >/dev/null 2>&1
    say "emulator $SERIAL ready"
}

start_mocks() {
    local m
    for m in pos-fdm-mock pos-printer-mock pos-export-mock pos-hubrise-mock pos-eft-mock; do
        [[ -x "$MOCKS/$m" ]] || die "$MOCKS/$m missing: run scripts/android/demo/demo.sh start once, or cargo build -p $m --target-dir target/android-mocks"
    done
    port_open "$FDM_PORT" || { PORT="$FDM_PORT" nohup "$MOCKS/pos-fdm-mock" >"$STATE/fdm-mock.log" 2>&1 & echo $! >"$STATE/fdm-mock.pid"; }
    port_open "$PRINTER_PORT" || { PORT="$PRINTER_PORT" HTTP_PORT="$PRINTER_HTTP" nohup "$MOCKS/pos-printer-mock" >"$STATE/printer-mock.log" 2>&1 & echo $! >"$STATE/printer-mock.pid"; }
    port_open "$KITCHEN_PORT" || { PORT="$KITCHEN_PORT" HTTP_PORT="$KITCHEN_HTTP" nohup "$MOCKS/pos-printer-mock" >"$STATE/kitchen-mock.log" 2>&1 & echo $! >"$STATE/kitchen-mock.pid"; }
    [[ -f "$STATE/age-key.txt" ]] || age-keygen -o "$STATE/age-key.txt" >/dev/null 2>&1 || die "age-keygen missing (brew install age)"
    [[ -f "$STATE/export-token" ]] || openssl rand -hex 16 >"$STATE/export-token"
    port_open "$EXPORT_PORT" || { nohup "$MOCKS/pos-export-mock" --listen "127.0.0.1:$EXPORT_PORT" --token-file "$STATE/export-token" --dir "$STATE/exports" >"$STATE/export-mock.log" 2>&1 & echo $! >"$STATE/export-mock.pid"; }
    port_open "$HUBRISE_PORT" || { HUBRISE_MOCK_PORT="$HUBRISE_PORT" nohup "$MOCKS/pos-hubrise-mock" >"$STATE/hubrise-mock.log" 2>&1 & echo $! >"$STATE/hubrise-mock.pid"; }
    port_open "$EFT_PORT" || { PILI_EFT_MOCK_PORT="$EFT_PORT" PILI_EFT_MOCK_OPI_PORT="$EFT_OPI_PORT" PILI_EFT_MOCK_OPI_DEVICE_PORT="$EFT_DEVICE_PORT" nohup "$MOCKS/pos-eft-mock" >"$STATE/eft-mock.log" 2>&1 & echo $! >"$STATE/eft-mock.pid"; }
    for _ in $(seq 1 20); do port_open "$FDM_PORT" && port_open "$PRINTER_HTTP" && port_open "$KITCHEN_HTTP" && port_open "$HUBRISE_PORT" && port_open "$EFT_OPI_PORT" && break; sleep 1; done
    port_open "$FDM_PORT" && port_open "$PRINTER_PORT" && port_open "$HUBRISE_PORT" && port_open "$EFT_OPI_PORT" || die "a mock does not listen, see $STATE/*-mock.log"
    local p
    for p in $FDM_PORT $PRINTER_PORT $PRINTER_HTTP $KITCHEN_PORT $EXPORT_PORT $HUBRISE_PORT $EFT_PORT $EFT_OPI_PORT; do
        adbd reverse "tcp:$p" "tcp:$p" >/dev/null || die "adb reverse $p failed"
    done
    adbd forward "tcp:$EFT_DEVICE_PORT" "tcp:$EFT_DEVICE_PORT" >/dev/null || die "adb forward failed"
    say "mocks up (FDM $FDM_PORT, printer $PRINTER_PORT / http $PRINTER_HTTP, kitchen $KITCHEN_PORT, export $EXPORT_PORT, HubRise $HUBRISE_PORT, card terminal $EFT_OPI_PORT)"
}

installed() { adbd shell pm list packages 2>/dev/null | tr -d '\r' | grep -qx "package:$PKG"; }
is_owner() { adbd shell dpm list-owners 2>/dev/null | tr -d '\r' | grep -q "$PKG"; }

cmd_up() {
    local terminal=0 fresh=0
    [[ "${1:-}" == "--terminal" ]] && terminal=1
    start_emulator
    [[ -f "$APK" ]] || die "APK missing: $APK (cd android && ./gradlew --offline :app:assembleDebug)"
    start_mocks
    installed || fresh=1
    local out
    out="$(adbd install -r "$APK" 2>&1 | tr -d '\r')"
    [[ "$out" == *Success* ]] || die "adb install: $out"
    if ! is_owner; then
        local o
        for _ in 1 2 3 4 5 6; do
            o="$(adbd shell dpm set-device-owner "$ADMIN" 2>&1 | tr -d '\r')"; [[ "$o" == *Success* ]] && break; sleep 3
        done
        is_owner || die "device owner refused: $o"
    fi
    adbd shell cmd locale set-app-locales "$PKG" --locales fr-FR >/dev/null 2>&1
    adbd shell am start -n "$MAIN" >/dev/null 2>&1
    sleep 6
    if [[ "${PILI_UP_IMPORT:-1}" == 0 ]]; then
        say "up (no bundle imported: PILI_UP_IMPORT=0)"
        return 0
    fi
    if [[ "$fresh" == 1 ]] || adbd logcat -d -s PiliShell:I 2>/dev/null | tail -n 3 | grep -q 'state=NotProvisioned'; then
        say "importing the SPF test bundle"
        rm -rf "$STATE/bundle"
        local opi=""
        [[ "$terminal" == 1 ]] && opi="127.0.0.1:$EFT_OPI_PORT"
        PILI_BUNDLE_KITCHEN_PRINTER="127.0.0.1:$KITCHEN_PORT" PILI_BUNDLE_EXPORT_URL="http://127.0.0.1:$EXPORT_PORT" \
            PILI_BUNDLE_EXPORT_RECIPIENT="$(grep -o 'age1[a-z0-9]*' "$STATE/age-key.txt" | head -n 1)" \
            PILI_BUNDLE_EXPORT_TOKEN="$(cat "$STATE/export-token")" \
            PILI_BUNDLE_PAYMENT_TERMINAL_OPI="$opi" PILI_BUNDLE_PAYMENT_TERMINAL_DEVICE_PORT="$EFT_DEVICE_PORT" \
            bash scripts/android/make_dev_bundle.sh ${PILI_UP_SEED_FILE:+--seed-file "$PILI_UP_SEED_FILE"} --seed --out "$STATE/bundle" \
            --fdm-url "http://127.0.0.1:$FDM_PORT/graphql" --printer "127.0.0.1:$PRINTER_PORT" >"$STATE/bundle.log" 2>&1 ||
            die "make_dev_bundle failed, see $STATE/bundle.log"
        adbd shell mkdir -p "$INBOX"
        local f
        for f in "$STATE"/bundle/*; do adbd push "$f" "$INBOX/" >/dev/null 2>&1 || die "adb push $f failed"; done
        for _ in $(seq 1 20); do tap_text "Importer depuis la boîte de réception de test" && break; sleep 2; done
        sleep 8
    fi
    say "up"
}

cmd_down() {
    local m
    for m in fdm-mock printer-mock kitchen-mock export-mock hubrise-mock eft-mock; do
        [[ -f "$STATE/$m.pid" ]] && kill "$(cat "$STATE/$m.pid")" 2>/dev/null
        rm -f "$STATE/$m.pid"
    done
    [[ -n "$SERIAL" ]] && { adbd reverse --remove-all; adbd forward --remove "tcp:$EFT_DEVICE_PORT"; } >/dev/null 2>&1
    say "mocks stopped"
}

cmd_reset() {
    [[ -n "$SERIAL" ]] || die "the emulator is not running"
    cmd_down
    if installed; then
        adbd shell am broadcast -a "$TEARDOWN_ACTION" -n "$TEARDOWN_COMPONENT" >/dev/null 2>&1; sleep 4
        adbd uninstall "$PKG" >/dev/null 2>&1
    fi
    say "app removed"
}

[[ -n "${1:-}" ]] || { sed -n '2,/^# ──*$/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'; exit 2; }
cmd="$1"; shift
case "$cmd" in
    up) cmd_up "$@" ;;
    down) cmd_down ;;
    reset) cmd_reset ;;
    shot)
        [[ "${1:-}" =~ ^[A-Za-z0-9_.-]+$ ]] || die "usage: shot NAME"
        mkdir -p "$SHOTS"
        adbd exec-out screencap -p >"$SHOTS/$1.png" && say "$SHOTS/$1.png" ;;
    tap) tap_text "${1:-}" || die "no node with the text '${1:-}'" ;;
    tapat) adbd shell input tap "${1:?x}" "${2:?y}" ;;
    text) adbd shell input text "$(printf '%s' "${1:-}" | sed 's/ /%s/g')" ;;
    key) adbd shell input keyevent "${1:?code}" ;;
    ui) ui_dump ;;
    has) ui_dump | awk -F'\t' -v t="${1:-}" '$1 == t || $2 == t {f=1} END {exit !f}' ;;
    fdm)
        body="{"
        sep=""
        for kv in "$@"; do
            k="${kv%%=*}"; v="${kv#*=}"
            case "$k" in
                errors | warnings) arr=""; if [[ -n "$v" ]]; then arr="\"${v//,/\",\"}\""; fi; body="$body$sep\"$k\":[$arr]" ;;
                initialized) body="$body$sep\"$k\":$v" ;;
                *) die "unknown control $k" ;;
            esac
            sep=","
        done
        curl -sf -X POST "http://127.0.0.1:$FDM_PORT/mock/control" -H 'content-type: application/json' -d "$body}" >/dev/null || die "the FDM mock refused $body}"
        say "fdm control $body}" ;;
    printer)
        case "${1:-}" in
            off)
                [[ -f "$STATE/printer-mock.pid" ]] && kill "$(cat "$STATE/printer-mock.pid")" 2>/dev/null
                rm -f "$STATE/printer-mock.pid"
                adbd reverse --remove "tcp:$PRINTER_PORT" >/dev/null 2>&1
                adbd reverse --remove "tcp:$PRINTER_HTTP" >/dev/null 2>&1
                say "ticket printer off" ;;
            on)
                port_open "$PRINTER_PORT" || { PORT="$PRINTER_PORT" HTTP_PORT="$PRINTER_HTTP" nohup "$MOCKS/pos-printer-mock" >"$STATE/printer-mock.log" 2>&1 & echo $! >"$STATE/printer-mock.pid"; }
                sleep 1
                adbd reverse "tcp:$PRINTER_PORT" "tcp:$PRINTER_PORT" >/dev/null && adbd reverse "tcp:$PRINTER_HTTP" "tcp:$PRINTER_HTTP" >/dev/null || die "adb reverse failed"
                say "ticket printer on" ;;
            *) die "usage: printer off|on" ;;
        esac ;;
    fdmproc)
        case "${1:-}" in
            off)
                [[ -f "$STATE/fdm-mock.pid" ]] && kill "$(cat "$STATE/fdm-mock.pid")" 2>/dev/null
                rm -f "$STATE/fdm-mock.pid"
                adbd reverse --remove "tcp:$FDM_PORT" >/dev/null 2>&1
                say "FDM mock off" ;;
            on)
                port_open "$FDM_PORT" || { PORT="$FDM_PORT" nohup "$MOCKS/pos-fdm-mock" >"$STATE/fdm-mock.log" 2>&1 & echo $! >"$STATE/fdm-mock.pid"; }
                sleep 1
                adbd reverse "tcp:$FDM_PORT" "tcp:$FDM_PORT" >/dev/null || die "adb reverse failed"
                say "FDM mock on" ;;
            *) die "usage: fdmproc off|on" ;;
        esac ;;
    card)
        result="${1:-}"
        case "$result" in approve | decline) ;; unknown) result=timeout ;; *) die "usage: card approve|decline|unknown [brand]" ;; esac
        body="{\"result\":\"$result\",\"delay_ms\":3000"
        [[ -n "${2:-}" ]] && body="$body,\"brand\":\"$2\""
        curl -sf -X POST "http://127.0.0.1:$EFT_PORT/mock/next" -H 'content-type: application/json' -d "$body}" >/dev/null || die "the card terminal mock refused the answer"
        say "next card payment: $1" ;;
    *) die "unknown command $cmd" ;;
esac
