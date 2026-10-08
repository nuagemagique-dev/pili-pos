#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - sign an Android APK with the DEVELOPMENT key (PD80)
#
# Usage: scripts/android/sign_dev.sh UNSIGNED_APK OUT_APK
#
# Signs a copy of an unsigned release APK so that it installs on the emulator. The key is a
# throwaway RSA-2048 development key: it is created once, if missing, under
#     $HOME/.cache/pili-android/dev-keystore/   (directory mode 700)
#         pili-dev.p12    PKCS12 keystore, alias pili-dev, CN=pili-pos development only
#         pili-dev.pass   random password, mode 600
# It is NEVER the release key (DECISIONS 9), is never committed and is never used for a release.
# This script refuses to run when the keystore directory would lie inside a git work tree.
# Signature schemes: v1 off, v2 and v3 on (minSdk 33). apksigner verify runs afterwards.
#
# Tools: keytool (JAVA_HOME or PATH), $ANDROID_HOME/build-tools/35.0.0/apksigner.
# Exit: 0 signed and verified, 1 failure, 2 usage error or missing tool.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    sed -n '2,/^# ──*$/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
    exit 0
fi
if [[ $# -ne 2 ]]; then
    echo "usage: sign_dev.sh UNSIGNED_APK OUT_APK" >&2
    exit 2
fi
IN="$1"
OUT="$2"
[[ -f "$IN" ]] || { echo "sign_dev: no such file: $IN" >&2; exit 2; }

ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
APKSIGNER="$ANDROID_HOME/build-tools/35.0.0/apksigner"
[[ -x "$APKSIGNER" ]] || { echo "sign_dev: apksigner not found: $APKSIGNER" >&2; exit 2; }
KEYTOOL="$(command -v keytool || true)"
[[ -z "${JAVA_HOME:-}" || ! -x "$JAVA_HOME/bin/keytool" ]] || KEYTOOL="$JAVA_HOME/bin/keytool"
[[ -n "$KEYTOOL" ]] || { echo "sign_dev: keytool not found" >&2; exit 2; }

DIR="$HOME/.cache/pili-android/dev-keystore"
KS="$DIR/pili-dev.p12"
PASS="$DIR/pili-dev.pass"

# The key must never live inside a repository.
if (cd "$HOME/.cache" 2>/dev/null && git rev-parse --is-inside-work-tree >/dev/null 2>&1); then
    echo "sign_dev: $DIR lies inside a git work tree; refusing" >&2
    exit 1
fi

if [[ ! -f "$KS" || ! -f "$PASS" ]]; then
    umask 077
    mkdir -p "$DIR"
    chmod 700 "$DIR"
    rm -f "$KS" "$PASS"
    # 32 random hex characters from the system CSPRNG.
    pw="$(head -c 16 /dev/urandom | od -An -tx1 | tr -d ' \n')"
    printf '%s\n' "$pw" >"$PASS"
    chmod 600 "$PASS"
    if ! "$KEYTOOL" -genkeypair -keystore "$KS" -storetype PKCS12 -storepass:file "$PASS" -keypass:file "$PASS" \
        -alias pili-dev -keyalg RSA -keysize 2048 -validity 3650 -dname "CN=pili-pos development only" >/dev/null 2>&1; then
        rm -f "$KS" "$PASS"
        echo "sign_dev: keytool could not create the development key" >&2
        exit 1
    fi
    chmod 600 "$KS"
    echo "sign_dev: created the development key $KS (RSA 2048, 10 years)" >&2
fi

mkdir -p "$(dirname "$OUT")"
rm -f "$OUT"
if ! "$APKSIGNER" sign --ks "$KS" --ks-pass "file:$PASS" --ks-key-alias pili-dev \
    --min-sdk-version 24 --v1-signing-enabled false --v2-signing-enabled true --v3-signing-enabled true --out "$OUT" "$IN"; then
    echo "sign_dev: apksigner sign failed" >&2
    exit 1
fi
if ! verify="$("$APKSIGNER" verify --verbose "$OUT" 2>&1)"; then
    echo "sign_dev: apksigner verify failed: $verify" >&2
    exit 1
fi
printf '%s\n' "$verify" | grep -E "^Verified using v[0-9.]+ .*scheme.*: true" >&2
echo "sign_dev: signed $OUT with the DEVELOPMENT key (never a release key)"
