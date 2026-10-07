#!/usr/bin/env python3
"""Check that every Tauri command wrapper is a single call into the engine.

Default run: parse src-tauri/src/ipc/*.rs, require each #[tauri::command]
function to hold exactly one call whose path ends in
commands::<module>::<fn>, whose first argument is exactly `state.inner()`
and whose other arguments use the allowed forms, and require the wrapper
names to equal each tauri::generate_handler! list of src-tauri/src/main.rs.

  --map            TSV: command, module, engine_fn, async|sync (sorted)
  --signatures DIR one normalised line per #[tauri::command] fn of DIR/*.rs
                   (proves the IPC contract did not change)

Standard library only.
"""

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
IPC_DIR = ROOT / "src-tauri" / "src" / "ipc"
MAIN_RS = ROOT / "src-tauri" / "src" / "main.rs"

IDENT = r"[A-Za-z_][A-Za-z0-9_]*"
CALL_PREFIXES = ("crate::commands::", "pos_engine::commands::", "commands::")
# p, &p, p.as_ref(), p.as_deref(), p.f, &p.f, p.f.as_ref(), p.f.as_deref()
ARG_RES = [
    re.compile(rf"^{IDENT}$"),
    re.compile(rf"^&{IDENT}$"),
    re.compile(rf"^{IDENT}\.as_ref\(\)$"),
    re.compile(rf"^{IDENT}\.as_deref\(\)$"),
    re.compile(rf"^{IDENT}\.{IDENT}$"),
    re.compile(rf"^&{IDENT}\.{IDENT}$"),
    re.compile(rf"^{IDENT}\.{IDENT}\.as_ref\(\)$"),
    re.compile(rf"^{IDENT}\.{IDENT}\.as_deref\(\)$"),
]
FN_RE = re.compile(rf"(?:pub\s+)?(async\s+)?fn\s+({IDENT})\s*(?:<[^>]*>)?\s*\(")


class Wrapper:
    def __init__(self, path, line, name, is_async, params, ret, body, module):
        self.path = path
        self.line = line
        self.name = name
        self.is_async = is_async
        self.params = params
        self.ret = ret
        self.body = body
        self.module = module


def strip_comments(text):
    """Blank out comments and string contents, keeping offsets and newlines."""
    out = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            while i < n and text[i] != "\n":
                out.append(" ")
                i += 1
        elif c == "/" and nxt == "*":
            depth = 0
            while i < n:
                if text.startswith("/*", i):
                    depth += 1
                    out.append("  ")
                    i += 2
                elif text.startswith("*/", i):
                    depth -= 1
                    out.append("  ")
                    i += 2
                    if depth == 0:
                        break
                else:
                    out.append("\n" if text[i] == "\n" else " ")
                    i += 1
        elif c == '"':
            out.append('"')
            i += 1
            while i < n and text[i] != '"':
                if text[i] == "\\" and i + 1 < n:
                    out.append("  ")
                    i += 2
                else:
                    out.append("\n" if text[i] == "\n" else "_")
                    i += 1
            out.append('"')
            i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def match_close(text, start, open_ch, close_ch):
    """Index of the bracket closing the one at text[start]."""
    depth = 0
    for i in range(start, len(text)):
        if text[i] == open_ch:
            depth += 1
        elif text[i] == close_ch:
            depth -= 1
            if depth == 0:
                return i
    raise ValueError("unbalanced bracket")


def split_top(text, sep=","):
    """Split on `sep` outside (), [], {} and <> (generics)."""
    parts = []
    depth = 0
    cur = []
    for i, c in enumerate(text):
        if c in "([{<":
            depth += 1
        elif c in ")]}>":
            if c == ">" and i > 0 and text[i - 1] == "-":
                pass  # the arrow of a return type
            else:
                depth -= 1
        if c == sep and depth == 0:
            parts.append("".join(cur))
            cur = []
        else:
            cur.append(c)
    tail = "".join(cur)
    if tail.strip():
        parts.append(tail)
    return parts


def collapse(s):
    return re.sub(r"\s+", " ", s).strip()


