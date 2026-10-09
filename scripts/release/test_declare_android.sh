#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of scripts/release/declare_android.sh (P6-T9)
#
# Usage: scripts/release/test_declare_android.sh
#
# Everything happens in one mktemp directory outside this repository, deleted on exit: a
# throwaway RSA-2048 keystore, a throwaway EC keystore, throwaway minisign key pairs, tiny test
# APKs built with aapt2 and zipalign (no Gradle), a copy of the script inside a scratch git
# repository (so the local tag and the release key of plat_update_keys.xml are the scratch ones),
# and STUB oras and gh that record their arguments. The real oras and gh are never reached, the
# real script is only ever run with --dry-run apart from the stubbed cases, and no key of the
# release is involved. Prints "TESTDECLARE | <case> | PASS|FAIL | <detail>" and a final
# "TESTDECLARE | total | PASS|FAIL | <n> cases". Exit 0 when every case passes.
#
# Needs: $ANDROID_HOME build-tools 35.0.0 and a platform android.jar, keytool, minisign,
# python3, git.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPT_SRC="$HERE/declare_android.sh"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
BT="$ANDROID_HOME/build-tools/35.0.0"

total=0
failed=0
result() { # result NAME PASS|FAIL DETAIL
    total=$((total + 1))
    [[ "$2" == PASS ]] || failed=$((failed + 1))
    printf 'TESTDECLARE | %s | %s | %s\n' "$1" "$2" "$3"
}
expect() { # expect NAME CONDITION_RESULT(0=true) DETAIL_ON_FAIL
    if [[ "$2" -eq 0 ]]; then result "$1" PASS "ok"; else result "$1" FAIL "$3"; fi
}
bail() {
    printf 'TESTDECLARE | setup | FAIL | %s\n' "$1"
    printf 'TESTDECLARE | total | FAIL | setup failed\n'
    exit 1
}

for t in "$BT/apksigner" "$BT/zipalign" "$BT/aapt2"; do
    [[ -x "$t" ]] || bail "missing $t"
done
command -v keytool >/dev/null 2>&1 || bail "keytool not found"
command -v minisign >/dev/null 2>&1 || bail "minisign not found"
command -v python3 >/dev/null 2>&1 || bail "python3 not found"
command -v git >/dev/null 2>&1 || bail "git not found"
ANDROID_JAR=""
for d in $(ls -d "$ANDROID_HOME"/platforms/android-* 2>/dev/null | sort -t- -k2 -n -r); do
    if [[ -f "$d/android.jar" ]]; then ANDROID_JAR="$d/android.jar"; break; fi
done
[[ -n "$ANDROID_JAR" ]] || bail "no android.jar under $ANDROID_HOME/platforms"

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT
if git -C "$T" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    bail "the scratch directory $T lies inside a git work tree"
fi

# ── throwaway keys ─────────────────────────────────────────────
KEYS="$T/keys"
mkdir -p "$KEYS"
printf 'throwaway-pass-1\n' >"$KEYS/pass"
keytool -genkeypair -keystore "$KEYS/rsa.p12" -storetype PKCS12 -storepass:file "$KEYS/pass" -keypass:file "$KEYS/pass" \
    -alias rel -keyalg RSA -keysize 2048 -validity 30 -dname "CN=declare test only" >/dev/null 2>&1 || bail "keytool RSA"
keytool -genkeypair -keystore "$KEYS/ec.p12" -storetype PKCS12 -storepass:file "$KEYS/pass" -keypass:file "$KEYS/pass" \
    -alias rel -keyalg EC -groupname secp256r1 -validity 30 -dname "CN=declare test only EC" >/dev/null 2>&1 || bail "keytool EC"
minisign -G -W -f -p "$KEYS/decl.pub" -s "$KEYS/decl.key" >/dev/null 2>&1 || bail "minisign -G"
minisign -G -W -f -p "$KEYS/other.pub" -s "$KEYS/other.key" >/dev/null 2>&1 || bail "minisign -G (other)"
DECL_PUB_LINE="$(sed -n '2p' "$KEYS/decl.pub")"
[[ -n "$DECL_PUB_LINE" ]] || bail "no public key line"

