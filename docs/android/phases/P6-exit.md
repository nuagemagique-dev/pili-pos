# P6 exit report: Compliance and platform hardening (platform lane)

> Correction (2026-10-10): the Linux build is not filed as v1.1 under Art. 113. Nothing was filed, so Linux and Android are both version 1.0.0 initial filings (Art. 99), and the freeze tag is `linux-v1.0.0-rc1`, not `linux-v1.1.0-rc1` (`docs/android/DECISIONS.md`, "Decided by the user"). The report below is unchanged.

Written by the P6 planner on 2026-10-09, at the end of the P6 run, with the user away (PD221).

- **Base:** `android/lane-plat` at `ba57adf` (`docs(android): P4 phase cards, exit report and exit decisions`), equal to `android/int`.
- **Exit head:** `android/lane-plat` at `4cfe0b3` (`docs(android): record the P6 wave 7 merge and the open gate items`) before the exit commit, then the docs-only exit commit: `phases/P6.md`, this report, PD222 to PD230 in `DECISIONS.md` and the P6 exit check in `LINUX_FREEZE.md`.
- **Cards:** `docs/android/phases/P6.md`. Decisions: `docs/android/DECISIONS.md`, "Decided while the user was away (review)": PD201 to PD221 (planner, copied by P6-T1), the integrators' wave records, and PD222 to PD230 at this exit. Reference for what was built: `docs/android/PLATFORM.md` (sections 1 to 14).
- main (`96728e1`), `cert/v1.0` (`d6210ca`) and `android/int` (`ba57adf`) were not touched. Nothing was pushed. Not done by the chain: GitHub Actions runs, live module2, HubRise, Scaleway, GHCR or SPF calls, a real run of `declare_android.sh`, a release signing key or real minisign declaration key, any tag. Only throwaway keys in `mktemp` directories and the committed development minisign key were used.

## Verdict

- **All eleven cards are APPROVE and merged** into `android/lane-plat` with `--no-ff`, with no conflict.
  - P6-T6, P6-T8 and P6-T9 were reported "partial". Each time the reason was a gate item outside the card's scope, never the card's own work: the `FeatureEntriesTest` route set for T6 and T8, and the `hygiene-paths` gap for T9.
  - P6-T11 had one rework round, which fixed the `zIsDone` precondition after a real Z.
  - Integration fix commits: `215be38` and `e03146e` (the route set of `FeatureEntriesTest`), and `986108b` (ktlint and detekt on the `androidTest` platform steps).
- **`android/lane-plat` builds, and the phase gate passes except one known check.** The only red check is `hygiene-paths` (PD223). It refuses the two `scripts/release/` files of P6-T9 because `p4_gate.sh` never passes `--extra-paths` to the hygiene sub-gate. This is a foundation gate gap, not a defect of the lane, and the planner did not waive it.
  - **Planner's run of `p6_gate.sh --exit --android --base ba57adf`:** `GATE | total | FAIL | 2 failed (63 checks)`. The failures were `hygiene-paths` (PD223) and `app-screenshots`. The `app-screenshots` failure was the intermittent `KioskControllerTest` failure, and the planner traced its cause (PD225). Rerun alone, `:app:verifyRoborazziDebug` gave BUILD SUCCESSFUL.
  - **Planner's run of `p6_gate.sh --fast --plat-e2e --app-e2e --base ba57adf` on the emulator:** `GATE | total | FAIL | 1 failed (29 checks)`, and the only failure is `hygiene-paths`. `plat-e2e` passed 0 failed of 19 and `app-e2e` passed 0 failed of 11.
  - **The plan's P6 Gradle gate holds:** `plan-gradle` (`lint ktlintCheck detekt test koverVerifyDebug assembleRelease`) PASS.
- **The plan's P6 tasks are delivered and proven on the emulator:**
  - SPF export (`pos.db` and 7 backups, logs, audit, identity, `LISEZMOI.txt`, `MANIFEST.sha256` checked on the device and on the host);
  - About screen with the hashes;
  - declared-only updater: a real PackageInstaller install from a signed descriptor through an OCI registry mock, with refusals for `not_newer` and `sha256_mismatch`;
  - device-owner restrictions;
  - no cloud backup;
  - crash restart: `markFatal`, then the watchdog kills the process and the app is Ready and locked again;
  - release tooling, dry run only;
  - the reproducible `:app` APK.