def parse_wrappers(path):
    """Every #[tauri::command] fn of one file."""
    raw = path.read_text()
    text = strip_comments(raw)
    wrappers = []
    for m in re.finditer(r"#\[\s*tauri::command[^\]]*\]", text):
        fm = FN_RE.search(text, m.end())
        if not fm:
            continue
        # Only attributes or whitespace may sit between the attribute and fn.
        between = text[m.end() : fm.start()]
        if re.sub(r"#\[[^\]]*\]|\s", "", between):
            continue
        open_paren = fm.end() - 1
        close_paren = match_close(text, open_paren, "(", ")")
        brace = text.index("{", close_paren)
        header_tail = text[close_paren + 1 : brace]
        ret = ""
        if "->" in header_tail:
            ret = collapse(header_tail.split("->", 1)[1])
        end = match_close(text, brace, "{", "}")
        body = text[brace + 1 : end]
        params = [collapse(p) for p in split_top(text[open_paren + 1 : close_paren])]
        line = text.count("\n", 0, fm.start()) + 1
        wrappers.append(
            Wrapper(path, line, fm.group(2), bool(fm.group(1)), params, ret, body, path.stem)
        )
    return wrappers


def strip_ws_call(body):
    return collapse(body)


def check_wrapper(w):
    """Return (problem or None, engine path, engine module, engine fn)."""
    body = strip_ws_call(w.body)
    body = body.rstrip(";").strip() if body.endswith(";") else body
    awaited = False
    if body.endswith(".await"):
        awaited = True
        body = body[: -len(".await")].rstrip()
    # An async wrapper may call a sync engine function (it then runs on the
    # async runtime, as before), so `.await` is optional there.
    if awaited and not w.is_async:
        return ".await in a sync wrapper", None, None, None
    m = re.match(rf"^((?:{IDENT}::)+{IDENT})\s*\(", body)
    if not m:
        return "body is not a single call to commands::<module>::<fn>", None, None, None
    call_path = m.group(1)
    open_paren = m.end() - 1
    try:
        close = match_close(body, open_paren, "(", ")")
    except ValueError:
        return "unbalanced call", None, None, None
    if body[close + 1 :].strip():
        return "body holds more than one expression", None, None, None
    prefix = next((p for p in CALL_PREFIXES if call_path.startswith(p)), None)
    if prefix is None:
        return f"call path {call_path} does not end in commands::<module>::<fn>", None, None, None
    rest = call_path[len(prefix) :].split("::")
    if len(rest) != 2:
        return f"call path {call_path} is not commands::<module>::<fn>", None, None, None
    module, engine_fn = rest
    args = [collapse(a) for a in split_top(body[open_paren + 1 : close])]
    if not args or args[0] != "state.inner()":
        return "first argument must be exactly state.inner()", None, None, None
    for a in args[1:]:
        if not any(r.match(a) for r in ARG_RES):
            return f"argument `{a}` is not an allowed form", None, None, None
    return None, call_path, module, engine_fn


def read_handler_lists():
    """Each generate_handler! list of main.rs as a list of (module, cmd)."""
    text = strip_comments(MAIN_RS.read_text())
    lists = []
    for m in re.finditer(r"generate_handler!\s*\[", text):
        start = m.end() - 1
        end = match_close(text, start, "[", "]")
        entries = [collapse(e) for e in split_top(text[start + 1 : end])]
        lists.append([e for e in entries if e])
    return lists


def peppol_modules():
    """Modules of ipc/mod.rs gated by the peppol feature."""
    # Not strip_comments: it blanks the "peppol" string literal.
    text = "\n".join(
        line for line in (IPC_DIR / "mod.rs").read_text().split("\n") if not line.lstrip().startswith("//")
    )
    return set(
        re.findall(rf'#\[cfg\(feature = "peppol"\)\]\s*pub\s+mod\s+({IDENT})\s*;', text)
    )


