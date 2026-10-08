#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of scripts/android/heavy.sh
#
# Usage: bash scripts/android/test_heavy.sh
#
# Runs heavy.sh with a temporary HOME (so the real slots are never touched), no
# Gradle, no network. Cases: one slot serialises two 'sleep 2' jobs (wall time at
# least 4 s), two slots run them together (under 3.5 s), the exit status of the
# command passes through (also 75, the lockf busy status), a nested call inside a held
# slot does not wait, PILI_HEAVY=0 runs directly, no command is a usage error.
# Prints one line per case and a summary; exits 1 when a case fails.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
HEAVY="$ROOT/scripts/android/heavy.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
export HOME="$TMP/home"
mkdir -p "$HOME"
export TMPDIR="$TMP"
unset PILI_HEAVY PILI_HEAVY_HELD PILI_HEAVY_SLOTS
pass=0
fail=0

ok() { pass=$((pass + 1)); printf 'TEST | %s | PASS\n' "$1"; }
bad() { fail=$((fail + 1)); printf 'TEST | %s | FAIL | %s\n' "$1" "$2"; }

now_ms() { python3 -I -c 'import time; print(int(time.time() * 1000))'; }

# two_jobs SLOTS: wall time in ms of two concurrent 'heavy.sh sleep 2'.
two_jobs() {
    local t0 t1
    t0="$(now_ms)"
    PILI_HEAVY_SLOTS="$1" "$HEAVY" sleep 2 2>/dev/null &
    local a=$!
    PILI_HEAVY_SLOTS="$1" "$HEAVY" sleep 2 2>/dev/null &
    local b=$!
    wait "$a" "$b"
    t1="$(now_ms)"
    echo $((t1 - t0))
}

ms="$(two_jobs 1)"
if [[ "$ms" -ge 4000 ]]; then ok "one slot serialises two jobs (${ms} ms >= 4000)"; else bad "one slot serialises two jobs" "${ms} ms"; fi

ms="$(two_jobs 2)"
if [[ "$ms" -lt 3500 ]]; then ok "two slots run two jobs together (${ms} ms < 3500)"; else bad "two slots run two jobs together" "${ms} ms"; fi

"$HEAVY" sh -c 'exit 7' 2>/dev/null
rc=$?
if [[ $rc -eq 7 ]]; then ok "exit status 7 passes through"; else bad "exit status 7 passes through" "rc=$rc"; fi

"$HEAVY" sh -c 'exit 75' 2>/dev/null
rc=$?
if [[ $rc -eq 75 ]]; then ok "exit status 75 (the lockf busy status) passes through"; else bad "exit status 75 passes through" "rc=$rc"; fi

"$HEAVY" true 2>/dev/null
rc=$?
if [[ $rc -eq 0 ]]; then ok "exit status 0 passes through"; else bad "exit status 0 passes through" "rc=$rc"; fi

# Nested call inside a held slot: SLOTS=1, so a second slot would deadlock.
t0="$(now_ms)"
out="$(PILI_HEAVY_SLOTS=1 "$HEAVY" "$HEAVY" sh -c 'echo held=$PILI_HEAVY_HELD' 2>&1)"
rc=$?
t1="$(now_ms)"
if [[ $rc -eq 0 && "$out" == "held=1" && $((t1 - t0)) -lt 2500 ]]; then
    ok "a nested call inside a held slot does not wait"
else
    bad "a nested call inside a held slot does not wait" "rc=$rc out='$out' $((t1 - t0)) ms"
fi

# PILI_HEAVY=0 runs directly, even while the only slot is held.
PILI_HEAVY_SLOTS=1 "$HEAVY" sleep 3 2>/dev/null &
holder=$!
sleep 1
t0="$(now_ms)"
out="$(PILI_HEAVY=0 PILI_HEAVY_SLOTS=1 "$HEAVY" sh -c 'echo "held=${PILI_HEAVY_HELD:-unset}"' 2>&1)"
t1="$(now_ms)"
wait "$holder"
if [[ "$out" == "held=unset" && $((t1 - t0)) -lt 1500 ]]; then
    ok "PILI_HEAVY=0 runs directly while the slot is taken"
else
    bad "PILI_HEAVY=0 runs directly" "out='$out' $((t1 - t0)) ms"
fi

# A busy slot prints the waiting notice once.
PILI_HEAVY_SLOTS=1 "$HEAVY" sleep 4 2>/dev/null &
holder=$!
sleep 1
err="$(PILI_HEAVY_SLOTS=1 "$HEAVY" true 2>&1 >/dev/null)"
wait "$holder"
if [[ "$(printf '%s\n' "$err" | grep -c 'heavy.sh: waiting for a slot')" -eq 1 ]]; then
    ok "a busy slot prints the waiting notice once"
else
    bad "a busy slot prints the waiting notice once" "stderr='$err'"
fi

"$HEAVY" >/dev/null 2>&1
rc=$?
if [[ $rc -eq 2 ]]; then ok "no command is a usage error (exit 2)"; else bad "no command is a usage error" "rc=$rc"; fi

PILI_HEAVY_SLOTS=abc "$HEAVY" true >/dev/null 2>&1
rc=$?
if [[ $rc -eq 2 ]]; then ok "a bad PILI_HEAVY_SLOTS is refused (exit 2)"; else bad "a bad PILI_HEAVY_SLOTS is refused" "rc=$rc"; fi

if [[ -f "$HOME/.cache/pili-android/heavy/slot.0.lock" ]]; then
    ok "the lock files live under HOME/.cache/pili-android/heavy"
else
    bad "the lock files live under HOME/.cache/pili-android/heavy" "slot.0.lock missing"
fi

printf 'TEST | summary | %s | %d passed, %d failed\n' "$([[ $fail -eq 0 ]] && echo PASS || echo FAIL)" "$pass" "$fail"
[[ $fail -eq 0 ]]
