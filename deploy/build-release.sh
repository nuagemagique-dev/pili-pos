#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# Tokyo Sushi Bar — POS Release Build
# Builds for x86_64-unknown-linux-gnu (Debian 12 / Intel J6412)
#
# Option A: Build natively on a Debian 12 machine or VM
# Option B: Cross-compile via Docker from macOS
#
# Usage: ./build-release.sh [--docker]
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="x86_64-unknown-linux-gnu"
OUTPUT_DIR="$PROJECT_ROOT/deploy/dist"

mkdir -p "$OUTPUT_DIR"

if [[ "${1:-}" == "--docker" ]]; then
    echo "══════════════════════════════════════════════════"
    echo "  Building via Docker (Debian 12 container)"
    echo "══════════════════════════════════════════════════"

    docker run --rm \
        -v "$PROJECT_ROOT":/workspace \
        -w /workspace \
        rust:1.83-bookworm \
        bash -c "
            apt-get update -qq && \
            apt-get install -y -qq \
                libwebkit2gtk-4.1-dev \
                libgtk-3-dev \
                libayatana-appindicator3-dev \
                librsvg2-dev \
                libssl-dev \
                pkg-config \
                nodejs \
                npm && \
            cd ui && npm ci && cd .. && \
            cargo tauri build --target $TARGET
        "

    # Copy the binary out
    BINARY=$(find "$PROJECT_ROOT/target/$TARGET/release/bundle" -name "moyun-pos" -type f 2>/dev/null | head -1)
    if [[ -z "$BINARY" ]]; then
        BINARY="$PROJECT_ROOT/target/$TARGET/release/moyun-pos"
    fi

    if [[ -f "$BINARY" ]]; then
        cp "$BINARY" "$OUTPUT_DIR/moyun-pos"
        echo ""
        echo "✓ Binary: $OUTPUT_DIR/moyun-pos"
        ls -lh "$OUTPUT_DIR/moyun-pos"
    else
        echo "✗ Binary not found. Check build output."
        exit 1
    fi

else
    echo "══════════════════════════════════════════════════"
    echo "  Native build (run on Debian 12 target or VM)"
    echo "══════════════════════════════════════════════════"
    echo ""
    echo "Prerequisites (run once on the build machine):"
    echo "  sudo apt-get install -y \\"
    echo "    build-essential curl pkg-config \\"
    echo "    libwebkit2gtk-4.1-dev libgtk-3-dev \\"
    echo "    libayatana-appindicator3-dev librsvg2-dev \\"
    echo "    libssl-dev nodejs npm"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "  cargo install tauri-cli"
    echo ""

    cd "$PROJECT_ROOT/ui" && npm ci
    cd "$PROJECT_ROOT"
    cargo tauri build

    BINARY="$PROJECT_ROOT/target/release/moyun-pos"
    if [[ -f "$BINARY" ]]; then
        cp "$BINARY" "$OUTPUT_DIR/moyun-pos"
        echo ""
        echo "✓ Binary: $OUTPUT_DIR/moyun-pos"
        ls -lh "$OUTPUT_DIR/moyun-pos"
    fi
fi

# Copy deploy files alongside binary
cp "$PROJECT_ROOT/deploy/install.sh" "$OUTPUT_DIR/"
cp "$PROJECT_ROOT/deploy/update.sh" "$OUTPUT_DIR/"
cp "$PROJECT_ROOT/deploy/moyun-pos.service" "$OUTPUT_DIR/"
cp "$PROJECT_ROOT/src-tauri/seed.sql" "$OUTPUT_DIR/"
chmod +x "$OUTPUT_DIR/install.sh" "$OUTPUT_DIR/update.sh"

echo ""
echo "═══════════════════════════════════════════════════"
echo "  Deploy package ready in: $OUTPUT_DIR/"
echo ""
echo "  Files:"
ls -1 "$OUTPUT_DIR/"
echo ""
echo "  To install on POS terminal:"
echo "    scp -r $OUTPUT_DIR/* root@<pos-ip>:/tmp/moyun-pos/"
echo "    ssh root@<pos-ip> 'cd /tmp/moyun-pos && ./install.sh --binary ./moyun-pos --seed ./seed.sql'"
echo "═══════════════════════════════════════════════════"
