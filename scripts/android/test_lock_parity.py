#!/usr/bin/env python3
"""Self-test of scripts/android/lock_parity.py (text fixtures, no cargo)."""

import io
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lock_parity  # noqa: E402

ROOT_TREE = """\
pos-engine v1.1.0 (/repo/crates/pos-engine)
serde v1.0.228
serde_derive v1.0.228 (proc-macro)
tokio v1.48.0
├── not-a-package-line
libc v0.2.177 (*)
libc v0.2.177 (*)

anyhow v1.0.100 feature "std"
"""

# Same set, other order, paths and markers, blank lines.
SAME_TREE = """\

tokio v1.48.0 (*)
libc v0.2.177
serde_derive v1.0.228 (proc-macro) (*)
serde v1.0.228
pos-engine v1.1.0 (/other/place/android/rust/../../crates/pos-engine)
"""


def run(root_text, android_text, allow_text=None):
    """Run main() on fixtures; return (exit code, stdout, stderr)."""
    with tempfile.TemporaryDirectory() as d:
        rp, ap = os.path.join(d, "root.txt"), os.path.join(d, "android.txt")
        with open(rp, "w") as f:
            f.write(root_text)
        with open(ap, "w") as f:
            f.write(android_text)
        argv = [rp, ap]
        if allow_text is not None:
            alp = os.path.join(d, "allow.txt")
            with open(alp, "w") as f:
                f.write(allow_text)
            argv += ["--allow", alp]
        out, err = io.StringIO(), io.StringIO()
        rc = lock_parity.main(argv, out, err)
        return rc, out.getvalue(), err.getvalue()


class LockParity(unittest.TestCase):
    def test_parse_strips_groups_and_ignores_noise(self):
        pairs = lock_parity.parse_tree(ROOT_TREE)
        self.assertEqual(
            pairs,
            {
                ("pos-engine", "1.1.0"),
                ("serde", "1.0.228"),
                ("serde_derive", "1.0.228"),
                ("tokio", "1.48.0"),
                ("libc", "0.2.177"),
                ("anyhow", "1.0.100"),
            },
        )

    def test_equal(self):
        rc, out, _ = run(ROOT_TREE.replace('anyhow v1.0.100 feature "std"', "anyhow v1.0.100"), SAME_TREE + "anyhow v1.0.100\n")
        self.assertEqual(rc, 0, out)
        self.assertIn("lock parity: 6 packages, identical", out)
        self.assertNotIn("allowed", out)

    def test_version_bump(self):
        android = SAME_TREE.replace("tokio v1.48.0", "tokio v1.49.0") + "anyhow v1.0.100\n"
        rc, out, _ = run(ROOT_TREE, android)
        self.assertEqual(rc, 1, out)
        self.assertIn("DIFF tokio: version differs, root 1.48.0, android 1.49.0", out)
        self.assertIn("FAIL", out)

    def test_extra_package(self):
        android = SAME_TREE + "anyhow v1.0.100\nminisign-verify v0.2.4\n"
        rc, out, _ = run(ROOT_TREE, android)
        self.assertEqual(rc, 1, out)
        self.assertIn("DIFF minisign-verify: only in android (0.2.4)", out)
        # and the other direction
        rc, out, _ = run(ROOT_TREE, SAME_TREE)
        self.assertEqual(rc, 1, out)
        self.assertIn("DIFF anyhow: only in root (1.0.100)", out)

    def test_allowed(self):
        android = SAME_TREE + "anyhow v1.0.100\nminisign-verify v0.2.4\n"
        allow = "# header\n\nminisign-verify 0.2.4 added for verify_minisign (P2-T1)\n"
        rc, out, _ = run(ROOT_TREE, android, allow)
        self.assertEqual(rc, 0, out)
        self.assertIn("ALLOWED minisign-verify 0.2.4 (android only): added for verify_minisign", out)
        self.assertIn("lock parity: 6 packages, identical, 1 allowed", out)

    def test_allowed_version_bump_needs_both_lines(self):
        android = SAME_TREE.replace("tokio v1.48.0", "tokio v1.49.0") + "anyhow v1.0.100\n"
        rc, out, _ = run(ROOT_TREE, android, "tokio 1.49.0 newer in android\n")
        self.assertEqual(rc, 1, out)
        rc, out, _ = run(ROOT_TREE, android, "tokio 1.49.0 newer in android\ntokio v1.48.0 older in root\n")
        self.assertEqual(rc, 0, out)
        self.assertIn("2 allowed", out)

    def test_stale_allow(self):
        rc, out, _ = run(
            ROOT_TREE.replace('anyhow v1.0.100 feature "std"', "anyhow v1.0.100"),
            SAME_TREE + "anyhow v1.0.100\n",
            "ghost 9.9.9 nothing differs here\n",
        )
        self.assertEqual(rc, 1, out)
        self.assertIn("STALE allow line: ghost 9.9.9", out)

    def test_missing_reason(self):
        rc, _, err = run(ROOT_TREE, SAME_TREE, "tokio 1.49.0\n")
        self.assertEqual(rc, 2)
        self.assertIn("needs a reason", err)

    def test_empty_tree_is_an_error(self):
        rc, _, err = run("", SAME_TREE)
        self.assertEqual(rc, 2)
        self.assertIn("no package found", err)

    def test_usage(self):
        out, err = io.StringIO(), io.StringIO()
        self.assertEqual(lock_parity.main(["only-one"], out, err), 2)
        self.assertEqual(lock_parity.main(["a", "b", "--bogus"], out, err), 2)


if __name__ == "__main__":
    unittest.main(verbosity=1)
