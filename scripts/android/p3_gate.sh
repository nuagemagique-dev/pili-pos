#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - P3 gate
#
# One command that decides whether a P3 branch (the Android build pipeline: :pos-core
# as an Android library, the :smoke app, the build image, the CI workflow) is
# mergeable. Same output contract as p2_gate.sh. Prints one line per check:
#     GATE | <check> | PASS|FAIL|SKIP | <detail>
# runs every check even after a failure, ends with exactly one
#     GATE | total | PASS|FAIL | <n> failed (<m> checks)
# and exits 1 on any FAIL. The output of a failed check is printed on stderr.
# A check whose inputs do not exist yet (a Gradle module, a script) is SKIP.
#
# Usage: scripts/android/p3_gate.sh [--fast] [--android] [--root] [--exit]
#                                   [--emulator] [--device-tests] [--repro]
#                                   [--base REF] [--allow REGEX]
#   --fast, --android, --root, --exit, --base, --allow   passed to p2_gate.sh
#   --emulator      also run emulator-smoke and android-tests (needs the emulator)
#   --device-tests  also run device-engine-tests (engine tests on the emulator)
#   --repro         also run apk-repro (two container builds, slow)
#   The three options above are independent of --fast.
# Helper modes (used by the checks and by the self-test):
#   p3_gate.sh --count-unit-tests DIR   prints the number of tests (sum of tests minus
#                  skipped) of the TEST-*.xml files in DIR; exit 1 on a failure, an
#                  error or a count under 22
#   p3_gate.sh --check-aar FILE         exit 1 when the AAR breaks a rule of aar-check
#
# Checks, in order
#   p2_gate.sh   every check line of p2_gate.sh with the same --fast/--android/--root/
#                --exit/--base/--allow and --extra-paths '^(\.github/workflows/android\.yml|scripts/release/.*)$'
#                (its total line is not printed; its checks and failures are counted)
#   Full mode only (not --fast); each Gradle command runs as
#   (cd android && ./gradlew --no-daemon --console=plain <tasks>):
#   selftest-check-apk, selftest-p3-gate   the self-tests of this card
#   gradle-lint       lint
#   gradle-ktlint     ktlintCheck
#   gradle-detekt     detekt
#   unit-tests        :pos-core:testDebugUnitTest, then at least 22 tests, none failed
#   mismatch-test     :pos-core:mismatchTest
#   api-check         :pos-core:apiCheck
#   aar-check         :pos-core:assembleRelease, then pos-core-release.aar has
#                     jni/arm64-v8a/libpos_ffi.so, no other jni/<abi>/ and a proguard.txt
#                     with the four consumer rules (PD77)
#   assemble-release  :smoke:assembleRelease
#   apk-check         scripts/android/check_apk.sh on smoke-release-unsigned.apk
#   The first six Gradle checks are SKIP until android/pos-core/build.gradle.kts applies
#   the Android library plugin (lint, unit-tests, mismatch-test, api-check, aar-check) or
#   a module build file applies ktlint / detekt; assemble-release and apk-check are SKIP
#   until android/settings.gradle.kts includes :smoke.
#   Independent of --fast:
#   emulator-smoke, android-tests (--emulator)   scripts/android/emulator_smoke.sh,
#                     scripts/android/run_android_tests.sh
#   device-engine-tests (--device-tests)         scripts/android/device_engine_tests.sh --check
#   apk-repro (--repro)                          scripts/android/check_repro.sh
#                     each SKIP while its script is absent, else exit 0 is PASS
#
# Gradle always runs with --no-daemon; this script never stops a daemon.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

# SELF: absolute path of this script, so a re-invocation works after the cd below
# and when the script was started by a relative path from another directory (PD96).
SELF="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)/$(basename "${BASH_SOURCE[0]}")"

MIN_TESTS=22
AAR_RULES=(
    '-dontwarn java.awt.*'
    '-keep class com.sun.jna.** { *; }'
    '-keep class uniffi.pos_ffi.** { *; }'
    '-keep interface * extends com.sun.jna.Library'
)

