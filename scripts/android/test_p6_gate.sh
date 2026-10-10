#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of scripts/android/p6_gate.sh
#
# Usage: bash scripts/android/test_p6_gate.sh
#
# Runs p6_gate.sh against temporary git repositories (PILI_P6_ROOT) with a stub
# sub-gate (PILI_P6_SUBGATE) and stub self-test and end-to-end scripts. No Gradle, no
# network, no Docker, no emulator. Cases: option parsing, the pass-through of the p4
# options and the counting of the sub-gate lines, the total line, then each P6 check
# (plat-manifest, plat-backup-rules, plat-strings, plat-boundary, plat-keys,
# plat-engine-ref) in PASS and FAIL, the full-mode self-tests and --plat-e2e.
# Prints one line per case, then
#     TESTP6 | total | PASS|FAIL | <n> tests
# and exits 1 when a case fails.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
GATE="$ROOT/scripts/android/p6_gate.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
export HOME="$TMP/home"
mkdir -p "$HOME"
unset PILI_P6_NESTED PILI_P6_ROOT PILI_P6_SUBGATE PILI_P6_FREEZE_TAG PILI_P4_ROOT
pass=0
fail=0

ok() { pass=$((pass + 1)); printf 'TESTP6 | %s | PASS\n' "$1"; }
bad() { fail=$((fail + 1)); printf 'TESTP6 | %s | FAIL | %s\n' "$1" "$2"; }

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

RES=android/app/src/main/res
PLAT=android/app/src/main/kotlin/dev/nuagemagique/pilipos/platform
DEV_KEY="RWQmv3D5F4695q2BBUUzfk0ABpONbcMKPePaMqZkpr5UV8J0PCTEdO/v"
REL_KEY="RW$(printf 'B%.0s' $(seq 1 54))"

# write FILE: stdin to FILE (directories are created)
write() {
    mkdir -p "$(dirname "$1")"
    cat >"$1"
}

# A pristine repository that every case starts from (copied, never edited).
PRISTINE="$TMP/pristine"
build_pristine() {
    local r="$PRISTINE"
    mkdir -p "$r/scripts/android"
    write "$r/Cargo.toml" <<'EOF'
[package]
name = "decoy"
version = "0.0.1"

[workspace]
members = []

[workspace.package]
version = "1.0.0"
edition = "2021"
EOF
    write "$r/android/app/src/main/AndroidManifest.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    xmlns:tools="http://schemas.android.com/tools">
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.RECEIVE_BOOT_COMPLETED" />
    <application
        android:name=".PiliApplication"
        android:allowBackup="false"
        android:dataExtractionRules="@xml/data_extraction_rules"
        android:fullBackupContent="false">
    </application>
</manifest>
EOF
    write "$r/$RES/xml/data_extraction_rules.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<data-extraction-rules>
    <cloud-backup>
        <exclude domain="root" />
        <exclude domain="file" />
        <exclude domain="database" />
        <exclude domain="sharedpref" />
        <exclude domain="external" />
    </cloud-backup>
    <device-transfer>
        <exclude domain="root" />
        <exclude domain="file" />
        <exclude domain="database" />
        <exclude domain="sharedpref" />
        <exclude domain="external" />
    </device-transfer>
</data-extraction-rules>
EOF
    local lang title policy
    for lang in values values-en values-zh; do
        case "$lang" in values) title="À propos"; policy="Règles" ;; values-en) title="About"; policy="Rules" ;; *) title="关于"; policy="规则" ;; esac
        write "$r/$RES/$lang/strings_plat_about.xml" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="plat_about_title">$title</string>
</resources>
EOF
        write "$r/$RES/$lang/strings_plat_policy.xml" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="plat_policy_title">$policy</string>
    <plurals name="plat_policy_count">
        <item quantity="one">1</item>
        <item quantity="other">%d</item>
    </plurals>
</resources>
EOF
    done
    write "$r/$PLAT/PlatformRoutes.kt" <<'EOF'
package dev.nuagemagique.pilipos.platform

