#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - P7 emulator lock (end-to-end lane, P7D10)
#
# One emulator, several P7 scripts (uc_e2e.sh, p7_soak.sh, p7_powercut.sh, p7_gms.sh).
# They must never use the device at the same time, nor while an older end-to-end script
# (app_e2e.sh, plat_e2e.sh, device_engine_tests.sh, emulator_smoke.sh) runs. This file is a
# library to source, plus a small CLI.
#
# Library (source it, then call at the top of the script, before any device access):
#     source "$(dirname "$0")/p7_emulator_lock.sh"
#     p7_emulator_lock uc_e2e "$0" "$@"
#     p7_emulator_lock_note uc_e2e
#
#   p7_emulator_lock NAME SCRIPT_PATH [ARGS...]
#       PILI_P7_LOCK_HELD=1: returns 0 at once (a nested P7 script runs under the lock of
#       its parent). Otherwise it waits while pgrep -f
#       'scripts/android/(app_e2e|plat_e2e|device_engine_tests|emulator_smoke)[.]sh' lists a
#       pid that is neither the caller nor one of its ancestors (at most PILI_P7_EMU_WAIT
#       seconds, default 5400, one stderr line a minute), then runs
#       'bash SCRIPT_PATH ARGS...' under /usr/bin/lockf on $LOCKDIR/p7-use.lock with
#       PILI_P7_LOCK_HELD=1, and EXITS with its status. The call therefore returns only in
#       the re-run (HELD=1); the first call never returns (it clears the EXIT trap first, so
#       call p7_emulator_lock before the script sets its own traps).
#       The re-run is a child, not an exec, on purpose: lockf -t exits 75 when the lock is
#       busy but the script may exit 75 too, so a marker file touched by the child right
#       before it execs the script tells "lock never taken" from "script exited 75" (the
#       idiom of heavy.sh; the marker holds the script pid). On a timeout (waiting for an old
#       script, or for the lock) it
#       prints 'P7LOCK | busy | held by <holder line>' and exits 75. SIGINT and SIGTERM are
#       forwarded to the script under the lock and to its descendants (lockf does not pass a
#       signal on to the program it runs, so the marker file holds the script pid for this);
#       the wrapper then waits for the script to end, so the lock is held as long as the
#       script runs, even when the wrapper was asked to stop.
#   p7_emulator_lock_note NAME
#       In the re-run, writes $LOCKDIR/p7-use.holder ('NAME pid=<pid> since=<ISO UTC>') and
#       adds p7_emulator_lock_cleanup to the EXIT trap (the trap already set is kept and
#       runs after it). A nested script (the holder file names another live pid) leaves the
#       file alone.
#   p7_emulator_lock_cleanup
#       Removes the holder file when it names this process. Call it from your own EXIT
#       trap when you replace the trap after p7_emulator_lock_note.
#
# CLI
#   bash scripts/android/p7_emulator_lock.sh --status   prints 'free' or the holder line
#                                                       (exit 0 either way)
#   bash scripts/android/p7_emulator_lock.sh --help     prints this header
#   other arguments: usage error, exit 2
#
# Environment
#   PILI_P7_LOCK_DIR   lock directory (default $HOME/.cache/pili-android/emulator)
#   PILI_P7_EMU_WAIT   seconds to wait in total, old scripts plus lock (default 5400)
#   PILI_P7_POLL       seconds between two polls (default 3)
#   PILI_P7_PGREP      command used instead of pgrep (tests); called as '<cmd> -f <ERE>'
#   PILI_P7_LOCK_HELD  1 inside a held lock (set by this file)
# ─────────────────────────────────────────────────────────────────

P7_LOCK_OLD_ERE='scripts/android/(app_e2e|plat_e2e|device_engine_tests|emulator_smoke)[.]sh'

p7__lockdir() { printf '%s' "${PILI_P7_LOCK_DIR:-$HOME/.cache/pili-android/emulator}"; }

# Pids of the caller and of its ancestors, one per line.
p7__self_and_ancestors() {
    local p="$$" n=0
    while [[ -n "$p" && "$p" -gt 1 && "$n" -lt 64 ]]; do
        printf '%s\n' "$p"
        p="$(ps -o ppid= -p "$p" 2>/dev/null | tr -d ' ')"
        n=$((n + 1))
    done
}

