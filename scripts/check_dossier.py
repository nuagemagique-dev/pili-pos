#!/usr/bin/env python3
"""check_dossier.py - deterministic dossier/code consistency checker.

Scans docs/SUBMISSION_ARTIFACTS/**/*.md (or $PILI_DOSSIER_DIR/**/*.md) (skipping pdf_out and any evidence
JSON, which are not markdown anyway) and cross-checks it against the code
and deploy tree. Each finding is printed as:

    path:line: [rule] message

The process exits 1 if any finding was produced by an active rule, 0
otherwise. This script is python3 stdlib only, on purpose (no venv, no
pip install, so it runs the same locally and in CI).

Rules (stable names, selectable with --rules a,b):
    rust-ident     backticked Rust paths/functions must exist in the code
    repo-path      backticked repo-relative or absolute install paths must
                   exist (relative: on disk; absolute: in the set collected
                   from deploy/ and src-tauri/src)
    brand          "tokyo sushi", "tokyosushi", "tokyosushibar", and
                   "nuage magique" must not appear (this is a brand-neutral
                   certification dossier)
    em-dash        no U+2014 character anywhere
    placeholder    no leftover "TODO"/"TBD"/"XXX"/"FIXME"/"a produire"/
                   "a reformater" markers, unless allowlisted

--final adds five more checks meant for the last pre-submission pass:
    screenshots    every screenshot filename referenced in the capture
                   runbook exists on disk
    pdf-freshness  no PDF in pdf_out/ is older than its source .md
    manifest       MANIFEST.txt (if present) only lists files that exist
                   and never lists internal-only working documents
    pending        no "A effectuer" (to be done) left in a document listed
                   in MANIFEST.txt (a listed PDF is checked through its
                   source .md)
    captures-pdf   CAPTURES_ECRAN_FR.pdf is not older than the newest
                   screenshot in screenshots/

Usage:
    scripts/check_dossier.py
    scripts/check_dossier.py --final
    scripts/check_dossier.py --rules em-dash,brand
"""

import argparse
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

# Built from the Unicode code point on purpose, never typed literally: this
# script's own job is to detect this character, so it must never contain it.
EM_DASH = chr(0x2014)

ROOT = Path(__file__).resolve().parent.parent
# PILI_DOSSIER_DIR (relative to the repository root) points the checker at another dossier,
# e.g. docs/android/dossier; the default is the Linux dossier.
DOSSIER_REL = os.environ.get("PILI_DOSSIER_DIR", "docs/SUBMISSION_ARTIFACTS")
DOSSIER_DIR = ROOT / DOSSIER_REL

ALL_RULES = [
    "rust-ident",
    "repo-path",
    "brand",
    "em-dash",
    "placeholder",
    "screenshots",
    "pdf-freshness",
    "manifest",
    "pending",
    "captures-pdf",
]
DEFAULT_RULES = ["rust-ident", "repo-path", "brand", "em-dash", "placeholder"]
FINAL_EXTRA_RULES = ["screenshots", "pdf-freshness", "manifest", "pending", "captures-pdf"]

ALLOWLIST_PATH = ROOT / "scripts" / "check_dossier_allowlist.txt"

BACKTICK_RE = re.compile(r"`([^`\n]+)`")
IDENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
BARE_CALL_RE = re.compile(r"^[a-z_][a-z0-9_]*$")
GRAPHQL_CAMEL_RE = re.compile(r"^[a-z][a-zA-Z0-9]*$")

RUST_ITEM_RE = re.compile(
    r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:default\s+)?(?:async\s+)?'
    r'(?:unsafe\s+)?(?:extern\s+"[^"]*"\s+)?'
    r"(fn|struct|enum|const|mod|type)\s+([A-Za-z_][A-Za-z0-9_]*)"
)
ENUM_HEAD_RE = re.compile(r"\benum\s+[A-Za-z_][A-Za-z0-9_]*[^{]*\{")
VARIANT_LINE_RE = re.compile(r"^\s*(?:#\[[^\]]*\]\s*)*([A-Za-z_][A-Za-z0-9_]*)\s*[,({=]")

