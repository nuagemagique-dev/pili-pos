#!/usr/bin/env python3
"""Self-test of scripts/android/check_ui_parity.py.

Builds fixture repositories in temporary directories (no committed test data) and runs the
checker on them: one test per FAIL rule, the stale warning with and without --strict, the
lambda scan, the camelCase conversion, the parity tables and the real repository at HEAD.
Prints the unittest summary and one line

    SELFTEST | total | PASS|FAIL | <n> tests

and exits 1 when a test fails. Standard library only.
"""

import importlib.util
import io
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SCRIPT = HERE / "check_ui_parity.py"

_spec = importlib.util.spec_from_file_location("check_ui_parity", SCRIPT)
P = importlib.util.module_from_spec(_spec)
sys.modules["check_ui_parity"] = P
_spec.loader.exec_module(P)

KT_DIR = "android/app/src/main/kotlin/dev/nuagemagique/pilipos"


def kotlin_call(*cmds_camel):
    body = "\n".join("        it.%s()" % c for c in cmds_camel)
    return "package x\nclass A {\n    suspend fun f() {\n        gateway.call {\n%s\n        }\n    }\n}\n" % body


class Fixture:
    """A fixture repository in a temporary directory."""

    def __init__(self, commands=("login", "logout", "create_order"), ffi=None, declarations=None, tables=""):
        self.dir = Path(tempfile.mkdtemp(prefix="uiparity."))
        self.commands = list(commands)
        self.write_ipc(self.commands)
        self.write_api(self.commands if ffi is None else ffi)
        if declarations is None:
            declarations = ["%s pending:P5-T4" % c for c in self.commands]
        self.write_parity(declarations, tables)

    def cleanup(self):
        shutil.rmtree(self.dir, ignore_errors=True)

    def put(self, rel, text):
        path = self.dir / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def write_ipc(self, commands):
        first = commands[:-1] if len(commands) > 1 else commands
        last = commands[-1:] if len(commands) > 1 else []
        self.put(
            "ui/src/api/ipc.ts",
            "".join('export const f%d = () => invoke("%s", {});\n' % (i, c) for i, c in enumerate(first))
            + 'import { invoke } from "x";\n',
        )
        self.put(
            "ui/src/api/ipc-modifiers.ts",
            "".join('export const g%d = () => invoke<Foo[]>("%s", { a });\n' % (i, c) for i, c in enumerate(last)),
        )

    def write_api(self, methods):
        lines = ["public abstract interface class uniffi/pos_ffi/Other {", "\tpublic abstract fun notMine (I)V;", "}", ""]
        lines.append("public abstract interface class uniffi/pos_ffi/PosEngineInterface {")
        lines.append("\tpublic static final field Companion Luniffi/pos_ffi/PosEngineInterface$Companion;")
        for m in methods:
            lines.append("\tpublic abstract fun %s (Lkotlin/coroutines/Continuation;)Ljava/lang/Object;" % P.camel(m))
        lines += ["}", ""]
        self.put("android/pos-core/api/pos-core.api", "\n".join(lines))

    def write_parity(self, declarations, tables=""):
        self.put(
            "docs/android/PARITY.md",
            "# Parity\n\n" + tables + "\n```ui-parity\n# comment\n\n" + "\n".join(declarations) + "\n```\n",
        )

    def kotlin(self, name, text, sub="ui"):
        self.put("%s/%s/%s" % (KT_DIR, sub, name), text)

    def run(self, *args):
        proc = subprocess.run(
            [sys.executable, str(SCRIPT), "--repo", str(self.dir)] + list(args),
            capture_output=True,
            text=True,
        )
        return proc.returncode, proc.stdout


def lines_of(out, command):
    return [ln for ln in out.splitlines() if ln.startswith("UIPARITY | %s | " % command)]


def problems_of(out):
    return [ln for ln in out.splitlines() if ln.startswith("UIPARITY | problem | FAIL | ")]


def total_of(out):
    return out.strip().splitlines()[-1]


class FixtureCase(unittest.TestCase):
    def fx(self, **kw):
        f = Fixture(**kw)
        self.addCleanup(f.cleanup)
        return f


