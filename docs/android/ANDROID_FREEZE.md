# Android freeze: android-v1.0.0-rc3

Record of the freeze of the only product submitted for the v1 certification,
`pili-pos-android` 1.0.0 (dossier `docs/android/dossier`).

## Purpose

User decision of 2026-10-18 (INT-D41): v1 submits Android only. The freeze
therefore covers the inputs of the Android release APK, not the Linux product.
The Linux product (`src-tauri`, `ui/`, `deploy/`, `docs/SUBMISSION_ARTIFACTS`)
stays in the repository and keeps building and passing, but it is not submitted
and is not frozen by this record. The earlier Linux freeze `linux-v1.0.0-rc2`
and `docs/android/LINUX_FREEZE.md` stay as history (the tag is not deleted).

## Tag

| Item | Value |
|------|-------|
| tag | `android-v1.0.0-rc3` (local annotated tag, never pushed without the user) |
| final tag | `android-v1.0.0`, set at the release (it then replaces the rc3 tag as the default of `check_android_frozen.sh`) |
| tag target | the commit that carries the About string `pili-pos 1.0.0, android-v1.0.0-rc3` and whose unsigned `:app` release APK `check_repro.sh` rebuilt twice byte-identical (INT-D44, INT-D45) |

The tag cannot name its own commit, so the About string has no commit part. The
tag commit is recorded in the Values section below, in a commit made after the
tag (documentation only, not an APK input).

## Frozen paths

The inputs of the APK built by `android/docker/build_in_container.sh`:

- `Cargo.toml`, `rust-toolchain.toml`
- `android/rust` minus `pos-ffi/tests` (integration tests, not compiled into the APK; INT-D52), `android/build.gradle.kts`, `android/settings.gradle.kts`,
  `android/gradle.properties`, `android/gradle`, `android/gradlew`, `android/docker`
- `android/app` minus `src/test` and `src/androidTest`
- `android/pos-core` minus `src/test`
- `crates/<dir>` for every path package of
  `cargo tree --manifest-path android/rust/Cargo.toml -p pos-ffi -e normal,build --offline`

The root `Cargo.lock`, `src-tauri`, `ui`, `deploy` and `docs` are not frozen for
Android. Print the resolved list (pathspec excludes included) with:

```bash
scripts/android/check_android_frozen.sh --list
```

Check the freeze (working tree included):

```bash
scripts/android/check_android_frozen.sh            # against android-v1.0.0, else android-v1.0.0-rc3
scripts/android/check_android_frozen.sh --tag android-v1.0.0-rc3
scripts/android/check_android_frozen.sh --ref <commit>   # before the tag exists
```

Exit 0 frozen, 1 not frozen, 2 no freeze tag, unknown ref or bad usage. Its
self-test is `scripts/android/test_check_android_frozen.sh`. The gates that use
it: `android-frozen` (p2 and p7), `freeze-tag` (p4, with `FREEZE_SHA_RECORDED`)
and `plat-engine-ref` (p6).

## Freeze procedure

Run in the integration worktree, on a clean tree, with every lane merged.

1. Run the full gate of the repository (the list is in the rc3 task text and in
   `docs/android/BUILD.md`), including `p5_gate.sh`, `p6_gate.sh`, both
   `check_dossier.py` runs and `scripts/android/lane_paths.py --check`. All green.
2. Check that `plat_about_engine_ref` is exactly `pili-pos 1.0.0, android-v1.0.0-rc3`
   in `strings_plat_about.xml` (every locale) and that `scripts/android/p6_gate.sh --fast`
   gives `plat-engine-ref` PASS ("tag android-v1.0.0-rc3 not set yet").
3. Check that the EV-19 commit is an ancestor of the freeze:
   `git merge-base --is-ancestor 4ff25b110c975d497f548538a08d81387b504b25 HEAD` (INT-D46).
4. `scripts/android/check_android_frozen.sh --ref HEAD` prints "frozen against ref ...: N paths, clean".
5. `scripts/android/check_repro.sh --commit HEAD`: two clean offline container
   builds of the unsigned `:app` release APK, `REPRO | total | PASS`. Keep the
   `result.txt` lines for the Values section.
6. `git tag -a android-v1.0.0-rc3 -m "Android freeze rc3" HEAD` (never push).
7. `scripts/android/check_android_frozen.sh --tag android-v1.0.0-rc3` exits 0;
   fill `FREEZE_SHA_RECORDED` in `scripts/android/p4_gate.sh` with
   `git rev-parse 'android-v1.0.0-rc3^{commit}'`; `p6_gate.sh --fast` now
   reports `plat-engine-ref` PASS "matches android-v1.0.0-rc3".
8. Fill the Values section below and commit documentation only. Do not touch a frozen path.
9. EV-04 and EV-05 stay "À effectuer" in the dossier (INT-D47): their value of
   record belongs to the final `android-v1.0.0` release. The rc3 tag commit and
   the rc3 unsigned APK SHA-1 are candidates and live only here and in the
   `DOSSIER_EVIDENCE.md` notes. Never write a candidate SHA-1 as a value of record.

