#!/usr/bin/env python3
"""Unit tests of check_command_parity.py (standard library only)."""

import io
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import check_command_parity as cp  # noqa: E402

DATA = HERE / "testdata" / "command_parity"
REPO = DATA / "repo"
FFI = DATA / "ffi"
REAL_ROOT = HERE.parent.parent


def run(*argv):
    out = io.StringIO()
    code = cp.main(list(argv), out=out)
    return code, out.getvalue()


def parity(variant, *extra):
    return run("--repo", str(REPO), "--ffi-dir", str(FFI / variant), *extra)


def hygiene(variant, *extra):
    return run("--repo", str(REPO), "--ffi-dir", str(FFI / variant), "--hygiene", *extra)


class TempFfi:
    """A throwaway pos-ffi tree: {relative path: source}."""

    def __init__(self, files):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name) / "src"
        for rel, text in files.items():
            p = self.dir / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text)

    def __enter__(self):
        return self

    def __exit__(self, *a):
        self.tmp.cleanup()

    def hygiene(self):
        return run("--repo", str(REPO), "--ffi-dir", str(self.dir), "--hygiene")

    def parity(self, *extra):
        return run("--repo", str(REPO), "--ffi-dir", str(self.dir), *extra)


class CertSet(unittest.TestCase):
    def test_peppol_gated_command_is_not_in_the_cert_set(self):
        ipc = cp.load_ipc_module()
        args = type("A", (), {"repo": str(REPO), "map_file": None, "signatures_file": None})()
        cert = cp.build_cert(args, ipc)
        self.assertEqual(sorted(cert), ["check_gamma", "get_alpha", "set_beta"])
        self.assertEqual(cert["set_beta"], ("alpha", "do_set_beta", "sync", ["item_id", "label"]))
        self.assertEqual(cert["check_gamma"][2], "async")

    def test_hidden_map_and_signature_files(self):
        with tempfile.TemporaryDirectory() as d:
            m = Path(d) / "map.tsv"
            m.write_text("one\tmod_a\tdo_one\tsync\ntwo\tmod_b\tdo_two\tasync\n")
            s = Path(d) / "sigs.txt"
            s.write_text("one|sync|state: State|Result<(), String>\ntwo|async|state: State, x: i64, y: Option<String>|Result<(), String>\n")
            code, out = run(
                "--repo", str(REPO), "--ffi-dir", str(FFI / "missing_dir"),
                "--map-file", str(m), "--signatures-file", str(s),
            )
            self.assertEqual(code, 0)
            self.assertEqual(out.strip(), "parity: 0/2 (no pos-ffi yet)")


class ParityGood(unittest.TestCase):
    def test_good_tree_passes(self):
        code, out = parity("good")
        self.assertEqual(code, 0, out)
        self.assertEqual(out, "parity: 3/3 (wrapper, engine fn, FFI method)\nmissing: none\n")

    def test_strict_and_partial(self):
        code, out = parity("partial")
        self.assertEqual(code, 0)
        self.assertIn("parity: 2/3", out)
        self.assertIn("missing: check_gamma", out)
        code, _ = parity("partial", "--strict")
        self.assertEqual(code, 1)
        code, _ = parity("good", "--strict")
        self.assertEqual(code, 0)

    def test_require_modules(self):
        code, out = parity("partial", "--require-modules", "alpha")
        self.assertEqual(code, 0, out)
        code, out = parity("partial", "--require-modules", "alpha,gamma")
        self.assertEqual(code, 1)
        self.assertIn("require-modules: gamma: missing check_gamma", out)
        code, out = parity("good", "--require-modules", "nope")
        self.assertEqual(code, 1)
        self.assertIn("unknown engine module nope", out)

    def test_extras_are_not_commands(self):
        # open, shutdown and events live in good/core.rs and raise no issue
        code, out = parity("good")
        self.assertNotIn("shutdown", out)
        self.assertEqual(code, 0)


