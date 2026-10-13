#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - P4 gate
#
# One command that decides whether a P4 branch (the :app foundation: engine runtime,
# provisioning, kiosk, navigation, strings, theme) is mergeable. Same output contract
# as p2_gate.sh and p3_gate.sh. Prints one line per check:
#     GATE | <check> | PASS|FAIL|SKIP | <detail>
# runs every check even after a failure, ends with exactly one
#     GATE | total | PASS|FAIL | <n> failed (<m> checks)
# and exits 1 on any FAIL. The output of a failed check is printed on stderr.
# A check whose inputs do not exist yet (a script, a Gradle module) is SKIP.
#
# Usage: scripts/android/p4_gate.sh [--fast] [--app] [--android] [--root] [--exit]
#                                   [--emulator] [--device-tests] [--repro]
#                                   [--app-e2e] [--linux-repro]
#                                   [--base REF] [--allow REGEX] [--min-app-tests N]
#   --fast        skip the self-tests and the :app Gradle checks (card gates)
#   --app         lean card profile for cards that change only :app and the P4 scripts:
#                 hygiene and freeze from p2_gate.sh --hygiene-only instead of the whole
#                 p3 gate, plus app-only-scope; --android, --root, --emulator,
#                 --device-tests and --repro are accepted and ignored
#   --android, --root, --exit, --emulator, --device-tests, --repro
#                 passed to p3_gate.sh (--exit also adds plan-gradle here)
#   --app-e2e     also run the emulator end-to-end run of the app (app-e2e)
#   --linux-repro also build the frozen Linux tag twice in Docker (linux-repro)
#   --base REF    branch the checks compare with (default android/int)
#   --allow REGEX every changed file must also match REGEX (passed to the sub-gate)
#   --min-app-tests N   minimum number of :app unit tests (default 1)
# Helper mode (used by the check and by the self-test):
#   p4_gate.sh --count-tests DIR MIN   prints the number of tests (sum of tests minus
#                 skipped) of the TEST-*.xml files in DIR; exit 1 on a failure, an
#                 error, no file or fewer than MIN
#
# Checks, in order
#   p3-gate       default mode: every check line of p3_gate.sh with the same --fast/
#                 --android/--root/--exit/--emulator/--device-tests/--repro/--base/--allow
#                 (its total line is not printed; its checks and failures are counted);
#                 the whole p3 run holds one heavy.sh slot
#   --app mode instead of p3-gate:
#                 every check line of p2_gate.sh --hygiene-only --base/--allow, then
#   app-only-scope  FAIL when a file changed in BASE...HEAD, or modified or untracked in
#                 the working tree, is under android/rust/, android/pos-core/,
#                 android/smoke/ or android/docker/ ('use the full gate')
#   Always:
#   freeze-tag    tag linux-v1.0.0-rc1 exists, its commit is FREEZE_SHA and
#                 check_linux_frozen.sh --tag linux-v1.0.0-rc1 exits 0 (PD100). SKIP while
#                 the tag does not exist yet (the 1.0.0 re-freeze, user decision
#                 2026-10-10); FAIL when the tag exists and FREEZE_SHA is still empty
#   hygiene-keys  no tracked or untracked-not-ignored file under android/,
#                 scripts/android/ or docs/android/ is a minisign secret key (first line
#                 'untrusted comment: minisign [encrypted] secret key'), contains a PEM
#                 private key header, or is named *.key *.jks *.keystore *.p12
#   lanes         lane_paths.py --check; SKIP while the script is absent
#   vue-strings   vue_to_android.mjs --check; SKIP while the script is absent
#   kotlin-tests  kotlin_test_inventory.py --base BASE: no Kotlin test removed
#   ffi-boundary  SKIP while android/app/src/main/kotlin is absent. In the .kt files of
#                 android/app/src outside dev/nuagemagique/pilipos/engine/ and
#                 .../provisioning/: FAIL on PosEngine.open( and on any import of, or fully
#                 qualified reference to, the uniffi.pos_ffi free functions
#                 (uniffiEnsureInitialized initLogging installInitDb checkConfigBundle
#                 verifyBundle verifyMinisign settlePreview cashDueCents vatSplit qrMatrix
#                 posSwVersion), also on 'import uniffi.pos_ffi.*' together with a bare
#                 call '<name>('; calls on our own wrappers ('helpers.vatSplit(') pass
#   app-manifest  SKIP while android/app/src/main/AndroidManifest.xml is absent. The
#                 manifest has allowBackup=false and a dataExtractionRules, no
#                 REQUEST_INSTALL_PACKAGES, no android:debuggable, exported components only
#                 MainActivity and provisioning.PiliAdminReceiver, the service
#                 engine.EngineService with foregroundServiceType specialUse and the
#                 PROPERTY_SPECIAL_USE_FGS_SUBTYPE property, and the six permissions
#                 INTERNET FOREGROUND_SERVICE FOREGROUND_SERVICE_SPECIAL_USE
#                 POST_NOTIFICATIONS READ_PHONE_STATE RECEIVE_BOOT_COMPLETED
#   Full mode only (not --fast):
#   selftest-p4-gate, selftest-heavy, selftest-kotlin-tests, selftest-lanes (SKIP while
#                 test_lane_paths.py is absent), selftest-vue-strings (SKIP while
#                 test_vue_to_android.mjs is absent)
#   then the :app Gradle checks, each SKIP while android/settings.gradle.kts does not
#   include ':app', each one Gradle invocation under a heavy.sh slot:
#   app-lint (:app:lint), app-ktlint (:app:ktlintCheck), app-detekt (:app:detekt),
#   app-unit-tests (:app:testDebugUnitTest, then --count-tests with --min-app-tests),
#   app-screenshots (:app:verifyRoborazziDebug; SKIP while android/app/src/test/screenshots
#                 holds no .png), app-kover (:app:koverVerifyDebug :app:koverXmlReportDebug;
#                 detail: LINE and BRANCH percent of the XML report), app-assemble-release
#                 (:app:assembleRelease), app-apk-check (check_apk.sh on
#                 app-release-unsigned.apk)
#   --exit:
#   plan-gradle   the plan's literal command: lint ktlintCheck detekt test
#                 koverVerifyDebug assembleRelease in android/, one invocation
#   --app-e2e:
#   app-e2e       scripts/android/app_e2e.sh; SKIP while absent
#   --linux-repro:
#   linux-repro   needs Docker. Clones the repository (git clone --no-local of the git
#                 common dir) to $HOME/.cache/pili-freeze/p4-clone, checks out the tag,
#                 runs scripts/check_repro.sh there (log: $HOME/.cache/pili-freeze/
#                 p4-linux-repro.log) and compares the reported SHA-1 with the one in
#                 docs/android/LINUX_FREEZE.md; the clone's build outputs are removed
#
# Gradle runs as (cd android && heavy.sh ./gradlew --no-daemon --console=plain
# -Dorg.gradle.jvmargs="$PILI_GRADLE_JVMARGS" [--offline] <tasks>) and this script never
# stops a daemon.
# Environment: PILI_GRADLE_JVMARGS (default '-Xmx2g -XX:MaxMetaspaceSize=1g
# -Dfile.encoding=UTF-8'), PILI_GRADLE_OFFLINE=1 (adds --offline). For the self-test:
# PILI_P4_ROOT (repository root), PILI_P4_SUBGATE (script run instead of p3_gate.sh or
# p2_gate.sh --hygiene-only), PILI_P4_FREEZE_SHA (expected commit of the freeze tag),
# PILI_P4_NESTED=1 (the self-test checks are SKIP).
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