class CommandSetTests(FixtureCase):
    def test_commands_parsed_from_both_files_with_type_argument(self):
        f = self.fx(commands=("login", "logout", "create_order"))
        rc, out = f.run()
        self.assertEqual(rc, 0, out)
        self.assertTrue(total_of(out).endswith("(3 commands)"), out)
        self.assertEqual(len(lines_of(out, "create_order")), 1)

    def test_invoke_forms(self):
        text = (
            'invoke("a_b", {}); invoke<Foo>("c"); invoke<Array<Foo>>( "d_e_f" );\n'
            "invoke('single_q'); notinvoke(\"x\"); invoke(name); invoke(\"Upper\")\n"
        )
        self.assertEqual(P.INVOKE_RE.findall(text), ["a_b", "c", "d_e_f", "single_q"])

    def test_zero_names_is_a_parse_error(self):
        f = self.fx()
        f.put("ui/src/api/ipc.ts", "// nothing here\n")
        f.put("ui/src/api/ipc-modifiers.ts", "")
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("parse error: no invoke", out)
        self.assertIn("| total | FAIL |", total_of(out))

    def test_missing_ipc_file_is_a_parse_error(self):
        f = self.fx()
        (f.dir / "ui/src/api/ipc-modifiers.ts").unlink()
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("cannot read ui/src/api/ipc-modifiers.ts", out)


class FfiTests(FixtureCase):
    def test_camel_case_conversion(self):
        self.assertEqual(P.camel("login"), "login")
        self.assertEqual(P.camel("login_with_new_pin"), "loginWithNewPin")
        self.assertEqual(P.camel("hubrise_get_connection_status"), "hubriseGetConnectionStatus")
        self.assertEqual(P.camel("list_z_reports"), "listZReports")
        self.assertEqual(P.camel("get_order"), "getOrder")

    def test_not_in_ffi_fails(self):
        f = self.fx(ffi=("login", "logout"))
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("not-in-ffi: create_order has no method createOrder", out)

    def test_method_of_another_class_does_not_count(self):
        f = self.fx(ffi=("login", "logout"))
        text = (f.dir / "android/pos-core/api/pos-core.api").read_text()
        text = text.replace("\tpublic abstract fun notMine (I)V;", "\tpublic abstract fun createOrder (I)V;")
        (f.dir / "android/pos-core/api/pos-core.api").write_text(text)
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("not-in-ffi: create_order", out)

    def test_missing_interface_block_fails(self):
        f = self.fx()
        f.put("android/pos-core/api/pos-core.api", "public final class X {\n}\n")
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("no PosEngineInterface block", out)


