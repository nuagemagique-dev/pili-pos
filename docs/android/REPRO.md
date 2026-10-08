# Reproducible APK check (P3-T8)

Status: done on the local Apple M1 host. Two clean offline builds of the unsigned `:smoke` release
APK, in fresh `--network none` containers of the pinned image, give byte-identical APKs. The run on a
native x86_64 Linux host is still to be done by the user (see "Limits").

Decisions behind this document: DECISIONS 11.13 and PD83 (plan P3.4), spike S5
(`docs/android/spikes/S5.md`). The tool is `scripts/android/check_repro.sh`; the gate runs it as
`apk-repro` with `scripts/android/p3_gate.sh --repro`.

## What is compared, and why

The container builds `smoke-release-unsigned.apk` (`:smoke:assembleRelease`). The check compares the
two **unsigned** APKs byte for byte (`cmp`), and then shows three more views of the same fact:

| Check | What it proves |
|---|---|
| `apk-identical` | `cmp` of the two APKs, with the SHA-1 of each in the detail |
| `so-identical` | every `lib/**/*.so` inside the APKs has the same SHA-1 (the stripped `libpos_ffi.so` and the JNA `libjnidispatch.so`); the unstripped cargo output is reported as INFO |
| `entry-manifest` | the entry manifest of `scripts/android/apk_entry_diff.py` (sha256 over the sorted list of entry name and uncompressed-content sha256) is equal; it ignores zip layout and any signing block, so it can also be computed on a signed or installed APK |
| `apk-check` | `scripts/android/check_apk.sh` (zipalign 16 KB, one ABI, stored libraries, 0x4000 alignment, stripped) passes on the first APK |

Why unsigned: the release signing key never enters a container (DECISIONS 9). Signing happens
outside the build, with the pinned build-tools `apksigner` (`--v1-signing-enabled false`, v2 and v3
on). An unsigned APK has no key dependence, so a byte comparison is exact. The SHA-1 of the signed
APK additionally needs a deterministic signature, which spike S5 measured: apksig signs with
RSASSA-PKCS1-v1_5 for an RSA key and the signature bytes repeat for the same key and input, while
ECDSA uses a random nonce and does not. **The production signing key must therefore be RSA**
(RSA-2048 or larger) if the SHA-1 of the signed APK is to be the value of record. The signed SHA-1
also depends on the certificate, so it is only comparable between builds signed with the same key;
anyone without the key compares the unsigned SHA-1 or the entry manifest.

## How to run it

```
scripts/android/check_repro.sh [--commit REF] [--keep-work] [--clean-prefetch] [--prefetch-only]
```

Output: `REPRO | <check> | PASS|FAIL|INFO | <detail>` lines and one `REPRO | total | PASS|FAIL`;
exit 0 when the APKs are identical, 1 when they differ or a prefetch or build failed, 2 on a
precondition failure. The lines are also written to
`$HOME/.cache/pili-android/repro/<12 hex of the commit>/work/result.txt`, next to the logs
(`work/logs/`: `prefetch.log`, `build-1.log`, `build-2.log`, `gradle-1.log`, `gradle-2.log`,
`check_apk.txt`). Without `--keep-work` the snapshot and both run directories are deleted at the
end.

Preconditions (exit 2 otherwise): clean working tree, Docker running, at least 25 GB free
(`PILI_MIN_FREE_GB` overrides, see below), and the image `pili-android-build:p3` whose label
`pili.dockerfile.sha256` equals the sha256 of `android/docker/Dockerfile` at the commit; if the image
is missing or stale the script runs `scripts/android/build_image.sh`. The image is local and
never pushed.

Steps:

1. **Snapshot.** `git archive` of the commit into `src.tar`. `SOURCE_DATE_EPOCH` is the commit time
   (`git show -s --format=%ct`).
2. **Prefetch, network on, once per key.** The key is the sha256 over `android/rust/Cargo.lock`,
   `android/gradle/verification-metadata.xml`, `android/gradle/wrapper/gradle-wrapper.properties`
   and `android/docker/Dockerfile`. It names the Docker volume `pili-android-prefetch-<first 12 hex>`
   and the volume is reused while its `COMPLETE` marker (the sha256 of `android/docker/prefetch.gradle`)
   still matches; `--clean-prefetch` removes it first. The volume holds:
   - `cargo-registry`: `$CARGO_HOME/registry` after `cargo fetch --locked --manifest-path android/rust/Cargo.toml`;
   - `gradle-home`: `wrapper/` (the Gradle 8.11.1 distribution, checked against the
     `distributionSha256Sum` of the wrapper properties) and `caches/modules-2` (every Maven artifact,
     each verified against `verification-metadata.xml` when it was downloaded).