# ── test APKs ──────────────────────────────────────────────────
mk_apk() { # mk_apk NAME VERSION_NAME VERSION_CODE [PACKAGE]
    local name="$1" vn="$2" vc="$3" pkg="${4:-dev.nuagemagique.pilipos}" d="$T/apk-$1"
    mkdir -p "$d"
    cat >"$d/AndroidManifest.xml" <<XML
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="$pkg">
    <uses-sdk android:minSdkVersion="33" android:targetSdkVersion="35"/>
    <application android:label="declare test"/>
</manifest>
XML
    "$BT/aapt2" link -o "$d/base.apk" --manifest "$d/AndroidManifest.xml" -I "$ANDROID_JAR" \
        --version-code "$vc" --version-name "$vn" >/dev/null 2>&1 || return 1
    "$BT/zipalign" -P 16 -f 4 "$d/base.apk" "$T/$name.apk" >/dev/null 2>&1 || return 1
}
mk_apk ok 1.0.1 10001 || bail "test APK ok"
mk_apk dev 1.0.1-dev 10001 || bail "test APK dev"
mk_apk badcode 1.0.1 20001 || bail "test APK badcode"
mk_apk badpkg 1.0.1 10001 dev.example.other || bail "test APK badpkg"

# A zip whose stored entries are not 4-byte aligned: manifest and badging stay valid.
python3 -I - "$T/ok.apk" "$T/misaligned.apk" <<'PY' || bail "misaligned APK"
import sys, zipfile
src = zipfile.ZipFile(sys.argv[1])
out = zipfile.ZipFile(sys.argv[2], "w")
out.writestr(zipfile.ZipInfo("a"), b"x", zipfile.ZIP_STORED)
for n in src.namelist():
    zi = zipfile.ZipInfo(n)
    zi.compress_type = zipfile.ZIP_STORED
    out.writestr(zi, src.read(n))
out.close()
PY
if "$BT/zipalign" -c -P 16 4 "$T/misaligned.apk" >/dev/null 2>&1; then bail "the misaligned APK is aligned"; fi

# The SHA-1 the SPF would accept: the signed APK, signed here with the same flags.
signed_sha1() { # signed_sha1 APK -> SHA-1 of the signed copy
    "$BT/apksigner" sign --ks "$KEYS/rsa.p12" --ks-pass "file:$KEYS/pass" --ks-key-alias rel --min-sdk-version 33 \
        --v1-signing-enabled false --v2-signing-enabled true --v3-signing-enabled true --v4-signing-enabled false \
        --out "$T/ref-signed.apk" "$1" >/dev/null 2>&1 || return 1
    shasum -a 1 "$T/ref-signed.apk" | awk '{print $1}'
}
SHA1_OK="$(signed_sha1 "$T/ok.apk")" || bail "reference signing (ok)"
SHA1_DEV="$(signed_sha1 "$T/dev.apk")" || bail "reference signing (dev)"
SHA1_WRONG="0000000000000000000000000000000000000000"
CERT_SHA256="$(keytool -list -v -keystore "$KEYS/rsa.p12" -storepass:file "$KEYS/pass" -alias rel 2>/dev/null \
    | sed -n 's/^[[:space:]]*SHA256: //p' | head -n 1 | tr -d ':' | tr 'A-F' 'a-f')"
[[ "$CERT_SHA256" =~ ^[0-9a-f]{64}$ ]] || bail "keytool certificate SHA-256"

# ── a scratch project: script, release key xml, local tag ──────
PROJ="$T/proj"
mkdir -p "$PROJ/scripts/release" "$PROJ/android/app/src/main/res/xml"
cp "$SCRIPT_SRC" "$PROJ/scripts/release/declare_android.sh"
write_xml() { # write_xml RELEASE_KEY
    cat >"$PROJ/android/app/src/main/res/xml/plat_update_keys.xml" <<XML
<?xml version="1.0" encoding="utf-8"?>
<plat-update-keys>
    <key build="debug">RWQmv3D5F4695q2BBUUzfk0ABpONbcMKPePaMqZkpr5UV8J0PCTEdO/v</key>
    <key build="release">$1</key>
</plat-update-keys>
XML
}
write_xml "$DECL_PUB_LINE"
(
    cd "$PROJ" || exit 1
    git init -q . &&
        git add -A &&
        git -c user.name=t -c user.email=t@example.invalid -c commit.gpgsign=false commit -q -m "scratch" &&
        git tag android-v1.0.1
) || bail "scratch git repository"
DECLARE="$PROJ/scripts/release/declare_android.sh"

