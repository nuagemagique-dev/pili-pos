#!/usr/bin/env python3
"""Tests of scripts/android/uc_matrix_android.py (card P7-T9).

Run: python3 scripts/android/test_uc_matrix_android.py

The fake repository scripts/android/testdata/uc_matrix_android/base holds a few official folders,
Rust and UI test sources, a Linux scope, an Android scope, three scenarios and a results file.
Each test copies it to a temporary directory and changes what it needs. Its fixed vocabularies
(certification manual, FAQ) are replaced by two ids, and the required list by a short one, so the
fake repository stays small; two tests run the real repository (the 47 required ids, the
current state of the branch).
"""

import contextlib
import io
import json
import re
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
BASE = HERE / "testdata" / "uc_matrix_android" / "base"

sys.path.insert(0, str(HERE))
import uc_matrix_android as uca  # noqa: E402

# Required ids of the fake repository: M110 has a PASS scenario, M111 a scenario without result,
# Q100 no scenario, V_FDM_LOCKED (a UI-only id on Linux) no scenario.
FAKE_REQUIRED = ["M110", "M111", "Q100", "V_FDM_LOCKED"]


class Fake:
    """A temporary copy of the fake repository."""

    def __init__(self, case):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name) / "repo"
        shutil.copytree(BASE, self.root)
        case.addCleanup(self.tmp.cleanup)

    def path(self, rel):
        return self.root / rel

    def write(self, rel, text):
        p = self.path(rel)
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text, encoding="utf-8")

    def scenario(self, rel, ids, group_file=None):
        self.write(
            f"{uca.SCENARIOS_DIR}/{rel}",
            json.dumps({"format": 1, "id": ids[0] if ids else "SMOKE", "ids": ids, "steps": []}, indent=2) + "\n",
        )

    def results(self, entries):
        scenarios = [
            {"group": g, "scenario": s, "ids": [], "checks": {}, "verdict": v} for g, s, v in entries
        ]
        self.write(uca.RESULTS_JSON, json.dumps({"format": 1, "scenarios": scenarios}, indent=2) + "\n")

    def ucm(self):
        """The Linux module with the vocabularies of the fake repository."""
        ucm = uca.load_ucm(self.root)
        ucm.MANUAL_ITEMS = [ucm.Item("V_FDM_LOCKED", "FDM verrouillé"), ucm.Item("V_UPDATE_URLS_FAILED", "Échec URLs")]
        ucm.FAQ_ITEMS = []
        return ucm

    def run(self, *argv, required=FAKE_REQUIRED):
        """(exit code, stdout lines)."""
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = uca.main(["--repo", str(self.root), *argv], required=required, ucm=self.ucm())
        return code, out.getvalue().splitlines()

    def matrix(self, required=FAKE_REQUIRED):
        return uca.build_matrix(self.root, required, self.ucm())

    def row(self, id_, required=FAKE_REQUIRED):
        return next(r for r in self.matrix(required).rows() if r.id == id_)


def check_line(lines, name):
    for ln in lines:
        if ln.startswith(f"UCMATRIX | {name} |"):
            return ln
    raise AssertionError(f"no line for check {name}: {lines}")