# Rust cast targets that show up in SQL (`amount::numeric`, `id::text`, ...)
# so they never get mistaken for a missing Rust identifier.
SQL_CAST_TARGETS = {
    "numeric", "text", "integer", "int", "int2", "int4", "int8", "bigint",
    "smallint", "boolean", "bool", "timestamp", "timestamptz", "date",
    "time", "uuid", "varchar", "char", "jsonb", "json", "real", "float",
    "float4", "float8", "decimal", "bytea", "serial", "bigserial",
}

REPO_PREFIXES = ("crates/", "src-tauri/", "ui/", "deploy/", "scripts/", "docs/")
ABS_PREFIXES = ("/etc", "/var/lib", "/opt", "/usr", "/run")
PLACEHOLDER_CHARS = ("<", "*", "$")

BRAND_PATTERNS = ("tokyo sushi", "tokyosushi", "tokyosushibar", "nuage magique")
PLACEHOLDER_MARKERS = ("a produire", "TODO", "TBD", "XXX", "a reformater", "FIXME")
# "a produire" / "a reformater" cover both the plain and accented spelling
# ("a" vs "à") since the match below is case-insensitive and unaccented
# spellings differ only by that one letter; keep the accented forms too so
# a plain grep of this file also finds them.
PLACEHOLDER_MARKERS = (
    "a produire",
    "à produire",
    "TODO",
    "TBD",
    "XXX",
    "a reformater",
    "à reformater",
    "FIXME",
)

MANIFEST_INTERNAL_SUBSTRINGS = (
    "GAP_AUDIT",
    "ASSEMBLAGE_FINAL",
    "DOSSIER_SOUMISSION",
    "RUNBOOK",
    "dossier_review",
)

SCREENSHOT_RE = re.compile(r"(?:A\d{2,3}|\d\d[a-z]?)_[a-z0-9_]+\.(?:png|jpg)")


@dataclass(order=True)
class Finding:
    path: str
    line: int
    rule: str
    message: str = field(compare=False)

    def render(self) -> str:
        return f"{self.path}:{self.line}: [{self.rule}] {self.message}"


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def rel(path: Path) -> str:
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def iter_md_files(base: Path) -> list:
    if not base.is_dir():
        return []
    out = []
    for dirpath, dirnames, filenames in os.walk(base):
        dirnames[:] = [d for d in dirnames if d != "pdf_out"]
        for name in filenames:
            if name.endswith(".md"):
                out.append(Path(dirpath) / name)
    return sorted(out)


# --------------------------------------------------------------------------
# rust-ident
# --------------------------------------------------------------------------


def collect_enum_variants(text: str) -> set:
    names = set()
    for m in ENUM_HEAD_RE.finditer(text):
        depth = 1
        i = m.end()
        while i < len(text) and depth > 0:
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
            i += 1
        body = text[m.end():max(i - 1, m.end())]
        for line in body.splitlines():
            vm = VARIANT_LINE_RE.match(line)
            if vm:
                names.add(vm.group(1))
    return names


def file_defined_names(path: Path) -> set:
    text = read_text(path)
    names = set()
    for line in text.splitlines():
        m = RUST_ITEM_RE.match(line)
        if m:
            names.add(m.group(2))
    names |= collect_enum_variants(text)
    return names


@dataclass
class RustIndex:
    defined: set
    module_stems: set
    files_by_basename: dict
    file_cache: dict = field(default_factory=dict)

    def defined_in(self, path: Path) -> set:
        key = str(path)
        if key not in self.file_cache:
            self.file_cache[key] = file_defined_names(path)
        return self.file_cache[key]


def build_rust_index(root: Path) -> RustIndex:
    defined = set()
    module_stems = set()
    files_by_basename: dict = {}

    search_dirs = []
    crates_dir = root / "crates"
    if crates_dir.is_dir():
        for crate_dir in sorted(crates_dir.iterdir()):
            for sub in ("src", "tests"):
                d = crate_dir / sub
                if d.is_dir():
                    search_dirs.append(d)
    p = root / "src-tauri/src"
    if p.is_dir():
        search_dirs.append(p)

    for d in search_dirs:
        for rs_file in sorted(d.rglob("*.rs")):
            files_by_basename.setdefault(rs_file.name, []).append(rs_file)
            module_stems.add(rs_file.stem)
            text = read_text(rs_file)
            for line in text.splitlines():
                m = RUST_ITEM_RE.match(line)
                if m:
                    defined.add(m.group(2))
            defined |= collect_enum_variants(text)

    return RustIndex(defined, module_stems, files_by_basename)