# ── stub oras and gh ───────────────────────────────────────────
STUBS="$T/bin"
mkdir -p "$STUBS"
STUBLOG="$T/stub.log"
FORBID="$T/stub.forbid"
for tool in oras gh; do
    cat >"$STUBS/$tool" <<STUB
#!/usr/bin/env bash
printf '%s %s\n' "$tool" "\$*" >>"$STUBLOG"
if [[ -e "$FORBID" ]]; then echo "stub $tool called while forbidden" >&2; exit 99; fi
if [[ "$tool" == gh && "\$1" == auth ]]; then echo "stub-token"; fi
if [[ "$tool" == gh && "\$1" == api ]]; then echo "stub-user"; fi
if [[ "$tool" == oras && "\$1" == login ]]; then cat >/dev/null; fi
exit 0
STUB
    chmod +x "$STUBS/$tool"
done
: >"$STUBLOG"

# ── runner ─────────────────────────────────────────────────────
OUTTXT="$T/run.out"
RC=0
run() { # run ARGS... : output in $OUTTXT, exit code in $RC; keystore, key and password defaults
    PATH="$STUBS:$PATH" PILI_KS_PASS_FILE="$KEYS/pass" bash "$DECLARE" "$@" >"$OUTTXT" 2>&1
    RC=$?
}
has() { grep -qF -- "$1" "$OUTTXT"; }

# 1. a dry run passes and writes the three files
touch "$FORBID"
OUT1="$T/out1"
PILI_DECLARED_PUB="$KEYS/decl.pub" run --apk "$T/ok.apk" --version 1.0.1 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$OUT1" --dry-run
ok=1
[[ "$RC" -eq 0 ]] && has "DECLARE | total | PASS" && ok=0
expect "dry-run-pass" "$ok" "exit $RC: $(tail -n 3 "$OUTTXT" | tr '\n' ' ')"
for f in pili-pos-android-1.0.1.apk declared.json declared.json.minisig; do
    ok=1
    [[ -s "$OUT1/$f" ]] && ok=0
    expect "dry-run-file-$f" "$ok" "$OUT1/$f is missing"
done
ok=1
[[ "$(ls "$OUT1" | wc -l | tr -d ' ')" -eq 3 ]] && ok=0
expect "dry-run-only-three-files" "$ok" "unexpected files in $OUT1"

# 2. declared.json against the PD211 schema and the real signer, hashes and signature
python3 -I - "$OUT1" "$SHA1_OK" "$CERT_SHA256" <<'PY'
import hashlib, json, re, sys
out, sha1_want, cert = sys.argv[1], sys.argv[2], sys.argv[3]
raw = open(out + "/declared.json", "rb").read()
assert raw.endswith(b"\n") and not raw.endswith(b"\n\n"), "one trailing newline"
assert len(raw) <= 64 * 1024
d = json.loads(raw.decode("utf-8"))
keys = ["format", "platform", "package", "version", "versionCode", "asset", "sha1", "sha256", "signerSha256", "declared_at"]
assert list(d.keys()) == keys, list(d.keys())
assert d["format"] == 1 and type(d["format"]) is int
assert d["platform"] == "android"
assert d["package"] == "dev.nuagemagique.pilipos"
assert re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", d["version"]) and d["version"] == "1.0.1"
assert type(d["versionCode"]) is int and d["versionCode"] == 10001
assert re.fullmatch(r"pili-pos-android-[0-9]+\.[0-9]+\.[0-9]+\.apk", d["asset"]) and d["asset"] == "pili-pos-android-1.0.1.apk"
assert re.fullmatch(r"[0-9a-f]{40}", d["sha1"]) and d["sha1"] == sha1_want
assert re.fullmatch(r"[0-9a-f]{64}", d["sha256"])
assert re.fullmatch(r"[0-9a-f]{64}", d["signerSha256"]) and d["signerSha256"] == cert
assert re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z", d["declared_at"])
apk = open(out + "/" + d["asset"], "rb").read()
assert hashlib.sha1(apk).hexdigest() == d["sha1"]
assert hashlib.sha256(apk).hexdigest() == d["sha256"]
sig = open(out + "/declared.json.minisig", "r", encoding="utf-8").read()
assert sig.splitlines()[0].startswith("untrusted comment:")
tc = [l for l in sig.splitlines() if l.startswith("trusted comment: ")]
assert tc and tc[0] == "trusted comment: pili-pos-android 1.0.1 declared " + d["declared_at"], tc
PY
expect "declared-json-schema" "$?" "declared.json does not match the PD211 schema or the files"
minisign -V -q -p "$KEYS/decl.pub" -m "$OUT1/declared.json" -x "$OUT1/declared.json.minisig" >/dev/null 2>&1
expect "minisign-verify-throwaway-key" "$?" "minisign -V rejects the signature"
ok=1
minisign -V -q -p "$KEYS/other.pub" -m "$OUT1/declared.json" -x "$OUT1/declared.json.minisig" >/dev/null 2>&1 || ok=0
expect "minisign-verify-other-key-refused" "$ok" "a different key accepts the signature"
# the signed asset really carries the throwaway certificate, v3 only
vf="$("$BT/apksigner" verify --verbose --print-certs "$OUT1/pili-pos-android-1.0.1.apk" 2>&1)"
ok=1
printf '%s\n' "$vf" | grep -q "Verified using v3 scheme (APK Signature Scheme v3): true" \
    && printf '%s\n' "$vf" | grep -q "Verified using v1 scheme (JAR signing): false" \
    && printf '%s\n' "$vf" | grep -q "Signer #1 certificate SHA-256 digest: $CERT_SHA256" && ok=0