class CallSiteTests(FixtureCase):
    def called(self, f, cmd):
        rc, out = f.run()
        return [ln for ln in lines_of(out, cmd) if "| CALLED |" in ln], rc, out

    def test_call_in_lambda_is_found_with_file_and_line(self):
        f = self.fx()
        f.kotlin("A.kt", kotlin_call("logout"))
        got, _, _ = self.called(f, "logout")
        self.assertEqual(len(got), 1)
        self.assertIn("%s/ui/A.kt:5" % KT_DIR, got[0])

    def test_call_outside_a_lambda_is_not_a_call_site(self):
        f = self.fx()
        f.kotlin("A.kt", "class A {\n  fun f(engine: E) { engine.logout() }\n}\n")
        got, _, out = self.called(f, "logout")
        self.assertEqual(got, [], out)

    def test_call_split_over_lines_inside_a_lambda_is_found(self):
        f = self.fx()
        f.kotlin("A.kt", "class A {\n  fun f() {\n    gateway.call {\n      it\n        .logout\n        ()\n    }\n  }\n}\n")
        got, _, out = self.called(f, "logout")
        self.assertEqual(len(got), 1, out)
        self.assertIn("/A.kt:5 ", got[0] + " ")

    def test_name_and_paren_on_the_same_later_line(self):
        f = self.fx()
        f.kotlin("A.kt", "class A {\n  fun f() {\n    gateway.call {\n      it\n        .logout(\n        )\n    }\n  }\n}\n")
        got, _, _ = self.called(f, "logout")
        self.assertEqual(len(got), 1)

    def test_commented_occurrences_are_ignored(self):
        f = self.fx()
        f.kotlin(
            "A.kt",
            "class A {\n  fun f() {\n    gateway.call {\n"
            "      // it.logout()\n      /* it.login() */\n      /* outer /* nested */ it.logout() */\n"
            "    }\n  }\n}\n",
        )
        rc, out = f.run()
        self.assertEqual([ln for ln in out.splitlines() if "| CALLED |" in ln], [], out)

    def test_string_occurrences_are_ignored(self):
        f = self.fx()
        f.kotlin(
            "A.kt",
            'class A {\n  fun f() {\n    gateway.call {\n      log(".logout()")\n'
            '      log("""\n  .login()\n""")\n      log("${x}.create_order() '
            "'.logout('\")\n      val c = '('\n    }\n  }\n}\n",
        )
        rc, out = f.run()
        self.assertEqual([ln for ln in out.splitlines() if "| CALLED |" in ln], [], out)

    def test_nested_braces_do_not_end_the_lambda_early(self):
        f = self.fx()
        f.kotlin(
            "A.kt",
            "class A {\n  fun f() {\n    gateway.call {\n"
            "      items.forEach { x -> if (x) { y() } }\n      val m = mapOf(1 to { 2 })\n      it.logout()\n"
            "    }\n    other.logout()\n  }\n}\n",
        )
        got, _, out = self.called(f, "logout")
        self.assertEqual(len(got), 1, out)
        self.assertIn("/A.kt:6 ", got[0] + " ")

    def test_call_with_parenthesis_then_lambda(self):
        f = self.fx()
        f.kotlin(
            "A.kt",
            "class A {\n  fun f() {\n    gateway.call(timeout = 3) { it.logout() }\n"
            "    gateway.call(block = { it.login() })\n    gateway.call(it.create_order())\n  }\n}\n",
        )
        rc, out = f.run()
        self.assertEqual(len(lines_of(out, "logout")), 1)
        self.assertIn("| CALLED |", lines_of(out, "logout")[0])
        self.assertIn("| CALLED |", lines_of(out, "login")[0])
        self.assertNotIn("| CALLED |", lines_of(out, "create_order")[0])

    def test_generic_call_and_safe_call(self):
        f = self.fx()
        f.kotlin("A.kt", "class A {\n  fun f() {\n    g?.call<Unit> { it.logout() }\n  }\n}\n")
        got, _, _ = self.called(f, "logout")
        self.assertEqual(len(got), 1)

    def test_call_named_differently_does_not_open_a_lambda(self):
        f = self.fx()
        f.kotlin("A.kt", "class A {\n  fun f() {\n    gateway.callLater { it.logout() }\n    x.recall { it.login() }\n  }\n}\n")
        rc, out = f.run()
        self.assertEqual([ln for ln in out.splitlines() if "| CALLED |" in ln], [], out)

    def test_generated_i18n_directory_is_skipped(self):
        f = self.fx()
        f.kotlin("A.kt", kotlin_call("logout"), sub="i18n/generated")
        rc, out = f.run()
        self.assertEqual([ln for ln in out.splitlines() if "| CALLED |" in ln], [], out)

    def test_several_sites_are_listed_sorted_and_deduplicated(self):
        f = self.fx()
        f.kotlin("B.kt", kotlin_call("logout"))
        f.kotlin("A.kt", kotlin_call("logout"))
        got, _, _ = self.called(f, "logout")
        self.assertEqual(len(got), 1)
        self.assertLess(got[0].index("/A.kt:5"), got[0].index("/B.kt:5"))

    def test_strip_kotlin_keeps_offsets_and_handles_templates(self):
        src = 'val a = "x ${ "y}" } z" // c\n/* a\n b */ val b = """r"q""" + \'}\' + 1\n'
        out = P.strip_kotlin(src, drop_strings=True)
        self.assertEqual(len(out), len(src))
        self.assertEqual(out.count("\n"), src.count("\n"))
        self.assertNotIn("y}", out)
        self.assertNotIn("//", out)
        self.assertNotIn("}", out)
        kept = P.strip_kotlin(src, drop_strings=False)
        self.assertIn("x ${ \"y}\" } z", kept)
        self.assertNotIn("// c", kept)


