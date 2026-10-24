#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - make a plug-and-play installation USB key (macOS or Linux)
#
# Writes the kiosk image to the key, then, on the key's ESP:
#   - makes "Install" the default boot entry, with a 5 s menu: a new
#     terminal booted from the key installs itself (pili-installer
#     still asks OUI before touching a disk that holds fiscal data);
#   - copies the site directory to /pili/site/: site.yml (pos.json,
#     network) and secrets/ (FDM token and certificates, ...), which
#     pili-bootstrap applies on the terminal's first boot.
#
# The key then holds the site's secrets in clear: keep it like a key,
# and wipe them once the terminals are installed (--wipe-secrets).
#
# Usage:
#   deploy/usb/make-key.sh <linux-vX.Y.Z | image.img.xz | image.raw> --site <dir> <device>
#   deploy/usb/make-key.sh --wipe-secrets <device>
#   deploy/usb/make-key.sh --list            (candidate USB devices)
#
#   <device>   macOS: /dev/disk4   Linux: /dev/sdb   (whole disk)
#   linux-vX.Y.Z  downloads pili-pos-kiosk-linux-vX.Y.Z.img.xz from that
#              GitHub release of the repository (PILI_REPO, default
#              nuagemagique-dev/pili-pos) with gh; vX.Y.Z is taken for
#              linux-vX.Y.Z
#   --site     a directory shaped like deploy/usb/site.example/
#   --yes      no confirmation prompt
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

REPO="${PILI_REPO:-nuagemagique-dev/pili-pos}"
OS="$(uname -s)"
SOURCE=""
SITE_DIR=""
DEVICE=""
WIPE_ONLY=0
ASSUME_YES=0

die() { echo "ERROR: $*" >&2; exit 1; }
log() { echo "▸ $*"; }

list_devices() {
    if [[ "$OS" == Darwin ]]; then
        diskutil list external physical
    else
        lsblk -dpo NAME,SIZE,MODEL,TRAN,RM | awk 'NR == 1 || $4 == "usb" || $5 == "1"'
    fi
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --site)         SITE_DIR="$2"; shift 2 ;;
        --wipe-secrets) WIPE_ONLY=1; shift ;;
        --yes)          ASSUME_YES=1; shift ;;
        --list)         list_devices; exit 0 ;;
        -h|--help)      sed -n '2,28p' "$0"; exit 0 ;;
        -*)             die "unknown option: $1" ;;
        *)
            if [[ "$WIPE_ONLY" -eq 0 && -z "$SOURCE" ]]; then SOURCE="$1"
            elif [[ -z "$DEVICE" ]]; then DEVICE="$1"
            else die "unexpected argument: $1"; fi
            shift ;;
    esac
done
# `--wipe-secrets /dev/diskN`: the only positional is the device.
if [[ "$WIPE_ONLY" -eq 1 && -z "$DEVICE" && -n "$SOURCE" ]]; then
    DEVICE="$SOURCE"; SOURCE=""
fi
[[ -n "$DEVICE" ]] || die "no device given (see --list)"
[[ -b "$DEVICE" || -c "$DEVICE" ]] || die "$DEVICE is not a device"

# ── Refuse anything that is not a removable/external disk ──────────
if [[ "$OS" == Darwin ]]; then
    [[ "$DEVICE" =~ ^/dev/disk[0-9]+$ ]] || die "give the whole disk, e.g. /dev/disk4"
    INFO="$(diskutil info "$DEVICE")"
    grep -Eq 'Device Location: +External|Removable Media: +Removable' <<<"$INFO" \
        || die "$DEVICE is not an external/removable disk - refusing"
    grep -q 'Internal: +Yes' <<<"$INFO" && die "$DEVICE is internal - refusing"
    SIZE_DESC="$(awk -F': +' '/Disk Size/ {print $2; exit}' <<<"$INFO")"
    ESP_PART="${DEVICE}s1"
    RAW_DEV="${DEVICE/\/dev\/disk//dev/rdisk}"
