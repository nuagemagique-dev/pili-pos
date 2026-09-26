#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# moyun-pos - kiosk image build (mkosi, Debian 13 "trixie")
#
# Builds both profiles (production, admission) into
# deploy/image/mkosi.output/<profile>/moyun-pos-kiosk-<profile>.img.xz.
#
# Prerequisites:
#   - deploy/dist/moyun-pos (and, for the admission profile,
#     deploy/dist/pos-fdm-mock) - run `deploy/build-release.sh --docker`
#     first. This is what makes the image embed the exact binary whose
#     SHA-1 is in that run's build_info_*.txt (the value declared to
#     SPF Finances) - see .github/workflows/image.yml's `image` job,
#     which checks this on a read-only mount of the root partition.
#   - root, and a Linux x86_64 host (real or a --platform linux/amd64
#     container - the "privileged container" path from a macOS
#     development machine: `mkosi build` uses loopback devices only
#     for the UKI-splicing step below, not for the base disk image
#     (RepartOffline=yes, mkosi.conf), but that step still needs
#     CAP_SYS_ADMIN, hence "privileged").
#   - Everything else (mkosi itself, at the version pinned in
#     .mkosi-pin, plus binutils/systemd-ukify/xz-utils/gdisk) is
#     installed by this script if missing (apt-get, Debian/Ubuntu
#     hosts only - matches ubuntu-26.04 CI and the Mac's Debian
#     container).
#
# Usage: sudo ./build.sh [production] [admission]
#        (both are built if none are named)
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

IMAGE_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$IMAGE_DIR/../.." && pwd)"
DIST_DIR="$PROJECT_ROOT/deploy/dist"
MKOSI_PIN="$(tr -d '[:space:]' < "$IMAGE_DIR/.mkosi-pin")"
MKOSI_VENV="$IMAGE_DIR/.mkosi-venv"

# Kept in sync by hand with deploy/provision/provision.sh - the
# canonical copy of this pin lives there; this script only needs it to
# fetch and verify the same AppImage once, before staging a copy for
# each profile's offline, network-less chroot install.
RUSTDESK_VERSION="1.4.9"
RUSTDESK_URL="https://github.com/rustdesk/rustdesk/releases/download/${RUSTDESK_VERSION}/rustdesk-${RUSTDESK_VERSION}-x86_64.AppImage"
RUSTDESK_SHA256="7902cd60a4f29817eebe2668a15c9a1952ac690e8f7b07bfe7620fedd4e28217"

log() { echo "▸ $*"; }

if [[ $EUID -ne 0 ]]; then
    echo "ERROR: build.sh must run as root (mkosi's disk-image and" >&2
    echo "       loopback steps need it)." >&2
    exit 1
fi

if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
    echo "ERROR: build.sh needs Linux x86_64 - on macOS, run it inside a" >&2
    echo "       privileged linux/amd64 container (Docker Desktop)." >&2
    exit 1
fi

# ── Host tools ─────────────────────────────────────────────────
# NOTE: `ukify` is NOT installed from here, whatever the host ships: it
# is taken from mkosi's own tools tree (ToolsTree=default in
# mkosi.conf, built from Debian testing by mkosi itself on a
# Debian/Ubuntu host - see find_tool() below), which is also what makes
# the two extra Install/Update UKIs reproducible across the developer
# Mac and CI.
NEEDED_BINS=(python3 xz sgdisk losetup mount umount)
MISSING=0
for b in "${NEEDED_BINS[@]}"; do
    command -v "$b" >/dev/null 2>&1 || MISSING=1
