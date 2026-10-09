#!/usr/bin/env python3
"""Validate the UC scenario files of the Android end-to-end lane (P7).

A scenario is data (docs/android/UC.md, P7.md section 4): a list of steps named like the Tauri
commands. This checker proves every file under the scenario root is well formed before any
interpreter (pos-uc-check on the host, the Kotlin runner on the device) reads it.

Output: one line per scenario file

    UCSCEN | <group>/<file> | PASS|FAIL | <detail>

then exactly one

    UCSCEN | total | PASS|FAIL | <n> failed (<m> scenarios)

Exit 0 on PASS, 1 on FAIL, 2 on a usage error.

Usage: scripts/android/uc_scenarios_check.py [--root DIR] [--repo DIR] [--list-ids]
  --root DIR   scenario root (default <repo>/android/app/src/androidTest/assets/uc)
  --repo DIR   repository root (default: found from the script location); fixtures, the seed
               src-tauri/seed_spf_test.sql, scripts/uc_matrix.py and docs/android/UC.md are read
               from it
  --list-ids   print the UC ids the valid scenarios cover, one per line, and exit

Standard library only.
"""

import argparse
import importlib.util
import json
import os
import re
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
DEFAULT_REPO = SCRIPT_DIR.parent.parent
DEFAULT_ROOT_REL = "android/app/src/androidTest/assets/uc"
SEED_REL = "src-tauri/seed_spf_test.sql"
UC_MATRIX_REL = "scripts/uc_matrix.py"
UC_MD_REL = "docs/android/UC.md"

# The vocabulary (P7.md section 4.3). docs/android/UC.md lists the same commands between
# UC-COMMANDS markers and this script refuses to run when the two lists differ. A command is
# added to both interpreters, to UC.md and here in one card.
ENGINE_COMMANDS = (
    "login",
    "logout",
    "set_training_mode",
    "work_in",
    "work_out",
    "end_shift",
    "create_order",
    "add_order_line",
    "add_order_line_with_modifiers",
    "update_order_line_quantity",
    "remove_order_line",
    "set_active_order",
    "leave_order",
    "sign_table_order",
    "sign_pre_bill",
    "checkout",
    "close_unpaid",
    "refund_checkout",
    "copy_ticket",
    "money_in_out",
    "open_drawer_fiscal",
    "generate_x_report",
    "generate_z_report",
    "generate_user_x_report",
    "check_fdm_status",
    "list_fiscal_events",
    "get_order",
)
MOCK_COMMANDS = ("fdm_control", "fdm_control_reset", "printer_control", "sleep_ms")
VOCABULARY = ENGINE_COMMANDS + MOCK_COMMANDS

SLEEP_MS_MAX = 10000
DASHES = (chr(0x2014), chr(0x2013))  # em-dash and en-dash: banned everywhere in the repository

GROUP_RE = re.compile(r"^\d{2}_[a-z][a-z0-9_]*$")
FILE_RE = re.compile(r"^(\d{3})_([A-Za-z0-9_]+)\.json$")
NAME_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
REF_RE = re.compile(r"^\$([A-Za-z_][A-Za-z0-9_]*)((?:\.[A-Za-z0-9_]+)+)$")
ERROR_CODE_RE = re.compile(r"^[A-Za-z][A-Za-z0-9_]*$")
TOP_KEYS = {"format", "id", "ids", "title", "fixture", "fixtureDeviations", "compare", "steps"}
STEP_KEYS = {"cmd", "args", "save", "expect", "repeat"}

# Keys whose values must exist in the seed (P7.md section 4.2 checks, UC.md).
INSZ_KEYS = ("insz", "user_insz", "employeeInsz")
MENU_ITEM_KEYS = ("menu_item_id", "menuItemId")


# ───────────────────────────── argument specs ─────────────────────────────
# Spec tuples: ("int",) ("str",) ("bool",) ("num",) ("enum", values) ("null", spec) for a value
# that may also be null, ("list", spec) and ("obj", {key: (spec, required)}). A reference
# ("$name.path") is accepted wherever a value is expected: its type is only known at run time.