def strip_call_and_generics(segment: str) -> str:
    prev = None
    while segment != prev:
        prev = segment
        segment = re.sub(r"\([^()]*\)$", "", segment)
        segment = re.sub(r"<[^<>]*>$", "", segment)
    return segment


def is_graphql_camel(name: str) -> bool:
    return bool(GRAPHQL_CAMEL_RE.match(name)) and any(c.isupper() for c in name)


TOML_SECTION_RE = re.compile(r"^\s*\[([^\]]+)\]\s*$")
TOML_KV_RE = re.compile(r"^\s*([A-Za-z0-9_-]+)\s*=")


def collect_external_crates(root: Path) -> set:
    """Dependency names from every Cargo.toml, minus this workspace's own crates.

    Used to stop a doc reference into a well-known dependency (chrono, std,
    serde, ...) from being treated as a missing local identifier.
    """
    own = {"std", "core", "alloc"}
    deps = set()
    manifests = [root / "Cargo.toml", root / "src-tauri" / "Cargo.toml"]
    crates_dir = root / "crates"
    if crates_dir.is_dir():
        manifests += sorted(crates_dir.glob("*/Cargo.toml"))

    for manifest in manifests:
        if not manifest.is_file():
            continue
        section = ""
        in_package = False
        for line in read_text(manifest).splitlines():
            sm = TOML_SECTION_RE.match(line)
            if sm:
                section = sm.group(1)
                in_package = section == "package"
                continue
            kv = TOML_KV_RE.match(line)
            if not kv:
                continue
            name = kv.group(1)
            if in_package and name == "name":
                m2 = re.search(r'"([^"]+)"', line)
                if m2:
                    own.add(m2.group(1).replace("-", "_"))
            elif "dependencies" in section:
                deps.add(name.replace("-", "_"))

    return deps - own


def try_rust_ident(content: str, external_crates: frozenset = frozenset()):
    """Return a candidate dict if `content` looks like a Rust path/fn, else None."""
    if not content or any(ch in content for ch in PLACEHOLDER_CHARS):
        return None
    if "..." in content or "://" in content or any(c.isspace() for c in content):
        return None
    if "::" in content:
        segments = content.split("::")
        first = segments[0]
        if first.replace("-", "_") in external_crates:
            return None  # e.g. chrono::Utc::now() - not our code
        first_base = first.rsplit("/", 1)[-1]
        if "." in first_base:
            ext = first_base.rsplit(".", 1)[-1].lower()
            if ext != "rs":
                return None  # e.g. pos.json::key, pos.db::table - not Rust
        last = strip_call_and_generics(segments[-1])
        if not IDENT_RE.match(last):
            return None
        if last.lower() in SQL_CAST_TARGETS:
            return None
        if is_graphql_camel(last):
            return None
        file_hint = first if first_base.lower().endswith(".rs") else None
        return {"last": last, "file_hint": file_hint, "raw": content}
    if content.endswith("()"):
        name = content[:-2]
        if BARE_CALL_RE.match(name) and not is_graphql_camel(name):
            return {"last": name, "file_hint": None, "raw": content}
    return None


def source_roots(root: Path) -> list:
    """Where a short source path (`commands/auth.rs`) may live."""
    roots = [root, root / "src-tauri" / "src", root / "src-tauri"]
    crates_dir = root / "crates"
    if crates_dir.is_dir():
        roots += [c / "src" for c in sorted(crates_dir.iterdir())]
    return roots


def resolve_file_hint(root: Path, hint: str, rust_index: RustIndex) -> list:
    if "/" in hint:
        return [r / hint for r in source_roots(root) if (r / hint).is_file()]
    return list(rust_index.files_by_basename.get(hint, []))


def resolve_rust_ident(root: Path, info: dict, rust_index: RustIndex):
    """Return None if the identifier resolves, else an error message."""
    last = info["last"]
    if info["file_hint"]:
        matches = resolve_file_hint(root, info["file_hint"], rust_index)
        if not matches:
            return f"no source file matches `{info['file_hint']}`"
        for m in matches:
            if last in rust_index.defined_in(m) or m.stem == last:
                return None
        shown = ", ".join(rel(m) for m in matches)
        return f"`{last}` not found as fn/struct/enum/const/mod/type in {shown}"
    if last in rust_index.defined or last in rust_index.module_stems:
        return None
    return (
        f"`{last}` not found as fn/struct/enum/const/mod/type or module file "
        "under crates/ or src-tauri"
    )


