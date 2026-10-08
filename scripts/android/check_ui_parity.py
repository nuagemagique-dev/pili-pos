#!/usr/bin/env python3
"""Check that the Compose UI covers the commands of the Vue UI (phase P5, PD212).

The command set is the Vue IPC wrapper layer: every invoke("name" and invoke<T>("name"
of ui/src/api/ipc.ts and ui/src/api/ipc-modifiers.ts. For each command the script
looks for a Kotlin call site and reads the declarations of docs/android/PARITY.md.

  (a) commands   quoted snake_case names after invoke / invoke<T>( ; zero names is a FAIL
  (b) FFI        the camelCase form of each command is a method of the block
                 PosEngineInterface in android/pos-core/api/pos-core.api, else FAIL
  (c) call sites in every .kt file under android/app/src/main/kotlin (except
                 .../i18n/generated/), after removing comments and string literals,
                 every lambda opened by `.call {` or `.call(` followed by a lambda is
                 scanned for `.<camelName>(`; the command is CALLED at that file:line.
                 A call outside such a lambda is not a call site.
  (d) declares   the fenced block with the info string ui-parity in
                 docs/android/PARITY.md, one line per command:
                     <command> <status> <note...>
                 status: excluded, or pending:<owner> with owner P5-T<n> or
                 P5b-<view> (view: checkout, reports, settings, webshop). Lines that
                 start with # and blank lines are ignored. FAIL on an unknown command,
                 a duplicate, an unknown status or owner, a malformed line, and an
                 excluded command that has a call site.
  (e) tables     in docs/android/PARITY.md and docs/android/parity/*.md every markdown
                 table whose header row is | Vue | Compose | Commands | is read; each
                 backticked snake_case token of a Commands cell must be a known command
                 (FAIL otherwise); with --strict it must also be CALLED.

Output, one line per command (sorted):
    UIPARITY | <command> | CALLED | <file:line>[, <file:line>...]
    UIPARITY | <command> | EXCLUDED | <note>
    UIPARITY | <command> | PENDING | <owner>
    UIPARITY | <command> | MISSING | no call site and no declaration
A CALLED command that is also declared pending gets the suffix
" (stale pending:<owner>)". Problems print as
    UIPARITY | problem | FAIL | <text>
and the last line is
    UIPARITY | total | PASS|FAIL | called N, excluded N, pending N, stale N, missing N (M commands)
Exit 1 on any FAIL or MISSING; with --strict also on any pending or stale command.

  --repo DIR   repository root (default: from this script's path)
  --strict     pending and stale commands are errors (the P5b exit)

The helper strip_kotlin() is also used by p5_gate.sh. Standard library only.
"""

import argparse
import bisect
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent

IPC_FILES = ("ui/src/api/ipc.ts", "ui/src/api/ipc-modifiers.ts")
API_FILE = "android/pos-core/api/pos-core.api"
KOTLIN_DIR = "android/app/src/main/kotlin"
PARITY_MD = "docs/android/PARITY.md"
PARITY_DIR = "docs/android/parity"
GENERATED_MARK = "/i18n/generated/"

SNAKE = r"[a-z][a-z0-9]*(?:_[a-z0-9]+)*"
SNAKE_RE = re.compile("^" + SNAKE + "$")
INVOKE_RE = re.compile(r"\binvoke\s*(?:<[^()\"']*?>)?\s*\(\s*[\"'](" + SNAKE + r")[\"']")
OWNER_RE = re.compile(r"^(?:P5-T[0-9]+|P5b-(?:checkout|reports|settings|webshop))$")
CALL_RE = re.compile(r"\.\s*call\s*(?:<[^(){}]*>)?\s*([({])")
METHOD_RE = re.compile(r"\.\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(")
CHAR_RE = re.compile(r"'(?:\\u[0-9a-fA-F]{4}|\\.|[^'\\\n])'")


# ---------------------------------------------------------------- Kotlin text