TOOLS="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
SELF="$TOOLS/$(basename "${BASH_SOURCE[0]}")"
HEAVY="$TOOLS/heavy.sh"

FREEZE_TAG=linux-v1.0.0-rc1
# TAGGING: fill FREEZE_SHA_RECORDED with the full commit of linux-v1.0.0-rc1
# (git rev-parse 'linux-v1.0.0-rc1^{commit}') when the tag is set.
FREEZE_SHA_RECORDED=""
FREEZE_SHA="${PILI_P4_FREEZE_SHA:-$FREEZE_SHA_RECORDED}"
JVMARGS="${PILI_GRADLE_JVMARGS:--Xmx2g -XX:MaxMetaspaceSize=1g -Dfile.encoding=UTF-8}"

# ── helper mode ─────────────────────────────────────────────────
# count_tests DIR MIN: prints the count; exit 1 on failures, errors, no file or a low count.
count_tests() {
    python3 -I - "$1" "$2" <<'PY'
import glob
import sys
import xml.etree.ElementTree as ET

d, minimum = sys.argv[1], int(sys.argv[2])
files = sorted(glob.glob(d + "/TEST-*.xml"))
tests = skipped = failures = errors = 0
for f in files:
    root = ET.parse(f).getroot()
    suites = [root] if root.tag == "testsuite" else list(root.iter("testsuite"))
    for s in suites:
        tests += int(s.get("tests", "0"))
        skipped += int(s.get("skipped", "0"))
        failures += int(s.get("failures", "0"))
        errors += int(s.get("errors", "0"))
count = tests - skipped
print(count)
if not files:
    print("no TEST-*.xml in " + d, file=sys.stderr)
    sys.exit(1)
if failures or errors:
    print("%d failure(s), %d error(s)" % (failures, errors), file=sys.stderr)
    sys.exit(1)
if count < minimum:
    print("%d tests, at least %d expected" % (count, minimum), file=sys.stderr)
    sys.exit(1)
PY
}