# Prints the first running old end-to-end script that is not the caller or its ancestor:
# '<pid> <command line>'. Prints nothing when there is none.
p7__old_runner() {
    local pg="${PILI_P7_PGREP:-pgrep}" pids pid own
    pids="$($pg -f "$P7_LOCK_OLD_ERE" 2>/dev/null || true)"
    [[ -n "$pids" ]] || return 0
    own="$(p7__self_and_ancestors)"
    for pid in $pids; do
        [[ "$pid" =~ ^[0-9]+$ ]] || continue
        if printf '%s\n' "$own" | grep -qx -- "$pid"; then
            continue
        fi
        kill -0 "$pid" 2>/dev/null || continue
        printf '%s %s\n' "$pid" "$(ps -o command= -p "$pid" 2>/dev/null | cut -c1-120)"
        return 0
    done
    return 0
}

# The holder line, or 'unknown' when no (live) holder file exists.
p7__holder_line() {
    local f
    f="$(p7__lockdir)/p7-use.holder"
    if [[ -s "$f" ]]; then
        printf '%s' "$(head -n 1 "$f")"
    else
        printf 'unknown'
    fi
}

# Sends TERM to PID and to all its descendants (children first listed from ps).
p7__kill_tree() {
    local root="$1" pid kids
    [[ "$root" =~ ^[0-9]+$ ]] || return 0
    kids="$(ps -A -o pid= -o ppid= 2>/dev/null | awk -v p="$root" '$2 == p { print $1 }')"
    for pid in $kids; do
        p7__kill_tree "$pid"
    done
    kill -TERM "$root" 2>/dev/null || true
}

# Removes the holder file when its pid= is PID.
p7__remove_holder_of() {
    local f pid
    f="$(p7__lockdir)/p7-use.holder"
    [[ -f "$f" ]] || return 0
    pid="$(sed -n 's/.* pid=\([0-9][0-9]*\).*/\1/p' "$f" | head -n 1)"
    if [[ -n "$pid" && "$pid" == "$1" ]]; then
        rm -f "$f"
    fi
    return 0
}

p7_emulator_lock_cleanup() {
    local f pid
    f="$(p7__lockdir)/p7-use.holder"
    [[ -f "$f" ]] || return 0
    pid="$(sed -n 's/.* pid=\([0-9][0-9]*\).*/\1/p' "$f" | head -n 1)"
    if [[ "$pid" == "$$" ]]; then
        rm -f "$f"
    fi
    return 0
}

p7_emulator_lock_note() {
    local name="${1:-p7}" f other prev
    [[ "${PILI_P7_LOCK_HELD:-0}" == "1" ]] || return 0
    f="$(p7__lockdir)/p7-use.holder"
    if [[ -s "$f" ]]; then
        other="$(sed -n 's/.* pid=\([0-9][0-9]*\).*/\1/p' "$f" | head -n 1)"
        if [[ -n "$other" && "$other" != "$$" ]] && kill -0 "$other" 2>/dev/null; then
            return 0
        fi
    fi
    mkdir -p "$(p7__lockdir)" || return 0
    printf '%s pid=%s since=%s\n' "$name" "$$" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >"$f"
    # Compose with the EXIT trap the caller already has.
    prev="$(trap -p EXIT)"
    if [[ -n "$prev" ]]; then
        eval "set -- $prev"
        trap "p7_emulator_lock_cleanup; $3" EXIT
    else
        trap 'p7_emulator_lock_cleanup' EXIT
    fi
    return 0
}

