#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - reproducibility check of the unsigned release APK (P3.4, DECISIONS 11.13, PD83)
#
# Usage: scripts/android/check_repro.sh [--commit REF] [--keep-work] [--clean-prefetch]
#                                       [--prefetch-only]
#   --commit REF      the commit to build (default HEAD); the working tree must be clean
#   --keep-work       keep src.tar and both run directories (default: delete them, keep
#                     work/result.txt and work/logs)
#   --clean-prefetch  remove the prefetch volume of this commit's key first
#   --prefetch-only   stop after the prefetch (prints its lines, no total line)
#
# What it does
#   1. checks the preconditions (clean tree, Docker, free disk, the image
#      pili-android-build:p3 whose label pili.dockerfile.sha256 matches the Dockerfile of the
#      commit; otherwise it runs scripts/android/build_image.sh),
#   2. snapshots the commit with git archive,
#   3. prefetches once per key (sha256 over android/rust/Cargo.lock, android/gradle/
#      verification-metadata.xml, android/gradle/wrapper/gradle-wrapper.properties and the
#      Dockerfile), with the network on, into the Docker volume pili-android-prefetch-<12 hex>:
#      the cargo registry (cargo fetch --locked) and a Gradle user home (wrapper distribution,
#      modules-2) filled by android/docker/prefetch.gradle (method "init-script"). The method
#      "full-build" (PILI_PREFETCH_METHOD=full-build) additionally runs one network build of
#      :smoke:assembleRelease in the prefetch container and discards its outputs; use it when
#      an offline build misses something the init script did not fetch. The volume is reused,
#      and rebuilt when its COMPLETE marker is missing or names another prefetch.gradle,
#   4. runs two builds, each in a fresh "docker run --rm --platform linux/amd64 --network none"
#      with an empty /build, the snapshot and the prefetch volume read only, its own /out
#      (work/run1, work/run2); the volume content is copied into the build's own CARGO_HOME
#      and GRADLE_USER_HOME paths; the script is android/docker/build_in_container.sh --offline
#      with SOURCE_DATE_EPOCH = the commit time,
#   5. compares the two unsigned APKs.
# No key is involved: the APK stays unsigned, signing happens outside the build.
#
# Output, one line per check (also written to <work>/result.txt):
#     REPRO | <check> | PASS|FAIL|INFO | <detail>
# ending with exactly one "REPRO | total | PASS|FAIL | <detail>".
# Checks: build-1, build-2, apk-identical (cmp), so-identical (every lib/**/*.so inside the
# APKs), entry-manifest (apk_entry_diff.py), apk-check (check_apk.sh on the first APK).
# Exit 0 only when the two APKs are identical and every check passes; 1 when they differ or
# a prefetch/build failed; 2 on a precondition failure (clean tree, Docker, disk, image,
# usage).
#
# Environment
#   PILI_MIN_FREE_GB        free disk required in GB, 10^9 bytes (default 25)
#   PILI_PREFETCH_METHOD    init-script (default) or full-build
#   PILI_REPRO_CACHE        root of the work directories (default $HOME/.cache/pili-android/repro)
# The work directory is <cache>/<12 hex of the commit>/. Nothing is pushed, no registry is
# contacted except by the prefetch container (Maven, crates.io, Gradle distribution).
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    sed -n '2,/^# ──*$/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
    exit 0
fi

REF=HEAD
keep_work=0
clean_prefetch=0
prefetch_only=0
while [[ $# -gt 0 ]]; do
    case "$1" in
    --commit)
        [[ $# -ge 2 ]] || { echo "check_repro: --commit needs a REF" >&2; exit 2; }
        REF="$2"
        shift
        ;;
    --keep-work) keep_work=1 ;;
    --clean-prefetch) clean_prefetch=1 ;;
    --prefetch-only) prefetch_only=1 ;;
    *)
        echo "usage: check_repro.sh [--commit REF] [--keep-work] [--clean-prefetch] [--prefetch-only]" >&2
        exit 2
        ;;
    esac
    shift
