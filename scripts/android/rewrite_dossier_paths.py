#!/usr/bin/env python3
"""rewrite_dossier_paths.py - keep the dossier's source-path references true
while the Tauri-free fiscal core moves out of src-tauri (phase P1a).

Each P1a card that moves code runs this tool with --apply in the same
commit as the move, so scripts/check_dossier.py stays green on every commit.
A reference is only rewritten once its target exists in the working tree.

Scope (files rewritten): docs/SUBMISSION_ARTIFACTS/**/*.md, README.md,
CLAUDE.md, deploy/**/*.md. Never touched: docs/android/**, docs/internal/**,
the official SPF documents (everything else under docs/) and any file that
is not .md.

Rules, applied most specific first (the first rule that matches wins):
    a  src-tauri/src/commands/<m>.rs::<cmd>, <cmd> a Tauri command of module
       <m> (read from the generate_handler! lists of src-tauri/src/main.rs,
       entries commands::<m>::<cmd> or ipc::<m>::<cmd>)
         -> src-tauri/src/ipc/<m>.rs::<cmd>
    b  src-tauri/src/cli.rs::{init_db,apply_seed,check_config}
         -> crates/pos-engine/src/install.rs::<same>
    c  src-tauri/src/commands/ and the bare directory
         -> crates/pos-engine/src/commands/
    d  src-tauri/src/state.rs -> crates/pos-engine/src/state.rs
    e  src-tauri/tests/common/mod.rs -> crates/pos-engine-testkit/src/lib.rs;
       src-tauri/tests/common[/...] -> crates/pos-engine-testkit/src[/...]
    f  src-tauri/tests[/...] -> crates/pos-engine/tests[/...]

Matching respects path boundaries (src-tauri/tests2 and
src-tauri/src/commands.rs are never matched as the directory). A ::item or
:line suffix is kept unchanged; a :line suffix is reported for manual
review because line numbers move.

Modes:
    --plan      (default) per rule: matches and whether the target exists
                now; last line "total: <N> references in <F> files"; exit 0
    --apply     rewrite the matches whose target exists; idempotent
    --check     exit 1 when a match remains whose target exists (the code
                moved, the docs were not rewritten)
    --strict    exit 1 when any match of any rule remains (phase exit)
    --unmapped  list in-scope references to the moved paths that no rule
                covers; exit 1 when there are any

python3 stdlib only.
"""

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

DEFAULT_ROOT = Path(__file__).resolve().parent.parent.parent

ENGINE = "crates/pos-engine"
TESTKIT = "crates/pos-engine-testkit"

RULE_TITLES = {
    "a": "wrapper (command)  commands/<m>.rs::<cmd> -> ipc/<m>.rs::<cmd>",
    "b": "install functions  cli.rs::{init_db,apply_seed,check_config} -> pos-engine install.rs",
    "c": "engine commands    src-tauri/src/commands -> crates/pos-engine/src/commands",
    "d": "engine state       src-tauri/src/state.rs -> crates/pos-engine/src/state.rs",
    "e": "test harness       src-tauri/tests/common -> crates/pos-engine-testkit/src",
    "f": "integration tests  src-tauri/tests -> crates/pos-engine/tests",
}
RULE_ORDER = ["a", "b", "c", "d", "e", "f"]

# A reference starts at a path boundary: not glued to a word, a hyphen or a
# longer name on its left.
_LEFT = r"(?<![\w-])"
# ... and ends at a boundary: not followed by a word char, a hyphen, or a
# dot-then-word (so commands.rs and tests2 are other names, but a sentence
# full stop or a trailing slash is fine).
_RIGHT = r"(?![\w-])(?!\.\w)"

# One master pattern. Alternatives are ordered; each names the thing matched.
_MASTER = re.compile(
    _LEFT
    + "src-tauri/"
    + "(?:"
    # cli.rs only through the three install functions
    + r"src/cli\.rs(?=::(?P<cli>init_db|apply_seed|check_config)(?!\w))"
    # state.rs
    + r"|(?P<state>src/state\.rs)" + _RIGHT
    # commands/<m>.rs (a file)
    + r"|src/commands/(?P<cmdfile>[A-Za-z0-9_]+)\.rs" + _RIGHT
    # commands directory: bare, or followed by a slash
    + r"|(?P<cmddir>src/commands)(?:(?=/)|" + _RIGHT + ")"
    # tests/common/mod.rs
    + r"|(?P<testsmod>tests/common/mod\.rs)" + _RIGHT
    # tests/common/<file>
    + r"|tests/common/(?P<commonfile>[\w-]+\.\w+)" + _RIGHT
    # tests/common directory
    + r"|(?P<commondir>tests/common)(?:(?=/)|" + _RIGHT + ")"
    # tests/<file>
    + r"|tests/(?P<testfile>[\w-]+\.\w+)" + _RIGHT
    # tests directory (bare, with a slash, or before a partial name)
    + r"|(?P<testsdir>tests)(?:(?=/)|" + _RIGHT + ")"
    + ")"
)