def rule_rust_ident(md_files: list, rust_index: RustIndex, external_crates: frozenset) -> list:
    findings = []
    for path in md_files:
        text = read_text(path)
        for lineno, line in enumerate(text.splitlines(), 1):
            for m in BACKTICK_RE.finditer(line):
                info = try_rust_ident(m.group(1), external_crates)
                if info is None:
                    continue
                message = resolve_rust_ident(ROOT, info, rust_index)
                if message:
                    findings.append(Finding(rel(path), lineno, "rust-ident", message))
    return findings


# --------------------------------------------------------------------------
# repo-path
# --------------------------------------------------------------------------


def strip_trailing_punct(token: str) -> str:
    return token.rstrip(".,;:)'\"")


def collect_abs_allowed_paths(root: Path) -> set:
    allowed = set()
    # Files the image installs verbatim: deploy/image/**/mkosi.extra/<abs>.
    image_dir = root / "deploy" / "image"
    if image_dir.is_dir():
        for extra in image_dir.rglob("mkosi.extra"):
            for f in extra.rglob("*"):
                installed = "/" + f.relative_to(extra).as_posix()
                allowed.add(installed.rstrip("/"))
    literal_re = re.compile(r'"((?:/etc|/var/lib|/opt|/usr|/run)/[^"]*)"')
    token_re = re.compile(r"(?:/etc|/var/lib|/opt|/usr|/run)(?:/[^\s\"'`)]*)?")

    # The program: the Tauri shell and the Tauri-free engine it calls.
    for rs_root in (root / "src-tauri" / "src", root / "crates" / "pos-engine" / "src"):
        if rs_root.is_dir():
            for rs_file in rs_root.rglob("*.rs"):
                for m in literal_re.finditer(read_text(rs_file)):
                    allowed.add(m.group(1).rstrip("/"))

    deploy_dir = root / "deploy"
    if deploy_dir.is_dir():
        for f in deploy_dir.rglob("*"):
            if not f.is_file():
                continue
            for m in token_re.finditer(expand_shell_vars(read_text(f))):
                token = strip_trailing_punct(m.group(0))
                allowed.add(token.rstrip("/"))
    return allowed


SHELL_ASSIGN_RE = re.compile(
    r'^\s*(?:readonly\s+|local\s+|export\s+)?([A-Z_][A-Z0-9_]*)="?((?:/etc|/var/lib|/opt|/usr|/run)[^"\s]*)"?\s*$',
    re.MULTILINE,
)


def expand_shell_vars(text: str) -> str:
    """Replace `$VAR` / `${VAR}` by the absolute path a script assigns to
    it (`ETC_DIR="/var/lib/pili-etc"`), so `"$ETC_DIR/pili-pos"` is
    seen as `/var/lib/pili-etc/pili-pos`."""
    for name, value in SHELL_ASSIGN_RE.findall(text):
        text = re.sub(r"\$\{" + name + r"\}|\$" + name + r"\b", value, text)
    return text


# SQLite writes these next to the database it names.
SQLITE_SIBLING_SUFFIXES = ("-wal", "-shm", "-journal")
# A date in a file name (`pos.log.YYYY-MM-DD`): the directory is checked.
DATE_PLACEHOLDER_RE = re.compile(r"YYYY|HHMMSS")


def abs_path_known(path: str, allowed: set) -> bool:
    if path in allowed:
        return True
    for suffix in SQLITE_SIBLING_SUFFIXES:
        if path.endswith(suffix) and path[: -len(suffix)] in allowed:
            return True
    parent, _, name = path.rpartition("/")
    return bool(DATE_PLACEHOLDER_RE.search(name)) and parent in allowed


LINE_REF_RE = re.compile(r":\d+(?::\d+)?$")
BRACE_RE = re.compile(r"\{([^{}]*)\}")

# Build outputs (gitignored): named in the docs, never in the repo.
BUILD_OUTPUT_PREFIXES = ("deploy/dist", "src-tauri/target", "target", "ui/dist")


def expand_braces(path: str) -> list:
    """`a/{b,c}.rs` -> [`a/b.rs`, `a/c.rs`]."""
    m = BRACE_RE.search(path)
    if not m:
        return [path]
    out = []
    for alt in m.group(1).split(","):
        out += expand_braces(path[: m.start()] + alt + path[m.end():])
    return out


