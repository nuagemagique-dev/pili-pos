# The real binary offline against the HubRise, FDM and printer mocks
# (scripts/e2e_hubrise.sh): WebKitGTK + Xvfb + tauri-driver, as the CI
# integration job, and Node for the node:test suite.
FROM rust:1-bookworm
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends \
      libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
      libssl-dev pkg-config xvfb webkit2gtk-driver sqlite3 fonts-noto-core fonts-noto-cjk \
      curl ca-certificates xz-utils && rm -rf /var/lib/apt/lists/*
ARG NODE_VERSION=24.21.0
RUN arch=$(dpkg --print-architecture | sed 's/amd64/x64/') && \
    curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-${arch}.tar.xz" \
      | tar -xJ -C /usr/local --strip-components=1 && node --version
RUN rustup toolchain install 1.99.0 --profile minimal && \
    cargo +1.99.0 install tauri-driver --version 2.0.6 --locked
