#!/usr/bin/env bash
# Self-test of scripts/android/check_android_frozen.sh. Works in temporary
# clones of the current commit (the real repository is never tagged):
#   clean                              -> 0
#   edit of an android/app/src/main file -> 1
#   edit of an android/app/src/test file -> 0 (tests are not APK inputs)
#   edit of a crates/pos-engine file   -> 1
#   edit of a ui/ file                 -> 0 (Linux only)
#   edit of a src-tauri file           -> 0 (Linux only)
#   untracked file in android/rust     -> 1
#   edit of rust-toolchain.toml        -> 1
#   no tag                             -> 2
#   a linux-v1.0.0-rc2 tag is not an Android freeze tag -> 2
#   --ref HEAD on a clean tree         -> 0
#   --ref with an unknown commit       -> 2
#   --ref together with --tag          -> 2
#   --list contains android/rust and crates/pos-engine
# The suite runs twice: once with the real repository as the clone source, and
# once with a throwaway source that already carries the Android freeze tags
# (android-v1.0.0-rc3 and android-v1.0.0). The clone copies those tags; the
# suite removes them before tagging. The real repository's tag list must be
# identical before and after the run.
# Needs cargo (cargo tree --offline) and the vendored/cached dependencies.
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

tags_before="$(git -C "$ROOT" tag -l)"
fails=0
total=0

ARGS=()
run() { # <name> <want exit> [regex]; extra script arguments in ARGS
    local name="$1" want="$2" re="${3:-}" got out
    out="$(bash scripts/android/check_android_frozen.sh ${ARGS[@]+"${ARGS[@]}"} 2>&1)"
    got=$?
    if [[ "$got" -eq "$want" && ( -z "$re" || "$out" =~ $re ) ]]; then
        echo "PASS $name"
    else
        echo "FAIL $name (exit $got, want $want)"
        echo "$out" | tail -n 12 | sed 's/^/     /'
        fails=$((fails + 1))
    fi
}

# suite <label> <source repository>
suite() {
    local label="$1" src="$2" clone="$TMP/clone.$1" t
    echo "== $label"
    git clone -q --no-hardlinks "$src" "$clone" || { echo "FAIL $label: clone"; fails=$((fails + 1)); return; }
    (
        cd "$clone" || exit 1
        # The script under test is the working copy, so uncommitted edits are tested.
        cp "$ROOT/scripts/android/check_android_frozen.sh" scripts/android/check_android_frozen.sh
        for t in $(git tag -l 'android-v*'); do git tag -d "$t" >/dev/null; done
        git tag -a -m test android-v1.0.0-rc3 HEAD >/dev/null || { echo "FAIL $label: tag"; exit 1; }
        fails=0
        main_file="$(git ls-files 'android/app/src/main' | grep '\.kt$' | head -n 1)"
        test_file="$(git ls-files 'android/app/src/test' | grep '\.kt$' | head -n 1)"
        [[ -n "$main_file" && -n "$test_file" ]] || { echo "FAIL $label: sample files"; exit 1; }

        run "$label/clean" 0 'frozen against android-v1.0.0-rc3 \([0-9a-f]+\): [0-9]+ paths, clean'

        ARGS=(--ref HEAD)
        run "$label/ref-clean" 0 'frozen against ref [0-9a-f]+: [0-9]+ paths, clean'
        ARGS=()

        echo '// edit' >>"$main_file"
        run "$label/edit-app-main" 1 'differ'
        git checkout -q -- "$main_file"

        echo '// edit' >>"$test_file"
        run "$label/edit-app-test" 0
        git checkout -q -- "$test_file"

        echo '# edit' >>crates/pos-engine/Cargo.toml
        run "$label/edit-pos-engine" 1 'differ'
        git checkout -q -- crates/pos-engine/Cargo.toml

        echo '# edit' >>"$(git ls-files 'ui/*.md' 'ui/package.json' | head -n 1)"
        run "$label/edit-ui" 0
        git checkout -q -- ui

        echo '// edit' >>"$(git ls-files 'src-tauri/src/*.rs' | head -n 1)"
        run "$label/edit-src-tauri" 0
        git checkout -q -- src-tauri

        echo x >android/rust/untracked_probe.txt
        run "$label/untracked-in-android-rust" 1 'untracked'
        rm -f android/rust/untracked_probe.txt

        echo '# edit' >>rust-toolchain.toml
        run "$label/edit-rust-toolchain" 1 'differ'
        git checkout -q -- rust-toolchain.toml

        run "$label/clean-again" 0

        ARGS=(--list)
        list_out="$(bash scripts/android/check_android_frozen.sh --list 2>&1)"
        if grep -qx 'android/rust' <<<"$list_out" && grep -qx 'crates/pos-engine' <<<"$list_out"; then
            echo "PASS $label/list"
        else
            echo "FAIL $label/list"
            fails=$((fails + 1))
        fi

        ARGS=(--ref no-such-commit-anywhere)
        run "$label/ref-unknown" 2 'unknown ref'
        ARGS=(--tag android-v1.0.0-rc3 --ref HEAD)
        run "$label/ref-and-tag" 2 'mutually exclusive'
        ARGS=()

        git tag -d android-v1.0.0-rc3 >/dev/null
        run "$label/no-tag" 2 'no Android freeze tag yet'

        # The Linux freeze tag is not an Android freeze tag.
        git tag -f linux-v1.0.0-rc2 HEAD >/dev/null
        run "$label/linux-tag-only" 2 'no Android freeze tag yet'
        git tag -d linux-v1.0.0-rc2 >/dev/null
        exit "$fails"
    ) >"$TMP/suite.$1.out" 2>&1
    local rc=$?
    cat "$TMP/suite.$1.out"
    total=$((total + $(grep -c '^\(PASS\|FAIL\) ' "$TMP/suite.$1.out")))
    fails=$((fails + rc))
}

suite real "$ROOT"

# Throwaway source that already has the Android freeze tags.
SRC="$TMP/tagged-source"
git clone -q --no-hardlinks --no-tags "$ROOT" "$SRC" || { echo "FAIL: source clone"; exit 1; }
git -C "$SRC" tag -a -m test android-v1.0.0-rc3 HEAD
git -C "$SRC" tag -a -m test android-v1.0.0 HEAD
suite tagged-source "$SRC"

tags_after="$(git -C "$ROOT" tag -l)"
if [[ "$tags_before" != "$tags_after" ]]; then
    echo "FAIL: the tag list of the real repository changed during the run"
    fails=$((fails + 1))
fi

if [[ "$fails" -gt 0 ]]; then
    echo "TESTAF | total | FAIL | $total tests, $fails failing"
    echo "FAIL: $fails case(s)"
    exit 1
fi
echo "TESTAF | total | PASS | $total tests"
echo "PASS: all cases"