- **No freeze step at this exit** (PD228).

**Planner recommendation:** the lane is ready for the integrator's merge into `android/int`, in the order of X1D3: UI and platform lanes first, then `android/lane-export`. Before or with that merge, run one foundation card alone for PD223, PD224 and PD225, so that the `android/int` gate is green without exceptions. Merging into main, the re-freeze, pushes, the release keys and the real `declare_android.sh` wait for the user.

## Task status

| Card | Wave | Card commits (fork..tip) | Review | Merge on `android/lane-plat` |
|------|------|--------------------------|--------|------------------------------|
| P6-T1 foundation hooks (shared): `AppLifecycleHook` set, `EngineGateway.state` and `close()`, `EngineHelpers.verifyMinisign`, pump `markFatal` on stream end while Ready, LANES.md platform rules and routes | 1 | `ba57adf..da05838` (3) | APPROVE | `4f9c1a7` |
| P6-T2 `p6_gate.sh` (8 P6 checks, 104 self-tests), `BackupPolicyTest`, `PLATFORM.md` | 2 | `4f9c1a7..2ce09e3` (4) | APPROVE | `df37d61` |
| P6-T5 audit log (`audit.jsonl`, fsync, 1 MiB x 5 rotation), crash watchdog (budget 3 in 10 min, 60 s reopen), `BootReceiver` | 2 | `4f9c1a7..bc0c4d8` (2) | APPROVE | `d09d709` |
| P6-T3 device-owner policies (13 restrictions, uninstall block, time and zone, stay awake, keyguard, status bar, update policy, lock task NONE, adb off in release only) | 3 | `d09d709..0105e5d` (1) | APPROVE | `51f791a` |
| P6-T4 About screen and identity report (APK SHA-1 and SHA-256, `libpos_ffi.so` entry SHA-1, signer, ids, storage), route `platform/about` | 3 | `d09d709..1a92dff` (1) | APPROVE | `404245d` |
| P6-T6 SPF export over SAF (`platform/export`, `LISEZMOI.txt`, `MANIFEST.sha256`, read-back hashing) | 4 | `33963d2..ae61dac` (1) | APPROVE (partial: route set, fixed by `215be38`) | `174c47f` |
| P6-T7 updater verification core (descriptor, minisign, GHCR anonymous pull, USB source, `plat_update_keys.xml`) | 4 | `33963d2..2d37d36` (2) | APPROVE | `bf5c140` |
| P6-T8 updater install (preconditions, PackageInstaller session, receivers, completion hook, screen), route `platform/update` | 5 | `313701a..75415f6` (1) | APPROVE (partial: route set, fixed by `e03146e`) | `7cae8a2` |
| P6-T9 `declare_android.sh` (dry run only) and its 42 self-tests | 5 | `313701a..3a93c05` (2) | ESCALATE on the gate gap only, content approved (PD223) | `2364920` |
| P6-T10 reproducible `:app` APK (container build, `check_repro.sh`, `android.yml`, `REPRO.md`) | 6 | `2364920..9142ade` (4) | APPROVE | `a35101f` |
| P6-T11 platform end-to-end run on the emulator (`plat_e2e.sh`, registry mock, `PlatformE2eTest`), measured storage figures, `zIsDone` fix | 7 | `e03146e..a767ff5` (4) | APPROVE after one rework | `75d3264` |

The card branches are kept locally. Three of them have a lowercase `p6` in their names: `android/p6-P6-T1`, `android/p6-P6-T2` and `android/p6-P6-T3`. The other card branches are spelled `android/P6-P6-T<n>`. Every P6 task worktree is removed, except `moyun-pos-wt/P6-P6-T10`: it is clean, its branch is merged, and the planner's cleanup command was refused, so it is left for the integrator.

Across P6: 31 non-merge commits (25 card commits, 3 integration fixes, 3 integrator docs commits), 11 merges, 162 files, +18597/-35 lines against `ba57adf` (before the exit commit).

## Phase gate evidence

