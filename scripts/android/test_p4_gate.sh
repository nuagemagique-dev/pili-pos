#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of scripts/android/p4_gate.sh
#
# Usage: bash scripts/android/test_p4_gate.sh
#
# Runs p4_gate.sh against temporary git repositories (PILI_P4_ROOT) with a stub
# sub-gate (PILI_P4_SUBGATE), a stub gradlew, stub check scripts and a stub docker.
# No Gradle, no network, no Docker, no emulator; heavy.sh is switched off
# (PILI_HEAVY=0). Cases: option parsing, sub-gate selection and counting, the total
# line, app-only-scope, freeze-tag, hygiene-keys, kotlin-tests, lanes and vue-strings,
# ffi-boundary, app-manifest, --count-tests, the :app Gradle checks (SKIP and the
# stubbed run), plan-gradle, app-e2e, linux-repro.
# Prints one line per case and a summary; exits 1 when a case fails.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
GATE="$ROOT/scripts/android/p4_gate.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
export HOME="$TMP/home"
mkdir -p "$HOME"
export PILI_HEAVY=0
export PILI_P4_NESTED=1
unset PILI_GRADLE_OFFLINE PILI_GRADLE_JVMARGS PILI_HEAVY_HELD
pass=0
fail=0

ok() { pass=$((pass + 1)); printf 'TEST | %s | PASS\n' "$1"; }
bad() { fail=$((fail + 1)); printf 'TEST | %s | FAIL | %s\n' "$1" "$2"; }

# check NAME CONDITION-RESULT(0 = true) DETAIL
check() { if [[ "$2" -eq 0 ]]; then ok "$1"; else bad "$1" "$3"; fi; }

STUB_SUBGATE="$TMP/stub_subgate.sh"
cat >"$STUB_SUBGATE" <<'STUB'
#!/usr/bin/env bash
# Stub sub-gate: records its arguments, prints two check lines and a total line.
printf '%s\n' "$*" >>"${STUB_LOG:-/dev/null}"
echo "GATE | stub-one | PASS | ok"
if [[ "${STUB_FAIL:-0}" == "1" ]]; then
    echo "GATE | stub-two | FAIL | broken"
    [[ "${STUB_NO_TOTAL:-0}" == "1" ]] || echo "GATE | total | FAIL | 1 failed (2 checks)"
    exit 1
fi
echo "GATE | stub-two | SKIP | later"
[[ "${STUB_NO_TOTAL:-0}" == "1" ]] || echo "GATE | total | PASS | 0 failed (2 checks)"
STUB

