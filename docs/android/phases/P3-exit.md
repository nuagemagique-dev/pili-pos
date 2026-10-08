# P3 exit report: Android build pipeline

Written by the P3 planner on 2026-10-08, at the end of the P3 run.

- **Base:** `android/int` at `ac15866` (`docs(android): P2 phase cards, exit report and exit decisions`).
- **Exit head:** `android/int` at `6ae8174` (`chore(android): merge P3-T9`) before the exit commit, then the commit that adds `phases/P3.md`, this report and the exit decisions (PD89 to PD98).
- **Cards:** `docs/android/phases/P3.md`. Decisions: `docs/android/DECISIONS.md`, "Decided while the user was away (review)" (PD76 to PD88 from the planner, copied by P3-T1; PD89 to PD98 at this exit). New references: `docs/android/BUILD.md` (build pipeline), `docs/android/DEVICE_TESTS.md` (engine tests on the emulator), `docs/android/REPRO.md` (APK reproducibility).
- main (`96728e1`) and `cert/v1.0` (`d6210ca`) were not touched. Nothing was pushed, no GitHub Actions run, no live module2, HubRise, Scaleway or SPF call, no `declare.sh`, no release key (only the throwaway development key under `~/.cache/pili-android/dev-keystore/`), no tag.

## Verdict