def _skip_braces(src, j):
    """Index after the `}` that closes a template opened just before j."""
    n = len(src)
    depth = 1
    while j < n and depth > 0:
        c = src[j]
        if c == '"':
            j = _skip_string(src, j)
            continue
        if c == "'":
            m = CHAR_RE.match(src, j)
            if m:
                j = m.end()
                continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
        j += 1
    return j


def _skip_string(src, i):
    """Index after the string literal that starts at src[i] == '"' (templates included)."""
    n = len(src)
    if src.startswith('"""', i):
        j = i + 3
        while j < n:
            if src.startswith('"""', j):
                j += 3
                while j < n and src[j] == '"':
                    j += 1
                return j
            if src.startswith("${", j):
                j = _skip_braces(src, j + 2)
            else:
                j += 1
        return n
    j = i + 1
    while j < n:
        c = src[j]
        if c == "\\":
            j += 2
        elif c == '"':
            return j + 1
        elif src.startswith("${", j):
            j = _skip_braces(src, j + 2)
        elif c == "\n":
            return j
        else:
            j += 1
    return n


def _blank_string(seg):
    res = "".join(ch if ch == "\n" else " " for ch in seg)
    if seg.startswith('"""') and seg.endswith('"""') and len(seg) >= 6:
        return '"""' + res[3:-3] + '"""'
    if seg.endswith('"') and len(seg) >= 2:
        return '"' + res[1:-1] + '"'
    return res


def strip_kotlin(src, drop_strings=False):
    """Blank comments (nested block comments too); with drop_strings also the content
    of string and char literals (the quotes stay, a char literal becomes spaces).
    Offsets and newlines are kept, so line numbers do not move."""
    n = len(src)
    out = []
    i = 0
    while i < n:
        c = src[i]
        if src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif src.startswith("/*", i):
            depth = 0
            j = i
            while j < n:
                if src.startswith("/*", j):
                    depth += 1
                    j += 2
                elif src.startswith("*/", j):
                    depth -= 1
                    j += 2
                    if depth == 0:
                        break
                else:
                    j += 1
            out.append("".join("\n" if ch == "\n" else " " for ch in src[i:j]))
            i = j
        elif c == '"':
            j = _skip_string(src, i)
            seg = src[i:j]
            out.append(_blank_string(seg) if drop_strings else seg)
            i = j
        elif c == "'":
            m = CHAR_RE.match(src, i)
            if m:
                out.append(" " * len(m.group(0)) if drop_strings else m.group(0))
                i = m.end()
            else:
                out.append(c)
                i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def match_close(s, i, op, cl):
    """Index of the closer that matches the opener s[i]; len(s) when unbalanced."""
    depth = 0
    n = len(s)
    j = i
    while j < n:
        if s[j] == op:
            depth += 1
        elif s[j] == cl:
            depth -= 1
            if depth == 0:
                return j
        j += 1
    return n


def call_lambda_regions(s):
    """(start, end) spans of the lambdas opened by `.call {` or `.call(` ... a lambda."""
    regions = []
    for m in CALL_RE.finditer(s):
        o = m.end() - 1
        if s[o] == "{":
            regions.append((o + 1, match_close(s, o, "{", "}")))
            continue
        close = match_close(s, o, "(", ")")
        j = o + 1
        while j < close:
            if s[j] == "{":
                e = match_close(s, j, "{", "}")
                regions.append((j + 1, min(e, close)))
                j = e
            j += 1
        k = close + 1
        while k < len(s) and s[k] in " \t\r\n":
            k += 1
        if k < len(s) and s[k] == "{":
            regions.append((k + 1, match_close(s, k, "{", "}")))
    return regions


def camel(name):
    head, *rest = name.split("_")
    return head + "".join(p[:1].upper() + p[1:] for p in rest)


def call_sites(text, camel_to_cmd):
    """{command: sorted set of line numbers} of the commands called in call lambdas."""
    stripped = strip_kotlin(text, drop_strings=True)
    starts = [0] + [m.end() for m in re.finditer("\n", stripped)]
    found = {}
    for a, b in call_lambda_regions(stripped):
        for m in METHOD_RE.finditer(stripped, a, b):
            cmd = camel_to_cmd.get(m.group(1))
            if cmd:
                line = bisect.bisect_right(starts, m.start(1))
                found.setdefault(cmd, set()).add(line)
    return found


