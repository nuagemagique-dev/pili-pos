#!/usr/bin/env bash
# Stub of uc_e2e.sh, p7_powercut.sh and p7_soak.sh: records its arguments and prints a total line.
# STUB_TAG is the tag, STUB_TOTAL the verdict (PASS, FAIL or NONE for no total line).
printf 'script %s %s\n' "$(basename "$0")" "$*" >>"${STUB_LOG:-/dev/null}"
tag="${STUB_TAG:-UCE2E}"
case "$(basename "$0")" in
uc_e2e.sh) tag=UCE2E ;;
p7_powercut.sh) tag=POWERCUT ;;
p7_soak.sh) tag=SOAK ;;
esac
case "${STUB_TOTAL:-PASS}" in
PASS) echo "$tag | total | PASS | stub run" ;;
FAIL) echo "$tag | step | FAIL | broken"; echo "$tag | total | FAIL | 1 failed"; exit 1 ;;
NONE) echo "no total here" ;;
esac
