#!/usr/bin/env python3
"""Self-test of scripts/android/playwright_inventory.py (standard library only, no npm).

Run: python3 scripts/android/test_playwright_inventory.py
"""
import importlib.util
import io
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "playwright_inventory.py")
spec = importlib.util.spec_from_file_location("playwright_inventory", SCRIPT)
inv = importlib.util.module_from_spec(spec)
spec.loader.exec_module(inv)

S = inv.SEP

LISTING = (
    "Listing tests:\n"
    "  [e2e] › b.spec.ts:12:1 › second test\n"
    "  [e2e] › a.spec.ts:7:3 › Group › nested title\n"
    "  [e2e] › a.spec.ts:3:1 › first test\n"
    "  [e2e] › a.spec.ts:30:1 › first test\n"
    "Total: 4 tests in 2 files\n"
)


def run_cli(*args):
    out, err = io.StringIO(), io.StringIO()
    with redirect_stdout(out), redirect_stderr(err):
        rc = inv.main(list(args))
    return rc, out.getvalue(), err.getvalue()


class NormaliseTest(unittest.TestCase):
    def test_strips_prefix_and_position(self):
        self.assertEqual(
            inv.normalise_line("  [e2e] › a.spec.ts:7:3 › Group › nested title"),
            "a.spec.ts" + S + "Group" + S + "nested title",
        )

    def test_non_test_lines_ignored(self):
        for line in ("Listing tests:", "Total: 280 tests in 22 files", "", "   "):
            self.assertIsNone(inv.normalise_line(line))

    def test_title_with_colons_and_digits_is_kept(self):
        line = "  [e2e] › a.spec.ts:7:1 › at 12:30 the shop opens"
        self.assertEqual(inv.normalise_line(line), "a.spec.ts" + S + "at 12:30 the shop opens")

    def test_other_project_is_ignored(self):
        self.assertIsNone(inv.normalise_line("  [captures] › captures.spec.ts:5:1 › x"))

    def test_parse_sorts_keeps_duplicates_and_reads_total(self):
        tests, total = inv.parse_list(LISTING)
        self.assertEqual(total, 4)
        self.assertEqual(
            tests,
            [
                "a.spec.ts" + S + "Group" + S + "nested title",
                "a.spec.ts" + S + "first test",
                "a.spec.ts" + S + "first test",
                "b.spec.ts" + S + "second test",
            ],
        )


class CliTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = self.tmp.name
        self.baseline = os.path.join(self.dir, "list.txt")
        self.moves = os.path.join(self.dir, "moves.tsv")
        self.current = os.path.join(self.dir, "current.txt")
        self.write(self.current, LISTING)

    def write(self, path, text):
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(text)

    def diff(self, *extra):
        return run_cli("--diff", "--current", self.current, "--baseline", self.baseline,
                       "--moves", self.moves, "--min", "4", *extra)

    def make_baseline(self):
        rc, out, _ = run_cli("--write", "--current", self.current, "--baseline", self.baseline)
        self.assertEqual(rc, 0, out)

    def test_write_header_and_lines(self):
        self.make_baseline()
        with open(self.baseline, encoding="utf-8") as fh:
            lines = fh.read().splitlines()
        header = [l for l in lines if l.startswith("#")]
        body = [l for l in lines if not l.startswith("#")]
        self.assertTrue(any("source commit" in l for l in header))
        self.assertTrue(any("total: 4 tests in 2 files" in l for l in header))
        self.assertEqual(len(body), 4)
        self.assertEqual(body, sorted(body, key=lambda s: s.encode("utf-8")))

    def test_diff_unchanged(self):
        self.make_baseline()
        rc, out, _ = self.diff()
        self.assertEqual(rc, 0)
        self.assertIn("playwright inventory: added 0, removed 0, total 4", out)

    def test_diff_removed_test_fails(self):
        self.make_baseline()
        self.write(self.current, LISTING.replace("  [e2e] › b.spec.ts:12:1 › second test\n", "")
                   .replace("Total: 4", "Total: 3"))
        rc, out, _ = self.diff()
        self.assertEqual(rc, 1)
        self.assertIn("removed 1", out)
        self.assertIn("removed: b.spec.ts" + S + "second test", out)

    def test_diff_duplicate_lost_fails(self):
        self.make_baseline()
        self.write(self.current, LISTING.replace("  [e2e] › a.spec.ts:30:1 › first test\n", "")
                   .replace("Total: 4", "Total: 3"))
        rc, out, _ = self.diff()
        self.assertEqual(rc, 1)
        self.assertIn("removed 1", out)

    def test_diff_added_test_passes(self):
        self.make_baseline()
        self.write(self.current, LISTING.replace("Total: 4 tests", "Total: 5 tests")
                   .replace("Total:", "  [e2e] › c.spec.ts:1:1 › new test\nTotal:"))
        rc, out, _ = self.diff()
        self.assertEqual(rc, 0)
        self.assertIn("added 1, removed 0, total 5", out)

    def test_diff_below_floor_fails(self):
        self.make_baseline()
        rc, out, _ = run_cli("--diff", "--current", self.current, "--baseline", self.baseline,
                             "--moves", self.moves, "--min", "5")
        self.assertEqual(rc, 1)
        self.assertIn("below the floor", out)

    def test_move_rule_follows_a_rename(self):
        self.make_baseline()
        self.write(self.current, LISTING.replace("second test", "second test renamed"))
        rc, _, _ = self.diff()
        self.assertEqual(rc, 1)
        self.write(self.moves, "# comment\n\nb.spec.ts" + S + "second test\tb.spec.ts" + S + "second test renamed\n")
        rc, out, _ = self.diff()
        self.assertEqual(rc, 0, out)
        self.assertIn("added 0, removed 0, total 4", out)

    def test_stale_rule_old_title_still_present(self):
        self.make_baseline()
        self.write(self.moves, "a.spec.ts" + S + "first test\tb.spec.ts" + S + "second test\n")
        rc, out, _ = self.diff()
        self.assertEqual(rc, 1)
        self.assertIn("still present", out)

    def test_stale_rule_new_title_absent(self):
        self.make_baseline()
        self.write(self.current, LISTING.replace("second test", "second test renamed"))
        self.write(self.moves, "b.spec.ts" + S + "second test\tb.spec.ts" + S + "nowhere\n")
        rc, out, _ = self.diff()
        self.assertEqual(rc, 1)
        self.assertIn("new title is absent", out)

    def test_two_rules_same_old_title(self):
        self.make_baseline()
        self.write(self.current, LISTING.replace("second test", "second test renamed"))
        old = "b.spec.ts" + S + "second test"
        new = "b.spec.ts" + S + "second test renamed"
        self.write(self.moves, "%s\t%s\n%s\t%s\n" % (old, new, old, new))
        rc, out, _ = self.diff()
        self.assertEqual(rc, 1)
        self.assertIn("same old title", out)

    def test_usage_errors_exit_2(self):
        self.assertEqual(run_cli()[0], 2)
        self.assertEqual(run_cli("--diff", "--write")[0], 2)
        rc, _, _ = run_cli("--diff", "--current", self.current, "--baseline", os.path.join(self.dir, "missing.txt"))
        self.assertEqual(rc, 2)
        self.make_baseline()
        self.write(self.moves, "only-one-column\n")
        self.assertEqual(self.diff()[0], 2)

    def test_total_mismatch_is_an_input_error(self):
        self.make_baseline()
        self.write(self.current, LISTING.replace("Total: 4", "Total: 9"))
        self.assertEqual(self.diff()[0], 2)

    def test_failed_npx_exit_2(self):
        # No ui/ checkout is needed: a stub npx on PATH that fails.
        bindir = os.path.join(self.dir, "bin")
        os.makedirs(bindir)
        stub = os.path.join(bindir, "npx")
        self.write(stub, "#!/bin/sh\necho boom >&2\nexit 7\n")
        os.chmod(stub, 0o755)
        self.make_baseline()
        env = dict(os.environ, PATH=bindir + os.pathsep + os.environ.get("PATH", ""))
        proc = subprocess.run(
            [sys.executable, SCRIPT, "--diff", "--baseline", self.baseline, "--moves", self.moves],
            env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, universal_newlines=True,
        )
        self.assertEqual(proc.returncode, 2, proc.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=1)
