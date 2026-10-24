#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - declare a released Linux version (owner only, on the Mac)
#
# Linux kiosk only; the Android till is declared by
# scripts/release/declare_android.sh (tag "declared-android").
# Once SPF Finances has accepted a version (deploy/UPDATE-STRATEGY.md),
# this publishes what terminals trust:
#   declared.json          { format, platform "linux", version, sha1,
#                            sha256, asset, declared_at }
#   declared.json.minisig  minisign signature with the owner's OFFLINE key
# pushed with the release tarball as tag "declared" of the public package
# ghcr.io/nuagemagique-dev/pili-pos, attached to the linux-vX.Y.Z GitHub
# release too, which then turns from prerelease into latest.
#
# pili-bootstrap (deploy/ansible/roles/pos_binary) installs nothing
# else: no valid signature, no SHA-256/SHA-1 match, no install.
#
# Usage:
#   scripts/release/declare.sh --init-key
#       once: creates the key pair. The secret key stays in
#       ~/.minisign/ (password protected, back it up offline); the
#       public key goes to deploy/release/declared.pub - commit it,
#       every image built afterwards embeds it. The same key declares
#       the Android till (declare_android.sh): when the secret key
#       already exists, only its public key is written out (minisign -R).
#   scripts/release/declare.sh linux-vX.Y.Z --sha1 <SHA-1 accepted by SPF>
#
# Environment: PILI_REPO (default nuagemagique-dev/pili-pos),
# PILI_MINISIGN_KEY (default ~/.minisign/pili-pos-declared.key).
# Needs: gh (logged in, with the write:packages scope:
# `gh auth refresh -s write:packages`), oras, minisign.
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
REPO="${PILI_REPO:-nuagemagique-dev/pili-pos}"
PACKAGE="ghcr.io/$(echo "$REPO" | tr '[:upper:]' '[:lower:]')"
SECRET_KEY="${PILI_MINISIGN_KEY:-$HOME/.minisign/pili-pos-declared.key}"
PUBLIC_KEY="$PROJECT_ROOT/deploy/release/declared.pub"

die() { echo "ERROR: $*" >&2; exit 1; }
command -v minisign >/dev/null || die "minisign not found (brew install minisign)"

if [[ "${1:-}" == "--init-key" ]]; then
    if [[ -f "$SECRET_KEY" ]]; then
        # The declaration key already exists (made for the Android till):
        # never a second key, only its public half for the kiosk images.
        [[ -f "$PUBLIC_KEY" ]] && die "$PUBLIC_KEY already exists - nothing to do"
        mkdir -p "$(dirname "$PUBLIC_KEY")"
        minisign -R -s "$SECRET_KEY" -p "$PUBLIC_KEY"
        echo "✓ public key of the existing $SECRET_KEY: $PUBLIC_KEY (commit it; rebuild the images)"
        exit 0
    fi
    mkdir -p "$(dirname "$SECRET_KEY")" "$(dirname "$PUBLIC_KEY")"
    chmod 700 "$(dirname "$SECRET_KEY")"
    minisign -G -p "$PUBLIC_KEY" -s "$SECRET_KEY"
    echo "✓ secret key: $SECRET_KEY (back it up offline)"
    echo "✓ public key: $PUBLIC_KEY (commit it; rebuild the images)"
    exit 0
fi

VERSION="${1:-}"
# The release tag of the Linux product (release.yml); "vX.Y.Z" is taken
# for "linux-vX.Y.Z".
[[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] && VERSION="linux-$VERSION"
[[ "$VERSION" =~ ^linux-v[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "usage: $0 linux-vX.Y.Z --sha1 <declared SHA-1>"
shift
EXPECTED_SHA1=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --sha1) EXPECTED_SHA1="$2"; shift 2 ;;
        *) die "unknown option: $1" ;;
    esac
done
[[ "$EXPECTED_SHA1" =~ ^[0-9a-f]{40}$ ]] || die "--sha1 <40 hex chars, the value SPF accepted> is required"
[[ -f "$SECRET_KEY" ]] || die "$SECRET_KEY not found (run --init-key first)"
[[ -f "$PUBLIC_KEY" ]] || die "$PUBLIC_KEY not found (run --init-key first)"
command -v gh >/dev/null || die "gh not found"
command -v oras >/dev/null || die "oras not found (brew install oras)"

TARBALL="pili-pos-${VERSION}-x86_64.tar.gz"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "▸ Logging oras in to ghcr.io with the gh token…"
gh auth token | oras login ghcr.io -u "$(gh api user -q .login)" --password-stdin >/dev/null

# Exactly what the terminals will get: the package, not a local file.
echo "▸ Pulling $PACKAGE:$VERSION…"
oras pull "$PACKAGE:$VERSION" -o "$WORK"
[[ -f "$WORK/$TARBALL" ]] || die "$PACKAGE:$VERSION has no $TARBALL"

SHA256="$(shasum -a 256 "$WORK/$TARBALL" | awk '{print $1}')"
mkdir "$WORK/x"
tar -xzf "$WORK/$TARBALL" -C "$WORK/x" pili-pos
SHA1="$(shasum -a 1 "$WORK/x/pili-pos" | awk '{print $1}')"
if [[ "$SHA1" != "$EXPECTED_SHA1" ]]; then
    die "the released binary's SHA-1 is $SHA1, not the declared $EXPECTED_SHA1 - nothing published"
fi
echo "✓ SHA-1 $SHA1 matches the declared value"

SEMVER="${VERSION#linux-v}"
DECLARED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
cat > "$WORK/declared.json" <<JSON
{
  "format": 1,
  "platform": "linux",
  "version": "$SEMVER",
  "sha1": "$SHA1",
  "sha256": "$SHA256",
  "asset": "$TARBALL",
  "declared_at": "$DECLARED_AT"
}
JSON

echo "▸ Signing declared.json (minisign asks for the key's password)…"
minisign -S -s "$SECRET_KEY" -m "$WORK/declared.json" \
    -t "pili-pos $SEMVER declared $DECLARED_AT"
minisign -V -q -p "$PUBLIC_KEY" -m "$WORK/declared.json" -x "$WORK/declared.json.minisig" \
    || die "the signature does not verify with $PUBLIC_KEY - wrong key pair?"

echo "▸ Pushing $PACKAGE:declared…"
( cd "$WORK" && oras push "$PACKAGE:declared" \
    --artifact-type application/vnd.pili-pos.declared.v1 \
    "$TARBALL:application/gzip" \
    declared.json:application/json \
    declared.json.minisig:text/plain )

echo "▸ $VERSION: prerelease -> latest release…"
gh release upload "$VERSION" -R "$REPO" --clobber \
    "$WORK/declared.json" "$WORK/declared.json.minisig"
gh release edit "$VERSION" -R "$REPO" --prerelease=false --latest

echo "✓ $VERSION is the declared version - new terminals install it at first boot"
