#!/usr/bin/env python3
"""Check that the UniFFI facade (android/rust/pos-ffi) mirrors the cert commands.

Default run (parity): the cert set is the Tauri command map
(check_ipc_wrappers.py --map) minus the commands of the modules gated by
the peppol feature in src-tauri/src/ipc/mod.rs. Every method of
`#[uniffi::export] impl PosEngine` (except open, shutdown, events) must be
a cert command, be `async`, take the wrapper's parameter names in the same
order, call the map's engine function and use self.blocking for a sync
engine function or self.spawn for an async one.

  --repo DIR            repository root (default: from this script's path)
  --ffi-dir DIR         pos-ffi sources (default: android/rust/pos-ffi/src)
  --strict              exit 1 when a cert command has no valid FFI method
  --require-modules M   comma list of engine modules that must be complete
  --hygiene             FFI hygiene rules H1 to H7 (see HYGIENE below)
  --markdown            the parity table for the docs
  --parity              run the parity check even with --hygiene/--markdown

HYGIENE rules: H1 every exported fn is async (except PosEngine::open);
H2 cfg(target_os only in logging.rs; H3 records/enums of the contract files
derive Serialize + Deserialize and records carry deny_unknown_fields
(unless `// contract: custom` sits above them); H4 unique uniffi type
names; H5 no process-wide statics outside logging.rs; H6 no unsigned
integers except Vec<u8> in records, enums and exported signatures; H7 no
peppol or invoice names.

Standard library only.
"""

import argparse
import importlib.util
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
DEFAULT_FFI = Path("android") / "rust" / "pos-ffi" / "src"

IDENT = r"[A-Za-z_][A-Za-z0-9_]*"
EXTRAS = ("open", "shutdown", "events")
ENGINE_TYPE = "PosEngine"
UNSIGNED_RE = re.compile(r"\b(?:u8|u16|u32|u64|u128|usize)\b")
STATIC_PATTERNS = [
    (re.compile(r"(?<!')\bstatic\b"), "static"),
    (re.compile(r"\bthread_local\s*!"), "thread_local!"),
    (re.compile(r"\blazy_static\s*!"), "lazy_static!"),
    (re.compile(r"\bOnceLock\b"), "OnceLock"),
    (re.compile(r"\bOnceCell\b"), "OnceCell"),
    (re.compile(r"\bLazyLock\b"), "LazyLock"),
]
H7_RE = re.compile(r"peppol|invoice", re.I)
H3_FILES = {"events.rs", "helpers.rs", "install.rs", "core.rs"}
H3_DIRS = ("records/", "api/")
UNIFFI_KINDS = ("Record", "Enum", "Object", "Error")


def load_ipc_module():
    spec = importlib.util.spec_from_file_location("check_ipc_wrappers", HERE / "check_ipc_wrappers.py")
    mod = importlib.util.module_from_spec(spec)
    sys.modules.setdefault("check_ipc_wrappers", mod)
    spec.loader.exec_module(mod)
    return mod


# ---------------------------------------------------------------- Rust text


def strip_rust(text, keep_strings=False):
    """Blank comments (and string contents unless keep_strings), keeping offsets."""
    out = []
    i = 0
    n = len(text)

    def blank(s):
        return "".join("\n" if ch == "\n" else " " for ch in s)

    while i < n:
        c = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(blank(text[i:j]))
            i = j
        elif c == "/" and nxt == "*":
            depth = 0
            j = i
            while j < n:
                if text.startswith("/*", j):
                    depth += 1
                    j += 2
                elif text.startswith("*/", j):
                    depth -= 1
                    j += 2
                    if depth == 0:
                        break
                else:
                    j += 1
            out.append(blank(text[i:j]))
            i = j
        elif c in "rb" and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            m = re.compile(r'b?r(#*)"').match(text, i)
            if m:
                end_marker = '"' + m.group(1)
                j = text.find(end_marker, m.end())
                j = n if j < 0 else j + len(end_marker)
                out.append(text[i:j] if keep_strings else text[i : m.end()] + blank(text[m.end() : j - len(end_marker)]) + end_marker)
                i = j
            else:
                out.append(c)
                i += 1
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            j = min(j, n - 1)
            inner = text[i + 1 : j]
            out.append('"' + (inner if keep_strings else blank(inner)) + '"')
            i = j + 1
        elif c == "'":
            if nxt == "\\":
                j = text.find("'", i + 3)
                j = n - 1 if j < 0 else j
                out.append("'" + blank(text[i + 1 : j]) + "'")
                i = j + 1
            elif i + 2 < n and text[i + 2] == "'" and nxt != "'":
                out.append("' '")
                i += 3
            else:
                out.append(c)  # a lifetime
                i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def match_close(text, start, open_ch, close_ch):
    depth = 0
    for i in range(start, len(text)):
        if text[i] == open_ch:
            depth += 1
        elif text[i] == close_ch:
            depth -= 1
            if depth == 0:
                return i
    raise ValueError("unbalanced bracket")


