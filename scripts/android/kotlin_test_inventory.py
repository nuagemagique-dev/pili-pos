#!/usr/bin/env python3
"""pili-pos - Kotlin test inventory (PD111).

Lists the Kotlin tests of android/<module>/src/<set>/ where <set> is `test`,
`androidTest` or any directory whose name ends in `Test`. One id per function
annotated @Test:

    <module>/<set>:<package>.<TopLevelClassOfTheFile>#<function>

The annotation may be on the same line as `fun` or on lines above, other
annotations and modifiers may sit in between, backtick names are kept as written
(backticks included) and nested classes are attributed to the first top-level class
of the file. Comments and string literals are ignored.

Usage:
  kotlin_test_inventory.py --list
      print the ids of the working tree, sorted, one per line
  kotlin_test_inventory.py --base REF
      read the same files at REF (git ls-tree + git show) and compare: FAIL when
      an id of REF is missing in the working tree unless a rule of the moves file
      maps it. Prints exactly one line
          KTESTS | total | PASS|FAIL | base N, head M, removed K
      and the removed ids on stderr.
  Options: --root DIR (repository root, default: two levels above this script),
           --moves FILE (default scripts/android/kotlin_test_moves.tsv under the root)

Moves file: lines `old_prefix<TAB>new_prefix`, `#` starts a comment. An id of REF
that is missing in the working tree is mapped by the first rule whose old_prefix it
starts with and passes when the mapped id exists. A rule that matches no id of REF is
stale and fails the check.

Exit: 0 PASS, 1 FAIL, 2 usage or git error.
"""

import collections
import os
import re
import subprocess
import sys

SET_RE = re.compile(r"^android/([^/]+)/src/(test|androidTest|[^/]*Test)/(.+\.kt)$")
MODIFIERS = {
    "public", "private", "protected", "internal", "open", "final", "abstract",
    "override", "suspend", "inline", "operator", "infix", "external", "tailrec",
    "actual", "expect",
}
IDENT = re.compile(r"[A-Za-z_]\w*")
TEST_ANN = re.compile(r"@(?:[A-Za-z_]\w*\.)*Test\b(?![\w.])")
PACKAGE = re.compile(r"^[ \t]*package[ \t]+([\w.]+)", re.M)
CLASS_DECL = re.compile(r"(?<![\w.:])(?:class|object|interface)[ \t\r\n]+(`[^`\n]+`|[A-Za-z_]\w*)")
FUN_NAME = re.compile(
    r"\s*(?:<[^>]*>\s*)?(?:(?:`[^`\n]*`|[\w<>?,]+)\.)*(`[^`\n]*`|[A-Za-z_]\w*)\s*\("
)


class UsageError(Exception):
    pass


def clean(src):
    """Blank comments and string/char literal contents, keep length and newlines."""
    n = len(src)
    out = list(src)

    def blank(a, b):
        for k in range(a, min(b, n)):
            if out[k] != "\n":
                out[k] = " "

    def skip_template(i):
        # src[i:i+2] == "${": return the index after the matching "}"
        depth = 0
        while i < n:
            c = src[i]
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
                if depth == 0:
                    return i + 1
            elif c == '"':
                i = skip_string(i)
                continue
            i += 1
        return n

    def skip_string(i):
        # src[i] == '"': return the index after the literal (contents blanked)
        raw = src.startswith('"""', i)
        j = i + (3 if raw else 1)
        start = j
        while j < n:
            if raw:
                if src.startswith('"""', j):
                    while j + 3 < n and src[j + 3] == '"':
                        j += 1
                    blank(start, j)
                    return j + 3
            else:
                if src[j] == "\\":
                    j += 2
                    continue
                if src[j] == '"':
                    blank(start, j)
                    return j + 1
                if src[j] == "\n":
                    break
            if src.startswith("${", j):
                j = skip_template(j + 1)
                continue
            j += 1
        blank(start, j)
        return j

    i = 0
    while i < n:
        c = src[i]
        if c == "/" and src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            blank(i, j)
            i = j
        elif c == "/" and src.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth += 1
                    j += 2
                elif src.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif c == '"':
            i = skip_string(i)
        elif c == "'":
            m = re.match(r"'(?:\\u[0-9A-Fa-f]{4}|\\.|[^\\'\n])'", src[i:i + 12])
            if m:
                blank(i + 1, i + m.end() - 1)
                i += m.end()
            else:
                i += 1
        elif c == "`":
            j = src.find("`", i + 1)
            i = n if j < 0 else j + 1
        else:
            i += 1
    return "".join(out)


