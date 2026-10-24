#!/usr/bin/env python3
"""pili-pos - Playwright test inventory (the Vue suite never loses a test).

The inventory is the output of `npx playwright test --list --project=e2e` run
in ui/ (or the same text read from --current FILE). Every test line is
normalised to `<spec file> > <describe parts> > <title>` (separator U+203A,
as Playwright prints it):
  - leading spaces and the `[e2e] > ` project prefix are dropped,
  - `<name>.spec.ts:<line>:<col>` becomes `<name>.spec.ts`,
  - the `Listing tests:` and `Total:` lines are ignored.
Duplicates are kept (the lists compare as multisets); the output is sorted
byte-wise (LC_ALL=C order).

Usage:
  scripts/android/playwright_inventory.py --write   write the baseline
  scripts/android/playwright_inventory.py --diff    compare with the baseline
  common options:
    --current FILE    read the list from FILE instead of running npx
    --baseline FILE   default docs/android/baseline/playwright_list.txt
    --moves FILE      default docs/android/baseline/playwright_moves.tsv
    --min N           --diff fails when the current list has fewer than N
                      tests (default 280)

--write writes the baseline with `#` header lines (source commit, total).
--diff loads the baseline, applies the moves file (`#` comments; data lines
`old<TAB>new`, both normalised titles), then prints
    playwright inventory: added N, removed M, total T
plus the removed titles, and exits 1 when M > 0 or T < --min. A move rule is
stale (the run fails) when its old title is still in the current list, when
its new title is not in the current list, or when its old title is not in
the baseline; two rules with the same old title fail too. Exit 2 on a usage
error, a missing file or a failed npx call.
"""
import argparse
import collections
import os
import re
import subprocess
import sys

ROOT = os.path.realpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
DEFAULT_BASELINE = os.path.join(ROOT, "docs", "android", "baseline", "playwright_list.txt")
DEFAULT_MOVES = os.path.join(ROOT, "docs", "android", "baseline", "playwright_moves.tsv")
DEFAULT_MIN = 280

SEP = " › "
PROJECT_PREFIX = "[e2e]" + SEP
SPEC_POS = re.compile(r"^(\S+\.spec\.ts):\d+:\d+(?=" + re.escape(SEP) + r"|$)")
TOTAL_LINE = re.compile(r"^Total: (\d+) tests? in \d+ files?$")


class InventoryError(Exception):
    """A usage error or an unusable input (exit 2)."""


def normalise_line(line):
    """Normalise one test line, or return None for a non-test line."""
    text = line.strip(" \t\r\n")
    if not text or text.startswith("Listing tests:") or text.startswith("Total:"):
        return None
    if text.startswith(PROJECT_PREFIX):
        text = text[len(PROJECT_PREFIX):]
    elif text.startswith("["):
        # another project prefix: not part of this inventory
        return None
    new, count = SPEC_POS.subn(r"\1", text, count=1)
    if count == 0:
        return None
    return new


def parse_list(text):
    """Return (sorted test lines, the Total: figure or None)."""
    tests = []
    total = None
    for line in text.splitlines():
        m = TOTAL_LINE.match(line.strip())
        if m:
            total = int(m.group(1))
            continue
        norm = normalise_line(line)
        if norm is not None:
            tests.append(norm)
    return sorted(tests, key=lambda s: s.encode("utf-8")), total


def read_text(path):
    try:
        with open(path, encoding="utf-8") as fh:
            return fh.read()
    except OSError as exc:
        raise InventoryError("cannot read %s: %s" % (path, exc))


def read_baseline(path):
    tests = []
    for line in read_text(path).splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        tests.append(line.rstrip("\r\n"))
    return sorted(tests, key=lambda s: s.encode("utf-8"))


def read_moves(path):
    """Return the ordered list of (old, new) rules; a missing file means none."""
    if path is None or not os.path.exists(path):
        return []
    rules = []
    for n, line in enumerate(read_text(path).splitlines(), 1):
        if not line.strip() or line.startswith("#"):
            continue
        parts = line.rstrip("\r\n").split("\t")
        if len(parts) != 2 or not parts[0] or not parts[1]:
            raise InventoryError("%s:%d: expected `old<TAB>new`" % (path, n))
        rules.append((parts[0], parts[1]))
    return rules


