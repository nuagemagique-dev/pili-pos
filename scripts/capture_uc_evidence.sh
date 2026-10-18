#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# §IV evidence capture - per-UC JSON produced by pili-pos itself.
#
# Replays the official fixtures through the real commands of the
# certification build (--no-default-features --features cert) with
# UC_EVIDENCE_DIR set:
#   - spf_e2e_fixtures: Basic Set v1.8, one test per UC, orders built
#     from the fixture's products; writes <UC>_check.json, the
#     field-by-field comparison with the fixture;
#   - spf_e2e_report_fixtures: the Reports v1.3 service day T1–T8
#     followed by the M180, M182 and M184 reports;
#   - invoice_advanced: the Advanced Set invoices UC4121 and UC4122 (event I);
#   - spf_e2e_gift_vouchers: the Advanced Set multi-purpose vouchers UC260
#     (sale), UC261 and UC262 (use);
#   - spf_e2e_rounding: the five situations of the manual's
#     "Arrondissement des méthodes de paiement" (ROUND1 to ROUND5; only
#     the cash is rounded, so basic tests 1, 2 and 3 apply). Besides the
#     request and the response, the printed ticket is written to
#     evidence/tickets/ROUND<n>_ticket.txt (TICKET_EVIDENCE_DIR).
# For every fiscal event a test archived, the request pili-pos built
# and the FDM response it archived are written to
# docs/SUBMISSION_ARTIFACTS/evidence/uc/, then embedded into
# RESULTATS_TESTS_UC_FR.md by scripts/embed_uc_evidence.py.
#
# The FDM is a mock that replays the fixture's response; the same
# commands sign against the module2 simulator in the rehearsal
# (scripts/live_fdm.sh).
#
# Usage: scripts/capture_uc_evidence.sh
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

OUT="$PROJECT_ROOT/docs/SUBMISSION_ARTIFACTS/evidence/uc"
mkdir -p "$OUT"
TICKETS="$PROJECT_ROOT/docs/SUBMISSION_ARTIFACTS/evidence/tickets"
mkdir -p "$TICKETS"
rm -f "$OUT"/*.json   # stale evidence from renamed/renumbered UCs
rm -f "$TICKETS"/ROUND*_ticket.txt

UC_EVIDENCE_DIR="$OUT" TICKET_EVIDENCE_DIR="$TICKETS" cargo test -p pos-engine \
    --no-default-features --features cert \
    --test spf_e2e_fixtures --test spf_e2e_report_fixtures \
    --test spf_e2e_rounding --test invoice_advanced \
    --test spf_e2e_gift_vouchers

python3 "$PROJECT_ROOT/scripts/embed_uc_evidence.py"

echo ""
echo "✓ evidence written to $OUT"
ls -1 "$OUT" | head -40
echo "($(ls -1 "$OUT" | wc -l | tr -d ' ') files)"
