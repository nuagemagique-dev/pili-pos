#!/usr/bin/env bash
# Self-test of scripts/android/check_linux_frozen.sh. Works in temporary
# clones of the current commit (the real repository is never tagged):
#   clean                         -> 0
#   edit of Cargo.toml            -> 1
#   untracked file in src-tauri   -> 1
#   no tag                        -> 2
#   only the misnamed linux-v1.1.0-rc1 -> 2
#   --ref HEAD on a clean tree    -> 0
#   --ref HEAD after an edit of a frozen path -> 1
#   --ref with an unknown commit  -> 2
#   --ref together with --tag     -> 2
# The suite runs twice: once against the real repository as the clone source,
# and once against a throwaway source repository that already carries the real
# freeze tags (linux-v1.0.0-rc2 and linux-v1.0.0), the state after the soft
# freeze. The clone copies those tags; the suite removes them before tagging.
# The real repository's tag list must be identical before and after the run.
# Needs cargo (cargo tree --offline) and the vendored/cached dependencies.
set -uo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

tags_before="$(git -C "$ROOT" tag -l)"
fails=0

ARGS=()
run() { # <name> <want exit> [regex]; extra script arguments in ARGS
    local name="$1" want="$2" re="${3:-}" got out
    out="$(bash scripts/android/check_linux_frozen.sh ${ARGS[@]+"${ARGS[@]}"} 2>&1)"
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
        # The clone is of the committed HEAD: the script under test comes from
        # there. Use the working copy of the script, so uncommitted edits are tested.
        cp "$ROOT/scripts/android/check_linux_frozen.sh" scripts/android/check_linux_frozen.sh
        # Whatever freeze tags the source carried were copied: drop them all,
        # then tag HEAD (so the clone is checked against HEAD, never a real tag).
        for t in $(git tag -l 'linux-v*'); do git tag -d "$t" >/dev/null; done
        git tag -f linux-v1.0.0-rc2 HEAD >/dev/null || { echo "FAIL $label: tag"; exit 1; }
        fails=0

        run "$label/clean" 0 'frozen against linux-v1.0.0-rc2 \([0-9a-f]+\): [0-9]+ paths, clean'

        ARGS=(--ref HEAD)
        run "$label/ref-clean" 0 'frozen against ref [0-9a-f]+: [0-9]+ paths, clean'
        ARGS=()

        echo '# edit' >>Cargo.toml
        run "$label/edit-Cargo.toml" 1 'differ'
        ARGS=(--ref HEAD)
        run "$label/ref-edit-Cargo.toml" 1 'differ'
        ARGS=()
        git checkout -q -- Cargo.toml

        echo x >src-tauri/untracked_probe.txt
        run "$label/untracked-in-src-tauri" 1 'untracked'
        rm -f src-tauri/untracked_probe.txt

        run "$label/clean-again" 0

        ARGS=(--ref no-such-commit-anywhere)
        run "$label/ref-unknown" 2 'unknown ref'
        ARGS=(--tag linux-v1.0.0-rc2 --ref HEAD)
        run "$label/ref-and-tag" 2 'mutually exclusive'
        ARGS=()

        git tag -d linux-v1.0.0-rc2 >/dev/null
        run "$label/no-tag" 2 'no freeze tag yet'

        # The misnamed tag of the first soft freeze is not a freeze tag.
        git tag linux-v1.1.0-rc1 HEAD >/dev/null
        run "$label/misnamed-tag-only" 2 'no freeze tag yet \(neither linux-v1.0.0 nor linux-v1.0.0-rc2\)'
        git tag -d linux-v1.1.0-rc1 >/dev/null
        exit "$fails"
    ) >"$TMP/suite.$1.out" 2>&1
    local rc=$?
    cat "$TMP/suite.$1.out"
    fails=$((fails + rc))
}

suite real "$ROOT"

# Throwaway source that already has the real freeze tags.
SRC="$TMP/tagged-source"
git clone -q --no-hardlinks --no-tags "$ROOT" "$SRC" || { echo "FAIL: source clone"; exit 1; }
git -C "$SRC" tag linux-v1.0.0-rc2 HEAD
git -C "$SRC" tag linux-v1.0.0 HEAD
suite tagged-source "$SRC"

tags_after="$(git -C "$ROOT" tag -l)"
if [[ "$tags_before" != "$tags_after" ]]; then
    echo "FAIL: the tag list of the real repository changed during the run"
    fails=$((fails + 1))
fi

if [[ "$fails" -gt 0 ]]; then
    echo "FAIL: $fails case(s)"
    exit 1
fi
echo "PASS: all cases"
