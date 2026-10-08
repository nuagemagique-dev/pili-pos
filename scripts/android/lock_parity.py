#!/usr/bin/env python3
"""Lock parity of two `cargo tree --prefix none` outputs (PD59).

Usage: lock_parity.py ROOT_TREE ANDROID_TREE [--allow FILE]

Each tree file is the output of
`cargo tree ... -e normal,build --target all --prefix none`: one package per
line, `name vX.Y.Z`, optionally followed by ` (path)`, ` (proc-macro)`, ` (*)`
and feature suffixes. The (name, version) pairs of both trees must be identical
(a repeated package counts once; blank lines are ignored).

The allow file holds one difference per line: `name version reason...`. The
reason is mandatory; a line starting with `#` is a comment. An allowed difference is
printed as ALLOWED. An allow line that matches no difference is stale and is an
error. A (name, version) pair is a difference when it is in one tree only, so a
version bump needs two allow lines (the old and the new version).

Exit: 0 no unallowed difference, 1 differences or stale allow lines,
2 usage or file format error.
Success line: `lock parity: N packages, identical` or
`lock parity: N packages, identical, K allowed`.
"""

import re
import sys

ROOT_ONLY = "root only"
ANDROID_ONLY = "android only"

_LINE = re.compile(r"^(\S+) v(\S+)")


def parse_tree(text):
    """Set of (name, version) of a cargo tree --prefix none output."""
    pairs = set()
    for raw in text.splitlines():
        line = raw.strip()
        if not line:
            continue
        # "(...)" groups are paths, proc-macro markers or "(*)": drop them first.
        line = re.sub(r"\([^)]*\)", " ", line).strip()
        m = _LINE.match(line)
        if m:
            pairs.add((m.group(1), m.group(2)))
    return pairs


class AllowError(ValueError):
    pass


def parse_allow(text):
    """Dict (name, version) -> reason. Raises AllowError on a bad line."""
    allowed = {}
    for n, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split(None, 2)
        if len(parts) < 3 or not parts[2].strip():
            raise AllowError(
                "allow line %d: `name version reason...` needs a reason: %s" % (n, line)
            )
        key = (parts[0], parts[1].lstrip("v"))
        if key in allowed:
            raise AllowError("allow line %d: duplicate entry %s %s" % (n, key[0], key[1]))
        allowed[key] = parts[2].strip()
    return allowed


def compare(root, android, allowed):
    """Return (common_count, diffs, allowed_hits, stale).

    diffs: sorted list of (name, version, side) not allowed.
    allowed_hits: sorted list of (name, version, side, reason).
    stale: sorted list of (name, version, reason) matching nothing.
    """
    items = [(n, v, ROOT_ONLY) for n, v in root - android]
    items += [(n, v, ANDROID_ONLY) for n, v in android - root]
    items.sort()
    diffs, hits, used = [], [], set()
    for n, v, side in items:
        if (n, v) in allowed:
            hits.append((n, v, side, allowed[(n, v)]))
            used.add((n, v))
        else:
            diffs.append((n, v, side))
    stale = sorted((n, v, r) for (n, v), r in allowed.items() if (n, v) not in used)
    return len(root & android), diffs, hits, stale


def _explain(diffs, root, android):
    """Group unallowed differences by package name for the report."""
    names = sorted({n for n, _, _ in diffs})
    out = []
    for name in names:
        rv = sorted(v for n, v in root if n == name)
        av = sorted(v for n, v in android if n == name)
        if rv and av:
            out.append(
                "DIFF %s: version differs, root %s, android %s"
                % (name, ", ".join(rv), ", ".join(av))
            )
        elif rv:
            out.append("DIFF %s: only in root (%s)" % (name, ", ".join(rv)))
        else:
            out.append("DIFF %s: only in android (%s)" % (name, ", ".join(av)))
    return out


def main(argv, out=None, err=None):
    out = out or sys.stdout
    err = err or sys.stderr
    args = list(argv)
    allow_path = None
    paths = []
    while args:
        a = args.pop(0)
        if a == "--allow":
            if not args:
                print("--allow needs a file", file=err)
                return 2
            allow_path = args.pop(0)
        elif a in ("-h", "--help"):
            print(__doc__, file=out)
            return 0
        elif a.startswith("-"):
            print("unknown argument: %s" % a, file=err)
            return 2
        else:
            paths.append(a)
    if len(paths) != 2:
        print("usage: lock_parity.py ROOT_TREE ANDROID_TREE [--allow FILE]", file=err)
        return 2
    try:
        with open(paths[0], encoding="utf-8") as f:
            root = parse_tree(f.read())
        with open(paths[1], encoding="utf-8") as f:
            android = parse_tree(f.read())
        allowed = {}
        if allow_path:
            with open(allow_path, encoding="utf-8") as f:
                allowed = parse_allow(f.read())
    except (OSError, AllowError) as e:
        print("error: %s" % e, file=err)
        return 2
    for label, s, p in (("root", root, paths[0]), ("android", android, paths[1])):
        if not s:
            print("error: no package found in the %s tree %s" % (label, p), file=err)
            return 2

    common, diffs, hits, stale = compare(root, android, allowed)
    for n, v, side, reason in hits:
        print("ALLOWED %s %s (%s): %s" % (n, v, side, reason), file=out)
    for line in _explain(diffs, root, android):
        print(line, file=out)
    for n, v, r in stale:
        print("STALE allow line: %s %s matches no difference (%s)" % (n, v, r), file=out)
    if diffs or stale:
        parts = []
        if diffs:
            parts.append("%d differing package(s)" % len({n for n, _, _ in diffs}))
        if stale:
            parts.append("%d stale allow line(s)" % len(stale))
        print("lock parity: FAIL, %s" % ", ".join(parts), file=out)
        return 1
    if hits:
        print("lock parity: %d packages, identical, %d allowed" % (common, len(hits)), file=out)
    else:
        print("lock parity: %d packages, identical" % common, file=out)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