- **All nine cards are APPROVE and merged** into `android/int` with `--no-ff`, one merge per card, in the planned order, no conflicts, no integration fix commit, no rework round, no escalation.
- **`android/int` builds and the phase gate passes.** The planner reran the exit gate itself at `6ae8174`: `p3_gate.sh --exit --android` gives **0 failed of 34 checks**, a forced rerun of the host unit tests, `mismatchTest` and `apiCheck` passes (22 + 1 tests), and `p3_gate.sh --fast --emulator --device-tests` on `sunmi_d3_pro_api33` gives **0 failed of 17 checks**. The integrator's runs at the same commit cover `--exit --root --android` (0 failed of 35, root inventory 0 removed) and `--repro` (two byte-identical container builds). The exit commit adds documentation only.
- **The engine runs on Android** (the plan's P3 exit condition). On the arm64 API 33 emulator: the minified release `:smoke` app runs open, login, FDM status, a signed sale, a Z report and shutdown through the R8-processed bindings (11 checks); 19 instrumented tests of `:pos-core` pass against the host mocks through `adb reverse`; the engine's own tests (cert build graph, 88 binaries) pass 1178 tests, and the 193 that fail fail only because they read files of the host checkout (all `host-fixture`, 0 `android-difference`, so no engine question for the user).
- **Reproducible unsigned release APK.** Two `--network none` builds in the digest-pinned linux/amd64 image give the same bytes, SHA-1 `73cc27ef2d41732abdf424d713f11cf2d9589cd0`.
- **The Linux build inputs are untouched.** `git diff 042d911..6ae8174` on the 16 freeze paths is empty; P3 changed only `android/`, `scripts/android/`, `docs/android/` (not `docs/android/baseline/`) and the new `.github/workflows/android.yml`; the root `Cargo.toml` and `Cargo.lock` are unchanged.
- **No freeze step by this exit** (PD86, PD97). The plan's hard freeze (`linux-v1.1.0`, signed by the user) needs the user. No `linux-*` tag exists. A separate branch, `android/freeze-rc1`, carries the user's decisions on PD47 and PD71 and a soft freeze record waiting for its review and tag; it is not merged into `android/int` (see "Freeze state").

**Planner recommendation:** P4 can start on `android/int` at the exit commit. The orchestrator should first settle the soft freeze from `android/freeze-rc1` (review, local tag, merge into `android/int`), because once the tag exists the tag-mode freeze check fails on an `android/int` without those commits. Merging into main, the hard freeze, the image push and the native x86_64 run wait for the user.

## Task status

| Card | Wave | Branch tip | Worker | Review | Merge on `android/int` |
|------|------|------------|--------|--------|------------------------|
| P3-T1 gate tools (`p3_gate.sh`, `check_apk.sh`, `emulator.sh`, `p2_gate.sh` platform 33 and `--extra-paths`, PD76 to PD88) | 1 | `android/P3-P3-T1` `4432561` | done | APPROVE | `c92e015` |
| P3-T2 `:pos-core` as an Android library | 1 | `android/P3-P3-T2` `2a12d44` | done | APPROVE | `c75a73a` |
| P3-T3 pos-ffi shutdown after a cancelled shutdown, `abi-mismatch` all-targets | 1 | `android/P3-P3-T3` `548ab0d` | done | APPROVE | `03fcd92` |
| P3-T4 `:smoke`, `sign_dev.sh`, `emulator_smoke.sh`, archive determinism | 2 | `android/P3-P3-T4` `edc71e0` | done | APPROVE | `62b7ab5` |
| P3-T5 engine tests on the emulator | 2 | `android/P3-P3-T5` `eed018d` | done | APPROVE | `99ea08e` |
| P3-T6 instrumented tests, `run_android_tests.sh` | 3 | `android/P3-P3-T6` `153ff4a` | done | APPROVE | `7ec2789` |
| P3-T7 build image, `build_in_container.sh`, dependency verification | 4 | `android/P3-P3-T7` `61acd13` | done | APPROVE | `623c90b` |
| P3-T8 `check_repro.sh`, `apk_entry_diff.py`, `REPRO.md` | 5 | `android/P3-P3-T8` `7b9fbd5` | done | APPROVE | `62f22f1` |
| P3-T9 `android.yml`, `BUILD.md` | 5 | `android/P3-P3-T9` `93fad08` | done | APPROVE | `6ae8174` |

The card branches are kept locally; all nine P3 task worktrees are removed.

### What each card delivered (from the reviews)

- **T1.** `p3_gate.sh` (runs every `p2_gate.sh` check, adds the Gradle, APK, emulator, device-test and repro checks, one total line), `check_apk.sh` (zipalign `-P 16`, arm64 only, stored libraries, 0x4000 LOAD alignment, stripped, under 20 MiB), `emulator.sh` (`ensure`, `status`, `stop` only for an emulator the chain started, exit 3 for another AVD), `p2_gate.sh` `ndk-build` at `--platform 33` (API field 0x21 checked) and `--extra-paths`; self-tests 12 and 17 cases.
- **T2.** `:pos-core` is an AGP library: one shared `cargoNdkBuild` (`cargo ndk -t arm64-v8a --platform 33 ... --profile android-release -p pos-ffi`, no RUSTFLAGS) and `uniffiBindgen` (library mode on the unstripped `.so`), per-variant copy tasks, JNA `@aar` for the APK and the plain jar for unit tests, the four consumer R8 rules, `useLegacyPackaging = false`, lint, ktlint 12.2.0, detekt 1.23.7, catalog entries for later cards, the PD73 test fixes (mock shutdown hook, `MocksSmokeTest` cleanup, `rustInputs` with the root `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`). `pos-core.api` unchanged.
- **T3.** `shutdown_gate` holds the drain `JoinHandle`, so a shutdown after a cancelled shutdown awaits the same drain, releases the engine and ends the streams; two new tests (both fail on the old code); the `abi-mismatch` feature builds all targets (PD90).
- **T4.** `android/smoke` (`dev.nuagemagique.pilipos.smoke`, minified release, unsigned, no keep rule of its own), `sign_dev.sh` (throwaway RSA-2048 key outside the repo, v2 and v3 signatures, refuses to write inside a git work tree), `emulator_smoke.sh` (mocks on free host ports, device ports 19180 and 19190, cleans its own package and reverse rules), archive determinism (`isPreserveFileTimestamps = false`, reproducible file order). R8 mapping shows the bindings and JNA keep their names while app code is obfuscated.
- **T5.** `device_engine_tests.sh` cross-builds the tests of the cert build graph from the root workspace (`--locked`, `Cargo.lock` checked unchanged) with the NDK clang and runs them under `/data/local/tmp`, `--check` verdict against `device_tests_exclude.txt` (193 entries, exact names, all `host-fixture`), opt-in `--fixtures`; `DEVICE_TESTS.md` with per-binary figures.
- **T6.** `DeviceEndToEndTest` (8), `DeviceLifecycleTest` (7), `DeviceLoadTest` (4) under `src/androidTest`, seed SQL copied at build time (not committed), `run_android_tests.sh` (device ports 19280, 19290, 19291); without the mocks the end-to-end tests fail loudly, never skip.
- **T7.** `android/docker/Dockerfile` (base image by digest, Debian snapshot `20261001T000000Z`, every download checked by sha256 or sha1, Rust 1.99.0 with rustfmt and clippy, cargo-ndk 4.1.2, Temurin 17.0.20.1, SDK build-tools 35.0.0, platform 36, NDK 27.1.12297006), `build_image.sh` (image labelled with the Dockerfile sha256), `build_in_container.sh` (`SOURCE_DATE_EPOCH` from the commit, path remapping in RUSTFLAGS and CFLAGS, `--offline`, `--write-metadata`), `verification-metadata.xml` (433 components, sha256 only, host and Linux artifacts).
- **T8.** `check_repro.sh` (prefetch volume keyed on the four input files, two `--network none` builds in fresh containers, `cmp` of the APKs and the `.so` files, entry manifest, `check_apk.sh`, exit 0, 1 or 2), `apk_entry_diff.py` with 5 self-tests, `prefetch.gradle`, `REPRO.md`.
- **T9.** `.github/workflows/android.yml` (`workflow_dispatch` with a boolean `repro` input, and `android-v*` tags only; `contents: read`; a gate job and a repro job calling `check_repro.sh`), `docs/android/BUILD.md` (layout, pins, packaging, tasks, gate, emulator, signing, reproducibility, dependency verification, CI, housekeeping). Never pushed or run.

## Phase gate evidence

Planner's own runs on `6ae8174` (clean tree before the exit files, `main` `96728e1325f5e11d69b7571d17280e3027974e06`, `cert/v1.0` `d6210caabb32ca09f54be0f15410ce8447960be2`, no `linux-*` tag, `ui/dist` present):

| Step | Result |
|------|--------|
| `p3_gate.sh --exit --android --base ac15866...` | **PASS, 0 failed (34 checks)** |
| forced `:pos-core:testDebugUnitTest --rerun :pos-core:mismatchTest --rerun :pos-core:apiCheck --rerun` | BUILD SUCCESSFUL; EndToEnd 10, Lifecycle 7, MocksSmoke 3, Load 2, ChecksumMismatch 1, 0 failures |
| `p3_gate.sh --fast --emulator --device-tests --base ac15866...` | **PASS, 0 failed (17 checks)** |
| `device_engine_tests.sh --fixtures --check` | **PASS, 1371 passed, 0 failed** (88 binaries, 13 ignored) |

The 34 checks of the exit run:

| Check | Result |
|-------|--------|
| hygiene-paths | PASS, 57 changed paths, all allowed |
| hygiene-emdash, hygiene-artifacts | PASS |
| hygiene-subjects | PASS, 38 Conventional Commit subjects (9 merges included) |
| hygiene-root-lock | PASS, root `Cargo.lock` and `Cargo.toml` unchanged |
| frozen-provisional | PASS, `frozen against ref 042d911: 16 paths, clean` |
| linux-frozen | SKIP, no freeze tag |
| rust-fmt, rust-clippy | PASS |
| rust-tests | PASS, 236 passed |
| ffi-deps | PASS, pos-engine yes, pos-peppol no, tauri no |
| lock-parity | PASS, 240 packages, identical |
| command-parity (`--strict`) | PASS, missing none (84/84) |
| ffi-hygiene | PASS, 99 exports, 110 types |
| selftest-frozen, selftest-lock-parity, selftest-command-parity | PASS |
| jvm-tests | PASS |
| ndk-build | PASS, `libpos_ffi.so` 67668872 bytes (platform 33) |
| so-align | PASS, 4 LOAD segments, all 0x4000 |
| bindgen-android | PASS, `pos_ffi.kt` 14152 lines |
| jvm-check | PASS (`:pos-core:check`) |
| ffi-doc | PASS, FFI.md parity block current (88 lines) |
| selftest-check-apk | PASS, 12 passed |
| selftest-p3-gate | PASS, 17 passed |
| gradle-lint, gradle-ktlint, gradle-detekt | PASS |
| unit-tests | PASS, 22 tests |
| mismatch-test, api-check | PASS |
| aar-check | PASS, `jni/arm64-v8a/libpos_ffi.so` only, 4 consumer rules |
| assemble-release | PASS (`:smoke:assembleRelease`) |
| apk-check | PASS, 6 checks (host build, SHA-1 prefix `aa684fc6322c`; host builds are not the reproducible artifact) |

The 17 checks of the emulator run are the 14 fast checks above plus `emulator-smoke` PASS (`SMOKE | total | PASS | 0 failed (11 checks)`), `android-tests` PASS (19 tests) and `device-engine-tests` PASS (`88 binaries, passed 1178, failed 193, excluded 193, ignored 13, flaky 0`).

`test-inventory` (the `--root` check) was not rerun by the planner: the root target is 13 GB to rebuild and the host had 18 to 21 GB free. The integrator ran `p3_gate.sh --exit --root --android` at `6ae8174` (0 failed of 35 checks, root inventory 0 removed), and no root build input changed in P3 (empty freeze-path diff).

Integrator gates per wave (all with `ANDROID_HOME` exported; each first run without it failed only on the missing SDK): wave 1 `--root --android` 0 failed; wave 2 `--root --android`, `--fast --emulator` (11 smoke checks), `--fast --device-tests` 0 failed; wave 3 `--root --android` 33 checks, `--fast --emulator` 16 checks with 19 Android tests, then `emulator.sh stop`; wave 4 `--root --android` 33 checks; wave 5 `--root --android` 33, `--exit --root --android` 35, `--fast --repro` PASS, `--fast --emulator --device-tests` 17, all 0 failed.

## Reproducible APK

Integrator's `p3_gate.sh --fast --repro` at `6ae8174` (`~/.cache/pili-android/repro/6ae81747d8b0/work/result.txt`), on the macOS host, linux/amd64 under emulation:

| Item | Value |
|------|-------|
| image | `pili-android-build:p3`, ID `sha256:eb51089c48ad4b158ca24138b4949dced8a8600def0a608d94928c296ea9b833`, 4013231132 bytes, local only |
| `android/docker/Dockerfile` sha256 | `0045c8c5ef960329ecce979bed48b21127880f7a9c9784ad7869bf1e2767369f` (equal to the image label `pili.dockerfile.sha256`) |
| base image | `debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587` |
| `SOURCE_DATE_EPOCH` | 1791461034 (2026-10-08T12:03:54Z, the commit time) |
| prefetch volume | `pili-android-prefetch-164b3f8600de` (cargo registry 500 MB, Gradle home 657 MB) |
| build 1, build 2 | PASS offline, 1853 s and 1373 s |
| **unsigned APK** `smoke-release-unsigned.apk` | identical: **SHA-1 `73cc27ef2d41732abdf424d713f11cf2d9589cd0`**, **SHA-256 `9f7d6ddebdcc67c92d328fcdd6220f33c481f736f6e02f30525a37ed018d7b2f`**, 13684198 bytes |
| entry manifest | sha256 `a10fa0e725abf87d842c1acab234829c207e679d3a04b34652fca56f12b46380`, 25 entries |
| packaged `lib/arm64-v8a/libpos_ffi.so` (stripped by AGP) | SHA-1 `ee25ffdfc452a3b2c68d4e2121f0ba98b10565ef`, 12580648 bytes (12.0 MiB) |
| packaged `lib/arm64-v8a/libjnidispatch.so` | SHA-1 `ab124d34754c52e0373f7c66bc354db1afb6af78` |
| unstripped `libpos_ffi.so` (cargo output, paths remapped) | SHA-1 `e35f4578de618a9896f2bad64ea32774325ac79e`, 67640384 bytes, same in both builds |
| `check_apk.sh` on build 1 | PASS, 6 checks |

The same APK SHA-1 came out of the T7 container builds (two different `SOURCE_DATE_EPOCH` values) and of the T8 runs at `2133e8a`, `f361e37` and `7b9fbd5`. The native x86_64 Linux run (manual `repro` job of `android.yml`) was not run (PD83).

Host build sizes, for comparison (not reproducible artifacts): unstripped `.so` from the gate's `ndk-build` 67668872 bytes; the `.so` packaged in `pos-core-release.aar` 12610840 bytes.

## Device tests

| Figure | Value |
|--------|-------|
| emulator | `sunmi_d3_pro_api33` (API 33, arm64, google_apis, 1920x1080, 160 dpi, 4 GB) |
| smoke run (`emulator_smoke.sh`) | 11 checks PASS: ensure-initialized, version (`pili-pos-android/1.0.0-dev`), logging, install-db, open, events, login, fdm-status (connected), sale (order 1, 1200 cents, FDM `MCK00000001`), z-report 1, shutdown; R8 crash check PASS |
| instrumented tests (`run_android_tests.sh`) | 19 PASS: DeviceEndToEndTest 8, DeviceLifecycleTest 7, DeviceLoadTest 4 |
| engine tests (`device_engine_tests.sh --check`) | 88 binaries, 1384 listed, **1178 passed**, 193 failed and all 193 excluded as `host-fixture` (pos-engine 150, pos-fiscal 35, pos-hubrise-types 8), 13 ignored (live and staging tests, never run), 0 flaky, 0 `android-difference` |

Planner's opt-in run at the exit, `device_engine_tests.sh --fixtures --check` (own target `target/android-device-tests-fx`, fixtures pushed under `/data/local/tmp` and removed afterwards): `DEVICE | total | PASS | 88 binaries, passed 1371, failed 0, excluded 0, ignored 13, flaky 0`. Every runnable engine test of the cert build graph, the 150 pos-engine signing, report, crash-safety, FDM-failure and presence flows included, passes on the emulator once it can read its fixtures.

The 150 pos-engine exclusions all come from `workspace_root()` in `crates/pos-engine-testkit/src/fixtures.rs:56`, which resolves the UC fixtures from the host checkout at compile time. With `--fixtures` they get the fixtures on the device and pass (PD89).

## Test counts

| Figure | Start (`ac15866`) | Exit (`6ae8174`) |
|--------|-------------------|------------------|
| Root Rust ws tests listed | 1479 | 1479, 0 removed (integrator, floor 1355) |
| `android/rust` tests passed | 234 | 236 (two cancelled-shutdown lifecycle tests) |
| `:pos-core` host unit tests | 22 | 22 (EndToEnd 10, Lifecycle 7, MocksSmoke 3, Load 2) |
| `mismatchTest` | 1 | 1 |
| `:pos-core` instrumented tests | none | 19 |
| smoke run checks | none | 11 |
| engine tests passed on the emulator | none | 1178 (1371 with `--fixtures`) |
| gate self-tests | frozen 18, lock parity 10, command parity 52 | plus check_apk 12, p3_gate 17, apk_entry_diff 5 |

No test was removed, renamed or ignored.

## Freeze state

- No `linux-*` tag. `check_linux_frozen.sh` (tag mode) exits 2; `check_linux_frozen.sh --ref 042d911...` is clean (16 paths). `docs/android/LINUX_FREEZE.md` does not exist on `android/int`.
- **Branch `android/freeze-rc1`** (worktree `moyun-pos-wt/freeze-rc1`, tip `65b567f`, based on `623c90b`, not merged into `android/int`) carries: `228c562` (mutation scan scope in `scripts/build_and_hash.sh` and the two dossier files, the user's PD47 decision), `2e1e3ad` and `8dec93a` (the `ApplyOutcome::Rejected` fix in `crates/pos-catalog`, the HubRise crates, pos-ffi and the UI, the user's PD71 decision), and `65b567f` (`docs/android/LINUX_FREEZE.md`, a soft freeze record with `check_repro.sh` SHA-1 `9d230ef1bba90a4edde0157bcb107e4bb4d23d6e`, and a "Decided by the user" section in DECISIONS.md). Its record says the tag `linux-v1.1.0-rc1` is "set by the orchestrator after review"; that review and the tag are not part of this P3 exit, and the planner did not set them (PD97).
- The hard freeze `linux-v1.1.0` (the plan's P3 exit step) is the user's signature: not done.

## Left behind

| Item | State |
|------|-------|
| emulator | `sunmi_d3_pro_api33` started by the chain for the exit runs and stopped by the planner with `emulator.sh stop` at the end; no device owner, no `dev.nuagemagique.pilipos.*` package installed, no `adb reverse` rule |
| Docker images | `pili-android-build:p3` (`eb51089c48ad`, 4.0 GB) kept; no `pili-repro` container left |
| Docker volumes | `pili-android-prefetch-164b3f8600de` (about 1.2 GB) kept for the next repro run; older `pili-e2e-*` volumes are not P3's |
| caches | `~/.cache/pili-android/` (dev keystore, repro logs per commit, T7 logs, emulator log) |
| `int/target` | about 12 GB: `android-rust` 5.6 GB, `android-device-tests` 3.7 GB, `android-mocks` 1.3 GB, `android-rust-mismatch` 1.2 GB (rebuilt by the exit gate), `smoke`, `android-tests`; `android-device-tests-fx` (3.7 GB) removed after the fixtures run; no `debug` |
| Gradle daemons | none started by the chain left; `./gradlew --stop` run (the Gradle 9.3.1 daemon and the Kotlin daemons on the host belong to other projects) |
| free disk at the exit | about 19 GB (below the 25 GB a Docker card needs) |

## Decisions taken while the user was away

All in `docs/android/DECISIONS.md` under "Decided while the user was away (review)".

- **PD76 to PD88** (planner, before wave 1, copied by P3-T1): card layout, `:pos-core` as an Android library, NDK platform 33, stripping and packaging, the `:smoke` app and the development key, emulator sharing, engine tests on the emulator, the reproducible APK, CI, the gate, no freeze at the P3 exit, PD73 notes taken, exit without the user.
- **PD89 to PD98** (this exit):
  - PD89 device engine tests: the gate keeps the `host-fixture` exclusions; `--fixtures` is the exit evidence, not the gate default.
  - PD90 the `abi-mismatch` build renames the Rust function behind the `pos_sw_version` export (P3-T3 deviation, accepted).
  - PD91 `:pos-core` build wiring choices of P3-T2, and the gaps carried.
  - PD92 smoke and signing choices of P3-T4.
  - PD93 `DeviceLoadTest` checks a library mapped from the APK; detekt on `src/androidTest` goes to P4.
  - PD94 build image deviations of P3-T7 and the deleted Gradle 9.3.1 cache.
  - PD95 `check_repro.sh` options and `apk_entry_diff.py` exit status of P3-T8; the GNU `stat`/`date` defect carried.
  - PD96 gate review notes carried, and the forced test rerun at each exit.
  - PD97 freeze at the P3 exit: no tag; `android/freeze-rc1` left to its review.
  - PD98 exit records.

## Risks

1. **Soft freeze not on `android/int`.** Until `android/freeze-rc1` is reviewed, tagged and merged, only the provisional point `042d911` guards the Linux inputs. Once the tag exists, `check_linux_frozen.sh` in tag mode reports the PD47/PD71 commits as a diff on any `android/int` that lacks them, and `frozen-provisional` (`--ref 042d911`) fails on one that has them: the gates must move from `--ref 042d911` to the tag in the same step. Merging `android/freeze-rc1` gives a textual conflict at the end of DECISIONS.md (both append after PD88); keep both blocks.
2. **Reproducibility only under emulation.** Both builds ran linux/amd64 under emulation on Apple Silicon (23 to 31 minutes each at the exit). The native x86_64 run is the user's. `check_repro.sh` uses BSD `stat -f %z` and `date -r` for four INFO lines (lines 161, 166, 333, 362): on GNU coreutils these lines break the one-line format; PASS and FAIL are not affected (PD95).
3. **Dependency verification on a Linux runner.** The metadata was proven for the host gates (macOS) and the container `:smoke:assembleRelease`. The CI gate job is the first Linux run of lint, ktlint, detekt and the unit tests under strict verification and may need a metadata update; `connectedDebugAndroidTest` was never run in a container.
4. **150 pos-engine flows only with `--fixtures` on the device.** The default gate leaves the `spf_e2e_*` signing, report, crash-safety and FDM-failure flows to the host and the opt-in mode (PD89).
5. **Timing.** `cancelled_shutdown_then_shutdown_ends_the_streams` assumes the FDM HTTP client timeout is longer than about 0.7 s; one worker saw it fail once under load (20 of 20 passes in review, passes at the exit).
6. **Disappearing target directories.** Workers of T2 and T4 saw their worktree `target` deleted by something outside their commands; not reproduced, cause unknown.
7. **Disk.** About 19 GB free at the exit: below the 25 GB floor of a Docker card; `int/target` is about 12 GB (all regenerable), the build image 4 GB.
8. **Coverage gaps carried** (PD91, PD93, PD96): detekt does not scan `src/androidTest`; `uniffiBindgen` does not track the `uniffi-bindgen` wrapper sources; Gradle up-to-date results can satisfy the test checks without running them; the host-fixture exclusions match by name, not by failure text.

## What the next phase (P4) must know

- **Consuming `:pos-core`:** depend on `project(":pos-core")`; it brings the arm64 `libpos_ffi.so` (stripped at packaging, 16 KB aligned, stored), JNA as `@aar` and the four consumer R8 rules, so `:app` needs no keep rule of its own for the bindings or JNA. Only arm64-v8a is packaged. minSdk 33, compileSdk 36, NDK 27.1.12297006, AGP 8.9.1, Kotlin 2.2.20, R8 8.13.25, Gradle 8.11.1, JDK 17.
- **Runtime contract unchanged from P2** (FFI.md): `PosEngine(EngineOptions)` is synchronous, call it on `Dispatchers.IO`; every other call is `suspend`; `shutdown()` before reopening a data directory; after a cancelled shutdown the event streams end at the next `shutdown()` (P3-T3). `posSwVersion` is still `pili-pos-android/1.0.0-dev`.
- **Gate:** `scripts/android/p3_gate.sh` (or a `p4_gate.sh` on top of it) with `--allow`, `--emulator`, `--device-tests`, `--repro`; export `ANDROID_HOME` and `ANDROID_NDK_HOME` before any gate run. New modules join `lint`, `ktlintCheck` and `detekt` through `alias(libs.plugins.ktlint)` and `alias(libs.plugins.detekt)` (the gate greps for them). At an exit, force the test tasks with `--rerun`.
- **Dependencies:** any new Gradle dependency updates `android/gradle/verification-metadata.xml` in the same commit, with both passes of `docs/android/BUILD.md` section 9 (host `--write-verification-metadata sha256`, then the container `--write-metadata` run); otherwise the strict verification fails the build.
- **Emulator:** `scripts/android/emulator.sh ensure|status|stop`; one emulator at a time; reuse it; device ports 19180/19190 (smoke) and 19280/19290/19291 (instrumented tests) are taken, so P4 scripts pick their own range. Install release builds with `scripts/android/sign_dev.sh` (development key only). The plan allows wiping and provisioning the emulator in P4 (device owner); P3 did neither, and `emulator.sh` never wipes.
- **`:smoke` stays a test app.** `:app` must not depend on it; P6 moves the reproducibility check to `:app` (PD80).
- **Freeze:** follow the soft freeze step of `android/freeze-rc1` (risk 1). Until then every gate keeps `check_linux_frozen.sh --ref 042d911...`. The hard freeze waits for the user.
- **Container builds** run as root in the image; `cargoEnv` reads Java `user.home`, so a non-root container user needs `<home>/.cargo/bin` pointing at `/opt/cargo/bin`.

## Hashes and references

| Item | Value |
|------|-------|
| P3 base | `ac15866f8d916a94787c6e54ecbec64fe9139b40` |
| last card merge (`chore(android): merge P3-T9`) | `6ae81747d8b08e18c2cb183f8ee97a659734bb5c` (tree `a5664d032378d3f30b8c59d539fce80d58f1c761`) |
| card merges | T1 `c92e015`, T2 `c75a73a`, T3 `03fcd92`, T4 `62b7ab5`, T5 `99ea08e`, T6 `7ec2789`, T7 `623c90b`, T8 `62f22f1`, T9 `6ae8174` |
| diff `ac15866..6ae8174` | 57 files, 10071 insertions, 362 deletions, 38 commits (9 merges) |
| provisional freeze point | `042d91135940cb3f678a3f5e2947744a9d05ce4d` |
| main | `96728e1325f5e11d69b7571d17280e3027974e06` |
| `cert/v1.0` | `d6210caabb32ca09f54be0f15410ce8447960be2` |
| `android/freeze-rc1` | `65b567fdd2497277300340979c2c61ff17abc918` (not merged, no tag) |
| unsigned `:smoke` release APK | SHA-1 `73cc27ef2d41732abdf424d713f11cf2d9589cd0`, SHA-256 `9f7d6ddebdcc67c92d328fcdd6220f33c481f736f6e02f30525a37ed018d7b2f` |
| build image | `sha256:eb51089c48ad4b158ca24138b4949dced8a8600def0a608d94928c296ea9b833`, Dockerfile sha256 `0045c8c5ef960329ecce979bed48b21127880f7a9c9784ad7869bf1e2767369f` |
| `pos-core.api` | 7097 lines, unchanged since P2 |
| Linux baseline SHA-1 (P0.1, main `96728e1`) | `83313be28027d2e8d9e84cbddff4e584799c2fbc` |
| local tags | `android/p0-spike-S1` to `-S5` only; no `linux-*` tag |
| pins added | ktlint plugin 12.2.0, detekt 1.23.7, androidx.annotation 1.9.1, androidx.test runner, rules, core and ext-junit, cargo-ndk 4.1.2 in the image, Temurin 17.0.20.1, rustup 1.29.1, cmdline-tools 13114758 |

## Questions for the user

1. **Soft freeze from `android/freeze-rc1`.** It holds your PD47 and PD71 decisions and the `check_repro.sh` record (SHA-1 `9d230ef1...`). May the chain review it, set the local tag `linux-v1.1.0-rc1` on `65b567f`, merge the branch into `android/int` and switch every gate to the tag?
2. **Hard freeze.** The plan ends P3 with your signed `linux-v1.1.0`. When do you want to sign it, and on which commit (the rc1 target or a later one)?
3. **Build image and native run.** May the image `pili-android-build:p3` be pushed to GHCR, and the `android.yml` `repro` job (or another native x86_64 host) run to confirm the APK SHA-1 `73cc27ef...` off emulation? Both need a push.
4. **Device tests with fixtures.** Should `--fixtures` become the default of `p3_gate.sh --device-tests` (it runs the 150 pos-engine flows on the device, PD89)?
5. **Decisions PD76 to PD98**, in particular PD80 (development key outside the repo), PD83 (local image, sha256-only dependency verification) and PD94 (the deleted Gradle 9.3.1 cache).
6. **Worktrees** `moyun-pos-wt/P1b-P1b-T7` and `-T8` (carried from P1b): may they be removed (branches kept)?
7. **Merge to main.** `android/int` now carries P1a, P1b, P2 and P3. When do you want it reviewed and merged into main?
