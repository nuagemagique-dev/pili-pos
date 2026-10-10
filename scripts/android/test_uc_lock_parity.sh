#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of scripts/android/uc_lock_parity.sh
#
# Usage: bash scripts/android/test_uc_lock_parity.sh
#
# The script runs on stub `cargo tree` outputs (PILI_UC_PARITY_ROOT_TREE, PILI_UC_PARITY_TREE) and a
# temporary allow file (PILI_UC_PARITY_ALLOW): no cargo, no build, no network. Cases: identical
# trees (path, proc-macro and repeat markers ignored), a package in one tree only, a different
# version of one package, an allowed difference, an allow line without a reason, a stale allow line.
# Prints one line per case, then
#     TESTUCLOCK | total | PASS|FAIL | <n> tests
# and exits 1 when a case fails.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
SCRIPT="$ROOT/scripts/android/uc_lock_parity.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
pass=0
fail=0

ok() { pass=$((pass + 1)); printf 'TESTUCLOCK | %s | PASS\n' "$1"; }
bad() { fail=$((fail + 1)); printf 'TESTUCLOCK | %s | FAIL | %s\n' "$1" "$2"; }

# run NAME EXPECTED-EXIT EXPECTED-TEXT ROOT-TREE UC-TREE ALLOW-FILE
run() {
    local name="$1" want_rc="$2" want_text="$3" root_tree="$4" uc_tree="$5" allow="$6" out rc
    out="$(PILI_UC_PARITY_ROOT_TREE="$root_tree" PILI_UC_PARITY_TREE="$uc_tree" \
        PILI_UC_PARITY_ALLOW="$allow" bash "$SCRIPT" 2>&1)"
    rc=$?
    if [[ $rc -ne $want_rc ]]; then
        bad "$name" "exit $rc, wanted $want_rc: $out"
    elif [[ "$out" != *"$want_text"* ]]; then
        bad "$name" "output lacks '$want_text': $out"
    else
        ok "$name"
    fi
}

printf 'pos-engine v1.1.0 (/a/crates/pos-engine)\nserde v1.0.229\nserde_derive v1.0.229 (proc-macro)\nserde v1.0.229 (*)\n' >"$TMP/root.txt"
printf 'pos-engine v1.1.0 (/b/crates/pos-engine)\nserde v1.0.229\nserde_derive v1.0.229 (proc-macro)\n\n' >"$TMP/same.txt"
printf 'pos-engine v1.1.0 (/b/crates/pos-engine)\nserde v1.0.229\nserde_derive v1.0.229 (proc-macro)\nregex v1.13.1\n' >"$TMP/extra.txt"
printf 'pos-engine v1.1.0 (/b/crates/pos-engine)\nserde v1.0.230\nserde_derive v1.0.229 (proc-macro)\n' >"$TMP/bump.txt"
printf '# nothing allowed\n' >"$TMP/allow_none.txt"
printf 'regex 1.13.1 only the uccheck tests need it, removed with the tool\n' >"$TMP/allow_regex.txt"
printf 'regex 1.13.1\n' >"$TMP/allow_noreason.txt"
printf 'regex 1.13.1 stale\n' >"$TMP/allow_stale.txt"

run "identical trees" 0 "lock parity: 3 packages, identical" "$TMP/root.txt" "$TMP/same.txt" "$TMP/allow_none.txt"
run "a package in the uccheck tree only" 1 "regex" "$TMP/root.txt" "$TMP/extra.txt" "$TMP/allow_none.txt"
run "a different version" 1 "serde" "$TMP/root.txt" "$TMP/bump.txt" "$TMP/allow_none.txt"
run "an allowed difference" 0 "identical, 1 allowed" "$TMP/root.txt" "$TMP/extra.txt" "$TMP/allow_regex.txt"
run "an allow line without a reason" 1 "" "$TMP/root.txt" "$TMP/extra.txt" "$TMP/allow_noreason.txt"
run "a stale allow line" 1 "" "$TMP/root.txt" "$TMP/same.txt" "$TMP/allow_stale.txt"

total=$((pass + fail))
if [[ $fail -eq 0 ]]; then
    printf 'TESTUCLOCK | total | PASS | %d tests\n' "$total"
else
    printf 'TESTUCLOCK | total | FAIL | %d of %d tests failed\n' "$fail" "$total"
    exit 1
fi
