#!/usr/bin/env python3
"""Self-test of lane_paths.py. Run: python3 -I scripts/android/test_lane_paths.py"""
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "lane_paths.py")
REPO = os.path.dirname(os.path.dirname(HERE))
PKG = "android/app/src/main/kotlin/dev/nuagemagique/pilipos"
RES = "android/app/src/main/res"

BASE_LANES = """# demo
```lanes
# comment

foundation android/app/src/main/kotlin/dev/nuagemagique/pilipos/(engine|shell)/.*
foundation android/app/src/main/res/values(-[a-z]+)?/strings_(vue_[a-z]+|shell|engine|provisioning|kiosk)[.]xml
foundation docs/android/LANES[.]md
ui-common android/app/src/main/kotlin/dev/nuagemagique/pilipos/ui/.*
ui-common android/app/src/main/res/values(-[a-z]+)?/strings_ui_common[.]xml
ui-login android/app/src/main/kotlin/dev/nuagemagique/pilipos/feature/login/.*
ui-login android/app/src/main/res/values(-[a-z]+)?/strings_ui_login[.]xml
platform android/app/src/main/kotlin/dev/nuagemagique/pilipos/platform/.*
platform android/app/src/main/res/values(-[a-z]+)?/strings_plat_[a-z]+[.]xml
shared docs/android/DECISIONS[.]md
```

```lane-probes
android/app/src/main/kotlin/dev/nuagemagique/pilipos/engine/A.kt foundation
android/app/src/main/kotlin/dev/nuagemagique/pilipos/ui/B.kt ui-common
android/app/src/main/kotlin/dev/nuagemagique/pilipos/feature/login/C.kt ui-login
```
"""


def run(*args, cwd=None):
    r = subprocess.run([sys.executable, "-I", SCRIPT] + list(args), capture_output=True, text=True, cwd=cwd)
    return r.returncode, r.stdout, r.stderr