p7_emulator_lock() {
    if [[ $# -lt 2 ]]; then
        echo "p7_emulator_lock: usage: p7_emulator_lock NAME SCRIPT_PATH [ARGS...]" >&2
        exit 2
    fi
    local name="$1" script="$2"
    shift 2
    [[ "${PILI_P7_LOCK_HELD:-0}" == "1" ]] && return 0

    local lockf=/usr/bin/lockf dir wait_s start deadline now old last_msg=0 remaining
    [[ -x "$lockf" ]] || lockf="$(command -v lockf || true)"
    if [[ -z "$lockf" ]]; then
        echo "p7_emulator_lock: lockf not found" >&2
        exit 2
    fi
    dir="$(p7__lockdir)"
    mkdir -p "$dir" || exit 2
    wait_s="${PILI_P7_EMU_WAIT:-5400}"
    if ! [[ "$wait_s" =~ ^[0-9]+$ ]]; then
        echo "p7_emulator_lock: PILI_P7_EMU_WAIT must be a number of seconds (got '$wait_s')" >&2
        exit 2
    fi
    start="$(date +%s)"
    deadline=$((start + wait_s))

    # 1. wait for the older end-to-end scripts
    while true; do
        old="$(p7__old_runner)"
        [[ -n "$old" ]] || break
        now="$(date +%s)"
        if [[ "$now" -ge "$deadline" ]]; then
            printf 'P7LOCK | busy | held by %s\n' "$old"
            exit 75
        fi
        if [[ $((now - last_msg)) -ge 60 ]]; then
            echo "P7LOCK | wait | $name waits for $old ($((now - start))s of ${wait_s}s)" >&2
            last_msg="$now"
        fi
        sleep "${PILI_P7_POLL:-3}"
    done

    # 2. take the lock and run the script under it
    now="$(date +%s)"
    remaining=$((deadline - now))
    [[ "$remaining" -ge 1 ]] || remaining=1
    local marker child rc spid
    marker="$(mktemp "${TMPDIR:-/tmp}/pili-p7lock.XXXXXX")" || exit 2
    rm -f "$marker"
    # The sh child writes its pid (kept by exec: it is the script's pid) into the marker.
    PILI_P7_LOCK_HELD=1 "$lockf" -k -s -t "$remaining" "$dir/p7-use.lock" \
        /bin/sh -c 'printf "%s\n" "$$" >"$1" && printf "%s pid=%s since=%s\n" "$2" "$$" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >"$3" && shift 3 && exec "$@"' \
        p7-lock "$marker" "$name" "$dir/p7-use.holder" bash "$script" "$@" &
    child=$!
    # lockf does not forward signals: stop the script itself (and its descendants) too.
    # The lock is released when the script is gone, not before.
    # lockf itself is only killed while it still waits for the lock (no script pid yet).
    trap 'if [[ -s "$marker" ]]; then p7__kill_tree "$(head -n 1 "$marker")"; else kill -TERM "$child" 2>/dev/null; fi' INT TERM
    wait "$child"
    rc=$?
    # A signal interrupts the first wait; wait again for the real status.
    while kill -0 "$child" 2>/dev/null; do
        wait "$child"
        rc=$?
    done
    trap - INT TERM
    if [[ ! -s "$marker" ]]; then
        rm -f "$marker"
        printf 'P7LOCK | busy | held by %s\n' "$(p7__holder_line)"
        exit 75
    fi
    spid="$(head -n 1 "$marker")"
    rm -f "$marker"
    # The script is gone; remove the holder file only if it is still the script's own line
    # (a next holder may already have written its line).
    p7__remove_holder_of "$spid"
    # This first call is only a wrapper: the caller's own EXIT trap runs in the re-run.
    trap - EXIT
    exit "$rc"
}

# ── CLI ─────────────────────────────────────────────────────────
p7__cli() {
    local dir lockf=/usr/bin/lockf
    case "${1:-}" in
    --status)
        [[ $# -eq 1 ]] || { echo "usage: p7_emulator_lock.sh [--status|--help]" >&2; return 2; }
        [[ -x "$lockf" ]] || lockf="$(command -v lockf || true)"
        dir="$(p7__lockdir)"
        if [[ -n "$lockf" && -e "$dir/p7-use.lock" ]]; then
            "$lockf" -k -s -t 0 "$dir/p7-use.lock" true
            if [[ $? -eq 75 ]]; then
                printf '%s\n' "$(p7__holder_line)"
                return 0
            fi
        fi
        echo free
        return 0
        ;;
    -h | --help)
        awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
        return 0
        ;;
    *)
        echo "usage: p7_emulator_lock.sh [--status|--help]" >&2
        return 2
        ;;
    esac
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    set -uo pipefail
    export LC_ALL=C
    p7__cli "$@"
    exit $?
fi
