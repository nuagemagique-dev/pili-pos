#!/usr/bin/env bash
# Online orders live on both sides: placed on the real HubRise API (test
# location, connected by scripts/live_hubrise_login.sh) and signed by a
# live FDM (the module2 cloud simulator).
#
#   scripts/live_combined.sh live_prepaid_order_signed_by_the_fdm_then_refunded
#   scripts/live_combined.sh live_unpaid_order_registered_then_sold_by_the_fdm
#
# Each test places one HubRise order (free plan: 5 a month) and signs two
# FDM events. Reads PILI_LIVE_FDM_URL and PILI_LIVE_FDM_TOKEN from
# .env.live (git-ignored; the token is never printed). The pacing and
# recording variables of scripts/live_fdm.sh and scripts/live_hubrise.sh
# apply. A run cut short after placing its order can take that order over
# instead of placing another: PILI_LIVE_HUBRISE_REUSE_ORDER=<order id>.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ -f .env.live ]]; then
    set -a
    # shellcheck disable=SC1091
    . ./.env.live
    set +a
fi
if [[ -z "${PILI_LIVE_FDM_URL:-}" ]]; then
    echo "✗ PILI_LIVE_FDM_URL is not set (put it in .env.live)" >&2
    exit 1
fi
if [[ ! -f target/live-hubrise/token.json ]]; then
    echo "✗ no HubRise token: run scripts/live_hubrise_login.sh first" >&2
    exit 1
fi
if [[ $# -eq 0 ]]; then
    echo "✗ name one test (see the header of this script)" >&2
    exit 1
fi

export UC_EVIDENCE_DIR="${UC_EVIDENCE_DIR:-$ROOT/target/live-combined/evidence}"
echo "→ live FDM: ${PILI_LIVE_FDM_URL}"
echo "→ token: $([[ -n "${PILI_LIVE_FDM_TOKEN:-}" ]] && echo set || echo none)"
echo "→ evidence: ${UC_EVIDENCE_DIR}"

cargo test -p pili-pos --features hubrise --test live_combined -- --ignored --exact \
    --test-threads=1 --nocapture "$@" 2>&1 \
    | tee "$ROOT/target/live-combined.log" \
    | grep -E "^(live FDM call|[0-9]+ live calls|live HubRise|outbox pass|[0-9]+ live HubRise calls|test |thread|  left|  right|assertion)" || true

grep -q "test result: ok. 1 passed" "$ROOT/target/live-combined.log"
