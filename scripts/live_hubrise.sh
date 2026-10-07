#!/usr/bin/env bash
# Run the HubRise integration against the real HubRise API, on the test
# location connected by scripts/live_hubrise_login.sh. The FDM is the
# in-process mock: nothing is signed by a real FDM here.
#
#   scripts/live_hubrise.sh live_catalog_is_mirrored              # no order
#   scripts/live_hubrise.sh live_prepaid_order_signed_then_cancelled_and_refunded
#   scripts/live_hubrise.sh live_unpaid_order_registered_then_collected
#
# The test location is on HubRise's free plan (5 orders a month); each
# order test places one. Calls go one at a time, 1 s apart, at most
# PILI_LIVE_HUBRISE_MAX_REQUESTS (40) per run, through the pacing proxy
# of crates/pos-engine-testkit/src/live_hubrise.rs. With PILI_LIVE_RECORD_DIR
# set, each call is recorded there with customer details and tokens
# removed.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ ! -f target/live-hubrise/token.json ]]; then
    echo "✗ no token: run scripts/live_hubrise_login.sh first" >&2
    exit 1
fi
if [[ $# -eq 0 ]]; then
    echo "✗ name one test (see the header of this script)" >&2
    exit 1
fi

cargo test -p pos-engine --features hubrise --test live_hubrise -- --ignored --exact \
    --test-threads=1 --nocapture "$@" 2>&1 \
    | tee "$ROOT/target/live-hubrise.log" \
    | grep -E "^(live HubRise|outbox pass|[0-9]+ live HubRise calls|test |thread|  left|  right|assertion)" || true

grep -q "test result: ok. 1 passed" "$ROOT/target/live-hubrise.log"
