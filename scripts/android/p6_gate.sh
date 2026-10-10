#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - P6 gate (platform lane)
#
# One command that decides whether a P6 branch (compliance and platform hardening:
# no cloud backup, strings, update keys, boundary of the platform package) is
# mergeable. It runs p4_gate.sh with the same options and adds the P6 checks. Same
# output contract as p4_gate.sh. Prints one line per check:
#     GATE | <check> | PASS|FAIL|SKIP | <detail>
# runs every check even after a failure, ends with exactly one
#     GATE | total | PASS|FAIL | <n> failed (<m> checks)
# and exits 1 on any FAIL. The output of a failed check is printed on stderr.
#
# Usage: scripts/android/p6_gate.sh [--fast] [--app] [--android] [--root] [--exit]
#                                   [--emulator] [--device-tests] [--repro]
#                                   [--app-e2e] [--linux-repro] [--plat-e2e]
#                                   [--base REF] [--allow REGEX] [--min-app-tests N]
#   Every option of p4_gate.sh is passed through unchanged (see its header); the check
#   lines of p4_gate.sh are re-printed first (its total line is not), and its checks and
#   failures are counted. --fast skips the self-tests and the :app Gradle checks there and
#   here. --plat-e2e adds the emulator end-to-end run of the platform features.
#
# P6 checks, in order (all run from the repository root)
#   plat-manifest      android/app/src/main/AndroidManifest.xml: <application> has
#                      android:allowBackup="false", android:fullBackupContent="false" and
#                      android:dataExtractionRules="@xml/data_extraction_rules"; <manifest>
#                      has no android:installLocation (PD220: the data never moves to an
#                      adoptable SD card); no uses-permission among REQUEST_INSTALL_PACKAGES,
#                      READ_EXTERNAL_STORAGE, WRITE_EXTERNAL_STORAGE, MANAGE_EXTERNAL_STORAGE
#                      and READ_MEDIA_* (an entry with tools:node="remove" does not count)
#   plat-backup-rules  res/xml/data_extraction_rules.xml has <cloud-backup> and
#                      <device-transfer>, each excluding the domains root, file, database,
#                      sharedpref and external, and no <include> anywhere
#   plat-strings       for every res/values/strings_plat_<area>.xml: the same file exists in
#                      values-en/ and values-zh/ with exactly the same set of names; every
#                      name starts with plat_<area>_; no empty value; a string with
#                      translatable="false" exists only in values/ (absent from en and zh);
#                      a values-en or values-zh strings_plat file without a values/ twin fails
#   plat-boundary      under android/app/src/main/kotlin/dev/nuagemagique/pilipos/platform/
#                      (comments ignored): no EngineHolder, no force-stop or forceStop, no
#                      killBackgroundProcesses, no settlePreview, vatSplit or cashDueCents;
#                      every http:// literal is on a line that has 127.0.0.1 or localhost
#                      (the debug registry override, PD211)
#   plat-keys          SKIP while res/xml/plat_update_keys.xml is absent. Else an element with
#                      build="debug" (key = its text, or its key attribute) equals the key
#                      line (second line) of android/app/provisioning/dev-minisign.pub, and
#                      the element with build="release" is empty or matches ^RW[A-Za-z0-9+/]{54}$
#   plat-engine-ref    SKIP while no string plat_about_engine_ref exists in
#                      res/values/strings_plat_*.xml. Else it equals
#                      'pili-pos <V>, <TAG> (<SHORT>)': TAG = $PILI_P6_FREEZE_TAG (default
#                      linux-v1.0.0-rc1), V = version under [workspace.package] of
#                      'git show TAG:Cargo.toml', SHORT = first 7 hex of TAG's commit.
#                      While TAG does not exist yet, the string may hold the placeholder
#                      'pili-pos <V>, <TAG> (pending)' with V = the working tree's
#                      [workspace.package] version: SKIP ('filled at tagging'); any other
#                      value is a FAIL. Once TAG exists the placeholder is a FAIL.
#   Full mode only (not --fast):
#   selftest-p6-gate   scripts/android/test_p6_gate.sh
#   selftest-declare   scripts/release/test_declare_android.sh; SKIP while it is absent
#   --plat-e2e:
#   plat-e2e           scripts/android/plat_e2e.sh; SKIP while absent; PASS on its
#                      'PLATE2E | total | PASS' line
#
# Environment (for the self-test): PILI_P6_ROOT (repository root), PILI_P6_SUBGATE (script
# run instead of p4_gate.sh), PILI_P6_NESTED=1 (the self-test checks are SKIP),
# PILI_P6_FREEZE_TAG (tag of plat-engine-ref).
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

