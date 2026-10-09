#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of scripts/android/p7_emulator_lock.sh
#
# Usage: bash scripts/android/test_p7_emulator_lock.sh
#
# Every case runs in a temporary lock directory (PILI_P7_LOCK_DIR) with a fake pgrep
# (PILI_P7_PGREP) and waits of a few seconds at most. No emulator, no network. Cases: free
# lock acquired and status passed through, arguments passed, holder file written and
# removed, second holder times out with 75 and the busy line, a script exiting 75 itself
# is not 'busy', waiting for an older end-to-end process then proceeding, timing out on
# it, the caller and its ancestors ignored, PILI_P7_LOCK_HELD short-circuit, nested
# scripts, the caller's EXIT trap composed, TERM on the wrapper stopping the script, holder
# file removed only by its owner, --status free and held, usage errors.
# Prints one line per case, then
#     TESTP7LOCK | total | PASS|FAIL | <n> tests
# and exits 1 when a case fails.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
LIB="$ROOT/scripts/android/p7_emulator_lock.sh"
TMP="$(mktemp -d)"
BGPIDS=()
cleanup() {
    local p
    for p in ${BGPIDS[@]+"${BGPIDS[@]}"}; do kill "$p" 2>/dev/null || true; done
    rm -rf "$TMP"
}
trap cleanup EXIT
export HOME="$TMP/home"
mkdir -p "$HOME"
unset PILI_P7_LOCK_HELD PILI_P7_EMU_WAIT PILI_P7_POLL PILI_P7_PGREP PILI_P7_LOCK_DIR
pass=0
fail=0

ok() { pass=$((pass + 1)); printf 'TESTP7LOCK | %s | PASS\n' "$1"; }
bad() { fail=$((fail + 1)); printf 'TESTP7LOCK | %s | FAIL | %s\n' "$1" "$2"; }
# check NAME CONDITION-RESULT(0 = true) DETAIL
check() { if [[ "$2" -eq 0 ]]; then ok "$1"; else bad "$1" "$3"; fi; }

# Fake pgrep: prints the pids of $FAKE_PIDS (space separated) that are alive, and logs its arguments.
FAKE="$TMP/fake_pgrep"
cat >"$FAKE" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"${FAKE_LOG:-/dev/null}"
for p in ${FAKE_PIDS:-}; do
    kill -0 "$p" 2>/dev/null && echo "$p"
done
exit 0
STUB
chmod +x "$FAKE"
export PILI_P7_PGREP="$FAKE"
export FAKE_LOG="$TMP/fake.log"

# new_case NAME: fresh lock dir in LOCKDIR
new_case() {
    LOCKDIR="$TMP/lock.$1"
    rm -rf "$LOCKDIR"
    mkdir -p "$LOCKDIR"
    export PILI_P7_LOCK_DIR="$LOCKDIR"
    export PILI_P7_POLL=1
    unset PILI_P7_EMU_WAIT FAKE_PIDS
    : >"$FAKE_LOG"
}

# mk_script FILE BODY: a script that locks (name 'case'), notes, then runs BODY
mk_script() {
    cat >"$1" <<EOS
#!/usr/bin/env bash
source "$LIB"
p7_emulator_lock case "\$0" "\$@"
p7_emulator_lock_note case
$2
EOS
}

# ── 1. free lock acquired, exit status and arguments passed through ──
new_case free
mk_script "$TMP/s1.sh" 'echo "args=$* held=$PILI_P7_LOCK_HELD"; exit 7'
out="$(bash "$TMP/s1.sh" a "b c" 2>&1)"
rc=$?
check lock-free-passthrough $([[ $rc -eq 7 && "$out" == *"args=a b c held=1"* ]] && echo 0 || echo 1) "rc=$rc out=$out"

# ── 2. the pgrep command gets the documented pattern ──
check pgrep-pattern $(grep -qF 'scripts/android/(app_e2e|plat_e2e|device_engine_tests|emulator_smoke)[.]sh' "$FAKE_LOG" && grep -q '^-f ' "$FAKE_LOG" && echo 0 || echo 1) "log: $(cat "$FAKE_LOG")"