INT = ("int",)
STR = ("str",)
BOOL = ("bool",)
NUM = ("num",)
ORDER_TYPES = ("dine_in", "takeaway")
LEAVE_REASONS = ("navigate", "switch_order", "send_to_kitchen", "lock", "idle")
PAYMENT_METHODS = ("cash", "card", "meal_voucher", "app", "online")
MONEY_DIRECTIONS = ("in", "out", "declaration")
DELIVERY = (
    "obj",
    {
        "medium": (("enum", ("paper", "digital", "paper_digital")), True),
        "email": (("null", STR), False),
    },
)
PAYMENT = (
    "obj",
    {
        "method": (("enum", PAYMENT_METHODS), True),
        "amount_cents": (("null", INT), False),
    },
)
REFUND_ITEM = (
    "obj",
    {
        "menuItemId": (INT, True),
        "quantity": (INT, True),
        "lineId": (("null", INT), False),
    },
)


def _args(**keys):
    """Top-level args of a command: key -> (spec, required)."""
    return keys


def _req(spec):
    return (spec, True)


def _opt(spec):
    return (spec, False)


ENGINE_ARGS = {
    "login": _args(insz=_req(STR), pin=_opt(("null", STR))),
    "logout": _args(),
    "set_training_mode": _args(enabled=_req(BOOL)),
    "work_in": _args(employeeInsz=_req(STR)),
    "work_out": _args(employeeInsz=_req(STR)),
    "end_shift": _args(insz=_req(STR), pin=_opt(("null", STR))),
    "create_order": _args(
        input=_req(
            (
                "obj",
                {
                    "order_type": (("enum", ORDER_TYPES), True),
                    "table_number": (("null", INT), True),
                    "user_insz": (STR, True),
                },
            )
        )
    ),
    "add_order_line": _args(
        input=_req(
            (
                "obj",
                {
                    "order_id": (INT, True),
                    "menu_item_id": (INT, True),
                    "quantity": (INT, True),
                    "modifiers_json": (("null", STR), False),
                    "unit_price_cents": (("null", INT), False),
                },
            )
        )
    ),
    "add_order_line_with_modifiers": _args(
        input=_req(
            (
                "obj",
                {
                    "order_id": (INT, True),
                    "menu_item_id": (INT, True),
                    "quantity": (INT, True),
                    "modifier_ids": (("list", INT), True),
                },
            )
        )
    ),
    "update_order_line_quantity": _args(lineId=_req(INT), quantity=_req(INT)),
    "remove_order_line": _args(lineId=_req(INT)),
    "set_active_order": _args(orderId=_opt(("null", INT))),
    "leave_order": _args(orderId=_req(INT), reason=_req(("enum", LEAVE_REASONS))),
    "sign_table_order": _args(orderId=_req(INT)),
    "sign_pre_bill": _args(orderId=_req(INT)),
    "checkout": _args(
        orderId=_req(INT),
        payments=_req(("list", PAYMENT)),
        delivery=_opt(("null", DELIVERY)),
    ),
    "close_unpaid": _args(orderId=_req(INT), insz=_req(STR), pin=_req(STR)),
    "refund_checkout": _args(
        originalFiscalEventId=_req(INT),
        items=_req(("list", REFUND_ITEM)),
        employeeInsz=_req(STR),
        delivery=_opt(("null", DELIVERY)),
    ),
    "copy_ticket": _args(
        fiscalEventId=_req(INT),
        employeeInsz=_req(STR),
        delivery=_opt(("null", DELIVERY)),
    ),
    "money_in_out": _args(
        employeeInsz=_req(STR),
        amountCents=_req(INT),
        direction=_req(("enum", MONEY_DIRECTIONS)),
        reason=_req(STR),
    ),
    "open_drawer_fiscal": _args(employeeInsz=_req(STR)),
    "generate_x_report": _args(employeeInsz=_req(STR)),
    "generate_z_report": _args(employeeInsz=_req(STR)),
    "generate_user_x_report": _args(employeeInsz=_req(STR)),
    "check_fdm_status": _args(),
    "list_fiscal_events": _args(limit=_opt(("null", INT))),
    "get_order": _args(orderId=_req(INT)),
}