# Suffix that follows a matched file path.
_ITEM = re.compile(r"::(?P<item>\w+)")
_LINE = re.compile(r":(?P<line>\d+)(?![\w])")

# Loose scan for --unmapped: any mention, without boundary care.
_LOOSE = [
    re.compile(r"src-tauri/src/commands(?!\.rs\b)"),
    re.compile(r"src-tauri/src/state\.rs"),
    re.compile(r"src-tauri/src/cli\.rs::(?:init_db|apply_seed|check_config)\w*"),
    re.compile(r"src-tauri/tests"),
]

_ENTRY = re.compile(r"^\s*(?:commands|ipc)::(\w+)::(\w+)\s*,")


@dataclass
class Match:
    path: Path  # file the match is in
    rel: str  # repo-relative path of that file
    line: int  # 1-based line number
    start: int  # span of the path part to replace (suffix excluded)
    end: int
    rule: str
    old: str
    new: str
    target: str  # repo-relative target that must exist to apply
    line_suffix: bool  # followed by :<line>


def command_names(root: Path):
    """Set of (module, command) pairs from the generate_handler! lists."""
    main = root / "src-tauri" / "src" / "main.rs"
    pairs = set()
    try:
        text = main.read_text(encoding="utf-8")
    except OSError:
        return pairs
    for ln in text.splitlines():
        m = _ENTRY.match(ln)
        if m:
            pairs.add((m.group(1), m.group(2)))
    return pairs


def scope_files(root: Path):
    """In-scope markdown files, sorted, repo-relative posix strings."""
    found = []
    for name in ("README.md", "CLAUDE.md"):
        if (root / name).is_file():
            found.append(name)
    for base in ("docs/SUBMISSION_ARTIFACTS", "deploy"):
        d = root / base
        if d.is_dir():
            for p in d.rglob("*.md"):
                if p.is_file():
                    found.append(p.relative_to(root).as_posix())
    return sorted(set(found))


def classify(m, text, cmds):
    """Return (rule, new_path, target, line_suffix) for a master match."""
    old = m.group(0)
    after = text[m.end():]
    g = m.groupdict()
    item = None
    im = _ITEM.match(after)
    if im:
        item = im.group("item")
    line_suffix = False
    if not im and _LINE.match(after):
        line_suffix = True

    if g["cli"]:
        return "b", f"{ENGINE}/src/install.rs", f"{ENGINE}/src/install.rs", False
    if g["state"]:
        t = f"{ENGINE}/src/state.rs"
        return "d", t, t, line_suffix
    if g["cmdfile"]:
        mod = g["cmdfile"]
        if item is not None and (mod, item) in cmds:
            t = f"src-tauri/src/ipc/{mod}.rs"
            return "a", t, t, False
        t = f"{ENGINE}/src/commands/{mod}.rs"
        return "c", t, t, line_suffix
    if g["cmddir"]:
        t = f"{ENGINE}/src/commands"
        return "c", t, t, False
    if g["testsmod"]:
        t = f"{TESTKIT}/src/lib.rs"
        return "e", t, t, line_suffix
    if g["commonfile"]:
        t = f"{TESTKIT}/src/{g['commonfile']}"
        return "e", t, t, line_suffix
    if g["commondir"]:
        t = f"{TESTKIT}/src"
        return "e", t, t, False
    if g["testfile"]:
        t = f"{ENGINE}/tests/{g['testfile']}"
        return "f", t, t, line_suffix
    t = f"{ENGINE}/tests"
    return "f", t, t, False


def _new_text(rule, m):
    """Replacement for the path part of a master match."""
    g = m.groupdict()
    if rule == "a":
        return f"src-tauri/src/ipc/{g['cmdfile']}.rs"
    if rule == "b":
        return f"{ENGINE}/src/install.rs"
    if rule == "c":
        if g["cmdfile"]:
            return f"{ENGINE}/src/commands/{g['cmdfile']}.rs"
        return f"{ENGINE}/src/commands"
    if rule == "d":
        return f"{ENGINE}/src/state.rs"
    if rule == "e":
        if g["testsmod"]:
            return f"{TESTKIT}/src/lib.rs"
        if g["commonfile"]:
            return f"{TESTKIT}/src/{g['commonfile']}"
        return f"{TESTKIT}/src"
    if g["testfile"]:
        return f"{ENGINE}/tests/{g['testfile']}"
    return f"{ENGINE}/tests"