# tgit REPO git-args...: git inside a temporary repository only. Refuses anything else,
# so a bug of this test can never touch the real repository or its tags.
tgit() {
    local r="$1"
    shift
    if [[ -z "$r" || "$r" != "$TMP"/* || ! -d "$r/.git" ]]; then
        echo "tgit: refusing to run git in '$r'" >&2
        exit 1
    fi
    git -C "$r" -c user.name=t -c user.email=t@t -c commit.gpgsign=false "$@"
}

# new_repo: a temporary git repository with the tag base, the freeze tag and the real
# kotlin_test_inventory.py. Prints its path.
new_repo() {
    local r
    r="$(mktemp -d "$TMP/repo.XXXXXX")"
    mkdir -p "$r/scripts/android" "$r/android" "$r/docs/android"
    cp "$ROOT/scripts/android/kotlin_test_inventory.py" "$r/scripts/android/"
    printf '#!/bin/sh\necho "frozen against stub"\n' >"$r/scripts/android/check_linux_frozen.sh"
    echo "x" >"$r/README.md"
    {
        git -C "$r" init -q -b main
        tgit "$r" add -A
        tgit "$r" commit -q -m "chore: base"
        tgit "$r" tag base
        tgit "$r" tag linux-v1.0.0-rc2
    } >/dev/null 2>&1
    printf '%s' "$r"
}

# run_gate REPO [env VAR=...]... -- ARGS...: sets OUT, ERR, RC.
run_gate() {
    local repo="$1" envs=()
    shift
    while [[ $# -gt 0 && "$1" != "--" ]]; do
        envs+=("$1")
        shift
    done
    shift
    local sha
    [[ "$repo" == "$TMP"/* && -d "$repo/.git" ]] || { echo "run_gate: bad repo '$repo'" >&2; exit 1; }
    sha="$(tgit "$repo" rev-parse 'linux-v1.0.0-rc2^{commit}' 2>/dev/null)"
    OUT="$(env PILI_P4_ROOT="$repo" PILI_P4_SUBGATE="$STUB_SUBGATE" PILI_P4_FREEZE_SHA="$sha" STUB_LOG="$TMP/stub.log" ${envs[@]+"${envs[@]}"} \
        bash "$GATE" "$@" 2>"$TMP/err.txt")"
    RC=$?
    ERR="$(cat "$TMP/err.txt")"
}

# line CHECK: the status of the GATE line of CHECK in OUT (PASS, FAIL, SKIP or "").
status_of() { printf '%s\n' "$OUT" | sed -n "s/^GATE | $1 | \\([A-Z]*\\) | .*/\\1/p" | head -n 1; }
detail_of() { printf '%s\n' "$OUT" | sed -n "s/^GATE | $1 | [A-Z]* | //p" | head -n 1; }

# ── option parsing ──────────────────────────────────────────────
R="$(new_repo)"
run_gate "$R" -- --bogus
check "an unknown option exits 2" "$([[ $RC -eq 2 ]] && echo 0 || echo 1)" "rc=$RC"
run_gate "$R" -- --min-app-tests abc
check "a non-numeric --min-app-tests exits 2" "$([[ $RC -eq 2 ]] && echo 0 || echo 1)" "rc=$RC"
run_gate "$R" -- --base
check "--base without a value exits 2" "$([[ $RC -eq 2 ]] && echo 0 || echo 1)" "rc=$RC"
run_gate "$R" -- --help
check "--help prints the header and exits 0" "$([[ $RC -eq 0 && "$OUT" == *"P4 gate"* ]] && echo 0 || echo 1)" "rc=$RC"

# ── sub-gate selection, forwarding and the total line ───────────
: >"$TMP/stub.log"
run_gate "$R" -- --fast --exit --android --base base --allow '^x$'
args="$(cat "$TMP/stub.log")"
ok_default=1
[[ "$(status_of stub-one)" == "PASS" && "$(status_of app-only-scope)" == "" ]] && ok_default=0
[[ "$args" == *"--fast"* && "$args" == *"--exit"* && "$args" == *"--android"* && "$args" == *"--base base"* && "$args" == *"--allow ^x\$"* ]] || ok_default=1
check "default mode runs the p3 sub-gate with the forwarded options and no app-only-scope" "$ok_default" "args='$args' out=$OUT"

: >"$TMP/stub.log"
run_gate "$R" -- --app --fast --base base
ok_app=1
[[ "$(status_of stub-one)" == "PASS" && "$(status_of app-only-scope)" == "PASS" ]] && ok_app=0
check "--app runs the hygiene sub-gate and app-only-scope" "$ok_app" "out=$OUT"

# PD223: both gates let hygiene-paths accept scripts/release/ next to the workflow.
pd223_re='--extra-paths '"'"'^(\.github/workflows/android\.yml|scripts/release/.*)$'"'"
pd223=0
grep -qF -- "$pd223_re" "$GATE" || pd223=1
grep -qF -- "$pd223_re" "$ROOT/scripts/android/p3_gate.sh" || pd223=1
check "p4 and p3 gates pass --extra-paths for scripts/release to hygiene-paths" "$pd223" "regex missing in p4_gate.sh or p3_gate.sh"
# The regex accepts the release scripts and the workflow, and refuses other paths.
pd223_match=0
for f in scripts/release/declare_android.sh scripts/release/test_declare_android.sh .github/workflows/android.yml; do
    [[ "$f" =~ ^(\.github/workflows/android\.yml|scripts/release/.*)$ ]] || pd223_match=1
done
[[ "scripts/other/x.sh" =~ ^(\.github/workflows/android\.yml|scripts/release/.*)$ ]] && pd223_match=1
check "the PD223 regex accepts scripts/release and refuses scripts/other" "$pd223_match" "regex mismatch"

total_lines="$(printf '%s\n' "$OUT" | grep -c '^GATE | total | ')"
last="$(printf '%s\n' "$OUT" | tail -n 1)"
fmt=1
[[ $RC -eq 0 && "$total_lines" -eq 1 && "$last" =~ ^GATE\ \|\ total\ \|\ PASS\ \|\ 0\ failed\ \([0-9]+\ checks\)$ ]] && fmt=0
check "one total line 'GATE | total | PASS | 0 failed (N checks)' and exit 0" "$fmt" "rc=$RC last='$last'"

run_gate "$R" STUB_FAIL=1 -- --fast --base base
last="$(printf '%s\n' "$OUT" | tail -n 1)"
okf=1
[[ $RC -eq 1 && "$last" =~ ^GATE\ \|\ total\ \|\ FAIL\ \|\ 1\ failed\ \([0-9]+\ checks\)$ && "$(status_of stub-two)" == "FAIL" && "$(status_of freeze-tag)" == "PASS" ]] && okf=0
check "a failing sub-gate check counts once, later checks still run, exit 1" "$okf" "rc=$RC last='$last'"

run_gate "$R" STUB_NO_TOTAL=1 -- --fast --base base
check "a sub-gate without a total line is a FAIL" "$([[ $RC -eq 1 && "$(status_of p3-gate)" == "FAIL" ]] && echo 0 || echo 1)" "rc=$RC out=$OUT"

# ── app-only-scope ──────────────────────────────────────────────
R="$(new_repo)"
mkdir -p "$R/android/app/src/main/kotlin" "$R/scripts/android"
echo "class A" >"$R/android/app/src/main/kotlin/A.kt"
echo "# t" >"$R/scripts/android/p4_gate_note.md"
run_gate "$R" -- --app --fast --base base
check "app-only-scope accepts android/app and scripts/android paths" "$([[ "$(status_of app-only-scope)" == "PASS" ]] && echo 0 || echo 1)" "out=$OUT"
for d in rust pos-core smoke docker; do
    R="$(new_repo)"
    mkdir -p "$R/android/$d"
    echo "x" >"$R/android/$d/file.txt"
    run_gate "$R" -- --app --fast --base base
    check "app-only-scope refuses android/$d/" "$([[ "$(status_of app-only-scope)" == "FAIL" && "$(detail_of app-only-scope)" == *"use the full gate"* && $RC -eq 1 ]] && echo 0 || echo 1)" "out=$OUT"
done
R="$(new_repo)"
mkdir -p "$R/android/pos-core"
echo "x" >"$R/android/pos-core/f.kt"
tgit "$R" add -A
tgit "$R" commit -q -m "feat: core"
run_gate "$R" -- --app --fast --base base
check "app-only-scope sees committed changes of BASE...HEAD" "$([[ "$(status_of app-only-scope)" == "FAIL" ]] && echo 0 || echo 1)" "out=$OUT"

# ── freeze-tag ──────────────────────────────────────────────────
R="$(new_repo)"
run_gate "$R" -- --fast --base base
check "freeze-tag passes with the tag, the commit and a clean tag-mode check" "$([[ "$(status_of freeze-tag)" == "PASS" ]] && echo 0 || echo 1)" "out=$OUT"
tgit "$R" tag -d linux-v1.0.0-rc2 >/dev/null
OUT="$(env PILI_P4_ROOT="$R" PILI_P4_SUBGATE="$STUB_SUBGATE" bash "$GATE" --fast --base base 2>/dev/null)"
RC=$?
check "freeze-tag is SKIP with a clear message while the 1.0.0 tag does not exist" "$([[ "$(status_of freeze-tag)" == "SKIP" && "$(detail_of freeze-tag)" == *"no tag linux-v1.0.0-rc2 yet"* && $RC -eq 0 ]] && echo 0 || echo 1)" "rc=$RC out=$OUT"
tgit "$R" tag linux-v1.1.0-rc1
OUT="$(env PILI_P4_ROOT="$R" PILI_P4_SUBGATE="$STUB_SUBGATE" bash "$GATE" --fast --base base 2>/dev/null)"
check "the misnamed linux-v1.1.0-rc1 tag is not a freeze tag any more" "$([[ "$(status_of freeze-tag)" == "SKIP" ]] && echo 0 || echo 1)" "out=$OUT"
tgit "$R" tag -d linux-v1.1.0-rc1 >/dev/null
tgit "$R" tag linux-v1.0.0-rc2
OUT="$(env PILI_P4_ROOT="$R" PILI_P4_SUBGATE="$STUB_SUBGATE" PILI_P4_FREEZE_SHA= bash "$GATE" --fast --base base 2>/dev/null)"
check "freeze-tag fails when the tag exists and the recorded commit is not filled in" "$([[ "$(status_of freeze-tag)" == "FAIL" && "$(detail_of freeze-tag)" == *"FREEZE_SHA_RECORDED"*"not filled in"* ]] && echo 0 || echo 1)" "out=$OUT"
OUT="$(env PILI_P4_ROOT="$R" PILI_P4_SUBGATE="$STUB_SUBGATE" PILI_P4_FREEZE_SHA=65b567fdd2497277300340979c2c61ff17abc918 bash "$GATE" --fast --base base 2>/dev/null)"
check "freeze-tag fails when the tag points at another commit" "$([[ "$(status_of freeze-tag)" == "FAIL" && "$(detail_of freeze-tag)" == *"expected 65b567fd"* ]] && echo 0 || echo 1)" "out=$OUT"
printf '#!/bin/sh\necho "not frozen" >&2\nexit 1\n' >"$R/scripts/android/check_linux_frozen.sh"
tgit "$R" add -A
tgit "$R" commit -q -m "chore: change"
tgit "$R" tag -f linux-v1.0.0-rc2 >/dev/null
run_gate "$R" -- --fast --base base
check "freeze-tag fails when check_linux_frozen.sh --tag fails" "$([[ "$(status_of freeze-tag)" == "FAIL" ]] && echo 0 || echo 1)" "out=$OUT"

# ── hygiene-keys ────────────────────────────────────────────────
R="$(new_repo)"
mkdir -p "$R/android/app/provisioning"
printf 'untrusted comment: minisign public key ABCD\nRWQabc\n' >"$R/android/app/provisioning/dev-minisign.pub"
echo "plain text" >"$R/docs/android/notes.md"
run_gate "$R" -- --fast --base base
check "hygiene-keys accepts a minisign public key" "$([[ "$(status_of hygiene-keys)" == "PASS" ]] && echo 0 || echo 1)" "out=$OUT"
printf 'untrusted comment: minisign encrypted secret key\nRWRTY0Iy\n' >"$R/android/app/provisioning/dev.key.txt"
run_gate "$R" -- --fast --base base
check "hygiene-keys finds a minisign secret key header" "$([[ "$(status_of hygiene-keys)" == "FAIL" && "$(detail_of hygiene-keys)" == *"dev.key.txt"* ]] && echo 0 || echo 1)" "out=$OUT"
rm "$R/android/app/provisioning/dev.key.txt"
printf 'untrusted comment: minisign secret key 1234\nRWRTY0Iy\n' >"$R/android/app/provisioning/other.txt"
run_gate "$R" -- --fast --base base
check "hygiene-keys finds the unencrypted minisign secret key header" "$([[ "$(status_of hygiene-keys)" == "FAIL" ]] && echo 0 || echo 1)" "out=$OUT"
rm "$R/android/app/provisioning/other.txt"
printf 'binary' >"$R/android/app/release.jks"
run_gate "$R" -- --fast --base base
check "hygiene-keys finds a .jks file" "$([[ "$(status_of hygiene-keys)" == "FAIL" && "$(detail_of hygiene-keys)" == *"release.jks"* ]] && echo 0 || echo 1)" "out=$OUT"
rm "$R/android/app/release.jks"
printf -- '-----BEGIN RSA %s%s\nabc\n' "PRIVATE" " KEY-----" >"$R/docs/android/pem.txt"
run_gate "$R" -- --fast --base base
check "hygiene-keys finds a PEM private key block" "$([[ "$(status_of hygiene-keys)" == "FAIL" ]] && echo 0 || echo 1)" "out=$OUT"
rm "$R/docs/android/pem.txt"
tgit "$R" add -A
for ext in keystore p12 key; do
    printf 'x' >"$R/android/a.$ext"
    run_gate "$R" -- --fast --base base
    check "hygiene-keys refuses a .$ext file name" "$([[ "$(status_of hygiene-keys)" == "FAIL" ]] && echo 0 || echo 1)" "out=$OUT"
    rm "$R/android/a.$ext"
done

# ── kotlin-tests ────────────────────────────────────────────────
R="$(new_repo)"
mkdir -p "$R/android/m/src/test/kotlin"
printf 'package p\nclass ATest {\n  @Test fun one() {}\n}\n' >"$R/android/m/src/test/kotlin/ATest.kt"
tgit "$R" add -A
tgit "$R" commit -q -m "test: one"
tgit "$R" tag base2
run_gate "$R" -- --fast --base base2
check "kotlin-tests passes with the counts as detail" "$([[ "$(status_of kotlin-tests)" == "PASS" && "$(detail_of kotlin-tests)" == "base 1, head 1, removed 0" ]] && echo 0 || echo 1)" "out=$OUT"
printf 'package p\nclass ATest {\n  fun one() {}\n}\n' >"$R/android/m/src/test/kotlin/ATest.kt"
run_gate "$R" -- --fast --base base2
check "kotlin-tests fails when a test is removed" "$([[ "$(status_of kotlin-tests)" == "FAIL" && "$ERR" == *"removed"* ]] && echo 0 || echo 1)" "out=$OUT err=$ERR"

# ── lanes and vue-strings ───────────────────────────────────────
R="$(new_repo)"
run_gate "$R" -- --fast --base base
check "lanes and vue-strings are SKIP while their scripts are absent" "$([[ "$(status_of lanes)" == "SKIP" && "$(status_of vue-strings)" == "SKIP" ]] && echo 0 || echo 1)" "out=$OUT"
printf 'import sys\nsys.exit(0)\n' >"$R/scripts/android/lane_paths.py"
printf 'process.exit(1);\n' >"$R/scripts/android/vue_to_android.mjs"
run_gate "$R" -- --fast --base base
check "lanes PASS and vue-strings FAIL follow the exit status of their scripts" "$([[ "$(status_of lanes)" == "PASS" && "$(status_of vue-strings)" == "FAIL" ]] && echo 0 || echo 1)" "out=$OUT"

# ── ffi-boundary ────────────────────────────────────────────────
R="$(new_repo)"
run_gate "$R" -- --fast --base base
check "ffi-boundary is SKIP while android/app/src/main/kotlin is absent" "$([[ "$(status_of ffi-boundary)" == "SKIP" ]] && echo 0 || echo 1)" "out=$OUT"
K="$R/android/app/src/main/kotlin/dev/nuagemagique/pilipos"
mkdir -p "$K/engine" "$K/provisioning" "$K/feature/tables"
cat >"$K/engine/EngineHolder.kt" <<'KT'
package dev.nuagemagique.pilipos.engine
import uniffi.pos_ffi.PosEngine
import uniffi.pos_ffi.initLogging
import uniffi.pos_ffi.vatSplit
fun open() { initLogging("x"); PosEngine.open("a") }
KT
cat >"$K/provisioning/Bundle.kt" <<'KT'
package dev.nuagemagique.pilipos.provisioning
import uniffi.pos_ffi.*
fun check() { verifyBundle("a"); checkConfigBundle("b") }
KT
cat >"$K/feature/tables/Tables.kt" <<'KT'
package dev.nuagemagique.pilipos.feature.tables
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosEngine
// PosEngine.open( in a comment, uniffi.pos_ffi.vatSplit( too
fun total(helpers: EngineHelpers) = helpers.vatSplit(1, 2)
KT
run_gate "$R" -- --fast --base base
check "ffi-boundary allows engine/ and provisioning/, a wrapper call, types and comments" "$([[ "$(status_of ffi-boundary)" == "PASS" ]] && echo 0 || echo 1)" "out=$OUT"
printf 'package x\nfun f() { PosEngine.open("a") }\n' >"$K/feature/tables/Bad1.kt"
run_gate "$R" -- --fast --base base
check "ffi-boundary refuses PosEngine.open( outside engine/ (file:line listed)" "$([[ "$(status_of ffi-boundary)" == "FAIL" && "$ERR" == *"Bad1.kt:2"* ]] && echo 0 || echo 1)" "out=$OUT err=$ERR"
rm "$K/feature/tables/Bad1.kt"
printf 'package x\nimport uniffi.pos_ffi.qrMatrix\n' >"$K/feature/tables/Bad2.kt"
run_gate "$R" -- --fast --base base
check "ffi-boundary refuses an import of a free function" "$([[ "$(status_of ffi-boundary)" == "FAIL" ]] && echo 0 || echo 1)" "out=$OUT"
rm "$K/feature/tables/Bad2.kt"
printf 'package x\nimport uniffi.pos_ffi.*\nfun f() = cashDueCents(1)\n' >"$K/feature/tables/Bad3.kt"
run_gate "$R" -- --fast --base base
check "ffi-boundary refuses a star import with a bare call" "$([[ "$(status_of ffi-boundary)" == "FAIL" && "$ERR" == *"Bad3.kt:3"* ]] && echo 0 || echo 1)" "out=$OUT err=$ERR"
rm "$K/feature/tables/Bad3.kt"
printf 'package x\nfun f() = uniffi.pos_ffi.posSwVersion()\n' >"$K/feature/tables/Bad4.kt"
run_gate "$R" -- --fast --base base
check "ffi-boundary refuses a fully qualified call" "$([[ "$(status_of ffi-boundary)" == "FAIL" ]] && echo 0 || echo 1)" "out=$OUT"
rm "$K/feature/tables/Bad4.kt"
mkdir -p "$R/android/app/src/test/kotlin/dev/nuagemagique/pilipos/engine"
printf 'package x\nimport uniffi.pos_ffi.settlePreview\n' >"$R/android/app/src/test/kotlin/dev/nuagemagique/pilipos/engine/EngineTest.kt"
run_gate "$R" -- --fast --base base
check "ffi-boundary allows the engine/ package of a test source set" "$([[ "$(status_of ffi-boundary)" == "PASS" ]] && echo 0 || echo 1)" "out=$OUT"

# ── app-manifest ────────────────────────────────────────────────
write_manifest() { # REPO, then sed expressions applied to the good manifest
    local r="$1"
    shift
    mkdir -p "$r/android/app/src/main"
    cat >"$TMP/manifest.good" <<'XML'
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE_SPECIAL_USE" />
    <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
    <uses-permission android:name="android.permission.READ_PHONE_STATE" />
    <uses-permission android:name="android.permission.RECEIVE_BOOT_COMPLETED" />
    <application android:allowBackup="false" android:dataExtractionRules="@xml/data_extraction_rules">
        <activity android:name=".MainActivity" android:exported="true" />
        <receiver android:name="dev.nuagemagique.pilipos.provisioning.PiliAdminReceiver" android:exported="true" />
        <service android:name=".engine.EngineService" android:exported="false" android:foregroundServiceType="specialUse">
            <property android:name="android.app.PROPERTY_SPECIAL_USE_FGS_SUBTYPE" android:value="pos engine" />
        </service>
        <receiver android:name=".platform.update.InstallResultReceiver" android:exported="false" />
    </application>
</manifest>
XML
    if [[ $# -gt 0 ]]; then
        sed "$@" "$TMP/manifest.good" >"$r/android/app/src/main/AndroidManifest.xml"
    else
        cp "$TMP/manifest.good" "$r/android/app/src/main/AndroidManifest.xml"
    fi
}

manifest_case() { # NAME EXPECTED(PASS|FAIL) sed-expression...
    local name="$1" want="$2"
    shift 2
    local r
    r="$(new_repo)"
    if [[ $# -gt 0 ]]; then write_manifest "$r" "$@"; else write_manifest "$r"; fi
    run_gate "$r" -- --fast --base base
    check "$name" "$([[ "$(status_of app-manifest)" == "$want" ]] && echo 0 || echo 1)" "got $(status_of app-manifest): $(detail_of app-manifest)"
}

R="$(new_repo)"
run_gate "$R" -- --fast --base base
check "app-manifest is SKIP while the manifest is absent" "$([[ "$(status_of app-manifest)" == "SKIP" ]] && echo 0 || echo 1)" "out=$OUT"
manifest_case "app-manifest accepts a good manifest (relative and qualified names)" PASS
manifest_case "app-manifest rejects allowBackup true" FAIL 's/allowBackup="false"/allowBackup="true"/'
manifest_case "app-manifest rejects a missing dataExtractionRules" FAIL 's/ android:dataExtractionRules="@xml\/data_extraction_rules"//'
manifest_case "app-manifest rejects REQUEST_INSTALL_PACKAGES" FAIL 's|<application|<uses-permission android:name="android.permission.REQUEST_INSTALL_PACKAGES" /><application|'
manifest_case "app-manifest rejects android:debuggable" FAIL 's|<application |<application android:debuggable="true" |'
manifest_case "app-manifest rejects another exported component" FAIL 's|InstallResultReceiver" android:exported="false"|InstallResultReceiver" android:exported="true"|'
manifest_case "app-manifest rejects an EngineService without specialUse" FAIL 's/android:foregroundServiceType="specialUse"/android:foregroundServiceType="dataSync"/'
manifest_case "app-manifest rejects an EngineService without the subtype property" FAIL 's/PROPERTY_SPECIAL_USE_FGS_SUBTYPE/OTHER_PROPERTY/'
manifest_case "app-manifest rejects a missing permission" FAIL '/READ_PHONE_STATE/d'
manifest_case "app-manifest rejects a missing EngineService" FAIL 's|\.engine\.EngineService|.engine.OtherService|'
manifest_case "app-manifest rejects an exported component named like an allowed one in another package" FAIL 's|dev.nuagemagique.pilipos.provisioning.PiliAdminReceiver|dev.other.provisioning.PiliAdminReceiver|'

# ── --count-tests ───────────────────────────────────────────────
mkxml() { # DIR NAME TESTS SKIPPED FAILURES ERRORS
    python3 -I - "$@" <<'PY'
import os
import sys

d, name, tests, skipped, failures, errors = sys.argv[1], sys.argv[2], *map(int, sys.argv[3:7])
os.makedirs(d, exist_ok=True)
with open("%s/TEST-%s.xml" % (d, name), "w") as f:
    f.write('<testsuite name="%s" tests="%d" skipped="%d" failures="%d" errors="%d"></testsuite>\n'
            % (name, tests, skipped, failures, errors))
PY
}
mkxml "$TMP/c1" A 5 1 0 0
out="$(bash "$GATE" --count-tests "$TMP/c1" 4 2>/dev/null)"
rc=$?
check "--count-tests counts tests minus skipped" "$([[ $rc -eq 0 && "$out" == "4" ]] && echo 0 || echo 1)" "rc=$rc out=$out"
mkxml "$TMP/c2" A 5 0 1 0
out="$(bash "$GATE" --count-tests "$TMP/c2" 1 2>/dev/null)"
rc=$?
check "--count-tests fails on a failure" "$([[ $rc -eq 1 && "$out" == "5" ]] && echo 0 || echo 1)" "rc=$rc out=$out"
mkxml "$TMP/c3" A 5 0 0 1
bash "$GATE" --count-tests "$TMP/c3" 1 >/dev/null 2>&1
check "--count-tests fails on an error" "$([[ $? -eq 1 ]] && echo 0 || echo 1)" "no failure"
out="$(bash "$GATE" --count-tests "$TMP/c1" 5 2>/dev/null)"
rc=$?
check "--count-tests fails on a low count" "$([[ $rc -eq 1 && "$out" == "4" ]] && echo 0 || echo 1)" "rc=$rc out=$out"
mkdir -p "$TMP/c4"
bash "$GATE" --count-tests "$TMP/c4" 0 >/dev/null 2>&1
check "--count-tests fails when there is no TEST-*.xml" "$([[ $? -eq 1 ]] && echo 0 || echo 1)" "no failure"
mkxml "$TMP/c5" A 3 0 0 0
mkxml "$TMP/c5" B 4 0 0 0
out="$(bash "$GATE" --count-tests "$TMP/c5" 7 2>/dev/null)"
check "--count-tests sums several files" "$([[ "$out" == "7" ]] && echo 0 || echo 1)" "out=$out"
bash "$GATE" --count-tests "$TMP/c5" >/dev/null 2>&1
check "--count-tests without MIN exits 2" "$([[ $? -eq 2 ]] && echo 0 || echo 1)" "wrong rc"

# ── :app Gradle checks ──────────────────────────────────────────
R="$(new_repo)"
printf 'include(":pos-core")\ninclude(":smoke")\n// include(":app")\n' >"$R/android/settings.gradle.kts"
run_gate "$R" -- --base base
skips=0
for c in app-lint app-ktlint app-detekt app-unit-tests app-screenshots app-kover app-assemble-release app-apk-check; do
    [[ "$(status_of $c)" == "SKIP" ]] || skips=1
done
check "the :app checks are SKIP when settings.gradle.kts lacks :app" "$skips" "out=$OUT"
check "the self-test checks are SKIP when nested in a self-test" "$([[ "$(status_of selftest-p4-gate)" == "SKIP" && "$(status_of selftest-heavy)" == "SKIP" ]] && echo 0 || echo 1)" "out=$OUT"
run_gate "$R" -- --fast --base base
check "--fast prints no :app check and no self-test" "$([[ "$(status_of app-lint)" == "" && "$(status_of selftest-heavy)" == "" ]] && echo 0 || echo 1)" "out=$OUT"

R="$(new_repo)"
printf 'include(":app")\n' >"$R/android/settings.gradle.kts"
cat >"$R/android/gradlew" <<'STUB'
#!/usr/bin/env bash
# Stub gradlew: logs its arguments, fakes the outputs the gate reads.
printf '%s\n' "$*" >>"$STUB_GRADLE_LOG"
for t in "$@"; do
    case "$t" in
    :app:testDebugUnitTest)
        mkdir -p app/build/test-results/testDebugUnitTest
        echo '<testsuite name="A" tests="6" skipped="1" failures="0" errors="0"></testsuite>' >app/build/test-results/testDebugUnitTest/TEST-A.xml
        ;;
    :app:koverXmlReportDebug)
        mkdir -p app/build/reports/kover
        cat >app/build/reports/kover/reportDebug.xml <<'XML'
<?xml version="1.0" encoding="UTF-8"?>
<report name="kover"><counter type="INSTRUCTION" missed="1" covered="9"/><counter type="BRANCH" missed="40" covered="60"/><counter type="LINE" missed="25" covered="75"/></report>
XML
        ;;
    :app:assembleRelease)
        mkdir -p app/build/outputs/apk/release
        echo apk >app/build/outputs/apk/release/app-release-unsigned.apk
        ;;
    esac
done
[[ "${STUB_GRADLE_FAIL:-}" != "$*" ]]
STUB
chmod +x "$R/android/gradlew"
printf '#!/bin/sh\necho "APK | zipalign | PASS | ok"\necho "APK | sha256 | PASS | abcdef0123456789"\n' >"$R/scripts/android/check_apk.sh"
export STUB_GRADLE_LOG="$TMP/gradle.log"
: >"$STUB_GRADLE_LOG"
run_gate "$R" PILI_GRADLE_OFFLINE=1 PILI_GRADLE_JVMARGS=-Xmx1g -- --base base --min-app-tests 5
okg=0
for c in app-lint app-ktlint app-detekt app-unit-tests app-kover app-assemble-release app-apk-check; do
    [[ "$(status_of $c)" == "PASS" ]] || okg=1
done
[[ "$(status_of app-screenshots)" == "SKIP" ]] || okg=1
check "the :app checks run Gradle and PASS with a stub gradlew" "$okg" "out=$OUT"
check "app-unit-tests reports the test count" "$([[ "$(detail_of app-unit-tests)" == "5 tests" ]] && echo 0 || echo 1)" "detail=$(detail_of app-unit-tests)"
check "app-kover reports LINE and BRANCH percent" "$([[ "$(detail_of app-kover)" == "LINE 75.0%, BRANCH 60.0%" ]] && echo 0 || echo 1)" "detail=$(detail_of app-kover)"
glog="$(cat "$STUB_GRADLE_LOG")"
check "Gradle gets --no-daemon, --offline, the JVM args and one invocation per check" "$([[ "$glog" == *"--no-daemon --console=plain -Dorg.gradle.jvmargs=-Xmx1g --offline :app:lint"* && "$glog" == *":app:koverVerifyDebug :app:koverXmlReportDebug"* && "$(printf '%s\n' "$glog" | grep -c .)" -eq 6 ]] && echo 0 || echo 1)" "log=$glog"
run_gate "$R" -- --base base --min-app-tests 9
check "app-unit-tests fails below --min-app-tests" "$([[ "$(status_of app-unit-tests)" == "FAIL" ]] && echo 0 || echo 1)" "out=$OUT"
mkdir -p "$R/android/app/src/test/screenshots"
echo png >"$R/android/app/src/test/screenshots/a.png"
: >"$STUB_GRADLE_LOG"
run_gate "$R" -- --base base
check "app-screenshots runs verifyRoborazziDebug once a golden exists" "$([[ "$(status_of app-screenshots)" == "PASS" && "$(cat "$STUB_GRADLE_LOG")" == *":app:verifyRoborazziDebug"* ]] && echo 0 || echo 1)" "out=$OUT"
: >"$STUB_GRADLE_LOG"
run_gate "$R" -- --fast --exit --base base
check "--exit adds plan-gradle with the plan's literal command" "$([[ "$(status_of plan-gradle)" == "PASS" && "$(cat "$STUB_GRADLE_LOG")" == *"lint ktlintCheck detekt test koverVerifyDebug assembleRelease" ]] && echo 0 || echo 1)" "out=$OUT log=$(cat "$STUB_GRADLE_LOG")"
run_gate "$R" STUB_GRADLE_FAIL="--no-daemon --console=plain -Dorg.gradle.jvmargs=-Xmx2g -XX:MaxMetaspaceSize=1g -Dfile.encoding=UTF-8 :app:detekt" -- --base base
check "a failing Gradle check is a FAIL and the others still run" "$([[ "$(status_of app-detekt)" == "FAIL" && "$(status_of app-assemble-release)" == "PASS" && $RC -eq 1 ]] && echo 0 || echo 1)" "out=$OUT"

# ── --app-e2e ───────────────────────────────────────────────────
R="$(new_repo)"
run_gate "$R" -- --fast --app-e2e --base base
check "app-e2e is SKIP while app_e2e.sh is absent" "$([[ "$(status_of app-e2e)" == "SKIP" ]] && echo 0 || echo 1)" "out=$OUT"
printf '#!/bin/sh\nexit 0\n' >"$R/scripts/android/app_e2e.sh"
run_gate "$R" -- --fast --app-e2e --base base
check "app-e2e PASS on exit 0" "$([[ "$(status_of app-e2e)" == "PASS" ]] && echo 0 || echo 1)" "out=$OUT"
printf '#!/bin/sh\nexit 3\n' >"$R/scripts/android/app_e2e.sh"
run_gate "$R" -- --fast --app-e2e --base base
check "app-e2e FAIL on a non-zero exit" "$([[ "$(status_of app-e2e)" == "FAIL" ]] && echo 0 || echo 1)" "out=$OUT"

# ── --linux-repro (stub docker, stub check_repro.sh, temporary HOME) ──
mkdir -p "$TMP/bin"
printf '#!/bin/sh\nexit 0\n' >"$TMP/bin/docker"
chmod +x "$TMP/bin/docker"
R="$(new_repo)"
SHA="9d230ef1bba90a4edde0157bcb107e4bb4d23d6e"
printf 'Result: `✓ Reproducible: SHA-1 %s`\n' "$SHA" >"$R/docs/android/LINUX_FREEZE.md"
mkdir -p "$R/scripts" "$R/deploy"
cat >"$R/scripts/check_repro.sh" <<STUB
#!/bin/sh
mkdir -p deploy/dist target/repro
echo built >deploy/dist/pili-pos
echo "✓ Reproducible: SHA-1 \${REPRO_SHA:-$SHA}"
exit \${REPRO_RC:-0}
STUB
tgit "$R" add -A
tgit "$R" commit -q -m "chore: repro stub"
tgit "$R" tag -f linux-v1.0.0-rc2 >/dev/null
tgit "$R" tag -f base >/dev/null HEAD~1
run_gate "$R" PATH="$TMP/bin:$PATH" -- --fast --linux-repro --base base
okr=1
[[ "$(status_of linux-repro)" == "PASS" && ! -e "$HOME/.cache/pili-freeze/p4-clone/deploy/dist" && -f "$HOME/.cache/pili-freeze/p4-linux-repro.log" ]] && okr=0
check "linux-repro PASSes on the recorded SHA-1, cleans the outputs and keeps the log" "$okr" "out=$OUT"
run_gate "$R" PATH="$TMP/bin:$PATH" REPRO_SHA=0000000000000000000000000000000000000000 -- --fast --linux-repro --base base
check "linux-repro FAILs on another SHA-1" "$([[ "$(status_of linux-repro)" == "FAIL" ]] && echo 0 || echo 1)" "out=$OUT"
run_gate "$R" PATH="$TMP/bin:$PATH" REPRO_RC=1 -- --fast --linux-repro --base base
check "linux-repro FAILs when check_repro.sh fails" "$([[ "$(status_of linux-repro)" == "FAIL" ]] && echo 0 || echo 1)" "out=$OUT"
run_gate "$R" -- --fast --base base
check "linux-repro is not run without --linux-repro" "$([[ "$(status_of linux-repro)" == "" ]] && echo 0 || echo 1)" "out=$OUT"

printf 'TEST | summary | %s | %d passed, %d failed\n' "$([[ $fail -eq 0 ]] && echo PASS || echo FAIL)" "$pass" "$fail"
[[ $fail -eq 0 ]]