class ParityFailures(unittest.TestCase):
    def assert_issue(self, variant, fragment):
        code, out = parity(variant)
        self.assertEqual(code, 1, out)
        self.assertIn(fragment, out)
        self.assertLess(out.count("\n"), 6, out)
        return out

    def test_wrong_name(self):
        out = self.assert_issue("bad_name", "get_alfa: `get_alfa` is not a cert command")
        self.assertIn("missing: get_alpha", out)

    def test_peppol_command_is_not_a_cert_command(self):
        self.assert_issue("bad_peppol_cmd", "sign_thing: `sign_thing` is not a cert command")

    def test_not_async(self):
        self.assert_issue("bad_not_async", "set_beta: `set_beta` must be `pub async fn`")

    def test_wrong_param_name(self):
        self.assert_issue(
            "bad_param", "parameters (id, label) differ from the wrapper (item_id, label)"
        )

    def test_param_order_matters(self):
        with TempFfi(
            {
                "api/a.rs": "#[uniffi::export]\nimpl PosEngine {\n"
                "    pub async fn set_beta(&self, label: String, item_id: i64) -> R {\n"
                "        self.blocking(move |e| alpha::do_set_beta(e, item_id, label)).await\n"
                "    }\n}\n"
            }
        ) as t:
            code, out = t.parity()
            self.assertEqual(code, 1)
            self.assertIn("parameters (label, item_id) differ", out)

    def test_wrong_engine_fn(self):
        self.assert_issue("bad_engine_fn", "body does not call the engine function do_set_beta")

    def test_suffix_colliding_engine_fn_name(self):
        # xdo_get_alpha( ends with do_get_alpha( but is another function
        self.assert_issue("bad_suffix_call", "body does not call the engine function do_get_alpha")

    def test_method_call_is_not_an_engine_path(self):
        self.assert_issue("bad_method_call", "body does not call the engine function do_get_alpha")

    def test_wrong_engine_module(self):
        self.assert_issue("bad_module", "expected module gamma")

    def test_spawn_on_sync_engine_fn(self):
        self.assert_issue("bad_spawn_sync", "do_get_alpha is sync: use self.blocking, not self.spawn")

    def test_blocking_on_async_engine_fn(self):
        self.assert_issue(
            "bad_blocking_async", "do_check_gamma is async: use self.spawn, not self.blocking"
        )

    def test_neither_blocking_nor_spawn(self):
        with TempFfi(
            {
                "api/a.rs": "#[uniffi::export]\nimpl PosEngine {\n"
                "    pub async fn get_alpha(&self) -> R { alpha::do_get_alpha(&self.e) }\n}\n"
            }
        ) as t:
            code, out = t.parity()
            self.assertEqual(code, 1)
            self.assertIn("neither self.blocking nor self.spawn", out)

    def test_duplicate_method(self):
        body = (
            "#[uniffi::export]\nimpl PosEngine {\n"
            "    pub async fn get_alpha(&self) -> R { self.blocking(|e| alpha::do_get_alpha(e)).await }\n}\n"
        )
        with TempFfi({"api/a.rs": body, "api/b.rs": body}) as t:
            code, out = t.parity()
            self.assertEqual(code, 1)
            self.assertIn("duplicate method", out)

    def test_comments_and_strings_do_not_count(self):
        # the engine call is only in a comment and a string: still a failure
        with TempFfi(
            {
                "api/a.rs": "#[uniffi::export]\nimpl PosEngine {\n"
                "    pub async fn get_alpha(&self) -> R {\n"
                "        // alpha::do_get_alpha(e)\n"
                '        let s = "alpha::do_get_alpha(e)";\n'
                "        self.blocking(|e| other(e)).await\n"
                "    }\n}\n"
            }
        ) as t:
            code, out = t.parity()
            self.assertEqual(code, 1)
            self.assertIn("body does not call the engine function do_get_alpha", out)

    def test_other_types_methods_are_not_checked(self):
        with TempFfi(
            {
                "api/a.rs": "#[uniffi::export]\nimpl Helper {\n"
                "    pub async fn anything(&self) {}\n}\n"
            }
        ) as t:
            code, out = t.parity()
            self.assertEqual(code, 0, out)
            self.assertIn("parity: 0/3", out)


