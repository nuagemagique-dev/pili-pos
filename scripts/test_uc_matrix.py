#!/usr/bin/env python3
"""Unit tests for scripts/uc_matrix.py, on a tiny fake repo in a temp dir.

python3 stdlib only (unittest + tempfile), run with:
    python3 scripts/test_uc_matrix.py
"""

import shutil
import tempfile
import textwrap
import unittest
from pathlib import Path

import uc_matrix as m


class TokenBoundaryTests(unittest.TestCase):
    """token_present() must treat the token as a whole underscore-delimited
    word, never a substring of a longer word."""

    def test_exact_token_matches(self):
        self.assertTrue(m.token_present("m110_sign_sale_response_shape", "m110"))
        self.assertTrue(m.token_present("m110", "m110"))
        self.assertTrue(m.token_present("sale_m110_variant", "m110"))

    def test_m110_does_not_match_m1101(self):
        self.assertFalse(m.token_present("m1101_something_unrelated", "m110"))

    def test_t3_is_a_whole_token_only(self):
        # t3 must match only as a complete underscore-delimited word.
        self.assertTrue(m.token_present("t3_replay", "t3"))
        self.assertTrue(m.token_present("input_report_t3", "t3"))
        self.assertFalse(m.token_present("t30_amount", "t3"))
        self.assertFalse(m.token_present("test3", "t3"))
        self.assertFalse(m.token_present("latest3", "t3"))

    def test_v_buffer_full_does_not_match_v_buffer_near_full(self):
        self.assertFalse(m.token_present("v_buffer_near_full_warning", "v_buffer_full"))
        self.assertTrue(m.token_present("v_buffer_full_error", "v_buffer_full"))


class FakeRepo:
    """Builds a minimal fake repo tree exercising every code path of
    build_report(): a Basic-Set-shaped docs folder plus matching / non
    matching Rust and UI test sources and a scope.toml."""

    def __init__(self, root: Path):
        self.root = root

    def add_basic_set_folder(self, name: str) -> None:
        (self.root / m.BASIC_SET_SUBDIR / name).mkdir(parents=True, exist_ok=True)

    def write(self, relpath: str, content: str) -> None:
        path = self.root / relpath
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(textwrap.dedent(content), encoding="utf-8")


def build_fake_repo(root: Path) -> None:
    repo = FakeRepo(root)

    # Six fake Basic-Set-shaped ids exercising every status:
    #   M110 -> covered by a rust test name
    #   M120 -> covered by a `// covers:` marker on a generically named test
    #   M130 -> covered by a Playwright bracket tag
    #   M140 -> excluded, kind=hors_perimetre
    #   M150 -> excluded, kind=couvert_par, by=M110
    #   M160 -> no test, no exclusion -> MANQUANT
    for name in (
        "M110_signSale",
        "M120_something",
        "M130_something",
        "M140_something",
        "M150_something",
        "M160_something",
    ):
        repo.add_basic_set_folder(name)

    repo.write(
        "crates/fakecrate/tests/sample.rs",
        """\
        fn helper_not_a_test() {}

        #[test]
        fn m110_sign_sale_response_shape() {
            assert!(true);
        }

        // covers: M120
        #[test]
        fn some_generic_fixture_name() {
            assert!(true);
        }

        // A test of a feature outside the certified build (M140).
        #[test]
        fn m140_dev_build_only() {
            assert!(true);
        }

        // A near-miss that must NOT count towards M110 (token boundary).
        #[test]
        fn m1101_unrelated_case() {
            assert!(true);
        }
        """,
    )

    repo.write(
        "ui/e2e/sample.spec.ts",
        """\
        import { test } from '@playwright/test';

        test('[M130] cash sale registers N', async () => {
            expect(true).toBe(true);
        });
        """,
    )

    repo.write(
        "scripts/uc_scope.toml",
        """\
        [M140]
        reason = "Fonctionnalité hors périmètre pour le test."
        kind = "hors_perimetre"

        [M150]
        reason = "Couvert par un autre identifiant pour le test."
        kind = "couvert_par"
        by = "M110"
        """,
    )


