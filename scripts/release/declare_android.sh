#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - declare a released ANDROID version (PD216)
#
# FOR THE PRODUCER ONLY: this runs on the owner's Mac, with the OFFLINE signing key and the
# OFFLINE declaration key. The development chain must NEVER run it; it only ever runs it with
# --dry-run, against throwaway keys in mktemp directories and with stub oras and gh
# (scripts/release/test_declare_android.sh).
#
# Once the SPF has accepted the SHA-1 of the SIGNED APK, this publishes what the tills trust:
#   declared.json          { format, platform, package, version, versionCode, asset, sha1,
#                            sha256, signerSha256, declared_at }   (PD211, PLATFORM.md 8.1)
#   declared.json.minisig  prehashed minisign signature with the declaration key
# and the signed APK, pushed together as tag "declared-android" of the public package
# ghcr.io/brunomoyse/moyun-pos (the layer title of each file is its name, PLATFORM.md 8.3) and
# attached to the GitHub release android-vX.Y.Z.
#
# Usage:
#   scripts/release/declare_android.sh --apk UNSIGNED_APK --version X.Y.Z --sha1 SHA1 \
#       --keystore PATH --alias ALIAS [--minisign-key PATH] [--out DIR] [--dry-run] [--allow-dev]
#
#   --apk          the reproducible, zipaligned, UNSIGNED release APK (app-release-unsigned.apk)
#   --version      X.Y.Z, equal to the versionName of the APK
#   --sha1         the SHA-1 (40 lowercase hex) of the SIGNED APK that the SPF accepted
#   --keystore     the release keystore (RSA, 2048 bits or more; an EC or DSA key is refused)
#   --alias        the key alias in the keystore
#   --minisign-key the declaration secret key (default $PILI_MINISIGN_KEY, else
#                  ~/.minisign/pili-pos-declared.key); minisign asks for its password
#   --out DIR      where the three files are copied (default with --dry-run: a new directory
#                  under $TMPDIR); refused inside a git work tree; a real run publishes from
#                  a temporary directory
#   --dry-run      check, sign, write the files and print the commands, push nothing
#   --allow-dev    accept an X.Y.Z-dev versionName; only together with --dry-run
#
# Environment:
#   PILI_KS_PASS_FILE   file holding the keystore password (required)
#   PILI_REPO           GitHub repository (default brunomoyse/moyun-pos)
#   PILI_DECLARED_PUB   public key that verifies the signature, a file or the base64 key line
#                       (default: the release key of android/app/src/main/res/xml/plat_update_keys.xml;
#                       an override is accepted only with --dry-run)
#   PILI_MINISIGN_KEY   see --minisign-key
#
# Needs: apksigner, zipalign, aapt2 (build-tools 35.0.0 under $ANDROID_HOME), keytool, minisign,
# python3 for the key file; oras and gh (logged in, scope write:packages) only without --dry-run.
# The v2 scheme is requested but apksigner folds it into v3 at minSdk 33 (S5): v1 off, v3 on.
# The keystore and the minisign key must lie outside any git work tree.
#
# Every step prints "DECLARE | <step> | PASS|FAIL | <detail>", the last is "DECLARE | total | ...".
# Exit: 0 declared (or dry-run complete), 1 a check failed, 2 usage error.
# ─────────────────────────────────────────────────────────────────
set -euo pipefail
export LC_ALL=C

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
KEYS_XML="$PROJECT_ROOT/android/app/src/main/res/xml/plat_update_keys.xml"
PACKAGE_ID="dev.nuagemagique.pilipos"
BUILD_TOOLS="35.0.0"

usage() {
    sed -n '2,/^# ──*$/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
}
usage_error() {
    echo "declare_android: $*" >&2
    echo "usage: declare_android.sh --apk UNSIGNED_APK --version X.Y.Z --sha1 SHA1 --keystore PATH --alias ALIAS [--minisign-key PATH] [--out DIR] [--dry-run] [--allow-dev]" >&2
    exit 2
}

