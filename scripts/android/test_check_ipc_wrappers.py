#!/usr/bin/env python3
"""Unit tests of check_ipc_wrappers.py (standard library only)."""

import contextlib
import io
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_ipc_wrappers as chk  # noqa: E402

HEADER = "use tauri::State;\nuse super::Managed;\n\n"

VALID = HEADER + """
/// Log in.
#[tauri::command]
pub fn login(
    state: State<'_, Managed>,
    insz: String,
    pin: Option<String>,
) -> Result<User, String> {
    crate::commands::auth::do_login(state.inner(), insz, pin)
}

#[tauri::command]
pub async fn checkout(
    state: State<'_, Managed>,
    order_id: i64,
    req: Request,
    list: Vec<PaymentPart>,
    delivery: Option<TicketDeliveryRequest>,
) -> Result<CheckoutResult, String> {
    crate::commands::auth::do_checkout(
        state.inner(),
        order_id,
        req.order_id,
        &req.payments,
        &list,
        delivery.as_ref(),
        req.mode.as_ref(),
        req.when.as_deref(),
        lang.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn sync_engine_in_async_wrapper(state: State<'_, Managed>) -> Result<(), String> {
    commands::auth::do_logout(state.inner())
}
"""

MAIN = """
fn main() {
    let builder = builder.invoke_handler(tauri::generate_handler![
        ipc::auth::login,
        ipc::auth::checkout,
        ipc::auth::sync_engine_in_async_wrapper,
    ]);
}
"""


def wrapper(body, name="login", args="state: State<'_, Managed>, insz: String"):
    return (
        HEADER
        + f"#[tauri::command]\npub fn {name}({args}) -> Result<(), String> {{\n{body}\n}}\n"
    )


class CheckerTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.ipc = self.root / "ipc"
        self.ipc.mkdir()
        (self.ipc / "mod.rs").write_text("pub mod auth;\n")
        self.main = self.root / "main.rs"
        self._saved = (chk.IPC_DIR, chk.MAIN_RS, chk.ROOT)
        chk.IPC_DIR, chk.MAIN_RS, chk.ROOT = self.ipc, self.main, self.root

    def tearDown(self):
        chk.IPC_DIR, chk.MAIN_RS, chk.ROOT = self._saved
        self._tmp.cleanup()

    def run_check(self, ipc_source, main_source):
        (self.ipc / "auth.rs").write_text(ipc_source)
        self.main.write_text(main_source)
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = chk.run_check()
        return code, out.getvalue()

    def test_valid_wrappers(self):
        code, out = self.run_check(VALID, MAIN)
        self.assertEqual(code, 0, out)
        self.assertIn("commands: 3 (3 without peppol), single-call: 3/3", out)

    def test_two_statements(self):
        src = wrapper(
            "    log();\n    crate::commands::auth::do_login(state.inner(), insz)"
        )
        code, out = self.run_check(src, "generate_handler![ipc::auth::login,]")
        self.assertEqual(code, 1)
        self.assertRegex(out, r"auth\.rs:\d+: login: ")

    def test_first_argument_not_state_inner(self):
        src = wrapper("    crate::commands::auth::do_login(&state, insz)")
        code, out = self.run_check(src, "generate_handler![ipc::auth::login,]")
        self.assertEqual(code, 1)
        self.assertIn("first argument must be exactly state.inner()", out)

    def test_disallowed_argument_expression(self):
        for arg in ('insz.clone()', 'insz.unwrap_or("fr")', "insz + 1", "&insz[1..]"):
            src = wrapper(f"    crate::commands::auth::do_login(state.inner(), {arg})")
            code, out = self.run_check(src, "generate_handler![ipc::auth::login,]")
            self.assertEqual(code, 1, arg)
            self.assertIn("is not an allowed form", out, arg)

    def test_call_outside_commands(self):
        src = wrapper("    other::auth::do_login(state.inner(), insz)")
        code, out = self.run_check(src, "generate_handler![ipc::auth::login,]")
        self.assertEqual(code, 1)
        self.assertIn("does not end in commands::<module>::<fn>", out)

    def test_wrapper_missing_from_handler_list(self):
        main = MAIN.replace("        ipc::auth::checkout,\n", "")
        code, out = self.run_check(VALID, main)
        self.assertEqual(code, 1)
        self.assertIn("checkout: missing from handler list 1 of main.rs", out)

    def test_handler_without_wrapper(self):
        main = MAIN.replace("ipc::auth::login,", "ipc::auth::login,\n        ipc::auth::ghost,")
        code, out = self.run_check(VALID, main)
        self.assertEqual(code, 1)
        self.assertIn("ipc::auth::ghost has no wrapper", out)

    def test_handler_entry_not_ipc(self):
        main = MAIN.replace("ipc::auth::login", "commands::auth::login")
        code, out = self.run_check(VALID, main)
        self.assertEqual(code, 1)
        self.assertIn("is not ipc::<module>::<command>", out)

    def test_signatures_normalisation(self):
        d = self.root / "sig"
        d.mkdir()
        (d / "a.rs").write_text(
            "use tauri::State;\n"
            "#[tauri::command]\n"
            "pub async fn zeta(\n"
            "    state: tauri::State<'_, Managed>,\n"
            "    items:   Vec< Item >,\n"
            "    lang: Option<String>,\n"
            ") -> Result<\n    Vec<Row>,\n    String,\n> {\n"
            "    todo()\n}\n"
            "#[tauri::command]\n"
            "pub fn alpha(state: State<'_, AppState>) -> Result<(), String> { x() }\n"
        )
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            self.assertEqual(chk.run_signatures(d), 0)
        self.assertEqual(
            out.getvalue().splitlines(),
            [
                "alpha|sync|state: State|Result<(), String>",
                "zeta|async|state: State, items: Vec<Item>, lang: Option<String>|Result<Vec<Row>, String>",
            ],
        )

    def test_signatures_ignore_the_managed_type(self):
        a = self.root / "a"
        b = self.root / "b"
        a.mkdir()
        b.mkdir()
        (a / "x.rs").write_text(
            "#[tauri::command]\npub fn f(state: State<'_, AppState>, n: i64) -> Result<(), String> { g() }\n"
        )
        (b / "x.rs").write_text(
            "#[tauri::command]\npub fn f(state: State<'_, Managed>, n: i64) -> Result<(), String> { g() }\n"
        )
        outs = []
        for d in (a, b):
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                chk.run_signatures(d)
            outs.append(out.getvalue())
        self.assertEqual(outs[0], outs[1])


if __name__ == "__main__":
    unittest.main()