# ── 3. holder file written while running, removed after ──
new_case holder
mk_script "$TMP/s3.sh" 'cat "$PILI_P7_LOCK_DIR/p7-use.holder" >"$PILI_P7_LOCK_DIR/seen"; echo $$ >"$PILI_P7_LOCK_DIR/pid"'
bash "$TMP/s3.sh" >/dev/null 2>&1
seen="$(cat "$LOCKDIR/seen" 2>/dev/null)"
pid="$(cat "$LOCKDIR/pid" 2>/dev/null)"
check holder-written $([[ "$seen" =~ ^case\ pid=$pid\ since=[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:]{8}Z$ ]] && echo 0 || echo 1) "holder was '$seen', pid '$pid'"
check holder-removed $([[ ! -e "$LOCKDIR/p7-use.holder" ]] && echo 0 || echo 1) "p7-use.holder is still there"

# ── 4. a second holder times out: exit 75 and the busy line with the holder ──
new_case busy
mk_script "$TMP/hold.sh" 'sleep 8'
bash "$TMP/hold.sh" >/dev/null 2>&1 &
BGPIDS+=("$!")
for _ in $(seq 1 20); do [[ -s "$LOCKDIR/p7-use.holder" ]] && break; sleep 0.25; done
mk_script "$TMP/s4.sh" 'echo should-not-run'
out="$(PILI_P7_EMU_WAIT=2 bash "$TMP/s4.sh" 2>&1)"
rc=$?
check busy-timeout-75 $([[ $rc -eq 75 && "$out" == "P7LOCK | busy | held by case pid="* && "$out" != *should-not-run* ]] && echo 0 || echo 1) "rc=$rc out=$out"

# ── 5. --status while held prints the holder line ──
st="$(bash "$LIB" --status 2>&1)"
check status-held $([[ "$st" =~ ^case\ pid=[0-9]+\ since= ]] && echo 0 || echo 1) "status: $st"
wait "${BGPIDS[0]}" 2>/dev/null

# ── 6. --status free ──
new_case statusfree
st="$(bash "$LIB" --status 2>&1)"
rc=$?
check status-free $([[ $rc -eq 0 && "$st" == "free" ]] && echo 0 || echo 1) "rc=$rc status: $st"
# and after a held run that has finished
mk_script "$TMP/s6.sh" 'true'
bash "$TMP/s6.sh" >/dev/null 2>&1
st="$(bash "$LIB" --status 2>&1)"
check status-free-after-run $([[ "$st" == "free" ]] && echo 0 || echo 1) "status: $st"

# ── 7. a script that exits 75 by itself is not reported as busy ──
new_case exit75
mk_script "$TMP/s7.sh" 'echo ran; exit 75'
out="$(bash "$TMP/s7.sh" 2>&1)"
rc=$?
check script-exit-75-not-busy $([[ $rc -eq 75 && "$out" == "ran" ]] && echo 0 || echo 1) "rc=$rc out=$out"

# ── 8. waiting on a fake older e2e process, then proceeding ──
new_case wait
sleep 3 &
fakepid=$!
BGPIDS+=("$fakepid")
export FAKE_PIDS="$fakepid"
mk_script "$TMP/s8.sh" 'echo ran-at $(date +%s)'
t0="$(date +%s)"
out="$(PILI_P7_EMU_WAIT=30 bash "$TMP/s8.sh" 2>&1)"
rc=$?
t1="$(date +%s)"
check wait-then-proceed $([[ $rc -eq 0 && "$out" == *"ran-at "* && $((t1 - t0)) -ge 2 && $((t1 - t0)) -le 15 ]] && echo 0 || echo 1) "rc=$rc elapsed=$((t1 - t0)) out=$out"
wait "$fakepid" 2>/dev/null

