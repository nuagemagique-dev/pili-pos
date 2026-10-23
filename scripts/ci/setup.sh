#!/usr/bin/env bash
# Prepares a fresh Ubuntu job container on the self-hosted runner.
# Everything is installed inside the container (apt packages, rustup under
# /opt/rustup and /opt/cargo, Node under /opt/node), never in the host-mounted
# home, so nothing survives the job on the runner host.
#
# Usage: scripts/ci/setup.sh [--rust] [--node VERSION] [apt packages...]
set -euo pipefail

rust=0
node=""
pkgs=()
while [ $# -gt 0 ]; do
    case "$1" in
        --rust) rust=1 ;;
        --node) node="$2"; shift ;;
        *) pkgs+=("$1") ;;
    esac
    shift
done

export DEBIAN_FRONTEND=noninteractive
apt-get update -q
apt-get install -y -q --no-install-recommends \
    build-essential pkg-config ca-certificates curl git xz-utils python3 "${pkgs[@]}"

if [ "$rust" = 1 ]; then
    curl -sSf https://sh.rustup.rs | sh -s -- -y -q --profile minimal --default-toolchain none
    echo "$CARGO_HOME/bin" >>"$GITHUB_PATH"
    # rust-toolchain.toml pins the compiler and its components.
    "$CARGO_HOME/bin/rustup" show active-toolchain || "$CARGO_HOME/bin/rustup" toolchain install
fi

if [ -n "$node" ]; then
    case "$(uname -m)" in
        aarch64) arch=arm64 ;;
        x86_64) arch=x64 ;;
        *) echo "unsupported arch $(uname -m)" >&2; exit 1 ;;
    esac
    mkdir -p /opt/node
    curl -sSfL "https://nodejs.org/dist/v$node/node-v$node-linux-$arch.tar.xz" \
        | tar -xJ -C /opt/node --strip-components=1
    echo /opt/node/bin >>"$GITHUB_PATH"
fi