def match_open_back(text, end, open_ch, close_ch):
    depth = 0
    for i in range(end, -1, -1):
        if text[i] == close_ch:
            depth += 1
        elif text[i] == open_ch:
            depth -= 1
            if depth == 0:
                return i
    raise ValueError("unbalanced bracket")


def split_top(text, sep=","):
    parts, depth, cur = [], 0, []
    for i, c in enumerate(text):
        if c in "([{<":
            depth += 1
        elif c in ")]}>":
            if not (c == ">" and i > 0 and text[i - 1] == "-"):
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


def depth_map(text):
    """Bracket depth before each char (for (), [] and {})."""
    depths = []
    d = 0
    for c in text:
        if c in ")]}":
            d -= 1
        depths.append(d)
        if c in "([{":
            d += 1
    return depths


# ---------------------------------------------------------------- FFI parser


class Fn:
    def __init__(self, rel, line, name, is_async, params, sig, body, owner, ctor, exported):
        self.rel = rel
        self.line = line
        self.name = name
        self.is_async = is_async
        self.params = params
        self.sig = sig
        self.body = body
        self.owner = owner
        self.ctor = ctor
        self.exported = exported

    @property
    def param_names(self):
        names = []
        for p in self.params:
            p = collapse(p)
            if re.match(r"^&?\s*(?:'\w+\s+)?(?:mut\s+)?self\b", p):
                continue
            name = p.partition(":")[0].strip()
            name = re.sub(r"^mut\s+", "", name)
            names.append(name)
        return names


class TypeDef:
    def __init__(self, rel, line, kind, name, uniffi, derives, attrs, body, contract_custom):
        self.rel = rel
        self.line = line
        self.kind = kind  # struct | enum
        self.name = name
        self.uniffi = uniffi  # set of Record/Enum/Object/Error
        self.derives = derives  # last path segments
        self.attrs = attrs
        self.body = body
        self.contract_custom = contract_custom


class Source:
    def __init__(self, rel, raw):
        self.rel = rel
        self.raw = raw
        self.blank = strip_rust(raw)
        self.keep = strip_rust(raw, keep_strings=True)
        self.depths = depth_map(self.blank)
        self.errors = []

    def line_of(self, pos):
        return self.blank.count("\n", 0, pos) + 1


FN_RE = re.compile(
    rf"(?:\bpub(?:\s*\([^)]*\))?\s+)?(?:\b(async)\s+)?(?:\bunsafe\s+)?\bfn\s+({IDENT})"
)


def read_fn(src, m, owner, ctor, exported, attrs_from=None):
    text = src.blank
    open_paren = text.index("(", m.end())
    close_paren = match_close(text, open_paren, "(", ")")
    j = close_paren + 1
    while j < len(text) and text[j] not in "{;":
        j += 1
    sig = text[m.start() : j]
    body = ""
    end = j
    if j < len(text) and text[j] == "{":
        end = match_close(text, j, "{", "}")
        body = text[j + 1 : end]
    params = split_top(text[open_paren + 1 : close_paren])
    fn = Fn(src.rel, src.line_of(m.start()), m.group(2), bool(m.group(1)), params, sig, body, owner, ctor, exported)
    return fn, end