done
# mkosi fetches Debian packages (the image, and the Debian tools tree)
# and checks them against the Debian archive keyring: an Ubuntu host
# does not have it by default.
DEBIAN_KEYRING=/usr/share/keyrings/debian-archive-keyring.gpg
# ukify (from the tools tree, see find_tool) runs under the host's
# python3 and needs its pefile module.
python3 -c 'import pefile' 2>/dev/null || MISSING=1
if [[ "$MISSING" -eq 1 || ! -x "$MKOSI_VENV/bin/mkosi" || ! -f "$DEBIAN_KEYRING" ]]; then
    log "Installing host prerequisites (apt-get)…"
    apt-get update -qq
    apt-get install -y -qq \
        python3 python3-venv python3-pip \
        curl ca-certificates \
        binutils \
        xz-utils gdisk parted util-linux e2fsprogs \
        debian-archive-keyring python3-pefile
fi

# Prefer mkosi's own tools tree (present after the first `mkosi build`
# in this directory) over the host binary, and the host binary over
# nothing - in that order.
find_tool() {
    local name="$1"
    if [[ -x "$IMAGE_DIR/mkosi.tools/usr/bin/$name" ]]; then
        echo "$IMAGE_DIR/mkosi.tools/usr/bin/$name"
    elif command -v "$name" >/dev/null 2>&1; then
        command -v "$name"
    else
        echo ""
    fi
}

if [[ ! -x "$MKOSI_VENV/bin/mkosi" ]] || \
   ! "$MKOSI_VENV/bin/mkosi" --version | grep -qx "mkosi $MKOSI_PIN"; then
    log "Installing mkosi $MKOSI_PIN into $MKOSI_VENV…"
    rm -rf "$MKOSI_VENV" "$IMAGE_DIR/.mkosi-src"
    curl -fsSL -o /tmp/mkosi.tar.gz \
        "https://codeload.github.com/systemd/mkosi/tar.gz/refs/tags/v${MKOSI_PIN}"
    mkdir -p "$IMAGE_DIR/.mkosi-src"
    tar -xzf /tmp/mkosi.tar.gz -C "$IMAGE_DIR/.mkosi-src" --strip-components=1
    rm -f /tmp/mkosi.tar.gz
    python3 -m venv "$MKOSI_VENV"
    "$MKOSI_VENV/bin/pip" install --quiet "$IMAGE_DIR/.mkosi-src"
fi
MKOSI="$MKOSI_VENV/bin/mkosi"
"$MKOSI" --version

if [[ ! -f "$DIST_DIR/moyun-pos" ]]; then
    echo "ERROR: $DIST_DIR/moyun-pos not found - run" >&2
    echo "       'deploy/build-release.sh --docker' first." >&2
    exit 1
fi

# ── RustDesk: fetched and verified once, reused by every profile ──
CACHE_DIR="$IMAGE_DIR/.cache"
mkdir -p "$CACHE_DIR"
RUSTDESK_CACHED="$CACHE_DIR/rustdesk-${RUSTDESK_VERSION}.AppImage"
if [[ ! -f "$RUSTDESK_CACHED" ]]; then
    log "Downloading RustDesk ${RUSTDESK_VERSION}…"
    curl -fsSL -o "$RUSTDESK_CACHED.tmp" "$RUSTDESK_URL"
    mv "$RUSTDESK_CACHED.tmp" "$RUSTDESK_CACHED"
fi
echo "${RUSTDESK_SHA256}  ${RUSTDESK_CACHED}" | sha256sum -c --quiet - \
    || { echo "ERROR: RustDesk checksum mismatch" >&2; exit 1; }

PROFILES=("$@")
[[ "${#PROFILES[@]}" -eq 0 ]] && PROFILES=(production admission)

BUILD_INPUT_BASE="$IMAGE_DIR/mkosi.extra/opt/moyun-pos/.build-input"
BUILD_INPUT_ADMISSION="$IMAGE_DIR/mkosi.profiles/admission/mkosi.extra/opt/moyun-pos/.build-input"

cleanup_staging() {
    rm -rf "$BUILD_INPUT_BASE" "$BUILD_INPUT_ADMISSION"
}
trap cleanup_staging EXIT
cleanup_staging