# ---------------------------------------------------------------- inputs


def read_text(path):
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None


def read_commands(repo, problems):
    names = set()
    for rel in IPC_FILES:
        text = read_text(repo / rel)
        if text is None:
            problems.append("parse error: cannot read %s" % rel)
            continue
        names.update(INVOKE_RE.findall(text))
    if not names:
        problems.append("parse error: no invoke(\"name\") command found in %s" % ", ".join(IPC_FILES))
    return sorted(names)


def read_ffi_methods(repo, problems):
    text = read_text(repo / API_FILE)
    if text is None:
        problems.append("parse error: cannot read %s" % API_FILE)
        return set()
    methods = set()
    inside = False
    seen = False
    for line in text.splitlines():
        if line.startswith("public abstract interface class uniffi/pos_ffi/PosEngineInterface {"):
            inside = seen = True
            continue
        if inside:
            if line.startswith("}"):
                break
            m = re.match(r"\s*public abstract fun (\w+) ", line)
            if m:
                methods.add(m.group(1))
    if not seen:
        problems.append("parse error: no PosEngineInterface block in %s" % API_FILE)
    return methods


def read_call_sites(repo, camel_to_cmd):
    base = repo / KOTLIN_DIR
    sites = {}
    if not base.is_dir():
        return sites
    for path in sorted(base.rglob("*.kt")):
        rel = path.relative_to(repo).as_posix()
        if GENERATED_MARK in "/" + rel:
            continue
        text = read_text(path)
        if text is None:
            continue
        for cmd, lines in call_sites(text, camel_to_cmd).items():
            for ln in lines:
                sites.setdefault(cmd, set()).add((rel, ln))
    return sites


def fenced_lines(text):
    """Yield (lineno, line, info) where info is None outside a fence, else the fence's info
    string; the fence lines themselves are yielded with info '<fence>'."""
    info = None
    for no, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        if info is None:
            if stripped.startswith("```"):
                info = stripped[3:].strip()
                yield no, line, "<fence>"
            else:
                yield no, line, None
        else:
            if stripped.startswith("```"):
                info = None
                yield no, line, "<fence>"
            else:
                yield no, line, info


def read_declarations(repo, commands, problems):
    """{command: (kind, value)} with kind excluded or pending; reports the rule violations."""
    text = read_text(repo / PARITY_MD)
    decls = {}
    if text is None:
        problems.append("parse error: cannot read %s" % PARITY_MD)
        return decls
    found_block = False
    known = set(commands)
    first_line = {}
    for no, line, info in fenced_lines(text):
        if info != "ui-parity":
            continue
        found_block = True
        body = line.strip()
        if not body or body.startswith("#"):
            continue
        parts = body.split(None, 2)
        if len(parts) < 2:
            problems.append("%s:%d malformed declaration (want '<command> <status> <note>'): %s" % (PARITY_MD, no, body))
            continue
        cmd, status = parts[0], parts[1]
        note = parts[2] if len(parts) > 2 else ""
        if cmd not in known:
            problems.append("%s:%d unknown command %s" % (PARITY_MD, no, cmd))
            continue
        if cmd in decls:
            problems.append("%s:%d duplicate declaration of %s (first at line %d)" % (PARITY_MD, no, cmd, first_line[cmd]))
            continue
        if status == "excluded":
            decls[cmd] = ("excluded", note)
        elif status.startswith("pending:"):
            owner = status[len("pending:"):]
            if not OWNER_RE.match(owner):
                problems.append("%s:%d unknown owner '%s' for %s" % (PARITY_MD, no, owner, cmd))
                continue
            decls[cmd] = ("pending", owner)
        else:
            problems.append("%s:%d unknown status '%s' for %s" % (PARITY_MD, no, status, cmd))
            continue
        first_line[cmd] = no
    if not found_block:
        problems.append("parse error: no ui-parity block in %s" % PARITY_MD)
    return decls