def parse_exports(src):
    """Exported fns (free and in `#[uniffi::export] impl`), all `impl` blocks seen."""
    text = src.blank
    fns = []
    for am in re.finditer(r"#\s*\[\s*uniffi::export\b", text):
        try:
            attr_end = match_close(text, text.index("[", am.start()), "[", "]")
        except ValueError:
            src.errors.append((src.line_of(am.start()), "unbalanced attribute"))
            continue
        pos = attr_end + 1
        while True:  # further attributes
            m = re.compile(r"\s*#\s*\[").match(text, pos)
            if not m:
                break
            pos = match_close(text, m.end() - 1, "[", "]") + 1
        ws = re.compile(r"\s*").match(text, pos)
        fm = FN_RE.match(text, ws.end())
        if fm:
            try:
                fn, _ = read_fn(src, fm, None, False, True)
            except ValueError as e:
                src.errors.append((src.line_of(fm.start()), str(e)))
                continue
            fns.append(fn)
            continue
        im = re.compile(r"(?:pub\s+)?(?:unsafe\s+)?impl\b").match(text, ws.end())
        if not im:
            continue  # export of a trait or other item: not checked
        brace = text.find("{", im.end())
        if brace < 0:
            continue
        header = collapse(text[im.end() : brace])
        header = re.sub(r"\bwhere\b.*$", "", header).strip()
        if " for " in header:
            header = header.split(" for ", 1)[1]
        else:
            header = re.sub(r"^<[^>]*>\s*", "", header)
        tm = re.match(IDENT, header)
        owner = tm.group(0) if tm else "?"
        try:
            end = match_close(text, brace, "{", "}")
        except ValueError as e:
            src.errors.append((src.line_of(brace), str(e)))
            continue
        want_depth = src.depths[brace] + 1
        prev_end = brace + 1
        for fm in FN_RE.finditer(text, brace + 1, end):
            if src.depths[fm.start()] != want_depth or fm.start() < prev_end:
                continue
            attrs = text[prev_end : fm.start()]
            try:
                fn, fn_end = read_fn(src, fm, owner, "uniffi::constructor" in attrs, True)
            except ValueError as e:
                src.errors.append((src.line_of(fm.start()), str(e)))
                continue
            fns.append(fn)
            prev_end = fn_end + 1
    return fns


def attrs_before(text, pos):
    """The attribute run just before `pos`: (list of attr texts, start offset)."""
    attrs = []
    start = pos
    while True:
        s = text[:start].rstrip()
        vis = re.search(r"\bpub(?:\s*\([^)]*\))?$", s)
        if vis and start == pos:
            s = s[: vis.start()].rstrip()
            start = len(s)
        if not s.endswith("]"):
            break
        try:
            lb = match_open_back(s, len(s) - 1, "[", "]")
        except ValueError:
            break
        hash_at = s[:lb].rstrip()
        if not hash_at.endswith("#"):
            break
        attrs.append(s[lb : len(s)])
        start = len(hash_at) - 1
    attrs.reverse()
    return attrs, start


def parse_types(src):
    text = src.blank
    raw_lines = src.raw.split("\n")
    types = []
    for m in re.finditer(rf"\b(struct|enum)\s+({IDENT})", text):
        attrs, first = attrs_before(text, m.start())
        derives = []
        for a in attrs:
            dm = re.search(r"\bderive\s*\(", a)
            if dm:
                inner_start = dm.end() - 1
                try:
                    inner = a[inner_start + 1 : match_close(a, inner_start, "(", ")")]
                except ValueError:
                    continue
                derives += [collapse(d) for d in split_top(inner)]
        uniffi = {d.split("::")[-1] for d in derives if re.search(r"(^|::)uniffi::(?:%s)$" % "|".join(UNIFFI_KINDS), d)}
        last = {d.split("::")[-1] for d in derives}
        # body: first of { ( ; after the name (generics skipped)
        j = m.end()
        while j < len(text) and text[j] not in "{(;":
            j += 1
        body = ""
        if j < len(text) and text[j] in "{(":
            close = ")" if text[j] == "(" else "}"
            try:
                body = text[j + 1 : match_close(text, j, text[j], close)]
            except ValueError:
                body = text[j + 1 :]
        # contract marker: contiguous comment lines above the first attribute,
        # or comment lines inside the attribute run
        first_line = src.line_of(first) if attrs else src.line_of(m.start())
        item_line = src.line_of(m.start())
        marker = False
        k = first_line - 2
        while k >= 0 and raw_lines[k].strip().startswith("//"):
            if raw_lines[k].strip() == "// contract: custom":
                marker = True
            k -= 1
        for k in range(first_line - 1, item_line - 1):
            if raw_lines[k].strip() == "// contract: custom":
                marker = True
        types.append(TypeDef(src.rel, item_line, m.group(1), m.group(2), uniffi, last, attrs, body, marker))
    return types