TOOLS="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
SELF="$TOOLS/$(basename "${BASH_SOURCE[0]}")"
ROOT="${PILI_P6_ROOT:-$(cd "$TOOLS/../.." && pwd -P)}"
FREEZE_TAG="${PILI_P6_FREEZE_TAG:-linux-v1.0.0-rc1}"

fast=0
do_plat_e2e=0
sub_args=()
while [[ $# -gt 0 ]]; do
    case "$1" in
    --fast)
        fast=1
        sub_args+=("$1")
        ;;
    --plat-e2e) do_plat_e2e=1 ;;
    --app | --android | --root | --exit | --emulator | --device-tests | --repro | --app-e2e | --linux-repro)
        sub_args+=("$1")
        ;;
    --base | --allow | --min-app-tests)
        [[ $# -ge 2 ]] || { echo "$1 needs a value" >&2; exit 2; }
        sub_args+=("$1" "$2")
        shift
        ;;
    -h | --help)
        sed -n '2,/^set -uo/p' "$SELF" | sed '$d; s/^# \{0,1\}//'
        exit 0
        ;;
    *)
        echo "unknown argument: $1" >&2
        exit 2
        ;;
    esac
    shift
done

cd "$ROOT" || exit 2

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
n_checks=0
n_failed=0

clean_detail() {
    local d
    d="$(tr '\n|' ' /' | sed 's/  */ /g; s/^ //; s/ $//' | cut -c1-160)"
    printf '%s' "${d:-ok}"
}

last_line() { grep -v '^[[:space:]]*$' "$1" | tail -n 1 | clean_detail; }

# gate <check> <command...>: PASS when the command exits 0.
gate() {
    local name="$1" log
    shift
    n_checks=$((n_checks + 1))
    log="$TMP/check.$n_checks.log"
    if "$@" >"$log" 2>&1; then
        printf 'GATE | %s | PASS | %s\n' "$name" "$(last_line "$log")"
    else
        n_failed=$((n_failed + 1))
        printf 'GATE | %s | FAIL | %s\n' "$name" "$(last_line "$log")"
        tail -n 40 "$log" | sed 's/^/    | /' >&2
    fi
}

skip() { # <check> <detail>
    n_checks=$((n_checks + 1))
    printf 'GATE | %s | SKIP | %s\n' "$1" "$2"
}

# ── sub-gate: p4_gate.sh with the same options ──────────────────
# Its check lines are passed through, its total line is dropped.
run_subgate() {
    local sub="${PILI_P6_SUBGATE:-$TOOLS/p4_gate.sh}" out="$TMP/sub.out" rc line total=0
    if [[ -z "${PILI_P6_SUBGATE:-}" && -z "${PILI_P4_ROOT:-}" ]]; then
        export PILI_P4_ROOT="$ROOT"
    fi
    bash "$sub" ${sub_args[@]+"${sub_args[@]}"} >"$out"
    rc=$?
    while IFS= read -r line; do
        case "$line" in
        "GATE | total | "*) total=1 ;;
        "GATE | "*)
            printf '%s\n' "$line"
            n_checks=$((n_checks + 1))
            case "$line" in "GATE | "*" | FAIL | "*) n_failed=$((n_failed + 1)) ;; esac
            ;;
        esac
    done <"$out"
    if [[ "$total" -eq 0 ]]; then
        n_checks=$((n_checks + 1))
        n_failed=$((n_failed + 1))
        printf 'GATE | p4-gate | FAIL | the sub-gate printed no total line (exit %d)\n' "$rc"
    fi
}
run_subgate