Planner's runs on `4cfe0b3`, the exit docs uncommitted in the tree. Reference commits: `main` `96728e1325f5e11d69b7571d17280e3027974e06`, `cert/v1.0` `d6210caabb32ca09f54be0f15410ce8447960be2`, `linux-v1.1.0-rc1^{commit}` `65b567fdd2497277300340979c2c61ff17abc918`, `android/int` `ba57adf06191a1f386863f98f24930b6f5aa543c`. There is no `linux-v1.1.0` or `linux-v1.0.0*` tag.

| Step (P6.md "Phase gate") | Result |
|------|--------|
| 1. clean tree | PASS (0 entries before the exit docs) |
| 2. `check_linux_frozen.sh --tag linux-v1.1.0-rc1` | `frozen against linux-v1.1.0-rc1 (65b567f): 16 paths, clean` |
| 3. `lane_paths.py --check` | `LANES | total | PASS | 3518 files, 77 rules, 72 probes` |
| 4. `kotlin_test_inventory.py --base ba57adf` | `KTESTS | total | PASS | base 318, head 727, removed 0` |
| 5. lane scope diff against `ba57adf` | Outside the platform regex: only the P6-T1 files, which its own regex allowed (PD203), and `nav/FeatureEntriesTest.kt` (PD224). No em-dash in the diff. |
| 6. `p6_gate.sh --exit --android --base ba57adf` | `GATE | total | FAIL | 2 failed (63 checks)`: `hygiene-paths` (PD223) and `app-screenshots` (`KioskControllerTest`, PD225), which passed on rerun. `--root` was not run (PD222). |
| 7. `p6_gate.sh --fast --plat-e2e --app-e2e --base ba57adf` (emulator) | `GATE | total | FAIL | 1 failed (29 checks)`: `hygiene-paths` only. `plat-e2e` 0 failed of 19, `app-e2e` 0 failed of 11. Run under the non-app profile (PD222). |
| 8. `check_repro.sh` | PASS at `986108b` (integrator wave 7): two byte-identical unsigned `:app` APKs, SHA-1 `f7c9aab317c0759aa92458e9e7ddda36704eb61f` (PD227) |
| 9. `p4_gate.sh --fast --linux-repro` | SHA-1 `9d230ef1bba90a4edde0157bcb107e4bb4d23d6e` reproduced from the tag on 2026-10-09 (integrator wave 7; independent of the lane head) |

Key lines of the step 6 run:
- Hygiene, freeze and lanes: `hygiene-subjects` 42 Conventional Commits; `hygiene-root-lock` PASS; `linux-frozen` and `freeze-tag` PASS; `lanes` 3519 files; `kotlin-tests` base 318, head 727, removed 0.
- Rust and FFI: `rust-tests` 236 passed; `lock-parity` 240 packages identical; `command-parity` missing none; `ffi-hygiene` 99 exports, 110 types; `ffi-doc` current.
- Native build: `ndk-build` `libpos_ffi.so` 67748792 bytes; `so-align` 4 LOAD segments at 0x4000; `bindgen-android` 14151 lines.
- `:pos-core` and `:smoke`: `jvm-tests`, `jvm-check`, `unit-tests` 22, `mismatch-test`, `api-check`, `aar-check`, `assemble-release` and `apk-check` (`:smoke`) all PASS.
- Gradle quality checks: `gradle-lint`, `gradle-ktlint` and `gradle-detekt` PASS.
- `:app`:
  - `app-lint`, `app-ktlint` and `app-detekt` PASS;
  - `app-unit-tests` 683 tests;
  - `app-kover` LINE 94.2%, BRANCH 75.3% (floor 80/60);
  - `app-assemble-release` PASS and `app-apk-check` 6 checks;
  - `plan-gradle` PASS.
- P6 checks:
  - `plat-manifest`: backup off, no installLocation, 6 permissions, none for storage or install;
  - `plat-backup-rules`: both sections exclude the 5 domains;
  - `plat-strings`: 3 files, 113 names, same keys in fr, en and zh;
  - `plat-boundary`: 55 source files;
  - `plat-keys`: debug key equals `dev-minisign.pub`, release key empty;
  - `plat-engine-ref`: `pili-pos 1.1.0, linux-v1.1.0-rc1 (65b567f)`;
  - `selftest-p6-gate`: 104 tests;
  - `selftest-declare`: 42 cases.
- Self-tests: `selftest-p4-gate` 75 and `selftest-p3-gate` 17 PASS.