def missing_relative_paths(candidate: str, doc: Path) -> list:
    """The expansions of `candidate` found nowhere. A relative path is read
    from the repo root, the document's folder, the dossier folder, or a
    source root (`commands/x.rs`)."""
    if candidate.rstrip("/").startswith(BUILD_OUTPUT_PREFIXES):
        return []
    roots = [ROOT, doc.parent, DOSSIER_DIR] + source_roots(ROOT)
    return [
        one
        for one in expand_braces(candidate)
        if not any((r / one).exists() for r in roots)
    ]


def repo_path_candidate(content: str):
    """Return the path to verify (with any `::suffix` or `:line` ref dropped), or None."""
    if not content or any(ch in content for ch in PLACEHOLDER_CHARS):
        return None
    if "..." in content or "://" in content or any(c.isspace() for c in content):
        return None  # ellipsis abbreviation, URL, or a shell command line
    if content.startswith("~"):
        return None  # dev-machine home directory, not a repo path
    base = content.split("::", 1)[0] if "::" in content else content
    base = LINE_REF_RE.sub("", base)
    if not base:
        return None
    for p in ABS_PREFIXES:
        if base == p or base.startswith(p + "/"):
            return base
    looks_relative = ("/" in base and re.search(r"\.[A-Za-z0-9]{1,10}$", base)) or base.startswith(
        REPO_PREFIXES
    )
    if looks_relative:
        return base
    return None


def rule_repo_path(md_files: list, abs_allowed: set, external_crates: frozenset) -> list:
    findings = []
    for path in md_files:
        text = read_text(path)
        for lineno, line in enumerate(text.splitlines(), 1):
            for m in BACKTICK_RE.finditer(line):
                content = m.group(1)
                if try_rust_ident(content, external_crates) is not None:
                    continue  # already owned by rust-ident
                candidate = repo_path_candidate(content)
                if candidate is None:
                    continue
                is_abs = any(
                    candidate == p or candidate.startswith(p + "/") for p in ABS_PREFIXES
                )
                if is_abs:
                    norm = candidate.rstrip("/")
                    if not abs_path_known(norm, abs_allowed):
                        findings.append(
                            Finding(
                                rel(path),
                                lineno,
                                "repo-path",
                                f"absolute path `{norm}` not found in deploy/, src-tauri/src or crates/pos-engine/src",
                            )
                        )
                else:
                    for missing in missing_relative_paths(candidate, path):
                        findings.append(
                            Finding(rel(path), lineno, "repo-path", f"path `{missing}` does not exist")
                        )
    return findings


# --------------------------------------------------------------------------
# brand
# --------------------------------------------------------------------------


def rule_brand(md_files: list) -> list:
    findings = []
    for path in md_files:
        text = read_text(path)
        for lineno, line in enumerate(text.splitlines(), 1):
            lowered = line.lower()
            hits = sorted({p for p in BRAND_PATTERNS if p in lowered})
            if hits:
                findings.append(
                    Finding(rel(path), lineno, "brand", f"forbidden brand term(s): {', '.join(hits)}")
                )
    return findings


# --------------------------------------------------------------------------
# em-dash
# --------------------------------------------------------------------------


def rule_em_dash(md_files: list) -> list:
    findings = []
    for path in md_files:
        text = read_text(path)
        for lineno, line in enumerate(text.splitlines(), 1):
            if EM_DASH in line:
                findings.append(
                    Finding(
                        rel(path),
                        lineno,
                        "em-dash",
                        "line contains an em dash (U+2014); use a comma, colon, "
                        "parentheses, or two hyphens instead",
                    )
                )
    return findings


# --------------------------------------------------------------------------
# placeholder
# --------------------------------------------------------------------------


def load_allowlist(path: Path) -> dict:
    allow: dict = {}
    if not path.is_file():
        return allow
    for raw_line in read_text(path).splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if ":" not in line:
            continue
        file_part, substring = line.split(":", 1)
        allow.setdefault(file_part.strip(), []).append(substring)
    return allow