class Env:
    """Temporary git repo plus LANES file."""

    def __init__(self, lanes_text=BASE_LANES):
        self.dir = tempfile.mkdtemp(prefix="lanes_")
        subprocess.run(["git", "init", "-q", self.dir], check=True)
        self.write("docs/android/LANES.md", lanes_text)

    def write(self, rel, text=""):
        p = os.path.join(self.dir, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w", encoding="utf-8") as fh:
            fh.write(text)
        return p

    def check(self):
        return run("--check", "--root", self.dir)

    def close(self):
        shutil.rmtree(self.dir, ignore_errors=True)


def strings(*names):
    body = "".join('  <string name="%s">x</string>\n' % n for n in names)
    return '<?xml version="1.0" encoding="utf-8"?>\n<resources>\n%s</resources>\n' % body


class LanePathsTest(unittest.TestCase):
    def setUp(self):
        self.env = Env()
        self.addCleanup(self.env.close)

    def lanes(self, text):
        return self.env.write("docs/android/LANES.md", text)

    # parsing
    def test_01_parse_comments_and_blank_lines(self):
        code, out, err = run("--list-owners", "--root", self.env.dir)
        self.assertEqual(code, 0, err)
        self.assertEqual(out.split(), ["foundation", "ui-common", "ui-login", "platform", "shared"])

    def test_02_malformed_rule_line_fails(self):
        self.lanes(BASE_LANES.replace("shared docs/android/DECISIONS[.]md", "shared"))
        code, out, err = self.env.check()
        self.assertEqual(code, 1)
        self.assertIn("malformed", err)
        self.assertIn("FAIL", out)

    def test_03_bad_regex_fails(self):
        self.lanes(BASE_LANES.replace("docs/android/DECISIONS[.]md", "docs/(android"))
        code, _, err = self.env.check()
        self.assertEqual(code, 1)
        self.assertIn("does not compile", err)

    def test_04_non_posix_regex_fails(self):
        self.lanes(BASE_LANES.replace("docs/android/DECISIONS[.]md", "docs/\\d+"))
        code, _, err = self.env.check()
        self.assertEqual(code, 1)
        self.assertIn("POSIX", err)

    # --regex
    def test_05_regex_single_owner(self):
        code, out, _ = run("--regex", "platform", "--root", self.env.dir)
        self.assertEqual(code, 0)
        rx = out.strip()
        self.assertTrue(rx.startswith("^(") and rx.endswith(")$"))
        self.assertTrue(re.match(rx, PKG + "/platform/X.kt"))
        self.assertFalse(re.match(rx, PKG + "/engine/X.kt"))

    def test_06_regex_ui_prefix(self):
        code, out, _ = run("--regex", "ui", "--root", self.env.dir)
        self.assertEqual(code, 0)
        rx = out.strip()
        self.assertTrue(re.match(rx, PKG + "/ui/components/NumPad.kt"))
        self.assertTrue(re.match(rx, PKG + "/feature/login/LoginRoutes.kt"))
        self.assertFalse(re.match(rx, PKG + "/platform/PlatformRoutes.kt"))
        self.assertFalse(re.match(rx, PKG + "/engine/EngineHolder.kt"))
        # 'ui' must not match an owner that merely starts with 'ui' letters
        code, _, _ = run("--regex", "u", "--root", self.env.dir)
        self.assertEqual(code, 2)

    def test_07_regex_unknown_owner_exit_2(self):
        code, out, _ = run("--regex", "nobody", "--root", self.env.dir)
        self.assertEqual(code, 2)
        self.assertEqual(out, "")

    @unittest.skipUnless(shutil.which("bash"), "bash missing")
    def test_08_bash_and_python_regex_agree(self):
        samples = [
            PKG + "/ui/components/NumPad.kt",
            PKG + "/feature/login/LoginRoutes.kt",
            PKG + "/feature/loginx/Other.kt",
            PKG + "/platform/PlatformRoutes.kt",
            PKG + "/engine/EngineHolder.kt",
            RES + "/values-zh/strings_ui_common.xml",
            RES + "/values/strings_ui_login.xml",
            RES + "/values/strings_plat_about.xml",
            "docs/android/DECISIONS.md",
            "Cargo.lock",
        ]
        for owner in ("ui", "platform", "foundation", "shared"):
            _, out, _ = run("--regex", owner, "--root", self.env.dir)
            rx = out.strip()
            for s in samples:
                py = bool(re.match(rx, s))
                sh = subprocess.run(
                    ["bash", "-c", '[[ "$1" =~ $2 ]]', "x", s, rx], capture_output=True
                )
                self.assertIn(sh.returncode, (0, 1), "bash rejected %s: %s" % (rx, sh.stderr))
                self.assertEqual(py, sh.returncode == 0, "%s vs %s" % (owner, s))

    # --owner
    def test_09_owner_modes(self):
        for path, want in (
            (PKG + "/feature/login/LoginRoutes.kt", "ui-login"),
            (PKG + "/platform/x/Y.kt", "platform"),
            (PKG + "/engine/EngineHolder.kt", "foundation"),
            ("README.md", "none"),
        ):
            code, out, err = run("--owner", path, "--root", self.env.dir)
            self.assertEqual((code, out.strip()), (0, want), err)

    def test_10_owner_two_matches_exit_1(self):
        self.lanes(BASE_LANES.replace("```lane-probes", "```lanes\nplatform docs/android/DECISIONS[.]md\n```\n\n```lane-probes", 1))
        code, _, err = run("--owner", "docs/android/DECISIONS.md", "--root", self.env.dir)
        self.assertEqual(code, 1)
        self.assertIn("several owners", err)

    # --check: probes
    def test_11_probe_wrong_owner_fails(self):
        self.lanes(BASE_LANES.replace("feature/login/C.kt ui-login", "feature/login/C.kt platform"))
        code, _, err = self.env.check()
        self.assertEqual(code, 1)
        self.assertIn("expected platform", err)

    def test_12_probe_without_rule_fails(self):
        self.lanes(BASE_LANES.replace("feature/login/C.kt ui-login", "feature/orphan/C.kt ui-login"))
        code, _, err = self.env.check()
        self.assertEqual(code, 1)
        self.assertIn("matches no rule", err)

    def test_13_probe_matching_two_owners_fails(self):
        self.lanes(BASE_LANES.replace("```lane-probes", "```lanes\nplatform android/app/src/main/kotlin/dev/nuagemagique/pilipos/ui/.*\n```\n\n```lane-probes", 1))
        code, _, err = self.env.check()
        self.assertEqual(code, 1)
        self.assertIn("matches several owners", err)

    def test_14_same_owner_twice_is_fine(self):
        self.lanes(BASE_LANES.replace("```lane-probes", "```lanes\nfoundation android/app/src/main/kotlin/dev/nuagemagique/pilipos/engine/A[.]kt\n```\n\n```lane-probes", 1))
        code, out, err = self.env.check()
        self.assertEqual(code, 0, err)
        self.assertIn("LANES | total | PASS |", out)

    # --check: tree
    def test_15_unowned_file_under_android_app_fails(self):
        self.env.write("android/app/src/main/kotlin/dev/nuagemagique/pilipos/Stray.kt", "")
        code, _, err = self.env.check()
        self.assertEqual(code, 1)
        self.assertIn("Stray.kt under android/app/ has no owner", err)

    def test_16_unowned_file_outside_android_app_is_fine(self):
        self.env.write("README.md", "x")
        code, _, err = self.env.check()
        self.assertEqual(code, 0, err)

    def test_17_ignored_untracked_file_is_skipped(self):
        self.env.write(".gitignore", "android/app/build/\n")
        self.env.write("android/app/build/out.bin", "")
        code, _, err = self.env.check()
        self.assertEqual(code, 0, err)

    def test_18_file_owned_twice_fails(self):
        self.lanes(BASE_LANES.replace("```lane-probes", "```lanes\nplatform docs/android/.*\n```\n\n```lane-probes", 1))
        code, _, err = self.env.check()
        self.assertEqual(code, 1)
        self.assertIn("docs/android/LANES.md matches several owners", err)

    # --check: strings
    def test_19_strings_prefix_passes(self):
        self.env.write(RES + "/values/strings_shell.xml", strings("shell_title", "shell_ok"))
        self.env.write(RES + "/values-zh/strings_ui_login.xml", strings("ui_login_pin"))
        self.env.write(RES + "/values/strings_plat_about.xml", strings("plat_about_title"))
        code, out, err = self.env.check()
        self.assertEqual(code, 0, err)
        self.assertIn("PASS", out)

    def test_20_strings_missing_prefix_fails(self):
        self.env.write(RES + "/values/strings_shell.xml", strings("shell_title", "kiosk_oops"))
        code, _, err = self.env.check()
        self.assertEqual(code, 1)
        self.assertIn("'kiosk_oops' lacks prefix 'shell_'", err)

    def test_21_plurals_and_arrays_are_checked(self):
        xml = '<resources><plurals name="shell_n"><item quantity="one">1</item></plurals><string-array name="bad_arr"><item>a</item></string-array></resources>'
        self.env.write(RES + "/values/strings_shell.xml", xml)
        code, _, err = self.env.check()
        self.assertEqual(code, 1)
        self.assertIn("'bad_arr' lacks prefix", err)
        self.assertNotIn("'shell_n'", err)

    def test_22_unknown_strings_file_fails(self):
        self.env.write(RES + "/values/strings_misc.xml", strings("misc_a"))
        code, _, err = self.env.check()
        self.assertEqual(code, 1)
        self.assertIn("unknown hand-written strings file name 'strings_misc.xml'", err)

    def test_23_unknown_ui_area_fails(self):
        self.env.write(RES + "/values/strings_ui_nothing.xml", strings("ui_nothing_a"))
        code, _, err = self.env.check()
        self.assertEqual(code, 1)
        self.assertIn("unknown hand-written strings file name", err)

    def test_24_vue_strings_exempt(self):
        self.env.write(RES + "/values/strings_vue_core.xml", strings("anything_goes", "whatever"))
        code, _, err = self.env.check()
        self.assertEqual(code, 0, err)

    def test_24b_nightly_strings_need_the_nightly_prefix(self):
        self.env.write(
            "docs/android/LANES.md",
            BASE_LANES.replace(
                "shared docs/android/DECISIONS",
                "nightly android/app/src/main/res/values(-[a-z]+)?/strings_nightly[.]xml\nshared docs/android/DECISIONS",
                1,
            ),
        )
        self.env.write(RES + "/values/strings_nightly.xml", strings("nightly_title", "nightly_retry"))
        code, _, err = self.env.check()
        self.assertEqual(code, 0, err)
        self.env.write(RES + "/values-en/strings_nightly.xml", strings("nightly_title", "shell_oops"))
        code, _, err = self.env.check()
        self.assertEqual(code, 1)
        self.assertIn("'shell_oops' lacks prefix 'nightly_'", err)

    # the real file
    def test_25_real_lanes_file_passes_check(self):
        code, out, err = run("--check", "--root", REPO)
        self.assertEqual(code, 0, err)
        self.assertTrue(out.startswith("LANES | total | PASS | "), out)

    def test_26_real_lanes_file_has_all_owners_and_enough_probes(self):
        code, out, _ = run("--list-owners", "--root", REPO)
        owners = out.split()
        for o in ("foundation", "ui-common", "platform", "shared", "frozen") + tuple(
            "ui-" + v for v in ("login", "tables", "order", "checkout", "reports", "settings", "webshop")
        ):
            self.assertIn(o, owners)
        with open(os.path.join(REPO, "docs/android/LANES.md"), encoding="utf-8") as fh:
            text = fh.read()
        self.assertNotIn(chr(0x2014), text)
        probes = re.search(r"```lane-probes\n(.*?)\n```", text, re.S).group(1).split("\n")
        self.assertGreaterEqual(len([p for p in probes if p.strip()]), 50)

    def test_27_real_regexes_work_in_bash_too(self):
        if not shutil.which("bash"):
            self.skipTest("bash missing")
        for owner in ("ui", "platform", "foundation", "frozen", "shared"):
            _, out, _ = run("--regex", owner, "--root", REPO)
            rx = out.strip()
            sh = subprocess.run(["bash", "-c", '[[ "x" =~ $1 ]]', "x", rx], capture_output=True)
            self.assertIn(sh.returncode, (0, 1), "bash rejected the %s regex: %s" % (owner, sh.stderr))


if __name__ == "__main__":
    unittest.main(verbosity=1)