A change of a frozen path after the tag needs the user and a new freeze.

## Values

Freeze done on 2026-10-18 (INT-D50). These rc3 values are candidates, **not values of
record**: the values of record (EV-04, EV-05) belong to the final `android-v1.0.0` release
(INT-D47), and the dossier keeps its "À effectuer" placeholders.

| Item | Value |
|------|-------|
| tag | `android-v1.0.0-rc3`, local annotated tag (never pushed); message "pili-pos-android 1.0.0 rc3 freeze: unsigned :app release APK reproducible (SHA-1 34566c79c177c894c9f0616b1a00043516650f9c), engine and app frozen together (INT-D44)" |
| tag commit | `8c53acc3c7a0b2424b6cabb15e613e0d43753663` (`docs(android): redact the development host path from the GMS e2e evidence`, 2026-10-18 19:16:04 +0200) |
| date | 2026-10-18 |
| versionName | 1.0.0 (from `android/rust/pos-ffi/Cargo.toml` version 1.0.0) |
| versionCode | 10000 (MAJOR * 10000 + MINOR * 100 + PATCH) |
| posSwVersion | pili-pos-android/1.0.0 |
| engine workspace version | 1.0.0 (`Cargo.toml` `[workspace.package]`) |
| About string | `pili-pos 1.0.0, android-v1.0.0-rc3` (`plat_about_engine_ref`, `strings_plat_about.xml`) |
| `check_android_frozen.sh --tag android-v1.0.0-rc3` | `frozen against android-v1.0.0-rc3 (8c53acc3): 23 paths, clean` |
| frozen path count | 23 paths plus 3 pathspec excludes (list below) |
| EV-19 ancestor | `git merge-base --is-ancestor 4ff25b110c975d497f548538a08d81387b504b25 android-v1.0.0-rc3` succeeds |
| `linux-v1.0.0-rc2` | unchanged (history) |

`check_android_frozen.sh --list` at the tag commit:

```
Cargo.toml
rust-toolchain.toml
android/rust
android/build.gradle.kts
android/settings.gradle.kts
android/gradle.properties
android/gradle
android/gradlew
android/docker
android/app
android/pos-core
crates/pos-catalog
crates/pos-core
crates/pos-db
crates/pos-eft
crates/pos-engine
crates/pos-export
crates/pos-fiscal
crates/pos-hubrise
crates/pos-hubrise-types
crates/pos-mail
crates/pos-peppol
crates/pos-print
:(exclude)android/app/src/test
:(exclude)android/app/src/androidTest
:(exclude)android/pos-core/src/test
:(exclude)android/rust/pos-ffi/tests
```

### Build inputs

| Item | Value |
|------|-------|
| `Cargo.lock` (root) sha256 | `b6ef847219d77ced642091c93197a76a43aeaa35570ee634f383c21a4f6d0ed9` |
| `android/rust/Cargo.lock` sha256 | `594ff94df31c50d08506d1731d675fe1e0425f9b31f4bce34fba5f862117cf35` |
| rustc | 1.99.0 (b940084d7 2026-09-28), from `rust-toolchain.toml`; cargo-ndk 4.1.2 |
| NDK, build-tools, JDK | NDK 27.1.12297006 (r27b), build-tools 35.0.0, OpenJDK 17.0.20.1 |
| Gradle | 8.11.1 (wrapper) |
| build image | `pili-android-build:p3`, image id `sha256:4b11c2a8c86db0f0994307715f4c94451ca6c8e831aa01f46cc820ae9d693649` (local, never pushed, built 2026-10-18 by `build_image.sh`), base `debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587` |

### Reproducible build (`check_repro.sh --commit 8c53acc3c7a0b2424b6cabb15e613e0d43753663`)

Run from a clean `git clone --no-local` of the integration worktree
(`~/.cache/pili-freeze/android-rc3-clone`, detached at the tag commit), cold Docker (the image
was built by the run), Apple M1 host with `linux/amd64` emulation. Result file:
`~/.cache/pili-android/repro/8c53acc3c7a0/work/result.txt` (kept with `work/logs`).

| Item | Value |
|------|-------|
| commit (INFO line) | `8c53acc3c7a0b2424b6cabb15e613e0d43753663` |
| image id | `sha256:4b11c2a8c86db0f0994307715f4c94451ca6c8e831aa01f46cc820ae9d693649` |
| Dockerfile sha256 | `0045c8c5ef960329ecce979bed48b21127880f7a9c9784ad7869bf1e2767369f` |
| SOURCE_DATE_EPOCH | 1792343764 (2026-10-18T17:16:04Z, the commit time) |
| snapshot sha256 | `a896003fa5098c9fc6d12d79cb690f528001afac69825ec26f840cc6e92809f4`, 131358720 bytes |
| prefetch key and volume | `02c313528ccc924a14d0c75d55186c777a7cd7da296672928ac44347191b70e3`, volume `pili-android-prefetch-02c313528ccc`, filled in 244 s (method init-script), 1600 MB (cargo-registry 549 MB, gradle-home 1052 MB) |