def rule_placeholder(md_files: list, allowlist: dict) -> list:
    findings = []
    for path in md_files:
        relpath = rel(path)
        allowed_substrings = allowlist.get(relpath, [])
        text = read_text(path)
        for lineno, line in enumerate(text.splitlines(), 1):
            lowered = line.lower()
            for marker in PLACEHOLDER_MARKERS:
                marker_lower = marker.lower()
                start = 0
                while True:
                    idx = lowered.find(marker_lower, start)
                    if idx == -1:
                        break
                    start = idx + 1
                    if any(sub in line for sub in allowed_substrings):
                        continue
                    findings.append(
                        Finding(
                            relpath,
                            lineno,
                            "placeholder",
                            f"unresolved placeholder marker '{marker}'",
                        )
                    )
    return findings


# --------------------------------------------------------------------------
# --final: screenshots
# --------------------------------------------------------------------------


def rule_screenshots(root: Path) -> list:
    candidates = [
        root / DOSSIER_REL / "CAPTURES_ECRAN_RUNBOOK_FR.md",
        root / DOSSIER_REL / "internal" / "CAPTURES_ECRAN_RUNBOOK_FR.md",
        root / "docs" / "internal" / "CAPTURES_ECRAN_RUNBOOK_FR.md",
    ]
    runbook = next((c for c in candidates if c.is_file()), None)
    if runbook is None:
        return []

    screenshot_dirs = [
        root / DOSSIER_REL / "screenshots",
        root / "docs" / "internal" / "screenshots",
    ]

    findings = []
    text = read_text(runbook)
    relpath = rel(runbook)
    for lineno, line in enumerate(text.splitlines(), 1):
        for m in SCREENSHOT_RE.finditer(line):
            name = m.group(0)
            if not any((d / name).is_file() for d in screenshot_dirs):
                findings.append(
                    Finding(relpath, lineno, "screenshots", f"screenshot `{name}` not found on disk")
                )
    return findings


# --------------------------------------------------------------------------
# --final: pdf-freshness
# --------------------------------------------------------------------------


def rule_pdf_freshness(root: Path) -> list:
    dossier = root / DOSSIER_REL
    pdf_out = dossier / "pdf_out"
    if not dossier.is_dir() or not pdf_out.is_dir():
        return []
    findings = []
    for md in sorted(dossier.glob("*.md")):
        pdf = pdf_out / f"{md.stem}.pdf"
        if not pdf.is_file():
            continue
        if pdf.stat().st_mtime < md.stat().st_mtime:
            findings.append(
                Finding(
                    rel(md),
                    1,
                    "pdf-freshness",
                    f"{rel(pdf)} is older than its source markdown; regenerate it",
                )
            )
    return findings


# --------------------------------------------------------------------------
# --final: manifest
# --------------------------------------------------------------------------


def is_manifest_internal(entry: str) -> bool:
    if entry.endswith(".md"):
        return True
    if Path(entry).name == ".DS_Store":
        return True
    if "scripts/" in entry:
        return True
    return any(token in entry for token in MANIFEST_INTERNAL_SUBSTRINGS)


def rule_manifest(root: Path) -> list:
    manifest = root / DOSSIER_REL / "MANIFEST.txt"
    if not manifest.is_file():
        return [Finding(rel(manifest), 1, "manifest", "MANIFEST.txt is missing: the ZIP has no content list")]
    findings = []
    relpath = rel(manifest)
    dossier = root / DOSSIER_REL
    for lineno, raw_line in enumerate(read_text(manifest).splitlines(), 1):
        entry = raw_line.strip()
        if not entry or entry.startswith("#"):
            continue
        if is_manifest_internal(entry):
            findings.append(
                Finding(relpath, lineno, "manifest", f"internal-only artifact listed: `{entry}`")
            )
            continue
        if not (dossier / entry).exists() and not (root / entry).exists():
            findings.append(Finding(relpath, lineno, "manifest", f"listed file does not exist: `{entry}`"))
    return findings


# --------------------------------------------------------------------------
# --final: pending ("A effectuer" left in a listed document)
# --------------------------------------------------------------------------

PENDING_RE = re.compile(r"[AÀ]\s+effectuer", re.IGNORECASE)


def manifest_sources(root: Path) -> list:
    """Text files behind the manifest's entries: a listed file itself when it
    is text, the source .md of a listed pdf_out/<name>.pdf."""
    manifest = root / DOSSIER_REL / "MANIFEST.txt"
    if not manifest.is_file():
        return []
    dossier = root / DOSSIER_REL
    sources = []
    for raw_line in read_text(manifest).splitlines():
        entry = raw_line.strip()
        if not entry or entry.startswith("#") or is_manifest_internal(entry):
            continue
        path = Path(entry)
        if path.suffix == ".pdf":
            source = dossier / f"{path.stem}.md"
        elif path.suffix in (".txt", ".json", ".csv"):
            source = dossier / entry
        else:
            continue
        if source.is_file() and source not in sources:
            sources.append(source)
    return sources