stage_shared_inputs() {
    mkdir -p "$BUILD_INPUT_BASE"
    install -m 755 "$DIST_DIR/moyun-pos" "$BUILD_INPUT_BASE/moyun-pos"
    install -m 755 "$RUSTDESK_CACHED" "$BUILD_INPUT_BASE/rustdesk.AppImage"
    # provision.sh --phase image needs these next to it inside the
    # chroot (see mkosi.postinst.chroot); staged fresh from the
    # canonical copies in deploy/provision/ and deploy/update.sh on
    # every build, never committed under mkosi.extra/ itself.
    mkdir -p "$IMAGE_DIR/mkosi.extra/opt/moyun-pos/deploy/provision"
    install -m 755 "$PROJECT_ROOT/deploy/provision/provision.sh" \
        "$IMAGE_DIR/mkosi.extra/opt/moyun-pos/deploy/provision/provision.sh"
    install -m 644 "$PROJECT_ROOT/deploy/provision/packages.txt" \
        "$IMAGE_DIR/mkosi.extra/opt/moyun-pos/deploy/provision/packages.txt"
    install -m 755 "$PROJECT_ROOT/deploy/update.sh" \
        "$IMAGE_DIR/mkosi.extra/opt/moyun-pos/deploy/update.sh"
    # The kiosk session files provision.sh installs from ../ (next to
    # deploy/provision/): the systemd unit and the PAM service.
    install -m 644 "$PROJECT_ROOT/deploy/moyun-pos.service" \
        "$IMAGE_DIR/mkosi.extra/opt/moyun-pos/deploy/moyun-pos.service"
    install -D -m 644 "$PROJECT_ROOT/deploy/kiosk/pam.d-moyun-pos" \
        "$IMAGE_DIR/mkosi.extra/opt/moyun-pos/deploy/kiosk/pam.d-moyun-pos"
}

stage_admission_inputs() {
    mkdir -p "$BUILD_INPUT_ADMISSION"
    install -m 644 "$PROJECT_ROOT/src-tauri/seed_spf_test.sql" \
        "$BUILD_INPUT_ADMISSION/seed_spf_test.sql"
    if [[ ! -f "$DIST_DIR/pos-fdm-mock" ]]; then
        echo "ERROR: $DIST_DIR/pos-fdm-mock not found - run" >&2
        echo "       'deploy/build-release.sh --docker' first (it now" >&2
        echo "       also builds pos-fdm-mock for linux/amd64)." >&2
        exit 1
    fi
    install -m 755 "$DIST_DIR/pos-fdm-mock" "$BUILD_INPUT_ADMISSION/pos-fdm-mock"
}