| | build 1 | build 2 |
|---|---|---|
| unsigned APK SHA-1 | `34566c79c177c894c9f0616b1a00043516650f9c` | `34566c79c177c894c9f0616b1a00043516650f9c` |
| unsigned APK SHA-256 | `2417930a0ccc716ee56436191ded5077bc6639b5a51fe8af41edfdf7f5835e17` | identical (`cmp` equal) |
| size | 21904420 bytes | identical (`cmp` equal) |
| `libpos_ffi.so` in the APK (stripped) SHA-1 | `bc17c72f1ccfeb4625395b4a186893bb0d90f2ec` (16139112 bytes) | `bc17c72f1ccfeb4625395b4a186893bb0d90f2ec` |
| `libpos_ffi.so` cargo output (unstripped) SHA-1 | `94867993105b244284bf305c95b50f102f4eceec` (90526328 bytes) | `94867993105b244284bf305c95b50f102f4eceec` |
| `libjnidispatch.so` SHA-1 | `ab124d34754c52e0373f7c66bc354db1afb6af78` | same |
| `libandroidx.graphics.path.so` SHA-1 | `d7c17fcb44b30a6efe89d436aa102abe52e008a6` | same |
| entry-manifest sha256 | `54ee8ea2f107ceb032bb50899fcdba4afdabb99c77daba0f564d8835e47b535c` (117 entries) | same |
| duration | 1476 s wall (gradle 1468 s) | 1261 s wall (gradle 1241 s) |

Checks: build-1 PASS, build-2 PASS, apk-identical PASS, so-identical PASS, entry-manifest PASS,
apk-check PASS (`check_apk.sh`, 6 checks). Free disk 36 GB before the run, 19 GB after.

Total line: `REPRO | total | PASS | the two unsigned APKs are byte-identical (commit 8c53acc3c7a0)`.

### Gates at the tag commit

The full gate ran on `8c53acc3` before the tag (clean private Cargo target, `gradlew clean`
first, app unit tests forced with `--rerun`), all green:

| Step | Result |
|---|---|
| cargo fmt --check, clippy workspace, clippy pili-pos cert | PASS |
| cargo test --workspace | 2565 passed, 0 failed, 13 ignored (170 suites) |
| cargo test -p pili-pos cert | 14 passed |
| cargo test -p pos-engine cert | 1160 passed, 0 failed, 11 ignored (82 suites) |
| FFI in `android/rust` (fmt, clippy, test) | 315 passed |
| check_ipc_wrappers, check_command_parity --strict | 148/148 single-call, 148/148 parity (internal checks) |
| check_ui_parity --strict | called 136, excluded 10, pending 0, stale 0, missing 0 (146 commands) |
| ui build, Playwright | PASS, 712 passed |
| p5_gate | PASS, 0 failed (35 checks), app unit tests 2807, kover LINE 97.8% / BRANCH 74.7% |
| p6_gate (lint, unit tests, Roborazzi verify, kover, release assemble, check_apk) | PASS, 0 failed (59 checks), plat-engine-ref PASS ("tag android-v1.0.0-rc3 not set yet") |
| SKIPs before the tag | `freeze-tag` and `android-frozen` (no tag yet), `linux-frozen` (retired, INT-D44), `frozen-provisional` (retired) |
| check_dossier (Linux and Android) | PASS / PASS |
| lane_paths --check | PASS, 5093 files, 90 rules |
| shell, Python and mjs self-tests | all rc=0 |

After the tag, at the post-tag documentation commit: see INT-D50 in `DECISIONS.md` for the
`--fast` p5 and p6 results with `freeze-tag`, `android-frozen` and `plat-engine-ref` PASS.

## Not covered

- EV-01, EV-02, EV-03: the signed APK, its library and the signing certificate
  need the release key (never in a container, never in the repository).
- EV-04 and EV-05: the values of record (tag commit, build image fingerprint and the two
  unsigned APK SHA-1) belong to the final `android-v1.0.0` release, not to rc3 (INT-D47).
- EV-14: the live run against the real FDM service.
- EV-15: real hardware (Sunmi D3 Pro, Epson printer, CCV A920 probe).
- The native x86_64 reproducibility run: still to be done by the user (see
  `docs/android/REPRO.md`, "Limits").

## Relation to linux-v1.0.0-rc2

`linux-v1.0.0-rc2` is the freeze of the Linux product, set when both filings
were planned. It is kept as history, with `LINUX_FREEZE.md`,
`check_linux_frozen.sh` and the p1a and x1 phase gates that cite it. Nothing in
the Android dossier depends on it.