done

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
cd "$ROOT" || exit 2
IMAGE="pili-android-build:p3"
PLATFORM="linux/amd64"
MIN_FREE_GB="${PILI_MIN_FREE_GB:-25}"
METHOD="${PILI_PREFETCH_METHOD:-init-script}"
CACHE_ROOT="${PILI_REPRO_CACHE:-$HOME/.cache/pili-android/repro}"
STARTED=()   # names of the containers this run started (removed on exit)

# ── output ──────────────────────────────────────────────────────
RESULT_FILE=""
PENDING=()
n_fail=0
emit() { # check status detail
    local line
    line="$(printf 'REPRO | %s | %s | %s' "$1" "$2" "$3")"
    printf '%s\n' "$line"
    if [[ -n "$RESULT_FILE" ]]; then printf '%s\n' "$line" >>"$RESULT_FILE"; else PENDING+=("$line"); fi
    [[ "$2" == "FAIL" ]] && n_fail=$((n_fail + 1))
    return 0
}
say() { printf 'check_repro: %s\n' "$*" >&2; }
precondition_fail() { # check detail
    emit "$1" FAIL "$2"
    emit total FAIL "precondition failed"
    exit 2
}
# shellcheck disable=SC2329 # invoked through trap
cleanup() {
    local c
    for c in ${STARTED[@]+"${STARTED[@]}"}; do
        [[ -n "$c" ]] && { docker rm -f "$c" >/dev/null 2>&1 || true; }
    done
}
trap cleanup EXIT
trap 'exit 130' INT TERM

# ── preconditions ───────────────────────────────────────────────
for t in git docker python3 tar shasum unzip cmp; do
    command -v "$t" >/dev/null 2>&1 || precondition_fail tool "$t is not installed"
done
COMMIT="$(git rev-parse --verify --quiet "${REF}^{commit}")" || precondition_fail commit "unknown ref $REF"
SHORT="${COMMIT:0:12}"
if [[ -n "$(git status --porcelain)" ]]; then
    precondition_fail clean-tree "the working tree is not clean (git status --porcelain is not empty)"
fi
docker info >/dev/null 2>&1 || precondition_fail docker "the Docker daemon is not running"
mkdir -p "$CACHE_ROOT" || precondition_fail disk "cannot create $CACHE_ROOT"
free_kb="$(df -Pk "$CACHE_ROOT" | awk 'NR==2 {print $4}')"
[[ "$free_kb" =~ ^[0-9]+$ ]] || precondition_fail disk "cannot read the free disk space of $CACHE_ROOT"
if [[ $((free_kb * 1024)) -lt $((MIN_FREE_GB * 1000000000)) ]]; then
    precondition_fail disk "only $((free_kb * 1024 / 1000000000)) GB free, at least $MIN_FREE_GB GB needed (PILI_MIN_FREE_GB)"
fi

DOCKERFILE_SHA="$(git show "$COMMIT:android/docker/Dockerfile" | shasum -a 256 | cut -d' ' -f1)"
image_label() { docker image inspect "$IMAGE" --format '{{index .Config.Labels "pili.dockerfile.sha256"}}' 2>/dev/null; }
if [[ "$(image_label)" != "$DOCKERFILE_SHA" ]]; then
    say "image $IMAGE is missing or its Dockerfile label differs: running scripts/android/build_image.sh"
    bash "$ROOT/scripts/android/build_image.sh" >&2 || precondition_fail image "build_image.sh failed"
    [[ "$(image_label)" == "$DOCKERFILE_SHA" ]] ||
        precondition_fail image "the image label is not the sha256 of android/docker/Dockerfile at $SHORT"
fi
IMAGE_ID="$(docker image inspect "$IMAGE" --format '{{.Id}}')"
EPOCH="$(git show -s --format=%ct "$COMMIT")"
[[ "$EPOCH" =~ ^[0-9]+$ ]] || precondition_fail commit "cannot read the commit time of $SHORT"
case "$METHOD" in init-script | full-build) ;; *) precondition_fail usage "PILI_PREFETCH_METHOD must be init-script or full-build" ;; esac