# ── helper modes ────────────────────────────────────────────────
# count_unit_tests DIR: prints the count; exit 1 on failures, errors or a low count.
count_unit_tests() {
    python3 -I - "$1" "$MIN_TESTS" <<'PY'
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

# check_aar FILE: exit 1 when a rule of aar-check is broken.
check_aar() {
    python3 -I - "$1" "${AAR_RULES[@]}" <<'PY'
import sys
import zipfile

path, rules = sys.argv[1], sys.argv[2:]
problems = []
try:
    z = zipfile.ZipFile(path)
except (OSError, zipfile.BadZipFile) as e:
    print("cannot read %s: %s" % (path, e))
    sys.exit(1)
names = z.namelist()
if "jni/arm64-v8a/libpos_ffi.so" not in names:
    problems.append("jni/arm64-v8a/libpos_ffi.so is missing")
abis = sorted({n.split("/")[1] for n in names if n.startswith("jni/") and n.count("/") >= 2})
extra = [a for a in abis if a != "arm64-v8a"]
if extra:
    problems.append("other jni ABI directories: " + ", ".join(extra))
if "proguard.txt" not in names:
    problems.append("proguard.txt is missing")
else:
    lines = [ln.rstrip() for ln in z.read("proguard.txt").decode("utf-8", "replace").splitlines()]
    for r in rules:
        if r not in lines:
            problems.append("proguard.txt lacks the line: " + r)
if problems:
    print("; ".join(problems))
    sys.exit(1)
print("aar ok: jni/arm64-v8a/libpos_ffi.so only, %d consumer rules" % len(rules))
PY
}

case "${1:-}" in
--count-unit-tests)
    [[ $# -eq 2 ]] || { echo "usage: p3_gate.sh --count-unit-tests DIR" >&2; exit 2; }
    count_unit_tests "$2"
    exit $?
    ;;
--check-aar)
    [[ $# -eq 2 ]] || { echo "usage: p3_gate.sh --check-aar FILE" >&2; exit 2; }
    check_aar "$2"
    exit $?
    ;;
esac

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
cd "$ROOT" || exit 2

fast=0
do_emulator=0
do_device=0
do_repro=0
p2_args=()
while [[ $# -gt 0 ]]; do
    case "$1" in
    --fast)
        fast=1
        p2_args+=(--fast)
        ;;
    --android | --root | --exit) p2_args+=("$1") ;;
    --emulator) do_emulator=1 ;;
    --device-tests) do_device=1 ;;
    --repro) do_repro=1 ;;
    --base | --allow)
        p2_args+=("$1" "${2:?$1 needs a value}")
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

# ── p2_gate.sh ──────────────────────────────────────────────────
# Its check lines are passed through, its total line is dropped.
P2_OUT="$TMP/p2.out"
bash scripts/android/p2_gate.sh ${p2_args[@]+"${p2_args[@]}"} --extra-paths '^(\.github/workflows/android\.yml|scripts/release/.*)$' >"$P2_OUT"
p2_rc=$?
p2_total=0
while IFS= read -r line; do
    case "$line" in
    "GATE | total | "*) p2_total=1 ;;
    "GATE | "*)
        printf '%s\n' "$line"
        n_checks=$((n_checks + 1))
        case "$line" in "GATE | "*" | FAIL | "*) n_failed=$((n_failed + 1)) ;; esac
        ;;
    esac
done <"$P2_OUT"
if [[ "$p2_total" -eq 0 ]]; then
    n_checks=$((n_checks + 1))
    n_failed=$((n_failed + 1))
    printf 'GATE | p2-gate | FAIL | p2_gate.sh printed no total line (exit %d)\n' "$p2_rc"
fi

# ── Gradle helpers ──────────────────────────────────────────────
gradle() { (cd android && ./gradlew --no-daemon --console=plain "$@"); }

# android_lib: :pos-core applies the Android library plugin.
android_lib() {
    [[ -f android/pos-core/build.gradle.kts ]] &&
        grep -Eq 'libs\.plugins\.android\.library|com\.android\.library' android/pos-core/build.gradle.kts
}

