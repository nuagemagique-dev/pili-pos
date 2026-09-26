#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# moyun-pos - Release Build (certification build)
# Builds the cert binary for x86_64 Linux (Debian 12 / Intel J6412)
# through scripts/build_and_hash.sh, then assembles the deploy package.
#
# Option A: Build natively on a Debian 12 x86_64 machine or VM
# Option B: Build in a linux/amd64 Debian 12 container (e.g. from macOS)
#
# The SHA-1 declared to SPF Finances is the one of THIS Linux binary
# (see the build_info_*.txt report copied into the package), never the
# hash of a macOS development build.
#
# Usage: ./build-release.sh [--docker]
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUTPUT_DIR="$PROJECT_ROOT/deploy/dist"
LINUX_TARGET_DIR="$PROJECT_ROOT/target/linux-amd64"

# Build inputs of the reference (container) build, pinned so that a
# rebuild from the same commit gives the same binary:
# - the Rust image by digest, not tag (`rust:1.98.1-bookworm` is rebuilt
#   whenever Debian ships security updates); its toolchain is the one
#   rust-toolchain.toml pins. Digest of the linux/amd64 image, resolved
#   from the Docker Hub registry API.
# - Node from the nodejs.org tarball, checked against its SHA-256, not
#   Debian's nodejs/npm packages (whose version moves with the archive).
# The system libraries (webkit2gtk, gtk, openssl) come from Debian 12 at
# build time; the binary links them dynamically.
# .github/workflows/image.yml's `repro` job builds twice and compares
# (scripts/check_repro.sh).
RUST_BUILD_IMAGE="rust:1.98.1-bookworm@sha256:c49256cbe5ea0188bc658a689500d70c41eb51f009a7a7be209caf60a944f3ec"
NODE_VERSION="v22.23.3"
NODE_TARBALL="node-${NODE_VERSION}-linux-x64.tar.xz"
NODE_SHA256="df450af89261115ef9f9e3830c3eeb2cc9213b63c720b1af623cb5dcbe2e02de"

# Paths that would otherwise be embedded in the binary (panic messages,
# debug info) are rewritten to fixed names, so the result does not
# depend on where the repository or cargo live on the build host.
REMAP_FLAGS="--remap-path-prefix=/workspace=/moyun-pos --remap-path-prefix=/usr/local/cargo=/cargo-home"

rm -rf "$OUTPUT_DIR"
# Created by the host user, so the container never owns target/.
mkdir -p "$OUTPUT_DIR" "$LINUX_TARGET_DIR"

if [[ "${1:-}" == "--docker" ]]; then
    echo "══════════════════════════════════════════════════"
    echo "  Building in a linux/amd64 Debian 12 container"
    echo "══════════════════════════════════════════════════"

    # - rustup in the image installs the toolchain pinned by
    #   rust-toolchain.toml on first use.
    # - ui/node_modules gets its own anonymous volume so the Linux
    #   `npm ci` never overwrites the host's macOS node_modules.
    # - RUSTFLAGS remaps the bind-mount path and cargo's install
    #   location to fixed, neutral placeholders: without this, the
    #   embedded debug info would encode this exact host's absolute
    #   path, so two builds run from different clones/checkouts (or
    #   even the same one, unmounted and remounted) would differ.
    docker run --rm \
        --platform linux/amd64 \
        -v "$PROJECT_ROOT":/workspace \
        -v /workspace/ui/node_modules \
        -w /workspace \
        -e CARGO_TARGET_DIR=/workspace/target/linux-amd64 \
        -e RUSTFLAGS="$REMAP_FLAGS" \
        -e HOST_UID="$(id -u)" \
        -e HOST_GID="$(id -g)" \
        "$RUST_BUILD_IMAGE" \
        bash -c "
            set -euo pipefail
            apt-get update -qq
            apt-get install -y -qq \
                libwebkit2gtk-4.1-dev \
                libgtk-3-dev \
                libayatana-appindicator3-dev \
                librsvg2-dev \
                libssl-dev \
                pkg-config \
                xz-utils
            curl -fsSL -o /tmp/$NODE_TARBALL https://nodejs.org/dist/$NODE_VERSION/$NODE_TARBALL
            echo '$NODE_SHA256  /tmp/$NODE_TARBALL' | sha256sum -c -
            mkdir -p /opt/node
            tar -xJf /tmp/$NODE_TARBALL -C /opt/node --strip-components=1
            export PATH=/opt/node/bin:\$PATH
            git config --global --add safe.directory /workspace
            scripts/build_and_hash.sh
            # pos-fdm-mock: the admission kiosk image's fallback FDM
            # (deploy/image/mkosi.profiles/admission/). Same container,
            # same toolchain, same CARGO_TARGET_DIR as the cert binary
            # above, so it is a real linux/amd64 binary too - never
            # cross-compiled from the macOS host.
            cargo build --release --package pos-fdm-mock
            # The container runs as root: hand the outputs back to the
            # host user, who deletes them before the next clean build.
            chown -R \$HOST_UID:\$HOST_GID target/linux-amd64 ui/dist build_info_*.txt
        "
    BINARY="$LINUX_TARGET_DIR/release/moyun-pos"
    POS_FDM_MOCK="$LINUX_TARGET_DIR/release/pos-fdm-mock"
