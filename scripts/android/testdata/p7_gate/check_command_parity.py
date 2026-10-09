#!/usr/bin/env python3
"""Stub of check_command_parity.py: exits 1 when STUB_CMDPARITY_FAIL=1."""
import os
import sys

if os.environ.get("STUB_CMDPARITY_FAIL") == "1":
    print("missing: some_command")
    sys.exit(1)
print("missing: none")