APK="" VERSION="" SHA1_WANT="" KEYSTORE="" ALIAS="" MINISIGN_KEY="${PILI_MINISIGN_KEY:-$HOME/.minisign/pili-pos-declared.key}"
OUT="" DRY=0 ALLOW_DEV=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        -h | --help) usage; exit 0 ;;
        --apk) [[ $# -ge 2 ]] || usage_error "--apk needs a value"; APK="$2"; shift 2 ;;
        --version) [[ $# -ge 2 ]] || usage_error "--version needs a value"; VERSION="$2"; shift 2 ;;
        --sha1) [[ $# -ge 2 ]] || usage_error "--sha1 needs a value"; SHA1_WANT="$2"; shift 2 ;;
        --keystore) [[ $# -ge 2 ]] || usage_error "--keystore needs a value"; KEYSTORE="$2"; shift 2 ;;
        --alias) [[ $# -ge 2 ]] || usage_error "--alias needs a value"; ALIAS="$2"; shift 2 ;;
        --minisign-key) [[ $# -ge 2 ]] || usage_error "--minisign-key needs a value"; MINISIGN_KEY="$2"; shift 2 ;;
        --out) [[ $# -ge 2 ]] || usage_error "--out needs a value"; OUT="$2"; shift 2 ;;
        --dry-run) DRY=1; shift ;;
        --allow-dev) ALLOW_DEV=1; shift ;;
        *) usage_error "unknown option: $1" ;;
    esac
done
[[ -n "$APK" ]] || usage_error "--apk is required"
[[ -n "$VERSION" ]] || usage_error "--version is required"
[[ -n "$SHA1_WANT" ]] || usage_error "--sha1 is required"
[[ -n "$KEYSTORE" ]] || usage_error "--keystore is required"
[[ -n "$ALIAS" ]] || usage_error "--alias is required"
[[ "$ALLOW_DEV" -eq 0 || "$DRY" -eq 1 ]] || usage_error "--allow-dev is accepted only together with --dry-run"
[[ "$SHA1_WANT" =~ ^[0-9a-f]{40}$ ]] || usage_error "--sha1 must be 40 lowercase hex characters"
[[ -n "${PILI_KS_PASS_FILE:-}" ]] || usage_error "PILI_KS_PASS_FILE (the keystore password file) is required"
[[ -f "$PILI_KS_PASS_FILE" ]] || usage_error "no such password file: $PILI_KS_PASS_FILE"
[[ -f "$APK" ]] || usage_error "no such APK: $APK"
[[ -f "$KEYSTORE" ]] || usage_error "no such keystore: $KEYSTORE"
[[ -f "$MINISIGN_KEY" ]] || usage_error "no such minisign key: $MINISIGN_KEY"
if [[ -n "${PILI_DECLARED_PUB:-}" && "$DRY" -eq 0 ]]; then
    usage_error "PILI_DECLARED_PUB is accepted only with --dry-run (a real run verifies with the release key of plat_update_keys.xml)"
fi

REPO="${PILI_REPO:-brunomoyse/moyun-pos}"
PACKAGE="ghcr.io/$(echo "$REPO" | tr '[:upper:]' '[:lower:]')"
ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
BT="$ANDROID_HOME/build-tools/$BUILD_TOOLS"
APKSIGNER="$BT/apksigner"
ZIPALIGN="$BT/zipalign"
AAPT2="$BT/aapt2"
KEYTOOL="$(command -v keytool || true)"
[[ -z "${JAVA_HOME:-}" || ! -x "$JAVA_HOME/bin/keytool" ]] || KEYTOOL="$JAVA_HOME/bin/keytool"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

step() { # step NAME PASS|FAIL DETAIL
    printf 'DECLARE | %s | %s | %s\n' "$1" "$2" "$3"
}
die() { # die STEP DETAIL: a failed check ends the run
    step "$1" FAIL "$2"
    step total FAIL "$1: $2"
    exit 1
}
in_git_tree() { # in_git_tree FILE
    local d
    d="$(cd "$(dirname "$1")" && pwd)"
    git -C "$d" rev-parse --is-inside-work-tree >/dev/null 2>&1
}

# ── a. tools ───────────────────────────────────────────────────
missing=""
for t in "$APKSIGNER" "$ZIPALIGN" "$AAPT2"; do
    [[ -x "$t" ]] || missing+=" $t"
done
[[ -n "$KEYTOOL" ]] || missing+=" keytool"
command -v minisign >/dev/null 2>&1 || missing+=" minisign"
command -v python3 >/dev/null 2>&1 || missing+=" python3"
if [[ "$DRY" -eq 0 ]]; then
    command -v oras >/dev/null 2>&1 || missing+=" oras"
    command -v gh >/dev/null 2>&1 || missing+=" gh"
fi
[[ -z "$missing" ]] || die tools "missing:$missing"
# Key material must never lie inside a repository.
if in_git_tree "$KEYSTORE"; then die tools "the keystore lies inside a git work tree: $KEYSTORE"; fi
if in_git_tree "$MINISIGN_KEY"; then die tools "the minisign key lies inside a git work tree: $MINISIGN_KEY"; fi
if [[ "$DRY" -eq 1 ]]; then
    step tools PASS "apksigner, zipalign, aapt2 ($BUILD_TOOLS), keytool, minisign; oras and gh not needed with --dry-run"
else
    step tools PASS "apksigner, zipalign, aapt2 ($BUILD_TOOLS), keytool, minisign, oras, gh"
fi


# ── b. version and tag ─────────────────────────────────────────
if [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+-dev$ ]]; then
    [[ "$ALLOW_DEV" -eq 1 ]] || die version "$VERSION is a -dev version; refused (only --allow-dev together with --dry-run)"
    BASE="${VERSION%-dev}"
elif [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    BASE="$VERSION"
else
    die version "'$VERSION' is not X.Y.Z"
fi
IFS=. read -r MAJOR MINOR PATCH <<<"$BASE"
if [[ "$MAJOR" -gt 214747 || "$MINOR" -ge 100 || "$PATCH" -ge 100 ]]; then
    die version "$BASE is outside the versionCode scheme (MINOR and PATCH below 100)"
fi
WANT_CODE=$((10#$MAJOR * 10000 + 10#$MINOR * 100 + 10#$PATCH))
TAG="android-v$BASE"
if [[ "$DRY" -eq 1 ]]; then
    step version PASS "$VERSION, versionCode $WANT_CODE, tag $TAG not required with --dry-run"
else
    git -C "$PROJECT_ROOT" rev-parse -q --verify "refs/tags/$TAG^{commit}" >/dev/null 2>&1 \
        || die version "the local tag $TAG does not exist (tag the commit that check_repro.sh checked)"
    step version PASS "$VERSION, versionCode $WANT_CODE, local tag $TAG at $(git -C "$PROJECT_ROOT" rev-parse --short "$TAG^{commit}")"
fi

# ── c. badging ─────────────────────────────────────────────────
badging="$("$AAPT2" dump badging "$APK" 2>&1)" || die badging "aapt2 dump badging failed: $(printf '%s' "$badging" | head -n 1)"
pkg_line="$(printf '%s\n' "$badging" | grep -E "^package: " | head -n 1 || true)"
got_pkg="$(printf '%s' "$pkg_line" | sed -n "s/^package: name='\([^']*\)'.*/\1/p")"
got_code="$(printf '%s' "$pkg_line" | sed -n "s/.* versionCode='\([^']*\)'.*/\1/p")"
got_name="$(printf '%s' "$pkg_line" | sed -n "s/.* versionName='\([^']*\)'.*/\1/p")"
[[ "$got_pkg" == "$PACKAGE_ID" ]] || die badging "package is '$got_pkg', expected $PACKAGE_ID"
ok_name=0
if [[ "$got_name" == "$VERSION" ]]; then
    ok_name=1
elif [[ "$ALLOW_DEV" -eq 1 && ( "$got_name" == "$BASE" || "$got_name" == "$BASE-dev" ) ]]; then
    ok_name=1
fi
if [[ "$ok_name" -ne 1 ]]; then
    if [[ "$got_name" == *-dev ]]; then
        die badging "versionName '$got_name' is a -dev build; refused without --allow-dev (and --dry-run)"
    fi
    die badging "versionName is '$got_name', expected '$VERSION'"
fi
[[ "$got_code" == "$WANT_CODE" ]] || die badging "versionCode is '$got_code', expected $WANT_CODE (MAJOR*10000+MINOR*100+PATCH)"
step badging PASS "$got_pkg versionName $got_name versionCode $got_code"

# ── d. alignment ───────────────────────────────────────────────
if ! za="$("$ZIPALIGN" -c -P 16 -v 4 "$APK" 2>&1)"; then
    die zipalign "the unsigned APK is not aligned (zipalign -c -P 16 -v 4): $(printf '%s' "$za" | grep -E 'BAD|FAILED' | head -n 2 | tr '\n' ' ')"
fi
step zipalign PASS "zipalign -c -P 16 -v 4 passes"

# ── e. key ─────────────────────────────────────────────────────
if ! kt="$("$KEYTOOL" -list -v -keystore "$KEYSTORE" -storepass:file "$PILI_KS_PASS_FILE" -alias "$ALIAS" 2>&1)"; then
    die key "keytool could not read alias '$ALIAS': $(printf '%s' "$kt" | head -n 1)"
fi
key_line="$(printf '%s\n' "$kt" | grep -E "Subject Public Key Algorithm:" | head -n 1 || true)"
if [[ "$key_line" =~ ([0-9]+)-bit[[:space:]]+([A-Za-z]+) ]]; then
    key_bits="${BASH_REMATCH[1]}"
    key_alg="${BASH_REMATCH[2]}"
else
    die key "cannot read the key algorithm from keytool: '$key_line'"
fi
[[ "$key_alg" == "RSA" ]] || die key "the key is $key_alg; only RSA is accepted (an EC or DSA signature is not deterministic, DECISIONS 9)"
[[ "$key_bits" -ge 2048 ]] || die key "the RSA key has $key_bits bits; 2048 or more is required"
step key PASS "RSA $key_bits bits, alias $ALIAS"

# ── f. sign a copy ─────────────────────────────────────────────
cp "$APK" "$WORK/unsigned.apk"
SIGNED="$WORK/signed.apk"
if ! sg="$("$APKSIGNER" sign --ks "$KEYSTORE" --ks-pass "file:$PILI_KS_PASS_FILE" --ks-key-alias "$ALIAS" \
    --min-sdk-version 33 --v1-signing-enabled false --v2-signing-enabled true --v3-signing-enabled true \
    --v4-signing-enabled false --out "$SIGNED" "$WORK/unsigned.apk" 2>&1)"; then
    die sign "apksigner sign failed: $(printf '%s' "$sg" | head -n 1)"
fi
rm -f "$SIGNED.idsig"
if ! vf="$("$APKSIGNER" verify --verbose --print-certs "$SIGNED" 2>&1)"; then
    die sign "apksigner verify failed: $(printf '%s' "$vf" | head -n 1)"
fi
printf '%s\n' "$vf" | grep -q "Verified using v1 scheme (JAR signing): false" || die sign "a v1 signature is present"
printf '%s\n' "$vf" | grep -q "Verified using v3 scheme (APK Signature Scheme v3): true" || die sign "no verified v3 signature"
printf '%s\n' "$vf" | grep -q "Number of signers: 1" || die sign "expected exactly one signer"
SIGNER_SHA256="$(printf '%s\n' "$vf" | sed -n 's/^Signer #1 certificate SHA-256 digest: //p' | head -n 1)"
[[ "$SIGNER_SHA256" =~ ^[0-9a-f]{64}$ ]] || die sign "cannot read the signer SHA-256 from apksigner verify"
step sign PASS "signed a copy (v1 off, v2 and v3 requested, minSdk 33), signer SHA-256 $SIGNER_SHA256"

# ── g. hashes ──────────────────────────────────────────────────
SHA1_GOT="$(shasum -a 1 "$SIGNED" | awk '{print $1}')"
SHA256_GOT="$(shasum -a 256 "$SIGNED" | awk '{print $1}')"
if [[ "$SHA1_GOT" != "$SHA1_WANT" ]]; then
    die sha1 "the signed APK has SHA-1 $SHA1_GOT, not the declared $SHA1_WANT; nothing was written"
fi
step sha1 PASS "SHA-1 $SHA1_GOT matches the value the SPF accepted; SHA-256 $SHA256_GOT"

# ── h. descriptor, signature ───────────────────────────────────
ASSET="pili-pos-android-$BASE.apk"
DECLARED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
DIR="$WORK/files"
mkdir "$DIR"
cp "$SIGNED" "$DIR/$ASSET"
printf '{"format":1,"platform":"android","package":"%s","version":"%s","versionCode":%s,"asset":"%s","sha1":"%s","sha256":"%s","signerSha256":"%s","declared_at":"%s"}\n' \
    "$PACKAGE_ID" "$BASE" "$WANT_CODE" "$ASSET" "$SHA1_GOT" "$SHA256_GOT" "$SIGNER_SHA256" "$DECLARED_AT" >"$DIR/declared.json"
step descriptor PASS "declared.json for $BASE, versionCode $WANT_CODE, asset $ASSET"

# The public key that must verify the signature.
PUB_ARGS=()
if [[ -n "${PILI_DECLARED_PUB:-}" ]]; then
    if [[ -f "$PILI_DECLARED_PUB" ]]; then
        PUB_ARGS=(-p "$PILI_DECLARED_PUB")
    else
        PUB_ARGS=(-P "$PILI_DECLARED_PUB")
    fi
    PUB_SRC="PILI_DECLARED_PUB"
else
    [[ -f "$KEYS_XML" ]] || die minisign-verify "no $KEYS_XML to read the release key from"
    rel="$(python3 -I - "$KEYS_XML" <<'PY'
import sys
import xml.etree.ElementTree as ET
for el in ET.parse(sys.argv[1]).getroot().iter("key"):
    if el.get("build") == "release":
        print((el.text or el.get("key") or "").strip())
        break
PY
)" || rel=""
    [[ -n "$rel" ]] || die minisign-verify "the release key of plat_update_keys.xml is empty: create the declaration key first (DECISIONS 9, Q7)"
    PUB_ARGS=(-P "$rel")
    PUB_SRC="plat_update_keys.xml"
