#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of the helper modes of scripts/android/p3_gate.sh
#
# Usage: bash scripts/android/test_p3_gate.sh
#
# --count-unit-tests DIR on fixture TEST-*.xml sets and --check-aar FILE on fixture
# AARs, both built with python3 in a mktemp dir (no Gradle, no emulator, no network).
# Prints one line per case and a summary; exits 1 when a case fails.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
GATE="$ROOT/scripts/android/p3_gate.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
pass=0
fail=0

ok() { pass=$((pass + 1)); printf 'TEST | %s | PASS\n' "$1"; }
bad() { fail=$((fail + 1)); printf 'TEST | %s | FAIL | %s\n' "$1" "$2"; }

# mkxml DIR NAME TESTS SKIPPED FAILURES ERRORS: one TEST-NAME.xml with that many testcases.
mkxml() {
    python3 -I - "$@" <<'PY'
import os
import sys

d, name, tests, skipped, failures, errors = sys.argv[1], sys.argv[2], *map(int, sys.argv[3:7])
os.makedirs(d, exist_ok=True)
cases = []
for i in range(tests):
    body = ""
    if i < skipped:
        body = "<skipped/>"
    elif i < skipped + failures:
        body = '<failure message="boom">trace</failure>'
    elif i < skipped + failures + errors:
        body = '<error message="boom">trace</error>'
    cases.append('<testcase name="t%d" classname="%s" time="0.01">%s</testcase>' % (i, name, body))
with open("%s/TEST-%s.xml" % (d, name), "w") as f:
    f.write('<?xml version="1.0" encoding="UTF-8"?>\n')
    f.write('<testsuite name="%s" tests="%d" skipped="%d" failures="%d" errors="%d" timestamp="2026-10-08T10:00:00">\n'
            % (name, tests, skipped, failures, errors))
    f.write("".join(cases) + "\n</testsuite>\n")
PY
}

# count_case NAME DIR WANT_RC WANT_STDOUT
count_case() {
    local name="$1" dir="$2" want_rc="$3" want_out="$4" out rc
    out="$("$GATE" --count-unit-tests "$dir" 2>/dev/null)"
    rc=$?
    if [[ $rc -eq $want_rc && "$out" == "$want_out" ]]; then
        ok "$name"
    else
        bad "$name" "rc=$rc out='$out', want rc=$want_rc out='$want_out'"
    fi
}

mkxml "$TMP/t22" A 22 0 0 0
count_case "22 passing tests: 22, exit 0" "$TMP/t22" 0 22
mkxml "$TMP/t21" A 21 0 0 0
count_case "21 tests: exit 1" "$TMP/t21" 1 21
mkxml "$TMP/tfail" A 22 0 1 0
count_case "one failure: exit 1" "$TMP/tfail" 1 22
mkxml "$TMP/terr" A 22 0 0 1
count_case "one error: exit 1" "$TMP/terr" 1 22
mkxml "$TMP/tskip" A 22 2 0 0
count_case "skipped tests are not counted (22 - 2 = 20, exit 1)" "$TMP/tskip" 1 20
mkxml "$TMP/tskip2" A 25 3 0 0
count_case "22 passing plus 3 skipped: 22, exit 0" "$TMP/tskip2" 0 22
mkxml "$TMP/tsum" A 10 0 0 0
mkxml "$TMP/tsum" B 12 0 0 0
count_case "several files are summed (10 + 12), exit 0" "$TMP/tsum" 0 22
mkdir -p "$TMP/tempty"
count_case "no TEST-*.xml: exit 1" "$TMP/tempty" 1 0
"$GATE" --count-unit-tests >/dev/null 2>&1
rc=$?
[[ $rc -eq 2 ]] && ok "--count-unit-tests without DIR exits 2" || bad "--count-unit-tests without DIR exits 2" "rc=$rc"

# mkaar OUT [--no-rule N] [--abi NAME]... [--no-so] [--no-proguard]
mkaar() {
    local out="$1"
    shift
    python3 -I - "$out" "$@" <<'PY'
import sys
import zipfile

out, args = sys.argv[1], sys.argv[2:]
rules = [
    "-dontwarn java.awt.*",
    "-keep class com.sun.jna.** { *; }",
    "-keep class uniffi.pos_ffi.** { *; }",
    "-keep interface * extends com.sun.jna.Library",
]
abis, so, proguard, drop = [], True, True, None
i = 0
while i < len(args):
    if args[i] == "--no-rule":
        drop = int(args[i + 1])
        i += 1
    elif args[i] == "--abi":
        abis.append(args[i + 1])
        i += 1
    elif args[i] == "--no-so":
        so = False
    elif args[i] == "--no-proguard":
        proguard = False
    i += 1
with zipfile.ZipFile(out, "w") as z:
    z.writestr("AndroidManifest.xml", "<manifest/>")
    z.writestr("classes.jar", b"PK")
    if so:
        z.writestr("jni/arm64-v8a/libpos_ffi.so", b"\x7fELF")
    for a in abis:
        z.writestr("jni/%s/libpos_ffi.so" % a, b"\x7fELF")
    if proguard:
        kept = [r for n, r in enumerate(rules) if n != drop]
        z.writestr("proguard.txt", "# consumer rules\r\n" + "\r\n".join(kept) + "\r\n")
PY
}

aar_case() { # NAME FILE WANT_RC
    local name="$1" file="$2" want="$3" out rc
    out="$("$GATE" --check-aar "$file" 2>&1)"
    rc=$?
    if [[ $rc -eq $want ]]; then
        ok "$name"
    else
        bad "$name" "rc=$rc want $want: $out"
    fi
}

mkaar "$TMP/good.aar"
aar_case "correct AAR passes" "$TMP/good.aar" 0
mkaar "$TMP/norule.aar" --no-rule 2
aar_case "missing -keep class uniffi.pos_ffi rule fails" "$TMP/norule.aar" 1
mkaar "$TMP/norule0.aar" --no-rule 0
aar_case "missing -dontwarn rule fails" "$TMP/norule0.aar" 1
mkaar "$TMP/x86.aar" --abi x86_64
aar_case "extra jni/x86_64 fails" "$TMP/x86.aar" 1
mkaar "$TMP/noso.aar" --no-so
aar_case "missing libpos_ffi.so fails" "$TMP/noso.aar" 1
mkaar "$TMP/noproguard.aar" --no-proguard
aar_case "missing proguard.txt fails" "$TMP/noproguard.aar" 1
printf 'not a zip' >"$TMP/garbage.aar"
aar_case "unreadable AAR fails" "$TMP/garbage.aar" 1
aar_case "missing AAR fails" "$TMP/missing.aar" 1

printf 'TEST | summary | %s | %d passed, %d failed\n' "$([[ $fail -eq 0 ]] && echo PASS || echo FAIL)" "$pass" "$fail"
[[ $fail -eq 0 ]]
