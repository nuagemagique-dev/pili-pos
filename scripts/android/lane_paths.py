#!/usr/bin/env python3
"""Lane ownership checker for the Android port.

Reads the 'lanes' and 'lane-probes' fenced blocks of docs/android/LANES.md.

  --check                 validate the rules against the probes and the tree
  --regex OWNER_OR_PREFIX print ^(r1|r2|...)$ for the owner (or owner-*)
  --owner PATH            print the owner of PATH, or 'none'
  --list-owners           print every owner
  --lanes FILE            read FILE instead of docs/android/LANES.md
  --root DIR              repo root (default: found from the script location)

Run with python3 -I. Standard library only.
"""
import os
import re
import subprocess
import sys
import xml.etree.ElementTree as ET

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DEFAULT_ROOT = os.path.dirname(os.path.dirname(SCRIPT_DIR))
LANES_REL = "docs/android/LANES.md"
VIEWS = ("login", "tables", "order", "checkout", "reports", "settings", "webshop")
STRINGS_RE = re.compile(r"^android/app/src/main/res/values[^/]*/strings_([A-Za-z0-9_]+)[.]xml$")
FIXED_PREFIX = {
    "shell": "shell_",
    "engine": "engine_",
    "provisioning": "prov_",
    "kiosk": "kiosk_",
    "nightly": "nightly_",
}


class Rule:
    def __init__(self, owner, regex, lineno):
        self.owner = owner
        self.regex = regex
        self.lineno = lineno
        self.compiled = re.compile("^(" + regex + ")$")


def parse_lanes(path):
    """Return (rules, probes, problems)."""
    rules, probes, problems = [], [], []
    try:
        with open(path, encoding="utf-8") as fh:
            lines = fh.read().split("\n")
    except OSError as exc:
        return rules, probes, ["cannot read %s: %s" % (path, exc)]
    block = None
    for i, raw in enumerate(lines, 1):
        line = raw.rstrip("\r")
        if block is None:
            m = re.match(r"^```([A-Za-z0-9_-]*)\s*$", line)
            if m:
                block = m.group(1) or "-"
            continue
        if line.startswith("```"):
            block = None
            continue
        if block not in ("lanes", "lane-probes"):
            continue
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        parts = s.split()
        if len(parts) != 2:
            problems.append("%s:%d: malformed %s line (want '<a> <b>'): %s" % (path, i, "rule" if block == "lanes" else "probe", s))
            continue
        if block == "lanes":
            owner, rx = parts
            if not re.match(r"^[a-z][a-z0-9-]*$", owner):
                problems.append("%s:%d: bad owner name %r" % (path, i, owner))
                continue
            if "\\" in rx or "(?" in rx:
                problems.append("%s:%d: regex must be POSIX ERE (no backslash, no (?): %s" % (path, i, rx))
                continue
            try:
                rules.append(Rule(owner, rx, i))
            except re.error as exc:
                problems.append("%s:%d: regex does not compile: %s (%s)" % (path, i, rx, exc))
        else:
            probes.append((parts[0], parts[1], i))
    if block is not None:
        problems.append("%s: unterminated fenced block %r" % (path, block))
    return rules, probes, problems


def owners_of(rules, path):
    return sorted({r.owner for r in rules if r.compiled.match(path)})


def list_owners(rules):
    seen = []
    for r in rules:
        if r.owner not in seen:
            seen.append(r.owner)
    return seen


def regex_for(rules, arg):
    chosen = [o for o in list_owners(rules) if o == arg or o.startswith(arg + "-")]
    if not chosen:
        return None
    parts = [r.regex for r in rules if r.owner in chosen]
    return "^(" + "|".join(parts) + ")$"


def repo_files(root):
    out = []
    for args in (["ls-files", "-z"], ["ls-files", "-z", "-o", "--exclude-standard"]):
        res = subprocess.run(["git", "-C", root] + args, capture_output=True)
        if res.returncode != 0:
            raise RuntimeError("git %s failed: %s" % (" ".join(args), res.stderr.decode(errors="replace").strip()))
        out += [p for p in res.stdout.decode("utf-8", "surrogateescape").split("\0") if p]
    return sorted(set(out))