fi

# Not captured: minisign prints its "Password:" prompt on stderr and must reach the terminal.
if ! minisign -S -s "$MINISIGN_KEY" -m "$DIR/declared.json" -x "$DIR/declared.json.minisig" \
    -t "pili-pos-android $BASE declared $DECLARED_AT" >/dev/null; then
    die minisign-sign "minisign -S failed (wrong password or unreadable key $MINISIGN_KEY)"
fi
step minisign-sign PASS "declared.json.minisig written (prehashed), trusted comment 'pili-pos-android $BASE declared $DECLARED_AT'"
if ! verify_out="$(minisign -V -q "${PUB_ARGS[@]}" -m "$DIR/declared.json" -x "$DIR/declared.json.minisig" 2>&1)"; then
    die minisign-verify "the signature does not verify with the key from $PUB_SRC (wrong key pair?): $(printf '%s' "$verify_out" | tail -n 1)"
fi
step minisign-verify PASS "minisign -V accepts the signature with the key from $PUB_SRC"

# ── i. publish ─────────────────────────────────────────────────
PUSH_CMD=(oras push "$PACKAGE:declared-android" --artifact-type application/vnd.pili-pos.declared-android.v1
    "$ASSET:application/vnd.android.package-archive" declared.json:application/json declared.json.minisig:text/plain)