Earlier lane-wide runs: the wave 3 and wave 4 integrators passed `p6_gate.sh --exit --android --root` (64 checks, root inventory included) and `--app --fast --plat-e2e --app-e2e` (23 checks). After the T11 merge, the wave 7 integrator passed the following: `plat_e2e.sh` alone (19 checks), `app-e2e` (11 checks), `check_repro.sh` and `p4_gate.sh --fast --linux-repro`, each green except `hygiene-paths`.

### Platform end-to-end run (planner's exit run, `sunmi_d3_pro_api33`)

`plat-e2e` 0 failed of 19. The measurements of this run, from the step logcats under `target/android-plat-e2e/`:

- **Policies.** `PLATE2E_POLICIES ok restrictions=13 zone=Europe/Brussels`, with lock task features 0, auto time on, auto zone off, uninstall blocked, stay awake 7 and keyguard off. `no_debugging_features`, `no_physical_media` and `no_config_wifi` are absent in debug.
- **Sales and storage.**
  - 50 one-line sales: `PLATE2E_DBSIZE_LOGICAL ... bytes_before=294912 bytes_after=618496 bytes_per_sale=6471 free_bytes=0`.
  - 10 six-line sales: `PLATE2E_DBSIZE_LINES ... bytes_per_sale=14336`.
  - File size with the WAL: `bytes_per_sale=88544`. This is WAL rewrite, not growth.
  - After the Z: `PLATE2E_BACKUPS after_z files=1 bytes=778240`.
  - These are the same figures as P6-T11's run, so the measurement is reproducible.
- **Export.** `PLATE2E_EXPORT ok files=8 bytes=1562865` on the removable volume `BFD1-0A11`. `MANIFEST.sha256` was checked on the device and on the host, and the `integrity_check` of `pos.db` returned ok.
- **Update.** `PLATE2E_PRECONDITIONS z=1 zEvent=62 needsZ=false eventsAfter=63:R open=0/0` (the paired R of the Z is accepted). Then `PLATE2E_UPDATE verified version=1.0.1 versionCode=10001`, then `committed to=10001`. The new process is Ready and LOCKED.
- **Refusals.** `not_newer` and `sha256_mismatch` were each refused with their own code.
- **Watchdog.** `PLATE2E_FATAL sent`, then the process was killed and restarted: Ready, LOCKED.
- **Audit.** `PLATE2E_AUDIT ok rows=32 kinds=13 update_done_version=1.0.1 versions=[10000, 10001]`.

**Emulator state left:**
- `dpm list-owners` prints no owners, and no pilipos package is installed.
- `adb reverse --list` is empty, and the export folder is removed.
- The AVD was not wiped. The planner did not start the emulator (it was already running and was reused, PD218), so it is left running.

## Test counts and coverage

| Item | At `ba57adf` | At `4cfe0b3` |
|------|--------------|--------------|
| Kotlin inventory (`kotlin_test_inventory.py`) | 318 | 727 (removed 0) |
| `:app` host unit tests (Robolectric included) | 275 (P4 exit) | 683 |
| `:app` instrumented | 1 (`ProvisioningE2eTest`) | 2 classes (`ProvisioningE2eTest`, and `PlatformE2eTest` with one step per `am instrument` run) |
| `:pos-core` host unit tests + `mismatchTest` | 22 + 1 | 22 + 1 |
| `android/rust` tests | 236 | 236 |
| root workspace inventory | baseline | unchanged by construction (no root path changed, PD222) |
| `:app` Kover (floor 80/60) | LINE 95.0%, BRANCH 76.0% | LINE 94.2%, BRANCH 75.3% |
| gate self-tests | p4 75, heavy 11 | p4 75, heavy 11, p6 104, declare 42, plus the registry mock test |

## Hashes and references