# pos-fdm-mock `Control` is serde camelCase: the wire names differ from the Rust field names that
# P7.md section 4.3 lists. A snake_case key would be ignored by the mock without any error.
FDM_CONTROL_KEYS = {
    "errors": ("list", STR),
    "warnings": ("list", STR),
    "bufferPct": ("null", NUM),
    "initialized": ("null", BOOL),
    "token": ("null", STR),
    "allowedPos": ("null", ("list", STR)),
    "placeholderSignatures": BOOL,
}
FDM_CONTROL_SNAKE = {
    "buffer_pct": "bufferPct",
    "allowed_pos": "allowedPos",
    "placeholder_signatures": "placeholderSignatures",
}
PRINTER_CONTROL_KEYS = {
    "paper": ("enum", ("ok", "near_end", "end")),
    "coverOpen": BOOL,
    "error": BOOL,
    "unplugged": BOOL,
}
PRINTER_CONTROL_SNAKE = {"cover_open": "coverOpen"}


def is_ref(value):
    return isinstance(value, str) and value.startswith("$")


def type_name(value):
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, int):
        return "integer"
    if isinstance(value, float):
        return "number"
    if isinstance(value, str):
        return "string"
    if isinstance(value, list):
        return "array"
    return "object"


def check_value(value, spec, path, errs):
    """Append to errs what is wrong with value against spec."""
    if is_ref(value):
        return  # resolved at run time; its form is checked by the reference walk
    kind = spec[0]
    if kind == "null":
        if value is not None:
            check_value(value, spec[1], path, errs)
        return
    if kind == "int":
        if not isinstance(value, int) or isinstance(value, bool):
            errs.append(f"{path} must be an integer, got {type_name(value)}")
    elif kind == "num":
        if not isinstance(value, (int, float)) or isinstance(value, bool):
            errs.append(f"{path} must be a number, got {type_name(value)}")
    elif kind == "str":
        if not isinstance(value, str):
            errs.append(f"{path} must be a string, got {type_name(value)}")
    elif kind == "bool":
        if not isinstance(value, bool):
            errs.append(f"{path} must be a boolean, got {type_name(value)}")
    elif kind == "enum":
        if value not in spec[1]:
            errs.append(f"{path} must be one of {', '.join(spec[1])}, got {json.dumps(value)}")
    elif kind == "list":
        if not isinstance(value, list):
            errs.append(f"{path} must be an array, got {type_name(value)}")
            return
        for i, item in enumerate(value):
            check_value(item, spec[1], f"{path}[{i}]", errs)
    elif kind == "obj":
        check_object(value, spec[1], path, errs)


def check_object(value, keys, path, errs):
    """keys: key -> (spec, required)."""
    if not isinstance(value, dict):
        errs.append(f"{path} must be an object, got {type_name(value)}")
        return
    for key in value:
        if key not in keys:
            errs.append(f"{path}: unknown key {key!r}")
    for key, (spec, required) in keys.items():
        if key not in value:
            if required:
                errs.append(f"{path}: missing key {key!r}")
            continue
        check_value(value[key], spec, f"{path}.{key}", errs)


def check_mock_args(cmd, args, errs):
    path = f"{cmd} args"
    if cmd == "fdm_control_reset":
        if args != {}:
            errs.append(f"{path} must be {{}}")
    elif cmd == "sleep_ms":
        if set(args) != {"ms"}:
            errs.append(f"{path} must be exactly {{\"ms\": N}}")
        elif not is_ref(args["ms"]):
            ms = args["ms"]
            if not isinstance(ms, int) or isinstance(ms, bool):
                errs.append(f"{path}.ms must be an integer, got {type_name(ms)}")
            elif not 0 <= ms <= SLEEP_MS_MAX:
                errs.append(f"{path}.ms must be between 0 and {SLEEP_MS_MAX}, got {ms}")
    else:
        keys, snake = (
            (FDM_CONTROL_KEYS, FDM_CONTROL_SNAKE)
            if cmd == "fdm_control"
            else (PRINTER_CONTROL_KEYS, PRINTER_CONTROL_SNAKE)
        )
        for key, value in args.items():
            if key in snake:
                errs.append(f"{path}: key {key!r} is ignored by the mock, write {snake[key]!r} (camelCase)")
            elif key not in keys:
                errs.append(f"{path}: unknown key {key!r}")
            else:
                check_value(value, keys[key], f"{path}.{key}", errs)


# ───────────────────────────── repository data ─────────────────────────────