def rule_pending(root: Path) -> list:
    findings = []
    for source in manifest_sources(root):
        for lineno, line in enumerate(read_text(source).splitlines(), 1):
            if PENDING_RE.search(line):
                findings.append(
                    Finding(
                        rel(source),
                        lineno,
                        "pending",
                        "'A effectuer' left in a document listed in MANIFEST.txt",
                    )
                )
    return findings


# --------------------------------------------------------------------------
# --final: captures-pdf
# --------------------------------------------------------------------------


def rule_captures_pdf(root: Path) -> list:
    dossier = root / DOSSIER_REL
    pdf = dossier / "CAPTURES_ECRAN_FR.pdf"
    shots = dossier / "screenshots"
    if not shots.is_dir():
        return []
    images = [p for p in shots.iterdir() if p.suffix in (".png", ".jpg")]
    if not images:
        return []
    if not pdf.is_file():
        return [Finding(rel(pdf), 1, "captures-pdf", "CAPTURES_ECRAN_FR.pdf is missing: run scripts/make_captures_pdf.sh")]
    newest = max(images, key=lambda p: p.stat().st_mtime)
    if pdf.stat().st_mtime < newest.stat().st_mtime:
        return [
            Finding(
                rel(pdf),
                1,
                "captures-pdf",
                f"older than {rel(newest)}; rerun scripts/make_captures_pdf.sh",
            )
        ]
    return []


# --------------------------------------------------------------------------
# main
# --------------------------------------------------------------------------


def run(active_rules: list) -> list:
    md_files = iter_md_files(DOSSIER_DIR)
    findings: list = []

    needs_rust = "rust-ident" in active_rules
    needs_path = "repo-path" in active_rules
    rust_index = build_rust_index(ROOT) if needs_rust else None
    abs_allowed = collect_abs_allowed_paths(ROOT) if needs_path else None
    external_crates = (
        frozenset(collect_external_crates(ROOT)) if (needs_rust or needs_path) else frozenset()
    )

    if needs_rust:
        findings += rule_rust_ident(md_files, rust_index, external_crates)
    if needs_path:
        findings += rule_repo_path(md_files, abs_allowed, external_crates)
    if "brand" in active_rules:
        findings += rule_brand(md_files)
    if "em-dash" in active_rules:
        findings += rule_em_dash(md_files)
    if "placeholder" in active_rules:
        allowlist = load_allowlist(ALLOWLIST_PATH)
        findings += rule_placeholder(md_files, allowlist)
    if "screenshots" in active_rules:
        findings += rule_screenshots(ROOT)
    if "pdf-freshness" in active_rules:
        findings += rule_pdf_freshness(ROOT)
    if "manifest" in active_rules:
        findings += rule_manifest(ROOT)
    if "pending" in active_rules:
        findings += rule_pending(ROOT)
    if "captures-pdf" in active_rules:
        findings += rule_captures_pdf(ROOT)

    findings.sort(key=lambda f: (f.path, f.line, f.rule))
    return findings


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument(
        "--final",
        action="store_true",
        help="also run the pre-submission-only checks (screenshots, pdf-freshness, manifest, pending, captures-pdf)",
    )
    parser.add_argument(
        "--rules",
        type=str,
        default=None,
        help="comma-separated subset of rule names to run, overriding the default set",
    )
    args = parser.parse_args(argv)

    if args.rules:
        requested = [r.strip() for r in args.rules.split(",") if r.strip()]
        unknown = [r for r in requested if r not in ALL_RULES]
        if unknown:
            parser.error(f"unknown rule(s): {', '.join(unknown)} (known: {', '.join(ALL_RULES)})")
        active_rules = requested
    else:
        active_rules = list(DEFAULT_RULES)
        if args.final:
            active_rules += FINAL_EXTRA_RULES

    findings = run(active_rules)
    for finding in findings:
        print(finding.render())

    if findings:
        print(f"\n{len(findings)} finding(s) across {len(active_rules)} active rule(s).", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
