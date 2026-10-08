#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - lock parity of the root and the android/rust workspaces (PD59)
#
# android/rust has its own Cargo.lock. The crates that pos-engine pulls in must
# stay at the versions of the certified Linux build, so the set of
# (name, version) of
#   root:        cargo tree --locked --offline -p pos-engine --no-default-features
#                  --features cert -e normal,build --target all --prefix none
#   android/rust: cargo tree --locked --offline -p pos-engine
#                  -e normal,build --target all --prefix none
# must be identical (the features of pos-engine come from pos-ffi in android/rust).
# A difference is allowed only with a reason line in
# scripts/android/lock_parity_allow.txt. The comparison is lock_parity.py.
#
# Usage: scripts/android/check_lock_parity.sh
# Exit: 0 identical, 1 differences or cargo failure, 2 no android/rust yet.
# ─────────────────────────────────────────────────────────────────
set -euo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
cd "$ROOT"

if [[ ! -f android/rust/Cargo.toml ]]; then
    echo "no android/rust yet"
    exit 2
fi
command -v cargo >/dev/null || { echo "cargo is required" >&2; exit 2; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cargo tree --locked --offline -p pos-engine --no-default-features --features cert \
    -e normal,build --target all --prefix none >"$TMP/root.txt" ||
    { echo "cargo tree failed in the root workspace" >&2; exit 1; }
(cd android/rust && cargo tree --locked --offline -p pos-engine \
    -e normal,build --target all --prefix none) >"$TMP/android.txt" ||
    { echo "cargo tree failed in android/rust" >&2; exit 1; }

python3 scripts/android/lock_parity.py "$TMP/root.txt" "$TMP/android.txt" \
    --allow scripts/android/lock_parity_allow.txt
