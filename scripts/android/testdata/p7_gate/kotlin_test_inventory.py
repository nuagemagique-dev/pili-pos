#!/usr/bin/env python3
"""Stub of kotlin_test_inventory.py: prints a KTESTS total line, FAIL when STUB_KTESTS_FAIL=1."""
import os
import sys

if os.environ.get("STUB_KTESTS_FAIL") == "1":
    print("KTESTS | total | FAIL | base 5, head 4, removed 1")
    sys.exit(1)
print("KTESTS | total | PASS | base 5, head 5, removed 0")