// The platform never touches EngineHolder or calls force-stop (these words are in a comment).
/* vatSplit in a block comment /* nested */ still a comment, http://example.org too */
object PlatformRoutes {
    const val ABOUT = "platform/about"
    const val DEBUG_REGISTRY = "http://127.0.0.1:8099"
    const val LOCAL = "http://localhost:8099"
}
EOF
    write "$r/android/app/provisioning/dev-minisign.pub" <<EOF
untrusted comment: minisign public key E6BD8E17F970BF26
$DEV_KEY
EOF
    {
        git -C "$r" init -q -b main
        tgit "$r" add -A
        tgit "$r" commit -q -m "chore: base"
        tgit "$r" tag base
        tgit "$r" tag linux-v1.0.0-rc1
    } >/dev/null 2>&1
}
build_pristine
SHORT="$(tgit "$PRISTINE" rev-parse 'linux-v1.0.0-rc1^{commit}' | cut -c1-7)"
GOOD_REF="pili-pos 1.0.0, linux-v1.0.0-rc1 (${SHORT})"
PLACEHOLDER_REF="pili-pos 1.0.0, linux-v1.0.0-rc1 (pending)"

# new_repo: a copy of the pristine repository. Prints its path.
new_repo() {
    local r
    r="$(mktemp -d "$TMP/repo.XXXXXX")"
    cp -R "$PRISTINE/." "$r/"
    printf '%s' "$r"
}

