#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of scripts/android/compare_fdm_journals.sh
#
# Works on copies of docs/android/baseline/fdm_journal_hubrise_e2e_run{1,2}*
# in a temp dir (no Docker, the baseline files are never modified). Cases:
#   run1 vs run2                                             PASS
#   run1 with pili-pos/1.0.0 -> 1.1.0, --sw-version 1.1.0    PASS
#   the same copy without --sw-version                       FAIL
#   a copy with one signSale entry removed                   FAIL
#   a copy with one Status entry moved to another position   PASS
#   a missing file                                           exit 2
#
# Usage: scripts/android/test_compare_fdm_journals.sh
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
CMP="$ROOT/scripts/android/compare_fdm_journals.sh"
BASE="$ROOT/docs/android/baseline"

command -v jq >/dev/null || { echo "jq is required" >&2; exit 2; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
cp "$BASE"/fdm_journal_hubrise_e2e_run1.json "$BASE"/fdm_journal_hubrise_e2e_run1.normalised.json \
    "$BASE"/fdm_journal_hubrise_e2e_run2.json "$BASE"/fdm_journal_hubrise_e2e_run2.normalised.json "$TMP/"

fails=0
cases=0

# expect <label> <expected exit code> <compare args...>
expect() {
    local label="$1" want="$2" rc
    shift 2
    cases=$((cases + 1))
    "$CMP" --baseline-dir "$TMP" "$@" >"$TMP/out.txt" 2>"$TMP/err.txt"
    rc=$?
    if [[ "$rc" -eq "$want" ]]; then
        echo "ok   $label (exit $rc)"
    else
        fails=$((fails + 1))
        echo "FAIL $label: exit $rc, wanted $want"
        sed 's/^/     | /' "$TMP/out.txt"
    fi
}

# variant <name> <jq filter for the raw and the normalised journal>
variant() {
    local name="$1" filter="$2" f
    for f in "" ".normalised"; do
        jq "$filter" "$TMP/fdm_journal_hubrise_e2e_run1$f.json" >"$TMP/fdm_journal_$name$f.json" || {
            echo "x jq failed building the $name variant" >&2
            exit 2
        }
    done
}

expect "run1 vs run2" 0 --dir "$TMP" --name hubrise_e2e_run2

for f in "" ".normalised"; do
    sed 's|pili-pos/1\.0\.0|pili-pos/1.1.0|g' "$TMP/fdm_journal_hubrise_e2e_run1$f.json" >"$TMP/fdm_journal_v110$f.json"
done
expect "version 1.1.0 with --sw-version" 0 --dir "$TMP" --name v110 --sw-version pili-pos/1.1.0
expect "version 1.1.0 without --sw-version" 1 --dir "$TMP" --name v110

variant nosale '(map(.operation) | index("signSale")) as $i | del(.[$i])'
expect "one signSale entry removed" 1 --dir "$TMP" --name nosale

variant movedstatus '.[1:] + .[:1]'
jq -e '.[0].operation == "Status"' "$TMP/fdm_journal_hubrise_e2e_run1.json" >/dev/null || {
    echo "x the first run1 entry is no longer a Status entry: adjust this test" >&2
    exit 2
}
expect "one Status entry moved" 0 --dir "$TMP" --name movedstatus

expect "missing file exits 2" 2 --dir "$TMP" --name does_not_exist
"$CMP" >/dev/null 2>&1
rc=$?
cases=$((cases + 1))
if [[ "$rc" -eq 2 ]]; then echo "ok   no argument (exit 2)"; else fails=$((fails + 1)); echo "FAIL no argument: exit $rc, wanted 2"; fi

if [[ "$fails" -eq 0 ]]; then
    echo "compare_fdm_journals self-test: $cases case(s) passed"
    exit 0
fi
echo "compare_fdm_journals self-test: $fails of $cases case(s) failed"
exit 1
