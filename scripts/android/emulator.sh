#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - the Android emulator of the build chain (PD81)
#
# Usage: scripts/android/emulator.sh ensure|status|stop
#
#   ensure   reuse the running emulator of the AVD, or start it headless; waits for
#            sys.boot_completed and sets the three animation scales to 0. Prints the
#            serial (emulator-NNNN) on stdout and nothing else; progress goes to stderr.
#            Exit 3 when a running emulator is another AVD (a second emulator is never
#            started), exit 1 when the emulator does not come up.
#   status   prints three lines: serial (or none), avd, started-by-chain (yes/no)
#   stop     stops the emulator only when the chain started it (the marker file exists,
#            its PID is alive and is the emulator of the AVD): adb emu kill, waits up to
#            60 s, removes the marker. Otherwise it says so and leaves everything alone
#            (exit 0). It never kills, wipes or cold-boots another emulator.
#
# Environment: PILI_AVD (default sunmi_d3_pro_api33), ANDROID_HOME. State directory:
# $HOME/.cache/pili-android/emulator/ (lock.d, started-by-chain, emulator.log).
# Only one emulator runs at a time (16 GB host). Concurrent calls are serialised by a
# mkdir lock (waits up to 600 s, breaks a lock older than 900 s).
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

AVD="${PILI_AVD:-sunmi_d3_pro_api33}"
ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
STATE="$HOME/.cache/pili-android/emulator"
LOCK="$STATE/lock.d"
MARK="$STATE/started-by-chain"
LOG="$STATE/emulator.log"
EMU="$ANDROID_HOME/emulator/emulator"
ADB="$ANDROID_HOME/platform-tools/adb"
[[ -x "$ADB" ]] || ADB="$(command -v adb || true)"

log() { printf 'emulator.sh: %s\n' "$*" >&2; }
die() { # <code> <message>
    log "$2"
    exit "$1"
}

usage() { echo "usage: emulator.sh ensure|status|stop" >&2; exit 2; }
[[ $# -eq 1 ]] || usage
CMD="$1"
case "$CMD" in
ensure | status | stop) ;;
-h | --help)
    sed -n '2,/^# ──*$/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
    exit 0
    ;;
*) usage ;;
esac

[[ -n "$ADB" && -x "$ADB" ]] || die 2 "adb not found (ANDROID_HOME=$ANDROID_HOME)"
mkdir -p "$STATE"

# ── helpers ─────────────────────────────────────────────────────
now() { date +%s; }

file_mtime() { stat -f %m "$1" 2>/dev/null || stat -c %Y "$1" 2>/dev/null || echo 0; }

HAVE_LOCK=0
release_lock() { [[ "$HAVE_LOCK" -eq 0 ]] || { rmdir "$LOCK" 2>/dev/null || rm -rf "$LOCK"; HAVE_LOCK=0; }; }
trap release_lock EXIT

acquire_lock() {
    local deadline=$(($(now) + 600)) age
    while ! mkdir "$LOCK" 2>/dev/null; do
        age=$(($(now) - $(file_mtime "$LOCK")))
        if [[ "$age" -gt 900 ]]; then
            log "breaking a lock older than 900 s ($age s)"
            rm -rf "$LOCK"
            continue
        fi
        [[ "$(now)" -lt "$deadline" ]] || die 1 "could not take $LOCK within 600 s"
        sleep 2
    done
    HAVE_LOCK=1
}

# emulator_serials: serials of emulator-* devices (state device or offline).
emulator_serials() { "$ADB" devices 2>/dev/null | awk '$1 ~ /^emulator-/ && ($2 == "device" || $2 == "offline") { print $1 }'; }

avd_of() { "$ADB" -s "$1" emu avd name 2>/dev/null | head -n 1 | tr -d '\r'; }

boot_done() { [[ "$("$ADB" -s "$1" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r\n')" == "1" ]]; }

# marker_field <key>: value of key=value in the marker file.
marker_field() { sed -n "s/^$1=//p" "$MARK" 2>/dev/null | head -n 1; }

alive() { [[ -n "${1:-}" ]] && kill -0 "$1" 2>/dev/null; }

# chain_owns: the marker exists, its PID is alive and is the emulator of the AVD.
chain_owns() {
    local pid cmd
    [[ -f "$MARK" ]] || return 1
    pid="$(marker_field pid)"
    alive "$pid" || return 1
    cmd="$(ps -p "$pid" -o command= 2>/dev/null)"
    [[ "$cmd" == *"-avd $AVD"* || "$cmd" == *"@$AVD"* ]]
}