def apply_moves(baseline, current, rules):
    """Return (mapped baseline list, list of stale/duplicate rule problems)."""
    problems = []
    cur = collections.Counter(current)
    base = collections.Counter(baseline)
    seen = set()
    mapped = collections.Counter(base)
    for old, new in rules:
        if old in seen:
            problems.append("two rules with the same old title: %s" % old)
            continue
        seen.add(old)
        if cur[old] > 0:
            problems.append("stale rule, the old title is still present: %s" % old)
            continue
        if cur[new] == 0:
            problems.append("stale rule, the new title is absent: %s" % new)
            continue
        if base[old] == 0:
            problems.append("stale rule, the old title is not in the baseline: %s" % old)
            continue
        mapped[old] -= 1
        if mapped[old] == 0:
            del mapped[old]
        mapped[new] += 1
    return mapped, problems


def run_npx():
    ui = os.path.join(ROOT, "ui")
    try:
        proc = subprocess.run(
            ["npx", "playwright", "test", "--list", "--project=e2e"],
            cwd=ui,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            universal_newlines=True,
        )
    except OSError as exc:
        raise InventoryError("cannot run npx: %s" % exc)
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr[-2000:])
        raise InventoryError("npx playwright test --list failed (exit %d)" % proc.returncode)
    return proc.stdout


def load_current(current_file):
    text = read_text(current_file) if current_file else run_npx()
    tests, total = parse_list(text)
    if not tests:
        raise InventoryError("no test line found in the Playwright list output")
    if total is not None and total != len(tests):
        raise InventoryError("the output says Total: %d but %d test lines were parsed" % (total, len(tests)))
    return tests


def source_commit():
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            universal_newlines=True,
        )
        return out.stdout.strip() or "unknown"
    except OSError:
        return "unknown"


def do_write(args):
    tests = load_current(args.current)
    files = len({t.split(SEP, 1)[0] for t in tests})
    header = [
        "# pili-pos Playwright inventory (npx playwright test --list --project=e2e in ui/)",
        "# source commit: %s" % source_commit(),
        "# total: %d tests in %d files" % (len(tests), files),
        "# one normalised line per test: <spec file>%s<describe>%s<title>; written once, never edited" % (SEP, SEP),
    ]
    with open(args.baseline, "w", encoding="utf-8", newline="\n") as fh:
        fh.write("\n".join(header + tests) + "\n")
    print("playwright inventory: wrote %d tests to %s" % (len(tests), args.baseline))
    return 0


def do_diff(args):
    baseline = read_baseline(args.baseline)
    if not baseline:
        raise InventoryError("the baseline %s holds no test" % args.baseline)
    current = load_current(args.current)
    rules = read_moves(args.moves)
    mapped, problems = apply_moves(baseline, current, rules)
    cur = collections.Counter(current)
    added = sum((cur - mapped).values())
    removed_c = mapped - cur
    removed = sum(removed_c.values())
    total = len(current)
    print("playwright inventory: added %d, removed %d, total %d" % (added, removed, total))
    for title in sorted(removed_c.elements(), key=lambda s: s.encode("utf-8")):
        print("  removed: %s" % title)
    failed = False
    for p in problems:
        print("  %s" % p)
        failed = True
    if removed > 0:
        failed = True
    if total < args.min:
        print("  total %d is below the floor %d" % (total, args.min))
        failed = True
    return 1 if failed else 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    mode = ap.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--diff", action="store_true")
    ap.add_argument("--current", metavar="FILE")
    ap.add_argument("--baseline", default=DEFAULT_BASELINE, metavar="FILE")
    ap.add_argument("--moves", default=DEFAULT_MOVES, metavar="FILE")
    ap.add_argument("--min", type=int, default=DEFAULT_MIN, metavar="N")
    try:
        args = ap.parse_args(argv)
    except SystemExit as exc:
        return 0 if exc.code == 0 else 2
    try:
        return do_write(args) if args.write else do_diff(args)
    except InventoryError as exc:
        sys.stderr.write("playwright inventory: %s\n" % exc)
        return 2


if __name__ == "__main__":
    sys.exit(main())