BASE="$CACHE_ROOT/$SHORT"
WORK="$BASE/work"
SNAP="$BASE/src.tar"
LOGS="$WORK/logs"
rm -rf "$WORK" "$SNAP"
mkdir -p "$LOGS" "$WORK/run1" "$WORK/run2" || precondition_fail disk "cannot create $WORK"
RESULT_FILE="$WORK/result.txt"
: >"$RESULT_FILE"
for l in ${PENDING[@]+"${PENDING[@]}"}; do printf '%s\n' "$l" >>"$RESULT_FILE"; done

emit commit INFO "$COMMIT"
emit image INFO "$IMAGE_ID ($IMAGE)"
emit dockerfile-sha256 INFO "$DOCKERFILE_SHA"
emit source-date-epoch INFO "$EPOCH ($(date -u -r "$EPOCH" +%Y-%m-%dT%H:%M:%SZ))"
emit free-disk INFO "$((free_kb * 1024 / 1000000000)) GB free before the run (minimum $MIN_FREE_GB)"

# ── snapshot ────────────────────────────────────────────────────
git archive --format=tar -o "$SNAP" "$COMMIT" || precondition_fail snapshot "git archive failed"
emit snapshot INFO "sha256 $(shasum -a 256 "$SNAP" | cut -d' ' -f1), $(stat -f %z "$SNAP" 2>/dev/null || stat -c %s "$SNAP") bytes"

# ── prefetch ────────────────────────────────────────────────────
KEY="$({
    git show "$COMMIT:android/rust/Cargo.lock"
    git show "$COMMIT:android/gradle/verification-metadata.xml"
    git show "$COMMIT:android/gradle/wrapper/gradle-wrapper.properties"
    git show "$COMMIT:android/docker/Dockerfile"
} | shasum -a 256 | cut -d' ' -f1)"
VOL="pili-android-prefetch-${KEY:0:12}"
emit prefetch-key INFO "$KEY (volume $VOL)"

dock() { docker run --rm --platform "$PLATFORM" "$@"; }
# The COMPLETE marker holds the sha256 of prefetch.gradle: a volume filled by another version of
# the init script is not reused (the key itself only covers the four inputs named above).
PF_SHA="$(git show "$COMMIT:android/docker/prefetch.gradle" | shasum -a 256 | cut -d' ' -f1)"
vol_complete() { [[ "$(dock -v "$VOL:/prefetch:ro" "$IMAGE" cat /prefetch/COMPLETE 2>/dev/null)" == "$PF_SHA" ]]; }

if [[ "$clean_prefetch" -eq 1 ]] && docker volume inspect "$VOL" >/dev/null 2>&1; then
    docker volume rm "$VOL" >/dev/null 2>&1 || true
    emit prefetch-clean INFO "removed volume $VOL"
fi
if docker volume inspect "$VOL" >/dev/null 2>&1 && vol_complete; then
    pf_method="$(dock -v "$VOL:/prefetch:ro" "$IMAGE" cat /prefetch/METHOD 2>/dev/null)"
    emit prefetch INFO "reused volume $VOL (method ${pf_method:-unknown})"
else
    docker volume rm "$VOL" >/dev/null 2>&1 || true
    docker volume create "$VOL" >/dev/null || precondition_fail prefetch "cannot create volume $VOL"
    say "prefetch into $VOL (network on, method $METHOD); log $LOGS/prefetch.log"
    t0=$(date +%s)
    PF_NAME="pili-repro-$$-prefetch"
    STARTED+=("$PF_NAME")
    # shellcheck disable=SC2016 # expanded inside the container
    dock --name "$PF_NAME" \
        -e "SOURCE_DATE_EPOCH=$EPOCH" -e "PILI_PREFETCH_METHOD=$METHOD" -e "PILI_PREFETCH_SHA=$PF_SHA" \
        -v "$SNAP:/in/src.tar:ro" -v "$VOL:/prefetch" \
        "$IMAGE" bash -c '
