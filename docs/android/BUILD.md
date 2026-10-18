# Android build pipeline

Reference for building, checking and reproducing the Android app of pili-pos (Sunmi D3 Pro,
Android 13, arm64). It covers the Rust library, the Gradle modules, the gates, the emulator
workflow, signing, reproducibility, dependency verification and CI. Decisions behind it:
[DECISIONS.md](DECISIONS.md) (section 5 pins, 9 signing, 11 build rules, PD76 to PD88). The
Kotlin facade is described in [FFI.md](FFI.md), the engine tests on the device in
[DEVICE_TESTS.md](DEVICE_TESTS.md), the reproducibility runs in [REPRO.md](REPRO.md).

Commands run from the repository root unless a `cd` says otherwise. Every shell needs rustup's
cargo first on `PATH` (Homebrew's cargo has no Android std, DECISIONS 11.4):

```sh
export PATH="$HOME/.cargo/bin:$PATH"; unset CARGO_TARGET_DIR
export ANDROID_HOME="$HOME/Library/Android/sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/27.1.12297006"
export JAVA_HOME="$(/usr/libexec/java_home -v 17)"
```

## 1. Layout

| Path | Content |
|---|---|
| `android/rust/` | Cargo workspace of its own (the root workspace excludes `android`): `pos-ffi` (UniFFI facade over `crates/pos-engine`) and `uniffi-bindgen`. Own `Cargo.lock`. Output goes to `target/android-rust` (`.cargo/config.toml`). |
| `android/pos-core/` | `:pos-core`, the Android library: generated Kotlin bindings, the arm64 `libpos_ffi.so`, consumer R8 rules, host unit tests, instrumented tests, `api/pos-core.api`. |
| `android/smoke/` | `:smoke`, a never-shipped test app (`dev.nuagemagique.pilipos.smoke`) that proves the consumer R8 rules on a minified release APK. It is the APK of the reproducibility check (PD80). |
| `android/docker/` | `Dockerfile` of the build image and `build_in_container.sh` (runs inside the image). |
| `android/gradle/` | `libs.versions.toml` (version catalog), `verification-metadata.xml`, the wrapper. |
| `android/config/detekt/` | detekt rules. |
| `scripts/android/` | Gates and tools (sections 5 to 9). |
| `docs/android/` | This file, DECISIONS, FFI, DEVICE_TESTS, REPRO, spikes, phase reports. |
| `target/` | Build output, not committed: `android-rust` (cargo-ndk and bindgen), `android-mocks` (host mocks), `android-rust-mismatch`, `android-device-tests`, `smoke`, `android-tests`. |

v1 submits Android only (INT-D41). The Android release APK inputs are frozen by the Android freeze tag
`android-v1.0.0-rc3`: `scripts/android/check_android_frozen.sh` must pass once the tag exists (exit 2
means no freeze tag exists yet; `--ref COMMIT` checks against any commit; `--list` prints the frozen
paths). The record is `ANDROID_FREEZE.md`. The Linux freeze (`check_linux_frozen.sh`,
`linux-v1.0.0-rc2`, `LINUX_FREEZE.md`) is history.

## 2. Toolchain pins

Authoritative table: DECISIONS section 5. A bump is a deliberate commit that changes the pin in
every place below and the table.

| Pin | Value | Where it lives |
|---|---|---|
| Rust | 1.99.0 with rustfmt, clippy, target `aarch64-linux-android` | `rust-toolchain.toml` (root, frozen), `android/docker/Dockerfile` (`RUST_VERSION`), `android.yml` |
| cargo-ndk | 4.1.2 (`cargo install --locked cargo-ndk --version 4.1.2`) | `Dockerfile` (`CARGO_NDK_VERSION`), `android.yml` |
| NDK | 27.1.12297006 (r27b) | `android.ndkVersion` in `android/pos-core/build.gradle.kts` and `android/smoke/build.gradle.kts`, `Dockerfile`, `android.yml` |
| platform for cargo-ndk | 33 (`--platform 33`, PD78) | `cargoNdkBuild` in `:pos-core`, `p2_gate.sh` (`ndk-build`) |
| uniffi | `=0.32.2` in `pos-ffi` and the bindgen binary | `android/rust/Cargo.toml`, `android/rust/Cargo.lock` |
| Gradle | 8.11.1 (sha256 in `distributionSha256Sum`) | `android/gradle/wrapper/gradle-wrapper.properties` |
| AGP / Kotlin / R8 | 8.9.1 / 2.2.20 / 8.13.25 (R8 forced through the root `buildscript` classpath) | `android/gradle/libs.versions.toml` |
| JNA | 5.19.1 (`@aar` for the APK, plain jar for host tests) | `libs.versions.toml` |
| compileSdk / targetSdk / minSdk | 36 / 36 / 33, ABI `arm64-v8a` only | module build files |
| JDK | 17 (container: Temurin 17.0.20.1) | `Dockerfile`, `android.yml` |
| Build-tools | 35.0.0 (`zipalign`, `apksigner`) | `Dockerfile`, `android.yml`, `check_apk.sh`, `sign_dev.sh` |
| Build image | `debian:bookworm-slim@sha256:...` digest, apt snapshot `20261001T000000Z`, rustup 1.29.1, cmdline-tools 13114758 | `android/docker/Dockerfile` |

## 3. The native library and its packaging

`:pos-core` builds `libpos_ffi.so` with the task `cargoNdkBuild`:

```sh
cargo ndk -t arm64-v8a --platform 33 -o <jniLibs dir> build --locked --profile android-release -p pos-ffi
```

- **Platform 33** (minSdk), not cargo-ndk's default 21 (PD78). The gate check `ndk-build` of
  `p2_gate.sh --android` runs the same command in `android/rust`, so both share one cargo cache
  in `target/android-rust`. Neither sets `RUSTFLAGS`; only the container build does (section 8).
- **Profile `android-release`**: `release` with `panic = "unwind"`, `opt-level = "z"`,
  `lto = true`, `codegen-units = 1`, `strip = "none"`, `debug = "line-tables-only"`. The link
  argument `-Wl,-z,max-page-size=16384` comes from `android/rust/.cargo/config.toml` (under
  `RUSTFLAGS` it is repeated by hand).
- **Unstripped cargo output** `target/android-rust/aarch64-linux-android/android-release/libpos_ffi.so`
  is the input of the bindings. The task `uniffiBindgen` runs `uniffi-bindgen generate --library`
  on it with `android/rust/android-uniffi.toml` (library mode, DECISIONS 11.6). Bindings are
  generated into `android/pos-core/build/generated/uniffi-android` and never committed.
- **AGP strips at packaging**, with the NDK of `ndkVersion`. cargo-ndk copies the unstripped
  library to `build/generated/cargo-ndk/jniLibs`; each variant gets its own copy as a generated
  jniLibs source (PD79). Debug and release package the same `android-release` library; there is
  no debug arm64 build.
- **16 KB pages**: every LOAD segment is aligned to `0x4000`, libraries are stored uncompressed
  and page aligned (`useLegacyPackaging = false`). `check_apk.sh` verifies it with
  `zipalign -c -P 16 -v 4` and `llvm-readelf`.
- **arm64-v8a only** (`abiFilters`): the AAR has `jni/arm64-v8a/libpos_ffi.so` and no other ABI.
- **JNA**: the APK gets `net.java.dev.jna:jna:5.19.1@aar` (carries `libjnidispatch.so`, 16 KB
  aligned). Host unit tests get the plain `jna:5.19.1` jar (host `libjnidispatch`) and run the
  same bindings against the host debug library `target/android-rust/debug/libpos_ffi.{dylib,so}`
  (PD77).
- **Consumer R8 rules** (`android/pos-core/consumer-rules.pro`): `-dontwarn java.awt.*`,
  `-keep class com.sun.jna.** { *; }`, `-keep class uniffi.pos_ffi.** { *; }`,
  `-keep interface * extends com.sun.jna.Library`. The `:smoke` app adds none of its own.

## 4. Gradle tasks of `:pos-core`

Run from `android/` with `./gradlew --no-daemon --console=plain <task>` (the gate never uses the
daemon). The Kotlin compiler runs in process (`kotlin.compiler.execution.strategy=in-process`),
the Gradle JVM gets `-Xmx3g`.

| Task | Does | Depends on |
|---|---|---|
| `cargoNdkBuild` | arm64 library with cargo-ndk (above) | rustup, `ANDROID_NDK_HOME` set by the task |
| `uniffiBindgen` | Kotlin bindings from the unstripped `.so` | `cargoNdkBuild` |
| `copyJniLibs<Variant>`, `copyUniffiBindings<Variant>` | wire the two outputs into a variant | the two tasks above |
| `cargoBuildHost` | `cargo build --locked -p pos-ffi` (host library) | only the unit tests use it |
| `cargoBuildMocks` | FDM and printer mocks from the root workspace into `target/android-mocks` (`--locked`, root lock untouched) | only the unit tests use it |
| `cargoBuildMismatch` | `pos-ffi` with feature `abi-mismatch` into `target/android-rust-mismatch` | `mismatchTest` |
| `testDebugUnitTest` | the JVM tests (at least 22) against the host library and the mocks | `cargoBuildHost`, `cargoBuildMocks` |
| `mismatchTest` | `ChecksumMismatchTest`: loading the mismatched library must fail (DECISIONS 11.5) | `cargoBuildMismatch`, test compilation |
| `apiCheck` / `apiDump` | compares / rewrites `api/pos-core.api`; accept a new dump only when the public API of the bindings is unchanged | bindings |
| `assembleRelease` | `pos-core-release.aar` | `cargoNdkBuild`, `uniffiBindgen` |
| `connectedDebugAndroidTest` | instrumented tests on the emulator (section 6) | `scripts/android/run_android_tests.sh` drives it |
| `check` | the usual checks plus `mismatchTest` | |

Project-wide: `lint`, `ktlintCheck`, `detekt` (also on `:smoke`). `:smoke:assembleRelease` builds
`android/smoke/build/outputs/apk/release/smoke-release-unsigned.apk` (minified, unsigned,
`vcsInfo` and `dependenciesInfo` off, build outputs without timestamps and in stable order).
`assembleRelease` of the AAR and of the APK never depend on the host build or the mocks.

## 5. The host gate

```sh
scripts/android/p3_gate.sh --fast --base android/int                       # card gate, no Gradle
scripts/android/p3_gate.sh --base android/int --allow '<regex of the card>' # full gate
```

It prints `GATE | <check> | PASS|FAIL|SKIP | <detail>` per check and one `GATE | total |
PASS|FAIL | ...` line; exit 1 on any failure. It runs every check of `p2_gate.sh` and, without
`--fast`, the Gradle checks.

Flags:

| Flag | Meaning |
|---|---|
| `--fast` | no self-tests, no Gradle (card gate) |
| `--base REF` | branch the diff checks compare with (default `android/int`) |
| `--allow REGEX` | every changed file must also match REGEX (the card's allowed paths) |
| `--android` | also build the arm64 library and check it (`ndk-build`, `so-align`, `bindgen-android`) |
| `--root` | also run the root test inventory (`test_inventory.sh --diff`, needs `ui/dist`) |
| `--exit` | phase exit: also `jvm-check` (`:pos-core:check`), `ffi-doc`, command parity `--strict` |
| `--emulator` | also `emulator_smoke.sh` and `run_android_tests.sh` (needs the emulator) |
| `--device-tests` | also `device_engine_tests.sh --check` (needs the emulator) |
| `--repro` | also `check_repro.sh` (two container builds, slow, needs Docker) |

Checks (from `p2_gate.sh` and `p3_gate.sh`):

| Check | What it enforces |
|---|---|
| `hygiene-paths` | changed files only under `android/`, `scripts/android/`, `docs/android/` (not `docs/android/baseline/`), plus `.github/workflows/android.yml`, and the `--allow` regex |
| `hygiene-emdash`, `hygiene-artifacts`, `hygiene-subjects`, `hygiene-root-lock` | no em-dash in added lines; no build output, key or local config tracked; Conventional Commit subjects; root `Cargo.toml` and `Cargo.lock` unchanged |
| `frozen-provisional`, `linux-frozen`, `android-frozen` | `frozen-provisional` is retired once a Linux tag exists; `linux-frozen` is retired (SKIP, INT-D44); `android-frozen` runs `check_android_frozen.sh` against the Android freeze tag (SKIP while there is none). `freeze-tag` (p4) follows `android-v1.0.0-rc3`; `--linux-repro` is Linux history, not part of the v1 freeze |
| `rust-fmt`, `rust-clippy`, `rust-tests` | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` in `android/rust` |
| `ffi-deps` | `pos-ffi` pulls in `pos-engine`, no `pos-peppol`, no tauri |
| `lock-parity` | `check_lock_parity.sh`: crates shared with the certified build keep the versions of the root `Cargo.lock` (exceptions with a reason in `lock_parity_allow.txt`) |
| `command-parity`, `ffi-hygiene` | `check_command_parity.py` (every cert command has a valid `PosEngine` method; `--strict` needs all 88) and `--hygiene` (rules H1 to H6) |
| `selftest-*` | self-tests of the gate tools |
| `gradle-lint`, `gradle-ktlint`, `gradle-detekt` | `lint`, `ktlintCheck`, `detekt` |
| `jvm-tests` | `:pos-core:test` (host JVM tests of the full p2 mode; SKIP before `android/settings.gradle.kts` exists) |
| `unit-tests` | `:pos-core:testDebugUnitTest`, at least 22 tests, none failed |
| `mismatch-test`, `api-check` | `:pos-core:mismatchTest`, `:pos-core:apiCheck` |
| `aar-check` | `:pos-core:assembleRelease`, then the AAR has `jni/arm64-v8a/libpos_ffi.so`, no other ABI, a `proguard.txt` with the four consumer rules |
| `assemble-release`, `apk-check` | `:smoke:assembleRelease`, then `check_apk.sh` on the unsigned APK |

`check_apk.sh APK` checks `zipalign -c -P 16 -v 4`, only `lib/arm64-v8a/`, `libpos_ffi.so` and
`libjnidispatch.so` present and stored, LOAD alignment `0x4000`, no `.debug_*` section in
`libpos_ffi.so` and a size under 20 MiB, then prints the APK sha1 and sha256 as INFO lines.

When to use the slow flags: a card runs `--fast` on every commit and the full gate once at the
end. The card that owns an emulator or Docker script runs the matching flag (`--emulator`,
`--device-tests`, `--repro`). The phase exit runs the full gate with `--android --root --exit
--emulator --device-tests --repro`.

## 6. Emulator workflow

The validation device is the AVD `sunmi_d3_pro_api33` (Sunmi D3 Pro profile, 1920x1080
landscape, 160 dpi, 4 GB, API 33, arm64, `google_apis`, no Play). Only one emulator runs at a
time (16 GB host).

```sh
scripts/android/emulator.sh ensure   # reuse the running one or start headless; prints the serial
scripts/android/emulator.sh status   # serial (or none), avd, started-by-chain (yes/no)
scripts/android/emulator.sh stop     # integrator only
```

Rules (PD81): `ensure` never starts a second emulator (exit 3 when another AVD runs) and never
wipes or provisions the device. Workers and card scripts never stop it. Only the integrator runs
`stop`, between waves and at the phase end, and `stop` acts only on an emulator the chain
started (it leaves any other alone). State lives in `~/.cache/pili-android/emulator/`.

The device reaches host mocks through `adb reverse` on fixed device ports, so two scripts can
share the emulator. Each script removes only its own reverse rules and packages. The mocks listen
on free host ports; the raw emulator alias for the host is `10.0.2.2`.

| Script | FDM mock | Printer ESC/POS | Printer HTTP jobs |
|---|---|---|---|
| `emulator_smoke.sh` | 19180 | 19190 | none |
| `run_android_tests.sh` | 19280 | 19290 | 19291 |
| `device_engine_tests.sh` | none: the tests serve their mocks in-process | none | none |

The three scripts:

```sh
scripts/android/emulator_smoke.sh [--no-build]   # SMOKE | <check> | PASS|FAIL lines, one total line
scripts/android/run_android_tests.sh             # ANDROIDTEST | <class> | PASS|FAIL, one total line
scripts/android/device_engine_tests.sh --check   # DEVICE | <crate>/<target> | PASS|FAIL, one total line
```

- **Smoke run**: builds the minified `:smoke` release APK, runs `check_apk.sh`, signs a copy with
  the development key (`target/smoke/smoke-release-dev-signed.apk`), installs it, runs the
  scripted engine scenario against the host FDM and printer mocks and reads the result from
  logcat. It proves the engine and the consumer R8 rules.
- **Instrumented tests**: `:pos-core:connectedDebugAndroidTest` (`DeviceLoadTest`,
  `DeviceLifecycleTest`, `DeviceEndToEndTest`). Passes only with at least 12 tests, none failed or
  skipped. Mock logs go to `target/android-tests/`. The seed SQL comes from
  `src-tauri/seed_spf_test.sql` as a generated asset, never copied into the tree.
- **Engine tests on the device** (PD82, DECISIONS 11.12): cross-builds the tests of `pos-engine`
  (feature `cert`) and the other crates of the cert build graph into `target/android-device-tests`,
  pushes each binary to `/data/local/tmp/pili-p3-device-tests`, runs it through adb and compares
  the result with `scripts/android/device_tests_exclude.txt`. Exclusion categories:
  `host-fixture` (the test fails only because it reads a file of the host checkout through
  `CARGO_MANIFEST_DIR`; `--fixtures` runs them all with their fixtures) and `android-difference`
  (any other reproducible failure: reported as WARN, listed under Engine questions in
  DEVICE_TESTS.md, never fixed under `crates/`). A stale exclusion fails the check. Other options:
  `--build-only`, `--run-only`, `--report-only`, `--crates LIST`, `--keep`, `--host-list`. Results:
  [DEVICE_TESTS.md](DEVICE_TESTS.md).

## 7. Signing

- Release APKs are built **unsigned** (`smoke-release-unsigned.apk`). No signing config exists in
  any build file and no key ever enters the build or the container (DECISIONS 9).
- `scripts/android/sign_dev.sh UNSIGNED_APK OUT_APK` signs a copy with a throwaway RSA-2048
  development key, created on first use under `~/.cache/pili-android/dev-keystore/`
  (`pili-dev.p12`, `pili-dev.pass`, directory mode 700). It refuses to run when that directory lies
  in a git work tree. Schemes: v1 off, v2 and v3 on; `apksigner verify` runs afterwards. It exists
  so the emulator accepts the APK and is never used for a release.
- The **release key** is created and kept by the user (DECISIONS 9: RSA, v2 and v3, offline
  custody, not in the repo, CI or a container). Signing a release is a later step on the
  producer's machine with the pinned build-tools `apksigner`; the reproducibility check compares
  the unsigned bytes.

## 8. Reproducibility

Goal (DECISIONS 11.13, PD83): the unsigned `:smoke` release APK is byte-identical across two
builds in fresh containers without network.

```sh
scripts/android/build_image.sh [--no-cache]      # builds pili-android-build:p3 (linux/amd64), local only
scripts/android/check_repro.sh [--commit REF] [--keep-work] [--clean-prefetch]
```

- **Image**: `build_image.sh` builds `android/docker/Dockerfile` as `pili-android-build:p3`
  with the label `pili.dockerfile.sha256` and prints `IMAGE | id`, `IMAGE | dockerfile`, `IMAGE |
  size`. It needs 20 GB free (`PILI_MIN_FREE_GB`). The image is never pushed (a registry push
  needs the user).
- **Build script** `android/docker/build_in_container.sh` runs inside the image: extracts
  `/in/src.tar` to the fixed path `/build/src`, sets every mtime to `SOURCE_DATE_EPOCH` (the
  commit time), exports `RUSTFLAGS` with `--remap-path-prefix` for source, `CARGO_HOME`,
  `RUSTUP_HOME` and the target dir plus the page-size link argument, `CFLAGS_aarch64_linux_android`
  with `-ffile-prefix-map`, `CARGO_INCREMENTAL=0`, then runs `./gradlew --no-daemon
  --no-build-cache --no-configuration-cache :smoke:assembleRelease`. It fails when a `Cargo.lock`
  changes. Output in `/out`: the APK, the unstripped `libpos_ffi.so`, `mapping.txt`, `gradle.log`;
  last lines `APK | sha256`, `SO | sha1`, `APK | sha1`. Options: `--offline`, `--write-metadata`.
- **check_repro.sh** prefetches crates and Gradle artifacts once, with the network on, into the
  Docker volume `pili-android-prefetch-<key>`, then runs two `docker run --network none` builds
  of `:smoke:assembleRelease` from a `git archive` of the commit and compares the two APKs byte
  for byte. It prints `REPRO` lines. `--commit REF` picks the commit (default `HEAD`),
  `--keep-work` keeps the work directories, `--clean-prefetch` removes the prefetch volume.
  Results and the image id and Dockerfile sha256 of every run go to [REPRO.md](REPRO.md).
- **Gate flag**: `scripts/android/p3_gate.sh --repro` runs it as the `apk-repro` check.
- **Native x86_64 run**: the Docker VM on the Apple M1 runs amd64 under emulation. The run on a
  native x86_64 Linux host is left to the user: manual run of the `repro` job of `android.yml`
  (section 10).

## 9. Gradle dependency verification

`android/gradle/verification-metadata.xml` holds sha256 checksums for every resolved artifact
(`verify-metadata` on, signatures off). It must cover the host (macOS, `aapt2` osx) and the
container (Linux, `aapt2` linux). A dependency add or bump changes the catalog and the metadata
in the same commit:

1. Edit `android/gradle/libs.versions.toml` (and the build file if needed). For a pinned tool
   change DECISIONS section 5 too.
2. Write the host entries, with the task list of the gate:

   ```sh
   cd android && ./gradlew --no-daemon --console=plain --write-verification-metadata sha256 \
     lint ktlintCheck detekt :pos-core:testDebugUnitTest :pos-core:mismatchTest :pos-core:apiCheck \
     :pos-core:assembleRelease :pos-core:assembleDebugAndroidTest :smoke:assembleRelease --continue
   ```

3. Merge the Linux entries from the build image (needs Docker and the image from section 8):

   ```sh
   git add -A    # stage new files; the archive below contains the staged state
   W="$(mktemp -d)"; mkdir "$W/out"; REF="$(git stash create)"
   git archive --format=tar -o "$W/src.tar" "${REF:-HEAD}"
   docker run --rm --platform linux/amd64 -e SOURCE_DATE_EPOCH="$(git log -1 --format=%ct)" \
     -v "$W:/in:ro" -v "$W/out:/out" pili-android-build:p3 \
     bash -c 'mkdir -p /build/src && tar -xf /in/src.tar -C /build/src \
              && exec bash /build/src/android/docker/build_in_container.sh "$@"' _ --write-metadata
   cp "$W/out/verification-metadata.xml" android/gradle/verification-metadata.xml
   ```

   `git stash create` prints nothing when the tree is clean, so `${REF:-HEAD}` archives `HEAD`.
   Run the container with the network on (no `--network none`): it downloads the Linux artifacts.
   An AGP bump also changes the artifacts that `connectedDebugAndroidTest` resolves (the
   `com.android.tools.utp` family), which the task list above does not always reach. After such a
   bump run one more metadata pass with the emulator up
   (`scripts/android/emulator.sh ensure`), adding `:pos-core:connectedDebugAndroidTest` to the
   task list, and confirm `scripts/android/run_android_tests.sh` still passes dependency
   verification.
4. Check the diff is only what you expected (new `<component>` entries; no `trusted-artifacts`,
   no `ignored-keys`), then re-run the host gate and, when the dependencies of the APK changed,
   `scripts/android/check_repro.sh`.
5. Commit the catalog change and `verification-metadata.xml` together
   (`build(android): ...`).

A `Dependency verification failed` error means the artifact is missing from the file or its
checksum changed: never delete the entry to silence it; find out why the bytes changed.

## 10. CI

`.github/workflows/android.yml` (name `Android`) is separate from the Linux `test.yml`, which
ignores `android/**` and `docs/android/**`. Triggers (PD84), and nothing else: `workflow_dispatch`
(boolean input `repro`, default false) and a push of a tag `android-v*`. No branch push, no
pull request, no path trigger. The chain writes and checks the file locally and never pushes it or
runs it; a run is the user's decision. It uses no secret, signs nothing and pushes no image.
Permissions: `contents: read`; one concurrency group per workflow and ref.

| Job | Runs | Steps |
|---|---|---|
| `gate` | always (on dispatch and tag) | JDK 17 (Temurin), Android `platforms;android-36`, `build-tools;35.0.0`, `ndk;27.1.12297006`, Rust 1.99.0 with rustfmt, clippy and `aarch64-linux-android`, cargo-ndk 4.1.2, Gradle setup; then in `android/rust` `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`; `check_command_parity.py --strict` and `--hygiene`; `check_lock_parity.sh`; `./gradlew --no-daemon --console=plain lint ktlintCheck detekt :pos-core:testDebugUnitTest :pos-core:mismatchTest :pos-core:apiCheck :pos-core:assembleRelease :smoke:assembleRelease --continue`; `check_apk.sh` on the unsigned smoke APK; reports and test results uploaded (14 days, `if: always()`) |
| `repro` | needs `gate`; only when input `repro` is true or the ref is an `android-v*` tag | `scripts/android/check_repro.sh` (builds the image locally with Docker), uploads its output and the image id, commit and Dockerfile sha256 as artifact `android-repro` |

There is no emulator job: the instrumented tests, the smoke run and the engine tests on the
device need the `sunmi_d3_pro_api33` AVD and stay local (section 6). The first manual run of the
`repro` job on `ubuntu-latest` (native x86_64) is the run DECISIONS 11.13 leaves to the user;
record its result in REPRO.md.

## 11. Housekeeping

- **No `./gradlew --stop` during parallel work.** Every gate runs `--no-daemon`, so there is
  nothing to stop; the stop command would kill a daemon of another worker. Only the integrator,
  between waves, may run it.
- **One heavy build at a time.** Gradle gets `-Xmx3g`; the Kotlin compiler runs in process so no
  Kotlin daemon lingers. Keep `CARGO_BUILD_JOBS` at 3 or less and `CARGO_INCREMENTAL=0`.
- **RAM budget (16 GB)**: the emulator takes about 4 GB, the Docker VM up to 12 GB. The emulator
  is stopped (by the integrator) before the Docker work; do not run both.
- **Disk floors**: `build_image.sh` needs 20 GB free; stop a chain run under 15 GB.
- **Cargo output** stays under the worktree `target/` (never `CARGO_TARGET_DIR`). A fresh worktree
  can start warm with `cp -cR <other>/target/android-rust target/` and the same for
  `android-mocks` (APFS clone, no extra space).
- **What to delete when space is short** (all regenerable): `target/android-device-tests` and
  `target/android-device-tests-fx` (3.7 GB each), `target/android-rust-mismatch`,
  `target/smoke`, `target/android-tests`, `android/*/build`, `android/.gradle`, leftover work
  directories of `check_repro.sh --keep-work`.
- **Docker hygiene**: `check_repro.sh` leaves the prefetch volume `pili-android-prefetch-<key>` so
  the next run is fast; list with `docker volume ls`, remove with `check_repro.sh
  --clean-prefetch` or `docker volume rm`. Clean stale layers with `docker builder prune` and
  `docker system df`. Never `docker system prune -a` while an image is still needed.
- **Leftover processes**: a crashed test JVM or script can leave a mock (`pos-fdm-mock`,
  `pos-printer-mock`) running; check with `pgrep -fl pos-.*-mock` and kill only your own.
- **Dev signing key** lives outside the repo (`~/.cache/pili-android/dev-keystore/`); never copy
  it into the tree and never use it for a release.