else
    NAME="$(basename "$DEVICE")"
    [[ -e "/sys/block/$NAME" ]] || die "give the whole disk, e.g. /dev/sdb"
    TRAN="$(lsblk -dno TRAN "$DEVICE" 2>/dev/null || true)"
    RM="$(cat "/sys/block/$NAME/removable" 2>/dev/null || echo 0)"
    [[ "$TRAN" == usb || "$RM" == 1 ]] || die "$DEVICE is not a USB/removable disk - refusing"
    SIZE_DESC="$(lsblk -dno SIZE,MODEL "$DEVICE")"
    if [[ "$NAME" =~ [0-9]$ ]]; then ESP_PART="${DEVICE}p1"; else ESP_PART="${DEVICE}1"; fi
    RAW_DEV="$DEVICE"
fi

SUDO=""
[[ $EUID -ne 0 ]] && SUDO="sudo"

ESP_MNT=""
mount_esp() {
    ESP_MNT="$(mktemp -d)"
    if [[ "$OS" == Darwin ]]; then
        diskutil quiet mount -mountPoint "$ESP_MNT" "$ESP_PART" \
            || die "cannot mount the key's ESP ($ESP_PART)"
    else
        $SUDO mount -t vfat -o "uid=$(id -u),gid=$(id -g)" "$ESP_PART" "$ESP_MNT" \
            || die "cannot mount the key's ESP ($ESP_PART)"
    fi
}
umount_esp() {
    [[ -n "$ESP_MNT" ]] || return 0
    sync
    if [[ "$OS" == Darwin ]]; then
        diskutil quiet unmount "$ESP_MNT" || true
    else
        $SUDO umount "$ESP_MNT" || true
    fi
    rmdir "$ESP_MNT" 2>/dev/null || true
    ESP_MNT=""
}
WORK=""
cleanup() {
    umount_esp
    [[ -n "$WORK" ]] && rm -rf "$WORK"
}
trap cleanup EXIT

# ── --wipe-secrets: remove secrets/ from the key, keep the rest ────
if [[ "$WIPE_ONLY" -eq 1 ]]; then
    mount_esp
    if [[ -d "$ESP_MNT/pili/site/secrets" ]]; then
        # FAT keeps no copies; overwrite the content before unlinking.
        find "$ESP_MNT/pili/site/secrets" -type f -exec sh -c \
            'for f; do dd if=/dev/zero of="$f" bs=1 count="$(wc -c < "$f")" conv=notrunc 2>/dev/null; done' _ {} +
        rm -rf "$ESP_MNT/pili/site/secrets"
        log "secrets removed from $DEVICE"
    else
        log "no secrets on $DEVICE"
    fi
    exit 0
fi

