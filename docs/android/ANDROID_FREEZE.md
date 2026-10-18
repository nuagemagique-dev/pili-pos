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
- `android/rust`, `android/build.gradle.kts`, `android/settings.gradle.kts`,
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

All rows are to fill at tagging.

| Item | Value |
|------|-------|
| tag commit | to fill at tagging |
| date | to fill at tagging |
| versionName | 1.0.0 (to confirm at tagging) |
| versionCode | 10000 (to confirm at tagging) |
| posSwVersion | pili-pos-android/1.0.0 (to confirm at tagging) |
| engine workspace version | 1.0.0 (`Cargo.toml` `[workspace.package]`, to confirm at tagging) |
| About string | to fill at tagging |
| `check_android_frozen.sh` output | to fill at tagging |
| frozen path count | to fill at tagging |

### Reproducible build (`check_repro.sh --commit <tag commit>`)

| Item | Value |
|------|-------|
| image id | to fill at tagging |
| Dockerfile sha256 | to fill at tagging |
| SOURCE_DATE_EPOCH | to fill at tagging |
| snapshot sha256 | to fill at tagging |
| prefetch key and volume | to fill at tagging |

| | build 1 | build 2 |
|---|---|---|
| unsigned APK SHA-1 | to fill at tagging | to fill at tagging |
| unsigned APK SHA-256 | to fill at tagging | to fill at tagging |
| size | to fill at tagging | to fill at tagging |
| `libpos_ffi.so` stripped SHA-1 | to fill at tagging | to fill at tagging |
| `libpos_ffi.so` unstripped SHA-1 | to fill at tagging | to fill at tagging |
| other libraries (`libjnidispatch.so`) SHA-1 | to fill at tagging | to fill at tagging |
| entry-manifest sha256 | to fill at tagging | to fill at tagging |
| duration | to fill at tagging | to fill at tagging |

Total line (`REPRO | total | ...`): to fill at tagging.

### Gates at the tag commit

Full gate results: to fill at tagging.

## Not covered

- EV-01, EV-02, EV-03: the signed APK, its library and the signing certificate
  need the release key (never in a container, never in the repository).
- EV-05: the value of record (build image fingerprint and the two unsigned APK
  SHA-1) belongs to the final `android-v1.0.0` release, not to rc3.
- EV-15: real hardware (Sunmi D3 Pro, Epson printer, CCV A920 probe).
- The native x86_64 reproducibility run: still to be done by the user (see
  `docs/android/REPRO.md`, "Limits").

## Relation to linux-v1.0.0-rc2

`linux-v1.0.0-rc2` is the freeze of the Linux product, set when both filings
were planned. It is kept as history, with `LINUX_FREEZE.md`,
`check_linux_frozen.sh` and the p1a and x1 phase gates that cite it. Nothing in
the Android dossier depends on it.
