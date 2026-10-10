#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - Linux freeze check
#
# The certified Linux build must not change while the Android port lands.
# This compares the frozen paths with a freeze tag (working tree included):
#
#   Cargo.toml Cargo.lock rust-toolchain.toml src-tauri ui
#   deploy/build-release.sh scripts/build_and_hash.sh
#   crates/<dir> for every path package of the cert build graph of pili-pos
#
# The cert build graph is `cargo tree -p pili-pos --no-default-features
# --features cert -e normal,build --offline --prefix none`; its path packages
# are printed. Fails when:
#   - git diff TAG -- <paths> differs (working tree included),
#   - git status --porcelain --untracked-files=all -- <paths> is not empty,
#   - a path package of that graph lives outside crates/ (src-tauri aside).
#
# Usage: scripts/android/check_linux_frozen.sh [--tag TAG | --ref COMMIT]
#   TAG defaults to linux-v1.0.0 when it exists, else linux-v1.0.0-rc1 (the
#   1.0.0 re-freeze; the misnamed linux-v1.1.0-rc1 is not a freeze tag).
#   --ref COMMIT compares with any commit-ish instead of a tag (the provisional
#   freeze point of PD48); same paths, same checks. --tag and --ref exclude
#   each other.
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

if [[ -n "$ref" ]]; then
    full="$(git rev-parse -q --verify "$ref^{commit}" 2>/dev/null)" || {
        echo "unknown ref: $ref" >&2
        exit 2
    }
    base="$full"
    label="ref $(git rev-parse --short "$full")"
else
    if [[ -z "$tag" ]]; then
        if tag_exists linux-v1.0.0; then
            tag=linux-v1.0.0
        elif tag_exists linux-v1.0.0-rc1; then
            tag=linux-v1.0.0-rc1
        else
            echo "no freeze tag yet (neither linux-v1.0.0 nor linux-v1.0.0-rc1): nothing to check against" >&2
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

command -v cargo >/dev/null || { echo "cargo is required" >&2; exit 2; }

paths=(Cargo.toml Cargo.lock rust-toolchain.toml src-tauri ui deploy/build-release.sh scripts/build_and_hash.sh)
status=0

tree="$(cargo tree -p pili-pos --no-default-features --features cert -e normal,build --offline --prefix none 2>&1)" || {
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
    src-tauri) ;; # the root package, frozen as a whole
    crates/*) paths+=("$rel") ;;
    *)
        echo "x path package of the cert build graph outside crates/: $rel" >&2
        status=1
        ;;
    esac
done <<<"$dirs"

# unique, stable order
uniq_paths=()
while IFS= read -r p; do uniq_paths+=("$p"); done < <(printf '%s\n' "${paths[@]}" | awk '!seen[$0]++')
paths=("${uniq_paths[@]}")

echo "frozen paths (${#paths[@]}):"
printf '  %s\n' "${paths[@]}"

if ! git diff --exit-code --stat "$base" -- "${paths[@]}"; then
    echo "x frozen paths differ from $base" >&2
    status=1
fi
untracked="$(git status --porcelain --untracked-files=all -- "${paths[@]}")"
if [[ -n "$untracked" ]]; then
    echo "x uncommitted or untracked changes in frozen paths:" >&2
    printf '%s\n' "$untracked" | sed 's/^/  /' >&2
    status=1
fi

[[ $status -eq 0 ]] || exit 1
echo "frozen against $label: ${#paths[@]} paths, clean"
