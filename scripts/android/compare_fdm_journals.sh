#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - compare an FDM request journal with the baseline (PD40)
#
# Compares what pos-fdm-mock logged during a run of the HubRise e2e flow
# (scripts/android/capture_fdm_journals.sh) with the baseline run. No
# Docker: it only reads four JSON files. Prints one line per check
#     JOURNAL | <check> | PASS|FAIL|INFO | <detail>
# then
#     JOURNAL | total | PASS|FAIL
# and exits 1 on any FAIL, 2 on a usage error, a missing file or a missing jq.
#
# Usage: scripts/android/compare_fdm_journals.sh --dir DIR --name NAME
#            [--baseline-dir docs/android/baseline]
#            [--baseline-name hubrise_e2e_run1]
#            [--sw-version pili-pos/1.0.0]
#
# Inputs (the same pair for the baseline, in --baseline-dir):
#   DIR/fdm_journal_NAME.json             raw journal: array of
#                                         {at, operation, request, response}
#   DIR/fdm_journal_NAME.normalised.json  volatile fields removed (jq -S)
#
# Checks
#   business    sha256 of `jq -S '[.[] | select(.operation != "Status")]'` of the
#               normalised journal. In the baseline view every string equal to
#               pili-pos/1.0.0 is first replaced by --sw-version (the identity
#               of the build under test); the current view is not mapped.
#               PASS when the two hashes are equal. (The unmapped baseline run1
#               business hash is
#               f5c085c3050bde77114a10f2778efa1d2c206f9766e0e11d6c99c64e89d58bae.)
#   counters    from the raw journals, (operation, eventLabel, eventCounter,
#               totalCounter) of every non-Status call, read from the call's own
#               fdmRef (response.data.<operation>.fdmRef). PASS when equal.
#   sw-version  every posSwVersion found in the requests of the current raw
#               journal equals --sw-version (at least one must exist).
#   status-only hashes of the Status entries of both normalised journals. INFO,
#               never fails (the engine's FDM monitor may move or add polls).
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

usage() { sed -n '2,/^set -uo/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'; }

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
dir=""
name=""
baseline_dir="$ROOT/docs/android/baseline"
baseline_name="hubrise_e2e_run1"
sw_version="pili-pos/1.0.0"
BASELINE_SW="pili-pos/1.0.0"

while [[ $# -gt 0 ]]; do
    case "$1" in
    --dir)
        dir="${2:?--dir needs a directory}"
        shift
        ;;
    --name)
        name="${2:?--name needs a value}"
        shift
        ;;
    --baseline-dir)
        baseline_dir="${2:?--baseline-dir needs a directory}"
        shift
        ;;
    --baseline-name)
        baseline_name="${2:?--baseline-name needs a value}"
        shift
        ;;
    --sw-version)
        sw_version="${2:?--sw-version needs a value}"
        shift
        ;;
    -h | --help)
        usage
        exit 0
        ;;
    *)
        echo "unknown argument: $1" >&2
        exit 2
        ;;
    esac
    shift
done

[[ -n "$dir" && -n "$name" ]] || { echo "--dir and --name are required" >&2; usage >&2; exit 2; }
command -v jq >/dev/null || { echo "jq is required" >&2; exit 2; }

if command -v sha256sum >/dev/null; then
    sha() { sha256sum | cut -d' ' -f1; }
else
    sha() { shasum -a 256 | cut -d' ' -f1; }
fi

cur_raw="$dir/fdm_journal_$name.json"
cur_norm="$dir/fdm_journal_$name.normalised.json"
base_raw="$baseline_dir/fdm_journal_$baseline_name.json"
base_norm="$baseline_dir/fdm_journal_$baseline_name.normalised.json"
for f in "$cur_raw" "$cur_norm" "$base_raw" "$base_norm"; do
    [[ -f "$f" ]] || { echo "missing file: $f" >&2; exit 2; }
    jq -e 'type == "array"' "$f" >/dev/null 2>&1 || { echo "not a JSON array: $f" >&2; exit 2; }
done

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

failed=0
report() { # <check> <PASS|FAIL|INFO> <detail>
    [[ "$2" != FAIL ]] || failed=$((failed + 1))
    printf 'JOURNAL | %s | %s | %s\n' "$1" "$2" "$3"
}

