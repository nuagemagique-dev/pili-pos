#!/usr/bin/env python3
"""test_check_dossier.py - unit tests for scripts/check_dossier.py.

Builds a small fake repo under a temp directory (crates/, src-tauri/,
deploy/, docs/SUBMISSION_ARTIFACTS/, scripts/) and runs each rule against
it in isolation with --rules, checking both a passing and a failing case
per rule. python3 stdlib only (unittest), no network, no external deps.

Run with:
    python3 scripts/test_check_dossier.py
"""

import importlib.util
import io
import os
import sys
import time
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

SCRIPT_PATH = Path(__file__).resolve().parent / "check_dossier.py"

# Built from its code point, never typed literally: this test file must
# never contain the character the "em-dash" rule is checking for.
EM_DASH = chr(0x2014)


def load_module():
    spec = importlib.util.spec_from_file_location("check_dossier_under_test", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


class CheckDossierTestCase(unittest.TestCase):
    """One shared fake repo, exercised one rule at a time via --rules."""

    @classmethod
    def setUpClass(cls):
        import tempfile

        cls._tmpdir = tempfile.TemporaryDirectory()
        cls.root = Path(cls._tmpdir.name)
        cls.build_fake_repo(cls.root)

        cls.cd = load_module()
        cls.cd.ROOT = cls.root
        cls.cd.DOSSIER_DIR = cls.root / "docs" / "SUBMISSION_ARTIFACTS"
        cls.cd.ALLOWLIST_PATH = cls.root / "scripts" / "check_dossier_allowlist.txt"

    @classmethod
    def tearDownClass(cls):
        cls._tmpdir.cleanup()

    @staticmethod
    def build_fake_repo(root: Path) -> None:
        # --- Cargo workspace: one real crate + the tauri app -------------
        write(
            root / "Cargo.toml",
            "[workspace]\n"
            'members = ["crates/pos-fiscal", "src-tauri"]\n\n'
            "[workspace.dependencies]\n"
            'chrono = "0.4"\n',
        )
        write(
            root / "crates" / "pos-fiscal" / "Cargo.toml",
            '[package]\nname = "pos-fiscal"\n\n[dependencies]\nchrono = { workspace = true }\n',
        )
        write(
            root / "crates" / "pos-fiscal" / "src" / "signature.rs",
            "pub fn verify_short_signature(sig: &str) -> bool {\n"
            "    !sig.is_empty()\n"
            "}\n\n"
            "pub enum SignatureError {\n"
            "    Invalid,\n"
            "    Expired,\n"
            "}\n",
        )
        write(
            root / "crates" / "pos-fiscal" / "tests" / "full_service.rs",
            "#[test]\nfn end_to_end_ok() {\n    assert!(true);\n}\n",
        )
        write(
            root / "src-tauri" / "Cargo.toml",
            '[package]\nname = "pili-pos"\n\n[dependencies]\n'
            'pos-fiscal = { path = "../crates/pos-fiscal" }\n',
        )
        write(
            root / "src-tauri" / "src" / "commands" / "checkout.rs",
            "pub fn do_checkout() -> bool {\n    true\n}\n",
        )

        # --- deploy/: the source of truth for absolute install paths -----
        write(
            root / "deploy" / "install.sh",
            '#!/usr/bin/env bash\nPOS_HOME="/var/lib/pili-pos"\nETC_DIR="/var/lib/pili-etc"\n'
            'mkdir -p "$POS_HOME" "${ETC_DIR}/pili-pos" "$POS_HOME/logs"\ntouch "$POS_HOME/pos.db"\n',
        )

        # --- scripts/: allowlist for the placeholder rule -----------------
        write(
            root / "scripts" / "check_dossier_allowlist.txt",
            "# test allowlist\n"
            "docs/SUBMISSION_ARTIFACTS/PLACEHOLDER_FR.md:XXXXXXXXXXX\n",
        )

        dossier = root / "docs" / "SUBMISSION_ARTIFACTS"

        write(
            dossier / "RUST_IDENT_FR.md",
            "Passing references:\n"
            "- `crates/pos-fiscal/src/signature.rs::verify_short_signature`\n"
            "- `do_checkout()`\n"
            "- `commands::checkout`\n"
            "- `pos-fiscal::signature::SignatureError::Invalid`\n"
            "- `chrono::Utc::now()`\n"
            "- `pos_fiscal::full_service`\n"
            "\n"
            "Failing references:\n"
            "- `pos_fiscal::signature::verify_signature`\n"
            "- `nonexistent_helper_fn()`\n"
            "- `crates/pos-fiscal/src/signature.rs::not_a_real_fn`\n",
        )

        write(
            dossier / "REPO_PATH_FR.md",
            "Passing references:\n"
            "- `deploy/install.sh`\n"
            "- `/var/lib/pili-pos`\n"
            "- `crates/pos-fiscal/src/signature.rs:5`\n"
            "- `/var/lib/pili-pos/backups/pos-*.db`\n"
            "- `~/.local/share/pili-pos/pos.db`\n"
            "- `/var/lib/pili-etc/pili-pos`\n"
            "- `/var/lib/pili-pos/logs/pos.log.YYYY-MM-DD`\n"
            "- `/var/lib/pili-pos/pos.db-wal`\n"
            "- `crates/pos-fiscal/src/{signature,missing_but_braced}.rs`\n"
            "- `commands/checkout.rs`\n"
            "\n"
            "Failing references:\n"
            "- `crates/pos-fiscal/src/does_not_exist.rs`\n"
            "- `/var/lib/elsewhere/unknown-YYYY.db`\n"
            "- `/etc/does-not-exist/config.json`\n",
        )

        write(
            dossier / "BRAND_FR.md",
            "Passing: this dossier describes a brand-neutral POS terminal.\n"
            "\n"
            "Failing: installed for Tokyo Sushi Bar and for Nuage Magique.\n",
        )

        write(
            dossier / "EMDASH_FR.md",
            "Passing: a normal sentence, with a comma, and a colon: fine.\n"
            f"Failing: a sentence with an em dash {EM_DASH} right here.\n",
        )

        write(
            dossier / "PLACEHOLDER_FR.md",
            "Allowlisted (masked INSZ in an install example): 'XXXXXXXXXXX'\n"
            "Not allowlisted: TODO fix this section.\n",
        )

        # --- --final: screenshots -----------------------------------------
        write(dossier / "screenshots" / "01_login.png", "not a real png")
        write(
            dossier / "CAPTURES_ECRAN_RUNBOOK_FR.md",
            "### `01_login.png`\nexists on disk.\n\n"
            "### `02_missing.png`\ndoes not exist on disk.\n",
        )

        # --- --final: pdf-freshness ----------------------------------------
        now = time.time()
        fresh_md = dossier / "FRESH_FR.md"
        fresh_pdf = dossier / "pdf_out" / "FRESH_FR.pdf"
        write(fresh_md, "fresh doc\n")
        write(fresh_pdf, "pdf bytes")
        os.utime(fresh_md, (now - 100, now - 100))
        os.utime(fresh_pdf, (now, now))  # newer than the source: OK

        stale_md = dossier / "STALE_FR.md"
        stale_pdf = dossier / "pdf_out" / "STALE_FR.pdf"
        write(stale_md, "stale doc\n")
        write(stale_pdf, "old pdf bytes")
        os.utime(stale_pdf, (now - 200, now - 200))
        os.utime(stale_md, (now, now))  # newer than its own PDF: stale

        # --- --final: manifest ----------------------------------------------
        write(
            dossier / "MANIFEST.txt",
            "pdf_out/FRESH_FR.pdf\n"
            "pdf_out/DOES_NOT_EXIST.pdf\n"
            "GAP_AUDIT_FR.md\n",
        )

    def run_rule(self, rule: str):
        return self.cd.run([rule])

    def messages_for(self, findings, path_suffix: str):
        return [f.render() for f in findings if f.path.endswith(path_suffix)]

    # -- rust-ident ---------------------------------------------------------

    def test_rust_ident_pass_and_fail(self):
        findings = self.run_rule("rust-ident")
        msgs = self.messages_for(findings, "RUST_IDENT_FR.md")
        joined = "\n".join(msgs)

        # The exact example from the task: verify_signature does not exist,
        # only verify_short_signature does.
        self.assertTrue(
            any("verify_signature" in m and "not found" in m for m in msgs),
            f"expected a missing verify_signature finding, got:\n{joined}",
        )
        self.assertTrue(any("nonexistent_helper_fn" in m for m in msgs))
        self.assertTrue(any("not_a_real_fn" in m for m in msgs))

        # None of the passing references (verify_short_signature, do_checkout(),
        # commands::checkout, SignatureError::Invalid, chrono::Utc::now(),
        # pos_fiscal::full_service) should have produced a finding: exactly
        # the 3 failing references above are expected.
        self.assertEqual(
            len(msgs),
            3,
            f"expected exactly 3 rust-ident findings (the failing refs), got:\n{joined}",
        )

    # -- repo-path ------------------------------------------------------------

    def test_repo_path_pass_and_fail(self):
        findings = self.run_rule("repo-path")
        msgs = self.messages_for(findings, "REPO_PATH_FR.md")
        joined = "\n".join(msgs)

        self.assertTrue(any("does_not_exist.rs" in m for m in msgs))
        self.assertTrue(any("/etc/does-not-exist/config.json" in m for m in msgs))
        self.assertTrue(any("missing_but_braced.rs" in m for m in msgs))
        self.assertTrue(any("unknown-YYYY.db" in m for m in msgs))
        self.assertEqual(len(msgs), 4, f"expected exactly 4 repo-path findings, got:\n{joined}")

    def test_shell_variables_are_expanded(self):
        text = 'ETC_DIR="/var/lib/pili-etc"\ncp a "$ETC_DIR/pili-pos/pos.json" "${ETC_DIR}/x"\n'
        out = self.cd.expand_shell_vars(text)
        self.assertIn("/var/lib/pili-etc/pili-pos/pos.json", out)
        self.assertIn("/var/lib/pili-etc/x", out)

    def test_known_absolute_paths(self):
        allowed = {"/var/lib/pili-pos/pos.db", "/var/lib/pili-pos/logs"}
        known = self.cd.abs_path_known
        self.assertTrue(known("/var/lib/pili-pos/pos.db-shm", allowed))
        self.assertTrue(known("/var/lib/pili-pos/logs/pos.log.YYYY-MM-DD", allowed))
        self.assertFalse(known("/var/lib/pili-pos/other.db-wal", allowed))
        self.assertFalse(known("/var/lib/pili-pos/logs/pos.log", allowed))
        self.assertFalse(known("/var/lib/elsewhere/YYYY.log", allowed))

    # -- brand ------------------------------------------------------------------

    def test_brand_pass_and_fail(self):
        findings = self.run_rule("brand")
        msgs = self.messages_for(findings, "BRAND_FR.md")
        self.assertEqual(len(msgs), 1)
        self.assertIn("tokyo sushi", msgs[0])
        self.assertIn("nuage magique", msgs[0])

    # -- em-dash ------------------------------------------------------------------

    def test_em_dash_pass_and_fail(self):
        findings = self.run_rule("em-dash")
        msgs = self.messages_for(findings, "EMDASH_FR.md")
        self.assertEqual(len(msgs), 1)
        self.assertIn("em dash", msgs[0])

    # -- placeholder ---------------------------------------------------------

    def test_placeholder_pass_and_fail(self):
        findings = self.run_rule("placeholder")
        msgs = self.messages_for(findings, "PLACEHOLDER_FR.md")
        self.assertEqual(len(msgs), 1, f"expected only the un-allowlisted TODO, got:\n{msgs}")
        self.assertIn("TODO", msgs[0])

    # -- --final: screenshots ------------------------------------------------

    def test_screenshots_pass_and_fail(self):
        findings = self.run_rule("screenshots")
        msgs = self.messages_for(findings, "CAPTURES_ECRAN_RUNBOOK_FR.md")
        self.assertEqual(len(msgs), 1)
        self.assertIn("02_missing.png", msgs[0])

    # -- --final: pdf-freshness ------------------------------------------------

    def test_pdf_freshness_pass_and_fail(self):
        findings = self.run_rule("pdf-freshness")
        rendered = [f.render() for f in findings]
        self.assertTrue(any("STALE_FR" in m for m in rendered))
        self.assertFalse(any("FRESH_FR" in m for m in rendered))

    # -- --final: manifest ---------------------------------------------------

    def test_manifest_pass_and_fail(self):
        findings = self.run_rule("manifest")
        rendered = [f.render() for f in findings]
        self.assertTrue(any("DOES_NOT_EXIST.pdf" in m for m in rendered))
        self.assertTrue(any("GAP_AUDIT_FR.md" in m and "internal" in m for m in rendered))
        self.assertFalse(any("FRESH_FR.pdf" in m for m in rendered))

    def test_a_missing_manifest_is_a_finding(self):
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            findings = self.cd.rule_manifest(Path(tmp))
        self.assertEqual(len(findings), 1)
        self.assertIn("MANIFEST.txt is missing", findings[0].render())

    # -- CLI plumbing ---------------------------------------------------------

    def test_rules_flag_selects_exact_subset(self):
        findings = self.cd.run(["em-dash", "brand"])
        rule_names = {f.rule for f in findings}
        self.assertEqual(rule_names, {"em-dash", "brand"})

    def test_final_flag_adds_extra_rules(self):
        buf = io.StringIO()
        with redirect_stdout(buf), redirect_stderr(io.StringIO()):
            code = self.cd.main(["--final"])
        self.assertEqual(code, 1)
        output = buf.getvalue()
        # The default rules still run...
        self.assertIn("[brand]", output)
        # ...and --final adds the three pre-submission-only checks.
        self.assertIn("[manifest]", output)
        self.assertIn("[screenshots]", output)
        self.assertIn("[pdf-freshness]", output)

    def test_rules_flag_overrides_final(self):
        # --rules is an explicit whitelist: it wins even when --final is
        # also given, so only the named rule runs.
        buf = io.StringIO()
        with redirect_stdout(buf), redirect_stderr(io.StringIO()):
            code = self.cd.main(["--final", "--rules", "manifest"])
        self.assertEqual(code, 1)
        output = buf.getvalue()
        self.assertIn("[manifest]", output)
        self.assertNotIn("[brand]", output)

    def test_unknown_rule_is_rejected(self):
        # argparse prints usage/error to stderr and calls sys.exit(2).
        with self.assertRaises(SystemExit):
            with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                self.cd.main(["--rules", "not-a-real-rule"])

    def test_default_rules_exclude_final_only_checks(self):
        findings = self.cd.run(self.cd.DEFAULT_RULES)
        rule_names = {f.rule for f in findings}
        self.assertFalse(rule_names & {"screenshots", "pdf-freshness", "manifest"})


if __name__ == "__main__":
    sys.exit(unittest.main())
