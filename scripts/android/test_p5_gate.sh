#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of scripts/android/p5_gate.sh
#
# Usage: bash scripts/android/test_p5_gate.sh
#
# Runs p5_gate.sh against fixture trees in mktemp (PILI_P5_ROOT) with a stub p4 gate
# (PILI_P5_P4_GATE) that records its arguments and prints chosen lines. No git, no
# Gradle, no network. Cases: option parsing and pass-through, the stub's FAIL lines
# counted and its total line dropped, ui-parity (also with --parity-strict), every
# ui-contracts pattern and the ui/theme exemption, every ui-money pattern, what is not
# flagged (comparisons, unary minus, lambda arrows, comments, strings), the Money.kt
# marker, ui-strings (SKIP, missing locale, differing names, wrong prefix), the total
# line and the exit codes.
# Prints one line per case and a summary
#     SELFTEST | total | PASS|FAIL | <n> passed, <m> failed
# and exits 1 when a case fails.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
GATE="$ROOT/scripts/android/p5_gate.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
export PILI_P5_NESTED=1
pass=0
fail=0

ok() { pass=$((pass + 1)); printf 'TEST | %s | PASS\n' "$1"; }
bad() { fail=$((fail + 1)); printf 'TEST | %s | FAIL | %s\n' "$1" "$2"; }

# check NAME CONDITION-RESULT(0 = true) DETAIL
check() { if [[ "$2" -eq 0 ]]; then ok "$1"; else bad "$1" "$3"; fi; }

# contains TEXT PATTERN: 0 when TEXT contains the fixed string PATTERN.
contains() { [[ "$1" == *"$2"* ]]; }

PKG="android/app/src/main/kotlin/dev/nuagemagique/pilipos"
RES="android/app/src/main/res"

STUB="$TMP/stub_p4.sh"
cat >"$STUB" <<'STUB'
#!/usr/bin/env bash
# Stub p4 gate: records its arguments, prints two check lines and a total line.
printf '%s\n' "$*" >>"${STUB_LOG:-/dev/null}"
echo "GATE | stub-one | PASS | ok"
if [[ "${STUB_FAIL:-0}" == "1" ]]; then
    echo "GATE | stub-two | FAIL | broken"
    echo "stub failure detail" >&2
    [[ "${STUB_NO_TOTAL:-0}" == "1" ]] || echo "GATE | total | FAIL | 1 failed (2 checks)"
    exit 1
fi
echo "GATE | stub-two | SKIP | later"
[[ "${STUB_NO_TOTAL:-0}" == "1" ]] || echo "GATE | total | PASS | 0 failed (2 checks)"
STUB

# new_root: a fixture tree that passes ui-parity (one command, one declaration), with
# empty ui/ and feature/ directories. Prints its path.
new_root() {
    local r
    r="$(mktemp -d "$TMP/root.XXXXXX")"
    mkdir -p "$r/ui/src/api" "$r/android/pos-core/api" "$r/docs/android" "$r/$PKG/ui" "$r/$PKG/feature/login" "$r/$PKG/engine"
    printf 'export const a = () => invoke("alpha_cmd");\n' >"$r/ui/src/api/ipc.ts"
    printf 'export const b = 1;\n' >"$r/ui/src/api/ipc-modifiers.ts"
    printf 'public abstract interface class uniffi/pos_ffi/PosEngineInterface {\n\tpublic abstract fun alphaCmd (Lkotlin/coroutines/Continuation;)Ljava/lang/Object;\n}\n' \
        >"$r/android/pos-core/api/pos-core.api"
    printf '# Parity\n\n```ui-parity\nalpha_cmd pending:P5-T4\n```\n' >"$r/docs/android/PARITY.md"
    printf '%s' "$r"
}

# kt ROOT REL TEXT: writes a Kotlin file below the package directory.
kt() {
    mkdir -p "$(dirname "$1/$PKG/$2")"
    printf '%s\n' "$3" >"$1/$PKG/$2"
}