set -euo pipefail
export CARGO_HOME=/opt/cargo GRADLE_USER_HOME=/build/gradle-home
find /prefetch -mindepth 1 -delete
mkdir -p /build/src "$GRADLE_USER_HOME"
tar -xf /in/src.tar -C /build/src
rm -f /build/src/android/local.properties
echo "PREFETCH | cargo fetch --locked --manifest-path android/rust/Cargo.toml"
(cd /build/src && cargo fetch --locked --manifest-path android/rust/Cargo.toml)
echo "PREFETCH | gradle -I docker/prefetch.gradle prefetchAll"
(cd /build/src/android && ./gradlew --no-daemon --no-build-cache --no-configuration-cache --console=plain \
    -Dorg.gradle.jvmargs="-Xmx3g -Dfile.encoding=UTF-8" -I docker/prefetch.gradle prefetchAll prefetchLazy)
if [ "$PILI_PREFETCH_METHOD" = "full-build" ]; then
    echo "PREFETCH | one full network build of :smoke:assembleRelease (outputs discarded)"
    mkdir -p /tmp/prefetch-out
    (cd /build/src/android && ./gradlew --no-daemon --no-build-cache --no-configuration-cache --console=plain \
        -Dorg.gradle.jvmargs="-Xmx3g -Dfile.encoding=UTF-8" :smoke:assembleRelease)
fi
cp -a /opt/cargo/registry /prefetch/cargo-registry
[ -d /opt/cargo/git ] && cp -a /opt/cargo/git /prefetch/cargo-git
mkdir -p /prefetch/gradle-home/caches
cp -a /build/gradle-home/wrapper /prefetch/gradle-home/wrapper
cp -a /build/gradle-home/caches/modules-2 /prefetch/gradle-home/caches/modules-2
echo "$PILI_PREFETCH_METHOD" > /prefetch/METHOD
echo "$PILI_PREFETCH_SHA" > /prefetch/COMPLETE
echo "PREFETCH | done"
' >"$LOGS/prefetch.log" 2>&1
    rc=$?
    STARTED=("${STARTED[@]/$PF_NAME/}")
    if [[ "$rc" -ne 0 ]]; then
        tail -n 30 "$LOGS/prefetch.log" >&2
        emit prefetch FAIL "the prefetch container failed (exit $rc), log $LOGS/prefetch.log"
        emit total FAIL "prefetch failed"
        exit 1
    fi
    pf_secs=$(($(date +%s) - t0))
    emit prefetch INFO "filled volume $VOL in ${pf_secs}s (method $METHOD)"
fi
pf_size="$(dock -v "$VOL:/prefetch:ro" "$IMAGE" du -sm /prefetch 2>/dev/null | cut -f1)"
# shellcheck disable=SC2016 # expanded inside the container
pf_parts="$(dock -v "$VOL:/prefetch:ro" "$IMAGE" bash -c 'printf "cargo-registry %s MB, gradle-home %s MB" "$(du -sm /prefetch/cargo-registry | cut -f1)" "$(du -sm /prefetch/gradle-home | cut -f1)"' 2>/dev/null)"
emit prefetch-size INFO "${pf_size:-?} MB (${pf_parts:-?})"
if [[ "$prefetch_only" -eq 1 ]]; then
    exit "$((n_fail > 0))"
fi

