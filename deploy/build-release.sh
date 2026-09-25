#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# moyun-pos — Release Build (certification build)
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

# Pinned by digest, not tag: `rust:1-bookworm` is a moving tag (rebuilt
# whenever Debian ships security updates), which would make the
# certification build non-reproducible byte-for-byte even with the same
# rust-toolchain.toml. .github/workflows/image.yml's `repro` job relies
# on this being deterministic (same image, run twice, same binary
# SHA-1).
# TODO(image-repro): this digest could not be resolved from this
# environment (no outbound registry access at the time this was
# written — see the final report). Before relying on the `repro` CI
# job, resolve the current `rust:1-bookworm` digest (e.g.
# `docker pull rust:1-bookworm && docker inspect --format='{{index .RepoDigests 0}}' rust:1-bookworm`
# on a machine with registry access) and replace the tag below with
# `rust@sha256:<digest>`.
RUST_BUILD_IMAGE="rust:1-bookworm"

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

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
        -e RUSTFLAGS="--remap-path-prefix=/workspace=/moyun-pos --remap-path-prefix=/usr/local/cargo=/cargo-home" \
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
                nodejs \
                npm
            git config --global --add safe.directory /workspace
            scripts/build_and_hash.sh
            # pos-fdm-mock: the admission kiosk image's fallback FDM
            # (deploy/image/mkosi.profiles/admission/). Same container,
            # same toolchain, same CARGO_TARGET_DIR as the cert binary
            # above, so it is a real linux/amd64 binary too — never
            # cross-compiled from the macOS host.
            cargo build --release --package pos-fdm-mock
        "
    BINARY="$LINUX_TARGET_DIR/release/moyun-pos"
    POS_FDM_MOCK="$LINUX_TARGET_DIR/release/pos-fdm-mock"
else
    echo "══════════════════════════════════════════════════"
    echo "  Native build (run on a Debian 12 x86_64 machine)"
    echo "══════════════════════════════════════════════════"
    if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
        echo "✗ Native mode needs Linux x86_64 — use --docker on this host."
        exit 1
    fi
    echo ""
    echo "Prerequisites (run once on the build machine):"
    echo "  sudo apt-get install -y \\"
    echo "    build-essential curl pkg-config \\"
    echo "    libwebkit2gtk-4.1-dev libgtk-3-dev \\"
    echo "    libayatana-appindicator3-dev librsvg2-dev \\"
    echo "    libssl-dev nodejs npm"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo ""

    CARGO_TARGET_DIR="$LINUX_TARGET_DIR" "$PROJECT_ROOT/scripts/build_and_hash.sh"
    CARGO_TARGET_DIR="$LINUX_TARGET_DIR" cargo build --release --package pos-fdm-mock
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
# part of a plain terminal deploy — copied best-effort.
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