def split_row(line):
    s = line.strip()
    if s.startswith("|"):
        s = s[1:]
    if s.endswith("|"):
        s = s[:-1]
    return [c.strip() for c in s.split("|")]


def table_tokens(repo):
    """[(file, line, token)] for the backticked snake_case tokens of every Commands cell."""
    files = []
    if (repo / PARITY_MD).is_file():
        files.append(repo / PARITY_MD)
    pdir = repo / PARITY_DIR
    if pdir.is_dir():
        files.extend(sorted(pdir.glob("*.md")))
    out = []
    for path in files:
        text = read_text(path)
        if text is None:
            continue
        rel = path.relative_to(repo).as_posix()
        in_table = False
        sep_next = False
        for no, line, info in fenced_lines(text):
            if info is not None:
                in_table = False
                continue
            s = line.strip()
            if not s.startswith("|"):
                in_table = False
                sep_next = False
                continue
            cells = split_row(s)
            if not in_table:
                if cells == ["Vue", "Compose", "Commands"]:
                    in_table = True
                    sep_next = True
                continue
            if sep_next:
                sep_next = False
                if re.match(r"^[\s|:-]+$", s):
                    continue
            if len(cells) >= 3:
                for tok in re.findall(r"`([^`]*)`", cells[2]):
                    if SNAKE_RE.match(tok):
                        out.append((rel, no, tok))
    return out


# ---------------------------------------------------------------- main


def run(repo, strict):
    problems = []
    commands = read_commands(repo, problems)
    ffi = read_ffi_methods(repo, problems)
    camel_to_cmd = {camel(c): c for c in commands}
    if ffi:
        for c in commands:
            if camel(c) not in ffi:
                problems.append("not-in-ffi: %s has no method %s in PosEngineInterface" % (c, camel(c)))
    sites = read_call_sites(repo, camel_to_cmd)
    decls = read_declarations(repo, commands, problems)

    lines = []
    called = excluded = pending = stale = missing = 0
    for c in commands:
        kind, value = decls.get(c, (None, None))
        cs = sorted(sites.get(c, ()))
        if cs:
            called += 1
            where = ", ".join("%s:%d" % s for s in cs)
            suffix = ""
            if kind == "pending":
                stale += 1
                suffix = " (stale pending:%s)" % value
            elif kind == "excluded":
                problems.append("%s is excluded but has a call site (%s)" % (c, where))
            lines.append("UIPARITY | %s | CALLED | %s%s" % (c, where, suffix))
        elif kind == "excluded":
            excluded += 1
            lines.append("UIPARITY | %s | EXCLUDED | %s" % (c, value or "-"))
        elif kind == "pending":
            pending += 1
            lines.append("UIPARITY | %s | PENDING | %s" % (c, value))
        else:
            missing += 1
            lines.append("UIPARITY | %s | MISSING | no call site and no declaration" % c)

    known = set(commands)
    for rel, no, tok in table_tokens(repo):
        if tok not in known:
            problems.append("%s:%d table cites unknown command %s" % (rel, no, tok))
        elif strict and tok not in sites:
            problems.append("%s:%d table cites %s, which has no call site (--strict)" % (rel, no, tok))

    for ln in lines:
        print(ln)
    for p in problems:
        print("UIPARITY | problem | FAIL | %s" % p)
    bad = bool(problems) or missing > 0 or (strict and (pending > 0 or stale > 0))
    print(
        "UIPARITY | total | %s | called %d, excluded %d, pending %d, stale %d, missing %d (%d commands)"
        % ("FAIL" if bad else "PASS", called, excluded, pending, stale, missing, len(commands))
    )
    return 1 if bad else 0


def main(argv=None):
    ap = argparse.ArgumentParser(description="Command parity between the Vue UI and the Compose UI")
    ap.add_argument("--repo", default=str(ROOT), help="repository root (default: from this script's path)")
    ap.add_argument("--strict", action="store_true", help="pending and stale commands are errors")
    args = ap.parse_args(argv)
    return run(Path(args.repo).resolve(), args.strict)


if __name__ == "__main__":
    sys.exit(main())
