#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# §V evidence capture - POS behaviour on FDM warnings/errors.
#
# For each FDM message code required by §V of the certification
# manual (5 mandatory + selected optional), this script:
#   1. starts pos-fdm-mock with MOCK_FDM_ERROR / MOCK_FDM_WARNING,
#   2. sends the official M110 signSale payload (Basic Set v1.7
#      fixture) and a Status query,
#   3. records the JSON request + JSON response pairs under
#      docs/SUBMISSION_ARTIFACTS/evidence/fdm_errors/.
#
# The JSON half of the §V checkbox ("le JSON ainsi que les captures
# d'écran ont été ajoutés") comes from here; the screenshot half is
# taken by the operator following CAPTURES_ECRAN_RUNBOOK_FR.md.
#
# Usage: scripts/capture_fdm_error_evidence.sh
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

PORT=8091
URL="http://localhost:${PORT}/graphql"
OUT="$PROJECT_ROOT/docs/SUBMISSION_ARTIFACTS/evidence/fdm_errors"
FIXTURE_DIR="$PROJECT_ROOT/docs/20250916_UC_BasicSet_POS_FDM_1.7/M110_signSale"

mkdir -p "$OUT"

echo "── building pos-fdm-mock ─────────────────────────────────"
cargo build --release --package pos-fdm-mock

MOCK_BIN="$PROJECT_ROOT/target/release/pos-fdm-mock"
MOCK_PID=""

start_mock() { # $1 = env spec ("ERROR=CODE" or "WARNING=CODE[,CODE]")
    local kind="${1%%=*}" codes="${1#*=}"
    local err="" warn=""
    [[ "$kind" == "ERROR" ]] && err="$codes" || warn="$codes"
    MOCK_FDM_ERROR="$err" MOCK_FDM_WARNING="$warn" PORT="$PORT" \
        "$MOCK_BIN" >/dev/null 2>&1 &
    MOCK_PID=$!
    # wait for the socket
    for _ in $(seq 1 50); do
        curl -s -o /dev/null "$URL" -X POST -H 'content-type: application/json' \
            -d '{"query":"query { Status { initialized } }"}' && return 0
        sleep 0.1
    done
    echo "✗ mock did not come up"; exit 1
}

stop_mock() {
    [[ -n "$MOCK_PID" ]] && kill "$MOCK_PID" 2>/dev/null && wait "$MOCK_PID" 2>/dev/null || true
    MOCK_PID=""
}
trap stop_mock EXIT

# Official M110 payload: query text + variables from the SPF fixture.
SALE_QUERY=$(python3 -c "import json,sys; print(json.dumps(open(sys.argv[1]).read()))" \
    "$FIXTURE_DIR/M110_signSale_GQL-Query.graphql")
SALE_VARIABLES=$(cat "$FIXTURE_DIR/M110_signSale_GQL-Variables.json")
SALE_REQUEST=$(python3 - "$FIXTURE_DIR" <<'EOF'
import json, sys, pathlib
d = pathlib.Path(sys.argv[1])
req = {
    "query": (d / "M110_signSale_GQL-Query.graphql").read_text(),
    "variables": json.loads((d / "M110_signSale_GQL-Variables.json").read_text()),
}
print(json.dumps(req, indent=2, ensure_ascii=False))
EOF
)

# Exactly the query FdmClient::status() sends (spec SDL spelling with
# alias; message is a localized scalar, origin/code/display live under
# extensions - POS spec v2.2 MessageItem shape).
STATUS_REQUEST='{"query":"query { fdmStatus: Status(language: FR) { device { fdmId fdmSwVersion fdmDateTime bufferCapacityUsed } initialized informations { message extensions { category code showPos } } warnings { message extensions { category code showPos } } errors { message extensions { category code showPos } } }"}'

capture() { # $1 = env spec, $2 = code label for filenames
    local spec="$1" code="$2"
    echo "→ $code"
    start_mock "$spec"

    # signSale attempt (what the POS sends at checkout)
    echo "$SALE_REQUEST" > "$OUT/${code}_signSale_request.json"
    curl -s "$URL" -X POST -H 'content-type: application/json' \
        -d "$SALE_REQUEST" | python3 -m json.tool > "$OUT/${code}_signSale_response.json"

    # Status query (what FdmStatus.vue polls to display the message)
    echo "$STATUS_REQUEST" | python3 -m json.tool > "$OUT/${code}_status_request.json"
    curl -s "$URL" -X POST -H 'content-type: application/json' \
        -d "$STATUS_REQUEST" | python3 -m json.tool > "$OUT/${code}_status_response.json"

    stop_mock
}

# §V mandatory errors (affichage M)
capture "ERROR=BUFFER_FULL"          "BUFFER_FULL"
capture "ERROR=FDM_LOCKED"           "FDM_LOCKED"
capture "ERROR=FDM_NOT_OPERATIONAL"  "FDM_NOT_OPERATIONAL"
capture "ERROR=UNKNOWN_POS"          "UNKNOWN_POS"
# §V mandatory warnings (affichage M)
capture "WARNING=CLIENT_CERT_EXPIRED" "CLIENT_CERT_EXPIRED"
capture "WARNING=BUFFER_NEAR_FULL"    "BUFFER_NEAR_FULL"
# §V optional (at least one required - we capture two)
capture "ERROR=UNAUTHORIZED"          "UNAUTHORIZED"
capture "WARNING=RTC_SYNC_FAILED"     "RTC_SYNC_FAILED"

echo ""
echo "✓ evidence written to $OUT"
ls -1 "$OUT"
