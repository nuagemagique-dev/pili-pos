#!/usr/bin/env python3
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of apk_entry_diff.py (P3-T8)
#
# Usage: python3 scripts/android/test_apk_entry_diff.py
#
# Builds synthetic zips in a temporary directory (nothing is written elsewhere) and checks
# the exit status and the report: equal contents with different timestamps and compression,
# a changed entry, a missing entry, a changed order, a bad file and a usage error.
# Exit 0 when every case passes, 1 otherwise.
# ─────────────────────────────────────────────────────────────────
import os
import subprocess
import sys
import tempfile
import unittest
import zipfile

TOOL = os.path.join(os.path.dirname(os.path.abspath(__file__)), "apk_entry_diff.py")


def make_zip(path, entries, date_time=(1981, 1, 1, 0, 0, 0), method=zipfile.ZIP_DEFLATED):
    with zipfile.ZipFile(path, "w") as z:
        for name, data in entries:
            info = zipfile.ZipInfo(name, date_time)
            info.compress_type = method
            z.writestr(info, data)


def run(*args):
    p = subprocess.run(
        [sys.executable, "-I", TOOL, *args], capture_output=True, text=True, check=False
    )
    return p.returncode, p.stdout, p.stderr


BASE = [
    ("AndroidManifest.xml", b"<manifest/>" * 50),
    ("classes.dex", bytes(range(256)) * 20),
    ("lib/arm64-v8a/libpos_ffi.so", b"\x7fELF" + b"\0" * 500),
]


class EntryDiffTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)

    def p(self, name):
        return os.path.join(self.tmp.name, name)

    def test_equal_contents_other_timestamps_and_compression(self):
        make_zip(self.p("a.apk"), BASE, (1981, 1, 1, 0, 0, 0))
        make_zip(self.p("b.apk"), BASE, (2026, 10, 7, 12, 30, 0), zipfile.ZIP_STORED)
        code, out, _ = run(self.p("a.apk"), self.p("b.apk"))
        self.assertEqual(code, 0, out)
        self.assertIn("RESULT | EQUAL", out)
        self.assertIn("META | classes.dex | csize,method,date_time", out)
        lines = [ln for ln in out.splitlines() if ln.startswith("MANIFEST")]
        self.assertEqual(len(lines), 2)
        self.assertEqual(lines[0].split(" | ")[2], lines[1].split(" | ")[2])

    def test_changed_entry(self):
        make_zip(self.p("a.apk"), BASE)
        changed = [BASE[0], ("classes.dex", b"other" * 100), BASE[2]]
        make_zip(self.p("b.apk"), changed)
        code, out, _ = run(self.p("a.apk"), self.p("b.apk"))
        self.assertEqual(code, 1, out)
        self.assertIn("DIFF | classes.dex | sha256 A ", out)
        self.assertNotIn("DIFF | AndroidManifest.xml", out)
        self.assertIn("RESULT | DIFFERENT | 1 entries", out)
        manifests = [ln.split(" | ")[2] for ln in out.splitlines() if ln.startswith("MANIFEST")]
        self.assertNotEqual(manifests[0], manifests[1])

    def test_missing_entry(self):
        make_zip(self.p("a.apk"), BASE)
        make_zip(self.p("b.apk"), BASE[:2])
        code, out, _ = run(self.p("a.apk"), self.p("b.apk"))
        self.assertEqual(code, 1, out)
        self.assertIn("ONLY | A | lib/arm64-v8a/libpos_ffi.so", out)
        code, out, _ = run(self.p("b.apk"), self.p("a.apk"))
        self.assertEqual(code, 1, out)
        self.assertIn("ONLY | B | lib/arm64-v8a/libpos_ffi.so", out)

    def test_order_only_is_meta(self):
        make_zip(self.p("a.apk"), BASE)
        make_zip(self.p("b.apk"), list(reversed(BASE)))
        code, out, _ = run(self.p("a.apk"), self.p("b.apk"))
        self.assertEqual(code, 0, out)
        self.assertIn("META | (order)", out)

    def test_not_a_zip_and_usage(self):
        with open(self.p("bad.apk"), "wb") as f:
            f.write(b"not a zip")
        make_zip(self.p("a.apk"), BASE)
        code, _, err = run(self.p("a.apk"), self.p("bad.apk"))
        self.assertEqual(code, 2)
        self.assertIn("apk_entry_diff:", err)
        code, _, _ = run(self.p("a.apk"))
        self.assertEqual(code, 2)
        code, _, _ = run(self.p("a.apk"), self.p("missing.apk"))
        self.assertEqual(code, 2)


if __name__ == "__main__":
    unittest.main(verbosity=2)