# ── two offline builds ──────────────────────────────────────────
APK_NAME="smoke-release-unsigned.apk"
dur_wall=()
dur_gradle=()
build_ok=()
run_build() { # n
    local n="$1" t0 rc g name
    name="pili-repro-$$-build$n"
    say "build $n: fresh container, --network none, offline; log $LOGS/build-$n.log"
    STARTED+=("$name")
    t0=$(date +%s)
    # shellcheck disable=SC2016 # expanded inside the container
    {
        echo "RUN | docker run --rm --platform $PLATFORM --network none SOURCE_DATE_EPOCH=$EPOCH image=$IMAGE_ID"
        echo "RUN | /in/src.tar (git archive $COMMIT) read only, /prefetch ($VOL) read only, /out = run$n"
        echo "RUN | build_in_container.sh --offline"
    } >"$LOGS/build-$n.log"
    dock --name "$name" --network none \
        -e "SOURCE_DATE_EPOCH=$EPOCH" \
        -v "$SNAP:/in/src.tar:ro" -v "$VOL:/prefetch:ro" -v "$WORK/run$n:/out" \
        "$IMAGE" bash -c '
set -euo pipefail
mkdir -p /opt/cargo /build/gradle-home /build/src
cp -a /prefetch/cargo-registry /opt/cargo/registry
if [ -d /prefetch/cargo-git ]; then cp -a /prefetch/cargo-git /opt/cargo/git; fi
cp -a /prefetch/gradle-home/. /build/gradle-home/
tar -xf /in/src.tar -C /build/src
exec bash /build/src/android/docker/build_in_container.sh --offline
' >>"$LOGS/build-$n.log" 2>&1
    rc=$?
    STARTED=("${STARTED[@]/$name/}")
    dur_wall[n]=$(($(date +%s) - t0))
    g="$(sed -n 's/^BUILD | gradle-seconds | \([0-9]*\)$/\1/p' "$LOGS/build-$n.log" | tail -n 1)"
    dur_gradle[n]="${g:-?}"
    if [[ "$rc" -eq 0 && -f "$WORK/run$n/$APK_NAME" ]]; then
        build_ok[n]=1
        emit "build-$n" PASS "$APK_NAME built offline in ${dur_wall[n]}s (gradle ${dur_gradle[n]}s)"
    else
        build_ok[n]=0
        tail -n 40 "$LOGS/build-$n.log" >&2
        emit "build-$n" FAIL "docker run exit $rc after ${dur_wall[n]}s, log $LOGS/build-$n.log"
    fi
    [[ -f "$WORK/run$n/gradle.log" ]] && cp "$WORK/run$n/gradle.log" "$LOGS/gradle-$n.log"
    return 0
}
run_build 1
if [[ "${build_ok[1]}" -eq 1 ]]; then
    run_build 2
else
    build_ok[2]=0
    emit build-2 FAIL "not run, build 1 failed"
fi

