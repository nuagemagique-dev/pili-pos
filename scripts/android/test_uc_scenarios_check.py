#!/usr/bin/env python3
"""Tests of scripts/android/uc_scenarios_check.py (card P7-T2).

Run: python3 scripts/android/test_uc_scenarios_check.py

The small trees under scripts/android/testdata/uc_scenarios_check/ are the scenario roots: `good`
passes, every `bad_*` tree has one problem and must FAIL with a message that names it. The
repository itself (seed, scripts/uc_matrix.py, official UC folders, docs/android/UC.md) is read
from the real tree.
"""

import hashlib
import importlib.util
import io
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
TREES = HERE / "testdata" / "uc_scenarios_check"
SMOKE_ROOT = REPO / "android/app/src/androidTest/assets/uc"
SMOKE_SHA256 = "198014c546dcab66ac40e8386d3806d3889b9d4069ca9b64088f16ab8456dcc5"

spec = importlib.util.spec_from_file_location("uc_scenarios_check", HERE / "uc_scenarios_check.py")
checker = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = checker
spec.loader.exec_module(checker)


def run_main(*argv):
    """(exit code, stdout, stderr) of checker.main(argv)."""
    out, err = io.StringIO(), io.StringIO()
    old = sys.stdout, sys.stderr
    sys.stdout, sys.stderr = out, err
    try:
        code = checker.main(list(argv))
    finally:
        sys.stdout, sys.stderr = old
    return code, out.getvalue(), err.getvalue()


def run_tree(name):
    return run_main("--root", str(TREES / name))


# tree -> a phrase the FAIL line must contain
BAD_TREES = {
    "bad_format": "format must be 1",
    "bad_missing_title": "title is required",
    "bad_id_mismatch": "differs from the file name id",
    "bad_ids_empty": "ids is empty",
    "bad_unknown_id": "unknown UC id 'ZZ999'",
    "bad_id_not_in_ids": "is not in ids",
    "bad_cmd": "unknown command 'refund'",
    "bad_args_missing": "args is required",
    "bad_args_not_object": "args must be an object",
    "bad_args_key": "unknown key 'order_id'",
    "bad_args_value": "must be one of navigate",
    "bad_ref_unknown": "no earlier step saves",
    "bad_ref_before_save": "no earlier step saves",
    "bad_ref_malformed": "malformed reference",
    "bad_repeat": "repeat must be a positive integer",
    "bad_expect": "unknown key 'oops' in expect",
    "bad_fixture_missing": "does not exist",
    "bad_fixture_novars": "holds no *GQL-Variables.json",
    "bad_fixture_outside": "inside the repository",
    "bad_deviation_reason": "needs a non-empty reason",
    "bad_deviation_nofixture": "fixtureDeviations without a fixture",
    "bad_menu_item": "menu_item_id 99999",
    "bad_insz": "is not a user of",
    "bad_dup_number": "number 010 is already used",
    "bad_group_name": "group directory must be named",
    "bad_file_name": "file must be named <NNN>_<ID>.json",
    "bad_json": "invalid JSON",
    "bad_dup_key": "duplicate key format",
    "bad_fdm_control_snake": "write 'bufferPct'",
    "bad_mock_save": "a mock command takes no save",
    "bad_sleep": "between 0 and 10000",
    "bad_unknown_step_key": "unknown key 'expects'",
    "bad_unknown_top_key": "unknown top-level key 'note'",
    "bad_empty_steps": "steps is required",
    "bad_dash": "em-dash or en-dash",
    "bad_stray_file": "a file at the scenario root",
    "bad_empty": "no scenario found",
}


def _make_bad_test(tree, phrase):
    def test(self):
        code, out, _err = run_tree(tree)
        self.assertEqual(code, 1, out)
        fail_lines = [line for line in out.splitlines() if "| FAIL |" in line and "| total |" not in line]
        self.assertTrue(fail_lines, out)
        self.assertTrue(any(phrase in line for line in fail_lines), f"{phrase!r} not in {out}")
        self.assertRegex(out.splitlines()[-1], r"^UCSCEN \| total \| FAIL \| [1-9]\d* failed \(\d+ scenarios\)$")

    test.__name__ = f"test_{tree}_fails"
    test.__doc__ = f"{tree}: FAIL naming '{phrase}'"
    return test


class BadTrees(unittest.TestCase):
    pass


