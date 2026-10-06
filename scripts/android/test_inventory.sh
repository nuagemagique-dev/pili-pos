#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - test inventory
#
# Lists every test of the workspace, built from the real test binaries,
# so the Android port can prove it never loses a test (plan: "Tests never
# drop"). Two passes:
#   ws    cargo test --workspace            (default features)
#   cert  cargo test -p pili-pos --no-default-features --features cert
#
# For each pass: `cargo test --no-run --message-format=json` maps every
# test executable to its package and target, each executable is run with
# `--list --format terse` (and `--list` for its `N tests` total), and the doctests are listed per library with
# `cargo test --doc -p <pkg> -- --list`. Only `: test` entries are kept
# (no benches).
#
# Line format:  <section>:<package>::<target-kind>:<target-name>::<test path>
#   section      ws | cert
#   target-kind  lib | bin | test | doctest ...   (cargo's target kind)
# Doctest names drop the `(line N)` suffix so moving code is not a lost
# test; two doctests of one item that only differ by line get a stable
# `#2`, `#3` suffix (in line order) so none is silently merged.
#
# The output is `LC_ALL=C sort -u`, byte-identical across runs. Per
# section, the number of lines must equal the sum of the `N tests`
# totals the binaries report, or the script fails (no silent collisions).
#
# Usage:
#   scripts/android/test_inventory.sh                  write the baseline
#   scripts/android/test_inventory.sh --out FILE       write somewhere else
#   scripts/android/test_inventory.sh --diff           compare with the baseline,
#                                                      exit 1 if a test disappeared
#   scripts/android/test_inventory.sh --diff --baseline FILE
#
# Needs the embedded UI bundle (ui/dist): cd ui && npm ci && npm run build
# Honours CARGO_TARGET_DIR. Never modifies ui/ sources.
# ─────────────────────────────────────────────────────────────────
set -euo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEFAULT_BASELINE="$ROOT/docs/android/baseline/test_inventory.txt"

mode="write"
out=""
baseline="$DEFAULT_BASELINE"

usage() { sed -n '2,/^set -euo/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'; }

while [[ $# -gt 0 ]]; do
    case "$1" in
    --diff) mode="diff" ;;
    --out)
        out="${2:?--out needs a file}"
        shift
        ;;
    --baseline)
        baseline="${2:?--baseline needs a file}"
        shift
        ;;
    -h | --help)
        usage
        exit 0
        ;;
    *)
        echo "unknown argument: $1" >&2
        usage >&2
        exit 2
        ;;
    esac
    shift
done

command -v jq >/dev/null || { echo "jq is required" >&2; exit 2; }
command -v cargo >/dev/null || { echo "cargo is required" >&2; exit 2; }

cd "$ROOT"