MANIFEST=android/app/src/main/AndroidManifest.xml
RULES=android/app/src/main/res/xml/data_extraction_rules.xml
RES=android/app/src/main/res
PLATFORM_SRC=android/app/src/main/kotlin/dev/nuagemagique/pilipos/platform
KEYS_XML=$RES/xml/plat_update_keys.xml
DEV_PUB=android/app/provisioning/dev-minisign.pub

# ── plat-manifest ───────────────────────────────────────────────
plat_manifest() {
    python3 -I - "$MANIFEST" <<'PY'
import sys
import xml.etree.ElementTree as ET

A = "{http://schemas.android.com/apk/res/android}"
T = "{http://schemas.android.com/tools}"
FORBIDDEN = {
    "android.permission." + n
    for n in ("REQUEST_INSTALL_PACKAGES", "READ_EXTERNAL_STORAGE", "WRITE_EXTERNAL_STORAGE", "MANAGE_EXTERNAL_STORAGE")
}
problems = []
try:
    root = ET.parse(sys.argv[1]).getroot()
except (OSError, ET.ParseError) as e:
    print("cannot read the manifest: %s" % e)
    sys.exit(1)

if root.get(A + "installLocation") is not None:
    problems.append("<manifest> has android:installLocation=%s (PD220)" % root.get(A + "installLocation"))
app = root.find("application")
if app is None:
    print("no <application> element")
    sys.exit(1)
for attr, want in (("allowBackup", "false"), ("fullBackupContent", "false"), ("dataExtractionRules", "@xml/data_extraction_rules")):
    got = app.get(A + attr)
    if got != want:
        problems.append("application android:%s is %s, expected %s" % (attr, "missing" if got is None else repr(got), want))
n = 0
for el in root.iter():
    if not el.tag.startswith("uses-permission"):
        continue
    if el.get(T + "node") == "remove":
        continue
    name = el.get(A + "name", "")
    n += 1
    if name in FORBIDDEN or name.startswith("android.permission.READ_MEDIA_"):
        problems.append("forbidden permission " + name)
if problems:
    print("\n".join(problems))
    print("%d manifest problem(s): %s" % (len(problems), "; ".join(problems[:3])))
    sys.exit(1)
print("backup off, no installLocation, %d permissions, none for storage or install" % n)
PY
}
gate plat-manifest plat_manifest

# ── plat-backup-rules ───────────────────────────────────────────
plat_backup_rules() {
    python3 -I - "$RULES" <<'PY'
import sys
import xml.etree.ElementTree as ET

DOMAINS = {"root", "file", "database", "sharedpref", "external"}
problems = []
try:
    root = ET.parse(sys.argv[1]).getroot()
except (OSError, ET.ParseError) as e:
    print("cannot read data_extraction_rules.xml: %s" % e)
    sys.exit(1)
if root.tag != "data-extraction-rules":
    problems.append("root element is <%s>" % root.tag)
for section in ("cloud-backup", "device-transfer"):
    els = root.findall(section)
    if len(els) != 1:
        problems.append("%d <%s> section(s), expected 1" % (len(els), section))
        continue
    got = {e.get("domain") for e in els[0].findall("exclude")}
    missing = sorted(DOMAINS - got)
    if missing:
        problems.append("<%s> does not exclude %s" % (section, ", ".join(missing)))
for el in root.iter("include"):
    problems.append("an <include> element is present (domain %s)" % el.get("domain"))
if problems:
    print("\n".join(problems))
    print("%d rule problem(s): %s" % (len(problems), "; ".join(problems[:3])))
    sys.exit(1)
print("both sections exclude root, file, database, sharedpref, external; no include")
PY
}
gate plat-backup-rules plat_backup_rules