# plugin_applied <word>: a module build file mentions <word> outside comments and `apply false`.
plugin_applied() {
    local f
    for f in android/build.gradle.kts android/*/build.gradle.kts; do
        [[ -f "$f" ]] || continue
        grep -v '^[[:space:]]*//' "$f" | grep -v 'apply false' | grep -qi -- "$1" && return 0
    done
    return 1
}

has_smoke() {
    [[ -f android/settings.gradle.kts ]] &&
        grep -v '^[[:space:]]*//' android/settings.gradle.kts | grep -Eq "[\"']:smoke[\"']"
}

unit_tests() {
    local dir=android/pos-core/build/test-results/testDebugUnitTest n
    gradle :pos-core:testDebugUnitTest || return 1
    n="$(bash "$SELF" --count-unit-tests "$dir")" || { echo "unit tests: $n (failed or fewer than $MIN_TESTS)"; return 1; }
    echo "$n tests"
}

aar_check() {
    gradle :pos-core:assembleRelease || return 1
    check_aar android/pos-core/build/outputs/aar/pos-core-release.aar
}

apk_check() {
    local apk=android/smoke/build/outputs/apk/release/smoke-release-unsigned.apk
    [[ -f "$apk" ]] || { echo "missing $apk"; return 1; }
    local rc
    bash scripts/android/check_apk.sh "$apk" >"$TMP/apk.out"
    rc=$?
    grep -E '\| FAIL \|' "$TMP/apk.out"
    [[ $rc -eq 0 ]] || return 1
    echo "$(grep -c '| PASS |' "$TMP/apk.out") checks passed, $(grep '| sha256 |' "$TMP/apk.out" | sed 's/.*| //' | cut -c1-12)"
}

# ── full mode ───────────────────────────────────────────────────
if [[ "$fast" -eq 0 ]]; then
    gate selftest-check-apk bash scripts/android/test_check_apk.sh
    gate selftest-p3-gate bash scripts/android/test_p3_gate.sh

    if android_lib; then
        gate gradle-lint gradle lint
    else
        skip gradle-lint ":pos-core is not an Android library yet"
    fi
    if plugin_applied ktlint; then
        gate gradle-ktlint gradle ktlintCheck
    else
        skip gradle-ktlint "no module build file applies ktlint yet"
    fi
    if plugin_applied detekt; then
        gate gradle-detekt gradle detekt
    else
        skip gradle-detekt "no module build file applies detekt yet"
    fi
    if android_lib; then
        gate unit-tests unit_tests
        gate mismatch-test gradle :pos-core:mismatchTest
        gate api-check gradle :pos-core:apiCheck
        gate aar-check aar_check
    else
        skip unit-tests ":pos-core is not an Android library yet"
        skip mismatch-test ":pos-core is not an Android library yet"
        skip api-check ":pos-core is not an Android library yet"
        skip aar-check ":pos-core is not an Android library yet"
    fi
    if has_smoke; then
        gate assemble-release gradle :smoke:assembleRelease
        gate apk-check apk_check
    else
        skip assemble-release "android/settings.gradle.kts does not include :smoke yet"
        skip apk-check "android/settings.gradle.kts does not include :smoke yet"
    fi
fi

# ── scripts of other cards, independent of --fast ───────────────
# script_gate <check> <script> [args...]: SKIP while the script is absent.
script_gate() {
    local name="$1" script="$2"
    shift 2
    if [[ -f "$script" ]]; then
        gate "$name" bash "$script" "$@"
    else
        skip "$name" "$script is absent"
    fi
}

if [[ "$do_emulator" -eq 1 ]]; then
    script_gate emulator-smoke scripts/android/emulator_smoke.sh
    script_gate android-tests scripts/android/run_android_tests.sh
fi
if [[ "$do_device" -eq 1 ]]; then
    script_gate device-engine-tests scripts/android/device_engine_tests.sh --check
fi
if [[ "$do_repro" -eq 1 ]]; then
    script_gate apk-repro scripts/android/check_repro.sh
fi

if [[ "$n_failed" -eq 0 ]]; then
    printf 'GATE | total | PASS | 0 failed (%d checks)\n' "$n_checks"
    exit 0
fi
printf 'GATE | total | FAIL | %d failed (%d checks)\n' "$n_failed" "$n_checks"
exit 1
