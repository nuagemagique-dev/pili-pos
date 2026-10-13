#!/usr/bin/env python3
"""Stub of lane_paths.py for the p7_gate.sh self-test: fixed regexes, --check by environment."""
import os
import sys

args = sys.argv[1:]
if args[:1] == ["--check"]:
    if os.environ.get("STUB_LANES_FAIL") == "1":
        print("LANES | total | FAIL | stub")
        sys.exit(1)
    print("LANES | total | PASS | stub")
    sys.exit(0)
if args[:1] == ["--regex"] and len(args) == 2:
    regexes = {
        "e2e-parity": "^(docs/android/uc/.*|scripts/android/(test_)?(p7|uc)_[a-z0-9_]+[.](sh|py)|android/uccheck/.*)$",
        "shared": "^(docs/android/DECISIONS[.]md)$",
    }
    if os.environ.get("STUB_REGEX_EMPTY") == "1":
        regexes = {k: "" for k in regexes}
    print(regexes.get(args[1], ""))
    sys.exit(0)
sys.exit(2)