def load_ffi(ffi_dir):
    sources = []
    for f in sorted(Path(ffi_dir).rglob("*.rs")):
        rel = f.relative_to(ffi_dir).as_posix()
        sources.append(Source(rel, f.read_text()))
    return sources


# ---------------------------------------------------------------- cert set


def build_cert(args, ipc):
    """dict command -> (engine module, engine fn, 'sync'|'async', [param names])."""
    repo = Path(args.repo)
    ipc.ROOT = repo
    ipc.IPC_DIR = repo / "src-tauri" / "src" / "ipc"
    ipc.MAIN_RS = repo / "src-tauri" / "src" / "main.rs"
    rows = []
    sig_params = {}
    if args.map_file:
        for line in Path(args.map_file).read_text().splitlines():
            if line.strip():
                rows.append(tuple(line.split("\t")))
    else:
        for f in sorted(ipc.IPC_DIR.glob("*.rs")):
            for w in ipc.parse_wrappers(f):
                problem, _, module, engine_fn = ipc.check_wrapper(w)
                if problem:
                    raise SystemExit(f"{f.name}:{w.line}: {w.name}: {problem}")
                rows.append((w.name, module, engine_fn, "async" if w.is_async else "sync"))
    if args.signatures_file:
        for line in Path(args.signatures_file).read_text().splitlines():
            if line.strip():
                parts = line.split("|")
                sig_params[parts[0]] = param_names_of(parts[2])
    else:
        for f in sorted(ipc.IPC_DIR.glob("*.rs")):
            for w in ipc.parse_wrappers(f):
                sig_params[w.name] = param_names_of(ipc.render_signature(w).split("|")[2])
    gated = gated_modules(ipc)
    gated_cmds = set()
    for g in gated:
        p = ipc.IPC_DIR / f"{g}.rs"
        if p.exists():
            gated_cmds |= {w.name for w in ipc.parse_wrappers(p)}
    cert = {}
    for name, module, engine_fn, kind in rows:
        if name in gated_cmds or module in gated:
            continue
        cert[name] = (module, engine_fn, kind, sig_params.get(name, []))
    return cert


def param_names_of(param_text):
    names = []
    for p in split_top(param_text):
        name = p.partition(":")[0].strip()
        if name and name != "state":
            names.append(name)
    return names


def gated_modules(ipc):
    path = ipc.IPC_DIR / "mod.rs"
    if not path.exists():
        return set()
    text = "\n".join(l for l in path.read_text().split("\n") if not l.lstrip().startswith("//"))
    return set(re.findall(rf'#\[cfg\(feature = "peppol"\)\]\s*pub\s+mod\s+({IDENT})\s*;', text))


# ---------------------------------------------------------------- parity


def camel(name):
    head, *rest = name.split("_")
    return head + "".join(p[:1].upper() + p[1:] for p in rest)


def engine_methods(sources):
    fns = []
    for s in sources:
        fns += [f for f in parse_exports(s) if f.owner == ENGINE_TYPE and not f.ctor]
    return fns


def call_paths(body, engine_fn):
    return re.findall(rf"((?:{IDENT}\s*::\s*)*{re.escape(engine_fn)})\s*\(", body)


def check_method(fn, cert):
    """Issues of one PosEngine method."""
    issues = []
    if fn.name not in cert:
        return [f"`{fn.name}` is not a cert command"]
    module, engine_fn, kind, names = cert[fn.name]
    if not fn.is_async:
        issues.append(f"`{fn.name}` must be `pub async fn`")
    if fn.param_names != names:
        issues.append(
            f"parameters ({', '.join(fn.param_names)}) differ from the wrapper ({', '.join(names)})"
        )
    paths = [re.sub(r"\s+", "", p) for p in call_paths(fn.body, engine_fn)]
    if not paths:
        issues.append(f"body does not call the engine function {engine_fn}")
    else:
        bad = [p for p in paths if "::" in p and p.split("::")[-2] != module]
        if len(bad) == len(paths):
            issues.append(f"body calls {bad[0]}, expected module {module}")
    uses_spawn = bool(re.search(r"\bself\s*\.\s*spawn\s*\(", fn.body))
    uses_blocking = bool(re.search(r"\bself\s*\.\s*blocking\s*\(", fn.body))
    if kind == "sync" and uses_spawn:
        issues.append(f"{engine_fn} is sync: use self.blocking, not self.spawn")
    elif kind == "async" and uses_blocking:
        issues.append(f"{engine_fn} is async: use self.spawn, not self.blocking")
    elif not (uses_spawn or uses_blocking):
        issues.append("body uses neither self.blocking nor self.spawn")
    return issues