# subst FILE OLD NEW: replaces every OLD by NEW in FILE; fails when OLD is absent.
subst() {
    python3 -I - "$1" "$2" "$3" <<'PY'
import sys

path, old, new = sys.argv[1:4]
with open(path, encoding="utf-8") as f:
    data = f.read()
if old not in data:
    print("subst: %r not in %s" % (old, path), file=sys.stderr)
    sys.exit(1)
with open(path, "w", encoding="utf-8") as f:
    f.write(data.replace(old, new))
PY
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
    [[ "$repo" == "$TMP"/* && -d "$repo/.git" ]] || { echo "run_gate: bad repo '$repo'" >&2; exit 1; }
    OUT="$(env PILI_P6_ROOT="$repo" PILI_P6_SUBGATE="$STUB_SUBGATE" STUB_LOG="$TMP/stub.log" ${envs[@]+"${envs[@]}"} \
        bash "$GATE" "$@" 2>"$TMP/err.txt")"
    RC=$?
    ERR="$(cat "$TMP/err.txt")"
}

status_of() { printf '%s\n' "$OUT" | sed -n "s/^GATE | $1 | \\([A-Z]*\\) | .*/\\1/p" | head -n 1; }
detail_of() { printf '%s\n' "$OUT" | sed -n "s/^GATE | $1 | [A-Z]* | //p" | head -n 1; }

# expect CASE CHECK STATUS: the check CHECK has status STATUS in OUT.
expect() {
    local got
    got="$(status_of "$2")"
    check "$1" "$([[ "$got" == "$3" ]] && echo 0 || echo 1)" "$2 is '$got', expected $3 (detail: $(detail_of "$2"))"
}

# fresh CASE-FREE helper: a mutated repository, then the fast gate.
fast() { run_gate "$1" -- --fast; }

# ── option parsing ──────────────────────────────────────────────
R="$(new_repo)"
run_gate "$R" -- --bogus
check "an unknown option exits 2" "$([[ $RC -eq 2 ]] && echo 0 || echo 1)" "rc=$RC"
run_gate "$R" -- --base
check "--base without a value exits 2" "$([[ $RC -eq 2 ]] && echo 0 || echo 1)" "rc=$RC"
run_gate "$R" -- --allow
check "--allow without a value exits 2" "$([[ $RC -eq 2 ]] && echo 0 || echo 1)" "rc=$RC"
run_gate "$R" -- --help
check "--help prints the header and exits 0" "$([[ $RC -eq 0 && "$OUT" == *"P6 gate"* ]] && echo 0 || echo 1)" "rc=$RC"

# ── pass-through, counting, total line ──────────────────────────
: >"$TMP/stub.log"
run_gate "$R" -- --app --fast --base base --allow '^x$' --min-app-tests 3 --plat-e2e
args="$(cat "$TMP/stub.log")"
okp=1
[[ "$args" == *"--app"* && "$args" == *"--fast"* && "$args" == *"--base base"* && "$args" == *"--allow ^x\$"* && "$args" == *"--min-app-tests 3"* && "$args" != *"--plat-e2e"* ]] && okp=0
check "the p4 options are passed to the sub-gate and --plat-e2e is not" "$okp" "args='$args'"

: >"$TMP/stub.log"
run_gate "$R" -- --exit --android --root --emulator --device-tests --repro --app-e2e --linux-repro
args="$(cat "$TMP/stub.log")"
all=0
for o in --exit --android --root --emulator --device-tests --repro --app-e2e --linux-repro; do
    [[ "$args" == *"$o"* ]] || all=1
done
check "every other p4 option is passed through" "$all" "args='$args'"

fast "$R"
total_lines="$(printf '%s\n' "$OUT" | grep -c '^GATE | total | ')"
last="$(printf '%s\n' "$OUT" | tail -n 1)"
fmt=1
[[ $RC -eq 0 && "$total_lines" -eq 1 && "$last" =~ ^GATE\ \|\ total\ \|\ PASS\ \|\ 0\ failed\ \([0-9]+\ checks\)$ ]] && fmt=0
check "a good tree: one total line 'GATE | total | PASS | 0 failed (N checks)' and exit 0" "$fmt" "rc=$RC last='$last' err=$ERR"
expect "the sub-gate check lines are re-printed" stub-one PASS
expect "a SKIP line of the sub-gate is re-printed" stub-two SKIP
# 2 stub checks + 6 P6 checks in --fast
check "the checks are counted: 2 sub-gate + 6 P6 checks in --fast" "$([[ "$last" == *"(8 checks)" ]] && echo 0 || echo 1)" "last='$last'"
order="$(printf '%s\n' "$OUT" | sed -n 's/^GATE | \([a-z0-9-]*\) | .*/\1/p' | tr '\n' ' ')"
check "the P6 checks come after the sub-gate lines in order" "$([[ "$order" == "stub-one stub-two plat-manifest plat-backup-rules plat-strings plat-boundary plat-keys plat-engine-ref total " ]] && echo 0 || echo 1)" "order='$order'"
expect "plat-manifest PASS on a good tree" plat-manifest PASS
expect "plat-backup-rules PASS on a good tree" plat-backup-rules PASS
expect "plat-strings PASS on a good tree" plat-strings PASS
expect "plat-boundary PASS on a good tree" plat-boundary PASS
expect "plat-keys SKIP while plat_update_keys.xml is absent" plat-keys SKIP
expect "plat-engine-ref SKIP while no engine ref string exists" plat-engine-ref SKIP

run_gate "$R" STUB_FAIL=1 -- --fast
last="$(printf '%s\n' "$OUT" | tail -n 1)"
check "a failing sub-gate check fails the gate (exit 1, count 1)" "$([[ $RC -eq 1 && "$last" =~ ^GATE\ \|\ total\ \|\ FAIL\ \|\ 1\ failed\ \([0-9]+\ checks\)$ && "$(status_of stub-two)" == "FAIL" && "$(status_of plat-manifest)" == "PASS" ]] && echo 0 || echo 1)" "rc=$RC last='$last'"

run_gate "$R" STUB_NO_TOTAL=1 -- --fast
check "a sub-gate without a total line fails" "$([[ $RC -eq 1 && "$(status_of p4-gate)" == "FAIL" ]] && echo 0 || echo 1)" "rc=$RC out=$OUT"

# ── plat-manifest ───────────────────────────────────────────────
M=android/app/src/main/AndroidManifest.xml
manifest_case() { # <case> <old> <new>
    local r
    r="$(new_repo)"
    subst "$r/$M" "$2" "$3" || { bad "$1" "subst failed"; return; }
    fast "$r"
    expect "$1" plat-manifest FAIL
    LAST_REPO="$r"
}
manifest_case "plat-manifest FAIL when allowBackup is true" 'allowBackup="false"' 'allowBackup="true"'
manifest_case "plat-manifest FAIL when fullBackupContent is missing" 'android:fullBackupContent="false"' ''
manifest_case "plat-manifest FAIL when dataExtractionRules points elsewhere" '@xml/data_extraction_rules' '@xml/other_rules'
r="$(new_repo)"
subst "$r/$M" '<manifest xmlns:android="http://schemas.android.com/apk/res/android"' '<manifest xmlns:android="http://schemas.android.com/apk/res/android" android:installLocation="auto"'
fast "$r"
expect "plat-manifest FAIL when android:installLocation=auto is added to <manifest>" plat-manifest FAIL
check "the failed check prints its problems on stderr" "$([[ "$ERR" == *installLocation* ]] && echo 0 || echo 1)" "err=$ERR"
for perm in REQUEST_INSTALL_PACKAGES READ_EXTERNAL_STORAGE WRITE_EXTERNAL_STORAGE MANAGE_EXTERNAL_STORAGE READ_MEDIA_IMAGES READ_MEDIA_VIDEO; do
    r="$(new_repo)"
    subst "$r/$M" '<application' "<uses-permission android:name=\"android.permission.$perm\" /><application"
    fast "$r"
    expect "plat-manifest FAIL on $perm" plat-manifest FAIL
done
r="$(new_repo)"
subst "$r/$M" '<application' '<uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" tools:node="remove" /><application'
fast "$r"
expect "plat-manifest PASS when a storage permission is only removed (tools:node=remove)" plat-manifest PASS
r="$(new_repo)"
rm "$r/$M"
fast "$r"
expect "plat-manifest FAIL when the manifest is missing" plat-manifest FAIL

# ── plat-backup-rules ───────────────────────────────────────────
RULES=$RES/xml/data_extraction_rules.xml
r="$(new_repo)"
python3 -I - "$r/$RULES" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
i = s.index("<device-transfer>")
head, tail = s[:i], s[i:]
tail = tail.replace('<exclude domain="external" />', "", 1)
open(p, "w").write(head + tail)
PY
fast "$r"
expect "plat-backup-rules FAIL when device-transfer does not exclude external" plat-backup-rules FAIL
check "the failed rules check names the missing domain" "$([[ "$ERR" == *"device-transfer"*"external"* ]] && echo 0 || echo 1)" "err=$ERR"
r="$(new_repo)"
python3 -I - "$r/$RULES" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
i = s.index("<cloud-backup>")
j = s.index("</cloud-backup>") + len("</cloud-backup>")
open(p, "w").write(s[:i] + s[j:])
PY
fast "$r"
expect "plat-backup-rules FAIL when cloud-backup is missing" plat-backup-rules FAIL
r="$(new_repo)"
subst "$r/$RULES" '</cloud-backup>' '<include domain="file" path="keep" /></cloud-backup>'
fast "$r"
expect "plat-backup-rules FAIL when an include is present" plat-backup-rules FAIL
r="$(new_repo)"
subst "$r/$RULES" 'domain="sharedpref"' 'domain="sharedpreferences"'
fast "$r"
expect "plat-backup-rules FAIL when a domain is misspelled" plat-backup-rules FAIL
r="$(new_repo)"
rm "$r/$RULES"
fast "$r"
expect "plat-backup-rules FAIL when the file is missing" plat-backup-rules FAIL

# ── plat-strings ────────────────────────────────────────────────
r="$(new_repo)"
subst "$r/$RES/values-zh/strings_plat_policy.xml" '<string name="plat_policy_title">规则</string>' ''
fast "$r"
expect "plat-strings FAIL when a key is missing in values-zh" plat-strings FAIL
check "the failed strings check names the key and the folder" "$([[ "$ERR" == *"values-zh"*"plat_policy_title"* ]] && echo 0 || echo 1)" "err=$ERR"
r="$(new_repo)"
rm "$r/$RES/values-en/strings_plat_about.xml"
fast "$r"
expect "plat-strings FAIL when a values-en file is missing" plat-strings FAIL
r="$(new_repo)"
subst "$r/$RES/values/strings_plat_about.xml" 'plat_about_title' 'plat_abut_title'
subst "$r/$RES/values-en/strings_plat_about.xml" 'plat_about_title' 'plat_abut_title'
subst "$r/$RES/values-zh/strings_plat_about.xml" 'plat_about_title' 'plat_abut_title'
fast "$r"
expect "plat-strings FAIL on a wrong prefix (same in all three folders)" plat-strings FAIL
r="$(new_repo)"
subst "$r/$RES/values/strings_plat_about.xml" 'plat_about_title' 'ui_title'
subst "$r/$RES/values-en/strings_plat_about.xml" 'plat_about_title' 'ui_title'
subst "$r/$RES/values-zh/strings_plat_about.xml" 'plat_about_title' 'ui_title'
fast "$r"
expect "plat-strings FAIL on a name outside the plat_<area>_ prefix" plat-strings FAIL
r="$(new_repo)"
subst "$r/$RES/values-en/strings_plat_about.xml" '>About<' '><'
fast "$r"
expect "plat-strings FAIL on an empty value" plat-strings FAIL
r="$(new_repo)"
subst "$r/$RES/values-en/strings_plat_about.xml" '</resources>' '<string name="plat_about_extra">Extra</string></resources>'
fast "$r"
expect "plat-strings FAIL on an extra key in values-en" plat-strings FAIL
r="$(new_repo)"
write "$r/$RES/values-en/strings_plat_orphan.xml" <<'EOF'
<resources><string name="plat_orphan_x">x</string></resources>
EOF
fast "$r"
expect "plat-strings FAIL on a values-en file without a values twin" plat-strings FAIL
r="$(new_repo)"
subst "$r/$RES/values/strings_plat_about.xml" '</resources>' '<string name="plat_about_engine_ref" translatable="false">pili-pos x</string></resources>'
fast "$r"
expect "plat-strings PASS with a translatable=false string only in values/" plat-strings PASS
r="$(new_repo)"
subst "$r/$RES/values/strings_plat_about.xml" '</resources>' '<string name="plat_about_engine_ref" translatable="false">pili-pos x</string></resources>'
subst "$r/$RES/values-zh/strings_plat_about.xml" '</resources>' '<string name="plat_about_engine_ref" translatable="false">pili-pos x</string></resources>'
fast "$r"
expect "plat-strings FAIL when a translatable=false string is also in values-zh" plat-strings FAIL
r="$(new_repo)"
subst "$r/$RES/values/strings_plat_about.xml" '</resources>' '<string name="plat_about_engine_ref">pili-pos x</string></resources>'
fast "$r"
expect "plat-strings FAIL when a translatable string is only in values/" plat-strings FAIL
r="$(new_repo)"
subst "$r/$RES/values-zh/strings_plat_policy.xml" '<item quantity="one">1</item>' '<item quantity="one"></item>'
fast "$r"
expect "plat-strings FAIL on an empty plurals item" plat-strings FAIL
r="$(new_repo)"
rm -f "$r/$RES"/values/strings_plat_*.xml
fast "$r"
expect "plat-strings FAIL when no strings_plat file exists" plat-strings FAIL

# ── plat-boundary ───────────────────────────────────────────────
K="$PLAT/PlatformRoutes.kt"
boundary_case() { # <case> <kotlin line> <PASS|FAIL>
    local r
    r="$(new_repo)"
    printf '%s\n' "$2" >>"$r/$K"
    fast "$r"
    expect "$1" plat-boundary "$3"
}
boundary_case "plat-boundary FAIL on EngineHolder in code" 'val h = EngineHolder' FAIL
boundary_case "plat-boundary FAIL on forceStop in code" 'fun x() = am.forceStop()' FAIL
boundary_case "plat-boundary FAIL on force-stop in a string" 'val c = "am force-stop pkg"' FAIL
boundary_case "plat-boundary FAIL on killBackgroundProcesses" 'fun k() = am.killBackgroundProcesses(p)' FAIL
boundary_case "plat-boundary FAIL on settlePreview" 'val s = settlePreview(1)' FAIL
boundary_case "plat-boundary FAIL on vatSplit" 'val v = vatSplit(1)' FAIL
boundary_case "plat-boundary FAIL on cashDueCents" 'val c = cashDueCents(1)' FAIL
boundary_case "plat-boundary FAIL on an http:// literal to a real host" 'const val U = "http://ghcr.io/v2"' FAIL
boundary_case "plat-boundary PASS on an https:// literal" 'const val U = "https://ghcr.io/v2"' PASS
boundary_case "plat-boundary PASS on http://127.0.0.1" 'const val U2 = "http://127.0.0.1:1"' PASS
boundary_case "plat-boundary PASS on an http:// literal in a line comment" '// see http://example.org and EngineHolder' PASS
boundary_case "plat-boundary PASS on a name that only contains a forbidden word" 'val EngineHolderLike = 1' PASS
r="$(new_repo)"
write "$r/$PLAT/audit/AuditLog.kt" <<'EOF'
package dev.nuagemagique.pilipos.platform.audit

class AuditLog { val s = """raw http://evil.example string""" }
EOF
fast "$r"
expect "plat-boundary FAIL on an http:// literal in a raw string of a sub-package" plat-boundary FAIL
check "the failed boundary check names the file and line" "$([[ "$ERR" == *"AuditLog.kt:3"* ]] && echo 0 || echo 1)" "err=$ERR"
r="$(new_repo)"
printf 'val a = "x"\nval b = EngineHolder\n' >"$r/android/app/src/main/kotlin/dev/nuagemagique/pilipos/Other.kt"
fast "$r"
expect "plat-boundary ignores files outside platform/" plat-boundary PASS

# ── plat-keys ───────────────────────────────────────────────────
KEYS=$RES/xml/plat_update_keys.xml
keys_case() { # <case> <debug> <release> <PASS|FAIL>
    local r
    r="$(new_repo)"
    write "$r/$KEYS" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<update-keys>
    <key build="debug">$2</key>
    <key build="release">$3</key>
</update-keys>
EOF
    fast "$r"
    expect "$1" plat-keys "$4"
}
keys_case "plat-keys PASS with the dev key and an empty release key" "$DEV_KEY" "" PASS
keys_case "plat-keys PASS with the dev key and a valid release key" "$DEV_KEY" "$REL_KEY" PASS
keys_case "plat-keys FAIL with a wrong debug key" "RWQmv3D5F4695q2BBUUzfk0ABpONbcMKPePaMqZkpr5UV8J0PCTEdO/w" "" FAIL
keys_case "plat-keys FAIL with an empty debug key" "" "" FAIL
keys_case "plat-keys FAIL with a malformed release key" "$DEV_KEY" "RWshort" FAIL
keys_case "plat-keys FAIL with a release key of the wrong prefix" "$DEV_KEY" "XX$(printf 'B%.0s' $(seq 1 54))" FAIL
r="$(new_repo)"
write "$r/$KEYS" <<EOF
<update-keys><key build="debug">$DEV_KEY</key></update-keys>
EOF
fast "$r"
expect "plat-keys FAIL when the release entry is missing" plat-keys FAIL
r="$(new_repo)"
write "$r/$KEYS" <<EOF
<update-keys><key build="debug" key="$DEV_KEY" /><key build="release" /></update-keys>
EOF
fast "$r"
expect "plat-keys PASS with the key in an attribute" plat-keys PASS
r="$(new_repo)"
write "$r/$KEYS" <<EOF
<update-keys><key build="debug">$DEV_KEY</key><key build="release"></key></update-keys>
EOF
rm "$r/android/app/provisioning/dev-minisign.pub"
fast "$r"
expect "plat-keys FAIL when dev-minisign.pub is missing" plat-keys FAIL

# ── plat-engine-ref ─────────────────────────────────────────────
r="$(new_repo)"
subst "$r/$RES/values/strings_plat_about.xml" '</resources>' "<string name=\"plat_about_engine_ref\" translatable=\"false\">$GOOD_REF</string></resources>"
fast "$r"
expect "plat-engine-ref PASS when the string equals 'pili-pos V, TAG (SHORT)'" plat-engine-ref PASS
expect "plat-strings still PASS with the engine ref string" plat-strings PASS
check "the PASS detail is the reference" "$([[ "$(detail_of plat-engine-ref)" == "$GOOD_REF" ]] && echo 0 || echo 1)" "detail=$(detail_of plat-engine-ref)"
for wrong in "pili-pos 1.0.1, linux-v1.0.0-rc1 (${SHORT})" "pili-pos 1.0.0, linux-v1.0.0-rc1 (0000000)" "pili-pos 1.0.0, linux-v1.1.0-rc1 (${SHORT})" "pili-pos 0.0.1, linux-v1.0.0-rc1 (${SHORT})" "pili-pos 1.0.0 , linux-v1.0.0-rc1 (${SHORT})" "$PLACEHOLDER_REF"; do
    r="$(new_repo)"
    subst "$r/$RES/values/strings_plat_about.xml" '</resources>' "<string name=\"plat_about_engine_ref\" translatable=\"false\">$wrong</string></resources>"
    fast "$r"
    expect "plat-engine-ref FAIL on '$wrong'" plat-engine-ref FAIL
done
r="$(new_repo)"
tgit "$r" tag custom-freeze >/dev/null 2>&1
printf '\n' >>"$r/Cargo.toml"
subst "$r/Cargo.toml" 'version = "1.0.0"' 'version = "2.0.0"'
tgit "$r" commit -q -am "chore: bump" >/dev/null 2>&1
tgit "$r" tag custom-freeze-2 >/dev/null 2>&1
short2="$(tgit "$r" rev-parse 'custom-freeze-2^{commit}' | cut -c1-7)"
subst "$r/$RES/values/strings_plat_about.xml" '</resources>' "<string name=\"plat_about_engine_ref\" translatable=\"false\">pili-pos 2.0.0, custom-freeze-2 (${short2})</string></resources>"
run_gate "$r" PILI_P6_FREEZE_TAG=custom-freeze-2 -- --fast
expect "plat-engine-ref follows PILI_P6_FREEZE_TAG" plat-engine-ref PASS
run_gate "$r" PILI_P6_FREEZE_TAG=no-such-tag -- --fast
expect "plat-engine-ref FAIL when the freeze tag does not exist" plat-engine-ref FAIL
# Before the 1.0.0 re-freeze: no tag yet, the string holds the placeholder.
r="$(new_repo)"
tgit "$r" tag -d linux-v1.0.0-rc1 >/dev/null 2>&1
subst "$r/$RES/values/strings_plat_about.xml" '</resources>' "<string name=\"plat_about_engine_ref\" translatable=\"false\">$PLACEHOLDER_REF</string></resources>"
fast "$r"
expect "plat-engine-ref SKIP on the placeholder while the tag does not exist" plat-engine-ref SKIP
check "the SKIP detail says the placeholder is filled at tagging" "$([[ "$(detail_of plat-engine-ref)" == *"no tag linux-v1.0.0-rc1 yet"*"filled at tagging"* ]] && echo 0 || echo 1)" "detail=$(detail_of plat-engine-ref)"
for wrong in "pili-pos 1.0.0, linux-v1.0.0-rc1 (${SHORT})" "pili-pos 1.1.0, linux-v1.0.0-rc1 (pending)" "pili-pos 1.0.0, linux-v1.1.0-rc1 (pending)"; do
    r="$(new_repo)"
    tgit "$r" tag -d linux-v1.0.0-rc1 >/dev/null 2>&1
    subst "$r/$RES/values/strings_plat_about.xml" '</resources>' "<string name=\"plat_about_engine_ref\" translatable=\"false\">$wrong</string></resources>"
    fast "$r"
    expect "plat-engine-ref FAIL on '$wrong' while the tag does not exist" plat-engine-ref FAIL
done

# ── full mode: self-tests ───────────────────────────────────────
r="$(new_repo)"
write "$r/scripts/android/test_p6_gate.sh" <<'EOF'
#!/usr/bin/env bash
echo "TESTP6 | total | PASS | 1 tests"
EOF
run_gate "$r" -- --app
expect "selftest-p6-gate PASS in full mode" selftest-p6-gate PASS
expect "selftest-declare SKIP while test_declare_android.sh is absent" selftest-declare SKIP
last="$(printf '%s\n' "$OUT" | tail -n 1)"
check "full mode counts the two self-test checks (2 + 6 + 2)" "$([[ $RC -eq 0 && "$last" == *"(10 checks)" ]] && echo 0 || echo 1)" "rc=$RC last='$last'"
write "$r/scripts/release/test_declare_android.sh" <<'EOF'
#!/usr/bin/env bash
echo "declare ok"
EOF
run_gate "$r" -- --app
expect "selftest-declare PASS when the script exists and exits 0" selftest-declare PASS
write "$r/scripts/release/test_declare_android.sh" <<'EOF'
#!/usr/bin/env bash
echo "declare broken"
exit 1
EOF
run_gate "$r" -- --app
expect "selftest-declare FAIL when the script exits 1" selftest-declare FAIL
check "a failing self-test fails the gate" "$([[ $RC -eq 1 ]] && echo 0 || echo 1)" "rc=$RC"
write "$r/scripts/android/test_p6_gate.sh" <<'EOF'
#!/usr/bin/env bash
echo "TESTP6 | total | FAIL | 1 of 2"
exit 1
EOF
run_gate "$r" -- --app
expect "selftest-p6-gate FAIL when its script exits 1" selftest-p6-gate FAIL
run_gate "$r" PILI_P6_NESTED=1 -- --app
expect "selftest-p6-gate SKIP when nested" selftest-p6-gate SKIP
expect "selftest-declare SKIP when nested" selftest-declare SKIP
run_gate "$r" -- --app --fast
check "--fast lists no self-test" "$([[ -z "$(status_of selftest-p6-gate)" && -z "$(status_of selftest-declare)" ]] && echo 0 || echo 1)" "out=$OUT"
rm "$r/scripts/android/test_p6_gate.sh"
run_gate "$r" -- --app
expect "selftest-p6-gate FAIL when test_p6_gate.sh is absent" selftest-p6-gate FAIL

# ── --plat-e2e ──────────────────────────────────────────────────
r="$(new_repo)"
fast "$r"
check "plat-e2e is not listed without --plat-e2e" "$([[ -z "$(status_of plat-e2e)" ]] && echo 0 || echo 1)" "out=$OUT"
run_gate "$r" -- --fast --plat-e2e
expect "plat-e2e SKIP while plat_e2e.sh is absent" plat-e2e SKIP
write "$r/scripts/android/plat_e2e.sh" <<'EOF'
#!/usr/bin/env bash
echo "PLATE2E | emulator | PASS | reused"
echo "PLATE2E | total | PASS | 0 failed (1 checks)"
EOF
run_gate "$r" -- --fast --plat-e2e
expect "plat-e2e PASS on its 'PLATE2E | total | PASS' line" plat-e2e PASS
check "the plat-e2e detail is the end of its total line" "$([[ "$(detail_of plat-e2e)" == "0 failed (1 checks)" ]] && echo 0 || echo 1)" "detail=$(detail_of plat-e2e)"
write "$r/scripts/android/plat_e2e.sh" <<'EOF'
#!/usr/bin/env bash
echo "PLATE2E | export | FAIL | no stick"
echo "PLATE2E | total | FAIL | 1 failed (2 checks)"
exit 1
EOF
run_gate "$r" -- --fast --plat-e2e
expect "plat-e2e FAIL on a failing run" plat-e2e FAIL
check "a failing plat-e2e fails the gate" "$([[ $RC -eq 1 ]] && echo 0 || echo 1)" "rc=$RC"
write "$r/scripts/android/plat_e2e.sh" <<'EOF'
#!/usr/bin/env bash
echo "no total line"
EOF
run_gate "$r" -- --fast --plat-e2e
expect "plat-e2e FAIL when the run prints no PASS total, even with exit 0" plat-e2e FAIL

# ── every check runs after a failure ────────────────────────────
r="$(new_repo)"
subst "$r/$M" 'allowBackup="false"' 'allowBackup="true"'
subst "$r/$RULES" 'domain="root"' 'domain="rooot"'
printf 'val h = EngineHolder\n' >>"$r/$K"
fast "$r"
last="$(printf '%s\n' "$OUT" | tail -n 1)"
check "three failing P6 checks are all reported and counted" "$([[ $RC -eq 1 && "$(status_of plat-manifest)" == FAIL && "$(status_of plat-backup-rules)" == FAIL && "$(status_of plat-boundary)" == FAIL && "$(status_of plat-strings)" == PASS && "$last" =~ ^GATE\ \|\ total\ \|\ FAIL\ \|\ 3\ failed\ \([0-9]+\ checks\)$ ]] && echo 0 || echo 1)" "rc=$RC last='$last'"

n=$((pass + fail))
if [[ "$fail" -eq 0 ]]; then
    printf 'TESTP6 | total | PASS | %d tests\n' "$n"
    exit 0
fi
printf 'TESTP6 | total | FAIL | %d of %d tests failed\n' "$fail" "$n"
exit 1