UPLOAD_CMD=(gh release upload "$TAG" -R "$REPO" --clobber "$ASSET" declared.json declared.json.minisig)

if [[ "$DRY" -eq 1 && -z "$OUT" ]]; then
    OUT="$(mktemp -d "${TMPDIR:-/tmp}/declared-android-$BASE.XXXXXX")"
fi
if [[ -n "$OUT" ]]; then
    # The signed APK must not land in a work tree as untracked files.
    probe="$OUT"
    while [[ ! -d "$probe" ]]; do probe="$(dirname "$probe")"; done
    if git -C "$probe" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        die out "--out $OUT lies inside a git work tree"
    fi
    mkdir -p "$OUT"
    cp "$DIR/$ASSET" "$DIR/declared.json" "$DIR/declared.json.minisig" "$OUT/"
fi

if [[ "$DRY" -eq 1 ]]; then
    echo "DECLARE | dry-run | INFO | (cd $OUT &&"
    echo "DECLARE | dry-run | INFO |   ${PUSH_CMD[*]} &&"
    echo "DECLARE | dry-run | INFO |   ${UPLOAD_CMD[*]})"
    step push PASS "dry-run: nothing pushed; wrote $ASSET, declared.json, declared.json.minisig into $OUT"
    step total PASS "dry-run for $BASE complete (versionCode $WANT_CODE, SHA-1 $SHA1_GOT)"
    exit 0
fi

gh release view "$TAG" -R "$REPO" >/dev/null 2>&1 \
    || die push "the GitHub release $TAG does not exist in $REPO: create it first (nothing was pushed)"
echo "Logging oras in to ghcr.io with the gh token..."
gh auth token | oras login ghcr.io -u "$(gh api user -q .login)" --password-stdin >/dev/null \
    || die push "oras login to ghcr.io failed"
( cd "$DIR" && "${PUSH_CMD[@]}" ) || die push "oras push to $PACKAGE:declared-android failed"
( cd "$DIR" && "${UPLOAD_CMD[@]}" ) || die push "gh release upload $TAG failed (does the release exist?)"
step push PASS "pushed $PACKAGE:declared-android and uploaded the three files to $TAG"
step total PASS "$BASE declared (versionCode $WANT_CODE, SHA-1 $SHA1_GOT)"