3. **Two builds.** Each is `docker run --rm --platform linux/amd64 --network none` with an empty
   `/build`, `/in/src.tar` and `/prefetch` mounted read only, and its own `/out` (`work/run1`,
   `work/run2`). The container copies the volume into its own `CARGO_HOME` (`/opt/cargo/registry`)
   and `GRADLE_USER_HOME` (`/build/gradle-home`), extracts the snapshot to `/build/src` (the same
   path in both runs) and runs `android/docker/build_in_container.sh --offline` (Gradle
   `--offline`, `CARGO_NET_OFFLINE=true`, `RUSTFLAGS` path remaps, `CFLAGS` prefix maps,
   `CARGO_INCREMENTAL=0`, `SOURCE_DATE_EPOCH`). No directory is shared between the two builds
   except the read-only inputs. The head of each build log records the `docker run` flags, and the
   log contains `BUILD | start | ... | SOURCE_DATE_EPOCH=<commit time>`.
4. **Compare**, as in the table above.

### Prefetch method

Method used: **init script** (`android/docker/prefetch.gradle`, no full network build). The init
script adds the task `prefetchAll` to every project. It resolves the buildscript classpath and every
resolvable configuration through a lenient artifact view (`:pos-core` 108 configurations, `:smoke`
119, the root project 1; none skipped), so every download passes Gradle's dependency verification
with the network on. AGP also resolves two things of its own version through detached
configurations while the build runs, which no project configuration lists; the first offline
build of this card failed on them:

- `com.android.tools.build:aapt2:8.9.1-12782657:linux@jar` (task `compileReleaseLibraryResources`);
- `com.android.tools.lint:lint-gradle:31.9.1` with its dependencies (task `lintVitalAnalyzeRelease`,
  which `assembleRelease` runs because `checkReleaseBuilds` is on).

The task `prefetchLazy` resolves both, taking the versions from `verification-metadata.xml`, so the
file stays the single list of what the build may fetch. The fallback (`PILI_PREFETCH_METHOD=full-build`,
one extra network build of `:smoke:assembleRelease` in the prefetch container with the outputs
discarded) exists but was not needed. A new lazily resolved dependency would show up as a failed
offline build, never as a silent network access.

The prefetch container also installs `platform-tools` into its own SDK copy (AGP does that while
resolving configurations); the two builds do not need it and the SDK of the image is not changed.

## The run of record

Run on the head of the branch before this document was added, with `--keep-work`:

| Item | Value |
|---|---|
| Commit | `2133e8a1cb53a5cb039d50719500177495b16a9a` (`build(android): prefetch the lint-gradle classpath that AGP resolves at lint-vital time`) |
| Image | `pili-android-build:p3`, id `sha256:eb51089c48ad4b158ca24138b4949dced8a8600def0a608d94928c296ea9b833` |
| Dockerfile sha256 | `0045c8c5ef960329ecce979bed48b21127880f7a9c9784ad7869bf1e2767369f` |
| `SOURCE_DATE_EPOCH` | `1791449336` (2026-10-08T08:48:56Z, the commit time) |
| Snapshot sha256 | `0a31397c79abb07f3d0c89d45bf36cdc467319fb5f6c21d0eaded16776499d9c` (git archive, 56,064,000 bytes) |
| Prefetch key | `164b3f8600dececbfa52392339a2e98e3719324cf21ef3987476db988d676eb2`, volume `pili-android-prefetch-164b3f8600de` |
| Prefetch method and size | init script; 1157 MB (cargo registry 500 MB, Gradle home 657 MB); filled in about 8 minutes with the network on |
| Build 1 | 1696 s wall, 1667 s in Gradle (cargo-ndk, bindgen, R8, packaging) |
| Build 2 | 1348 s wall, 1328 s in Gradle |
| Unsigned APK | `smoke-release-unsigned.apk`, 13,684,198 bytes |
| APK SHA-1 (both builds) | `73cc27ef2d41732abdf424d713f11cf2d9589cd0` |
| APK SHA-256 | `9f7d6ddebdcc67c92d328fcdd6220f33c481f736f6e02f30525a37ed018d7b2f` |
| `libpos_ffi.so` in the APK (stripped) | SHA-1 `ee25ffdfc452a3b2c68d4e2121f0ba98b10565ef`, 12,580,648 bytes |
| `libjnidispatch.so` in the APK (JNA) | SHA-1 `ab124d34754c52e0373f7c66bc354db1afb6af78` |
| `libpos_ffi.so` unstripped (cargo output) | SHA-1 `e35f4578de618a9896f2bad64ea32774325ac79e`, 67,640,384 bytes, equal in both builds |
| Entry manifest sha256 | `a10fa0e725abf87d842c1acab234829c207e679d3a04b34652fca56f12b46380` (25 entries) |

