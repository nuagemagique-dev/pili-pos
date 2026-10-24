#
# Build image of the Android APK (P3, DECISIONS 11.13, PD83): linux/amd64, every input pinned.
# Base image by digest, apt by snapshot.debian.org date, every download by checksum (sha256, or
# the sha1 that the Google repository manifest publishes and sdkmanager checks itself).
# Derived from spike S5 (docs/android/spikes/S5.md); the Rust toolchain also carries the
# components that rust-toolchain.toml names (rustfmt, clippy), so rustup never needs the network
# at build time.
#
#   scripts/android/build_image.sh            # builds pili-android-build:p3
#
# The image holds no signing key and no source. The APK it builds stays unsigned.

# debian:bookworm-slim (Debian 12.15), multi-arch index digest checked 2026-10-06.
FROM --platform=linux/amd64 debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587

# ---- pinned versions and checksums -------------------------------------------------------------
# Debian snapshot date: every apt package version is fixed by this date.
ARG DEBIAN_SNAPSHOT=20261001T000000Z
# JDK 17: Eclipse Temurin tarball.
ARG JDK_VERSION=17.0.20.1+1
ARG JDK_FILE=OpenJDK17U-jdk_x64_linux_hotspot_17.0.20.1_1.tar.gz
ARG JDK_SHA256=3808d1d15e3ec6bd5b84057fb5d84c33d8a1536a258146bcea2e603fc726e08e
# Android command-line tools 19.0 (build 13114758). sha1 is the value in repository2-3.xml.
ARG CMDLINE_TOOLS_BUILD=13114758
ARG CMDLINE_TOOLS_SHA256=7ec965280a073311c339e571cd5de778b9975026cfcbe79f2b1cdcb1e15317ee
ARG CMDLINE_TOOLS_SHA1=5fdcc763663eefb86a5b8879697aa6088b041e70
# SDK packages. sdkmanager verifies the sha1 of each archive against repository2-3.xml:
#   build-tools_r35_linux.zip     2cfaa0bbb2336e9ec18ed3ecea84fa2e2af607bc
#   platform-36_r02.zip           2c1a80dd4d9f7d0e6dd336ec603d9b5c55a6f576
#   android-ndk-r27b-linux.zip    6fc476b2e57d7c01ac0c95817746b927035b9749
ARG BUILD_TOOLS_VERSION=35.0.0
ARG PLATFORM_VERSION=android-36
ARG NDK_VERSION=27.1.12297006
# Rust: rustup 1.29.1 (sha256 from static.rust-lang.org/rustup/archive/1.29.1/.../rustup-init.sha256).
ARG RUSTUP_VERSION=1.29.1
ARG RUSTUP_INIT_SHA256=dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71
ARG RUST_VERSION=1.99.0
ARG CARGO_NDK_VERSION=4.1.2

# ---- fixed environment ---------------------------------------------------------------------------
# No RUSTUP_TOOLCHAIN: the repository's rust-toolchain.toml (channel 1.99.0) selects the toolchain.
ENV LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    TZ=UTC \
    JAVA_HOME=/opt/jdk17 \
    ANDROID_HOME=/opt/android-sdk \
    ANDROID_SDK_ROOT=/opt/android-sdk \
    ANDROID_NDK_HOME=/opt/android-sdk/ndk/27.1.12297006 \
    RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:/opt/jdk17/bin:/opt/android-sdk/cmdline-tools/19.0/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# ---- apt: pinned by snapshot date -----------------------------------------------------------------