for _tree, _phrase in BAD_TREES.items():
    _test = _make_bad_test(_tree, _phrase)
    setattr(BadTrees, _test.__name__, _test)


class Trees(unittest.TestCase):
    def test_every_bad_tree_is_tested(self):
        on_disk = {p.name for p in TREES.iterdir() if p.is_dir() and p.name.startswith("bad_")}
        self.assertEqual(on_disk, set(BAD_TREES))

    def test_good_tree_passes(self):
        code, out, _ = run_tree("good")
        self.assertEqual(code, 0, out)
        self.assertEqual(
            out.splitlines(),
            [
                "UCSCEN | 00_smoke/001_SMOKE.json | PASS | ok",
                "UCSCEN | 10_basic/010_M111.json | PASS | ok",
                "UCSCEN | total | PASS | 0 failed (2 scenarios)",
            ],
        )

    def test_dup_number_fails_only_the_second_file(self):
        _code, out, _ = run_tree("bad_dup_number")
        self.assertIn("UCSCEN | 10_basic/010_M110.json | PASS | ok", out)
        self.assertIn("UCSCEN | 10_basic/010_M111.json | FAIL |", out)
        self.assertIn("UCSCEN | total | FAIL | 1 failed (2 scenarios)", out)

    def test_a_snake_case_mock_key_names_the_camel_case_one(self):
        _code, out, _ = run_tree("bad_fdm_control_snake")
        self.assertIn("'buffer_pct' is ignored by the mock", out)

    def test_two_problems_in_one_file_are_both_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            group = Path(tmp) / "10_basic"
            group.mkdir()
            (group / "010_M110.json").write_text(
                '{"format": 3, "id": "M110", "ids": ["M110"], "title": "", "steps": [{"cmd": "nope", "args": {}}]}\n'
            )
            code, out, _ = run_main("--root", tmp)
        self.assertEqual(code, 1)
        for phrase in ("format must be 1", "title is required", "unknown command 'nope'"):
            self.assertIn(phrase, out)


class RealTree(unittest.TestCase):
    def test_default_run_passes_on_the_committed_scenarios(self):
        code, out, _ = run_main()
        self.assertEqual(code, 0, out)
        self.assertRegex(out.splitlines()[-1], r"^UCSCEN \| total \| PASS \| 0 failed \(\d+ scenarios\)$")

    def test_smoke_scenario_is_byte_identical_to_the_spec(self):
        data = (SMOKE_ROOT / "00_smoke/001_SMOKE.json").read_bytes()
        self.assertEqual(hashlib.sha256(data).hexdigest(), SMOKE_SHA256)
        self.assertTrue(data.endswith(b"\n"))

    def test_the_good_tree_smoke_is_the_committed_smoke(self):
        self.assertEqual(
            (TREES / "good/00_smoke/001_SMOKE.json").read_bytes(),
            (SMOKE_ROOT / "00_smoke/001_SMOKE.json").read_bytes(),
        )

    def test_the_spec_smoke_block_equals_the_file(self):
        text = (REPO / "docs/android/phases/P7.md").read_text(encoding="utf-8")
        block = re.search(r"### 4\.6 The smoke scenario.*?```json\n(.*?)\n```", text, re.S).group(1)
        self.assertEqual(block + "\n", (SMOKE_ROOT / "00_smoke/001_SMOKE.json").read_text(encoding="utf-8"))


class ListIds(unittest.TestCase):
    def test_list_ids_prints_the_covered_ids_sorted(self):
        code, out, _ = run_main("--root", str(TREES / "good"), "--list-ids")
        self.assertEqual(code, 0)
        self.assertEqual(out.splitlines(), ["M110", "M111"])

    def test_list_ids_of_the_repository_tree_is_sorted_and_without_duplicate(self):
        code, out, _ = run_main("--list-ids")
        self.assertEqual(code, 0)
        ids = out.splitlines()
        self.assertEqual(ids, sorted(set(ids)))
        self.assertIn("M110", ids)

    def test_list_ids_leaves_out_an_invalid_scenario(self):
        code, out, _ = run_main("--root", str(TREES / "bad_format"), "--list-ids")
        self.assertEqual(code, 0)
        self.assertEqual(out, "")


