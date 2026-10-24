# android/rust

The Rust side of the Android port: a Cargo workspace of its own (the repo-root workspace lists
`android` in `exclude`). It holds `pos-ffi`, the UniFFI facade over `crates/pos-engine`, and the
`uniffi-bindgen` binary. Build output goes to `../../target/android-rust` (see `.cargo/config.toml`).

The Gradle and Android pipeline (modules, gates, emulator, signing, reproducibility, CI) is described in
[`docs/android/BUILD.md`](../../docs/android/BUILD.md).

Use the pinned toolchain (`rust-toolchain.toml` at the repo root) and, with Homebrew's cargo
installed, put rustup first: `export PATH="$HOME/.cargo/bin:$PATH"`.

## Commands

All from this directory.

```sh
# Host checks and tests (the integration tests start the FDM and printer mocks in-process)
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked

# The Android library (arm64, 16 KB pages, unwinding panics)
export ANDROID_HOME=$HOME/Library/Android/sdk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/27.1.12297006
cargo ndk -t arm64-v8a build --locked --profile android-release -p pos-ffi
# -> ../../target/android-rust/aarch64-linux-android/android-release/libpos_ffi.so
#    (check: llvm-readelf -lW <so> | awk '/LOAD/ {print $NF}' prints 0x4000)

# Kotlin bindings, library mode, from the exact unstripped .so that gets packaged
cargo run --locked -p uniffi-bindgen -- generate \
  --library ../../target/android-rust/aarch64-linux-android/android-release/libpos_ffi.so \
  --language kotlin --config android-uniffi.toml --out-dir /tmp/pos-ffi-android --no-format

# Kotlin bindings for the JVM host tests (host library, no Android flags)
cargo build --locked -p pos-ffi
cargo run --locked -p uniffi-bindgen -- generate \
  --library ../../target/android-rust/debug/libpos_ffi.dylib \
  --language kotlin --config jvm-uniffi.toml --out-dir /tmp/pos-ffi-jvm --no-format
```

## Layout

- `pos-ffi/src/core.rs`: `PosEngine` (open, shutdown, events) and the `blocking` / `spawn` helpers
  every facade uses.
- `pos-ffi/src/error.rs`, `convert.rs`, `events.rs`: `PosError`, serde conversion, the event stream.
- `pos-ffi/src/logging.rs`: the only file with `cfg(target_os)` and process-wide state (logger,
  panic hook).
- `pos-ffi/src/api/*`, `records/*`, `helpers.rs`, `install.rs`: filled by the domain cards.
- `pos-ffi/tests/ffi/`: the one integration test binary, one module per domain; `support.rs`
  opens an engine on a seeded temp database with the mocks.

## Rules

- `Cargo.toml` and `Cargo.lock` change only deliberately (PD59): every dependency the domain cards
  need is already declared in `pos-ffi/Cargo.toml`, so a card adds sources and tests only.
- `Cargo.lock` started as a copy of the root lock, so the shared crates keep their root versions.
  Check after any change: `cargo tree -p pos-engine -e normal,build --target all --prefix none`
  (names and versions) must equal the root's
  `cargo tree -p pos-engine --no-default-features --features cert ...`.
- Never edit the root `Cargo.toml`, `Cargo.lock`, `crates/`, `src-tauri/` or `ui/` from here.
- No `cfg(target_os)` and no global state outside `logging.rs`.
