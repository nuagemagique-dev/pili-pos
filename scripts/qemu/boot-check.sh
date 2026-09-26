#!/usr/bin/env bash
# moyun-pos - check the first boot of a running scripts/qemu/run-kiosk.sh
# guest (admission image), from the journal it copies to its serial port
# (journald.conf.d/90-moyun-serial.conf):
#
#   1. moyun-provision reaches its one-time banner (LUKS2 format, TPM2
#      and recovery key enrollment, mkfs, factory data, passwords);
#   2. Enter is pressed on the banner, as the installer would;
#   3. moyun-pos.service starts and is still running 30 s later;
#   4. the kiosk frame is saved as screenshots/<out-basename>.png;
#   5. no unit failed, no ordering cycle was broken and nothing hit the
#      read-only root, in the whole boot.
#
# A non-blank frame alone proves nothing: an unbootable system still
# fills the screen with its errors.
#
# Usage: scripts/qemu/boot-check.sh <name> [out-basename]
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
NAME="${1:?usage: $0 <name> [out-basename]}"
OUT="${2:-$NAME}"
LOG="$SCRIPT_DIR/.run/$NAME/serial.log"
BAD='Failed to start|Dependency failed|ordering cycle|Failed with result|Job .* deleted|contradicts existing jobs|Read-only file system|moyun-provision.*ERROR|cage\[[0-9]+\]: .*\[ERROR\]'
# wlroots on QEMU's virtio-vga (pixman renderer, no cursor plane) logs
# these while probing the output; the frame still renders. Any other
# Cage error still fails the check.
BENIGN='cage\[[0-9]+\]: .*\[ERROR\] \[types/output/(output\.c:[0-9]+\] Basic output test failed|cursor\.c:[0-9]+\] Failed to (pick cursor format|render cursor buffer))'

[[ -f "$LOG" ]] || { echo "ERROR: no serial log at $LOG" >&2; exit 1; }

fail() {
    echo "::error::$1"
    echo "── last 120 lines of the guest journal ──"
    tail -n 120 "$LOG"
    exit 1
}

failures() {
    grep -E "$BAD" "$LOG" | grep -vE "$BENIGN" || true
}

check_no_failures() {
    local found
    found="$(failures)"
    if [[ -n "$found" ]]; then
        echo "── failures in the guest journal ──"
        echo "$found"
        fail "the boot has failed units or a broken ordering cycle"
    fi
}

wait_for() {
    local pattern="$1" secs="$2" what="$3" waited=0
    until grep -qE "$pattern" "$LOG"; do
        check_no_failures
        (( waited >= secs )) && fail "timed out after ${secs}s waiting for $what"
        sleep 5
        waited=$(( waited + 5 ))
    done
    echo "✓ $what (${waited}s)"
}

# The banner follows the password step; give the terminal a moment to draw it.
wait_for 'moyun-provision.*Setting the root password' 300 "first-boot provisioning"
sleep 5
"$SCRIPT_DIR/sendkey.sh" "$NAME" ret
wait_for 'moyun-provision.*Provisioning complete' 60 "provisioning banner acknowledged"
wait_for 'Started moyun-pos\.service' 180 "moyun-pos.service started"

sleep 30
if grep -qE 'moyun-pos\.service: (Main process exited|Failed|Scheduled restart)' "$LOG"; then
    fail "moyun-pos.service did not stay up"
fi
echo "✓ moyun-pos.service still running after 30s"

# The login screen's own background (ui/src/styles/tokens.css
# --bg-primary), not just "something is drawn". WebKit renders in
# software under QEMU: allow it up to two minutes.
SHOT="$REPO_ROOT/docs/SUBMISSION_ARTIFACTS/screenshots/$OUT.png"
for attempt in $(seq 1 12); do
    "$SCRIPT_DIR/screendump.sh" "$NAME" "$OUT" >/dev/null
    if python3 "$SCRIPT_DIR/check-not-blank.py" "$SHOT" --expect-color 1a1a2e --min-fraction 0.3; then
        echo "✓ kiosk UI on screen (attempt $attempt)"
        break
    fi
    (( attempt == 12 )) && fail "the kiosk UI never appeared on screen (see $OUT.png)"
    sleep 10
done

check_no_failures
echo "✓ no failed unit, no ordering cycle"