if [[ "${1:-}" == "--count-tests" ]]; then
    if [[ $# -ne 3 || ! "$3" =~ ^[0-9]+$ ]]; then
        echo "usage: p4_gate.sh --count-tests DIR MIN" >&2
        exit 2
    fi
    count_tests "$2" "$3"
    exit $?
fi

ROOT="${PILI_P4_ROOT:-$(cd "$TOOLS/../.." && pwd -P)}"
cd "$ROOT" || exit 2

fast=0
app_mode=0
do_app_e2e=0
do_linux_repro=0
exit_mode=0
base="android/int"
allow_re=""
min_app_tests=1
sub_args=()
while [[ $# -gt 0 ]]; do
    case "$1" in
    --fast)
        fast=1
        sub_args+=(--fast)
        ;;
    --app) app_mode=1 ;;
    --exit)
        exit_mode=1
        sub_args+=(--exit)
        ;;
    --android | --root | --emulator | --device-tests | --repro) sub_args+=("$1") ;;
    --app-e2e) do_app_e2e=1 ;;
    --linux-repro) do_linux_repro=1 ;;
    --base)
        [[ $# -ge 2 ]] || { echo "--base needs a ref" >&2; exit 2; }
        base="$2"
        sub_args+=(--base "$base")
        shift
        ;;
    --allow)
        [[ $# -ge 2 ]] || { echo "--allow needs a regex" >&2; exit 2; }
        allow_re="$2"
        sub_args+=(--allow "$allow_re")
        shift
        ;;
    --min-app-tests)
        [[ $# -ge 2 ]] || { echo "--min-app-tests needs a number" >&2; exit 2; }
        min_app_tests="$2"
        if ! [[ "$min_app_tests" =~ ^[0-9]+$ ]]; then
            echo "--min-app-tests needs a number" >&2
            exit 2
        fi
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

# script_gate <check> <script> [args...]: SKIP while the script is absent.
script_gate() {
    local name="$1" script="$2"
    shift 2
    if [[ -f "$script" ]]; then
        gate "$name" "$@"
    else
        skip "$name" "$script is absent"
    fi
}

have_base() { git rev-parse -q --verify "$base^{commit}" >/dev/null 2>&1; }

# ── sub-gate: p3_gate.sh, or p2_gate.sh --hygiene-only with --app ──
# Its check lines are passed through, its total line is dropped.
run_subgate() { # <name> <command...>
    local name="$1" out="$TMP/sub.out" rc line total=0
    shift
    "$@" >"$out"
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
        printf 'GATE | %s | FAIL | the sub-gate printed no total line (exit %d)\n' "$name" "$rc"
    fi
}

changed_files() {
    git diff --name-only --no-renames "$base...HEAD"
    git diff --name-only --no-renames HEAD
    git ls-files --others --exclude-standard
}

app_only_scope() {
    have_base || { echo "base ref not found: $base"; return 1; }
    local bad
    bad="$(changed_files | LC_ALL=C sort -u | grep -E '^android/(rust|pos-core|smoke|docker)/' || true)"
    if [[ -n "$bad" ]]; then
        printf '%s\n' "$bad" | head -n 20
        echo "$(printf '%s\n' "$bad" | wc -l | tr -d ' ') file(s) under android/rust, pos-core, smoke or docker: use the full gate"
        return 1
    fi
    echo "no file under android/rust, pos-core, smoke or docker"
}

if [[ "$app_mode" -eq 1 ]]; then
    if [[ -n "${PILI_P4_SUBGATE:-}" ]]; then
        run_subgate p2-gate bash "$PILI_P4_SUBGATE" ${sub_args[@]+"${sub_args[@]}"}
    else
        hy_args=(--hygiene-only --base "$base")
        [[ -z "$allow_re" ]] || hy_args+=(--allow "$allow_re")
        # PD223: the release scripts are platform files, accepted by hygiene-paths like the workflow.
        hy_args+=(--extra-paths '^(\.github/workflows/android\.yml|scripts/release/.*)$')
        run_subgate p2-gate bash "$TOOLS/p2_gate.sh" "${hy_args[@]}"
    fi
    gate app-only-scope app_only_scope
else
    sub="${PILI_P4_SUBGATE:-$TOOLS/p3_gate.sh}"
    run_subgate p3-gate "$HEAVY" bash "$sub" ${sub_args[@]+"${sub_args[@]}"}
fi

# ── freeze-tag ──────────────────────────────────────────────────
freeze_tag() {
    local sha
    git rev-parse -q --verify "refs/tags/$FREEZE_TAG" >/dev/null 2>&1 || { echo "tag $FREEZE_TAG does not exist"; return 1; }
    sha="$(git rev-parse "$FREEZE_TAG^{commit}")" || return 1
    if [[ -z "$FREEZE_SHA" ]]; then
        echo "tag $FREEZE_TAG exists ($sha) but FREEZE_SHA_RECORDED in scripts/android/p4_gate.sh is not filled in yet"
        return 1
    fi
    if [[ "$sha" != "$FREEZE_SHA" ]]; then
        echo "tag $FREEZE_TAG points at $sha, expected $FREEZE_SHA"
        return 1
    fi
    bash scripts/android/check_linux_frozen.sh --tag "$FREEZE_TAG" || return 1
}
if git rev-parse -q --verify "refs/tags/$FREEZE_TAG" >/dev/null 2>&1; then
    gate freeze-tag freeze_tag
else
    skip freeze-tag "no tag $FREEZE_TAG yet: the 1.0.0 re-freeze sets it (then fill FREEZE_SHA_RECORDED)"
fi

# ── hygiene-keys ────────────────────────────────────────────────
hygiene_keys() {
    {
        git ls-files -z -- android scripts/android docs/android
        git ls-files -z --others --exclude-standard -- android scripts/android docs/android
    } | python3 -I -c '
import os
import sys

names = sys.stdin.buffer.read().split(b"\0")
pem = b"PRIVATE" + b" KEY-----"
heads = (b"untrusted comment: minisign encrypted secret key", b"untrusted comment: minisign secret key")
bad = []
for raw in sorted({n for n in names if n}):
    path = raw.decode("utf-8", "replace")
    if not os.path.isfile(path):
        continue
    if path.endswith((".key", ".jks", ".keystore", ".p12")):
        bad.append(path + " (key file name)")
        continue
    if os.path.getsize(path) > 16 * 1024 * 1024:
        continue
    with open(path, "rb") as f:
        data = f.read()
    first = data.split(b"\n", 1)[0].lstrip(b"\xef\xbb\xbf")
    if first.startswith(heads):
        bad.append(path + " (minisign secret key)")
    elif pem in data:
        bad.append(path + " (private key block)")
if bad:
    print("\n".join(bad[:20]))
    print("%d secret key file(s): %s" % (len(bad), ", ".join(bad[:3])))
    sys.exit(1)
print("no secret key file under android/, scripts/android/ or docs/android/")
'
}
gate hygiene-keys hygiene_keys

# ── lanes, vue-strings, kotlin-tests ────────────────────────────
script_gate lanes scripts/android/lane_paths.py python3 scripts/android/lane_paths.py --check
script_gate vue-strings scripts/android/vue_to_android.mjs node scripts/android/vue_to_android.mjs --check

kotlin_tests() {
    local out rc
    out="$(python3 scripts/android/kotlin_test_inventory.py --base "$base" 2>"$TMP/kt.err")"
    rc=$?
    cat "$TMP/kt.err"
    printf '%s\n' "$out" | sed -n 's/^KTESTS | total | [A-Z]* | //p'
    [[ $rc -eq 0 ]]
}
gate kotlin-tests kotlin_tests

# ── ffi-boundary ────────────────────────────────────────────────
ffi_boundary() {
    python3 -I - <<'PY'
import os
import re
import sys

FUNCS = [
    "uniffiEnsureInitialized", "initLogging", "installInitDb", "checkConfigBundle",
    "verifyBundle", "verifyMinisign", "settlePreview", "cashDueCents", "vatSplit",
    "qrMatrix", "posSwVersion",
]
names = "|".join(FUNCS)
allowed = re.compile(r"/dev/nuagemagique/pilipos/(engine|provisioning)/")
open_re = re.compile(r"\bPosEngine\s*\.\s*(?:Companion\s*\.\s*)?open\s*\(")
fq_re = re.compile(r"\buniffi\s*\.\s*pos_ffi\s*\.\s*(?:%s)\b" % names)
star_re = re.compile(r"^\s*import\s+uniffi\.pos_ffi\.\*")
bare_re = re.compile(r"(?<![\w.])(?:%s)\s*\(" % names)
decl_re = re.compile(r"\bfun\s+(?:[\w<>?,.]+\.)?(?:%s)\s*\(" % names)


def strip_comments(src):
    out = []
    i, n, depth = 0, len(src), 0
    while i < n:
        if depth:
            if src.startswith("/*", i):
                depth += 1
                i += 2
            elif src.startswith("*/", i):
                depth -= 1
                i += 2
            else:
                out.append("\n" if src[i] == "\n" else " ")
                i += 1
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
            out.append(src[i])
            i += 1
    return "".join(out)


hits = []
for dirpath, dirnames, filenames in os.walk("android/app/src"):
    dirnames.sort()
    for fn in sorted(filenames):
        if not fn.endswith(".kt"):
            continue
        path = os.path.join(dirpath, fn)
        if allowed.search("/" + path):
            continue
        with open(path, encoding="utf-8", errors="replace") as f:
            lines = strip_comments(f.read()).split("\n")
        star = any(star_re.search(ln) for ln in lines)
        for no, ln in enumerate(lines, 1):
            if open_re.search(ln):
                hits.append("%s:%d PosEngine.open(" % (path, no))
            elif fq_re.search(ln):
                hits.append("%s:%d %s" % (path, no, fq_re.search(ln).group(0).replace(" ", "")))
            elif star and bare_re.search(ln) and not decl_re.search(ln):
                hits.append("%s:%d %s" % (path, no, bare_re.search(ln).group(0).strip()))
if hits:
    print("\n".join(hits[:30]))
    print("%d native entry point use(s) outside engine/ and provisioning/: %s" % (len(hits), ", ".join(hits[:5])))
    sys.exit(1)
print("no native entry point outside engine/ and provisioning/")
PY
}
if [[ -d android/app/src/main/kotlin ]]; then
    gate ffi-boundary ffi_boundary
else
    skip ffi-boundary "android/app/src/main/kotlin is absent"
fi

# ── app-manifest ────────────────────────────────────────────────
app_manifest() {
    python3 -I - android/app/src/main/AndroidManifest.xml <<'PY'
import sys
import xml.etree.ElementTree as ET

A = "{http://schemas.android.com/apk/res/android}"
PKG = "dev.nuagemagique.pilipos"
ALLOWED_EXPORTED = {PKG + ".MainActivity", PKG + ".provisioning.PiliAdminReceiver"}
PERMS = ["INTERNET", "FOREGROUND_SERVICE", "FOREGROUND_SERVICE_SPECIAL_USE",
         "POST_NOTIFICATIONS", "READ_PHONE_STATE", "RECEIVE_BOOT_COMPLETED"]
problems = []


def qualify(name):
    if name is None:
        return ""
    if name.startswith("."):
        return PKG + name
    if "." not in name:
        return PKG + "." + name
    return name


try:
    root = ET.parse(sys.argv[1]).getroot()
except (OSError, ET.ParseError) as e:
    print("cannot read the manifest: %s" % e)
    sys.exit(1)

app = root.find("application")
if app is None:
    print("no <application> element")
    sys.exit(1)
if app.get(A + "allowBackup") != "false":
    problems.append("application android:allowBackup is not false")
if not app.get(A + "dataExtractionRules"):
    problems.append("application android:dataExtractionRules is missing")
for el in root.iter():
    if el.get(A + "debuggable") is not None:
        problems.append("android:debuggable is set on <%s>" % el.tag)

perms = set()
for el in root.iter():
    if el.tag.startswith("uses-permission"):
        name = el.get(A + "name", "")
        perms.add(name)
        if name == "android.permission.REQUEST_INSTALL_PACKAGES":
            problems.append("REQUEST_INSTALL_PACKAGES is requested")
for p in PERMS:
    if "android.permission." + p not in perms:
        problems.append("uses-permission %s is missing" % p)

service = None
for el in app:
    if el.tag not in ("activity", "activity-alias", "service", "receiver", "provider"):
        continue
    name = qualify(el.get(A + "name"))
    if el.get(A + "exported") == "true" and name not in ALLOWED_EXPORTED:
        problems.append("%s %s is exported" % (el.tag, name))
    if el.tag == "service" and name == PKG + ".engine.EngineService":
        service = el
if service is None:
    problems.append("service engine.EngineService is missing")
else:
    types = (service.get(A + "foregroundServiceType") or "").split("|")
    if "specialUse" not in types:
        problems.append("EngineService has no foregroundServiceType specialUse")
    props = [p.get(A + "name") for p in service.findall("property")]
    if "android.app.PROPERTY_SPECIAL_USE_FGS_SUBTYPE" not in props:
        problems.append("EngineService lacks the PROPERTY_SPECIAL_USE_FGS_SUBTYPE property")
if problems:
    print("\n".join(problems))
    print("%d manifest problem(s): %s" % (len(problems), "; ".join(problems[:3])))
    sys.exit(1)
print("manifest ok: %d permissions, exported components restricted" % len(perms))
PY
}
if [[ -f android/app/src/main/AndroidManifest.xml ]]; then
    gate app-manifest app_manifest
else
    skip app-manifest "android/app/src/main/AndroidManifest.xml is absent"
fi

# ── Gradle helpers ──────────────────────────────────────────────
gradle() {
    local offline=()
    [[ "${PILI_GRADLE_OFFLINE:-0}" != "1" ]] || offline=(--offline)
    (cd android && "$HEAVY" ./gradlew --no-daemon --console=plain -Dorg.gradle.jvmargs="$JVMARGS" ${offline[@]+"${offline[@]}"} "$@")
}

has_app() {
    [[ -f android/settings.gradle.kts ]] &&
        grep -v '^[[:space:]]*//' android/settings.gradle.kts | grep -Eq "[\"']:app[\"']"
}

app_unit_tests() {
    local n
    gradle :app:testDebugUnitTest || return 1
    n="$(bash "$SELF" --count-tests android/app/build/test-results/testDebugUnitTest "$min_app_tests" 2>&1)" ||
        { echo "unit tests: $n (failed or fewer than $min_app_tests)"; return 1; }
    echo "$n tests"
}

app_kover() {
    gradle :app:koverVerifyDebug :app:koverXmlReportDebug || return 1
    python3 -I - android/app/build/reports/kover <<'PY'
import glob
import os
import sys
import xml.etree.ElementTree as ET

d = sys.argv[1]
files = sorted(glob.glob(os.path.join(d, "**", "*.xml"), recursive=True))
preferred = [f for f in files if os.path.basename(f).startswith("report")]
if not files:
    print("no Kover XML report under " + d)
    sys.exit(1)
root = ET.parse((preferred or files)[0]).getroot()
pct = {}
for c in root.findall("counter"):
    covered, missed = int(c.get("covered", "0")), int(c.get("missed", "0"))
    total = covered + missed
    pct[c.get("type")] = 100.0 * covered / total if total else 100.0
missing = [t for t in ("LINE", "BRANCH") if t not in pct]
if missing:
    print("the Kover report has no %s counter" % "/".join(missing))
    sys.exit(1)
print("LINE %.1f%%, BRANCH %.1f%%" % (pct["LINE"], pct["BRANCH"]))
PY
}

app_apk_check() {
    local apk=android/app/build/outputs/apk/release/app-release-unsigned.apk rc
    [[ -f "$apk" ]] || { echo "missing $apk"; return 1; }
    bash scripts/android/check_apk.sh "$apk" >"$TMP/apk.out"
    rc=$?
    grep -E '\| FAIL \|' "$TMP/apk.out"
    [[ $rc -eq 0 ]] || return 1
    echo "$(grep -c '| PASS |' "$TMP/apk.out") checks passed, $(grep '| sha256 |' "$TMP/apk.out" | sed 's/.*| //' | cut -c1-12)"
}

has_screenshots() { [[ -d android/app/src/test/screenshots ]] && [[ -n "$(find android/app/src/test/screenshots -name '*.png' -print -quit 2>/dev/null)" ]]; }

# ── full mode ───────────────────────────────────────────────────
if [[ "$fast" -eq 0 ]]; then
    if [[ -n "${PILI_P4_NESTED:-}" ]]; then
        for c in selftest-p4-gate selftest-heavy selftest-kotlin-tests selftest-lanes selftest-vue-strings; do
            skip "$c" "nested in a self-test"
        done
    else
        export PILI_P4_NESTED=1
        gate selftest-p4-gate bash scripts/android/test_p4_gate.sh
        gate selftest-heavy bash scripts/android/test_heavy.sh
        gate selftest-kotlin-tests python3 scripts/android/test_kotlin_test_inventory.py
        script_gate selftest-lanes scripts/android/test_lane_paths.py python3 scripts/android/test_lane_paths.py
        script_gate selftest-vue-strings scripts/android/test_vue_to_android.mjs node --test scripts/android/test_vue_to_android.mjs
        unset PILI_P4_NESTED
    fi

    if has_app; then
        gate app-lint gradle :app:lint
        gate app-ktlint gradle :app:ktlintCheck
        gate app-detekt gradle :app:detekt
        gate app-unit-tests app_unit_tests
        if has_screenshots; then
            gate app-screenshots gradle :app:verifyRoborazziDebug
        else
            skip app-screenshots "android/app/src/test/screenshots holds no .png yet"
        fi
        gate app-kover app_kover
        gate app-assemble-release gradle :app:assembleRelease
        gate app-apk-check app_apk_check
    else
        for c in app-lint app-ktlint app-detekt app-unit-tests app-screenshots app-kover app-assemble-release app-apk-check; do
            skip "$c" "android/settings.gradle.kts does not include :app yet"
        done
    fi
fi

# ── --exit ──────────────────────────────────────────────────────
if [[ "$exit_mode" -eq 1 ]]; then
    gate plan-gradle gradle lint ktlintCheck detekt test koverVerifyDebug assembleRelease
fi

# ── --app-e2e ───────────────────────────────────────────────────
if [[ "$do_app_e2e" -eq 1 ]]; then
    script_gate app-e2e scripts/android/app_e2e.sh bash scripts/android/app_e2e.sh
fi

# ── --linux-repro ───────────────────────────────────────────────
linux_repro() {
    local cache="$HOME/.cache/pili-freeze" clone log common want got rc
    clone="$cache/p4-clone"
    log="$cache/p4-linux-repro.log"
    git rev-parse -q --verify "refs/tags/$FREEZE_TAG" >/dev/null 2>&1 || { echo "tag $FREEZE_TAG does not exist yet"; return 1; }
    command -v docker >/dev/null 2>&1 || { echo "docker is not installed"; return 1; }
    mkdir -p "$cache" || return 1
    want="$(sed -n 's/.*Reproducible: SHA-1 \([0-9a-f]\{40\}\).*/\1/p' docs/android/LINUX_FREEZE.md | head -n 1)"
    [[ -n "$want" ]] || { echo "no recorded SHA-1 in docs/android/LINUX_FREEZE.md"; return 1; }
    common="$(git rev-parse --path-format=absolute --git-common-dir)" || return 1
    rm -rf "$clone"
    git clone -q --no-local "$common" "$clone" || { echo "git clone failed"; return 1; }
    git -C "$clone" checkout -q --detach "$FREEZE_TAG" || { echo "cannot check out $FREEZE_TAG in the clone"; return 1; }
    (cd "$clone" && bash scripts/check_repro.sh) >"$log" 2>&1
    rc=$?
    rm -rf "$clone/deploy/dist" "$clone/target" "$clone/ui/dist" "$clone/ui/node_modules"
    rm -f "$clone"/build_info_*.txt
    got="$(sed -n 's/.*Reproducible: SHA-1 \([0-9a-f]\{40\}\).*/\1/p' "$log" | tail -n 1)"
    if [[ $rc -ne 0 ]]; then
        tail -n 20 "$log"
        echo "check_repro.sh exited $rc (log: $log)"
        return 1
    fi
    if [[ -z "$got" ]]; then
        echo "check_repro.sh reported no SHA-1 (log: $log)"
        return 1
    fi
    if [[ "$got" != "$want" ]]; then
        echo "SHA-1 $got differs from the recorded $want"
        return 1
    fi
    echo "reproducible, SHA-1 $got equals the one in LINUX_FREEZE.md"
}
if [[ "$do_linux_repro" -eq 1 ]]; then
    gate linux-repro linux_repro
fi

if [[ "$n_failed" -eq 0 ]]; then
    printf 'GATE | total | PASS | 0 failed (%d checks)\n' "$n_checks"
    exit 0
fi
printf 'GATE | total | FAIL | %d failed (%d checks)\n' "$n_failed" "$n_checks"
exit 1
