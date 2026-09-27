#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - reproducible build check
#
# Runs the reference build (deploy/build-release.sh --docker) twice
# from a clean tree and checks both binaries have the same SHA-1: the
# value declared to SPF Finances must depend on the source only, never
# on the build host's state.
#
# Between the runs the build outputs are deleted (target/linux-amd64,
# ui/dist, deploy/dist, build_info_*.txt), so the second run is a full
# rebuild, not cargo returning its cached output.
#
# Leaves the second run's package in deploy/dist/ and, in OUT_DIR:
#   run1/, run2/   binary + build_info report of each run
#   sha1           the common SHA-1 (only written when both match)
#
# Usage: scripts/check_repro.sh [OUT_DIR]   (default: target/repro)
# Exit status: 0 when reproducible, 1 otherwise.
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${1:-$PROJECT_ROOT/target/repro}"
cd "$PROJECT_ROOT"

clean_outputs() {
    rm -rf deploy/dist target/linux-amd64 ui/dist
    rm -f build_info_*.txt
}

report_sha1() {
    grep '^SHA-1' "$1"/build_info_*.txt | awk '{print $3}'
}

rm -rf "$OUT_DIR"
for run in run1 run2; do
    echo "══ Reference build, $run ══"
    clean_outputs
    ./deploy/build-release.sh --docker
    mkdir -p "$OUT_DIR/$run"
    cp deploy/dist/pili-pos deploy/dist/build_info_*.txt "$OUT_DIR/$run/"
done

status=0
for run in run1 run2; do
    reported="$(report_sha1 "$OUT_DIR/$run")"
    actual="$(sha1sum "$OUT_DIR/$run/pili-pos" | awk '{print $1}')"
    echo "$run: report $reported, binary $actual"
    if [[ -z "$reported" || "$reported" != "$actual" ]]; then
        echo "✗ $run: the build_info report does not match its binary"
        status=1
    fi
done

sha1_1="$(sha1sum "$OUT_DIR/run1/pili-pos" | awk '{print $1}')"
sha1_2="$(sha1sum "$OUT_DIR/run2/pili-pos" | awk '{print $1}')"
if [[ "$sha1_1" != "$sha1_2" ]]; then
    echo "✗ Not reproducible: $sha1_1 != $sha1_2"
    status=1
fi

if [[ $status -eq 0 ]]; then
    echo "$sha1_1" > "$OUT_DIR/sha1"
    echo "✓ Reproducible: SHA-1 $sha1_1"
fi
exit $status
