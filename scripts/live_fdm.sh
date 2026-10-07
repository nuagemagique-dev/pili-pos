#!/usr/bin/env bash
# Sign an operator day against a live FDM (the module2 cloud simulator).
#
#   scripts/live_fdm.sh                                # the whole day (about 17 calls)
#   scripts/live_fdm.sh live_fdm_status_and_one_sale   # 2 calls, after switching a
#                                                      # simulated error in module2
#
# Reads PILI_LIVE_FDM_URL and PILI_LIVE_FDM_TOKEN from .env.live
# (git-ignored; the token is never printed). Calls go one at a time,
# 1 s apart, at most PILI_LIVE_MAX_REQUESTS (60) per run, through the
# pacing proxy of crates/pos-engine-testkit/src/live.rs. Each signed request
# and response is written to UC_EVIDENCE_DIR
# (docs/SUBMISSION_ARTIFACTS/Report_Testing/module2 by default). With
# PILI_LIVE_RECORD_DIR set, each call is also recorded raw there (the
# fixtures of crates/pos-fdm-mock/fixtures/module2).
# See docs/internal/LIVE_FDM_MODULE2.md.
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

export UC_EVIDENCE_DIR="${UC_EVIDENCE_DIR:-$ROOT/docs/SUBMISSION_ARTIFACTS/Report_Testing/module2}"
echo "→ live FDM: ${PILI_LIVE_FDM_URL}"
echo "→ token: $([[ -n "${PILI_LIVE_FDM_TOKEN:-}" ]] && echo set || echo none)"
echo "→ evidence: ${UC_EVIDENCE_DIR}"

cargo test -p pos-engine --test live_fdm -- --ignored --test-threads=1 --nocapture "$@" 2>&1 \
    | tee "$ROOT/target/live-fdm.log" \
    | grep -E "^(live FDM call|[0-9]+ live calls|test |thread|  left|  right|assertion)" || true

grep -q "test result: ok" "$ROOT/target/live-fdm.log"