class DeclarationTests(FixtureCase):
    def test_unknown_command_fails(self):
        f = self.fx(declarations=["login pending:P5-T4", "logout pending:P5-T4", "create_order pending:P5-T4", "bogus_cmd pending:P5-T4"])
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("unknown command bogus_cmd", out)

    def test_duplicate_declaration_fails(self):
        f = self.fx(declarations=["login pending:P5-T4", "login excluded again", "logout pending:P5-T4", "create_order pending:P5-T4"])
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("duplicate declaration of login", out)

    def test_unknown_status_fails(self):
        f = self.fx(declarations=["login done", "logout pending:P5-T4", "create_order pending:P5-T4"])
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("unknown status 'done' for login", out)

    def test_unknown_owner_fails(self):
        for owner in ("P6-T1", "P5b-nothing", "P5-T", "", "p5-t1"):
            f = self.fx(declarations=["login pending:%s" % owner, "logout pending:P5-T4", "create_order pending:P5b-webshop"])
            rc, out = f.run()
            self.assertEqual(rc, 1, owner)
            self.assertIn("unknown owner '%s' for login" % owner, out)

    def test_valid_owners_pass(self):
        owners = ["P5-T1", "P5-T12", "P5b-checkout", "P5b-reports", "P5b-settings", "P5b-webshop"]
        f = self.fx(commands=("a", "b", "c", "d", "e", "f"), declarations=["%s pending:%s" % (c, o) for c, o in zip("abcdef", owners)])
        rc, out = f.run()
        self.assertEqual(rc, 0, out)

    def test_malformed_line_fails(self):
        f = self.fx(declarations=["login", "logout pending:P5-T4", "create_order pending:P5-T4"])
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("malformed declaration", out)

    def test_excluded_command_with_call_site_fails(self):
        f = self.fx(declarations=["login excluded never", "logout pending:P5-T4", "create_order pending:P5-T4"])
        f.kotlin("A.kt", kotlin_call("login"))
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("login is excluded but has a call site", out)

    def test_missing_command_fails(self):
        f = self.fx(declarations=["login pending:P5-T4", "logout pending:P5-T4"])
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("UIPARITY | create_order | MISSING | no call site and no declaration", out)
        self.assertIn("missing 1", total_of(out))

    def test_called_command_needs_no_declaration(self):
        f = self.fx(declarations=["login pending:P5-T4", "logout pending:P5-T4"])
        f.kotlin("A.kt", kotlin_call("createOrder"))
        rc, out = f.run()
        self.assertEqual(rc, 0, out)

    def test_missing_block_or_file_fails(self):
        f = self.fx()
        f.put("docs/android/PARITY.md", "# no block here\n```text\nlogin pending:P5-T4\n```\n")
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("no ui-parity block", out)
        (f.dir / "docs/android/PARITY.md").unlink()
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("cannot read docs/android/PARITY.md", out)

    def test_lines_of_other_blocks_and_comments_are_ignored(self):
        f = self.fx()
        f.put(
            "docs/android/PARITY.md",
            "```text\nbogus x y\n```\n```ui-parity\n# bogus x y\n   \nlogin pending:P5-T4\nlogout pending:P5-T4\n"
            "create_order excluded note with  spaces\n```\n",
        )
        rc, out = f.run()
        self.assertEqual(rc, 0, out)
        self.assertIn("UIPARITY | create_order | EXCLUDED | note with  spaces", out)

    def test_line_formats_and_counts(self):
        f = self.fx(declarations=["login pending:P5-T4", "logout excluded gone", "create_order pending:P5b-reports"])
        rc, out = f.run()
        self.assertEqual(rc, 0, out)
        self.assertEqual(lines_of(out, "login"), ["UIPARITY | login | PENDING | P5-T4"])
        self.assertEqual(lines_of(out, "logout"), ["UIPARITY | logout | EXCLUDED | gone"])
        self.assertEqual(
            total_of(out), "UIPARITY | total | PASS | called 0, excluded 1, pending 2, stale 0, missing 0 (3 commands)"
        )
        names = [ln.split(" | ")[1] for ln in out.splitlines() if ln.count(" | ") == 3 and " | problem | " not in ln and " | total | " not in ln]
        self.assertEqual(names, sorted(names))


class StaleAndStrictTests(FixtureCase):
    def setUp(self):
        self.f = self.fx(declarations=["login pending:P5-T4", "logout pending:P5-T4", "create_order excluded no"])
        self.f.kotlin("A.kt", kotlin_call("login"))

    def test_stale_pending_is_a_warning_without_strict(self):
        rc, out = self.f.run()
        self.assertEqual(rc, 0, out)
        line = lines_of(out, "login")[0]
        self.assertIn("| CALLED |", line)
        self.assertTrue(line.endswith(" (stale pending:P5-T4)"), line)
        self.assertIn("called 1, excluded 1, pending 1, stale 1, missing 0", total_of(out))

    def test_stale_pending_fails_with_strict(self):
        rc, out = self.f.run("--strict")
        self.assertEqual(rc, 1)
        self.assertIn("| total | FAIL |", total_of(out))

    def test_pending_fails_with_strict_only(self):
        f = self.fx(declarations=["login pending:P5-T4", "logout excluded x", "create_order excluded y"])
        self.assertEqual(f.run()[0], 0)
        self.assertEqual(f.run("--strict")[0], 1)

    def test_strict_passes_when_everything_is_called_or_excluded(self):
        f = self.fx(declarations=["login excluded x"])
        f.kotlin("A.kt", kotlin_call("logout", "createOrder"))
        rc, out = f.run("--strict")
        self.assertEqual(rc, 0, out)
        self.assertIn("called 2, excluded 1, pending 0, stale 0, missing 0", total_of(out))