# ── 9. an older e2e process that never ends: timeout, 75, busy line names its pid ──
new_case oldbusy
sleep 8 &
fakepid=$!
BGPIDS+=("$fakepid")
export FAKE_PIDS="$fakepid"
out="$(PILI_P7_EMU_WAIT=2 bash "$TMP/s8.sh" 2>&1)"
rc=$?
check wait-timeout-75 $([[ $rc -eq 75 && "$out" == *"P7LOCK | busy | held by $fakepid "* && "$out" != *ran-at* ]] && echo 0 || echo 1) "rc=$rc out=$out"
kill "$fakepid" 2>/dev/null; wait "$fakepid" 2>/dev/null

# ── 10. the caller and its ancestors are not waited for ──
new_case ancestors
export FAKE_PIDS="$$"
out="$(PILI_P7_EMU_WAIT=3 bash "$TMP/s8.sh" 2>&1)"
rc=$?
check ancestors-ignored $([[ $rc -eq 0 && "$out" == *"ran-at "* ]] && echo 0 || echo 1) "rc=$rc out=$out"
unset FAKE_PIDS

# ── 11. PILI_P7_LOCK_HELD=1 returns at once, no lock file, no lockf ──
new_case held
mk_script "$TMP/s11.sh" 'echo body-ran'
out="$(PILI_P7_LOCK_HELD=1 PILI_P7_PGREP=/nonexistent bash "$TMP/s11.sh" 2>&1)"
rc=$?
check lock-held-shortcircuit $([[ $rc -eq 0 && "$out" == "body-ran" && ! -e "$LOCKDIR/p7-use.lock" ]] && echo 0 || echo 1) "rc=$rc out=$out lock=$(ls "$LOCKDIR")"

# ── 12. a nested P7 script runs under its parent's lock (no deadlock) ──
new_case nested
mk_script "$TMP/child.sh" 'echo child-ran'
mk_script "$TMP/parent.sh" "echo parent-ran; bash '$TMP/child.sh'; echo parent-end; cat \"\$PILI_P7_LOCK_DIR/p7-use.holder\""
out="$(PILI_P7_EMU_WAIT=3 bash "$TMP/parent.sh" 2>&1)"
rc=$?
check nested-no-deadlock $([[ $rc -eq 0 && "$out" == *"parent-ran"*"child-ran"*"parent-end"*"case pid="* ]] && echo 0 || echo 1) "rc=$rc out=$out"
check nested-holder-removed $([[ ! -e "$LOCKDIR/p7-use.holder" ]] && echo 0 || echo 1) "holder left behind"

# ── 13. the caller's own EXIT trap still runs, once, after the holder cleanup ──
new_case trap
cat >"$TMP/s13.sh" <<EOS
#!/usr/bin/env bash
source "$LIB"
p7_emulator_lock case "\$0" "\$@"
trap 'echo caller-trap' EXIT
p7_emulator_lock_note case
exit 4
EOS
out="$(bash "$TMP/s13.sh" 2>&1)"
rc=$?
n="$(printf '%s\n' "$out" | grep -c '^caller-trap$')"
check exit-trap-composed $([[ $rc -eq 4 && "$n" -eq 1 && ! -e "$LOCKDIR/p7-use.holder" ]] && echo 0 || echo 1) "rc=$rc traps=$n out=$out"

# ── 14. usage errors exit 2 ──
new_case usage
bash "$LIB" --bogus >/dev/null 2>&1
rc1=$?
bash "$LIB" >/dev/null 2>&1
rc2=$?
cat >"$TMP/s14.sh" <<EOS
#!/usr/bin/env bash
source "$LIB"
p7_emulator_lock only-a-name
EOS
bash "$TMP/s14.sh" >/dev/null 2>&1
rc3=$?
PILI_P7_EMU_WAIT=abc bash "$TMP/s1.sh" >/dev/null 2>&1
rc4=$?
check usage-errors-exit-2 $([[ $rc1 -eq 2 && $rc2 -eq 2 && $rc3 -eq 2 && $rc4 -eq 2 ]] && echo 0 || echo 1) "rc: $rc1 $rc2 $rc3 $rc4"