[[ -n "$SOURCE" ]] || die "no image given (linux-vX.Y.Z, .img.xz or .raw)"
# The Linux release tags are linux-vX.Y.Z (.github/workflows/release.yml).
[[ "$SOURCE" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] && SOURCE="linux-$SOURCE"
[[ -n "$SITE_DIR" ]] || die "--site <dir> is required (copy deploy/usb/site.example/)"
[[ -f "$SITE_DIR/site.yml" ]] || die "$SITE_DIR/site.yml not found"
if grep -v '^[[:space:]]*#' "$SITE_DIR/site.yml" | grep -q 'CHANGE_ME'; then
    die "$SITE_DIR/site.yml still has CHANGE_ME values"
fi

# ── The image ──────────────────────────────────────────────────────
WORK="$(mktemp -d)"
if [[ "$SOURCE" =~ ^linux-v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    command -v gh >/dev/null || die "gh not found (needed to download $SOURCE)"
    log "Downloading pili-pos-kiosk-$SOURCE.img.xz from $REPO…"
    gh release download "$SOURCE" -R "$REPO" -D "$WORK" \
        -p "pili-pos-kiosk-$SOURCE.img.xz" -p SHA256SUMS
    ( cd "$WORK" && grep " pili-pos-kiosk-$SOURCE.img.xz\$" SHA256SUMS \
        | { if command -v sha256sum >/dev/null; then sha256sum -c -; else shasum -a 256 -c -; fi; } ) \
        || die "checksum mismatch on the downloaded image"
    IMAGE="$WORK/pili-pos-kiosk-$SOURCE.img.xz"
else
    [[ -f "$SOURCE" ]] || die "$SOURCE not found"
    IMAGE="$SOURCE"
fi

echo
echo "  Image : $IMAGE"
echo "  Site  : $SITE_DIR"
echo "  Clé   : $DEVICE ($SIZE_DESC) - TOUT SON CONTENU SERA EFFACÉ"
echo
if [[ "$ASSUME_YES" -eq 0 ]]; then
    read -r -p "Tapez OUI pour écrire la clé : " CONFIRM
    [[ "$CONFIRM" == OUI ]] || die "cancelled"
fi

# ── Write ──────────────────────────────────────────────────────────
if [[ "$OS" == Darwin ]]; then
    diskutil quiet unmountDisk force "$DEVICE" || true
else
    for p in $(lsblk -lnpo NAME "$DEVICE" | tail -n +2); do $SUDO umount "$p" 2>/dev/null || true; done
fi
log "Writing the image to $RAW_DEV (several minutes)…"
BS=4M
[[ "$OS" == Darwin ]] && BS=4m
case "$IMAGE" in
    *.xz) xz -dc "$IMAGE" | $SUDO dd of="$RAW_DEV" bs="$BS" 2>/dev/null ;;
    *)    $SUDO dd if="$IMAGE" of="$RAW_DEV" bs="$BS" 2>/dev/null ;;
esac
sync
if [[ "$OS" == Darwin ]]; then
    sleep 2
    diskutil quiet unmountDisk force "$DEVICE" || true
else
    $SUDO partprobe "$DEVICE" 2>/dev/null || true
    $SUDO udevadm settle 2>/dev/null || true
fi

# ── Key-only settings on its ESP ──────────────────────────────────
mount_esp
[[ -f "$ESP_MNT/EFI/Linux/pili-pos-install.efi" ]] \
    || die "this image has no Install entry (EFI/Linux/pili-pos-install.efi) - built without ukify?"
mkdir -p "$ESP_MNT/loader"
# The image's own loader.conf is kept as loader.conf.pili-orig (the
# installer puts it back on the internal disk); the key's adds
# default/timeout to it. systemd-boot: entry ids of EFI/Linux UKIs are
# their file names.
LOADER="$ESP_MNT/loader/loader.conf"
[[ -f "$LOADER.pili-orig" ]] || { [[ -f "$LOADER" ]] && cp "$LOADER" "$LOADER.pili-orig"; } || true
{
    [[ -f "$LOADER.pili-orig" ]] && grep -vE '^[[:space:]]*(default|timeout)[[:space:]]' "$LOADER.pili-orig"
    printf 'default pili-pos-install.efi\ntimeout 5\n'
} > "$LOADER.new"
mv "$LOADER.new" "$LOADER"

log "Copying the site directory…"
rm -rf "$ESP_MNT/pili/site"
mkdir -p "$ESP_MNT/pili/site"
cp "$SITE_DIR/site.yml" "$ESP_MNT/pili/site/site.yml"
if [[ -d "$SITE_DIR/secrets" ]]; then
    cp -R "$SITE_DIR/secrets" "$ESP_MNT/pili/site/secrets"
    find "$ESP_MNT/pili/site/secrets" \( -name '.*' -o -name 'README.md' \) -delete 2>/dev/null || true
fi
find "$ESP_MNT/pili" -type f | sed "s|^$ESP_MNT/||; s|^|    |"
umount_esp

if [[ "$OS" == Darwin ]]; then
    diskutil quiet eject "$DEVICE" || true
fi
echo
echo "✓ Key ready. On the terminal: Secure Boot OFF, TPM ON, boot from USB."
echo "  The key holds the site's secrets: wipe them after the install with"
echo "  deploy/usb/make-key.sh --wipe-secrets $DEVICE"
