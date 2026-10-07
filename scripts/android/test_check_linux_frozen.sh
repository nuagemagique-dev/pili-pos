#!/usr/bin/env bash
# Self-test of scripts/android/check_linux_frozen.sh. Works in a temporary
# clone of the current commit (the real repository is never tagged):
#   clean                         -> 0
#   edit of Cargo.toml            -> 1
#   untracked file in src-tauri   -> 1
#   no tag                        -> 2
# Needs cargo (cargo tree --offline) and the vendored/cached dependencies.
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
CLONE="$TMP/clone"

git clone -q --no-hardlinks "$ROOT" "$CLONE" || { echo "FAIL: clone"; exit 1; }
cd "$CLONE" || exit 1
# The clone is of the committed HEAD: the script under test comes from there.
# Use the working copy of the scripts instead, so uncommitted edits are tested.
cp "$ROOT/scripts/android/check_linux_frozen.sh" scripts/android/check_linux_frozen.sh
git tag linux-v1.1.0-rc1 HEAD

fails=0
run() { # <name> <want exit> [regex]
    local name="$1" want="$2" re="${3:-}" got out
    out="$(bash scripts/android/check_linux_frozen.sh 2>&1)"
    got=$?
    if [[ "$got" -eq "$want" && ( -z "$re" || "$out" =~ $re ) ]]; then
        echo "PASS $name"
    else
        echo "FAIL $name (exit $got, want $want)"
        echo "$out" | tail -n 12 | sed 's/^/     /'
        fails=$((fails + 1))
    fi
}

run clean 0 'frozen against linux-v1.1.0-rc1 \([0-9a-f]+\): [0-9]+ paths, clean'

echo '# edit' >>Cargo.toml
run edit-Cargo.toml 1 'differ'
git checkout -q -- Cargo.toml

echo x >src-tauri/untracked_probe.txt
run untracked-in-src-tauri 1 'untracked'
rm -f src-tauri/untracked_probe.txt

run clean-again 0

git tag -d linux-v1.1.0-rc1 >/dev/null
run no-tag 2 'no freeze tag yet'

cd "$ROOT" || true
if [[ -n "$(git -C "$ROOT" tag -l 'linux-v*')" ]]; then
    echo "FAIL: the real repository has a linux-v* tag"
    fails=$((fails + 1))
fi

if [[ "$fails" -gt 0 ]]; then
    echo "FAIL: $fails case(s)"
    exit 1
fi
echo "PASS: all cases"