class BuildReportTests(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.mkdtemp(prefix="uc_matrix_test_")
        self.root = Path(self.tmpdir)
        build_fake_repo(self.root)
        self.addCleanup(shutil.rmtree, self.tmpdir, ignore_errors=True)

    def _basic_set_rows(self, report):
        for name, rows in report.sections:
            if name == "Basic Set":
                return {row.id: row for row in rows}
        raise AssertionError("Basic Set section not found")

    def test_covered_by_rust_test_name(self):
        report = m.build_report(self.root)
        rows = self._basic_set_rows(report)
        self.assertEqual(rows["M110"].status, "covered")
        self.assertIn("m110_sign_sale_response_shape", rows["M110"].rust_tests)
        # The near-miss fn must not be attributed to M110.
        self.assertNotIn("m1101_unrelated_case", rows["M110"].rust_tests)

    def test_covered_by_covers_marker(self):
        report = m.build_report(self.root)
        rows = self._basic_set_rows(report)
        self.assertEqual(rows["M120"].status, "covered")
        self.assertIn("some_generic_fixture_name", rows["M120"].rust_tests)

    def test_covered_by_playwright_tag(self):
        report = m.build_report(self.root)
        rows = self._basic_set_rows(report)
        self.assertEqual(rows["M130"].status, "covered")
        self.assertTrue(any("[M130]" in t for t in rows["M130"].ui_tests))

    def test_excluded_hors_perimetre(self):
        report = m.build_report(self.root)
        rows = self._basic_set_rows(report)
        self.assertEqual(rows["M140"].status, "hors_perimetre")
        self.assertEqual(rows["M140"].status_label, "Hors périmètre")

    def test_hors_perimetre_wins_over_a_dev_build_test(self):
        report = m.build_report(self.root)
        rows = self._basic_set_rows(report)
        self.assertEqual(rows["M140"].status, "hors_perimetre")
        self.assertIn("m140_dev_build_only", rows["M140"].rust_tests)

    def test_excluded_couvert_par(self):
        report = m.build_report(self.root)
        rows = self._basic_set_rows(report)
        self.assertEqual(rows["M150"].status, "couvert_par")
        self.assertEqual(rows["M150"].couvert_par, "M110")
        self.assertEqual(rows["M150"].status_label, "Couvert par M110")

    def test_missing_id_and_exit_code(self):
        report = m.build_report(self.root)
        rows = self._basic_set_rows(report)
        self.assertEqual(rows["M160"].status, "missing")
        self.assertIn("M160", report.missing_ids)

        exit_code = m.main(["--root", str(self.root)])
        self.assertEqual(exit_code, 1)

    def test_removing_the_missing_folder_clears_it_from_basic_set(self):
        # The fixed Manuel/FAQ vocabularies are always present and have no
        # matching test in this minimal fake repo, so the *global* missing
        # list is never empty here; what this checks is that removing the
        # M160 folder removes M160 specifically from the Basic Set section
        # (i.e. the id list is genuinely re-derived from disk each run, not
        # cached or hardcoded).
        shutil.rmtree(self.root / m.BASIC_SET_SUBDIR / "M160_something")
        report = m.build_report(self.root)
        rows = self._basic_set_rows(report)
        self.assertNotIn("M160", rows)
        self.assertNotIn("M160", report.missing_ids)

    def test_exit_code_for_is_a_pure_function_of_missing_ids(self):
        # exit_code_for() is the single place the pass/fail decision is
        # made; check it in isolation from any repo scanning.
        covered_report = m.Report(sections=[], missing_ids=[])
        self.assertEqual(m.exit_code_for(covered_report), 0)

        missing_report = m.Report(sections=[], missing_ids=["M999"])
        self.assertEqual(m.exit_code_for(missing_report), 1)

    def test_write_produces_stable_output(self):
        exit_code = m.main(["--root", str(self.root), "--write"])
        self.assertEqual(exit_code, 1)
        out_path = self.root / m.OUTPUT_MD
        self.assertTrue(out_path.is_file())
        first = out_path.read_text(encoding="utf-8")

        exit_code_2 = m.main(["--root", str(self.root), "--write"])
        self.assertEqual(exit_code_2, 1)
        second = out_path.read_text(encoding="utf-8")

        self.assertEqual(first, second, "rerunning --write without changes must be idempotent")
        self.assertIn("M160", second)
        self.assertIn("MANQUANT", second)


class ScopeValidationTests(unittest.TestCase):
    def setUp(self):
        self.tmpdir = tempfile.mkdtemp(prefix="uc_matrix_scope_test_")
        self.root = Path(self.tmpdir)
        self.addCleanup(shutil.rmtree, self.tmpdir, ignore_errors=True)

    def _write_scope(self, content: str) -> Path:
        path = self.root / "scripts" / "uc_scope.toml"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(textwrap.dedent(content), encoding="utf-8")
        return path

    def test_missing_scope_file_is_empty_dict(self):
        self.assertEqual(m.load_scope(self.root / "scripts" / "uc_scope.toml"), {})

    def test_invalid_kind_raises(self):
        path = self._write_scope(
            """\
            [M999]
            reason = "test"
            kind = "not_a_real_kind"
            """
        )
        with self.assertRaises(ValueError):
            m.load_scope(path)

    def test_couvert_par_without_by_raises(self):
        path = self._write_scope(
            """\
            [M999]
            reason = "test"
            kind = "couvert_par"
            """
        )
        with self.assertRaises(ValueError):
            m.load_scope(path)


if __name__ == "__main__":
    unittest.main()