# ── 16. TERM on the wrapper stops the script under the lock (lockf passes no signal on) ──
new_case term
mk_script "$TMP/s16.sh" 'echo $$ >"$LOCKDIR/script.pid"; sleep 20 & echo $! >"$LOCKDIR/sleep.pid"; wait'
LOCKDIR="$LOCKDIR" PILI_P7_LOCK_DIR="$LOCKDIR" bash "$TMP/s16.sh" >/dev/null 2>&1 &
wpid=$!
BGPIDS+=("$wpid")
for _ in $(seq 1 50); do [[ -s "$LOCKDIR/sleep.pid" ]] && break; sleep 0.1; done
spid="$(cat "$LOCKDIR/script.pid" 2>/dev/null)"
slpid="$(cat "$LOCKDIR/sleep.pid" 2>/dev/null)"
kill -TERM "$wpid" 2>/dev/null
for _ in $(seq 1 50); do kill -0 "$wpid" 2>/dev/null || break; sleep 0.1; done
sleep 0.3
alive=0
kill -0 "$wpid" 2>/dev/null && alive=1
kill -0 "$spid" 2>/dev/null && alive=1
kill -0 "$slpid" 2>/dev/null && alive=1
check term-stops-script $([[ -n "$spid" && -n "$slpid" && $alive -eq 0 ]] && echo 0 || echo 1) "wrapper/script/sleep alive: spid=$spid slpid=$slpid"
check term-then-free $([[ "$(bash "$LIB" --status)" == free && ! -e "$LOCKDIR/p7-use.holder" ]] && echo 0 || echo 1) "status=$(bash "$LIB" --status) holder=$(cat "$LOCKDIR/p7-use.holder" 2>/dev/null)"
[[ -n "$spid" ]] && kill -KILL "$spid" 2>/dev/null
[[ -n "$slpid" ]] && kill -KILL "$slpid" 2>/dev/null

# ── 17. a script that takes time to stop keeps the lock until it is gone ──
new_case termslow
mk_script "$TMP/s17.sh" 'trap "sleep 2; exit 0" TERM; echo ready >"$LOCKDIR/ready"; while :; do sleep 0.2; done'
LOCKDIR="$LOCKDIR" PILI_P7_LOCK_DIR="$LOCKDIR" bash "$TMP/s17.sh" >/dev/null 2>&1 &
wpid=$!
BGPIDS+=("$wpid")
for _ in $(seq 1 50); do [[ -e "$LOCKDIR/ready" ]] && break; sleep 0.1; done
kill -TERM "$wpid" 2>/dev/null
sleep 0.7
st_mid="$(bash "$LIB" --status)"
wait "$wpid" 2>/dev/null
st_end="$(bash "$LIB" --status)"
check term-slow-lock-held $([[ "$st_mid" == "case pid="* && "$st_end" == free ]] && echo 0 || echo 1) "mid=$st_mid end=$st_end"

# ── 18. the wrapper never erases the holder line of another process ──
new_case holderowner
printf 'next pid=999999 since=2026-01-01T00:00:00Z\n' >"$LOCKDIR/p7-use.holder"
out="$(PILI_P7_LOCK_DIR="$LOCKDIR" bash -c "source '$LIB'; p7__remove_holder_of 12345; [[ -f '$LOCKDIR/p7-use.holder' ]] && echo kept; p7__remove_holder_of 999999; [[ -f '$LOCKDIR/p7-use.holder' ]] || echo removed" 2>&1)"
check holder-only-own-pid $([[ "$out" == $'kept\nremoved' ]] && echo 0 || echo 1) "out=$out"

# ── 15. --help prints the header ──
h="$(bash "$LIB" --help 2>&1)"
check help-header $([[ "$h" == *"P7 emulator lock"* && "$h" == *"p7_emulator_lock NAME SCRIPT_PATH"* && "$h" != *"set -uo"* ]] && echo 0 || echo 1) "help: $(printf '%s' "$h" | head -n 3)"

total=$((pass + fail))
if [[ "$fail" -eq 0 ]]; then
    printf 'TESTP7LOCK | total | PASS | %d tests\n' "$total"
    exit 0
fi
printf 'TESTP7LOCK | total | FAIL | %d of %d tests failed\n' "$fail" "$total"
exit 1
