#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - Android freeze check (INT-D44)
#
# v1 submits pili-pos-android only. The frozen build must not change once the
# Android freeze tag is set. This compares the inputs of the :app release APK
# (built by android/docker/build_in_container.sh) with the freeze tag, working
# tree included:
#
#   Cargo.toml rust-toolchain.toml android/rust (minus pos-ffi/tests) android/build.gradle.kts
#   android/settings.gradle.kts android/gradle.properties android/gradle
#   android/gradlew android/docker
#   android/app           (minus src/test and src/androidTest)
#   android/pos-core      (minus src/test)
#   crates/<dir> for every path package of
#   `cargo tree --manifest-path android/rust/Cargo.toml -p pos-ffi
#    -e normal,build --offline --prefix none`
#
# Root Cargo.lock, src-tauri, ui, deploy and docs are not frozen for Android.
# Fails when:
#   - git diff TAG -- <paths> differs (working tree included),
#   - git status --porcelain --untracked-files=all -- <paths> is not empty,
#   - a path package of that graph lives outside crates/ and android/rust.
#
# Usage: scripts/android/check_android_frozen.sh [--tag TAG | --ref COMMIT] [--list]
#   TAG defaults to android-v1.0.0 when it exists, else android-v1.0.0-rc3.
#   --ref COMMIT compares with any commit-ish instead of a tag. --tag and --ref
#   exclude each other. --list prints the resolved path list (pathspecs
#   included), one per line, and exits 0 without any comparison.
# Exit: 0 frozen, 1 not frozen, 2 no freeze tag, unknown ref, bad usage (or no cargo).
# On success: "frozen against TAG (<sha>): N paths, clean"
#          or "frozen against ref <sha>: N paths, clean"
# ─────────────────────────────────────────────────────────────────
set -euo pipefail
export LC_ALL=C
export PATH="$HOME/.cargo/bin:$PATH"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
cd "$ROOT"

tag=""
ref=""
list=0
while [[ $# -gt 0 ]]; do
    case "$1" in
    --tag)
        tag="${2:?--tag needs a tag name}"
        shift
        ;;
    --ref)
        ref="${2:?--ref needs a commit}"
        shift
        ;;
    --list) list=1 ;;
    -h | --help)
        sed -n '2,/^set -euo/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
        exit 0
        ;;
    *)
        echo "unknown argument: $1" >&2
        exit 2
        ;;
    esac
    shift
done

if [[ -n "$tag" && -n "$ref" ]]; then
    echo "--tag and --ref are mutually exclusive" >&2
    exit 2
fi

tag_exists() { git rev-parse -q --verify "refs/tags/$1^{commit}" >/dev/null 2>&1; }

base=""
label=""
if [[ $list -eq 0 ]]; then
    if [[ -n "$ref" ]]; then
        full="$(git rev-parse -q --verify "$ref^{commit}" 2>/dev/null)" || {
            echo "unknown ref: $ref" >&2
            exit 2
        }
        base="$full"
        label="ref $(git rev-parse --short "$full")"
    else
        if [[ -z "$tag" ]]; then
            if tag_exists android-v1.0.0; then
                tag=android-v1.0.0
            elif tag_exists android-v1.0.0-rc3; then
                tag=android-v1.0.0-rc3
            else
                echo "no Android freeze tag yet (neither android-v1.0.0 nor android-v1.0.0-rc3): nothing to check against" >&2
                exit 2
            fi
        elif ! tag_exists "$tag"; then
            echo "freeze tag not found: $tag" >&2
            exit 2
        fi
        sha="$(git rev-parse --short "refs/tags/$tag^{commit}")"
        base="$tag"
        label="$tag ($sha)"
    fi
fi

command -v cargo >/dev/null || { echo "cargo is required" >&2; exit 2; }

paths=(
    Cargo.toml rust-toolchain.toml
    android/rust
    android/build.gradle.kts android/settings.gradle.kts android/gradle.properties
    android/gradle android/gradlew android/docker
    android/app
    android/pos-core
)
excludes=(
    ':(exclude)android/app/src/test'
    ':(exclude)android/app/src/androidTest'
    ':(exclude)android/pos-core/src/test'
    ':(exclude)android/rust/pos-ffi/tests'
)
status=0

tree="$(cargo tree --manifest-path android/rust/Cargo.toml -p pos-ffi -e normal,build --offline --prefix none 2>&1)" || {
    echo "x cargo tree failed:" >&2
    echo "$tree" >&2
    exit 1
}
# "name vX.Y.Z (/abs/dir) (*)" -> /abs/dir, one per path package.
dirs="$(printf '%s\n' "$tree" | sed -n 's/^.* v[^ ]* (\(\/[^)]*\)).*$/\1/p' | LC_ALL=C sort -u)"
while IFS= read -r dir; do
    [[ -n "$dir" ]] || continue
    real="$(cd "$dir" && pwd -P)"
    rel="${real#"$ROOT"/}"
    case "$rel" in
    android/rust | android/rust/*) ;; # covered by android/rust
    crates/*) paths+=("$rel") ;;
    *)
        echo "x path package of the pos-ffi build graph outside crates/ and android/rust: $rel" >&2
        status=1
        ;;
    esac
done <<<"$dirs"

# unique, stable order
uniq_paths=()
while IFS= read -r p; do uniq_paths+=("$p"); done < <(printf '%s\n' "${paths[@]}" | awk '!seen[$0]++')
paths=("${uniq_paths[@]}")
specs=("${paths[@]}" "${excludes[@]}")

if [[ $list -eq 1 ]]; then
    printf '%s\n' "${specs[@]}"
    exit "$status"
fi

echo "frozen paths (${#paths[@]}):"
printf '  %s\n' "${paths[@]}"

if ! git diff --exit-code --stat "$base" -- "${specs[@]}"; then
    echo "x frozen paths differ from $base" >&2
    status=1
fi
untracked="$(git status --porcelain --untracked-files=all -- "${specs[@]}")"
if [[ -n "$untracked" ]]; then
    echo "x uncommitted or untracked changes in frozen paths:" >&2
    printf '%s\n' "$untracked" | sed 's/^/  /' >&2
    status=1
fi

[[ $status -eq 0 ]] || exit 1
echo "frozen against $label: ${#paths[@]} paths, clean"
