#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# §IV evidence capture — per-UC JSON produced by moyun-pos itself.
#
# Replays the official fixtures through the real commands of the
# certification build (--no-default-features --features cert) with
# UC_EVIDENCE_DIR set:
#   - spf_e2e_fixtures: Basic Set v1.7, one test per UC, orders built
#     from the fixture's products; writes <UC>_check.json, the
#     field-by-field comparison with the fixture;
#   - spf_e2e_report_fixtures: the Reports v1.3 service day T1–T8
#     followed by the M180, M182 and M184 reports.
# For every fiscal event a test archived, the request moyun-pos built
# and the FDM response it archived are written to
# docs/SUBMISSION_ARTIFACTS/evidence/uc/, then embedded into
# RESULTATS_TESTS_UC_FR.md by scripts/embed_uc_evidence.py.
#
# The FDM is a mock that replays the fixture's response; the same
# tests run against the Checkbox during the rehearsal.
#
# Usage: scripts/capture_uc_evidence.sh
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

OUT="$PROJECT_ROOT/docs/SUBMISSION_ARTIFACTS/evidence/uc"
mkdir -p "$OUT"
rm -f "$OUT"/*.json   # stale evidence from renamed/renumbered UCs

UC_EVIDENCE_DIR="$OUT" cargo test -p moyun-pos \
    --no-default-features --features cert \
    --test spf_e2e_fixtures --test spf_e2e_report_fixtures

python3 "$PROJECT_ROOT/scripts/embed_uc_evidence.py"

echo ""
echo "✓ evidence written to $OUT"
ls -1 "$OUT" | head -40
echo "($(ls -1 "$OUT" | wc -l | tr -d ' ') files)"