def skip_balanced(text, i, open_c, close_c):
    depth = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c == "`":
            j = text.find("`", i + 1)
            i = n if j < 0 else j + 1
            continue
        if c == open_c:
            depth += 1
        elif c == close_c:
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return n


def skip_ws(text, i):
    n = len(text)
    while i < n and text[i].isspace():
        i += 1
    return i


def function_after_test(text, i):
    """text[i] follows `@Test`: return the name of the function it annotates or None."""
    n = len(text)
    i = skip_ws(text, i)
    if i < n and text[i] == "(":
        i = skip_balanced(text, i, "(", ")")
    while True:
        i = skip_ws(text, i)
        if i >= n:
            return None
        if text[i] == "@":
            i += 1
            if text.startswith("[", i):
                i = skip_balanced(text, i, "[", "]")
                continue
            m = re.match(r"(?:[A-Za-z_]\w*:)?[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*", text[i:])
            if not m:
                return None
            i += m.end()
            j = skip_ws(text, i)
            if j < n and text[j] == "(":
                i = skip_balanced(text, j, "(", ")")
            continue
        m = IDENT.match(text, i)
        if not m:
            return None
        word = m.group(0)
        if word in MODIFIERS:
            i = m.end()
            continue
        if word == "fun":
            fm = FUN_NAME.match(text, m.end())
            return fm.group(1) if fm else None
        return None


def depth_map(text):
    """depth[i] = brace depth before text[i]; backtick names are skipped."""
    depth = [0] * (len(text) + 1)
    d = 0
    i = 0
    n = len(text)
    while i < n:
        depth[i] = d
        c = text[i]
        if c == "`":
            j = text.find("`", i + 1)
            j = n - 1 if j < 0 else j
            for k in range(i + 1, j + 1):
                depth[k] = d
            i = j + 1
            continue
        if c == "{":
            d += 1
        elif c == "}":
            d = max(0, d - 1)
        i += 1
    depth[n] = d
    return depth


def tests_of_source(src, stem):
    """Return [(package, class, function)] for one Kotlin source file."""
    text = clean(src)
    pm = PACKAGE.search(text)
    package = pm.group(1) if pm else ""
    depth = depth_map(text)
    top = None
    for m in CLASS_DECL.finditer(text):
        if depth[m.start()] == 0:
            top = m.group(1)
            break
    if top is None:
        top = stem + "Kt"
    found = []
    for m in TEST_ANN.finditer(text):
        name = function_after_test(text, m.end())
        if name:
            found.append((package, top, name))
    return found


def ids_of(files):
    """files: iterable of (module, set, relpath-in-set, source)."""
    ids = []
    for module, tset, rel, src in files:
        stem = os.path.splitext(os.path.basename(rel))[0]
        for package, cls, fn in tests_of_source(src, stem):
            qual = (package + "." if package else "") + cls
            ids.append("%s/%s:%s#%s" % (module, tset, qual, fn))
    return sorted(ids)


def tree_files(root):
    base = os.path.join(root, "android")
    if not os.path.isdir(base):
        return
    for module in sorted(os.listdir(base)):
        src = os.path.join(base, module, "src")
        if not os.path.isdir(src):
            continue
        for tset in sorted(os.listdir(src)):
            if not (tset in ("test", "androidTest") or tset.endswith("Test")):
                continue
            sdir = os.path.join(src, tset)
            for dirpath, dirnames, filenames in os.walk(sdir):
                dirnames.sort()
                for fn in sorted(filenames):
                    if fn.endswith(".kt"):
                        full = os.path.join(dirpath, fn)
                        with open(full, encoding="utf-8", errors="replace") as f:
                            yield module, tset, os.path.relpath(full, sdir), f.read()


