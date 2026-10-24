#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - build a USB installation key AS A FILE, for
# scripts/qemu/run-kiosk.sh --usb-install: same result as
# deploy/usb/make-key.sh on a real stick (Install as the default boot
# entry, site directory in /pili/site/ on the ESP), without root or a
# device - the ESP is edited in place with mtools, in a Debian
# container.
#
# Usage: scripts/qemu/make-test-key.sh <image.img.xz|image.raw> <site-dir> <out.raw>
# ─────────────────────────────────────────────────────────────────
set -euo pipefail
IMAGE="${1:?usage: $0 <image> <site-dir> <out.raw>}"
SITE="${2:?usage: $0 <image> <site-dir> <out.raw>}"
OUT="${3:?usage: $0 <image> <site-dir> <out.raw>}"
[[ -f "$SITE/site.yml" ]] || { echo "ERROR: $SITE/site.yml not found" >&2; exit 1; }

case "$IMAGE" in
    *.img.xz) xz -dc "$IMAGE" > "$OUT" ;;
    *.raw)    cp "$IMAGE" "$OUT" ;;
    *) echo "ERROR: expected a .raw or .img.xz image" >&2; exit 1 ;;
esac

OUT_ABS="$(cd "$(dirname "$OUT")" && pwd)/$(basename "$OUT")"
SITE_ABS="$(cd "$SITE" && pwd)"
docker run --rm --platform linux/amd64 \
    -v "$OUT_ABS:/key.raw" -v "$SITE_ABS:/site:ro" debian:trixie bash -euc '
    apt-get update -qq >/dev/null
    DEBIAN_FRONTEND=noninteractive apt-get install -y -qq mtools fdisk >/dev/null
    START="$(partx -g -o START -n 1 /key.raw | tr -d " ")"
    ESP="/key.raw@@$((START * 512))"
    export MTOOLS_SKIP_CHECK=1
    mdir -i "$ESP" ::/EFI/Linux/pili-pos-install.efi >/dev/null \
        || { echo "ERROR: no Install entry in this image" >&2; exit 1; }
    # As deploy/usb/make-key.sh: keep the image loader.conf as
    # loader.conf.pili-orig, add default/timeout on top of it.
    mmd -i "$ESP" ::/loader 2>/dev/null || true
    : > /tmp/orig.conf
    if mtype -i "$ESP" ::/loader/loader.conf > /tmp/orig.conf 2>/dev/null; then
        mcopy -o -i "$ESP" /tmp/orig.conf ::/loader/loader.conf.pili-orig
    fi
    { grep -vE "^[[:space:]]*(default|timeout)[[:space:]]" /tmp/orig.conf || true
      printf "default pili-pos-install.efi\ntimeout 5\n"; } > /tmp/loader.conf
    mcopy -o -i "$ESP" /tmp/loader.conf ::/loader/loader.conf
    mmd -i "$ESP" ::/pili 2>/dev/null || true
    mdeltree -i "$ESP" ::/pili/site 2>/dev/null || true
    mmd -i "$ESP" ::/pili/site
    mcopy -i "$ESP" /site/site.yml ::/pili/site/site.yml
    if [[ -d /site/secrets ]]; then
        mcopy -s -i "$ESP" /site/secrets ::/pili/site/
        mdel -i "$ESP" ::/pili/site/secrets/README.md 2>/dev/null || true
    fi
    mdir -/ -i "$ESP" ::/pili
'
echo "✓ $OUT (boot it with scripts/qemu/run-kiosk.sh $OUT --usb-install)"