# The slim image has no ca-certificates yet, so apt talks plain http; package integrity is
# enforced by apt's signature check against debian-archive-keyring (Valid-Until is disabled
# because snapshot Release files expire). No python3: the in-container script needs only bash,
# tar, find and coreutils (check_apk.sh and the other python gates run on the host).
RUN rm -f /etc/apt/sources.list.d/* /etc/apt/sources.list \
 && printf 'deb [check-valid-until=no] http://snapshot.debian.org/archive/debian/%s/ bookworm main\n' "${DEBIAN_SNAPSHOT}" > /etc/apt/sources.list \
 && printf 'deb [check-valid-until=no] http://snapshot.debian.org/archive/debian-security/%s/ bookworm-security main\n' "${DEBIAN_SNAPSHOT}" >> /etc/apt/sources.list \
 && printf 'Acquire::Check-Valid-Until "false";\nAcquire::Retries "5";\n' > /etc/apt/apt.conf.d/99snapshot \
 && apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      ca-certificates curl unzip build-essential \
 && rm -rf /var/lib/apt/lists/*

# ---- JDK 17 (Temurin tarball, sha256 verified) -----------------------------------------------------
RUN mkdir -p /opt/jdk17 \
 && curl -fsSL -o /tmp/jdk.tgz "https://github.com/adoptium/temurin17-binaries/releases/download/jdk-${JDK_VERSION/+/%2B}/${JDK_FILE}" \
 && echo "${JDK_SHA256}  /tmp/jdk.tgz" | sha256sum -c - \
 && tar -xzf /tmp/jdk.tgz -C /opt/jdk17 --strip-components=1 \
 && rm /tmp/jdk.tgz \
 && java -version

# ---- Android command-line tools (sha256 + sha1 verified) -------------------------------------------
RUN curl -fsSL -o /tmp/cmdline.zip "https://dl.google.com/android/repository/commandlinetools-linux-${CMDLINE_TOOLS_BUILD}_latest.zip" \
 && echo "${CMDLINE_TOOLS_SHA256}  /tmp/cmdline.zip" | sha256sum -c - \
 && echo "${CMDLINE_TOOLS_SHA1}  /tmp/cmdline.zip" | sha1sum -c - \
 && mkdir -p /opt/android-sdk/cmdline-tools /tmp/cmdline \
 && unzip -q /tmp/cmdline.zip -d /tmp/cmdline \
 && mv /tmp/cmdline/cmdline-tools /opt/android-sdk/cmdline-tools/19.0 \
 && rm -rf /tmp/cmdline /tmp/cmdline.zip

# ---- SDK packages at exact versions ------------------------------------------------------------------
# build-tools and the platform come from sdkmanager (it checks the sha1 of each archive against
# repository2-3.xml). Process substitution instead of a pipe: `yes | ...` ends with SIGPIPE,
# exit 141, under pipefail.
RUN sdkmanager --sdk_root="${ANDROID_HOME}" --licenses < <(yes) >/dev/null \
 && sdkmanager --sdk_root="${ANDROID_HOME}" \
      "build-tools;${BUILD_TOOLS_VERSION}" \
      "platforms;${PLATFORM_VERSION}" \
 && rm -rf /root/.android

# NDK r27b, installed by sdkmanager in its own layer (it is the slowest download and unzip).
RUN sdkmanager --sdk_root="${ANDROID_HOME}" "ndk;${NDK_VERSION}" \
 && grep -E 'Pkg.Revision|Pkg.ReleaseName' "${ANDROID_NDK_HOME}/source.properties" \
 && sdkmanager --sdk_root="${ANDROID_HOME}" --list_installed \
 && rm -rf /root/.android

# ---- Rust 1.99.0 (rustup-init sha256 verified) + cargo-ndk 4.1.2 ----------------------------------
# rustfmt and clippy are the components rust-toolchain.toml names; without them rustup would
# download them at build time, which is impossible offline. aarch64-linux-android is the target.
RUN curl -fsSL -o /tmp/rustup-init "https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/x86_64-unknown-linux-gnu/rustup-init" \
 && echo "${RUSTUP_INIT_SHA256}  /tmp/rustup-init" | sha256sum -c - \
 && chmod +x /tmp/rustup-init \
 && /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain none \
 && rm /tmp/rustup-init \
 && rustup toolchain install "${RUST_VERSION}" --profile minimal \
      --component rustfmt --component clippy --target aarch64-linux-android \
 && rustup default "${RUST_VERSION}" \
 && cargo install --locked --version "${CARGO_NDK_VERSION}" cargo-ndk \
 && rm -rf /opt/cargo/registry /opt/cargo/git \
 && ln -s /opt/cargo /root/.cargo \
 && rustc --version && cargo --version && cargo ndk --version

# Debian's /etc/profile resets PATH for login shells (bash -l), which would hide the toolchain.
# A profile.d file puts the pinned PATH back, so `bash -lc` sees the same tools as `bash -c`.
RUN printf 'export PATH=/opt/cargo/bin:/opt/jdk17/bin:/opt/android-sdk/cmdline-tools/19.0/bin:$PATH\n' > /etc/profile.d/10-pili-toolchain.sh

# Record what is installed (read by the report).
RUN { \
      echo "debian: $(cat /etc/debian_version)"; \
      echo "jdk: $(java -version 2>&1 | head -1)"; \
      echo "rustc: $(rustc --version)"; \
      echo "cargo-ndk: $(cargo ndk --version)"; \
      echo "components: $(rustup component list --installed | tr '\n' ' ')"; \
      echo "targets: $(rustup target list --installed | tr '\n' ' ')"; \
      for f in /opt/android-sdk/build-tools/*/source.properties /opt/android-sdk/platforms/*/source.properties /opt/android-sdk/ndk/*/source.properties; do \
        echo "$f: $(grep -h -E '^Pkg.Revision|^Pkg.ReleaseName' "$f" | tr -d ' ' | tr '\n' ' ')"; \
      done; \
      dpkg-query -W -f='${Package}=${Version}\n' | sort; \
    } > /opt/pili-installed.txt

WORKDIR /build