# Two extra UKIs, reusing the .linux/.initrd/.osrel sections of the one
# mkosi already built, with a different embedded cmdline
# (moyun.installer=install|update) - the "Install"/"Update" boot menu
# entries on the USB stick (MANUEL_INSTALLATION_FR.md §3.2). Done here,
# as a plain step after `mkosi build`, rather than as an mkosi
# postoutput script: mkosi's postoutput hook timing/working directory
# for a disk image was not something this change could pin down with
# confidence (see the final report) - a separate, later step operating
# on the finished .raw file, fully under this script's control and
# easy to reason about, was the safer choice.
add_installer_uki_variants() {
    local raw="$1" workdir esp_loop esp_mnt root_mnt uki_src stem objcopy_bin ukify_bin

    objcopy_bin="$(find_tool objcopy)"
    ukify_bin="$(find_tool ukify)"
    if [[ -z "$objcopy_bin" || -z "$ukify_bin" ]]; then
        echo "  ⚠ objcopy/ukify not found (host or mkosi.tools) - skipping" >&2
        echo "    the Install/Update USB boot entries for this image." >&2
        return 0
    fi

    workdir="$(mktemp -d)"
    esp_mnt="$workdir/esp"
    root_mnt="$workdir/root"
    mkdir -p "$esp_mnt" "$root_mnt"

    esp_loop="$(losetup --show -f -P "$raw")"
    # shellcheck disable=SC2064  # values must expand now, not at trap time
    trap "umount '$esp_mnt' 2>/dev/null || true; umount '$root_mnt' 2>/dev/null || true; losetup -d '$esp_loop' 2>/dev/null || true; rm -rf '$workdir'" RETURN

    mount "${esp_loop}p1" "$esp_mnt"
    mount -o ro "${esp_loop}p2" "$root_mnt"
    uki_src="$(find "$esp_mnt/EFI/Linux" -maxdepth 1 -name '*.efi' | head -n1)"
    if [[ -z "$uki_src" ]]; then
        echo "  ⚠ no UKI found in the ESP - skipping Install/Update boot entries" >&2
        return 0
    fi
    stem="$workdir/uki"
    "$objcopy_bin" -O binary --only-section=.linux "$uki_src" "$stem.linux"
    "$objcopy_bin" -O binary --only-section=.initrd "$uki_src" "$stem.initrd"
    "$objcopy_bin" -O binary --only-section=.osrel "$uki_src" "$stem.osrel" 2>/dev/null || true

    # ukify defaults to the host's stub path. Prefer the stub shipped in
    # the image itself (systemd-boot-efi, same version as its bootloader),
    # then mkosi's tools tree, then the host.
    local stub="" candidate
    for candidate in \
        "$root_mnt/usr/lib/systemd/boot/efi/linuxx64.efi.stub" \
        "$IMAGE_DIR/mkosi.tools/usr/lib/systemd/boot/efi/linuxx64.efi.stub" \
        /usr/lib/systemd/boot/efi/linuxx64.efi.stub; do
        if [[ -f "$candidate" ]]; then
            stub="$candidate"
            break
        fi
    done
    if [[ -z "$stub" ]]; then
        echo "  ✗ linuxx64.efi.stub not found (image, mkosi.tools or host)" >&2
        return 1
    fi

    for mode in install update; do
        local args=(build
            --stub "$stub"
            --linux "$stem.linux"
            --initrd "$stem.initrd"
            --cmdline "ro quiet moyun.installer=${mode}"
            --output "$esp_mnt/EFI/Linux/moyun-pos-${mode}.efi")
        [[ -f "$stem.osrel" ]] && args+=(--os-release "$stem.osrel")
        "$ukify_bin" "${args[@]}"
    done
    log "  ✓ EFI/Linux/moyun-pos-install.efi, moyun-pos-update.efi added"
}

build_profile() {
    local profile="$1" out_dir raw_file img_xz
    out_dir="$IMAGE_DIR/mkosi.output/$profile"
    rm -rf "$out_dir"
    mkdir -p "$out_dir"

    stage_shared_inputs
    [[ "$profile" == "admission" ]] && stage_admission_inputs

    log "Building profile '$profile'…"
    ( cd "$IMAGE_DIR" && "$MKOSI" --profile="$profile" --output-dir="$out_dir" build )

    raw_file="$(find "$out_dir" -maxdepth 1 -name '*.raw' | head -n1)"
    if [[ -z "$raw_file" ]]; then
        echo "ERROR: no .raw output found for profile '$profile' in $out_dir" >&2
        exit 1
    fi

    add_installer_uki_variants "$raw_file"

    img_xz="$out_dir/moyun-pos-kiosk-${profile}.img.xz"
    log "Compressing $raw_file -> $img_xz…"
    xz -T0 -6 -c "$raw_file" > "$img_xz"
    # NOT a hash of the .raw image itself: this is the SHA-1 of the
    # moyun-pos BINARY this image embeds - the value declared to SPF
    # Finances (build_info_*.txt from build-release.sh --docker) - kept
    # alongside the image so .github/workflows/image.yml's `image` job
    # can compare it against the binary inside the image without
    # re-extracting deploy/dist/moyun-pos.
    sha1sum "$DIST_DIR/moyun-pos" | awk '{print $1}' > "$out_dir/moyun-pos-embedded-binary.sha1"

    cleanup_staging
    log "✓ $img_xz"
}

for profile in "${PROFILES[@]}"; do
    build_profile "$profile"
done