class Markdown(unittest.TestCase):
    def test_table_for_good_tree(self):
        code, out = parity("good", "--markdown")
        self.assertEqual(code, 0)
        lines = out.splitlines()
        self.assertEqual(lines[0], "| Command | Engine function | Call | FFI method (Kotlin) |")
        self.assertIn("| check_gamma | gamma::do_check_gamma | spawn | posEngine.checkGamma() |", lines)
        self.assertIn("| set_beta | alpha::do_set_beta | blocking | posEngine.setBeta() |", lines)
        self.assertEqual(lines[-1], "Parity: 3/3.")

    def test_missing_and_sorted_and_deterministic(self):
        _, a = parity("partial", "--markdown")
        _, b = parity("partial", "--markdown")
        self.assertEqual(a, b)
        rows = [l for l in a.splitlines() if l.startswith("| ") and l[2].islower()]
        self.assertEqual([r.split(" | ")[0] for r in rows], ["| check_gamma", "| get_alpha", "| set_beta"])
        self.assertIn("| check_gamma | gamma::do_check_gamma | spawn | missing |", a)
        self.assertTrue(a.rstrip().endswith("Parity: 2/3."))


class MarkdownMatchesParity(unittest.TestCase):
    def test_method_with_an_issue_is_not_counted(self):
        # get_alpha has a wrong param name: parity says 1/3, markdown must too
        files = {
            "api/a.rs": "#[uniffi::export]\nimpl PosEngine {\n"
            "    pub async fn get_alpha(&self, bogus: i64) -> R {\n"
            "        self.blocking(|e| alpha::do_get_alpha(e)).await\n    }\n"
            "    pub async fn set_beta(&self, item_id: i64, label: String) -> R {\n"
            "        self.blocking(|e| alpha::do_set_beta(e, item_id, label)).await\n    }\n}\n"
        }
        with TempFfi(files) as t:
            _, par = t.parity()
            _, md = t.parity("--markdown")
        self.assertIn("parity: 1/3", par)
        self.assertTrue(md.rstrip().endswith("Parity: 1/3."))
        self.assertIn("| get_alpha | alpha::do_get_alpha | blocking | missing |", md)
        self.assertIn("| set_beta | alpha::do_set_beta | blocking | posEngine.setBeta() |", md)


