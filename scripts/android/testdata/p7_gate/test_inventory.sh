#!/usr/bin/env bash
# Stub of test_inventory.sh --diff: STUB_WS is the ws count (default 1400), STUB_INV_FAIL=1 exits 1.
printf 'inventory %s\n' "$*" >>"${STUB_LOG:-/dev/null}"
if [[ "${STUB_INV_FAIL:-0}" == "1" ]]; then
    echo "x tests that disappeared:"
    exit 1
fi
echo "baseline 3000 lines, current 3001 lines: 1 added, 0 removed"
echo "ok no test disappeared (ws ${STUB_WS:-1400} >= 1355)"