| Item | Value |
|------|-------|
| exit head before the exit commit | `4cfe0b309947d243b480d188fd30cb9bc4fef6ec` |
| base | `ba57adf06191a1f386863f98f24930b6f5aa543c` |
| freeze tag | `linux-v1.1.0-rc1` -> `65b567fdd2497277300340979c2c61ff17abc918` |
| Linux SHA-1 reproduced | `9d230ef1bba90a4edde0157bcb107e4bb4d23d6e` |
| `:app` reproducible unsigned APK, two container builds at `986108b` | SHA-1 `f7c9aab317c0759aa92458e9e7ddda36704eb61f`, SHA-256 `ae9099e948d62a8d34336246c290353d97b093d255a6e0ca4a17bdb79c032e76`, 16002836 bytes, 108 entries (entry manifest sha256 `772ec13c...`); durations 1131 s and 1008 s |
| `libpos_ffi.so` in that APK (stripped) | SHA-1 `f761159ff3ec8f1f61b57bf7f12e944367d61949`, 12603048 bytes; unstripped cargo output `8a236b06f12e5ee458d40d2fef3d819ee40d504b`, 67730512 bytes |
| first `:app` repro of record (P6-T10, at `6c0142e`) | SHA-1 `a1c6039d0c8103a0daa674f558acb6ef4111126c` (in `REPRO.md`) |
| engine reference shown on the About screen | `pili-pos 1.1.0, linux-v1.1.0-rc1 (65b567f)` |
| `posSwVersion` on the emulator | `pili-pos-android/1.0.0-dev` |
| development minisign public key | `android/app/provisioning/dev-minisign.pub` (secret half only under `~/.cache/pili-android/dev-minisign/`) |
| release minisign key in `plat_update_keys.xml` | empty (every update refused in release, PD212) |

## Measured storage growth (answer to the SD card question)

The answer is in `PLATFORM.md` section 12, with the measured figures; the exit run reproduced them.

- **Where the SQLite lives.** `filesDir/engine/pos.db`, in app-private internal storage, never on the SD card (PD220). The `plat-manifest` gate check keeps it there: no `installLocation` and no storage permission.
- **What the card is for.** It is a removable volume for the SPF export and for the update from a stick, and later for the nightly export.
- **Growth.**
  - 6,471 bytes per one-line sale and 14,336 bytes per six-line sale.
  - So 236 to 523 MB a year at 100 sales a day, and 0.7 to 1.57 GB a year at 300.
  - The backups are a full copy after every Z, never deleted (PD219). They reach 40 to 268 GiB after a year, and 32 GiB between day 126 and day 325. This fills the till's internal storage within months unless a retention rule is added in the engine.
- **Recommendation.**
  - A microSDXC card, high endurance or A1/U3, formatted exFAT.
  - 64 GB for the first year at 100 sales a day.
  - 128 GB at 300 sales a day, or from the second year.

## Decisions taken while the user was away

PD201 to PD221 (planner, before the run, verbatim in `P6.md` and DECISIONS.md), the integrators' wave records (waves 3, 4 and 7) and, at this exit:

- **PD222** Step 7 ran under the non-app profile, because `app-only-scope` refuses `--app` after T10. Step 6 ran without `--root`, because the disk was too small. The root inventory is unchanged by construction and passed in waves 3 and 4.
- **PD223** `hygiene-paths` refuses `scripts/release/`, because `p4_gate.sh:310` passes no `--extra-paths`. The fix goes in a foundation card; the planner did not waive the check.
- **PD224** `nav/FeatureEntriesTest.kt` is the one foundation file the lane edits (two route lines). The wave 4 record's claim that the platform regex lists it is wrong. The UI lane does not touch the file.
- **PD225** The cause of the `KioskControllerTest` flake is a race with the P6-T3 `PolicyHook` started by the real `PiliApplication` under Robolectric. The fix is in the test (`HiltTestApplication`), and production is unaffected.
- **PD226** The watchdog reopens the engine as soon as `PlatformOps.busy` empties, and the update stays busy for 90 s after the commit.
- **PD227** The `:app` repro at `986108b` and the Linux repro of the tag count for the exit head. The exit head adds only docs.
- **PD228** No freeze step at the P6 exit.
- **PD229** The review findings carried forward are listed under "Risks" below.
- **PD230** The plan update for the GMS D3 Pro (hide the Play Store, `DISALLOW_INSTALL_APPS`, windowed system update policy, second AVD) is not in P6 and is a platform card for the next run.

## Risks

Non-blocking review findings, carried forward (PD229).