expect "asset-signature-v1-off-v3-on" "$ok" "apksigner verify: $(printf '%s' "$vf" | head -n 3 | tr '\n' ' ')"
# the printed commands
PILI_DECLARED_PUB="$KEYS/decl.pub" run --apk "$T/ok.apk" --version 1.0.1 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$T/out1b" --dry-run
ok=1
has "oras push ghcr.io/brunomoyse/moyun-pos:declared-android --artifact-type application/vnd.pili-pos.declared-android.v1 pili-pos-android-1.0.1.apk:application/vnd.android.package-archive declared.json:application/json declared.json.minisig:text/plain" \
    && has "gh release upload android-v1.0.1 -R brunomoyse/moyun-pos --clobber" && ok=0
expect "dry-run-prints-commands" "$ok" "the exact oras and gh commands are not printed"

# 3. the stubs were never called in the dry runs
ok=1
[[ ! -s "$STUBLOG" ]] && ok=0
expect "dry-run-never-calls-oras-or-gh" "$ok" "stub log: $(cat "$STUBLOG")"

# 4. a wrong SHA-1 fails and writes nothing
OUT4="$T/out4"
PILI_DECLARED_PUB="$KEYS/decl.pub" run --apk "$T/ok.apk" --version 1.0.1 --sha1 "$SHA1_WRONG" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$OUT4" --dry-run
ok=1
[[ "$RC" -eq 1 ]] && has "DECLARE | sha1 | FAIL" && has "DECLARE | total | FAIL" && [[ ! -e "$OUT4" ]] && ok=0
expect "wrong-sha1-refused" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"

# 5. an EC key is refused
PILI_DECLARED_PUB="$KEYS/decl.pub" run --apk "$T/ok.apk" --version 1.0.1 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/ec.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$T/out5" --dry-run
ok=1
[[ "$RC" -eq 1 ]] && has "DECLARE | key | FAIL" && has "EC" && [[ ! -e "$T/out5" ]] && ok=0
expect "ec-key-refused" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"