# ── plat-strings ────────────────────────────────────────────────
plat_strings() {
    python3 -I - "$RES" <<'PY'
import glob
import os
import re
import sys
import xml.etree.ElementTree as ET

res = sys.argv[1]
problems = []


def blank(text):
    return text is None or text.strip() in ("", '""')


def load(path):
    """name -> translatable (bool) of a strings file; records empties and duplicates."""
    names = {}
    try:
        root = ET.parse(path).getroot()
    except (OSError, ET.ParseError) as e:
        problems.append("%s: cannot parse (%s)" % (path, e))
        return None
    for el in root:
        if el.tag not in ("string", "plurals", "string-array"):
            continue
        name = el.get("name")
        if not name:
            problems.append("%s: a <%s> has no name" % (path, el.tag))
            continue
        if name in names:
            problems.append("%s: %s is defined twice" % (path, name))
        names[name] = el.get("translatable") != "false"
        if el.tag == "string":
            empty = blank("".join(el.itertext()))
        else:
            items = el.findall("item")
            empty = not items or any(blank("".join(i.itertext())) for i in items)
        if empty:
            problems.append("%s: %s is empty" % (path, name))
    return names


files = sorted(glob.glob(os.path.join(res, "values", "strings_plat_*.xml")))
if not files:
    problems.append("no values/strings_plat_<area>.xml file")
base_files = {os.path.basename(f) for f in files}
for lang in ("values-en", "values-zh"):
    for f in sorted(glob.glob(os.path.join(res, lang, "strings_plat_*.xml"))):
        if os.path.basename(f) not in base_files:
            problems.append("%s/%s has no values/ twin" % (lang, os.path.basename(f)))
total = 0
for f in files:
    base = os.path.basename(f)
    m = re.match(r"^strings_plat_([a-z]+)\.xml$", base)
    if not m:
        problems.append("%s: the file name is not strings_plat_<area>.xml (area a-z)" % base)
        continue
    prefix = "plat_%s_" % m.group(1)
    fr = load(f)
    if fr is None:
        continue
    total += len(fr)
    for name in sorted(fr):
        if not name.startswith(prefix):
            problems.append("%s: %s does not start with %s" % (base, name, prefix))
    for lang in ("values-en", "values-zh"):
        path = os.path.join(res, lang, base)
        if not os.path.isfile(path):
            problems.append("%s/%s is missing" % (lang, base))
            continue
        other = load(path)
        if other is None:
            continue
        for name in sorted(n for n, t in other.items() if not t):
            problems.append("%s/%s: %s is translatable=false, allowed only in values/" % (lang, base, name))
        want = {n for n, t in fr.items() if t}
        have = set(other)
        for name in sorted(want - have):
            problems.append("%s/%s: %s is missing" % (lang, base, name))
        for name in sorted(have - want):
            problems.append("%s/%s: %s is not a translatable name of values/%s" % (lang, base, name, base))
if problems:
    print("\n".join(problems[:40]))
    print("%d strings problem(s): %s" % (len(problems), "; ".join(problems[:3])))
    sys.exit(1)
print("%d file(s), %d names, same keys in fr, en and zh" % (len(files), total))
PY
}
gate plat-strings plat_strings

# ── plat-boundary ───────────────────────────────────────────────
plat_boundary() {
    python3 -I - "$PLATFORM_SRC" <<'PY'
import os
import re
import sys

root = sys.argv[1]
FORBIDDEN = re.compile(
    r"\bEngineHolder\b|force-stop|forceStop|killBackgroundProcesses|\b(?:settlePreview|vatSplit|cashDueCents)\b"
)


def strip_comments(src):
    """Blanks // and nested /* */ comments; string, raw string and char literals are kept."""
    out = []
    i, n, depth = 0, len(src), 0
    while i < n:
        c = src[i]
        if depth:
            if src.startswith("/*", i):
                depth += 1
                i += 2
            elif src.startswith("*/", i):
                depth -= 1
                i += 2
            else:
                out.append("\n" if c == "\n" else " ")
                i += 1
        elif src.startswith('"""', i):
            j = src.find('"""', i + 3)
            j = n if j < 0 else j + 3
            while j < n and src[j] == '"':
                j += 1
            out.append(src[i:j])
            i = j
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"' and src[j] != "\n":
                j += 2 if src[j] == "\\" else 1
            out.append(src[i : j + 1])
            i = j + 1
        elif c == "'":
            j = i + 1
            while j < n and src[j] != "'" and src[j] != "\n":
                j += 2 if src[j] == "\\" else 1
            out.append(src[i : j + 1])
            i = j + 1
        elif src.startswith("/*", i):
            depth = 1
            out.append("  ")
            i += 2
        elif src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        else:
            out.append(c)
            i += 1
    return "".join(out)


hits = []
count = 0
for dirpath, dirnames, filenames in os.walk(root):
    dirnames.sort()
    for fn in sorted(filenames):
        if not fn.endswith((".kt", ".java")):
            continue
        path = os.path.join(dirpath, fn)
        count += 1
        with open(path, encoding="utf-8", errors="replace") as f:
            lines = strip_comments(f.read()).split("\n")
        for no, ln in enumerate(lines, 1):
            m = FORBIDDEN.search(ln)
            if m:
                hits.append("%s:%d %s" % (path, no, m.group(0)))
            if "http://" in ln and "127.0.0.1" not in ln and "localhost" not in ln:
                hits.append("%s:%d http:// literal not on 127.0.0.1 or localhost" % (path, no))
if hits:
    print("\n".join(hits[:30]))
    print("%d boundary violation(s): %s" % (len(hits), "; ".join(hits[:3])))
    sys.exit(1)
print("%d source file(s) under platform/, no engine internals, no force-stop, no plain http" % count)
PY
}
gate plat-boundary plat_boundary

