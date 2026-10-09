#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - build the unsigned Android release APK inside the build image (P3.4, PD83)
#
# Runs INSIDE the image pili-android-build:p3 (android/docker/Dockerfile). Never signs.
#
# Usage: build_in_container.sh [--offline] [--write-metadata]
#   --offline         pass --offline to Gradle and set CARGO_NET_OFFLINE=true (needs prefetched
#                     Gradle artifacts and crates)
#   --write-metadata  run Gradle with --write-verification-metadata sha256 and copy the
#                     resulting android/gradle/verification-metadata.xml to /out
#
# Interface
#   /in/src.tar            input: a git archive of the commit (read only mount)
#   /out                   output directory (mount); receives
#                            app-release-unsigned.apk (the :app module, PD217), libpos_ffi.so (unstripped, from
#                            target/android-rust/aarch64-linux-android/android-release),
#                            mapping.txt, gradle.log, and verification-metadata.xml with
#                            --write-metadata
#   SOURCE_DATE_EPOCH      required, the commit time (seconds); every extracted file gets it as mtime
#   CARGO_HOME             default /opt/cargo
#   GRADLE_USER_HOME       default /build/gradle-home
# The tar is extracted to /build/src (fixed path, so the path remapping is identical in every
# run). When /build/src/android/gradlew already exists the extraction is skipped; the caller can
# therefore bootstrap this script out of the tar itself:
#   bash -c 'mkdir -p /build/src && tar -xf /in/src.tar -C /build/src \
#            && exec bash /build/src/android/docker/build_in_container.sh "$@"' _ [options]
# Prints the SHA-1 of the APK as its last line: "APK | sha1 | <sha1>".
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    sed -n '2,/^# ──*$/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//'
    exit 0
fi

offline=0
write_metadata=0
while [[ $# -gt 0 ]]; do
    case "$1" in
    --offline) offline=1 ;;
    --write-metadata) write_metadata=1 ;;
    *)
        echo "build_in_container: unknown argument: $1" >&2
        exit 2
        ;;
    esac
    shift
done

: "${SOURCE_DATE_EPOCH:?SOURCE_DATE_EPOCH is required (the commit time, seconds)}"
[[ "$SOURCE_DATE_EPOCH" =~ ^[0-9]+$ ]] || { echo "build_in_container: SOURCE_DATE_EPOCH is not a number" >&2; exit 2; }
export SOURCE_DATE_EPOCH

BUILD_SRC=/build/src
OUT=/out
export LANG=C.UTF-8 LC_ALL=C.UTF-8 TZ=UTC
export CARGO_INCREMENTAL=0
export CARGO_HOME="${CARGO_HOME:-/opt/cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-/opt/rustup}"
export GRADLE_USER_HOME="${GRADLE_USER_HOME:-/build/gradle-home}"
[[ "$offline" -eq 1 ]] && export CARGO_NET_OFFLINE=true
# The cargo target dir comes from android/rust/.cargo/config.toml: <repo>/target/android-rust.
TARGET_DIR="$BUILD_SRC/target/android-rust"

mkdir -p "$OUT" "$GRADLE_USER_HOME"
[[ -d "$OUT" && -w "$OUT" ]] || { echo "build_in_container: $OUT is not a writable directory" >&2; exit 2; }

# ── source ──────────────────────────────────────────────────────
if [[ ! -f "$BUILD_SRC/android/gradlew" ]]; then
    [[ -f /in/src.tar ]] || { echo "build_in_container: /in/src.tar is missing" >&2; exit 2; }
    mkdir -p "$BUILD_SRC"
    tar -xf /in/src.tar -C "$BUILD_SRC"
fi
# local.properties would carry a host sdk.dir; ANDROID_HOME comes from the image.
rm -f "$BUILD_SRC/android/local.properties"
# File mtimes are not an input of the build.
find "$BUILD_SRC" -exec touch -h -d "@${SOURCE_DATE_EPOCH}" {} +

# ── determinism ─────────────────────────────────────────────────
# RUSTFLAGS replaces (does not merge with) target.*.rustflags of android/rust/.cargo/config.toml,
# so the 16 KB page-size link argument is repeated here. rustc applies the LAST matching
# --remap-path-prefix, so the most specific prefix (the target dir) comes last.
unset CARGO_ENCODED_RUSTFLAGS CARGO_TARGET_DIR
export RUSTFLAGS="--remap-path-prefix=${BUILD_SRC}=/src --remap-path-prefix=${CARGO_HOME}=/cargo --remap-path-prefix=${RUSTUP_HOME}=/rustup --remap-path-prefix=${TARGET_DIR}=/target -C link-arg=-Wl,-z,max-page-size=16384"
# Same idea for the C that build scripts compile (aws-lc-sys, the bundled SQLite). cargo-ndk
# prepends its --target option to this value.
export CFLAGS_aarch64_linux_android="-ffile-prefix-map=${BUILD_SRC}=/src -ffile-prefix-map=${CARGO_HOME}=/cargo -ffile-prefix-map=${RUSTUP_HOME}=/rustup -ffile-prefix-map=${TARGET_DIR}=/target"

# --locked equivalent for the whole run: no Cargo.lock may change.
LOCKS=("$BUILD_SRC/Cargo.lock" "$BUILD_SRC/android/rust/Cargo.lock")
locks_hash() { sha256sum "${LOCKS[@]}"; }
before="$(locks_hash)"

# ── Gradle ──────────────────────────────────────────────────────
gradle_args=(--no-daemon --no-build-cache --no-configuration-cache --console=plain)
[[ "$offline" -eq 1 ]] && gradle_args+=(--offline)
[[ "$write_metadata" -eq 1 ]] && gradle_args+=(--write-verification-metadata sha256)
gradle_args+=(:app:assembleRelease)

echo "BUILD | start | $(date -u +%Y-%m-%dT%H:%M:%SZ) | SOURCE_DATE_EPOCH=$SOURCE_DATE_EPOCH"
t0=$(date +%s)
(cd "$BUILD_SRC/android" && ./gradlew "${gradle_args[@]}") 2>&1 | tee "$OUT/gradle.log"
t1=$(date +%s)
echo "BUILD | gradle-seconds | $((t1 - t0))"

[[ "$(locks_hash)" == "$before" ]] || { echo "build_in_container: a Cargo.lock changed during the build" >&2; exit 1; }

# ── outputs ─────────────────────────────────────────────────────
APK="$BUILD_SRC/android/app/build/outputs/apk/release/app-release-unsigned.apk"
SO="$TARGET_DIR/aarch64-linux-android/android-release/libpos_ffi.so"
MAPPING="$BUILD_SRC/android/app/build/outputs/mapping/release/mapping.txt"
for f in "$APK" "$SO" "$MAPPING"; do
    [[ -f "$f" ]] || { echo "build_in_container: missing output $f" >&2; exit 1; }
done
cp "$APK" "$OUT/app-release-unsigned.apk"
cp "$SO" "$OUT/libpos_ffi.so"
cp "$MAPPING" "$OUT/mapping.txt"
if [[ "$write_metadata" -eq 1 ]]; then
    cp "$BUILD_SRC/android/gradle/verification-metadata.xml" "$OUT/verification-metadata.xml"
fi
echo "APK | sha256 | $(sha256sum "$OUT/app-release-unsigned.apk" | cut -d' ' -f1)"
echo "SO | sha1 | $(sha1sum "$OUT/libpos_ffi.so" | cut -d' ' -f1)"
echo "APK | sha1 | $(sha1sum "$OUT/app-release-unsigned.apk" | cut -d' ' -f1)"
