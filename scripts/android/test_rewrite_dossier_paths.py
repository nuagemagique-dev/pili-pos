#!/usr/bin/env python3
"""Unit tests for rewrite_dossier_paths.py, run on a temporary fake repo."""

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

TOOL = Path(__file__).resolve().parent / "rewrite_dossier_paths.py"

MAIN_RS = """\
fn main() {
    let builder = builder.invoke_handler(tauri::generate_handler![
        // Reports
        commands::report::generate_x_report,
        commands::report::generate_z_report,
        commands::checkout::checkout,
        ipc::order::create_order,
    ]);
}
"""

IN_SCOPE = "docs/SUBMISSION_ARTIFACTS/DOC_FR.md"


class Repo:
    """A throwaway repository tree."""

    def __init__(self, tmp):
        self.root = Path(tmp)
        self.write("src-tauri/src/main.rs", MAIN_RS)

    def write(self, rel, text=""):
        p = self.root / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text, encoding="utf-8")

    def read(self, rel):
        return (self.root / rel).read_text(encoding="utf-8")

    def touch(self, *rels):
        for rel in rels:
            self.write(rel, "")

    def run(self, *args):
        r = subprocess.run(
            [sys.executable, str(TOOL), "--root", str(self.root), *args],
            capture_output=True,
            text=True,
        )
        return r.returncode, r.stdout + r.stderr


class RewriteTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.repo = Repo(self._tmp.name)

    def doc(self, text):
        self.repo.write(IN_SCOPE, text)

    def test_path_boundaries(self):
        # Neighbouring names are not the directories.
        text = (
            "`src-tauri/tests2/a.rs` `src-tauri/src/commands.rs` "
            "`src-tauri/src/commands2/x.rs` `my-src-tauri/tests/a.rs` "
            "`src-tauri/src/state.rs.bak`\n"
        )
        self.doc(text)
        code, out = self.repo.run("--plan")
        self.assertEqual(code, 0)
        self.assertTrue(out.rstrip().endswith("total: 0 references in 0 files"), out)
        # The directory itself, with a sentence full stop or a slash, matches.
        self.doc("see src-tauri/tests. And src-tauri/src/commands/ too\n")
        _, out = self.repo.run("--plan")
        self.assertTrue(out.rstrip().endswith("total: 2 references in 1 files"), out)

    def test_wrapper_vs_engine_function(self):
        self.repo.touch(
            "src-tauri/src/ipc/report.rs",
            "crates/pos-engine/src/commands/report.rs",
        )
        self.doc(
            "wrapper `src-tauri/src/commands/report.rs::generate_z_report` and "
            "engine `src-tauri/src/commands/report.rs::do_generate_z_report`\n"
        )
        code, _ = self.repo.run("--apply")
        self.assertEqual(code, 0)
        self.assertEqual(
            self.repo.read(IN_SCOPE),
            "wrapper `src-tauri/src/ipc/report.rs::generate_z_report` and "
            "engine `crates/pos-engine/src/commands/report.rs::do_generate_z_report`\n",
        )

    def test_command_of_another_module_is_not_a_wrapper(self):
        # checkout::checkout is a command, report::checkout is not.
        self.repo.touch("crates/pos-engine/src/commands/report.rs")
        self.doc("`src-tauri/src/commands/report.rs::checkout`\n")
        self.repo.run("--apply")
        self.assertIn("crates/pos-engine/src/commands/report.rs::checkout", self.repo.read(IN_SCOPE))

    def test_command_names_accept_ipc_entries(self):
        self.repo.touch("src-tauri/src/ipc/order.rs")
        self.doc("`src-tauri/src/commands/order.rs::create_order`\n")
        self.repo.run("--apply")
        self.assertEqual(self.repo.read(IN_SCOPE), "`src-tauri/src/ipc/order.rs::create_order`\n")

    def test_install_functions(self):
        self.doc(
            "`src-tauri/src/cli.rs::init_db` `src-tauri/src/cli.rs::apply_seed` "
            "`src-tauri/src/cli.rs::check_config` `src-tauri/src/cli.rs::parse`\n"
        )
        code, out = self.repo.run("--plan")
        self.assertTrue(out.rstrip().endswith("total: 3 references in 1 files"), out)
        self.repo.touch("crates/pos-engine/src/install.rs")
        self.repo.run("--apply")
        self.assertEqual(
            self.repo.read(IN_SCOPE),
            "`crates/pos-engine/src/install.rs::init_db` "
            "`crates/pos-engine/src/install.rs::apply_seed` "
            "`crates/pos-engine/src/install.rs::check_config` "
            "`src-tauri/src/cli.rs::parse`\n",
        )

    def test_state_and_tests_rules(self):
        self.repo.touch(
            "crates/pos-engine/src/state.rs",
            "crates/pos-engine-testkit/src/lib.rs",
            "crates/pos-engine-testkit/src/fdm_drop.rs",
            "crates/pos-engine/tests/spf_e2e_basic.rs",
        )
        self.repo.touch("crates/pos-engine-testkit/src/x.rs")
        self.doc(
            "`src-tauri/src/state.rs`; `src-tauri/tests/common/mod.rs`; "
            "`src-tauri/tests/common/fdm_drop.rs`; `src-tauri/tests/common`; "
            "`src-tauri/tests/spf_e2e_basic.rs::m110_sale`; `src-tauri/tests`\n"
        )
        # Directory targets exist because their files do.
        self.repo.run("--apply")
        self.assertEqual(
            self.repo.read(IN_SCOPE),
            "`crates/pos-engine/src/state.rs`; `crates/pos-engine-testkit/src/lib.rs`; "
            "`crates/pos-engine-testkit/src/fdm_drop.rs`; `crates/pos-engine-testkit/src`; "
            "`crates/pos-engine/tests/spf_e2e_basic.rs::m110_sale`; `crates/pos-engine/tests`\n",
        )

    def test_target_missing_is_not_applied(self):
        text = (
            "`src-tauri/src/commands/report.rs` `src-tauri/tests/a.rs` "
            "`src-tauri/src/commands/report.rs::generate_z_report`\n"
        )
        self.doc(text)
        self.repo.touch("crates/pos-engine/src/commands/other.rs")
        code, out = self.repo.run("--apply")
        self.assertEqual(code, 0)
        self.assertEqual(self.repo.read(IN_SCOPE), text)
        self.assertIn("applied: 0 references", out)

    def test_apply_only_moved_targets(self):
        self.repo.touch("crates/pos-engine/src/commands/report.rs")
        self.doc("`src-tauri/src/commands/report.rs` `src-tauri/src/commands/refund.rs`\n")
        self.repo.run("--apply")
        self.assertEqual(
            self.repo.read(IN_SCOPE),
            "`crates/pos-engine/src/commands/report.rs` `src-tauri/src/commands/refund.rs`\n",
        )

    def test_apply_is_idempotent(self):
        self.repo.touch(
            "crates/pos-engine/src/commands/report.rs",
            "crates/pos-engine/src/state.rs",
            "src-tauri/src/ipc/order.rs",
        )
        self.doc(
            "`src-tauri/src/commands/report.rs::do_x` `src-tauri/src/state.rs` "
            "`src-tauri/src/commands/order.rs::create_order`\n"
        )
        self.repo.run("--apply")
        once = self.repo.read(IN_SCOPE)
        code, out = self.repo.run("--apply")
        self.assertEqual(code, 0)
        self.assertEqual(self.repo.read(IN_SCOPE), once)
        self.assertIn("applied: 0 references", out)

    def test_check_exit_codes(self):
        self.doc("`src-tauri/src/state.rs`\n")
        # Target not there: nothing to rewrite yet.
        self.assertEqual(self.repo.run("--check")[0], 0)
        # Code moved, docs not rewritten.
        self.repo.touch("crates/pos-engine/src/state.rs")
        self.assertEqual(self.repo.run("--check")[0], 1)
        # Rewritten.
        self.repo.run("--apply")
        self.assertEqual(self.repo.run("--check")[0], 0)

    def test_strict_exit_codes(self):
        self.doc("`src-tauri/src/state.rs`\n")
        self.assertEqual(self.repo.run("--strict")[0], 1)
        self.repo.touch("crates/pos-engine/src/state.rs")
        self.repo.run("--apply")
        self.assertEqual(self.repo.run("--strict")[0], 0)

    def test_item_and_line_suffixes_kept(self):
        self.repo.touch(
            "crates/pos-engine/src/commands/report.rs",
            "crates/pos-engine/tests/spf_e2e_basic.rs",
        )
        self.doc(
            "`src-tauri/src/commands/report.rs:120` `src-tauri/src/commands/report.rs:12-30` "
            "`src-tauri/tests/spf_e2e_basic.rs::m171_sale_in_training` "
            "`src-tauri/src/commands/report.rs::{a,b}`\n"
        )
        code, out = self.repo.run("--plan")
        self.assertIn("manual review", out)
        self.assertIn("report.rs:<line>", out)
        code, out = self.repo.run("--apply")
        self.assertIn("line number to review by hand", out)
        self.assertEqual(
            self.repo.read(IN_SCOPE),
            "`crates/pos-engine/src/commands/report.rs:120` "
            "`crates/pos-engine/src/commands/report.rs:12-30` "
            "`crates/pos-engine/tests/spf_e2e_basic.rs::m171_sale_in_training` "
            "`crates/pos-engine/src/commands/report.rs::{a,b}`\n",
        )

    def test_brace_list_of_commands_is_a_wrapper_reference(self):
        self.repo.touch(
            "src-tauri/src/ipc/report.rs",
            "crates/pos-engine/src/commands/report.rs",
        )
        self.doc(
            "`src-tauri/src/commands/report.rs::{generate_x_report,generate_z_report}` "
            "`src-tauri/src/commands/report.rs::{ generate_x_report , generate_z_report }`\n"
        )
        code, out = self.repo.run("--plan")
        self.assertIn("rule a: 2 matches", out)
        self.assertNotIn("manual review", out)
        code, out = self.repo.run("--apply")
        self.assertEqual(code, 0)
        self.assertNotIn("review by hand", out)
        self.assertEqual(
            self.repo.read(IN_SCOPE),
            "`src-tauri/src/ipc/report.rs::{generate_x_report,generate_z_report}` "
            "`src-tauri/src/ipc/report.rs::{ generate_x_report , generate_z_report }`\n",
        )

    def test_brace_list_mixing_commands_and_engine_names_is_reviewed(self):
        self.repo.touch(
            "src-tauri/src/ipc/report.rs",
            "crates/pos-engine/src/commands/report.rs",
        )
        text = "`src-tauri/src/commands/report.rs::{generate_z_report, do_generate_z_report}`\n"
        self.doc(text)
        code, out = self.repo.run("--plan")
        self.assertIn("rule c: 1 matches", out)
        self.assertIn("manual review", out)
        code, out = self.repo.run("--apply")
        self.assertEqual(code, 0)
        self.assertIn("mixed wrapper and engine names to review by hand", out)
        self.assertEqual(
            self.repo.read(IN_SCOPE),
            "`crates/pos-engine/src/commands/report.rs::{generate_z_report, do_generate_z_report}`\n",
        )

    def test_brace_list_of_a_command_of_another_module_is_not_a_wrapper(self):
        self.repo.touch("crates/pos-engine/src/commands/report.rs")
        self.doc("`src-tauri/src/commands/report.rs::{checkout}`\n")
        self.repo.run("--apply")
        self.assertEqual(
            self.repo.read(IN_SCOPE),
            "`crates/pos-engine/src/commands/report.rs::{checkout}`\n",
        )

    def test_out_of_scope_files_untouched(self):
        self.repo.touch(
            "crates/pos-engine/src/state.rs",
            "crates/pos-engine/tests/a.rs",
        )
        text = "`src-tauri/src/state.rs` `src-tauri/tests/a.rs` `src-tauri/tests`\n"
        out_of_scope = [
            "docs/android/PLAN.md",
            "docs/internal/NOTES.md",
            "docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md",
            "docs/legislation/AM.md",
            "docs/SUBMISSION_ARTIFACTS/evidence/data.json",
            "docs/SUBMISSION_ARTIFACTS/MANIFEST.txt",
            "scripts/notes.md",
            "src-tauri/README.md",
            "deploy/run.sh",
        ]
        for rel in out_of_scope:
            self.repo.write(rel, text)
        in_scope = [
            "README.md",
            "CLAUDE.md",
            "deploy/REMOTE-OPS.md",
            "docs/SUBMISSION_ARTIFACTS/evidence/uc/README.md",
            IN_SCOPE,
        ]
        for rel in in_scope:
            self.repo.write(rel, text)
        code, out = self.repo.run("--plan")
        self.assertTrue(out.rstrip().endswith("total: 15 references in 5 files"), out)
        self.assertEqual(self.repo.run("--unmapped")[0], 0)
        self.repo.run("--apply")
        for rel in out_of_scope:
            self.assertEqual(self.repo.read(rel), text, rel)
        for rel in in_scope:
            self.assertNotEqual(self.repo.read(rel), text, rel)
            self.assertNotIn("src-tauri", self.repo.read(rel), rel)

    def test_unmapped(self):
        self.doc("`src-tauri/src/cli.rs::init_db_extra` and `x/src-tauri/tests/a.rs`\n")
        code, out = self.repo.run("--unmapped")
        self.assertEqual(code, 1, out)
        self.assertIn("init_db_extra", out)
        self.doc("`src-tauri/src/commands/report.rs` `src-tauri/src/cli.rs::init_db`\n")
        self.assertEqual(self.repo.run("--unmapped")[0], 0)

    def test_crlf_and_other_bytes_preserved(self):
        self.repo.touch("crates/pos-engine/src/state.rs")
        self.repo.root.joinpath(IN_SCOPE).parent.mkdir(parents=True, exist_ok=True)
        (self.repo.root / IN_SCOPE).write_bytes(
            "a é\r\n`src-tauri/src/state.rs`\r\n".encode("utf-8")
        )
        self.repo.run("--apply")
        self.assertEqual(
            (self.repo.root / IN_SCOPE).read_bytes(),
            "a é\r\n`crates/pos-engine/src/state.rs`\r\n".encode("utf-8"),
        )

    def test_default_mode_is_plan(self):
        self.doc("`src-tauri/src/state.rs`\n")
        code, out = self.repo.run()
        self.assertEqual(code, 0)
        self.assertTrue(out.rstrip().endswith("total: 1 references in 1 files"), out)


if __name__ == "__main__":
    unittest.main()