if [[ ! -f ui/dist/index.html ]]; then
    echo "x ui/dist is missing: the Tauri crate embeds the UI bundle at compile time." >&2
    echo "  Build it first:  cd ui && npm ci && npm run build" >&2
    exit 2
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# package id -> "name<TAB>manifest dir<TAB>has-lib" for every workspace member.
cargo metadata --format-version 1 --no-deps |
    jq -r '.packages[] | [.id, .name, (.manifest_path | sub("/Cargo.toml$"; "")),
        ((.targets | map(select(.kind | any(. == "lib" or . == "rlib" or . == "proc-macro"))) | length) > 0 | tostring),
        ((.targets | map(select(.kind | any(. == "lib" or . == "rlib" or . == "proc-macro")))[0].name) // "")] | @tsv' \
    >"$TMP/packages.tsv"

pkg_field() { # <package name> <1-based column of packages.tsv>
    awk -F'\t' -v p="$1" -v c="$2" '$2 == p { print $c; exit }' "$TMP/packages.tsv"
}

# inventory_pass <section> <cargo args...>
# Writes $TMP/<section>.txt (unsorted lines) and $TMP/<section>.reported
# (the sum of the `N tests` counts the binaries print).
inventory_pass() {
    local section="$1"
    shift
    local lines="$TMP/$section.txt" reported_total=0
    : >"$lines"

    echo "== $section: building test binaries (cargo test --no-run $*)" >&2
    if ! cargo test "$@" --no-run --message-format=json >"$TMP/$section.json" 2>"$TMP/$section.err"; then
        echo "x cargo test --no-run failed for pass '$section':" >&2
        tail -n 40 "$TMP/$section.err" >&2
        exit 1
    fi

    # package id, target kind, target name, executable (one per test binary).
    jq -r 'select(.reason == "compiler-artifact" and .profile.test == true and .executable != null)
        | [.package_id, .target.kind[0], .target.name, .executable] | @tsv' "$TMP/$section.json" |
        LC_ALL=C sort -u >"$TMP/$section.bins"

    local id kind name exe pkg dir listing n
    while IFS=$'\t' read -r id kind name exe; do
        pkg="$(awk -F'\t' -v i="$id" '$1 == i { print $2; exit }' "$TMP/packages.tsv")"
        dir="$(awk -F'\t' -v i="$id" '$1 == i { print $3; exit }' "$TMP/packages.tsv")"
        [[ -n "$pkg" ]] || { echo "x unknown package id: $id" >&2; exit 1; }
        listing="$TMP/listing.$$"
        (cd "$dir" && "$exe" --list --format terse) >"$listing" 2>"$TMP/listing.err" || {
            echo "x $exe --list failed:" >&2
            cat "$TMP/listing.err" >&2
            exit 1
        }
        # The terse listing has no summary line: the pretty one ends with
        # "N tests, M benchmarks", the figure the line count is checked against.
        n="$(cd "$dir" && "$exe" --list 2>/dev/null | sed -n 's/^\([0-9][0-9]*\) tests\{0,1\}, [0-9][0-9]* benchmarks\{0,1\}$/\1/p' | tail -n 1)"
        [[ -n "$n" ]] || { echo "x no 'N tests' total in the listing of $exe" >&2; exit 1; }
        reported_total=$((reported_total + n))
        sed -n 's/: test$//p' "$listing" | sed "s|^|$section:$pkg::$kind:$name::|" >>"$lines"
    done <"$TMP/$section.bins"

    # Doctests, per library of the passes' packages.
    local doc_pkgs
    if [[ "$section" == cert ]]; then
        doc_pkgs="pili-pos"
    else
        doc_pkgs="$(awk -F'\t' '$4 == "true" { print $2 }' "$TMP/packages.tsv" | LC_ALL=C sort)"
    fi
    local lib
    for pkg in $doc_pkgs; do
        [[ "$(pkg_field "$pkg" 4)" == true ]] || continue
        lib="$(pkg_field "$pkg" 5)"
        echo "== $section: doctests of $pkg" >&2
        if [[ "$section" == cert ]]; then
            cargo test --doc -p "$pkg" --no-default-features --features cert -- --list \
                >"$TMP/doc.out" 2>"$TMP/doc.err" || { echo "x doctest listing failed for $pkg:" >&2; tail -n 40 "$TMP/doc.err" >&2; exit 1; }
        else
            cargo test --doc -p "$pkg" -- --list \
                >"$TMP/doc.out" 2>"$TMP/doc.err" || { echo "x doctest listing failed for $pkg:" >&2; tail -n 40 "$TMP/doc.err" >&2; exit 1; }
        fi
        n="$(sed -n 's/^\([0-9][0-9]*\) tests\{0,1\}, [0-9][0-9]* benchmarks\{0,1\}$/\1/p' "$TMP/doc.out" | tail -n 1)"
        n="${n:-0}"
        reported_total=$((reported_total + n))
        # "src/lib.rs - item (line 12): test" -> "src/lib.rs - item", numbered
        # in line order when several doctests of one item collapse together.
        sed -n 's/: test$//p' "$TMP/doc.out" |
            sed -n 's/^\(.*\) (line \([0-9][0-9]*\))$/\1\t\2/p' |
            LC_ALL=C sort -t "$(printf '\t')" -k1,1 -k2,2n |
            awk -F'\t' '{ c[$1]++; print (c[$1] == 1 ? $1 : $1 "#" c[$1]) }' |
            sed "s|^|$section:$pkg::doctest:$lib::|" >>"$lines"
        # A doctest listing line without "(line N)" would be lost above: check.
        local raw kept
        raw="$(grep -c ': test$' "$TMP/doc.out" || true)"
        kept="$(grep -c ' (line [0-9][0-9]*): test$' "$TMP/doc.out" || true)"
        [[ "$raw" == "$kept" ]] || { echo "x $pkg: $((raw - kept)) doctest(s) without a (line N) suffix" >&2; exit 1; }
    done

    echo "$reported_total" >"$TMP/$section.reported"
}

inventory_pass ws --workspace
inventory_pass cert -p pili-pos --no-default-features --features cert

result="$TMP/inventory.txt"
cat "$TMP/ws.txt" "$TMP/cert.txt" | LC_ALL=C sort -u >"$result"

total=0
status=0
for section in ws cert; do
    got="$(grep -c "^$section:" "$result" || true)"
    want="$(cat "$TMP/$section.reported")"
    printf '%-5s %6d tests listed (binaries report %d)\n' "$section" "$got" "$want"
    if [[ "$got" != "$want" ]]; then
        echo "x $section: $got unique lines but the binaries report $want tests (name collision?)" >&2
        status=1
    fi
    total=$((total + got))
done
printf '%-5s %6d lines (ws + cert: a test of the cert binary is also in ws)\n' total "$total"
distinct="$(sed 's/^[a-z]*://' "$result" | LC_ALL=C sort -u | wc -l | tr -d ' ')"
printf '%-5s %6d distinct tests (section prefix dropped)\n' uniq "$distinct"
[[ $status -eq 0 ]] || exit 1

if [[ "$mode" == write ]]; then
    dest="${out:-$DEFAULT_BASELINE}"
    mkdir -p "$(dirname "$dest")"
    cp "$result" "$dest"
    echo "wrote $dest"
    exit 0
fi

# --diff: compare with the baseline.
[[ -f "$baseline" ]] || { echo "x baseline not found: $baseline" >&2; exit 2; }
[[ -z "$out" ]] || cp "$result" "$out"
LC_ALL=C sort -u "$baseline" >"$TMP/baseline.sorted"
LC_ALL=C comm -23 "$TMP/baseline.sorted" "$result" >"$TMP/removed.txt"
LC_ALL=C comm -13 "$TMP/baseline.sorted" "$result" >"$TMP/added.txt"
removed="$(wc -l <"$TMP/removed.txt" | tr -d ' ')"
added="$(wc -l <"$TMP/added.txt" | tr -d ' ')"
echo "baseline $(wc -l <"$TMP/baseline.sorted" | tr -d ' ') lines, current $(wc -l <"$result" | tr -d ' ') lines: $added added, $removed removed"
if [[ "$removed" -gt 0 ]]; then
    echo "x tests that disappeared:"
    sed 's/^/  - /' "$TMP/removed.txt"
    exit 1
fi
echo "ok no test disappeared"