def run_parity(args, cert, sources, ffi_exists, ffi_dir, out):
    total = len(cert)
    if not ffi_exists:
        print(f"parity: 0/{total} (no pos-ffi yet)", file=out)
        return 1 if args.strict else 0
    issues = []
    methods = {}
    pre = Path(ffi_dir).name
    for s in sources:
        for line, msg in s.errors:
            issues.append(f"{pre}/{s.rel}:{line}: parse error: {msg}")
    for fn in engine_methods(sources):
        if fn.name in EXTRAS:
            continue
        if fn.name in methods:
            issues.append(f"{pre}/{fn.rel}:{fn.line}: {fn.name}: duplicate method (first in {methods[fn.name].rel}:{methods[fn.name].line})")
            continue
        methods[fn.name] = fn
    ok_cmds = set()
    for name in sorted(methods):
        fn = methods[name]
        found = check_method(fn, cert)
        if found:
            issues += [f"{pre}/{fn.rel}:{fn.line}: {name}: {msg}" for msg in found]
        else:
            ok_cmds.add(name)
    for line in issues:
        print(line, file=out)
    missing = sorted(set(cert) - set(methods))
    print(f"parity: {len(ok_cmds)}/{total} (wrapper, engine fn, FFI method)", file=out)
    print("missing: " + (", ".join(missing) if missing else "none"), file=out)
    code = 1 if issues else 0
    if args.strict and len(ok_cmds) < total:
        code = 1
    if args.require_modules:
        for mod in [m.strip() for m in args.require_modules.split(",") if m.strip()]:
            cmds = sorted(c for c, v in cert.items() if v[0] == mod)
            if not cmds:
                print(f"require-modules: unknown engine module {mod}", file=out)
                code = 1
                continue
            lacking = [c for c in cmds if c not in ok_cmds]
            if lacking:
                print(f"require-modules: {mod}: missing {', '.join(lacking)}", file=out)
                code = 1
    return code


def run_markdown(args, cert, sources, ffi_exists, out):
    present = {f.name for f in engine_methods(sources)} if ffi_exists else set()
    print("| Command | Engine function | Call | FFI method (Kotlin) |", file=out)
    print("| --- | --- | --- | --- |", file=out)
    found = 0
    for name in sorted(cert):
        module, engine_fn, kind, _ = cert[name]
        call = "blocking" if kind == "sync" else "spawn"
        if name in present:
            found += 1
            method = f"posEngine.{camel(name)}()"
        else:
            method = "missing"
        print(f"| {name} | {module}::{engine_fn} | {call} | {method} |", file=out)
    print("", file=out)
    print(f"Parity: {found}/{len(cert)}.", file=out)
    return 0


# ---------------------------------------------------------------- hygiene


