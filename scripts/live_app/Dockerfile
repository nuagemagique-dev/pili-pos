FROM rust:1-bookworm
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends \
      libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
      libssl-dev pkg-config xvfb webkit2gtk-driver sqlite3 python3 fonts-noto-core \
      fonts-noto-cjk imagemagick dnsutils curl ca-certificates && rm -rf /var/lib/apt/lists/*
RUN rustup toolchain install 1.99.0 --profile minimal && \
    cargo +1.99.0 install tauri-driver --version 2.0.6 --locked