class RequiredList(unittest.TestCase):
    def test_required_list_has_47_distinct_ids(self):
        self.assertEqual(len(uca.REQUIRED), 47)
        self.assertEqual(len(set(uca.REQUIRED)), 47)

    def test_required_list_names_the_p7d9_ids(self):
        for id_ in ("M110", "M171B", "T8", "UC4119", "ROUND5", "V_FDM_LOCKED", "V_UNAUTHORIZED", "FAQ_Q205"):
            self.assertIn(id_, uca.REQUIRED)

    def test_every_required_id_is_known_to_the_real_linux_matrix(self):
        matrix = uca.build_matrix(REPO)
        self.assertEqual(matrix.unknown_required, [])
        self.assertEqual(len([r for r in matrix.rows() if r.required]), 47)

    def test_real_repository_passes_in_development_mode(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = uca.main(["--repo", str(REPO), "--check", "--allow-pending"])
        self.assertEqual(code, 0, out.getvalue())
        self.assertIn("UCMATRIX | total | PASS", out.getvalue())


class Statuses(unittest.TestCase):
    def test_scenario_with_pass_result_is_covered(self):
        fake = Fake(self)
        row = fake.row("M110")
        self.assertEqual(row.status, uca.ST_SCENARIO)
        self.assertEqual(row.status_label, "Couvert (scénario Android)")
        self.assertEqual(row.result_label, "PASS")
        self.assertEqual([s.rel for s in row.scenarios], ["10_basic/010_M110.json"])

    def test_scenario_without_result_is_pending(self):
        row = Fake(self).row("M111")
        self.assertEqual(row.status, uca.ST_PENDING)
        self.assertEqual(row.status_label, "En attente")
        self.assertEqual(row.result_label, "Aucun résultat")

    def test_scenario_with_fail_result_is_pending(self):
        fake = Fake(self)
        fake.results([("10_basic", "010_M110", "FAIL")])
        row = fake.row("M110")
        self.assertEqual(row.status, uca.ST_PENDING)
        self.assertEqual(row.result_label, "FAIL")

    def test_required_id_without_scenario_is_pending(self):
        fake = Fake(self)
        self.assertEqual(fake.row("Q100").status, uca.ST_PENDING)
        self.assertEqual(fake.row("V_FDM_LOCKED").status, uca.ST_PENDING)

    def test_not_required_id_with_rust_tests_is_covered_by_the_engine_tests(self):
        row = Fake(self).row("M184")
        self.assertEqual(row.status, uca.ST_ENGINE)
        self.assertEqual(row.status_label, "Couvert (tests moteur exécutés sur l'appareil)")
        self.assertEqual(row.rust_tests, ["m184_combined_z"])

    def test_linux_exclusions_are_mirrored(self):
        fake = Fake(self)
        self.assertEqual(fake.row("M122").status_label, "Hors périmètre")
        fake.write(
            "scripts/uc_scope.toml",
            '[M122]\nreason = "r"\nkind = "non_applicable"\n\n[M181]\nreason = "r"\nkind = "couvert_par"\nby = "M184"\n',
        )
        self.assertEqual(fake.row("M122").status_label, "Non applicable")
        self.assertEqual(fake.row("M181").status_label, "Couvert par M184")

    def test_android_exclusion_marks_a_ui_only_id(self):
        row = Fake(self).row("V_UPDATE_URLS_FAILED")
        self.assertEqual(row.status, uca.ST_EXCLU)
        self.assertEqual(row.status_label, "Exclu sur Android")

    def test_not_required_id_with_a_pass_scenario_is_covered_by_it(self):
        fake = Fake(self)
        fake.scenario("30_reports/010_M184.json", ["M184", "M181"])
        fake.results([("10_basic", "010_M110", "PASS"), ("30_reports", "010_M184", "PASS")])
        for id_ in ("M184", "M181"):
            self.assertEqual(fake.row(id_).status, uca.ST_SCENARIO, id_)

    def test_required_id_listed_in_a_linux_exclusion_still_needs_a_scenario(self):
        fake = Fake(self)
        self.assertEqual(fake.row("M181", ["M181"]).status, uca.ST_PENDING)
        fake.scenario("30_reports/010_M181.json", ["M181"])
        fake.results([("30_reports", "010_M181", "PASS")])
        self.assertEqual(fake.row("M181", ["M181"]).status, uca.ST_SCENARIO)

    def test_id_with_nothing_is_missing(self):
        fake = Fake(self)
        fake.write(uca.SCOPE_ANDROID, "")
        row = fake.row("V_UPDATE_URLS_FAILED")
        self.assertEqual(row.status, uca.ST_MISSING)
        self.assertEqual(row.status_label, "MANQUANT")


class Checks(unittest.TestCase):
    def test_pending_ids_fail_the_release_mode_and_are_listed(self):
        code, lines = Fake(self).run("--check")
        self.assertEqual(code, 1)
        pending = check_line(lines, "pending")
        self.assertIn("FAIL", pending)
        for id_ in ("M111", "Q100", "V_FDM_LOCKED"):
            self.assertIn(id_, pending)
        self.assertNotIn("M110", pending)
        self.assertIn("UCMATRIX | total | FAIL", lines[-1])

    def test_pending_ids_are_accepted_in_development_mode(self):
        code, lines = Fake(self).run("--check", "--allow-pending")
        self.assertEqual(code, 0, lines)
        self.assertIn("PASS", check_line(lines, "pending"))
        self.assertIn("not compared", check_line(lines, "doc"))
        self.assertIn("UCMATRIX | total | PASS", lines[-1])

    def test_missing_id_fails_even_with_allow_pending(self):
        fake = Fake(self)
        fake.write(uca.SCOPE_ANDROID, "")
        code, lines = fake.run("--check", "--allow-pending")
        self.assertEqual(code, 1)
        self.assertIn("V_UPDATE_URLS_FAILED", check_line(lines, "missing"))
        self.assertIn("FAIL", check_line(lines, "missing"))

    def test_stale_exclusion_for_an_id_with_rust_tests_fails(self):
        fake = Fake(self)
        fake.write(uca.SCOPE_ANDROID, fake.path(uca.SCOPE_ANDROID).read_text() + '\n[M184]\nreason = "r"\n')
        code, lines = fake.run("--check", "--allow-pending")
        self.assertEqual(code, 1)
        line = check_line(lines, "exclusions")
        self.assertIn("FAIL", line)
        self.assertIn("M184 (is covered by Rust tests)", line)

    def test_stale_exclusion_for_an_id_with_a_scenario_fails(self):
        fake = Fake(self)
        fake.scenario("50_rounding/010_V_UPDATE_URLS_FAILED.json", ["V_UPDATE_URLS_FAILED"])
        code, lines = fake.run("--check", "--allow-pending")
        self.assertEqual(code, 1)
        self.assertIn("V_UPDATE_URLS_FAILED (has a scenario)", check_line(lines, "exclusions"))

    def test_exclusion_of_an_unknown_id_fails(self):
        fake = Fake(self)
        fake.write(uca.SCOPE_ANDROID, fake.path(uca.SCOPE_ANDROID).read_text() + '\n[UC9999]\nreason = "r"\n')
        code, lines = fake.run("--check", "--allow-pending")
        self.assertEqual(code, 1)
        self.assertIn("UC9999 (unknown to the Linux matrix)", check_line(lines, "exclusions"))

    def test_exclusion_of_a_required_id_is_an_input_error(self):
        fake = Fake(self)
        fake.write(uca.SCOPE_ANDROID, '[M110]\nreason = "r"\n')
        code, lines = fake.run("--check", "--allow-pending")
        self.assertEqual(code, 1)
        self.assertIn("required id", check_line(lines, "inputs"))

    def test_exclusion_without_a_reason_is_an_input_error(self):
        fake = Fake(self)
        fake.write(uca.SCOPE_ANDROID, '[V_UPDATE_URLS_FAILED]\nkind = "exclu_android"\n')
        code, lines = fake.run("--check", "--allow-pending")
        self.assertEqual(code, 1)
        self.assertIn("missing a reason", check_line(lines, "inputs"))

    def test_unreadable_results_are_an_input_error(self):
        fake = Fake(self)
        fake.write(uca.RESULTS_JSON, "{not json")
        code, lines = fake.run("--check", "--allow-pending")
        self.assertEqual(code, 1)
        self.assertIn("results", check_line(lines, "inputs"))

    def test_unreadable_scenario_is_an_input_error(self):
        fake = Fake(self)
        fake.write(f"{uca.SCENARIOS_DIR}/10_basic/030_bad.json", "[1, 2")
        code, lines = fake.run("--check", "--allow-pending")
        self.assertEqual(code, 1)
        self.assertIn("030_bad.json", check_line(lines, "inputs"))

    def test_required_id_unknown_to_the_linux_matrix_fails(self):
        code, lines = Fake(self).run("--check", "--allow-pending", required=FAKE_REQUIRED + ["UC9999"])
        self.assertEqual(code, 1)
        self.assertIn("UC9999", check_line(lines, "required"))

    def test_absent_results_file_leaves_scenarios_without_result(self):
        fake = Fake(self)
        fake.path(uca.RESULTS_JSON).unlink()
        self.assertEqual(fake.row("M110").status, uca.ST_PENDING)

    def test_list_missing_prints_missing_and_pending_ids(self):
        fake = Fake(self)
        fake.write(uca.SCOPE_ANDROID, "")
        code, lines = fake.run("--list-missing")
        self.assertEqual(code, 1)
        self.assertIn("MANQUANT V_UPDATE_URLS_FAILED", lines)
        self.assertIn("EN ATTENTE Q100", lines)

    def test_a_mode_is_required(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as ctx:
            uca.main(["--repo", str(BASE)])
        self.assertEqual(ctx.exception.code, 2)

    def test_allow_pending_needs_check(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as ctx:
            uca.main(["--repo", str(BASE), "--write", "--allow-pending"])
        self.assertEqual(ctx.exception.code, 2)


class Document(unittest.TestCase):
    def test_write_creates_a_french_document_with_summary_and_sections(self):
        fake = Fake(self)
        code, lines = fake.run("--write")
        self.assertEqual(code, 0, lines)
        text = fake.path(uca.OUTPUT_MD).read_text(encoding="utf-8")
        for needle in (
            "# Matrice de traçabilité des tests SPF, version Android",
            "Date de génération : ",
            "## Synthèse",
            "## Basic Set",
            "## Reports",
            "## Advanced Set",
            "## Manuel",
            "| Id | Libellé | Scénario Android | Résultat Android | Tests moteur | Statut |",
            "| Couvert (scénario Android) | 1 |",
            "| En attente | 3 |",
            "| Exclu sur Android | 1 |",
            "## Exclusions propres à Android",
        ):
            self.assertIn(needle, text)
        self.assertRegex(text, r"\| M110 \| .* \| `10_basic/010_M110.json` \| PASS \| .* \| Couvert \(scénario Android\) \|")
        self.assertRegex(text, r"Date de génération : \d{4}-\d{2}-\d{2}")

    def test_generated_document_has_no_dash_and_no_brand(self):
        fake = Fake(self)
        fake.run("--write")
        text = fake.path(uca.OUTPUT_MD).read_text(encoding="utf-8")
        self.assertNotIn(chr(0x2014), text)
        self.assertNotIn(chr(0x2013), text)
        self.assertIsNone(re.search(r"tokyo|sushi|pili", text, re.I))

    def test_committed_real_document_lists_the_47_required_ids(self):
        text = (REPO / uca.OUTPUT_MD).read_text(encoding="utf-8")
        for id_ in uca.REQUIRED:
            self.assertRegex(text, rf"(?m)^\| {re.escape(id_)} \|", f"{id_} is not in the document")
        self.assertNotIn(chr(0x2014), text)
        self.assertNotIn(chr(0x2013), text)

    def _complete_fake(self):
        """A fake repository with nothing pending: only M110 is required."""
        fake = Fake(self)
        scope = fake.path(uca.SCOPE_ANDROID)
        scope.write_text(scope.read_text() + '\n[V_FDM_LOCKED]\nreason = "r"\n', encoding="utf-8")
        code, _ = fake.run("--write", required=["M110"])
        self.assertEqual(code, 0)
        return fake

    def test_check_passes_on_a_complete_up_to_date_document(self):
        fake = self._complete_fake()
        code, lines = fake.run("--check", required=["M110"])
        self.assertEqual(code, 0, lines)
        self.assertIn("up to date", check_line(lines, "doc"))
        self.assertIn("UCMATRIX | total | PASS", lines[-1])

    def test_check_ignores_the_date_line(self):
        fake = self._complete_fake()
        path = fake.path(uca.OUTPUT_MD)
        path.write_text(re.sub(r"(Date de génération : ).*", r"\g<1>2001-01-01", path.read_text(encoding="utf-8")), encoding="utf-8")
        code, lines = fake.run("--check", required=["M110"])
        self.assertEqual(code, 0, lines)

    def test_check_detects_drift_of_the_document(self):
        fake = self._complete_fake()
        path = fake.path(uca.OUTPUT_MD)
        path.write_text(path.read_text(encoding="utf-8").replace("| En attente | 0 |", "| En attente | 9 |"), encoding="utf-8")
        code, lines = fake.run("--check", required=["M110"])
        self.assertEqual(code, 1)
        self.assertIn("differs", check_line(lines, "doc"))

    def test_check_detects_a_scenario_added_after_the_document(self):
        fake = self._complete_fake()
        fake.scenario("10_basic/030_Q100.json", ["Q100"])
        code, lines = fake.run("--check", required=["M110"])
        self.assertEqual(code, 1)
        self.assertIn("differs", check_line(lines, "doc"))

    def test_check_fails_when_the_document_is_absent(self):
        fake = Fake(self)
        code, lines = fake.run("--check", required=["M110"])
        self.assertEqual(code, 1)
        self.assertIn("absent", check_line(lines, "doc"))


if __name__ == "__main__":
    unittest.main(verbosity=1)
