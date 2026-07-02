#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# §IV evidence capture — per-UC JSON produced by moyun-pos itself.
#
# Replays the SPF e2e suites (spf_e2e_basic, spf_e2e_reports) with
# UC_EVIDENCE_DIR set: each test then exports, for every fiscal event
# it archived, the request JSON that the moyun-pos command layer
# actually generated and the FDM response it archived, to
# docs/SUBMISSION_ARTIFACTS/evidence/uc/.
#
# This complements RESULTATS_TESTS_UC_FR.md: the fixture JSON in that
# document is the SPF's published reference; these files prove our
# implementation produces conforming payloads.
#
# Usage: scripts/capture_uc_evidence.sh
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

OUT="$PROJECT_ROOT/docs/SUBMISSION_ARTIFACTS/evidence/uc"
mkdir -p "$OUT"

UC_EVIDENCE_DIR="$OUT" cargo test -p moyun-pos \
    --test spf_e2e_basic --test spf_e2e_reports

echo ""
echo "✓ evidence written to $OUT"
ls -1 "$OUT" | head -40
echo "($(ls -1 "$OUT" | wc -l | tr -d ' ') files)"