# find_expected: sets FOUND to the serial of the running emulator of the AVD and returns
# 0; otherwise FOUND is empty and OTHER_AVD names another running AVD, if any. Called
# without command substitution so that the variables survive.
FOUND=""
OTHER_AVD=""
find_expected() {
    local s name
    FOUND=""
    OTHER_AVD=""
    for s in $(emulator_serials); do
        name="$(avd_of "$s")"
        if [[ "$name" == "$AVD" ]]; then
            FOUND="$s"
            return 0
        fi
        OTHER_AVD="${OTHER_AVD:-$name ($s)}"
    done
    return 1
}

wait_boot() { # <serial> <seconds>
    local deadline=$(($(now) + $2))
    until boot_done "$1"; do
        [[ "$(now)" -lt "$deadline" ]] || return 1
        sleep 3
    done
}

# ── commands ────────────────────────────────────────────────────
do_ensure() {
    local serial pid t0 deadline
    acquire_lock
    t0="$(now)"
    if find_expected; then
        serial="$FOUND"
        log "$AVD already runs as $serial"
        wait_boot "$serial" 300 || die 1 "$serial did not finish booting within 300 s"
        printf '%s\n' "$serial"
        return 0
    fi
    [[ -z "$OTHER_AVD" ]] || die 3 "another emulator is running: $OTHER_AVD; not starting a second one"
    [[ -x "$EMU" ]] || die 2 "emulator binary not found: $EMU"
    "$EMU" -list-avds 2>/dev/null | grep -qx "$AVD" || die 2 "no AVD named $AVD"

    log "starting $AVD headless (log $LOG)"
    : >"$LOG"
    nohup "$EMU" -avd "$AVD" -no-window -no-audio -no-snapshot-save -no-boot-anim >>"$LOG" 2>&1 </dev/null &
    pid=$!
    disown "$pid" 2>/dev/null || true
    printf 'pid=%s\nstarted=%s\n' "$pid" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >"$MARK"

    deadline=$(($(now) + 180))
    serial=""
    while [[ -z "$serial" ]]; do
        alive "$pid" || { rm -f "$MARK"; die 1 "the emulator process ended early; see $LOG"; }
        [[ "$(now)" -lt "$deadline" ]] || die 1 "no device within 180 s (emulator pid $pid kept; run emulator.sh stop)"
        if find_expected; then serial="$FOUND"; else sleep 2; fi
    done
    printf 'serial=%s\n' "$serial" >>"$MARK"
    log "device $serial is up after $(($(now) - t0)) s, waiting for sys.boot_completed"
    wait_boot "$serial" 300 || die 1 "$serial did not finish booting within 300 s (emulator pid $pid kept; run emulator.sh stop)"
    local key
    for key in window_animation_scale transition_animation_scale animator_duration_scale; do
        "$ADB" -s "$serial" shell settings put global "$key" 0 >/dev/null 2>&1 || log "could not set $key"
    done
    log "booted in $(($(now) - t0)) s"
    printf '%s\n' "$serial"
}

do_status() {
    local serial="" owner="no"
    find_expected || true
    serial="$FOUND"
    if [[ -z "$serial" && -n "$(emulator_serials)" ]]; then
        # another AVD is running: report it, it is never the chain's
        serial="$(emulator_serials | head -n 1)"
        printf 'serial: %s\navd: %s\nstarted-by-chain: no\n' "$serial" "$(avd_of "$serial")"
        return 0
    fi
    if [[ -z "$serial" ]]; then
        printf 'serial: none\navd: %s\nstarted-by-chain: no\n' "$AVD"
        return 0
    fi
    chain_owns && owner="yes (pid $(marker_field pid))"
    printf 'serial: %s\navd: %s\nstarted-by-chain: %s\n' "$serial" "$AVD" "$owner"
}

do_stop() {
    local pid serial deadline
    acquire_lock
    if [[ ! -f "$MARK" ]]; then
        echo "the emulator was not started by the chain (no $MARK); left as it is"
        return 0
    fi
    pid="$(marker_field pid)"
    if ! alive "$pid"; then
        rm -f "$MARK"
        echo "the emulator recorded in $MARK (pid $pid) is gone; marker removed"
        return 0
    fi
    if ! chain_owns; then
        echo "pid $pid is not the $AVD emulator; the emulator was not started by the chain; left as it is"
        return 0
    fi
    find_expected || true
    serial="$FOUND"
    [[ -n "$serial" ]] || serial="$(marker_field serial)"
    [[ -n "$serial" ]] || die 1 "no serial for pid $pid; cannot send adb emu kill"
    log "stopping $serial (pid $pid)"
    "$ADB" -s "$serial" emu kill >/dev/null 2>&1 || log "adb emu kill returned non-zero"
    deadline=$(($(now) + 60))
    while alive "$pid"; do
        [[ "$(now)" -lt "$deadline" ]] || die 1 "emulator pid $pid still alive after 60 s; marker kept"
        sleep 1
    done
    rm -f "$MARK"
    echo "stopped $serial (pid $pid)"
}

case "$CMD" in
ensure) do_ensure ;;
status) do_status ;;
stop) do_stop ;;
esac