class Usage(unittest.TestCase):
    def test_missing_root_is_a_usage_error(self):
        code, out, err = run_main("--root", str(TREES / "no_such_tree"))
        self.assertEqual(code, 2)
        self.assertEqual(out, "")
        self.assertIn("not a directory", err)

    def test_unknown_option_is_a_usage_error(self):
        code, _out, _err = run_main("--frobnicate")
        self.assertEqual(code, 2)

    def test_missing_repo_is_a_usage_error(self):
        code, _out, err = run_main("--repo", str(TREES / "no_such_repo"))
        self.assertEqual(code, 2)
        self.assertIn("repository root", err)

    def test_a_repo_without_the_seed_is_a_usage_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            code, _out, err = run_main("--repo", tmp, "--root", str(TREES / "good"))
        self.assertEqual(code, 2)
        self.assertIn("not found", err)

    def test_command_line_exit_codes(self):
        script = str(HERE / "uc_scenarios_check.py")
        ok = subprocess.run([sys.executable, script], capture_output=True, text=True, cwd=REPO)
        self.assertEqual(ok.returncode, 0, ok.stdout + ok.stderr)
        self.assertTrue(ok.stdout.rstrip().splitlines()[-1].startswith("UCSCEN | total | PASS"))
        bad = subprocess.run(
            [sys.executable, script, "--root", str(TREES / "bad_cmd")], capture_output=True, text=True, cwd=REPO
        )
        self.assertEqual(bad.returncode, 1)
        usage = subprocess.run([sys.executable, script, "--root", "/nonexistent"], capture_output=True, text=True)
        self.assertEqual(usage.returncode, 2)


class RepositoryData(unittest.TestCase):
    def test_known_ids_come_from_uc_matrix(self):
        ids = checker.known_ids(REPO)
        for wanted in (
            "M110", "M112B", "M133", "Q100", "M184", "T1", "T8", "UC201", "UC4109", "UC4119",
            "ROUND1", "ROUND5", "V_FDM_LOCKED", "V_BUFFER_NEAR_FULL", "FAQ_Q205",
        ):
            self.assertIn(wanted, ids)
        self.assertNotIn("SMOKE", ids)

    def test_seed_data(self):
        menu, users = checker.seed_data(REPO)
        self.assertTrue({10006, 12007, 22010, 500001} <= menu)
        self.assertTrue({"94060242576", "00000000097", "84022899837", "99012345667", "00000000029"} <= users)
        self.assertEqual(len(users), 7)
        self.assertEqual(len(menu), 25)

    def test_uc_matrix_is_not_modified_by_loading_it(self):
        before = (REPO / "scripts/uc_matrix.py").read_bytes()
        checker.load_uc_matrix(REPO)
        self.assertEqual(before, (REPO / "scripts/uc_matrix.py").read_bytes())


class Vocabulary(unittest.TestCase):
    def test_counts(self):
        self.assertEqual(len(checker.ENGINE_COMMANDS), 27)
        self.assertEqual(len(checker.MOCK_COMMANDS), 4)
        self.assertEqual(len(set(checker.VOCABULARY)), 31)

    def test_every_engine_command_has_an_argument_spec_and_only_those(self):
        self.assertEqual(set(checker.ENGINE_ARGS), set(checker.ENGINE_COMMANDS))

    def test_uc_md_lists_the_same_vocabulary(self):
        self.assertEqual(checker.check_uc_md(REPO), "")
        found = checker.uc_md_commands(REPO)
        self.assertEqual(found["engine"], [list(checker.ENGINE_COMMANDS)] * 2)
        self.assertEqual(found["mock"], [list(checker.MOCK_COMMANDS)])

    def _repo_with_uc_md(self, tmp, transform):
        docs = Path(tmp) / "docs/android"
        docs.mkdir(parents=True)
        text = (REPO / "docs/android/UC.md").read_text(encoding="utf-8")
        (docs / "UC.md").write_text(transform(text), encoding="utf-8")
        return tmp

    def test_a_missing_row_in_uc_md_is_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = self._repo_with_uc_md(
                tmp, lambda t: "\n".join(l for l in t.splitlines() if not l.startswith("| `get_order` |"))
            )
            problem = checker.check_uc_md(repo)
        self.assertIn("missing get_order", problem)

    def test_an_unknown_row_in_uc_md_is_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = self._repo_with_uc_md(tmp, lambda t: t.replace("| `sleep_ms` |", "| `sleep_ms` | x | y |\n| `wait_ms` |"))
            problem = checker.check_uc_md(repo)
        self.assertIn("unknown wait_ms", problem)

    def test_a_missing_uc_md_is_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertIn("not found", checker.check_uc_md(tmp))

    def test_uc_md_and_the_script_have_no_dash(self):
        banned = (chr(0x2014), chr(0x2013))
        for path in (
            REPO / "docs/android/UC.md",
            HERE / "uc_scenarios_check.py",
            HERE / "test_uc_scenarios_check.py",
        ):
            text = path.read_text(encoding="utf-8")
            for ch in banned:
                self.assertNotIn(ch, text, path.name)
        for path in TREES.rglob("*"):
            if path.is_file():
                text = path.read_text(encoding="utf-8")
                for ch in banned:
                    self.assertNotIn(ch, text, str(path))