# ── plat-keys ───────────────────────────────────────────────────
plat_keys() {
    python3 -I - "$KEYS_XML" "$DEV_PUB" <<'PY'
import re
import sys
import xml.etree.ElementTree as ET

problems = []
try:
    root = ET.parse(sys.argv[1]).getroot()
except (OSError, ET.ParseError) as e:
    print("cannot read plat_update_keys.xml: %s" % e)
    sys.exit(1)
try:
    with open(sys.argv[2], encoding="utf-8") as f:
        lines = f.read().split("\n")
except OSError as e:
    print("cannot read dev-minisign.pub: %s" % e)
    sys.exit(1)
dev = lines[1].strip() if len(lines) > 1 else ""
if not dev:
    print("dev-minisign.pub has no key line")
    sys.exit(1)

keys = {}
for el in root.iter():
    b = el.get("build")
    if b is None:
        continue
    if b in keys:
        problems.append("build=%s appears twice" % b)
    text = "".join(el.itertext()).strip()
    keys[b] = text or (el.get("key") or "").strip()
for b in ("debug", "release"):
    if b not in keys:
        problems.append("no key for build=%s" % b)
if "debug" in keys and keys["debug"] != dev:
    problems.append("the debug key %r differs from dev-minisign.pub" % keys["debug"][:20])
rel = keys.get("release")
if rel and not re.fullmatch(r"RW[A-Za-z0-9+/]{54}", rel):
    problems.append("the release key is neither empty nor a minisign public key")
if problems:
    print("\n".join(problems))
    print("%d key problem(s): %s" % (len(problems), "; ".join(problems[:3])))
    sys.exit(1)
print("debug key equals dev-minisign.pub, release key %s" % ("empty" if not rel else "is a valid minisign key"))
PY
}
if [[ -f "$KEYS_XML" ]]; then
    gate plat-keys plat_keys
else
    skip plat-keys "$KEYS_XML is absent"
fi

# ── plat-engine-ref ─────────────────────────────────────────────
# Prints the value of plat_about_engine_ref (empty output and exit 3 when no such string).
engine_ref_value() {
    python3 -I - "$RES" <<'PY'
import glob
import os
import sys
import xml.etree.ElementTree as ET

for f in sorted(glob.glob(os.path.join(sys.argv[1], "values", "strings_plat_*.xml"))):
    try:
        root = ET.parse(f).getroot()
    except (OSError, ET.ParseError):
        continue
    for el in root.iter("string"):
        if el.get("name") == "plat_about_engine_ref":
            print("".join(el.itertext()).strip())
            sys.exit(0)
sys.exit(3)
PY
}

# workspace_version: the version under [workspace.package] of the Cargo.toml on stdin.
workspace_version() {
    python3 -I -c '
import re
import sys

section = None
for line in sys.stdin:
    s = line.strip()
    m = re.match(r"^\[([^\]]+)\]", s)
    if m:
        section = m.group(1).strip()
        continue
    if section == "workspace.package":
        v = re.match(r"^version\s*=\s*\"([^\"]+)\"", s)
        if v:
            print(v.group(1))
            break
'
}