**Platform, for one platform follow-up card:**
- **Updater UX on a failure status.** `UpdateInstaller.kt:241-242` returns `Committed` whatever the status says, and `UpdateViewModel.kt:139` keeps the phase at Installing. After `STATUS_PENDING_USER_ACTION` or `STATUS_FAILURE*`, the engine is reopened, but the screen still says "installing, the till restarts". This is what a non-owner device or the real Sunmi installer would show.
- **Updater commit under cancellation.** If the view model is cleared between `opened.commit()` and the resume, `rollback()` abandons a session that was already committed, deletes the marker and audits `failed`, while the install still goes ahead. Run the commit under `NonCancellable`, or in the application scope.
- **Export in `viewModelScope`.** Leaving the screen during an export lets the export finish, `MANIFEST.sha256` included, but then audits `failed/cancelled` (`ExportViewModel.kt:133-151`, `Exporter.kt:107-108`).
- **SAF grant without persistable access.** `ExportFolderStore.kt:84` calls `takePersistableUriPermission` without a catch, so a tree result without a persistable grant would crash the kiosk activity. Low risk with the real picker.
- **`backups.txt`** does not list the fresh backup that becomes `pos.db` under its original name (documented in `PLATFORM.md` section 7).
- **File rotation during an export.** An audit or log file rotated between the plan and the copy fails the export with `READ_FAILED`. Each export also adds one backup.
- **`PlatformAudit.kt:112`** ignores a failed `renameTo` at rotation, so `audit.jsonl` could grow without limit. `RestartBudget.kt:39` writes without a temp file or fsync; it fails open by design.
- **About screen.**
  - `IdentityProvider.storageOf` sets the free bytes to 0 when `StatFs` throws, so the low-space warning appears for the wrong reason.
  - `aMissingLibraryHashShowsTheReason` does not check the shown text.
  - The warning says "2 Go" for 2 GiB.
- **`plat-manifest`** reads only the source manifest, so a library that merges in a storage permission would get past it.
- **Test-only.** `PlatformE2eEnv.dbLogical` opens `pos.db` with the framework SQLite while the engine's SQLite has the file open in the same process (a POSIX lock hazard, measurement step only).
- **Docs.**
  - `PLATFORM.md` section 3 should name the real key `no_physical_media`.
  - Its teardown sentence overclaims: the settings values stay after the owner is cleared.
  - Section 12 says "100 times" where the figures give about 180.
  - The `declare_android.sh` order (tag moved before `gh release upload`) could be noted in section 10.
- **`declare_android.sh --allow-dev`** writes the base version while the APK says `-dev`. This only affects the dry-run path.

**Foundation, for one foundation card alone in its wave (outside this lane):**
- PD223 `hygiene-paths` and `--extra-paths`.
- PD224 `FeatureEntriesTest` should read the routes from `PlatformRoutes`.
- PD225 `KioskControllerTest` on `HiltTestApplication`.
- `EngineEvents`: if the event stream ends while the state is still Opening, the engine becomes Ready with a dead stream, and nothing restarts it (the P6-T1 review).
- `ShellState.kt` shows `Closed(Shutdown)` as Opening, and `PolicyHook` does not reapply the full set when the app becomes owner while its process is already running (PD122, the P6-T3 review).

**Other risks:**
- **Internal storage (PD219).** The post-Z backups fill the till within months, as shown by the measurement above. A retention rule needs an engine change with the re-freeze. This is the most important open item of P6.
- **Update precondition (PD213).** It is interim and weaker than an engine `update_precheck`. It relies on the Z pair being two consecutive R events (`Z_PAIR_TYPE`, checked against `report.rs` and `signing.rs`).
- **Merge into `android/int`.**
  - `DECISIONS.md` and `LINUX_FREEZE.md` get appended sections from several lanes. The export lane also appends to `LINUX_FREEZE.md`, so expect a keep-both conflict at the end of that file.
  - `LANES.md` and `APP.md` are edited only by this lane (P6-T1).
  - After the export lane is merged, the freeze checks fail by design until the user re-freezes (X1D3). Then `p4_gate.sh` and `plat_about_engine_ref` must point at the new tag, and `plat-engine-ref` fails until they do.
