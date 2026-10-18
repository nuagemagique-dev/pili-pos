#!/usr/bin/env bash
# Stub of check_android_frozen.sh: records its arguments, fails when STUB_FROZEN_FAIL=1.
printf 'frozen %s\n' "$*" >>"${STUB_LOG:-/dev/null}"
if [[ "${STUB_FROZEN_FAIL:-0}" == "1" ]]; then
    echo "not frozen (stub)"
    exit 1
fi
echo "frozen stub $*"
