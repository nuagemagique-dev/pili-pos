#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - build the pinned Android build image (P3.4, PD83)
#
# Usage: scripts/android/build_image.sh [--no-cache]
#
# Builds android/docker/Dockerfile as pili-android-build:p3 (linux/amd64) with the label
# pili.dockerfile.sha256 = sha256 of the Dockerfile, then prints
#     IMAGE | id | <image id>
#     IMAGE | dockerfile | <sha256 of android/docker/Dockerfile>
#     IMAGE | size | <bytes>
# Exit 0 on success, 1 when docker build fails, 2 on a usage error, a missing tool or less than
# 20 GB (10^9 bytes) free on the disk of the repository (PILI_MIN_FREE_GB overrides the 20).
# The image is local only: this script never pushes it.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    sed -n '2,/^# ──*$/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
    exit 0
fi

no_cache=()
for a in "$@"; do
    case "$a" in
    --no-cache) no_cache=(--no-cache) ;;
    *)
        echo "usage: build_image.sh [--no-cache]" >&2
        exit 2
        ;;
    esac
done

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
IMAGE="pili-android-build:p3"
DOCKERFILE="$ROOT/android/docker/Dockerfile"
MIN_FREE_GB="${PILI_MIN_FREE_GB:-20}"

command -v docker >/dev/null 2>&1 || { echo "build_image: docker is not installed" >&2; exit 2; }
[[ -f "$DOCKERFILE" ]] || { echo "build_image: missing $DOCKERFILE" >&2; exit 2; }

free_kb="$(df -Pk "$ROOT" | awk 'NR==2 {print $4}')"
if [[ ! "$free_kb" =~ ^[0-9]+$ ]]; then
    echo "build_image: cannot read the free disk space of $ROOT" >&2
    exit 2
fi
free_bytes=$((free_kb * 1024))
if [[ "$free_bytes" -lt $((MIN_FREE_GB * 1000000000)) ]]; then
    echo "build_image: only $((free_bytes / 1000000000)) GB free, at least $MIN_FREE_GB GB needed" >&2
    exit 2
fi

sha="$(shasum -a 256 "$DOCKERFILE" | cut -d' ' -f1)"
docker build --platform linux/amd64 ${no_cache[@]+"${no_cache[@]}"} \
    --label "pili.dockerfile.sha256=$sha" \
    -t "$IMAGE" "$ROOT/android/docker" || {
    echo "build_image: docker build failed" >&2
    exit 1
}

echo "IMAGE | id | $(docker image inspect "$IMAGE" --format '{{.Id}}')"
echo "IMAGE | dockerfile | $sha"
echo "IMAGE | size | $(docker image inspect "$IMAGE" --format '{{.Size}}')"
