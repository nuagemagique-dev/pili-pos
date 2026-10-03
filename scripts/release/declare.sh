#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - declare a released version (owner only, on the Mac)
#
# Once SPF Finances has accepted a version (deploy/UPDATE-STRATEGY.md),
# this publishes the signed manifest terminals trust:
#   declared.json          { version, sha1, sha256, asset_url, declared_at }
#   declared.json.minisig  minisign signature with the owner's OFFLINE key
# on the fixed "declared" release of the releases repository, and turns
# the version's own release from prerelease into latest.
#
# pili-bootstrap (deploy/ansible/roles/pos_binary) installs nothing
# else: no valid signature, no SHA-256/SHA-1 match, no install.
#
# Usage:
#   scripts/release/declare.sh --init-key
#       once: creates the key pair. The secret key stays in
#       ~/.minisign/ (password protected, back it up offline); the
#       public key goes to deploy/release/declared.pub - commit it,
#       every image built afterwards embeds it.
#   scripts/release/declare.sh vX.Y.Z --sha1 <SHA-1 accepted by SPF>
#
# Environment: PILI_RELEASES_REPO (default brunomoyse/moyun-pos-releases),
# PILI_MINISIGN_KEY (default ~/.minisign/pili-pos-declared.key).
# Needs: gh (logged in, write access to the releases repo), minisign.
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
REPO="${PILI_RELEASES_REPO:-brunomoyse/moyun-pos-releases}"
SECRET_KEY="${PILI_MINISIGN_KEY:-$HOME/.minisign/pili-pos-declared.key}"
PUBLIC_KEY="$PROJECT_ROOT/deploy/release/declared.pub"

die() { echo "ERROR: $*" >&2; exit 1; }
command -v minisign >/dev/null || die "minisign not found (brew install minisign)"

if [[ "${1:-}" == "--init-key" ]]; then
    [[ -f "$SECRET_KEY" ]] && die "$SECRET_KEY already exists - refusing to overwrite it"
    mkdir -p "$(dirname "$SECRET_KEY")" "$(dirname "$PUBLIC_KEY")"
    chmod 700 "$(dirname "$SECRET_KEY")"
    minisign -G -p "$PUBLIC_KEY" -s "$SECRET_KEY"
    echo "✓ secret key: $SECRET_KEY (back it up offline)"
    echo "✓ public key: $PUBLIC_KEY (commit it; rebuild the images)"
    exit 0
fi

VERSION="${1:-}"
[[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "usage: $0 vX.Y.Z --sha1 <declared SHA-1>"
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

TARBALL="pili-pos-${VERSION}-x86_64.tar.gz"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "▸ Downloading $TARBALL from $REPO…"
gh release download "$VERSION" -R "$REPO" -p "$TARBALL" -D "$WORK"

SHA256="$(shasum -a 256 "$WORK/$TARBALL" | awk '{print $1}')"
tar -xzf "$WORK/$TARBALL" -C "$WORK" pili-pos
SHA1="$(shasum -a 1 "$WORK/pili-pos" | awk '{print $1}')"
if [[ "$SHA1" != "$EXPECTED_SHA1" ]]; then
    die "the released binary's SHA-1 is $SHA1, not the declared $EXPECTED_SHA1 - nothing published"
fi
echo "✓ SHA-1 $SHA1 matches the declared value"

SEMVER="${VERSION#v}"
DECLARED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
cat > "$WORK/declared.json" <<JSON
{
  "version": "$SEMVER",
  "sha1": "$SHA1",
  "sha256": "$SHA256",
  "asset_url": "https://github.com/$REPO/releases/download/$VERSION/$TARBALL",
  "declared_at": "$DECLARED_AT"
}
JSON

echo "▸ Signing declared.json (minisign asks for the key's password)…"
minisign -S -s "$SECRET_KEY" -m "$WORK/declared.json" \
    -t "pili-pos $SEMVER declared $DECLARED_AT"
minisign -V -q -p "$PUBLIC_KEY" -m "$WORK/declared.json" -x "$WORK/declared.json.minisig" \
    || die "the signature does not verify with $PUBLIC_KEY - wrong key pair?"

echo "▸ Publishing on the 'declared' release of $REPO…"
if ! gh release view declared -R "$REPO" >/dev/null 2>&1; then
    gh release create declared -R "$REPO" --latest=false \
        --title "Declared version" \
        --notes "Signed manifest of the pili-pos version declared to SPF Finances. Read by the terminals' first-boot bootstrap; overwritten by scripts/release/declare.sh."
fi
gh release upload declared -R "$REPO" --clobber \
    "$WORK/declared.json" "$WORK/declared.json.minisig"
gh release edit "$VERSION" -R "$REPO" --prerelease=false --latest

echo "✓ $VERSION is the declared version - new terminals install it at first boot"