class HygieneRules(unittest.TestCase):
    def violations(self, variant, rule):
        code, out = hygiene(variant)
        self.assertEqual(code, 1, out)
        lines = [l for l in out.splitlines() if f": {rule}:" in l]
        self.assertTrue(lines, out)
        return lines

    def test_clean_tree(self):
        for variant in ("good", "hyg_good"):
            code, out = hygiene(variant)
            self.assertEqual(code, 0, out)
            self.assertRegex(out, r"^hygiene: ok \(\d+ exports, \d+ types\)\n$")

    def test_h1_sync_export(self):
        lines = self.violations("hyg_h1", "H1")
        self.assertEqual(len(lines), 2)
        self.assertIn("api/x.rs:3: H1", lines[0])
        self.assertIn("free_sync", lines[1])

    def test_h1_only_open_may_be_a_sync_constructor(self):
        with TempFfi(
            {
                "core.rs": "#[uniffi::export]\nimpl PosEngine {\n"
                "    #[uniffi::constructor]\n    pub fn open() -> Arc<Self> { todo!() }\n"
                "    #[uniffi::constructor]\n    pub fn other() -> Arc<Self> { todo!() }\n}\n"
            }
        ) as t:
            code, out = t.hygiene()
            self.assertEqual(code, 1)
            self.assertEqual(out.count("H1"), 2, out)
            self.assertIn("other", out)
            self.assertNotIn("`open`", out)

    def test_h2_target_os(self):
        self.assertIn("H2: cfg(target_os)", self.violations("hyg_h2", "H2")[0])

    def test_h3_serde_derives_and_deny_unknown_fields(self):
        lines = self.violations("hyg_h3", "H3")
        text = "\n".join(lines)
        self.assertIn("NoSerde does not derive Serialize", text)
        self.assertIn("NoSerde does not derive Deserialize", text)
        self.assertIn("NoSerde lacks #[serde(deny_unknown_fields)]", text)
        self.assertIn("NoDeny lacks #[serde(deny_unknown_fields)]", text)
        self.assertIn("HalfSerde does not derive Deserialize", text)
        self.assertNotIn("HalfSerde lacks", text)  # enums need no deny_unknown_fields

    def test_h3_custom_contract_exemption(self):
        # hyg_good/events.rs has a record without deny_unknown_fields, marked custom
        code, out = hygiene("hyg_good")
        self.assertEqual(code, 0, out)
        src = (FFI / "hyg_good" / "events.rs").read_text()
        with TempFfi({"events.rs": src.replace("// contract: custom\n", "")}) as t:
            code, out = t.hygiene()
            self.assertEqual(code, 1)
            self.assertIn("Free lacks #[serde(deny_unknown_fields)]", out)

    def test_h3_marker_above_a_doc_comment(self):
        src = (
            "// contract: custom\n/// Doc.\n"
            "#[derive(uniffi::Record, serde::Serialize, serde::Deserialize)]\n"
            "pub struct R { pub id: i64 }\n"
        )
        with TempFfi({"records/r.rs": src}) as t:
            self.assertEqual(t.hygiene()[0], 0)

    def test_h3_only_in_contract_files(self):
        src = "#[derive(uniffi::Record)]\npub struct R { pub id: i64 }\n"
        with TempFfi({"misc.rs": src}) as t:
            self.assertEqual(t.hygiene()[0], 0)
        with TempFfi({"api/misc.rs": src}) as t:
            self.assertEqual(t.hygiene()[0], 1)

    def test_h4_duplicate_type_names(self):
        lines = self.violations("hyg_h4", "H4")
        self.assertEqual(len(lines), 1)
        self.assertIn("`Same`", lines[0])

    def test_h5_statics(self):
        text = "\n".join(self.violations("hyg_h5", "H5"))
        for label in ("`static`", "`thread_local!`", "`lazy_static!`", "`OnceLock`", "`OnceCell`", "`LazyLock`"):
            self.assertIn(label, text)

    def test_h5_static_lifetime_is_fine(self):
        with TempFfi({"x.rs": "pub fn f(s: &'static str) -> &'static str { s }\n"}) as t:
            self.assertEqual(t.hygiene()[0], 0)

    def test_h2_h5_allowed_in_logging(self):
        with TempFfi(
            {"logging.rs": '#[cfg(target_os = "android")]\nstatic X: OnceLock<u8> = OnceLock::new();\n'}
        ) as t:
            code, out = t.hygiene()
            self.assertEqual(code, 0, out)

    def test_h6_unsigned_integers(self):
        lines = self.violations("hyg_h6", "H6")
        text = "\n".join(lines)
        self.assertIn("`u32` in Wide", text)
        self.assertIn("`usize` in Wide", text)
        self.assertIn("`u64` in Wide2", text)
        self.assertIn("`u16` in the signature of `takes_u16`", text)
        self.assertNotIn("`u8`", text)  # Vec<u8> is allowed

    def test_h6_vec_u8_in_signature_is_allowed(self):
        with TempFfi(
            {"x.rs": "#[uniffi::export]\nimpl PosEngine {\n"
             "    pub async fn f(&self, b: Vec<u8>) -> Result<Vec<u8>, PosError> { todo!() }\n}\n"}
        ) as t:
            self.assertEqual(t.hygiene()[0], 0)

    def test_h7_peppol_and_invoice(self):
        lines = self.violations("hyg_h7", "H7")
        self.assertEqual(len(lines), 2)
        self.assertIn("api/x.rs:3", lines[0])
        self.assertIn("api/x.rs:9", lines[1])  # peppol_gated

    def test_h7_allows_bare_invoice_vocabulary_and_strings(self):
        # the cert engine has invoice vocabulary (event kind, Z-report data)
        src = (
            '#[derive(Debug, uniffi::Enum, serde::Serialize, serde::Deserialize)]\n'
            'pub enum FiscalKind { Invoice, Ticket }\n'
            'pub struct ZData { pub invoices: i64 }\n'
            'pub fn f() -> String { "peppol sign_invoice".into() }\n'
        )
        with TempFfi({"x.rs": src}) as t:
            code, out = t.hygiene()
            self.assertEqual(code, 0, out)

    def test_h7_camel_case_identifiers(self):
        with TempFfi({"x.rs": "pub struct IssueB2bInvoiceInput;\npub fn SignInvoiceFiscal() {}\n"}) as t:
            code, out = t.hygiene()
            self.assertEqual(code, 1)
            self.assertEqual(out.count("H7"), 2)

    def test_h7_ignores_comments(self):
        with TempFfi({"x.rs": "// peppol invoice\n/* invoice */\npub fn f() {}\n"}) as t:
            self.assertEqual(t.hygiene()[0], 0)

    def test_hygiene_is_independent_of_parity(self):
        # hyg_h2 has no cert command at all; hygiene alone reports only H2
        code, out = hygiene("hyg_h2")
        self.assertEqual(code, 1)
        self.assertNotIn("parity", out)