def load_uc_matrix(repo):
    """Import scripts/uc_matrix.py without running it (no edit, no side effect)."""
    path = Path(repo) / UC_MATRIX_REL
    if not path.is_file():
        raise FileNotFoundError(f"{UC_MATRIX_REL} not found under {repo}")
    spec = importlib.util.spec_from_file_location("uc_matrix_for_uc_scenarios_check", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module  # dataclasses look the module up by name
    try:
        spec.loader.exec_module(module)
    except Exception:
        sys.modules.pop(spec.name, None)
        raise
    return module


def known_ids(repo):
    """Every UC id uc_matrix.py knows: the three official sets, T1..T8, ROUND, V_, VI_, FAQ_."""
    um = load_uc_matrix(repo)
    root = Path(repo)
    items = []
    items += um.collect_basic_set_items(root)
    items += um.collect_reports_items(root)
    items += um.collect_advanced_set_items(root)
    items += list(getattr(um, "MANUAL_ITEMS", []))
    items += list(getattr(um, "FAQ_ITEMS", []))
    return {item.id for item in items}


def _strip_sql_comments(text):
    return "\n".join(line.split("--", 1)[0] for line in text.splitlines())


def _values_block(sql, table, columns_hint):
    """The text of `INSERT ... INTO table (...) VALUES ...;` statements, comments removed."""
    pattern = re.compile(
        r"INSERT\s+(?:OR\s+\w+\s+)?INTO\s+" + re.escape(table) + r"\s*\([^)]*" + columns_hint + r"[^)]*\)\s*VALUES(.*?);",
        re.IGNORECASE | re.DOTALL,
    )
    return "\n".join(m.group(1) for m in pattern.finditer(sql))


def seed_data(repo):
    """(menu item ids, user inszs) from src-tauri/seed_spf_test.sql."""
    path = Path(repo) / SEED_REL
    if not path.is_file():
        raise FileNotFoundError(f"{SEED_REL} not found under {repo}")
    sql = _strip_sql_comments(path.read_text(encoding="utf-8"))
    menu = {int(m) for m in re.findall(r"\(\s*(\d+)\s*,", _values_block(sql, "menu_items", r"\bid\b"))}
    users = set(re.findall(r"\(\s*'(\d{11})'\s*,", _values_block(sql, "users", r"\binsz\b")))
    if not menu or not users:
        raise ValueError(f"no menu items or no users found in {SEED_REL}")
    return menu, users


def uc_md_commands(repo):
    """The command lists UC.md declares: {'engine': [...], 'mock': [...]} (None when absent).

    Each list is read from the first column of the tables between
    <!-- UC-COMMANDS engine --> ... <!-- /UC-COMMANDS --> markers (one or more blocks).
    """
    path = Path(repo) / UC_MD_REL
    if not path.is_file():
        return None
    text = path.read_text(encoding="utf-8")
    found = {"engine": [], "mock": []}
    pattern = re.compile(r"<!--\s*UC-COMMANDS\s+(engine|mock)(?:\s+\w+)?\s*-->(.*?)<!--\s*/UC-COMMANDS\s*-->", re.DOTALL)
    for m in pattern.finditer(text):
        group = m.group(1)
        names = []
        for line in m.group(2).splitlines():
            cell = re.match(r"^\|\s*`([a-z][a-z0-9_]*)`\s*\|", line)
            if cell:
                names.append(cell.group(1))
        found[group].append(names)
    return found


def check_uc_md(repo):
    """Empty string when UC.md and this script list the same vocabulary, else the problem."""
    found = uc_md_commands(repo)
    if found is None:
        return f"{UC_MD_REL} not found under {repo}"
    problems = []
    for group, expected in (("engine", ENGINE_COMMANDS), ("mock", MOCK_COMMANDS)):
        blocks = found[group]
        if not blocks:
            problems.append(f"no UC-COMMANDS {group} block")
            continue
        for names in blocks:
            if tuple(names) != tuple(expected):
                missing = [c for c in expected if c not in names]
                extra = [c for c in names if c not in expected]
                detail = []
                if missing:
                    detail.append("missing " + ", ".join(missing))
                if extra:
                    detail.append("unknown " + ", ".join(extra))
                if not detail:
                    detail.append("different order")
                problems.append(f"{group} table differs from the script ({'; '.join(detail)})")
    return "; ".join(problems)


# ───────────────────────────── scenario checks ─────────────────────────────


class DuplicateKey(ValueError):
    pass


def _no_duplicates(pairs):
    seen = {}
    for key, value in pairs:
        if key in seen:
            raise DuplicateKey(key)
        seen[key] = value
    return seen


def load_json(path):
    """(data, error)."""
    try:
        raw = path.read_bytes()
    except OSError as e:
        return None, f"cannot read: {e}"
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as e:
        return None, f"not UTF-8: {e}"
    if text.startswith(chr(0xFEFF)):
        return None, "starts with a byte order mark"
    try:
        return json.loads(text, object_pairs_hook=_no_duplicates), None
    except DuplicateKey as e:
        return None, f"duplicate key {e}"
    except ValueError as e:
        return None, f"invalid JSON: {e}"


def walk_strings(value, path=""):
    """(path, string) for every string value, keys excluded."""
    if isinstance(value, str):
        yield path, value
    elif isinstance(value, list):
        for i, item in enumerate(value):
            yield from walk_strings(item, f"{path}[{i}]")
    elif isinstance(value, dict):
        for key, item in value.items():
            yield from walk_strings(item, f"{path}.{key}" if path else key)


def walk_keys(value, wanted, path=""):
    """(path, key, value) for every dict entry whose key is in wanted."""
    if isinstance(value, list):
        for i, item in enumerate(value):
            yield from walk_keys(item, wanted, f"{path}[{i}]")
    elif isinstance(value, dict):
        for key, item in value.items():
            sub = f"{path}.{key}" if path else key
            if key in wanted:
                yield sub, key, item
            yield from walk_keys(item, wanted, sub)


def check_expect(expect, errs, where):
    if not isinstance(expect, dict):
        errs.append(f"{where}: expect must be an object")
        return
    for key in expect:
        if key not in ("fields", "error"):
            errs.append(f"{where}: unknown key {key!r} in expect")
    if "fields" in expect:
        fields = expect["fields"]
        if not isinstance(fields, dict) or not fields:
            errs.append(f"{where}: expect.fields must be a non-empty object")
        else:
            for key in fields:
                if not key or any(not seg for seg in key.split(".")):
                    errs.append(f"{where}: expect.fields has a bad path {key!r}")
    if "error" in expect:
        code = expect["error"]
        if not isinstance(code, str) or not ERROR_CODE_RE.match(code):
            errs.append(f"{where}: expect.error must be an error code (letters, digits, underscore)")


def check_step(step, index, saved, ctx, errs):
    where = f"step {index + 1}"
    if not isinstance(step, dict):
        errs.append(f"{where}: must be an object")
        return
    for key in step:
        if key not in STEP_KEYS:
            errs.append(f"{where}: unknown key {key!r}")
    cmd = step.get("cmd")
    if not isinstance(cmd, str):
        errs.append(f"{where}: cmd is required and must be a string")
        return
    where = f"step {index + 1} ({cmd})"
    if cmd not in VOCABULARY:
        errs.append(f"{where}: unknown command {cmd!r}")
        return
    if "args" not in step:
        errs.append(f"{where}: args is required (an object)")
        args = {}
    elif not isinstance(step["args"], dict):
        errs.append(f"{where}: args must be an object")
        args = {}
    else:
        args = step["args"]
    if "save" in step:
        name = step["save"]
        if not isinstance(name, str) or not NAME_RE.match(name):
            errs.append(f"{where}: save must be a name (letters, digits, underscore)")
    if "repeat" in step:
        n = step["repeat"]
        if not isinstance(n, int) or isinstance(n, bool) or n < 1:
            errs.append(f"{where}: repeat must be a positive integer")
    if "expect" in step:
        check_expect(step["expect"], errs, where)

    if cmd in MOCK_COMMANDS:
        for key in ("save", "expect"):
            if key in step:
                errs.append(f"{where}: a mock command takes no {key}")
        if isinstance(step.get("args"), dict):
            local = []
            check_mock_args(cmd, args, local)
            errs.extend(f"{where}: {e}" for e in local)
    elif isinstance(step.get("args"), dict):
        local = []
        check_object(args, ENGINE_ARGS[cmd], "args", local)
        errs.extend(f"{where}: {e}" for e in local)

    # References: a whole-string "$name.path" must name a result saved by an earlier step.
    body = {k: step[k] for k in ("args", "expect") if k in step}
    for path, text in walk_strings(body):
        if not text.startswith("$"):
            continue
        m = REF_RE.match(text)
        if not m:
            errs.append(f"{where}: malformed reference {text!r} at {path} (form $name.path)")
        elif m.group(1) not in saved:
            errs.append(f"{where}: reference {text!r} names {m.group(1)!r}, which no earlier step saves")

    # Seed data.
    if isinstance(step.get("args"), dict):
        for path, key, value in walk_keys(args, INSZ_KEYS):
            if isinstance(value, str) and not is_ref(value) and value not in ctx["users"]:
                errs.append(f"{where}: {key} {value!r} is not a user of {SEED_REL}")
        for path, key, value in walk_keys(args, MENU_ITEM_KEYS):
            if isinstance(value, int) and not isinstance(value, bool) and value not in ctx["menu"]:
                errs.append(f"{where}: {key} {value} is not a menu item of {SEED_REL}")

    if isinstance(step.get("save"), str):
        saved.add(step["save"])


def check_fixture(data, repo, errs):
    fixture = data.get("fixture")
    deviations = data.get("fixtureDeviations")
    if fixture is not None:
        if not isinstance(fixture, str) or not fixture:
            errs.append("fixture must be a directory path (string)")
        else:
            repo_real = os.path.realpath(repo)
            target = os.path.realpath(os.path.join(repo_real, fixture))
            if os.path.isabs(fixture) or not (target == repo_real or target.startswith(repo_real + os.sep)):
                errs.append(f"fixture {fixture!r} must be a relative path inside the repository")
            elif not os.path.isdir(target):
                errs.append(f"fixture directory {fixture!r} does not exist")
            elif not any(n.endswith("GQL-Variables.json") for n in os.listdir(target)):
                errs.append(f"fixture directory {fixture!r} holds no *GQL-Variables.json file")
    if deviations is not None:
        if not isinstance(deviations, list):
            errs.append("fixtureDeviations must be an array")
        else:
            if deviations and fixture is None:
                errs.append("fixtureDeviations without a fixture")
            for i, dev in enumerate(deviations):
                if not isinstance(dev, dict):
                    errs.append(f"fixtureDeviations[{i}] must be an object")
                    continue
                for key in dev:
                    if key not in ("path", "reason"):
                        errs.append(f"fixtureDeviations[{i}]: unknown key {key!r}")
                for key in ("path", "reason"):
                    value = dev.get(key)
                    if not isinstance(value, str) or not value.strip():
                        errs.append(f"fixtureDeviations[{i}] needs a non-empty {key}")


def check_scenario(data, file_id, ctx, repo):
    """Every problem of one parsed scenario, as a list of strings."""
    errs = []
    if not isinstance(data, dict):
        return ["top level must be an object"]
    for key in data:
        if key not in TOP_KEYS:
            errs.append(f"unknown top-level key {key!r}")

    fmt = data.get("format")
    if "format" not in data:
        errs.append("missing key 'format'")
    elif fmt != 1 or isinstance(fmt, bool):
        errs.append(f"format must be 1, got {json.dumps(fmt)}")

    sid = data.get("id")
    if not isinstance(sid, str) or not sid:
        errs.append("id is required and must be a non-empty string")
    elif sid != file_id:
        errs.append(f"id {sid!r} differs from the file name id {file_id!r}")

    ids = data.get("ids")
    if not isinstance(ids, list) or any(not isinstance(i, str) or not i for i in ids):
        errs.append("ids is required and must be an array of non-empty strings")
        ids = None
    elif len(set(ids)) != len(ids):
        errs.append("ids has a duplicate")
    if ids is not None:
        if not ids and file_id != "SMOKE":
            errs.append("ids is empty (only SMOKE may cover no UC id)")
        if ids and isinstance(sid, str) and sid and sid not in ids:
            errs.append(f"id {sid!r} is not in ids")
    mentioned = list(ids or [])
    if isinstance(sid, str) and sid and sid != "SMOKE":
        mentioned.append(sid)
    for uc in dict.fromkeys(mentioned):
        if uc not in ctx["ids"]:
            errs.append(f"unknown UC id {uc!r} (not known to scripts/uc_matrix.py)")

    title = data.get("title")
    if not isinstance(title, str) or not title.strip():
        errs.append("title is required and must be a non-empty string")

    check_fixture(data, repo, errs)

    if "compare" in data:
        compare = data["compare"]
        if not isinstance(compare, dict):
            errs.append("compare must be an object")
        else:
            for key in compare:
                if key != "status":
                    errs.append(f"compare: unknown key {key!r}")
            if "status" in compare and compare["status"] not in ("ignore", "shape"):
                errs.append("compare.status must be 'ignore' or 'shape'")

    steps = data.get("steps")
    if not isinstance(steps, list) or not steps:
        errs.append("steps is required and must be a non-empty array")
    else:
        saved = set()
        for i, step in enumerate(steps):
            check_step(step, i, saved, ctx, errs)

    for path, text in walk_strings(data):
        if any(d in text for d in DASHES):
            errs.append(f"an em-dash or en-dash at {path or 'top level'}")
            break
    return errs


# ───────────────────────────── the run ─────────────────────────────


def scan_layout(root):
    """(entries, problems): entries are (group, filename, path); problems are (label, message)."""
    entries = []
    problems = []
    for group_dir in sorted(p for p in root.iterdir() if not p.name.startswith(".")):
        if not group_dir.is_dir():
            problems.append((group_dir.name, "a file at the scenario root (only <NN>_<group> directories belong here)"))
            continue
        if not GROUP_RE.match(group_dir.name):
            problems.append((group_dir.name + "/", "group directory must be named <NN>_<group> (two digits, lowercase)"))
            continue
        for item in sorted(p for p in group_dir.iterdir() if not p.name.startswith(".")):
            label = f"{group_dir.name}/{item.name}"
            if item.is_dir():
                problems.append((label, "a directory inside a group"))
            elif not FILE_RE.match(item.name):
                problems.append((label, "file must be named <NNN>_<ID>.json"))
            else:
                entries.append((group_dir.name, item.name, item))
    return entries, problems


def run(root, repo, list_ids=False, out=None):
    """Run the check. Returns the process exit code."""
    out = out or sys.stdout
    root = Path(root)
    repo = Path(repo)
    try:
        ctx = {"ids": known_ids(repo)}
        ctx["menu"], ctx["users"] = seed_data(repo)
    except (OSError, ValueError, ImportError, AttributeError) as e:
        print(f"uc_scenarios_check: {e}", file=sys.stderr)
        return 2

    entries, layout_problems = scan_layout(root)
    results = []  # (label, [errors]) in the order printed
    covered = set()

    md_problem = check_uc_md(repo)
    if md_problem:
        results.append((UC_MD_REL, [md_problem]))

    for label, message in layout_problems:
        results.append((label, [message]))

    seen_numbers = {}
    for group, name, path in entries:
        m = FILE_RE.match(name)
        number, file_id = m.group(1), m.group(2)
        label = f"{group}/{name}"
        errs = []
        key = (group, number)
        if key in seen_numbers:
            errs.append(f"number {number} is already used by {seen_numbers[key]}")
        else:
            seen_numbers[key] = name
        data, err = load_json(path)
        if err:
            errs.append(err)
        else:
            errs.extend(check_scenario(data, file_id, ctx, repo))
            if not errs and isinstance(data.get("ids"), list):
                covered.update(data["ids"])
        results.append((label, errs))

    if not entries and not layout_problems:
        results.append((str(root), ["no scenario found"]))

    if list_ids:
        for uc in sorted(covered):
            print(uc, file=out)
        return 0

    failed = 0
    scenarios = len(entries)
    for label, errs in results:
        if errs:
            failed += 1
            print(f"UCSCEN | {label} | FAIL | " + "; ".join(errs), file=out)
        else:
            print(f"UCSCEN | {label} | PASS | ok", file=out)
    verdict = "FAIL" if failed else "PASS"
    print(f"UCSCEN | total | {verdict} | {failed} failed ({scenarios} scenarios)", file=out)
    return 1 if failed else 0


def main(argv=None):
    parser = argparse.ArgumentParser(description="Validate the UC scenario files.", add_help=True)
    parser.add_argument("--root", help="scenario root directory")
    parser.add_argument("--repo", help="repository root", default=str(DEFAULT_REPO))
    parser.add_argument("--list-ids", action="store_true", help="print the covered UC ids")
    try:
        args = parser.parse_args(argv)
    except SystemExit as e:
        return 2 if e.code not in (0, None) else 0
    repo = Path(args.repo)
    if not repo.is_dir():
        print(f"uc_scenarios_check: repository root {repo} is not a directory", file=sys.stderr)
        return 2
    root = Path(args.root) if args.root else repo / DEFAULT_ROOT_REL
    if not root.is_dir():
        print(f"uc_scenarios_check: scenario root {root} is not a directory", file=sys.stderr)
        return 2
    return run(root, repo, list_ids=args.list_ids)


if __name__ == "__main__":
    sys.exit(main())
