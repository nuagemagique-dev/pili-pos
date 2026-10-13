#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - lock parity of the root and the android/uccheck workspaces (P7D5)
#
# android/uccheck is a third Cargo workspace (the host side of the P7 parity proof). Its
# Cargo.lock starts as a copy of the root Cargo.lock, pruned offline by cargo. The crates
# that pos-engine pulls in must stay at the versions of the certified Linux build, so the
# set of (name, version) of
#   root:          cargo tree --locked --offline -p pos-engine --no-default-features
#                    --features cert -e normal,build --target all --prefix none
#   android/uccheck: cargo tree --locked --offline --manifest-path android/uccheck/Cargo.toml
#                    -p pos-engine -e normal,build --target all --prefix none
# must be identical (the features of pos-engine come from pos-uc-check in android/uccheck).
# A difference is allowed only with a reason line in scripts/android/uc_lock_parity_allow.txt.
# The comparison is lock_parity.py (its two sides are called 'root' and 'android' in its
# messages; here the second one is android/uccheck).
#
# Usage: scripts/android/uc_lock_parity.sh
# Exit: 0 identical, 1 differences, cargo failure or malformed input, 2 no android/uccheck yet.
#
# Self-test hooks (scripts/android/test_uc_lock_parity.sh only): PILI_UC_PARITY_ROOT_TREE and
# PILI_UC_PARITY_TREE name files that replace the two cargo tree outputs, PILI_UC_PARITY_ALLOW
# replaces the allow file.
# ─────────────────────────────────────────────────────────────────
set -euo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
cd "$ROOT"

ALLOW="${PILI_UC_PARITY_ALLOW:-scripts/android/uc_lock_parity_allow.txt}"
ROOT_TREE="${PILI_UC_PARITY_ROOT_TREE:-}"
UC_TREE="${PILI_UC_PARITY_TREE:-}"

if [[ -z "$UC_TREE" && ! -f android/uccheck/Cargo.toml ]]; then
    echo "no android/uccheck yet"
    exit 2
fi
if [[ -z "$ROOT_TREE" || -z "$UC_TREE" ]]; then
    command -v cargo >/dev/null || { echo "cargo is required" >&2; exit 1; }
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if [[ -n "$ROOT_TREE" ]]; then
    cp "$ROOT_TREE" "$TMP/root.txt"
else
    cargo tree --locked --offline -p pos-engine --no-default-features --features cert \
        -e normal,build --target all --prefix none >"$TMP/root.txt" ||
        { echo "cargo tree failed in the root workspace" >&2; exit 1; }
fi
if [[ -n "$UC_TREE" ]]; then
    cp "$UC_TREE" "$TMP/uccheck.txt"
else
    cargo tree --locked --offline --manifest-path android/uccheck/Cargo.toml -p pos-engine \
        -e normal,build --target all --prefix none >"$TMP/uccheck.txt" ||
        { echo "cargo tree failed in android/uccheck" >&2; exit 1; }
fi

# lock_parity.py exits 2 on a format error: that must not read as "skip" to the gate.
rc=0
python3 scripts/android/lock_parity.py "$TMP/root.txt" "$TMP/uccheck.txt" \
    --allow "$ALLOW" || rc=$?
[[ $rc -eq 0 ]] || exit 1