# The value plat_about_engine_ref holds until the freeze tag is set.
engine_ref_placeholder() {
    local ver
    ver="$(workspace_version <Cargo.toml 2>/dev/null)"
    echo "pili-pos ${ver}, ${FREEZE_TAG} (pending)"
}

plat_engine_ref() {
    local got="$1" ver short want
    if ! git rev-parse -q --verify "refs/tags/$FREEZE_TAG" >/dev/null 2>&1; then
        echo "tag $FREEZE_TAG does not exist and plat_about_engine_ref is '$got', not the placeholder '$(engine_ref_placeholder)'"
        return 1
    fi
    if [[ "$got" == *"(pending)" ]]; then
        echo "tag $FREEZE_TAG exists: fill the placeholder of plat_about_engine_ref ('$got')"
        return 1
    fi
    ver="$(git show "$FREEZE_TAG:Cargo.toml" 2>/dev/null | workspace_version)"
    [[ -n "$ver" ]] || { echo "no version under [workspace.package] in $FREEZE_TAG:Cargo.toml"; return 1; }
    short="$(git rev-parse "$FREEZE_TAG^{commit}" | cut -c1-7)"
    want="pili-pos ${ver}, ${FREEZE_TAG} (${short})"
    if [[ "$got" != "$want" ]]; then
        echo "plat_about_engine_ref is '$got', expected '$want'"
        return 1
    fi
    echo "$got"
}
if ref="$(engine_ref_value 2>/dev/null)"; then
    if ! git rev-parse -q --verify "refs/tags/$FREEZE_TAG" >/dev/null 2>&1 && [[ "$ref" == "$(engine_ref_placeholder)" ]]; then
        skip plat-engine-ref "no tag $FREEZE_TAG yet: '$ref' is the placeholder, filled at tagging"
    else
        gate plat-engine-ref plat_engine_ref "$ref"
    fi
else
    skip plat-engine-ref "no string plat_about_engine_ref yet"
fi

# ── full mode: self-tests ───────────────────────────────────────
if [[ "$fast" -eq 0 ]]; then
    if [[ -n "${PILI_P6_NESTED:-}" ]]; then
        skip selftest-p6-gate "nested in a self-test"
        skip selftest-declare "nested in a self-test"
    else
        export PILI_P6_NESTED=1
        gate selftest-p6-gate bash scripts/android/test_p6_gate.sh
        if [[ -f scripts/release/test_declare_android.sh ]]; then
            gate selftest-declare bash scripts/release/test_declare_android.sh
        else
            skip selftest-declare "scripts/release/test_declare_android.sh is absent"
        fi
        unset PILI_P6_NESTED
    fi
fi

# ── --plat-e2e ──────────────────────────────────────────────────
plat_e2e() {
    local out="$TMP/plat_e2e.out" rc
    bash scripts/android/plat_e2e.sh >"$out" 2>&1
    rc=$?
    grep -E '^PLATE2E \| .* \| FAIL' "$out" | head -n 10
    if ! grep -q '^PLATE2E | total | PASS' "$out"; then
        tail -n 10 "$out"
        echo "plat_e2e.sh exited $rc without 'PLATE2E | total | PASS'"
        return 1
    fi
    grep '^PLATE2E | total | PASS' "$out" | tail -n 1 | sed 's/^PLATE2E | total | PASS | //'
}
if [[ "$do_plat_e2e" -eq 1 ]]; then
    if [[ -f scripts/android/plat_e2e.sh ]]; then
        gate plat-e2e plat_e2e
    else
        skip plat-e2e "scripts/android/plat_e2e.sh is absent"
    fi
fi

if [[ "$n_failed" -eq 0 ]]; then
    printf 'GATE | total | PASS | 0 failed (%d checks)\n' "$n_checks"
    exit 0
fi
printf 'GATE | total | FAIL | %d failed (%d checks)\n' "$n_failed" "$n_checks"
exit 1