class TableTests(FixtureCase):
    TABLE = "| Vue | Compose | Commands |\n| --- | --- | --- |\n| a.vue | A.kt | %s |\n\n"

    def test_unknown_token_fails(self):
        f = self.fx(tables=self.TABLE % "`login` `no_such_cmd`")
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("table cites unknown command no_such_cmd", out)

    def test_uncalled_token_passes_without_strict_and_fails_with_it(self):
        f = self.fx(tables=self.TABLE % "`logout`", declarations=["login excluded x", "logout excluded y", "create_order excluded z"])
        self.assertEqual(f.run()[0], 0)
        rc, out = f.run("--strict")
        self.assertEqual(rc, 1)
        self.assertIn("table cites logout, which has no call site", out)

    def test_called_token_passes_with_strict(self):
        f = self.fx(tables=self.TABLE % "`logout`", declarations=["login excluded x", "create_order excluded z"])
        f.kotlin("A.kt", kotlin_call("logout"))
        rc, out = f.run("--strict")
        self.assertEqual(rc, 0, out)

    def test_dash_and_non_snake_tokens_are_ignored(self):
        f = self.fx(tables=self.TABLE % "- `PosEngine` `Foo_Bar`")
        rc, out = f.run()
        self.assertEqual(rc, 0, out)

    def test_other_tables_and_fenced_tables_are_ignored(self):
        other = "| Name | Commands |\n| --- | --- |\n| x | `bogus_one` |\n\n"
        fenced = "```\n| Vue | Compose | Commands |\n| - | - | - |\n| a | b | `bogus_two` |\n```\n"
        f = self.fx(tables=other + fenced)
        rc, out = f.run()
        self.assertEqual(rc, 0, out)

    def test_parity_directory_files_are_read(self):
        f = self.fx()
        f.put("docs/android/parity/login.md", "# Login\n\n" + self.TABLE % "`bogus_three`")
        rc, out = f.run()
        self.assertEqual(rc, 1)
        self.assertIn("docs/android/parity/login.md:5 table cites unknown command bogus_three", out)
        f.put("docs/android/parity/login.md", "# Login\n\n" + self.TABLE % "`login`")
        self.assertEqual(f.run()[0], 0)

    def test_table_ends_at_first_non_table_line(self):
        t = self.TABLE % "`login`" + "text\n| a.vue | A.kt | `bogus_four` |\n"
        f = self.fx(tables=t)
        self.assertEqual(f.run()[0], 0)


class RealRepositoryTests(unittest.TestCase):
    def test_real_repository_passes_at_head(self):
        proc = subprocess.run([sys.executable, str(SCRIPT), "--repo", str(ROOT)], capture_output=True, text=True)
        self.assertEqual(proc.returncode, 0, proc.stdout[-2000:])
        last = proc.stdout.strip().splitlines()[-1]
        self.assertTrue(last.startswith("UIPARITY | total | PASS | "), last)
        self.assertEqual([ln for ln in proc.stdout.splitlines() if "| problem |" in ln], [])

    def test_real_repository_fails_strict_while_commands_are_pending(self):
        proc = subprocess.run([sys.executable, str(SCRIPT), "--repo", str(ROOT), "--strict"], capture_output=True, text=True)
        text = proc.stdout
        pending = len([ln for ln in text.splitlines() if "| PENDING |" in ln])
        stale = len([ln for ln in text.splitlines() if "(stale pending:" in ln])
        if pending or stale:
            self.assertEqual(proc.returncode, 1)
        else:
            self.assertEqual(proc.returncode, 0, text[-2000:])


def main():
    stream = io.StringIO()
    suite = unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__])
    result = unittest.TextTestRunner(stream=stream, verbosity=0).run(suite)
    if not result.wasSuccessful():
        sys.stderr.write(stream.getvalue())
    n = result.testsRun
    status = "PASS" if result.wasSuccessful() else "FAIL"
    print("SELFTEST | total | %s | %d tests" % (status, n))
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main())