class RealRepo(unittest.TestCase):
    def script(self, *args):
        return subprocess.run(
            [sys.executable, str(HERE / "check_command_parity.py"), *args],
            capture_output=True, text=True, cwd=REAL_ROOT,
        )

    def test_no_pos_ffi_yet(self):
        if (REAL_ROOT / cp.DEFAULT_FFI).exists():
            self.skipTest("pos-ffi exists")
        r = self.script()
        self.assertEqual((r.returncode, r.stdout), (0, "parity: 0/84 (no pos-ffi yet)\n"))
        r = self.script("--strict")
        self.assertEqual((r.returncode, r.stdout), (1, "parity: 0/84 (no pos-ffi yet)\n"))
        r = self.script("--hygiene")
        self.assertEqual((r.returncode, r.stdout), (0, "hygiene: no pos-ffi yet\n"))

    def test_markdown_lists_the_84_cert_commands(self):
        r = self.script("--markdown")
        self.assertEqual(r.returncode, 0, r.stderr)
        rows = [l for l in r.stdout.splitlines() if l.startswith("| ") and l[2].islower()]
        self.assertEqual(len(rows), 84)
        self.assertNotIn("sign_invoice_fiscal", r.stdout)
        self.assertNotIn("issue_b2b_invoice", r.stdout)
        if not (REAL_ROOT / cp.DEFAULT_FFI).exists():
            self.assertTrue(all(row.endswith("| missing |") for row in rows))
            self.assertTrue(r.stdout.rstrip().endswith("Parity: 0/84."))
        self.assertEqual(rows, sorted(rows))


class NoEmDash(unittest.TestCase):
    def test_sources_and_fixtures_have_no_em_dash(self):
        files = [HERE / "check_command_parity.py", Path(__file__)]
        files += [p for p in DATA.rglob("*") if p.is_file()]
        for p in files:
            self.assertNotIn(chr(8212), p.read_text(), str(p))


if __name__ == "__main__":
    unittest.main()
