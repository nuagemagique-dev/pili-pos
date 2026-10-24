#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - run the Android app as a demo on the D3 Pro (GMS) emulator
#
# Usage: scripts/android/demo/demo.sh start [--no-build] | stop | reset | status | order <platform> | card <result> [brand]
#   start   starts the emulator sunmi_d3_pro_gms_api33 WITH a window (reused when it runs),
#           builds the debug APK (skipped with --no-build when it exists), starts the FDM and
#           printer mocks on the host, installs the app as device owner, imports a development
#           bundle with the demo seed (Tokyo Sushi Bar menu) and opens the app. Running it again
#           keeps the data of the till (the app is updated in place).
#   stop    stops the mocks and the adb reverse rules; the emulator and the app stay.
#   reset   stop, then release the device owner and uninstall the app (fresh till on next start).
#   status  shows what runs.
#   order   sends one online order to the till through the HubRise mock: deliveroo, ubereats,
#           web (website, paid online) or cash (website, paid in cash at collection). A fresh till
#           gets one of each at start.
#   card    sets how the NEXT payment on the card terminal mock ends: approve (default brand
#           Bancontact), decline or unknown (the answer is lost: the till shows "Réponse du
#           terminal inconnue" and Vérifier then finds the payment approved). An optional brand
#           (Visa, Maestro, Mastercard...) follows. Without a card command the next payment is approved.
# Card terminal: the demo pos.json links a CCV terminal over the LAN (OPI, as the restaurant's CCV A920)
# that is the local mock pos-eft-mock: OPI channel 0 on :19410 (the till connects, adb reverse), channel 1
# on :19412 (the mock connects back to the till for the receipt, adb forward); the page
# http://127.0.0.1:19400/ and /mock/requests are its HTTP side; no real terminal. A till that already runs keeps its old pos.json: use reset to get the terminal.
# HubRise: switched on in the demo pos.json; the till is connected to the local HubRise mock
# (:3001, dev token, no real account), which serves the Tokyo menu as its catalog: the menu is
# managed by HubRise, as in the restaurant.
# Tickets: the printer mock shows every printed ticket at http://127.0.0.1:19391/ (kitchen tickets:
# http://127.0.0.1:19393/).
# Nightly export: received files land in ~/.cache/pili-demo/exports (decrypt the backup with
# age -d -i ~/.cache/pili-demo/age-key.txt).
# PINs (demo seed): Mengmeng Z. (manager) 1111, Sa X. (server) 2222.
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
HUBRISE_PORT=3001
EFT_PORT="${PILI_EFT_MOCK_PORT:-19400}"
EFT_OPI_PORT="${PILI_EFT_MOCK_OPI_PORT:-19410}"
EFT_DEVICE_PORT="${PILI_EFT_MOCK_OPI_DEVICE_PORT:-19412}"
STATE="$HOME/.cache/pili-demo"
MOCKS="$ROOT/target/android-mocks/debug"
APK="$ROOT/android/app/build/outputs/apk/debug/app-debug.apk"
SEED="$ROOT/scripts/android/demo/seed_demo_tokyosushi.sql"
CATALOG="$ROOT/scripts/android/demo/hubrise_catalog_tokyosushi.json"
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
    if [[ ! -x "$MOCKS/pos-fdm-mock" || ! -x "$MOCKS/pos-printer-mock" || ! -x "$MOCKS/pos-export-mock" || ! -x "$MOCKS/pos-hubrise-mock" || ! -x "$MOCKS/pos-eft-mock" ]]; then
        say "building the mocks"
        cargo build --locked -p pos-fdm-mock -p pos-printer-mock -p pos-export-mock -p pos-hubrise-mock -p pos-eft-mock --target-dir "$ROOT/target/android-mocks" \
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
    local hubrise_new=0
    if ! port_open "$HUBRISE_PORT"; then
        HUBRISE_MOCK_PORT="$HUBRISE_PORT" nohup "$MOCKS/pos-hubrise-mock" >"$STATE/hubrise-mock.log" 2>&1 &
        echo $! >"$STATE/hubrise-mock.pid"
        hubrise_new=1
    fi
    if ! port_open "$EFT_PORT"; then
        PILI_EFT_MOCK_PORT="$EFT_PORT" PILI_EFT_MOCK_OPI_PORT="$EFT_OPI_PORT" PILI_EFT_MOCK_OPI_DEVICE_PORT="$EFT_DEVICE_PORT" \
            nohup "$MOCKS/pos-eft-mock" >"$STATE/eft-mock.log" 2>&1 &
        echo $! >"$STATE/eft-mock.pid"
    fi
    for _ in $(seq 1 20); do port_open "$EFT_PORT" && port_open "$EFT_OPI_PORT" && port_open "$FDM_PORT" && port_open "$PRINTER_PORT" && port_open "$PRINTER_HTTP" && port_open "$KITCHEN_PORT" && port_open "$HUBRISE_PORT" && break; sleep 1; done
    port_open "$FDM_PORT" && port_open "$PRINTER_PORT" && port_open "$HUBRISE_PORT" && port_open "$EFT_PORT" && port_open "$EFT_OPI_PORT" || die "a mock does not listen, see $STATE/*-mock.log"
    # The HubRise mock starts with its own sample catalog: it serves the Tokyo menu instead (the
    # till's catalog sync then finds its own menu, every row keyed by the seed).
    if [[ "$hubrise_new" == 1 ]]; then
        python3 -I -c 'import json,sys; print(json.dumps({"data": json.load(open(sys.argv[1]))}))' "$CATALOG" |
            curl -sf -X PUT "http://127.0.0.1:$HUBRISE_PORT/mock/catalog" -H 'content-type: application/json' -d @- >/dev/null ||
            die "the HubRise mock refused the demo catalog, see $STATE/hubrise-mock.log"
    fi
    adbd reverse "tcp:$FDM_PORT" "tcp:$FDM_PORT" >/dev/null &&
        adbd reverse "tcp:$PRINTER_PORT" "tcp:$PRINTER_PORT" >/dev/null &&
        adbd reverse "tcp:$PRINTER_HTTP" "tcp:$PRINTER_HTTP" >/dev/null &&
        adbd reverse "tcp:$KITCHEN_PORT" "tcp:$KITCHEN_PORT" >/dev/null &&
        adbd reverse "tcp:$EXPORT_PORT" "tcp:$EXPORT_PORT" >/dev/null &&
        adbd reverse "tcp:$HUBRISE_PORT" "tcp:$HUBRISE_PORT" >/dev/null &&
        adbd reverse "tcp:$EFT_PORT" "tcp:$EFT_PORT" >/dev/null &&
        adbd reverse "tcp:$EFT_OPI_PORT" "tcp:$EFT_OPI_PORT" >/dev/null &&
        adbd forward "tcp:$EFT_DEVICE_PORT" "tcp:$EFT_DEVICE_PORT" >/dev/null || die "adb reverse or forward failed"
    say "mocks up: FDM :$FDM_PORT, printer :$PRINTER_PORT, tickets http://127.0.0.1:$PRINTER_HTTP/ , kitchen http://127.0.0.1:$KITCHEN_HTTP/ , HubRise :$HUBRISE_PORT, card terminal (CCV, OPI) :$EFT_OPI_PORT, page http://127.0.0.1:$EFT_PORT/"
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
            PILI_BUNDLE_KITCHEN_PRINTER="127.0.0.1:$KITCHEN_PORT" PILI_BUNDLE_HUBRISE_ENABLED=1 \
            PILI_BUNDLE_EXPORT_URL="http://127.0.0.1:$EXPORT_PORT" \
            PILI_BUNDLE_PAYMENT_TERMINAL_OPI="127.0.0.1:$EFT_OPI_PORT" PILI_BUNDLE_PAYMENT_TERMINAL_DEVICE_PORT="$EFT_DEVICE_PORT" \
            PILI_BUNDLE_EXPORT_RECIPIENT="$(grep -o 'age1[a-z0-9]*' "$STATE/age-key.txt" | head -n 1)" \
            PILI_BUNDLE_EXPORT_TOKEN="$(cat "$STATE/export-token")" bash scripts/android/make_dev_bundle.sh --seed-file "$SEED" --out "$STATE/bundle" \
            --fdm-url "http://127.0.0.1:$FDM_PORT/graphql" --printer "127.0.0.1:$PRINTER_PORT" >"$STATE/bundle.log" 2>&1 ||
            die "make_dev_bundle failed, see $STATE/bundle.log"
        adbd shell mkdir -p "$INBOX"
        for f in "$STATE"/bundle/*; do adbd push "$f" "$INBOX/" >/dev/null 2>&1 || die "adb push $(basename "$f") failed"; done
        for _ in $(seq 1 20); do tap_text "Importer depuis la boîte de réception de test" "Import from the test inbox" && break; sleep 2; done
        sleep 8
        for p in deliveroo ubereats web cash; do send_order "$p"; done
    fi
    say "app open on the emulator"
}

# One online order for the demo, through the HubRise mock (its control API), on products of the
# demo seed (HubRise ref TSB-<menu item id>). Prices are the menu's; the webshop lines carry the
# takeaway VAT rate (food and soft drinks 6 %, beer 21 %), platform lines none.
send_order() {
    local body
    body="$(python3 -I - "$1" <<'PY'
import json, sys

def eur(c):
    return f"{c // 100}.{c % 100:02d} EUR"

def item(ref, name, cents, qty, rate=None):
    i = {"product_name": name, "sku_ref": ref, "price": eur(cents), "quantity": str(qty), "options": []}
    if rate:
        i["tax_rate"] = rate
    return i

def total(items):
    return sum(int(i["price"].split()[0].replace(".", "")) * int(i["quantity"]) for i in items)

kind = sys.argv[1]
if kind == "deliveroo":
    items = [item("TSB-127", "Poke saumon", 1380, 2), item("TSB-152", "Coca-Cola", 250, 1)]
    order = {"channel": "Deliveroo", "service_type": "delivery", "collection_code": "DLV-4821",
             "customer": {"first_name": "Lucie", "last_name": "M."},
             "payments": [{"ref": "DELIVEROO", "name": "Deliveroo", "amount": eur(total(items))}]}
elif kind == "ubereats":
    items = [item("TSB-81", "California saumon avocat", 560, 2), item("TSB-129", "Soupe miso", 400, 2),
             item("TSB-136", "Raviolis poulet x6", 680, 1)]
    order = {"channel": "Uber Eats", "service_type": "delivery",
             "customer": {"first_name": "Thomas"},
             "payments": [{"ref": "UBER_EATS", "name": "Uber Eats", "amount": eur(total(items))}]}
elif kind == "web":
    items = [item("TSB-6", "Plateau saumon", 2180, 1, "6.0"), item("TSB-160", "Bière Asahi", 390, 2, "21.0")]
    order = {"channel": "Website", "service_type": "collection",
             "customer": {"first_name": "Sophie", "last_name": "Lambert", "phone": "+32470000000"},
             "payments": [{"ref": "ONLINE", "name": "Paiement en ligne", "amount": eur(total(items))}]}
elif kind == "cash":
    items = [item("TSB-50", "Maki saumon", 470, 2, "6.0"), item("TSB-148", "Edamame", 350, 1, "6.0"),
             item("TSB-158", "Thé glacé litchi", 380, 1, "6.0")]
    order = {"channel": "Website", "service_type": "collection",
             "customer": {"first_name": "Marc", "last_name": "Dubois", "phone": "+32490000000"},
             "payments": []}
else:
    sys.exit(f"unknown platform {kind} (deliveroo, ubereats, web, cash)")
order.update({"status": "new", "asap": True, "items": items, "charges": [], "discounts": []})
print(json.dumps({"order": order}))
PY
)" || die "unknown platform ${1:-} (deliveroo, ubereats, web, cash)"
    curl -sf -X POST "http://127.0.0.1:$HUBRISE_PORT/mock/orders" -H 'content-type: application/json' -d "$body" >/dev/null ||
        die "the HubRise mock refused the $1 order (is it up? see $STATE/hubrise-mock.log)"
    say "online order sent: $1"
}

# How the next payment on the card terminal mock ends (its control API, one answer per payment).
# unknown = the answer is lost (the mock's "timeout"); the check then finds the payment approved.
set_card() {
    local result="${1:-}" brand="${2:-}" body
    case "$result" in
        approve | decline) ;;
        unknown) result=timeout ;;
        *) die "usage: demo.sh card approve|decline|unknown [brand]" ;;
    esac
    [[ -z "$brand" || "$brand" =~ ^[A-Za-z0-9\ ._-]+$ ]] || die "unsafe brand '$brand'"
    body="{\"result\":\"$result\",\"delay_ms\":3000"
    [[ -n "$brand" ]] && body="$body,\"brand\":\"$brand\""
    body="$body}"
    curl -sf -X POST "http://127.0.0.1:$EFT_PORT/mock/next" -H 'content-type: application/json' -d "$body" >/dev/null ||
        die "the card terminal mock refused the answer (is it up? see $STATE/eft-mock.log)"
    say "next card payment: ${1}${brand:+ ($brand)}"
}

stop_mocks() {
    for m in fdm-mock printer-mock kitchen-mock export-mock hubrise-mock eft-mock; do
        [[ -f "$STATE/$m.pid" ]] && kill "$(cat "$STATE/$m.pid")" 2>/dev/null; rm -f "$STATE/$m.pid"
    done
    SERIAL="$(serial)"; [[ -n "$SERIAL" ]] && { adbd reverse --remove-all; adbd forward --remove "tcp:$EFT_DEVICE_PORT"; } >/dev/null 2>&1
    say "mocks stopped"
}

case "${1:-}" in
    start)
        [[ "${2:-}" == "--no-build" ]] && NO_BUILD=1
        start_emulator; build_apk; start_mocks; install_and_provision
        say "ready. PIN Mengmeng Z. 1111 (manager), Sa X. 2222 (server). Tickets: http://127.0.0.1:$PRINTER_HTTP/" ;;
    stop) stop_mocks ;;
    reset)
        stop_mocks; SERIAL="$(serial)"
        # The app lives in the emulator: without it running, nothing can be removed.
        [[ -n "$SERIAL" ]] || die "the emulator is not running: run start, then reset, then start again"
        if installed; then
            adbd shell am broadcast -a "$TEARDOWN_ACTION" -n "$TEARDOWN_COMPONENT" >/dev/null 2>&1; sleep 4
            adbd uninstall "$PKG" >/dev/null 2>&1
        fi
        say "app removed; the next start provisions a fresh till" ;;
    status)
        SERIAL="$(serial)"; say "emulator: ${SERIAL:-not running} ($AVD)"
        [[ -n "$SERIAL" ]] && { installed && say "app: installed$(is_owner && echo ', device owner')" || say "app: not installed"; }
        port_open "$FDM_PORT" && say "FDM mock: up" || say "FDM mock: down"
        port_open "$PRINTER_HTTP" && say "printer mock: up (http://127.0.0.1:$PRINTER_HTTP/)" || say "printer mock: down"
        port_open "$HUBRISE_PORT" && say "HubRise mock: up" || say "HubRise mock: down"
        port_open "$EFT_PORT" && say "card terminal mock (CCV, OPI :$EFT_OPI_PORT): up (http://127.0.0.1:$EFT_PORT/)" || say "card terminal mock: down" ;;
    order)
        port_open "$HUBRISE_PORT" || die "the HubRise mock is down: run start first"
        send_order "${2:-}" ;;
    card)
        port_open "$EFT_PORT" || die "the card terminal mock is down: run start first"
        set_card "${2:-}" "${3:-}" ;;
    *) sed -n 2,20p "$0"; exit 2 ;;
esac