# 6. -dev versions: refused without --allow-dev, accepted with it (dry-run only)
PILI_DECLARED_PUB="$KEYS/decl.pub" run --apk "$T/dev.apk" --version 1.0.1 --sha1 "$SHA1_DEV" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$T/out6a" --dry-run
ok=1
[[ "$RC" -eq 1 ]] && has "DECLARE | badging | FAIL" && has "-dev" && [[ ! -e "$T/out6a" ]] && ok=0
expect "dev-apk-refused-without-allow-dev" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"
PILI_DECLARED_PUB="$KEYS/decl.pub" run --apk "$T/dev.apk" --version 1.0.1-dev --sha1 "$SHA1_DEV" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$T/out6b" --dry-run
ok=1
[[ "$RC" -eq 1 ]] && has "DECLARE | version | FAIL" && [[ ! -e "$T/out6b" ]] && ok=0
expect "dev-version-refused-without-allow-dev" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"
PILI_DECLARED_PUB="$KEYS/decl.pub" run --apk "$T/dev.apk" --version 1.0.1 --sha1 "$SHA1_DEV" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$T/out6c" --dry-run --allow-dev
ok=1
[[ "$RC" -eq 0 ]] && has "DECLARE | total | PASS" && grep -q '"version":"1.0.1"' "$T/out6c/declared.json" && ok=0
expect "dev-apk-accepted-with-allow-dev-dry-run" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"

# 7. --allow-dev without --dry-run is a usage error
run --apk "$T/dev.apk" --version 1.0.1 --sha1 "$SHA1_DEV" --keystore "$KEYS/rsa.p12" --alias rel \
    --minisign-key "$KEYS/decl.key" --allow-dev
ok=1
[[ "$RC" -eq 2 ]] && has "--allow-dev" && ok=0
expect "allow-dev-needs-dry-run" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"
# other usage errors
run --apk "$T/ok.apk" --version 1.0.1 --keystore "$KEYS/rsa.p12" --alias rel
ok=1
[[ "$RC" -eq 2 ]] && ok=0
expect "missing-sha1-is-usage-error" "$ok" "exit $RC"
run --apk "$T/ok.apk" --version 1.0.1 --sha1 "$SHA1_OK" --keystore "$KEYS/rsa.p12" --alias rel \
    --minisign-key "$KEYS/decl.key" --bogus
ok=1
[[ "$RC" -eq 2 ]] && ok=0
expect "unknown-option-is-usage-error" "$ok" "exit $RC"
PILI_DECLARED_PUB="$KEYS/decl.pub" run --apk "$T/ok.apk" --version 1.0.1 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key"
ok=1
[[ "$RC" -eq 2 ]] && has "PILI_DECLARED_PUB" && ok=0
expect "declared-pub-override-needs-dry-run" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"

# 8. other refusals: package, versionCode, version string, alignment, key location, wrong key
PILI_DECLARED_PUB="$KEYS/decl.pub" run --apk "$T/badpkg.apk" --version 1.0.1 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$T/out8a" --dry-run
ok=1
[[ "$RC" -eq 1 ]] && has "DECLARE | badging | FAIL" && has "package" && ok=0
expect "wrong-package-refused" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"
PILI_DECLARED_PUB="$KEYS/decl.pub" run --apk "$T/badcode.apk" --version 1.0.1 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$T/out8b" --dry-run
ok=1
[[ "$RC" -eq 1 ]] && has "DECLARE | badging | FAIL" && has "versionCode" && ok=0
expect "wrong-versioncode-refused" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"
PILI_DECLARED_PUB="$KEYS/decl.pub" run --apk "$T/ok.apk" --version 1.0 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$T/out8c" --dry-run
ok=1
[[ "$RC" -eq 1 ]] && has "DECLARE | version | FAIL" && ok=0
expect "malformed-version-refused" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"
PILI_DECLARED_PUB="$KEYS/decl.pub" run --apk "$T/ok.apk" --version 1.0.2 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$T/out8d" --dry-run
ok=1
[[ "$RC" -eq 1 ]] && has "DECLARE | badging | FAIL" && ok=0
expect "version-differs-from-apk-refused" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"
PILI_DECLARED_PUB="$KEYS/decl.pub" run --apk "$T/misaligned.apk" --version 1.0.1 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$T/out8e" --dry-run
ok=1
[[ "$RC" -eq 1 ]] && has "DECLARE | zipalign | FAIL" && [[ ! -e "$T/out8e" ]] && ok=0
expect "unaligned-apk-refused" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"
cp "$KEYS/rsa.p12" "$PROJ/inside.p12"
PILI_DECLARED_PUB="$KEYS/decl.pub" run --apk "$T/ok.apk" --version 1.0.1 --sha1 "$SHA1_OK" \
    --keystore "$PROJ/inside.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$T/out8f" --dry-run
