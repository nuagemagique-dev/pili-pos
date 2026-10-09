#!/usr/bin/env python3
"""Stub of check_ui_parity.py: exits 1 when STUB_UIPARITY_FAIL=1."""
import os
import sys

if os.environ.get("STUB_UIPARITY_FAIL") == "1":
    print("UIPARITY | total | FAIL | missing 1")
    sys.exit(1)
print("UIPARITY | total | PASS | missing 0")