def check_strings_file(root, rel, problems):
    m = STRINGS_RE.match(rel)
    if not m:
        return
    name = m.group(1)
    if name.startswith("vue_"):
        return
    prefix = None
    if name in FIXED_PREFIX:
        prefix = FIXED_PREFIX[name]
    elif name.startswith("ui_") and name[3:] in ("common", "eft") + VIEWS:
        prefix = name + "_"
    elif re.match(r"^plat_[a-z]+$", name):
        prefix = name + "_"
    if prefix is None:
        problems.append("%s: unknown hand-written strings file name 'strings_%s.xml'" % (rel, name))
        return
    full = os.path.join(root, rel)
    if not os.path.isfile(full):
        return
    try:
        tree = ET.parse(full)
    except (ET.ParseError, OSError) as exc:
        problems.append("%s: cannot parse: %s" % (rel, exc))
        return
    for el in tree.getroot():
        if el.tag in ("string", "plurals", "string-array"):
            n = el.get("name")
            if n is None or not n.startswith(prefix):
                problems.append("%s: entry %r lacks prefix %r" % (rel, n, prefix))


def do_check(rules, probes, problems, root):
    files = []
    try:
        files = repo_files(root)
    except (RuntimeError, OSError) as exc:
        problems.append("cannot list files: %s" % exc)
    for path, expected, lineno in probes:
        got = owners_of(rules, path)
        if not got:
            problems.append("probe %s matches no rule" % path)
        elif len(got) > 1:
            problems.append("probe %s matches several owners: %s" % (path, ", ".join(got)))
        elif got[0] != expected:
            problems.append("probe %s is owned by %s, expected %s" % (path, got[0], expected))
    for rel in files:
        got = owners_of(rules, rel)
        if len(got) > 1:
            problems.append("%s matches several owners: %s" % (rel, ", ".join(got)))
        elif not got and rel.startswith("android/app/"):
            problems.append("%s under android/app/ has no owner" % rel)
        check_strings_file(root, rel, problems)
    return files




def main(argv):
    lanes = None
    root = DEFAULT_ROOT
    mode = None
    arg = None
    i = 0
    while i < len(argv):
        a = argv[i]
        if a in ("--lanes", "--root"):
            if i + 1 >= len(argv):
                print("lane_paths: %s needs a value" % a, file=sys.stderr)
                return 2
            if a == "--lanes":
                lanes = argv[i + 1]
            else:
                root = os.path.abspath(argv[i + 1])
            i += 2
            continue
        if a in ("--check", "--list-owners"):
            mode = a
        elif a in ("--regex", "--owner"):
            if i + 1 >= len(argv):
                print("lane_paths: %s needs a value" % a, file=sys.stderr)
                return 2
            mode, arg = a, argv[i + 1]
            i += 1
        else:
            print("lane_paths: unknown argument %s" % a, file=sys.stderr)
            return 2
        i += 1
    if mode is None:
        print(__doc__, file=sys.stderr)
        return 2
    if lanes is None:
        lanes = os.path.join(root, LANES_REL)
    rules, probes, problems = parse_lanes(lanes)

    if mode == "--check":
        files = do_check(rules, probes, problems, root)
        if not rules and not any("cannot read" in p for p in problems):
            problems.append("no rules found in %s" % lanes)
        if problems:
            for p in problems:
                print(p, file=sys.stderr)
            print("LANES | total | FAIL | %d problems" % len(problems))
            return 1
        print("LANES | total | PASS | %d files, %d rules, %d probes" % (len(files), len(rules), len(probes)))
        return 0

    if problems:
        for p in problems:
            print(p, file=sys.stderr)
        return 1
    if mode == "--list-owners":
        for o in list_owners(rules):
            print(o)
        return 0
    if mode == "--regex":
        rx = regex_for(rules, arg)
        if rx is None:
            print("lane_paths: no owner matches %r" % arg, file=sys.stderr)
            return 2
        print(rx)
        return 0
    if mode == "--owner":
        path = arg[2:] if arg.startswith("./") else arg
        got = owners_of(rules, path)
        if len(got) > 1:
            print("lane_paths: %s matches several owners: %s" % (path, ", ".join(got)), file=sys.stderr)
            return 1
        print(got[0] if got else "none")
        return 0
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
