#!/usr/bin/env bash
# Self-test of scripts/android/test_inventory.sh --diff (moves, --current,
# --min-ws). Synthetic inventories in a temp dir, no cargo.
set -uo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INV="$ROOT/scripts/android/test_inventory.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
T=$'\t'

fails=0
check() { # <name> <want exit> <want output regex or ""> <args...>
    local name="$1" want="$2" re="$3" got out
    shift 3
    out="$(bash "$INV" --diff "$@" 2>&1)"
    got=$?
    if [[ "$got" -eq "$want" && ( -z "$re" || "$out" =~ $re ) ]]; then
        echo "PASS $name"
    else
        echo "FAIL $name (exit $got, want $want, output must match '$re')"
        echo "$out" | sed 's/^/     /'
        fails=$((fails + 1))
    fi
}

printf '%s\n' \
    'ws:pkg-a::lib:pkg_a::old::tests::one' \
    'ws:pkg-a::lib:pkg_a::old::tests::two' \
    'ws:pkg-a::test:it::flow' \
    'cert:pkg-a::lib:pkg_a::old::tests::one' >"$TMP/base.txt"
: >"$TMP/none.tsv"
printf '%s\n' '# comment' '' "pkg-a::lib:pkg_a::old::${T}pkg-b::lib:pkg_b::new::" >"$TMP/moves.tsv"

# 1. identical
cp "$TMP/base.txt" "$TMP/cur1.txt"
check identical 0 'moves: 0 rules, 0 baseline lines renamed' \
    --baseline "$TMP/base.txt" --current "$TMP/cur1.txt" --moves "$TMP/none.tsv" --min-ws 3

# 2. a removed line
grep -v 'old::tests::two' "$TMP/base.txt" >"$TMP/cur2.txt"
check removed 1 'disappeared' \
    --baseline "$TMP/base.txt" --current "$TMP/cur2.txt" --moves "$TMP/none.tsv" --min-ws 1

# 3. a moved line with a matching rule (3 lines renamed: two ws, one cert)
sed 's/pkg-a::lib:pkg_a::old::/pkg-b::lib:pkg_b::new::/' "$TMP/base.txt" >"$TMP/cur3.txt"
check moved 0 'moves: 1 rules, 3 baseline lines renamed' \
    --baseline "$TMP/base.txt" --current "$TMP/cur3.txt" --moves "$TMP/moves.tsv" --min-ws 3
# the same inventory without the rule loses tests
check moved-without-rule 1 'disappeared' \
    --baseline "$TMP/base.txt" --current "$TMP/cur3.txt" --moves "$TMP/none.tsv" --min-ws 3

# 4. a stale rule
printf '%s\n' "pkg-z::lib:nothing${T}pkg-y::lib:nothing" >"$TMP/stale.tsv"
check stale-rule 1 'stale rule' \
    --baseline "$TMP/base.txt" --current "$TMP/cur1.txt" --moves "$TMP/stale.tsv" --min-ws 3

# 5. a collision: two baseline lines map to one name
printf '%s\n' \
    "pkg-a::lib:pkg_a::old::tests::one${T}pkg-a::lib:pkg_a::new::same" \
    "pkg-a::lib:pkg_a::old::tests::two${T}pkg-a::lib:pkg_a::new::same" >"$TMP/collide.tsv"
check collision 1 'collision' \
    --baseline "$TMP/base.txt" --current "$TMP/cur1.txt" --moves "$TMP/collide.tsv" --min-ws 3

# 6. ws below --min-ws
check min-ws 1 'below the floor of 4' \
    --baseline "$TMP/base.txt" --current "$TMP/cur1.txt" --moves "$TMP/none.tsv" --min-ws 4

# 7. longest prefix wins: old::* goes to pkg-b, the other lib test to pkg-c
cp "$TMP/base.txt" "$TMP/base7.txt"
echo 'ws:pkg-a::lib:pkg_a::other::x' >>"$TMP/base7.txt"
sed 's/pkg-a::lib:pkg_a::old::/pkg-b::lib:pkg_b::new::/; s/pkg-a::lib:pkg_a::other::/pkg-c::lib:pkg_c::other::/' "$TMP/base7.txt" >"$TMP/cur7.txt"
printf '%s\n' \
    "pkg-a::lib:pkg_a::${T}pkg-c::lib:pkg_c::" \
    "pkg-a::lib:pkg_a::old::${T}pkg-b::lib:pkg_b::new::" >"$TMP/longest.tsv"
check longest-prefix 0 'moves: 2 rules, 4 baseline lines renamed' \
    --baseline "$TMP/base7.txt" --current "$TMP/cur7.txt" --moves "$TMP/longest.tsv" --min-ws 3

# 8. the baseline file is untouched
cp "$TMP/base.txt" "$TMP/base.copy"
bash "$INV" --diff --baseline "$TMP/base.txt" --current "$TMP/cur3.txt" --moves "$TMP/moves.tsv" --min-ws 3 >/dev/null 2>&1
if cmp -s "$TMP/base.txt" "$TMP/base.copy"; then echo "PASS baseline-untouched"; else echo "FAIL baseline-untouched"; fails=$((fails + 1)); fi

if [[ "$fails" -gt 0 ]]; then
    echo "FAIL: $fails case(s)"
    exit 1
fi
echo "PASS: all cases"