# run_gate ROOT [env VAR=...]... -- ARGS...: sets OUT, ERR, RC; always --fast unless FULL=1.
run_gate() {
    local root="$1" envs=() fast=(--fast)
    shift
    while [[ $# -gt 0 && "$1" != "--" ]]; do
        envs+=("$1")
        shift
    done
    shift
    [[ "${FULL:-0}" != "1" ]] || fast=()
    [[ "$root" == "$TMP"/* ]] || { echo "run_gate: bad root '$root'" >&2; exit 1; }
    OUT="$(env PILI_P5_ROOT="$root" PILI_P5_P4_GATE="$STUB" STUB_LOG="$TMP/stub.log" ${envs[@]+"${envs[@]}"} \
        bash "$GATE" ${fast[@]+"${fast[@]}"} "$@" 2>"$TMP/err.txt")"
    RC=$?
    ERR="$(cat "$TMP/err.txt")"
}

status_of() { printf '%s\n' "$OUT" | sed -n "s/^GATE | $1 | \\([A-Z]*\\) | .*/\\1/p" | head -n 1; }
detail_of() { printf '%s\n' "$OUT" | sed -n "s/^GATE | $1 | [A-Z]* | //p" | head -n 1; }

# expect_status NAME CHECK STATUS
expect_status() {
    local got
    got="$(status_of "$2")"
    check "$1" "$([[ "$got" == "$3" ]] && echo 0 || echo 1)" "check $2 is '$got', expected $3 (out: $OUT) (err: $ERR)"
}

# ── option parsing and pass-through ─────────────────────────────
R="$(new_root)"
run_gate "$R" -- --bogus
check "an unknown option exits 2" "$([[ $RC -eq 2 ]] && echo 0 || echo 1)" "rc=$RC"
run_gate "$R" -- --base
check "--base without a value exits 2" "$([[ $RC -eq 2 ]] && echo 0 || echo 1)" "rc=$RC"
run_gate "$R" -- --allow
check "--allow without a value exits 2" "$([[ $RC -eq 2 ]] && echo 0 || echo 1)" "rc=$RC"
run_gate "$R" -- --min-app-tests
check "--min-app-tests without a value exits 2" "$([[ $RC -eq 2 ]] && echo 0 || echo 1)" "rc=$RC"

: >"$TMP/stub.log"
run_gate "$R" -- --exit --app-e2e --linux-repro --base android/lane-ui --allow '^docs/$' --min-app-tests 7 --parity-strict
LOGGED="$(cat "$TMP/stub.log")"
check "every option reaches p4_gate.sh --app" "$([[ "$LOGGED" == "--app --fast --exit --app-e2e --linux-repro --base android/lane-ui --allow ^docs/\$ --min-app-tests 7" ]] && echo 0 || echo 1)" "got: $LOGGED"
check "--parity-strict is not passed to p4_gate.sh" "$(contains "$LOGGED" "parity" && echo 1 || echo 0)" "got: $LOGGED"

: >"$TMP/stub.log"
run_gate "$R" -- --base b1
LOGGED="$(cat "$TMP/stub.log")"
check "a plain run passes --app --fast --base" "$([[ "$LOGGED" == "--app --fast --base b1" ]] && echo 0 || echo 1)" "got: $LOGGED"

: >"$TMP/stub.log"
FULL=1 run_gate "$R" -- --base b1
LOGGED="$(cat "$TMP/stub.log")"
check "full mode does not pass --fast" "$([[ "$LOGGED" == "--app --base b1" ]] && echo 0 || echo 1)" "got: $LOGGED"
FULL=0

# ── the stub's lines ────────────────────────────────────────────
run_gate "$R" -- --base b1
check "the stub's check lines are printed" "$(contains "$OUT" "GATE | stub-one | PASS | ok" && contains "$OUT" "GATE | stub-two | SKIP | later" && echo 0 || echo 1)" "out: $OUT"
check "the stub's total line is dropped" "$(contains "$OUT" "0 failed (2 checks)" && echo 1 || echo 0)" "out: $OUT"
check "one total line only" "$([[ "$(printf '%s\n' "$OUT" | grep -c '^GATE | total | ')" -eq 1 ]] && echo 0 || echo 1)" "out: $OUT"
check "a passing run counts the stub's and the own checks" "$([[ "$(printf '%s\n' "$OUT" | tail -n 1)" == "GATE | total | PASS | 0 failed (6 checks)" ]] && echo 0 || echo 1)" "last: $(printf '%s\n' "$OUT" | tail -n 1)"
check "a passing run exits 0" "$([[ $RC -eq 0 ]] && echo 0 || echo 1)" "rc=$RC"

run_gate "$R" STUB_FAIL=1 -- --base b1
check "a FAIL line of the stub is counted" "$([[ "$(printf '%s\n' "$OUT" | tail -n 1)" == "GATE | total | FAIL | 1 failed (6 checks)" ]] && echo 0 || echo 1)" "last: $(printf '%s\n' "$OUT" | tail -n 1)"
check "the stub's FAIL line is printed" "$(contains "$OUT" "GATE | stub-two | FAIL | broken" && echo 0 || echo 1)" "out: $OUT"
check "the stub's failing output stays on stderr" "$(contains "$ERR" "stub failure detail" && echo 0 || echo 1)" "err: $ERR"
check "a failing stub exits 1" "$([[ $RC -eq 1 ]] && echo 0 || echo 1)" "rc=$RC"
check "every own check still runs after a failing stub" "$([[ -n "$(status_of ui-parity)" && -n "$(status_of ui-contracts)" && -n "$(status_of ui-money)" && -n "$(status_of ui-strings)" ]] && echo 0 || echo 1)" "out: $OUT"

run_gate "$R" STUB_NO_TOTAL=1 -- --base b1
expect_status "a p4 gate without a total line is a FAIL" p4-gate FAIL
check "a p4 gate without a total line fails the run" "$([[ $RC -eq 1 ]] && echo 0 || echo 1)" "rc=$RC"

OUT="$(env PILI_P5_ROOT="$R" PILI_P5_P4_GATE="$TMP/absent.sh" bash "$GATE" --fast 2>/dev/null)"
RC=$?
expect_status "an absent p4 gate is a FAIL" p4-gate FAIL

# ── full mode adds the self-tests ───────────────────────────────
FULL=1 run_gate "$R" -- --base b1
expect_status "full mode lists selftest-p5-gate (nested: SKIP)" selftest-p5-gate SKIP
expect_status "full mode lists selftest-ui-parity (nested: SKIP)" selftest-ui-parity SKIP
run_gate "$R" -- --base b1
check "--fast has no self-test line" "$(contains "$OUT" "selftest-" && echo 1 || echo 0)" "out: $OUT"
FULL=0

# ── ui-parity ───────────────────────────────────────────────────
run_gate "$R" -- --base b1
expect_status "ui-parity passes on the fixture" ui-parity PASS
check "ui-parity detail is the total line of the checker" "$(contains "$(detail_of ui-parity)" "UIPARITY / total / PASS / called 0, excluded 0, pending 1" && echo 0 || echo 1)" "detail: $(detail_of ui-parity)"
run_gate "$R" -- --base b1 --parity-strict
expect_status "--parity-strict fails on a pending command" ui-parity FAIL
check "--parity-strict makes the run fail" "$([[ $RC -eq 1 ]] && echo 0 || echo 1)" "rc=$RC"
R2="$(new_root)"
rm "$R2/docs/android/PARITY.md"
run_gate "$R2" -- --base b1
expect_status "ui-parity fails without PARITY.md" ui-parity FAIL
R2="$(new_root)"
printf '# Parity\n\n```ui-parity\n```\n' >"$R2/docs/android/PARITY.md"
run_gate "$R2" -- --base b1
expect_status "ui-parity fails for an undeclared command" ui-parity FAIL

# ── ui-contracts ────────────────────────────────────────────────
# contract_case NAME FILE-REL SNIPPET: the snippet in a fresh tree must FAIL ui-contracts at line 2.
contract_case() {
    local r
    r="$(new_root)"
    kt "$r" "$2" "$(printf 'package x\n%s' "$3")"
    run_gate "$r" -- --base b1
    expect_status "ui-contracts catches $1" ui-contracts FAIL
    check "ui-contracts names file and line for $1" "$(contains "$ERR" "$PKG/$2:2 " && echo 0 || echo 1)" "err: $ERR"
    check "ui-contracts failure fails the run for $1" "$([[ $RC -eq 1 ]] && echo 0 || echo 1)" "rc=$RC"
}
contract_case "hiltViewModel" ui/A.kt 'val vm = hiltViewModel<Foo>()'
contract_case "EngineHolder" feature/login/A.kt 'class A(val h: EngineHolder)'
contract_case "EngineFactory" ui/A.kt 'val f = EngineFactory()'
contract_case "EngineNatives" ui/A.kt 'import dev.nuagemagique.pilipos.engine.EngineNatives'
contract_case "NavHostController" ui/A.kt 'fun f(c: NavHostController) {}'
contract_case "getIdentifier(" ui/A.kt 'val id = res.getIdentifier("x", "string", pkg)'
contract_case "ZoneId.systemDefault" ui/A.kt 'val z = ZoneId.systemDefault()'
contract_case "TimeZone.getDefault" ui/A.kt 'val z = TimeZone.getDefault()'
contract_case "Locale.getDefault" ui/A.kt 'val l = Locale.getDefault()'
contract_case "a Color(0x literal" ui/A.kt 'val c = Color(0xFF112233)'
contract_case "a numeric .sp literal" ui/A.kt 'val s = 14.sp'
contract_case "a decimal .sp literal" feature/login/A.kt 'val s = 14.5.sp'

R="$(new_root)"
kt "$R" ui/theme/PiliTypography.kt 'val a = Color(0xFF112233); val s = 14.sp; val z = ZoneId.systemDefault()'
kt "$R" ui/theme/sub/Deep.kt 'val s = 12.sp'
kt "$R" engine/EngineHolder.kt 'class EngineHolder; val f = EngineFactory(); val v = hiltViewModel<X>()'
kt "$R" ui/B.kt $'package x\n// hiltViewModel in a comment\n/* Color(0xFF000000) and 14.sp */\nval s = size.sp\nval t = tokens.body.sp\nval n = 14'
run_gate "$R" -- --base b1
expect_status "ui-contracts exempts ui/theme, skips engine/ and ignores comments and identifiers.sp" ui-contracts PASS

# ── ui-money ────────────────────────────────────────────────────
# money_flag NAME SNIPPET: must FAIL ui-money at line 2.
money_flag() {
    local r
    r="$(new_root)"
    kt "$r" ui/M.kt "$(printf 'package x\n%s' "$2")"
    run_gate "$r" -- --base b1
    expect_status "ui-money catches $1" ui-money FAIL
    check "ui-money names file and line for $1" "$(contains "$ERR" "$PKG/ui/M.kt:2 " && echo 0 || echo 1)" "err: $ERR"
}
money_flag "a product on the left" 'val a = totalCents * 2'
money_flag "a product on the right" 'val a = 2 * totalCents'
money_flag "a division" 'val a = total / lineCents'
money_flag "a remainder" 'val a = x % cents'
money_flag "a sum with a member" 'val a = base + line.amountCents'
money_flag "a difference" 'val a = base - priceCents'
money_flag "a parenthesised factor" 'val a = (b + c) * unitCents'
money_flag "+=" 'sum += lineCents'
money_flag "-=" 'total -= cents'
money_flag "*= on the identifier" 'cents *= 2'
money_flag "/= on the identifier" 'cents /= 100'
money_flag "%= on the identifier" 'cents %= 100'
money_flag "+= on the identifier" 'cents += 1'
money_flag "-= on the identifier" 'cents -= 1'
money_flag "a trailing operator" 'val a = totalCents +'
money_flag "sumOf with a lambda" 'val a = lines.sumOf { it.lineCents }'
money_flag "sumOf with a type argument" 'val a = lines.sumOf<Long> { it.lineCents }'
money_flag "fold with a lambda" 'val a = lines.fold(0L) { acc, l -> acc + l.cents }'
money_flag "fold over several lines" $'val a = lines.fold(0L) {\n acc, l ->\n acc.coerceAtLeast(l.cents)\n}'
money_flag "sumOf with a reference" 'val a = lines.sumOf(Line::totalCents)'
money_flag ".times(" 'val a = x.totalCents.times(2)'
money_flag ".div(" 'val a = cents.div(100)'
money_flag ".rem(" 'val a = cents.rem(100)'
money_flag ".plus( on the receiver" 'val a = cents.plus(1)'
money_flag ".minus( on the receiver" 'val a = x.lineCents.minus(1)'
money_flag ".plus( with the identifier as argument" 'val a = total.plus(lineCents)'
money_flag "roundToInt" 'val a = x.roundToInt(); val centsLabel = 1'
money_flag "roundToLong" 'val a = value.roundToLong(); val b = centsText'
money_flag "Math.round" 'val a = Math.round(amountCents)'
money_flag "a feature file" 'val a = totalCents * 2'
money_flag "a member call then /" 'val a = cents.toDouble() / 100'
money_flag "a member call then / (float)" 'val a = cents.toFloat() / 100f'
money_flag "a chain with a call then +" 'val a = line.priceCents.toLong() + 1'
money_flag "a safe call then *" 'val a = line?.priceCents?.toLong() * 2'
money_flag "a parenthesised elvis then +" 'val a = (cents ?: 0) + 1'
money_flag "a parenthesised sum then *" 'val a = 2 * (base + totalCents)'
money_flag "a parenthesised group with a member call then /" 'val a = (cents ?: 0).toLong() / 100'
money_flag "a called identifier then *" 'val a = totalCents() * 2'
money_flag "two called identifiers with -" 'val a = subtotalCents(a) - discountCents(b)'
money_flag "a called identifier then + a number" 'val a = totalCents() + 1'
money_flag "a called identifier then + a non-literal" 'val a = formatCents(x) + suffix'

R="$(new_root)"
kt "$R" feature/login/F.kt $'package x\nval a = lines.sumOf { it.lineCents }'
run_gate "$R" -- --base b1
expect_status "ui-money scans feature/" ui-money FAIL

R="$(new_root)"
kt "$R" ui/theme/T.kt 'val a = totalCents * 2'
kt "$R" engine/E.kt 'val a = totalCents * 2'
run_gate "$R" -- --base b1
expect_status "ui-money skips ui/theme and engine" ui-money PASS

R="$(new_root)"
kt "$R" ui/N.kt "$(cat <<'KT'
package x
fun f(totalCents: Long, other: Long, cents: Long, list: LongArray) {
    if (totalCents > 0) {}
    if (totalCents >= other || other <= totalCents) {}
    if (totalCents == 0L || totalCents != other) {}
    val n = -totalCents
    val m = foo(-cents, +cents)
    val o = if (other > 0) -cents else cents
    fun g(): Long { return -totalCents }
    val p = list.map { cents -> format(cents) }
    val q = mapOf(1 to -cents)
    val r = formatCents(totalCents) + " EUR"
    val r2 = "EUR " + formatCents(totalCents)
    val r3 = formatCents(totalCents) + " (" + label + ")"
    val r4 = (totalCents > 0) && (other > 0)
    val r5 = vatSplit(totalCents).first
    val r6 = lines.map { it.label }.joinToString()
    val s = "total cents * 2 = ${'$'}{a * cents}"
    val t = "sumOf { it.cents }"
    // val u = totalCents * 2
    /* val v = totalCents / 100 */
    val w = vatSplit(totalCents)
    val x = other
    val y = totalCents
    val z = foo(*list)
    val k = listOf(*centsArray)
    when (other) { in 0..cents -> 1 else -> 2 }
    cents++
    --cents
    val label = labelFor(centsText)
    val lbl = items.map { it.name }.joinToString()
    val total = lines.sumOf { it.quantity }
    val folded = lines.fold(0) { acc, l -> acc + l.quantity }
}
KT
)"
run_gate "$R" -- --base b1
expect_status "ui-money ignores comparisons, unary minus, arrows, calls, comments and strings" ui-money PASS
check "ui-money PASS detail names no violation" "$(contains "$ERR" "money arithmetic" && echo 1 || echo 0)" "err: $ERR"

# the Money.kt marker (PD208)
R="$(new_root)"
kt "$R" ui/format/Money.kt $'package x\nval d = totalCents / 100 // money-format-only\nval r = totalCents % 100   // money-format-only  '
run_gate "$R" -- --base b1
expect_status "a money-format-only line of Money.kt is allowed" ui-money PASS

R="$(new_root)"
kt "$R" ui/format/Money.kt $'package x\nval d = totalCents / 100'
run_gate "$R" -- --base b1
expect_status "a line of Money.kt without the marker is flagged" ui-money FAIL

R="$(new_root)"
kt "$R" ui/format/Money.kt $'package x\nval d = totalCents / 100 // money-format-only too'
run_gate "$R" -- --base b1
expect_status "a marker that does not end the line of Money.kt is flagged" ui-money FAIL

R="$(new_root)"
kt "$R" ui/format/Other.kt $'package x\nval d = totalCents / 100 // money-format-only'
run_gate "$R" -- --base b1
expect_status "the marker in another file is flagged" ui-money FAIL
check "the marker in another file is named" "$(contains "$ERR" "marker outside ui/format/Money.kt" && echo 0 || echo 1)" "err: $ERR"

R="$(new_root)"
kt "$R" feature/login/L.kt $'package x\nval d = 1 // money-format-only'
run_gate "$R" -- --base b1
expect_status "the marker without arithmetic outside Money.kt is flagged" ui-money FAIL

R="$(new_root)"
kt "$R" ui/format/Money.kt $'package x\nval a = 1\n// money-format-only\nval d = totalCents / 100'
run_gate "$R" -- --base b1
expect_status "a marker on its own line does not cover the next line" ui-money FAIL

# the CashDisplay.kt marker (UI-PD306)
R="$(new_root)"
kt "$R" ui/format/CashDisplay.kt $'package x\nval d = tenderedCents - dueCents // money-display-only\nval e = totalCents / parts   // money-display-only  '
run_gate "$R" -- --base b1
expect_status "a money-display-only line of CashDisplay.kt is allowed" ui-money PASS

R="$(new_root)"
kt "$R" ui/format/CashDisplay.kt $'package x\nval d = tenderedCents - dueCents'
run_gate "$R" -- --base b1
expect_status "a line of CashDisplay.kt without the marker is flagged" ui-money FAIL

R="$(new_root)"
kt "$R" ui/format/CashDisplay.kt $'package x\nval d = tenderedCents - dueCents // money-display-only too'
run_gate "$R" -- --base b1
expect_status "a display marker that does not end the line of CashDisplay.kt is flagged" ui-money FAIL

R="$(new_root)"
kt "$R" ui/format/Other.kt $'package x\nval d = tenderedCents - dueCents // money-display-only'
run_gate "$R" -- --base b1
expect_status "the display marker in another file is flagged" ui-money FAIL
check "the display marker in another file is named" "$(contains "$ERR" "marker outside ui/format/CashDisplay.kt" && echo 0 || echo 1)" "err: $ERR"

R="$(new_root)"
kt "$R" feature/checkout/C.kt $'package x\nval d = 1 // money-display-only'
run_gate "$R" -- --base b1
expect_status "the display marker without arithmetic outside CashDisplay.kt is flagged" ui-money FAIL

R="$(new_root)"
kt "$R" ui/format/CashDisplay.kt $'package x\nval d = tenderedCents - dueCents // money-format-only'
run_gate "$R" -- --base b1
expect_status "the format marker in CashDisplay.kt is flagged" ui-money FAIL
check "the format marker in CashDisplay.kt is named" "$(contains "$ERR" "marker outside ui/format/Money.kt" && echo 0 || echo 1)" "err: $ERR"

R="$(new_root)"
kt "$R" ui/format/Money.kt $'package x\nval d = totalCents / 100 // money-display-only'
run_gate "$R" -- --base b1
expect_status "the display marker in Money.kt is flagged" ui-money FAIL

R="$(new_root)"
kt "$R" ui/format/CashDisplay.kt $'package x\nval a = 1\n// money-display-only\nval d = tenderedCents - dueCents'
run_gate "$R" -- --base b1
expect_status "a display marker on its own line does not cover the next line" ui-money FAIL

# ── ui-strings ──────────────────────────────────────────────────
# strings_file DIR NAME NAMES...: writes res/DIR/strings_ui_NAME.xml with those string names.
strings_file() {
    local root="$1" dir="$2" area="$3" f
    shift 3
    mkdir -p "$root/$RES/$dir"
    f="$root/$RES/$dir/strings_ui_$area.xml"
    {
        echo '<?xml version="1.0" encoding="utf-8"?>'
        echo '<resources>'
        for n in "$@"; do echo "    <string name=\"$n\">x</string>"; done
        echo '</resources>'
    } >"$f"
}

R="$(new_root)"
run_gate "$R" -- --base b1
expect_status "ui-strings is SKIP while no strings_ui file exists" ui-strings SKIP

R="$(new_root)"
for d in values values-en values-zh; do strings_file "$R" "$d" common ui_common_ok ui_common_cancel; done
run_gate "$R" -- --base b1
expect_status "ui-strings passes with complete twins" ui-strings PASS

R="$(new_root)"
strings_file "$R" values common ui_common_ok
strings_file "$R" values-en common ui_common_ok
run_gate "$R" -- --base b1
expect_status "ui-strings fails with a missing values-zh twin" ui-strings FAIL
check "ui-strings names the missing twin" "$(contains "$ERR" "values-zh/strings_ui_common.xml is missing" && echo 0 || echo 1)" "err: $ERR"

R="$(new_root)"
strings_file "$R" values common ui_common_ok
strings_file "$R" values-zh common ui_common_ok
run_gate "$R" -- --base b1
expect_status "ui-strings fails with a missing values-en twin" ui-strings FAIL

R="$(new_root)"
strings_file "$R" values common ui_common_ok ui_common_cancel
strings_file "$R" values-en common ui_common_ok
strings_file "$R" values-zh common ui_common_ok ui_common_cancel
run_gate "$R" -- --base b1
expect_status "ui-strings fails when a twin lacks a name" ui-strings FAIL
check "ui-strings names the lacking name" "$(contains "$ERR" "values-en/strings_ui_common.xml lacks 1 name(s): ui_common_cancel" && echo 0 || echo 1)" "err: $ERR"

R="$(new_root)"
strings_file "$R" values common ui_common_ok
strings_file "$R" values-en common ui_common_ok
strings_file "$R" values-zh common ui_common_ok ui_common_extra
run_gate "$R" -- --base b1
expect_status "ui-strings fails when a twin has an extra name" ui-strings FAIL

R="$(new_root)"
strings_file "$R" values common ui_common_ok other_name
strings_file "$R" values-en common ui_common_ok other_name
strings_file "$R" values-zh common ui_common_ok other_name
run_gate "$R" -- --base b1
expect_status "ui-strings fails on a name without the ui_<area>_ prefix" ui-strings FAIL
check "ui-strings names the prefix" "$(contains "$ERR" "without the prefix ui_common_" && echo 0 || echo 1)" "err: $ERR"

R="$(new_root)"
for d in values values-en values-zh; do strings_file "$R" "$d" login ui_common_ok; done
run_gate "$R" -- --base b1
expect_status "ui-strings requires the prefix of the file's own area" ui-strings FAIL

R="$(new_root)"
for d in values values-en values-zh; do strings_file "$R" "$d" login ui_login_title; strings_file "$R" "$d" tables ui_tables_title; done
run_gate "$R" -- --base b1
expect_status "ui-strings passes with several areas" ui-strings PASS

R="$(new_root)"
for d in values values-en values-zh; do strings_file "$R" "$d" order_cart ui_order_cart_title; done
run_gate "$R" -- --base b1
expect_status "ui-strings checks an area with an underscore" ui-strings PASS

R="$(new_root)"
strings_file "$R" values order_cart ui_order_cart_title
run_gate "$R" -- --base b1
expect_status "ui-strings fails an area with an underscore that lacks its twins" ui-strings FAIL
check "ui-strings names the missing twin of an underscore area" "$(contains "$ERR" "values-en/strings_ui_order_cart.xml is missing" && echo 0 || echo 1)" "err: $ERR"

R="$(new_root)"
for d in values values-en values-zh; do strings_file "$R" "$d" order_cart ui_order_title; done
run_gate "$R" -- --base b1
expect_status "ui-strings requires the full underscore area as prefix" ui-strings FAIL

R="$(new_root)"
for d in values values-en values-zh; do strings_file "$R" "$d" common ui_common_ok; done
printf '<resources><string name="ui_common_ok">' >"$R/$RES/values-en/strings_ui_common.xml"
run_gate "$R" -- --base b1
expect_status "ui-strings fails on a file that does not parse" ui-strings FAIL

# ── total line and exit codes ───────────────────────────────────
R="$(new_root)"
run_gate "$R" -- --base b1
check "a clean run ends with GATE | total | PASS" "$([[ "$(printf '%s\n' "$OUT" | tail -n 1)" == "GATE | total | PASS | "* ]] && echo 0 || echo 1)" "last: $(printf '%s\n' "$OUT" | tail -n 1)"
check "a clean run prints the four own checks in order" "$([[ "$(printf '%s\n' "$OUT" | grep -o '^GATE | ui-[a-z]* ' | tr -d '\n')" == "GATE | ui-parity GATE | ui-contracts GATE | ui-money GATE | ui-strings " ]] && echo 0 || echo 1)" "out: $OUT"
check "a clean run exits 0" "$([[ $RC -eq 0 ]] && echo 0 || echo 1)" "rc=$RC"

R="$(new_root)"
kt "$R" ui/A.kt $'package x\nval s = 14.sp'
kt "$R" ui/M.kt $'package x\nval a = totalCents * 2'
run_gate "$R" -- --base b1
check "two failing checks are both counted" "$([[ "$(printf '%s\n' "$OUT" | tail -n 1)" == "GATE | total | FAIL | 2 failed (6 checks)" ]] && echo 0 || echo 1)" "last: $(printf '%s\n' "$OUT" | tail -n 1)"
check "two failing checks exit 1" "$([[ $RC -eq 1 ]] && echo 0 || echo 1)" "rc=$RC"

run_gate "$R" -- --help
check "--help exits 0 and documents the options" "$([[ $RC -eq 0 ]] && contains "$OUT" "--parity-strict" && echo 0 || echo 1)" "rc=$RC"

printf 'SELFTEST | total | %s | %d passed, %d failed\n' "$([[ $fail -eq 0 ]] && echo PASS || echo FAIL)" "$pass" "$fail"
[[ $fail -eq 0 ]]