def run_check():
    problems = []
    wrappers = []
    for f in sorted(IPC_DIR.glob("*.rs")):
        wrappers.extend(parse_wrappers(f))
    for w in wrappers:
        problem, *_ = check_wrapper(w)
        if problem:
            problems.append(f"{w.path.relative_to(ROOT)}:{w.line}: {w.name}: {problem}")
    names = {}
    for w in wrappers:
        key = (w.module, w.name)
        if key in names:
            problems.append(f"{w.path.relative_to(ROOT)}:{w.line}: {w.name}: duplicate wrapper")
        names[key] = w
    peppol_mods = peppol_modules()
    all_set = set(names)
    base_set = {k for k in names if k[0] not in peppol_mods}
    lists = read_handler_lists()
    if not lists:
        problems.append(f"{MAIN_RS.relative_to(ROOT)}:1: no generate_handler! list found")
    variants = []
    for n, entries in enumerate(lists, 1):
        listed = []
        for e in entries:
            m = re.match(rf"^ipc::({IDENT})::({IDENT})$", e)
            if not m:
                problems.append(
                    f"{MAIN_RS.relative_to(ROOT)}:1: handler list {n}: entry `{e}` is not ipc::<module>::<command>"
                )
                continue
            listed.append((m.group(1), m.group(2)))
        if len(listed) != len(set(listed)):
            problems.append(f"{MAIN_RS.relative_to(ROOT)}:1: handler list {n} has a duplicate entry")
        listed_set = set(listed)
        # A list is either every wrapper (peppol build) or every wrapper
        # outside the peppol-only modules (build without peppol).
        expected = all_set if listed_set & (all_set - base_set) else base_set
        variants.append(expected is all_set)
        for key in sorted(listed_set - expected):
            problems.append(
                f"{MAIN_RS.relative_to(ROOT)}:1: handler list {n}: ipc::{key[0]}::{key[1]} has no wrapper in src-tauri/src/ipc"
            )
        for key in sorted(expected - listed_set):
            w = names[key]
            problems.append(
                f"{w.path.relative_to(ROOT)}:{w.line}: {w.name}: missing from handler list {n} of main.rs"
            )
    if peppol_mods and lists and sorted(variants) != [False, True]:
        problems.append(
            f"{MAIN_RS.relative_to(ROOT)}:1: expected one handler list with and one without the peppol commands"
        )
    if problems:
        for p in problems:
            print(p)
        return 1
    total = len(wrappers)
    without = len([k for k in names if k[0] not in peppol_mods])
    ok = total - len(problems)
    print(f"commands: {total} ({without} without peppol), single-call: {ok}/{total}")
    return 0


def run_map():
    rows = []
    for f in sorted(IPC_DIR.glob("*.rs")):
        for w in parse_wrappers(f):
            problem, _, module, engine_fn = check_wrapper(w)
            if problem:
                print(f"{w.path.relative_to(ROOT)}:{w.line}: {w.name}: {problem}", file=sys.stderr)
                return 1
            rows.append((w.name, module, engine_fn, "async" if w.is_async else "sync"))
    for row in sorted(rows):
        print("\t".join(row))
    return 0


def normalise_param(p):
    name, _, ty = p.partition(":")
    name = name.strip()
    ty = collapse(ty)
    ty = re.sub(r",\s*>", ">", ty)
    ty = re.sub(r"\s*([<>,&])\s*", r"\1", ty)
    ty = re.sub(r",(?=\S)", ", ", ty)
    if re.match(r"^(tauri::)?State<", ty):
        return f"{name}: State"
    return f"{name}: {ty}"


def render_signature(w):
    params = ", ".join(normalise_param(p) for p in w.params if p)
    ret = re.sub(r",\s*>", ">", w.ret)
    ret = re.sub(r"\s*([<>,])\s*", r"\1", ret)
    ret = re.sub(r",(?=\S)", ", ", ret)
    return f"{w.name}|{'async' if w.is_async else 'sync'}|{params}|{ret}"


def run_signatures(directory):
    d = Path(directory)
    lines = []
    for f in sorted(d.glob("*.rs")):
        for w in parse_wrappers(f):
            lines.append(render_signature(w))
    for line in sorted(lines, key=lambda l: l.split("|", 1)[0]):
        print(line)
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--map", action="store_true")
    ap.add_argument("--signatures", metavar="DIR")
    args = ap.parse_args(argv)
    if args.signatures:
        return run_signatures(args.signatures)
    if args.map:
        return run_map()
    return run_check()


if __name__ == "__main__":
    sys.exit(main())