ok=1
[[ "$RC" -eq 1 ]] && has "git work tree" && ok=0
expect "keystore-inside-git-tree-refused" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"
rm -f "$PROJ/inside.p12"
PILI_DECLARED_PUB="$KEYS/other.pub" run --apk "$T/ok.apk" --version 1.0.1 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$T/out8g" --dry-run
ok=1
[[ "$RC" -eq 1 ]] && has "DECLARE | minisign-verify | FAIL" && ok=0
expect "signature-not-verifying-with-wrong-pub-refused" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"
write_xml ""
run --apk "$T/ok.apk" --version 1.0.1 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$T/out8h" --dry-run
ok=1
[[ "$RC" -eq 1 ]] && has "DECLARE | minisign-verify | FAIL" && has "empty" && ok=0
expect "empty-release-key-refused" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"
write_xml "$DECL_PUB_LINE"
run --apk "$T/ok.apk" --version 1.0.1 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key" --out "$T/out8i" --dry-run
ok=1
[[ "$RC" -eq 0 ]] && has "from plat_update_keys.xml" && ok=0
expect "default-key-is-the-release-key-of-the-xml" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"
ok=1
[[ ! -s "$STUBLOG" ]] && ok=0
expect "stubs-still-uncalled-after-refusals" "$ok" "stub log: $(cat "$STUBLOG")"

# 9. a real (stubbed) run: the tag must exist, then oras and gh are called as documented
rm -f "$FORBID"
git -C "$PROJ" tag -d android-v1.0.1 >/dev/null 2>&1
run --apk "$T/ok.apk" --version 1.0.1 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key"
ok=1
[[ "$RC" -eq 1 ]] && has "DECLARE | version | FAIL" && has "android-v1.0.1" && [[ ! -s "$STUBLOG" ]] && ok=0
expect "real-run-needs-local-tag" "$ok" "exit $RC: $(tail -n 2 "$OUTTXT" | tr '\n' ' ')"
git -C "$PROJ" tag android-v1.0.1
: >"$STUBLOG"
run --apk "$T/ok.apk" --version 1.0.1 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key"
ok=1
[[ "$RC" -eq 0 ]] && has "DECLARE | total | PASS" && ok=0
expect "real-run-with-stubs-passes" "$ok" "exit $RC: $(tail -n 3 "$OUTTXT" | tr '\n' ' ')"
ok=1
grep -qF "oras push ghcr.io/brunomoyse/moyun-pos:declared-android --artifact-type application/vnd.pili-pos.declared-android.v1 pili-pos-android-1.0.1.apk:application/vnd.android.package-archive declared.json:application/json declared.json.minisig:text/plain" "$STUBLOG" && ok=0
expect "real-run-oras-push-arguments" "$ok" "stub log: $(cat "$STUBLOG" | tr '\n' ';')"
ok=1
grep -qF "gh release upload android-v1.0.1 -R brunomoyse/moyun-pos --clobber pili-pos-android-1.0.1.apk declared.json declared.json.minisig" "$STUBLOG" && ok=0
expect "real-run-gh-release-upload-arguments" "$ok" "stub log: $(cat "$STUBLOG" | tr '\n' ';')"
: >"$STUBLOG"
PILI_REPO="Example/Repo-Name" run --apk "$T/ok.apk" --version 1.0.1 --sha1 "$SHA1_OK" \
    --keystore "$KEYS/rsa.p12" --alias rel --minisign-key "$KEYS/decl.key"
ok=1
grep -qF "oras push ghcr.io/example/repo-name:declared-android " "$STUBLOG" && grep -qF -- "-R Example/Repo-Name" "$STUBLOG" && ok=0
expect "real-run-package-name-lowercased" "$ok" "stub log: $(cat "$STUBLOG" | tr '\n' ';')"

# 10. nothing of the keys or the work directory leaked into the repository under test
ok=1
[[ -z "$(git -C "$PROJ" status --porcelain)" ]] && ok=0
expect "scratch-repo-clean" "$ok" "the scripts wrote into the scratch repository"

if [[ "$failed" -eq 0 ]]; then
    printf 'TESTDECLARE | total | PASS | %d cases\n' "$total"
    exit 0
fi
printf 'TESTDECLARE | total | FAIL | %d of %d cases failed\n' "$failed" "$total"
exit 1