def find_matches(root: Path, cmds):
    out = []
    for rel in scope_files(root):
        p = root / rel
        try:
            text = p.read_text(encoding="utf-8", newline="")
        except (OSError, UnicodeDecodeError):
            continue
        for m in _MASTER.finditer(text):
            rule, _new, target, line_suffix = classify(m, text, cmds)
            out.append(
                Match(
                    path=p,
                    rel=rel,
                    line=text.count("\n", 0, m.start()) + 1,
                    start=m.start(),
                    end=m.end(),
                    rule=rule,
                    old=m.group(0),
                    new=_new_text(rule, m),
                    target=target,
                    line_suffix=line_suffix,
                )
            )
    return out


def target_exists(root: Path, mt: Match) -> bool:
    return (root / mt.target).exists()


def unmapped(root: Path, matches):
    """In-scope loose mentions not covered by any rule match."""
    covered = {}
    for mt in matches:
        covered.setdefault(mt.rel, []).append((mt.start, mt.end))
    found = []
    for rel in scope_files(root):
        try:
            text = (root / rel).read_text(encoding="utf-8", newline="")
        except (OSError, UnicodeDecodeError):
            continue
        spans = covered.get(rel, [])
        seen = set()
        for rx in _LOOSE:
            for m in rx.finditer(text):
                if any(s <= m.start() < e for s, e in spans):
                    continue
                if m.start() in seen:
                    continue
                seen.add(m.start())
                found.append((rel, text.count("\n", 0, m.start()) + 1, m.group(0)))
    return sorted(found)


def apply(root: Path, matches):
    """Rewrite the matches whose target exists. Returns {rel: count}."""
    by_file = {}
    for mt in matches:
        if target_exists(root, mt):
            by_file.setdefault(mt.rel, []).append(mt)
    changed = {}
    for rel, mts in by_file.items():
        p = root / rel
        text = p.read_text(encoding="utf-8", newline="")
        # Replace right to left so earlier offsets stay valid. A match span
        # ends where the path part ends (the master pattern never consumes
        # the ::item or :line suffix).
        for mt in sorted(mts, key=lambda x: x.start, reverse=True):
            text = text[: mt.start] + mt.new + text[mt.end :]
        p.write_text(text, encoding="utf-8", newline="")
        changed[rel] = len(mts)
    return changed


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    mode = ap.add_mutually_exclusive_group()
    mode.add_argument("--plan", action="store_true", help="report only (default)")
    mode.add_argument("--apply", action="store_true", help="rewrite where the target exists")
    mode.add_argument("--check", action="store_true", help="fail if a rewritable match remains")
    mode.add_argument("--strict", action="store_true", help="fail if any match remains")
    mode.add_argument("--unmapped", action="store_true", help="list references no rule covers")
    ap.add_argument("--root", default=str(DEFAULT_ROOT), help="repository root (tests only)")
    args = ap.parse_args(argv)
    root = Path(args.root).resolve()

    cmds = command_names(root)
    matches = find_matches(root, cmds)

    if args.unmapped:
        found = unmapped(root, matches)
        for rel, ln, txt in found:
            print(f"{rel}:{ln}: unmapped reference {txt}")
        print(f"unmapped: {len(found)}")
        return 1 if found else 0

    if args.apply:
        changed = apply(root, matches)
        for rel in sorted(changed):
            print(f"rewrote {changed[rel]:3d} in {rel}")
        total = sum(changed.values())
        left = [mt for mt in matches if not target_exists(root, mt)]
        review = [mt for mt in matches if mt.line_suffix and target_exists(root, mt)]
        for mt in review:
            print(f"{mt.rel}:{mt.line}: line number to review by hand ({mt.old}:<line>)")
        print(
            f"applied: {total} references in {len(changed)} files; "
            f"{len(left)} left (target not there yet)"
        )
        return 0

    if args.check:
        bad = [mt for mt in matches if target_exists(root, mt)]
        for mt in bad:
            print(f"{mt.rel}:{mt.line}: {mt.old} moved to {mt.new}, rewrite with --apply")
        print(f"check: {len(bad)} references to rewrite")
        return 1 if bad else 0

    if args.strict:
        for mt in matches:
            print(f"{mt.rel}:{mt.line}: [{mt.rule}] {mt.old} -> {mt.new}")
        print(f"strict: {len(matches)} references remain")
        return 1 if matches else 0

    # --plan (default)
    for r in RULE_ORDER:
        rm = [mt for mt in matches if mt.rule == r]
        have = sum(1 for mt in rm if target_exists(root, mt))
        if not rm:
            state = "n/a"
        elif have == len(rm):
            state = "yes"
        elif have == 0:
            state = "no"
        else:
            state = f"partial ({have} of {len(rm)})"
        print(f"rule {r}: {len(rm)} matches, target exists: {state}  [{RULE_TITLES[r]}]")
    review = [mt for mt in matches if mt.line_suffix]
    if review:
        print("manual review (line numbers move):")
        for mt in review:
            print(f"  {mt.rel}:{mt.line}: {mt.old}:<line>")
    files = len({mt.rel for mt in matches})
    print(f"total: {len(matches)} references in {files} files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