- **Shared disk and daemons.** Gate runs on this Mac fail on "No space left on device" and on "Gradle build daemon has been stopped" (another agent's `./gradlew --stop`). The exit run had 11 to 17 GB free; `lane-export/target` holds about 51 GB. Rerun a Gradle check that fails this way before calling it a defect.
- **Hardware residue** (APP.md section 10, PLATFORM.md section 12):
  - the real SD card and USB stick: exFAT versus FAT32, removal during a write, the largest card the D3 Pro accepts;
  - the Sunmi PackageInstaller under device owner;
  - recovery and safe boot under the restriction set;
  - the real serial;
  - the free internal space of the real device;
  - Play Protect on the GMS variant.

## What the next phase must know

- **Merge order.** Merge `android/lane-plat` into `android/int` with `--no-ff`, with the UI lane, before `android/lane-export` (X1D3). Gate `android/int` with the P6 phase gate and the base `ba57adf`, once the UI lane has merged. Then gate it with `x1_gate.sh` after the export lane.
- **Fix the gate first.** One foundation card alone in its wave, for PD223, PD224 and PD225. Then every lane-wide gate is green without exceptions, and the flaky Robolectric test stops hiding real failures.
- **Platform follow-up card.** It takes the platform findings under "Risks" and the GMS items of PD230: hide the Play Store and other user-facing Google apps, `DISALLOW_INSTALL_APPS` (except the updater session), a windowed `SystemUpdatePolicy` outside service hours, and `plat_e2e.sh` on `sunmi_d3_pro_gms_api33` as well.
- **Contracts the UI lane and P7 use.**
  - Routes and rows: `platform/about` (900, everyone), `platform/export` (910, managers) and `platform/update` (920, managers), registered by `PlatformEntry` (LANES.md "Platform routes").
  - Lifecycle hooks: the `AppLifecycleHook` set (APP.md section 11).
  - Engine access: `EngineGateway.state` and `close()`.
  - Audit log: `filesDir/platform/audit.jsonl`.
  - Everything is documented in `PLATFORM.md`.
- **P7 runbook.** `plat_e2e.sh` already covers the SPF export, the updater and the crash restart on the emulator. The P7 D3 Pro runbook adds the 24 h soak, the power cut, the Epson tickets and the GMS emulator.
- **Release (P8, the user's).**
  - The release minisign key goes into `plat_update_keys.xml` (PD212).
  - The RSA APK signing key.
  - A real run of `declare_android.sh` (push of `declared-android`, `android-vX.Y.Z` release).
  - The one-line `declare.sh --android` dispatch once `feat/usb-autoinstall` is merged (PD216).
- **Freeze.** `linux-v1.1.0-rc1` is still the tag. The re-freeze `linux-v1.0.0-rc1` (candidate SHA-1 `d4fb09a2ae2cf2501fe4d6847eb72e954d0b95ed` from the export lane) is the user's. Retention of the post-Z backups (PD219) and an engine `update_precheck` (PD213) are best made in the same re-freeze.
- **Emulator.** Reuse a running `sunmi_d3_pro_api33`, never wipe it, and kill the process for the restart checks (PD118). `plat_e2e.sh` waits while another end-to-end run uses the emulator, and it leaves no owner, package, reverse rule or export folder behind.

## Questions for the user

1. **SD card (PD220).** Do you accept that the database never lives on the SD card? For exports: a microSDXC card, high endurance, formatted exFAT, 64 GB at about 100 sales a day in the first year, 128 GB at about 300 sales a day or from the second year.
2. **Backup retention (PD219).** How many post-Z backups should stay on the till, for example 14 days? Without a rule they fill the internal storage within months. This needs an engine change with the re-freeze.
3. **Update precheck (PD213).** Should an engine `update_precheck` (booking period closed, no pending sign) be added with the re-freeze?
4. **Power menu (PD206).** Do you confirm that the staff lose the power menu in production (lock task features NONE)?
5. **Declaration key (PD212, PD216).** Should `declared-android` reuse the Linux minisign declaration key, or get a second one? Who holds the RSA 4096 APK signing key?
6. **Package location (PD211).** Is `ghcr.io/nuagemagique-dev/pili-pos` with the tag `declared-android` the right public channel, given the brand-neutral dossier?
7. **Manager PIN (PD214).** Is the logged-in Admin session enough, or do you want a second PIN prompt before an update?
8. **GMS variant (PD230).** Should the next platform card hide the Play Store and other user-facing Google apps, block other installs, and set a windowed system update policy? If so, what are the service hours for that window?