class DocumentedCallChain(unittest.TestCase):
    """The UC.md command table is read back against the code it describes."""

    @classmethod
    def setUpClass(cls):
        text = (REPO / "docs/android/UC.md").read_text(encoding="utf-8")
        block = re.search(r"<!-- UC-COMMANDS engine args -->(.*?)<!-- /UC-COMMANDS -->", text, re.S).group(1)
        cls.rows = {}
        for line in block.splitlines():
            cells = [c.strip() for c in line.strip().strip("|").split(" | ")]
            m = re.match(r"^`([a-z_]+)`$", cells[0]) if cells else None
            if m and len(cells) >= 5:
                cls.rows[m.group(1)] = cells
        out = subprocess.run(
            [sys.executable, str(HERE / "check_ipc_wrappers.py"), "--map"],
            capture_output=True, text=True, cwd=REPO, check=True,
        ).stdout
        cls.wrapper_map = {}
        for line in out.splitlines():
            parts = line.split("\t")
            if len(parts) >= 3:
                cls.wrapper_map[parts[0]] = (parts[1], parts[2])

    def test_one_row_per_engine_command(self):
        self.assertEqual(list(self.rows), list(checker.ENGINE_COMMANDS))

    def test_wrapper_and_engine_function_match_check_ipc_wrappers(self):
        for cmd, cells in self.rows.items():
            module_file, fn = self.wrapper_map[cmd]
            self.assertEqual(cells[2], f"`{module_file}.rs`", cmd)
            self.assertTrue(cells[3].startswith("`") and cells[3].endswith("`"), cmd)
            self.assertEqual(cells[3].strip("`").split("::")[-1], fn, cmd)

    def test_facade_method_exists_in_the_named_file(self):
        for cmd, cells in self.rows.items():
            m = re.match(r"^`([a-zA-Z]+)\(.*\)` \(`([a-z_]+\.rs)`\)$", cells[4])
            self.assertIsNotNone(m, cmd)
            method, file = m.groups()
            snake = re.sub(r"(?<!^)([A-Z])", lambda x: "_" + x.group(1).lower(), method)
            self.assertEqual(snake, cmd, cmd)
            source = (REPO / "android/rust/pos-ffi/src/api" / file).read_text(encoding="utf-8")
            self.assertIn(f"pub async fn {cmd}(", source, f"{cmd} in {file}")

    def test_top_level_keys_equal_what_ipc_ts_sends(self):
        sources = (REPO / "ui/src/api/ipc.ts").read_text(encoding="utf-8") + (
            REPO / "ui/src/api/ipc-modifiers.ts"
        ).read_text(encoding="utf-8")
        for cmd, spec in checker.ENGINE_ARGS.items():
            m = re.search(r'invoke\("' + cmd + r'"(,\s*\{)?', sources)
            self.assertIsNotNone(m, cmd)
            sent = set()
            if m.group(1):
                depth, i, entry = 1, m.end(), ""
                entries = []
                while depth:
                    ch = sources[i]
                    if ch in "{[(":
                        depth += 1
                    elif ch in "}])":
                        depth -= 1
                        if depth == 0:
                            break
                    if ch == "," and depth == 1:
                        entries.append(entry)
                        entry = ""
                    else:
                        entry += ch
                    i += 1
                entries.append(entry)
                for e in entries:
                    key = re.match(r"\s*([A-Za-z_]+)", e)
                    if key:
                        sent.add(key.group(1))
            self.assertEqual(sent, set(spec), cmd)


if __name__ == "__main__":
    unittest.main(verbosity=1)