# ── compare ─────────────────────────────────────────────────────
A1="$WORK/run1/$APK_NAME"
A2="$WORK/run2/$APK_NAME"
if [[ "${build_ok[1]}" -eq 1 && "${build_ok[2]}" -eq 1 ]]; then
    s1="$(shasum -a 1 "$A1" | cut -d' ' -f1)"
    s2="$(shasum -a 1 "$A2" | cut -d' ' -f1)"
    if cmp -s "$A1" "$A2"; then
        emit apk-identical PASS "cmp equal, SHA-1 build-1 $s1 build-2 $s2"
    else
        emit apk-identical FAIL "cmp differs, SHA-1 build-1 $s1 build-2 $s2"
    fi

    # every native library inside the APKs
    so_names="$( (unzip -Z1 "$A1"; unzip -Z1 "$A2") | grep -E '^lib/.*\.so$' | sort -u)"
    so_bad=0
    so_detail=""
    while IFS= read -r so; do
        [[ -n "$so" ]] || continue
        h1="$(unzip -p "$A1" "$so" 2>/dev/null | shasum -a 1 | cut -d' ' -f1)"
        h2="$(unzip -p "$A2" "$so" 2>/dev/null | shasum -a 1 | cut -d' ' -f1)"
        [[ "$h1" == "$h2" ]] || so_bad=1
        so_detail+="${so_detail:+, }$so $h1$([[ "$h1" == "$h2" ]] || echo " != $h2")"
    done <<<"$so_names"
    if [[ -z "$so_names" ]]; then
        emit so-identical FAIL "no lib/**/*.so in the APKs"
    elif [[ "$so_bad" -eq 0 ]]; then
        emit so-identical PASS "$so_detail"
    else
        emit so-identical FAIL "$so_detail"
    fi
    u1="$(shasum -a 1 "$WORK/run1/libpos_ffi.so" 2>/dev/null | cut -d' ' -f1)"
    u2="$(shasum -a 1 "$WORK/run2/libpos_ffi.so" 2>/dev/null | cut -d' ' -f1)"
    us="$(stat -f %z "$WORK/run1/libpos_ffi.so" 2>/dev/null || stat -c %s "$WORK/run1/libpos_ffi.so" 2>/dev/null)"
    emit so-unstripped INFO "libpos_ffi.so (cargo output) SHA-1 build-1 $u1 build-2 $u2 $([[ "$u1" == "$u2" ]] && echo same || echo DIFFERENT), $us bytes"

    # content digest of the entries
    diff_out="$(python3 -I "$ROOT/scripts/android/apk_entry_diff.py" "$A1" "$A2" 2>&1)"
    diff_rc=$?
    m1="$(printf '%s\n' "$diff_out" | sed -n 's/^MANIFEST | A | \([0-9a-f]*\) | \([0-9]*\)$/\1/p')"
    m2="$(printf '%s\n' "$diff_out" | sed -n 's/^MANIFEST | B | \([0-9a-f]*\) | \([0-9]*\)$/\1/p')"
    n_entries="$(printf '%s\n' "$diff_out" | sed -n 's/^MANIFEST | A | [0-9a-f]* | \([0-9]*\)$/\1/p')"
    if [[ "$diff_rc" -eq 0 && -n "$m1" && "$m1" == "$m2" ]]; then
        emit entry-manifest PASS "sha256 $m1 ($n_entries entries)"
    else
        emit entry-manifest FAIL "build-1 ${m1:-?} build-2 ${m2:-?} (apk_entry_diff.py exit $diff_rc)"
        printf '%s\n' "$diff_out" | grep -E '^(DIFF|ONLY|META|RESULT)' | while IFS= read -r l; do emit entry-diff INFO "$l"; done
    fi

    # the format checks of the first APK
    chk_out="$(bash "$ROOT/scripts/android/check_apk.sh" "$A1" 2>&1)"
    chk_rc=$?
    printf '%s\n' "$chk_out" >"$LOGS/check_apk.txt"
    if [[ "$chk_rc" -eq 0 ]]; then
        emit apk-check PASS "check_apk.sh on build-1 ($(printf '%s\n' "$chk_out" | grep -c ' | PASS | ') checks passed)"
    else
        printf '%s\n' "$chk_out" >&2
        emit apk-check FAIL "check_apk.sh exit $chk_rc on build-1, output in $LOGS/check_apk.txt"
    fi

    emit apk-sha1 INFO "$s1"
    emit apk-sha256 INFO "$(shasum -a 256 "$A1" | cut -d' ' -f1)"
    emit apk-size INFO "$(stat -f %z "$A1" 2>/dev/null || stat -c %s "$A1") bytes"
    so_stripped_size="$(unzip -l "$A1" lib/arm64-v8a/libpos_ffi.so 2>/dev/null | awk '$4=="lib/arm64-v8a/libpos_ffi.so" {print $1}')"
    emit so-stripped INFO "libpos_ffi.so in the APK SHA-1 $(unzip -p "$A1" lib/arm64-v8a/libpos_ffi.so | shasum -a 1 | cut -d' ' -f1), ${so_stripped_size:-?} bytes"
else
    emit apk-identical FAIL "not compared, a build failed"
fi
emit duration-build-1 INFO "${dur_wall[1]:-?}s wall, gradle ${dur_gradle[1]:-?}s"
emit duration-build-2 INFO "${dur_wall[2]:-?}s wall, gradle ${dur_gradle[2]:-?}s"
free_kb="$(df -Pk "$CACHE_ROOT" | awk 'NR==2 {print $4}')"
emit free-disk INFO "$((free_kb * 1024 / 1000000000)) GB free after the builds"

# ── clean up the work directory ─────────────────────────────────
if [[ "$keep_work" -eq 0 ]]; then
    rm -rf "$SNAP" "$WORK/run1" "$WORK/run2"
fi

if [[ "$n_fail" -eq 0 ]]; then
    emit total PASS "the two unsigned APKs are byte-identical (commit $SHORT)"
    exit 0
fi
emit total FAIL "$n_fail check(s) failed (commit $SHORT)"
exit 1