```
REPRO | build-1 | PASS | smoke-release-unsigned.apk built offline in 1696s (gradle 1667s)
REPRO | build-2 | PASS | smoke-release-unsigned.apk built offline in 1348s (gradle 1328s)
REPRO | apk-identical | PASS | cmp equal, SHA-1 build-1 73cc27ef... build-2 73cc27ef...
REPRO | so-identical | PASS | lib/arm64-v8a/libjnidispatch.so ab124d34..., lib/arm64-v8a/libpos_ffi.so ee25ffdf...
REPRO | entry-manifest | PASS | sha256 a10fa0e725ab... (25 entries)
REPRO | apk-check | PASS | check_apk.sh on build-1 (6 checks passed)
REPRO | total | PASS | the two unsigned APKs are byte-identical (commit 2133e8a1cb53)
```

Second run, on the head that adds this document (`f361e375bb67`, commit time 2026-10-08T10:08:15Z,
a different `SOURCE_DATE_EPOCH`, default cleanup, no `--keep-work`): `REPRO | total | PASS`, APK SHA-1
`73cc27ef2d41732abdf424d713f11cf2d9589cd0`, SHA-256 `9f7d6ddebdcc...`, stripped `libpos_ffi.so`
`ee25ffdf...`, unstripped `e35f4578...`, entry manifest `a10fa0e725ab...`: all equal to the first run.
The APK therefore does not depend on the commit time (it is not embedded in the build output and the
file times are normalised) nor on the docs-only commits. Build durations of that run: 2407 s and 1471 s
(another container was building on the same Docker VM during the first build).

### What the build forces to be equal

The determinism settings are those of S5 and `build_in_container.sh`: fixed build path
`/build/src` with `--remap-path-prefix` for the source, `CARGO_HOME`, `RUSTUP_HOME` and the target
directory; `-ffile-prefix-map` for C built by build scripts; `CARGO_INCREMENTAL=0`; file mtimes set
to `SOURCE_DATE_EPOCH`; `LANG=C.UTF-8`, `TZ=UTC`; archive tasks without timestamps and with a
stable order (`android/build.gradle.kts`); no VCS info and no dependency metadata blob in the APK.

### The mismatch tool

`python3 scripts/android/apk_entry_diff.py A.apk B.apk` prints the entry manifest of each APK and,
for each entry whose content differs, its sha256, sizes, compressed sizes, CRC and date; exit 0 when
the contents are equal, 1 otherwise. Container-only differences (timestamps, compression, order)
are listed as `META` and do not change the exit status. `check_repro.sh` prints these lines as
`entry-diff` when the manifests differ. Self-test: `python3 scripts/android/test_apk_entry_diff.py`.

Negative demonstration (run1's APK against a copy whose `AndroidManifest.xml` got one extra byte):

```
MANIFEST | A | a10fa0e725abf87d842c1acab234829c207e679d3a04b34652fca56f12b46380 | 25
MANIFEST | B | 5283f2720ea2397da174511bb782f2b2e4516892395ced8d1156e8d25cd686d9 | 25
DIFF | AndroidManifest.xml | sha256 A 6215a4a6... B 71b8b067... | size A 2360 B 2361 | csize A 859 B 857 | crc A 0aa528b2 B f7679805 | date A 1981-01-01T01:01:02 B 1981-01-01T01:01:02
RESULT | DIFFERENT | 1 entries
```

(exit 1). Run1's APK against run2's APK gives `RESULT | EQUAL` (exit 0).

## Limits

- **The image is local.** `pili-android-build:p3` is built from the Dockerfile on the host and is
  not pushed. A GHCR push, and the image digest of record that goes with each release, need the
  user. Until then the Dockerfile sha256 and the image id above identify the toolchain.
- **One architecture tested.** Both builds ran on an Apple M1 under Rosetta (linux/amd64). The
  native x86_64 Linux run is still to be done by the user, with the manual `repro` job of
  `.github/workflows/android.yml` (not run by any agent).
- **Trusted sources.** Maven artifacts are trusted through the sha256 values of
  `android/gradle/verification-metadata.xml` (no signature check); crates through the checksums of
  `android/rust/Cargo.lock`, which cargo verifies; the Gradle distribution through the
  `distributionSha256Sum` of the wrapper properties. The SDK, NDK, JDK and Rust toolchain are pinned
  in the image (checksums or sdkmanager sha1, see the Dockerfile). Nothing is rebuilt from source.
- **Disk guard.** The default minimum of free disk is 25 GB. On the 460 GB host used for this card
  only 20 GB were free, so the runs used `PILI_MIN_FREE_GB=15`. The Docker VM disk grew by several GB during the first build, which is why the guard exists.
- **Not covered:** the signed APK (no key in the build), the Linux binary of the frozen
  desktop application (a different SHA-1), and the timing of a native build (about 22 to 28 minutes
  per build here under emulation).