def run_hygiene(args, sources, ffi_exists, ffi_dir, out):
    if not ffi_exists:
        print("hygiene: no pos-ffi yet", file=out)
        return 0
    viol = []
    prefix = Path(ffi_dir).name

    def add(src, line, rule, msg):
        viol.append((src.rel, line, f"{prefix}/{src.rel}:{line}: {rule}: {msg}"))

    n_exports = 0
    all_types = []
    for src in sources:
        for line, msg in src.errors:
            add(src, line, "parse", msg)
        # H1 + H6 on exported fns
        for fn in parse_exports(src):
            n_exports += 1
            if not fn.is_async and not (fn.ctor and fn.name == "open" and fn.owner == ENGINE_TYPE):
                add(src, fn.line, "H1", f"exported fn `{fn.name}` is not async")
            if fn.ctor and not (fn.name == "open" and fn.owner == ENGINE_TYPE):
                add(src, fn.line, "H1", f"constructor `{fn.name}` is not allowed (only PosEngine::open)")
            sig = re.sub(r"Vec\s*<\s*u8\s*>", "Vec<>", fn.sig)
            for um in UNSIGNED_RE.finditer(sig):
                add(src, fn.line, "H6", f"`{um.group(0)}` in the signature of `{fn.name}`")
        # H2
        in_logging = src.rel == "logging.rs" or src.rel.startswith("logging/")
        if not in_logging:
            for m in re.finditer(r"\btarget_os\b", src.blank):
                add(src, src.line_of(m.start()), "H2", "cfg(target_os) outside logging.rs")
            # H5
            for rx, label in STATIC_PATTERNS:
                for m in rx.finditer(src.blank):
                    add(src, src.line_of(m.start()), "H5", f"`{label}` outside logging.rs")
        # H7
        for m in H7_RE.finditer(src.keep):
            add(src, src.line_of(m.start()), "H7", f"`{m.group(0)}` is not part of the cert build")
        # types
        for t in parse_types(src):
            if not t.uniffi:
                continue
            all_types.append(t)
            record_or_enum = t.uniffi & {"Record", "Enum"}
            scoped = src.rel in H3_FILES or src.rel.startswith(H3_DIRS)
            if record_or_enum and scoped:
                for need in ("Serialize", "Deserialize"):
                    if need not in t.derives:
                        add(src, t.line, "H3", f"{t.name} does not derive {need}")
                if "Record" in t.uniffi and t.kind == "struct" and not t.contract_custom:
                    if not any("deny_unknown_fields" in a for a in t.attrs):
                        add(src, t.line, "H3", f"{t.name} lacks #[serde(deny_unknown_fields)] (or `// contract: custom`)")
            if record_or_enum:
                body = re.sub(r"Vec\s*<\s*u8\s*>", "Vec<>", t.body)
                seen = set()
                for um in UNSIGNED_RE.finditer(body):
                    if um.group(0) not in seen:
                        seen.add(um.group(0))
                        add(src, t.line, "H6", f"`{um.group(0)}` in {t.name}")
    # H4
    by_name = {}
    for t in all_types:
        by_name.setdefault(t.name, []).append(t)
    for name, defs in sorted(by_name.items()):
        if len(defs) > 1:
            for t in defs[1:]:
                viol.append(
                    (t.rel, t.line, f"{prefix}/{t.rel}:{t.line}: H4: type `{name}` is also defined in {defs[0].rel}:{defs[0].line}")
                )
    if not viol:
        print(f"hygiene: ok ({n_exports} exports, {len(all_types)} types)", file=out)
        return 0
    for _, _, line in sorted(set(viol), key=lambda v: (v[0], v[1], v[2])):
        print(line, file=out)
    return 1


# ---------------------------------------------------------------- main


def main(argv=None, out=None):
    out = out or sys.stdout
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--repo", default=str(ROOT))
    ap.add_argument("--ffi-dir", default=None)
    ap.add_argument("--strict", action="store_true")
    ap.add_argument("--require-modules", default="")
    ap.add_argument("--hygiene", action="store_true")
    ap.add_argument("--markdown", action="store_true")
    ap.add_argument("--parity", action="store_true")
    ap.add_argument("--map-file", help=argparse.SUPPRESS)
    ap.add_argument("--signatures-file", help=argparse.SUPPRESS)
    args = ap.parse_args(argv)
    args.repo = str(Path(args.repo).resolve())
    ffi_dir = Path(args.ffi_dir) if args.ffi_dir else Path(args.repo) / DEFAULT_FFI
    ffi_exists = ffi_dir.is_dir()
    sources = load_ffi(ffi_dir) if ffi_exists else []
    modes = []
    if args.markdown:
        modes.append("markdown")
    if args.hygiene:
        modes.append("hygiene")
    if args.parity or not modes:
        modes.insert(0, "parity")
    ipc = load_ipc_module()
    cert = None
    code = 0
    for mode in modes:
        if mode == "hygiene":
            code = max(code, run_hygiene(args, sources, ffi_exists, ffi_dir, out))
            continue
        if cert is None:
            cert = build_cert(args, ipc)
        if mode == "markdown":
            code = max(code, run_markdown(args, cert, sources, ffi_exists, out))
        else:
            code = max(code, run_parity(args, cert, sources, ffi_exists, ffi_dir, out))
    return code


if __name__ == "__main__":
    sys.exit(main())
