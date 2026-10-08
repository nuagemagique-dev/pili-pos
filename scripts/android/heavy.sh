#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - heavy.sh: run a heavy build under a global slot
#
# Several agents share one 16 GB Mac. A Gradle or cargo build takes a slot, so at
# most PILI_HEAVY_SLOTS of them run at once (PD111).
#
# Usage: scripts/android/heavy.sh <command> [args...]
#   Waits until one of the slots is free, runs the command while holding it and exits
#   with the status of the command.
#
# Environment
#   PILI_HEAVY_SLOTS  number of slots (default 2)
#   PILI_HEAVY=0      run the command directly, no slot
#   PILI_HEAVY_HELD   set to 1 inside a held slot: a nested heavy.sh runs the command
#                     directly (no second slot, no deadlock)
#
# Slots are the lock files $HOME/.cache/pili-android/heavy/slot.<i>.lock, taken with
# /usr/bin/lockf (macOS has no flock). lockf -s -t 0 exits 75 when the lock is busy,
# but the command may exit 75 as well; so the child writes a marker file before it
# execs the command, and a run without marker means "slot busy". Slots are tried in
# order (-k keeps the lock file, so two runs can never lock two different inodes of the
# same slot), then the script sleeps 3 s and retries; it says 'heavy.sh: waiting for a slot'
# on stderr at most once a minute. The lock is released by the kernel when the
# command (and every process that inherited the lock) ends.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

if [[ $# -eq 0 ]]; then
    echo "usage: heavy.sh <command> [args...]" >&2
    exit 2
fi

if [[ "${PILI_HEAVY:-1}" == "0" || "${PILI_HEAVY_HELD:-0}" == "1" ]]; then
    exec "$@"
fi

slots="${PILI_HEAVY_SLOTS:-2}"
if ! [[ "$slots" =~ ^[0-9]+$ ]] || [[ "$slots" -lt 1 ]]; then
    echo "heavy.sh: PILI_HEAVY_SLOTS must be a positive integer (got '$slots')" >&2
    exit 2
fi
LOCKF=/usr/bin/lockf
[[ -x "$LOCKF" ]] || LOCKF="$(command -v lockf || true)"
if [[ -z "$LOCKF" ]]; then
    echo "heavy.sh: lockf not found" >&2
    exit 2
fi

dir="$HOME/.cache/pili-android/heavy"
mkdir -p "$dir" || exit 2
marker_dir="$(mktemp -d "${TMPDIR:-/tmp}/pili-heavy.XXXXXX")" || exit 2
trap 'rm -rf "$marker_dir"' EXIT

last_msg=0
while true; do
    for ((i = 0; i < slots; i++)); do
        marker="$marker_dir/started.$i"
        rm -f "$marker"
        PILI_HEAVY_HELD=1 "$LOCKF" -k -s -t 0 "$dir/slot.$i.lock" \
            /bin/sh -c 'touch "$1" && shift && exec "$@"' heavy-slot "$marker" "$@"
        rc=$?
        if [[ -e "$marker" ]]; then
            exit "$rc"
        fi
        # No marker: the slot was busy (lockf exit 75) or lockf itself failed.
        if [[ "$rc" -ne 75 ]]; then
            echo "heavy.sh: lockf failed on slot $i (exit $rc)" >&2
            exit 2
        fi
    done
    now="$(date +%s)"
    if [[ $((now - last_msg)) -ge 60 ]]; then
        echo "heavy.sh: waiting for a slot" >&2
        last_msg="$now"
    fi
    sleep 3
done