# ── (a) business view ───────────────────────────────────────────
jq -S --arg from "$BASELINE_SW" --arg to "$sw_version" \
    '[.[] | select(.operation != "Status")] | walk(if type == "string" and . == $from then $to else . end)' \
    "$base_norm" >"$TMP/base_business.json" || { echo "jq failed on $base_norm" >&2; exit 2; }
jq -S '[.[] | select(.operation != "Status")]' "$cur_norm" >"$TMP/cur_business.json" ||
    { echo "jq failed on $cur_norm" >&2; exit 2; }
base_bh="$(sha <"$TMP/base_business.json")"
cur_bh="$(sha <"$TMP/cur_business.json")"
if [[ "$base_bh" == "$cur_bh" ]]; then
    report business PASS "baseline (mapped to $sw_version) $base_bh, current $cur_bh"
else
    report business FAIL "baseline (mapped to $sw_version) $base_bh, current $cur_bh"
    diff "$TMP/base_business.json" "$TMP/cur_business.json" | head -n 20 | sed 's/^/    | /' >&2
fi

# ── (b) counters ────────────────────────────────────────────────
# operation comes from the entry, not from the response key
COUNTERS='.[] | select(.operation != "Status")
  | (.response.data | to_entries[0].value.fdmRef) as $r
  | "\(.operation) \($r.eventLabel) \($r.eventCounter) \($r.totalCounter)"'
if jq -r "$COUNTERS" "$base_raw" >"$TMP/base_counters.txt" 2>"$TMP/jq.err" &&
    jq -r "$COUNTERS" "$cur_raw" >"$TMP/cur_counters.txt" 2>>"$TMP/jq.err"; then
    flat() { tr '\n' ',' <"$1" | sed 's/,$//'; }
    if cmp -s "$TMP/base_counters.txt" "$TMP/cur_counters.txt"; then
        report counters PASS "$(wc -l <"$TMP/cur_counters.txt" | tr -d ' ') call(s): $(flat "$TMP/cur_counters.txt")"
    else
        report counters FAIL "baseline: $(flat "$TMP/base_counters.txt") current: $(flat "$TMP/cur_counters.txt")"
    fi
else
    report counters FAIL "cannot read the fdmRef of a call: $(head -n 1 "$TMP/jq.err" | tr '|' '/')"
fi

# ── (c) posSwVersion ────────────────────────────────────────────
if jq -r '.[] | .request | .. | objects | select(has("posSwVersion")) | .posSwVersion | tostring' "$cur_raw" >"$TMP/sw.txt" 2>/dev/null; then
    n_sw="$(wc -l <"$TMP/sw.txt" | tr -d ' ')"
    n_bad="$(grep -vxF -- "$sw_version" "$TMP/sw.txt" | wc -l | tr -d ' ')"
    if [[ "$n_sw" -eq 0 ]]; then
        report sw-version FAIL "no posSwVersion in the requests"
    elif [[ "$n_bad" -eq 0 ]]; then
        report sw-version PASS "$n_sw posSwVersion value(s), all $sw_version"
    else
        report sw-version FAIL "$n_bad of $n_sw posSwVersion value(s) differ from $sw_version: $(grep -vxF -- "$sw_version" "$TMP/sw.txt" | sort -u | head -n 3 | tr '\n|' ' /')"
    fi
else
    report sw-version FAIL "jq failed on the raw journal requests"
fi

# ── (d) Status entries (informational) ──────────────────────────
base_sh="$(jq -S '[.[] | select(.operation == "Status")]' "$base_norm" | sha)"
cur_sh="$(jq -S '[.[] | select(.operation == "Status")]' "$cur_norm" | sha)"
n_base_status="$(jq '[.[] | select(.operation == "Status")] | length' "$base_norm")"
n_cur_status="$(jq '[.[] | select(.operation == "Status")] | length' "$cur_norm")"
report status-only INFO "baseline $n_base_status entries $base_sh, current $n_cur_status entries $cur_sh, $([[ "$base_sh" == "$cur_sh" ]] && echo same || echo different)"

if [[ "$failed" -eq 0 ]]; then
    echo 'JOURNAL | total | PASS'
    exit 0
fi
echo 'JOURNAL | total | FAIL'
exit 1