else
    echo "══════════════════════════════════════════════════"
    echo "  Native build (run on a Debian 12 x86_64 machine)"
    echo "══════════════════════════════════════════════════"
    if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
        echo "✗ Native mode needs Linux x86_64 - use --docker on this host."
        exit 1
    fi
    echo ""
    echo "Prerequisites (run once on the build machine):"
    echo "  sudo apt-get install -y \\"
    echo "    build-essential curl pkg-config \\"
    echo "    libwebkit2gtk-4.1-dev libgtk-3-dev \\"
    echo "    libayatana-appindicator3-dev librsvg2-dev \\"
    echo "    libssl-dev"
    echo "  Node $NODE_VERSION (https://nodejs.org/dist/$NODE_VERSION/$NODE_TARBALL)"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo ""
    echo "Not the reference build: the SHA-1 declared to SPF Finances is the"
    echo "one of the --docker build (pinned image and Node)."
    echo ""

    # Same neutral names as the container build for this checkout and
    # this host's cargo home.
    NATIVE_REMAP="--remap-path-prefix=$PROJECT_ROOT=/moyun-pos --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo-home"
    RUSTFLAGS="$NATIVE_REMAP" CARGO_TARGET_DIR="$LINUX_TARGET_DIR" "$PROJECT_ROOT/scripts/build_and_hash.sh"
    RUSTFLAGS="$NATIVE_REMAP" CARGO_TARGET_DIR="$LINUX_TARGET_DIR" cargo build --release --package pos-fdm-mock
    BINARY="$LINUX_TARGET_DIR/release/moyun-pos"
    POS_FDM_MOCK="$LINUX_TARGET_DIR/release/pos-fdm-mock"
fi

if [[ ! -f "$BINARY" ]]; then
    echo "✗ Binary not found: $BINARY"
    exit 1
fi

# Assemble the deploy package
cp "$BINARY" "$OUTPUT_DIR/moyun-pos"
cp "$PROJECT_ROOT/deploy/install.sh" "$OUTPUT_DIR/"
cp "$PROJECT_ROOT/deploy/update.sh" "$OUTPUT_DIR/"
cp "$PROJECT_ROOT/deploy/moyun-pos.service" "$OUTPUT_DIR/"
mkdir -p "$OUTPUT_DIR/kiosk"
cp "$PROJECT_ROOT/deploy/kiosk/pam.d-moyun-pos" "$OUTPUT_DIR/kiosk/"
cp "$PROJECT_ROOT/src-tauri/seed_spf_test.sql" "$OUTPUT_DIR/"
LATEST_BUILD_INFO="$(find "$PROJECT_ROOT" -maxdepth 1 -name 'build_info_*.txt' -print0 \
    | xargs -0 ls -t | head -1)"
cp "$LATEST_BUILD_INFO" "$OUTPUT_DIR/"
chmod +x "$OUTPUT_DIR/install.sh" "$OUTPUT_DIR/update.sh"

# pos-fdm-mock: only meaningful for the admission kiosk image
# (deploy/image/build.sh stages it into mkosi.profiles/admission/), not
# part of a plain terminal deploy - copied best-effort.
if [[ -n "${POS_FDM_MOCK:-}" && -f "$POS_FDM_MOCK" ]]; then
    cp "$POS_FDM_MOCK" "$OUTPUT_DIR/pos-fdm-mock"
fi

echo ""
echo "═══════════════════════════════════════════════════"
echo "  Deploy package ready in: $OUTPUT_DIR/"
echo ""
echo "  Files:"
ls -1 "$OUTPUT_DIR/"
echo ""
echo "  SHA-1 (declare on the SPF form): $(grep '^SHA-1' "$OUTPUT_DIR"/build_info_*.txt | awk '{print $3}')"
echo ""
echo "  To install on a POS terminal (maintenance console, USB stick):"
echo "    sudo ./install.sh --binary ./moyun-pos --seed ./seed_spf_test.sql"
echo "    (admission image; use the establishment's catalogue seed in production)"
echo "═══════════════════════════════════════════════════"