def git(root, *args):
    try:
        r = subprocess.run(["git", "-C", root, *args], capture_output=True, check=False)
    except OSError as e:
        raise UsageError("cannot run git: %s" % e)
    if r.returncode != 0:
        raise UsageError("git %s failed: %s" % (" ".join(args), r.stderr.decode("utf-8", "replace").strip()))
    return r.stdout.decode("utf-8", "replace")


def ref_files(root, ref):
    git(root, "rev-parse", "--verify", "--quiet", ref + "^{commit}")
    listing = git(root, "ls-tree", "-r", "--name-only", ref, "--", "android")
    for path in sorted(listing.splitlines()):
        m = SET_RE.match(path)
        if m:
            yield m.group(1), m.group(2), m.group(3), git(root, "show", "%s:%s" % (ref, path))


def read_moves(path):
    rules = []
    if not os.path.isfile(path):
        return rules
    with open(path, encoding="utf-8") as f:
        for no, line in enumerate(f, 1):
            line = line.rstrip("\n")
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) != 2 or not parts[0] or not parts[1]:
                raise UsageError("%s:%d: expected 'old_prefix<TAB>new_prefix'" % (path, no))
            rules.append((parts[0], parts[1]))
    return rules


def compare(base_ids, head_ids, rules):
    """Return (removed, stale_rules)."""
    stale = [r for r in rules if not any(i.startswith(r[0]) for i in base_ids)]
    head = collections.Counter(head_ids)
    removed = []
    for orig in base_ids:
        if head[orig] > 0:
            head[orig] -= 1
            continue
        mapped = None
        for old, new in rules:
            if orig.startswith(old):
                mapped = new + orig[len(old):]
                break
        if mapped is not None and head[mapped] > 0:
            head[mapped] -= 1
            continue
        removed.append(orig if mapped is None else "%s (rule maps it to %s, not found)" % (orig, mapped))
    return removed, stale


def main(argv):
    root = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
    moves = None
    mode = None
    ref = None
    args = list(argv)
    try:
        while args:
            a = args.pop(0)
            if a == "--list":
                mode = mode or "list"
            elif a == "--base":
                if not args:
                    raise UsageError("--base needs a ref")
                mode = "base"
                ref = args.pop(0)
            elif a == "--root":
                if not args:
                    raise UsageError("--root needs a directory")
                root = os.path.abspath(args.pop(0))
            elif a == "--moves":
                if not args:
                    raise UsageError("--moves needs a file")
                moves = args.pop(0)
            elif a in ("-h", "--help"):
                print(__doc__)
                return 0
            else:
                raise UsageError("unknown argument: " + a)
        if mode is None:
            raise UsageError("usage: kotlin_test_inventory.py --list | --base REF [--root DIR] [--moves FILE]")
        head_ids = ids_of(tree_files(root))
        if mode == "list":
            for i in head_ids:
                print(i)
            return 0
        rules = read_moves(moves or os.path.join(root, "scripts", "android", "kotlin_test_moves.tsv"))
        base_ids = ids_of(ref_files(root, ref))
    except UsageError as e:
        print(str(e), file=sys.stderr)
        return 2
    removed, stale = compare(base_ids, head_ids, rules)
    for r in removed:
        print("removed: " + r, file=sys.stderr)
    for old, new in stale:
        print("stale rule (matches nothing at %s): %s -> %s" % (ref, old, new), file=sys.stderr)
    ok = not removed and not stale
    print("KTESTS | total | %s | base %d, head %d, removed %d" % (
        "PASS" if ok else "FAIL", len(base_ids), len(head_ids), len(removed)))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
