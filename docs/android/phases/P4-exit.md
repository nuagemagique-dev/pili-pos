# P4 exit report: App foundation and provisioning

> Correction (2026-10-10): the Linux build is not filed as v1.1 under Art. 113. Nothing was filed, so Linux and Android are both version 1.0.0 initial filings (Art. 99), and the freeze tag is `linux-v1.0.0-rc1`, not `linux-v1.1.0-rc1` (`docs/android/DECISIONS.md`, "Decided by the user"). The report below is unchanged.

Written by the P4 planner on 2026-10-08, at the end of the P4 run, with the user away (PD115).

- **Base:** `android/int` at `30b32a0` (`chore(android): merge the Linux v1.1.0-rc1 soft freeze`).
- **Exit head:** `android/int` at `96c99f1` (`chore(android): merge P4-T12 restart-check fixes`) before the exit commit, then the docs-only commit that adds `phases/P4.md`, this report, the exit decisions (PD116 to PD122) and the P4 exit record in `LINUX_FREEZE.md`.
- **Cards:** `docs/android/phases/P4.md`. Decisions: `docs/android/DECISIONS.md`, "Decided while the user was away (review)" (PD99 to PD115 from the planner, copied by P4-T1; PD116 to PD122 at this exit). New references for the lanes: `docs/android/LANES.md` (ownership, `--allow` regexes) and `docs/android/APP.md` (module, engine runtime, provisioning, kiosk, navigation contracts, gates, e2e run).
- main (`96728e1`) and `cert/v1.0` (`d6210ca`) were not touched. Nothing was pushed, no GitHub Actions run, no live module2, HubRise, Scaleway or SPF call, no `declare.sh`, no release key (only the development minisign key, whose secret half lives under `~/.cache/pili-android/dev-minisign/`, and the throwaway debug signing key outside the repo), no tag.

## Verdict

- **All twelve cards are APPROVE and merged** into `android/int` with `--no-ff`. No rework round, no escalation, no conflict, no integration fix commit. P4-T12 was merged twice: its first three commits (merge `1554b3d`), then its two restart-check fixes (merge `96c99f1`).
- **`android/int` builds and the phase gate passes.** The planner reran `p4_gate.sh --exit --android --base 30b32a0` at `96c99f1`: **GATE | total | PASS | 0 failed (55 checks)**. The integrator's runs at the same commit cover `--exit --root --android` (0 failed of 56, root inventory 0 removed), `--fast --app-e2e` (0 failed of 22; the e2e run itself 11 of 11 on `sunmi_d3_pro_api33`), `--fast --linux-repro` (Linux SHA-1 `9d230ef1bba90a4edde0157bcb107e4bb4d23d6e` reproduced) and the forced Gradle rerun (`:pos-core:testDebugUnitTest`, `mismatchTest`, `apiCheck`, `:app:testDebugUnitTest`, all `--rerun`, BUILD SUCCESSFUL).
- **The plan's P4 gate holds:** `./gradlew lint ktlintCheck detekt test koverVerifyDebug assembleRelease` (check `plan-gradle`) and `check_linux_frozen.sh` clean against `linux-v1.1.0-rc1`.
- **The app runs on the emulator end to end:** debug app installed, device owner, NotProvisioned, signed development bundle imported from the debug inbox, engine Ready with `pili-pos-android/1.0.0-dev`, deviceId equal to `Build.getSerial()`, Lock Task LOCKED, a real touch forwarded as user activity, manager login, an `FdmStatus` event with `health.connected` from the FDM mock, a real process kill coming back Ready from the kept record without import and relocked, then a full teardown.
- **No freeze step at this exit** (PD121): the plan requires none at the end of P4, and the plan's 2026-10-08 update moves the next freeze to `linux-v1.0.0-rc1`, after the nightly export feature, with the user.

**Planner recommendation:** the UI lane (P5) and the platform lane (P6) can start on `android/int` at the exit commit (PD115), after one foundation fix card that runs alone (PD122). Merging into main, the re-freeze, pushes and the release keys wait for the user.

## Task status

| Card | Wave | Card commits (base..tip) | Review | Merge on `android/int` |
|------|------|--------------------------|--------|------------------------|
| P4-T1 gate tools (`p4_gate.sh`, `heavy.sh`, `kotlin_test_inventory.py`, `p2_gate.sh --hygiene-only`, `frozen-provisional` retired, `p3_gate.sh` self path, PD99 to PD115) | 1 | `30b32a0..fbac516` (5) | APPROVE | `2115565` |
| P4-T2 `:app` skeleton (catalog, build file, manifest, stubs, detekt on androidTest, dependency verification) | 1 | `30b32a0..09f88f5` (3) | APPROVE | `359c23e` |
| P4-T4 lanes (`LANES.md`, `lane_paths.py`) | 1 | `30b32a0..4c88b21` (2) | APPROVE | `556063c` |
| P4-T3 Vue strings converter | 2 | `556063c..6d463fe` (3) | APPROVE | `f9c2571` |
| P4-T5 engine runtime | 2 | `556063c..63e6d3f` (4) | APPROVE | `f88682a` |
| P4-T6 provisioning | 2 | `556063c..8bbddf4` (6) | APPROVE | `0f1436e` |
| P4-T8 theme | 2 | `556063c..15318fe` (2) | APPROVE | `a05d101` |
| P4-T9 navigation | 2 | `556063c..5cd5b7e` (4) | APPROVE | `13a4441` |
| P4-T7 i18n Kotlin | 3 | `13a4441..2c20627` (4) | APPROVE | `f28922b` |
| P4-T10 kiosk | 3 | `13a4441..37f7fb5` (4) | APPROVE | `0acca09` |
| P4-T11 shell | 3 | `9e5642f..b44a879` (3) | APPROVE | `ea347c2` |
| P4-T12 emulator e2e, `APP.md` | 4 | `ea347c2..d61b310` (4), then `5354d72`, `dd4fbdc` | APPROVE | `1554b3d`, `96c99f1` |

The card branches are kept locally; every P4 task worktree is removed. The ref `android/P4-P4-T10` points at `1554b3d` (it was moved forward to `android/int` after its merge); the card's own commits are `de9f285..37f7fb5`. Across P4: 47 non-merge commits, 206 files, +22490/-36 lines against `30b32a0`.

### What each card delivered (from the reviews)

- **T1.** `p4_gate.sh` (p3 checks plus `freeze-tag`, `hygiene-keys`, `lanes`, `vue-strings`, `kotlin-tests`, `ffi-boundary`, `app-manifest` and the `:app` Gradle checks; `--app`, `--exit`, `--app-e2e`, `--linux-repro`), `heavy.sh` (global build slots), `kotlin_test_inventory.py`, `p2_gate.sh --hygiene-only` and the `frozen-provisional` SKIP (PD100). Self-tests: p4 gate 75, heavy 11, inventory 13.
- **T2.** `android/app` (`dev.nuagemagique.pilipos`, debug `.debug`), version from pos-ffi `Cargo.toml` (`1.0.0-dev`, versionCode 10000), R8, Kover floor 80/60, Roborazzi and Robolectric SDK 35, the full PD103 manifest, stubs for every later card, detekt on androidTest, `verification-metadata.xml` 771 to 1385 checksums (host and container passes).
- **T4.** `LANES.md` (73 rules, 66 probes) and `lane_paths.py` (`--check`, `--owner`, `--regex`), 27 self-tests.
- **T3.** `vue_to_android.mjs`: 481 keys (core 211, errors 87, checkout 13, reports 25, settings 21, webshop 124), 47 FDM texts, error codes 63 + 22, per-area `strings_vue_*.xml` in fr, en, zh, generated Kotlin tables, test snapshot, `--check`; 19 node tests.
- **T5.** `EngineHolder` (Closed, Opening, Ready, Fatal; open under a Mutex, logging once per process, pump before Ready, 35 s shutdown with StillOpen), `EngineGateway`, `EngineEvents` (replay 0, buffer 64, SUSPEND), `EngineHelpers`, `EngineService` (`specialUse`, typed start with fallback, START_STICKY); 47 tests.
- **T6.** Device owner helpers, serial reader, bundle import (owner, key, serial, copy, `verifyBundle`, `checkConfigBundle`, database, atomic swap, staging always deleted, previous bundle restored on failure), `ProvisioningStore` (Ready only with this device's serial), development key (public half only), `make_dev_bundle.sh` (prehashed minisign, 12 self-tests).
- **T8.** Theme tokens from `tokens.css` (16 colours, 6 training overrides, spacing, radius, type), `PiliTheme`, `PiliDensity` (`Density(widthPx / 1920f, 1f)`), two Roborazzi goldens; 16 tests.
- **T9.** `FeatureEntry`, `SettingsEntry`, `ShellOverlay`, `NavRegistry` (duplicate and start-route checks), `AppNavHost`, `AppNavigator`, one placeholder per feature and `platform.about`; 26 tests.
- **T7.** `AppLanguage`, `MessageCatalog` (placeholders filled by hand, French fallback), `ErrorTexts` and `FdmTexts` (ports of `errorMessages.ts` and `fdmMessages.ts`); 43 tests, all 85 error codes in 3 languages.
- **T10.** `KioskController` (lock task packages, GLOBAL_ACTIONS, persistent HOME, launcher and Settings suspended, immersive, relock on resume), `UserActivityForwarder` (at most one call per second while Ready), `MainActivity` hooks, debug teardown receiver; 30 tests.
- **T11.** `ShellViewModel`, `ShellRoot`, provisioning, mismatch, owner-required, opening and fatal screens, `ProvisioningCoordinator` (close, import, reopen in finally); 30 tests.
- **T12.** `app_e2e.sh` (11 checks), `ProvisioningE2eTest`, `E2eEntryPoint` and `DebugKillReceiver` in the debug source set, `APP.md`.

## Phase gate evidence

Planner's own run on `96c99f1` (`main` `96728e1325f5e11d69b7571d17280e3027974e06`, `cert/v1.0` `d6210caabb32ca09f54be0f15410ce8447960be2`, `linux-v1.1.0-rc1^{commit}` `65b567fdd2497277300340979c2c61ff17abc918`, no `linux-v1.1.0` tag, the exit docs uncommitted in the tree):

| Step | Result |
|------|--------|
| clean tree, the three SHAs, no `linux-v1.1.0` | PASS |
| `check_linux_frozen.sh` | `frozen against linux-v1.1.0-rc1 (65b567f): 16 paths, clean` |
| `git diff --quiet 30b32a0 -- <frozen and barred paths, .github>` | empty |
| `lane_paths.py --check` | `LANES | total | PASS | 3383 files, 73 rules, 66 probes` |
| `kotlin_test_inventory.py --base 30b32a0` | `KTESTS | total | PASS | base 42, head 318, removed 0` |
| `p4_gate.sh --exit --android --base 30b32a0` | **GATE | total | PASS | 0 failed (55 checks)** |

Key lines of that run: `rust-tests` 236 passed; `lock-parity` 240 packages identical; `command-parity` missing none; `unit-tests` (`:pos-core`) 22 tests; `mismatch-test`, `api-check`, `aar-check`, `assemble-release`, `apk-check` (`:smoke`) PASS; `ndk-build` `libpos_ffi.so` 67760032 bytes, 0x4000 aligned; `freeze-tag` PASS; `hygiene-keys` PASS; `vue-strings` 481 keys, fdm 47, error codes 63 + 22; `ffi-boundary` PASS; `app-manifest` 6 permissions, exported components restricted; `app-unit-tests` 275 tests; `app-screenshots` PASS; `app-kover` LINE 95.0%, BRANCH 76.0%; `plan-gradle` PASS (`lint ktlintCheck detekt test koverVerifyDebug assembleRelease`, 291 actionable tasks); `app-assemble-release` and `app-apk-check` (`:app`, 6 checks) PASS.

Integrator runs at `96c99f1` (integrate-4): `p4_gate.sh --android` 0 failed of 52; `--fast --app-e2e` 0 failed of 22; `--exit --root --android` 0 failed of 56 (root inventory 0 removed); `--fast --linux-repro` SHA-1 `9d230ef1bba90a4edde0157bcb107e4bb4d23d6e`; forced Gradle rerun BUILD SUCCESSFUL. Its first `--android` run failed 12 Gradle checks with "SDK location not found" only because its shell had no `ANDROID_HOME`; it passed after the export.

### App end-to-end run (P4-T12 reviewer and integrator, `sunmi_d3_pro_api33`)

`APPE2E | total | PASS | 0 failed (11 checks)`: emulator, precheck, build, mocks, bundle, install, launch, push, instrumented, restart, teardown. The restart line: `APPE2E | restart | PASS | pid 9627 -> 9676 after the kill, Ready again without import (state=Ready state=Ready ), mLockTaskModeState=LOCKED`. The new pid opened `files/engine/pos.db` from the kept record with no NotProvisioned line and logged `kiosk: entering lock task`.

**Emulator state left:** `dpm list-owners` prints no owner, no pilipos package installed, `adb reverse --list` empty, no mock process. The AVD was not wiped. The integrator stopped the emulator with `emulator.sh stop` (chain-started marker); `adb devices` lists no device now.

## Test counts and coverage

| Item | At `30b32a0` | At `96c99f1` |
|------|--------------|--------------|
| Kotlin inventory (`kotlin_test_inventory.py`) | 42 | 318 (removed 0) |
| `:pos-core` host unit tests + `mismatchTest` | 22 + 1 | 22 + 1 |
| `:pos-core` instrumented | 19 | 19 |
| `:app` host unit tests (Robolectric included) | none | 275 |
| `:app` instrumented | none | 1 (`ProvisioningE2eTest`, run by `app_e2e.sh` with `am instrument`) |
| `android/rust` tests | 236 | 236 |
| `:app` Kover (floor 80/60) | none | LINE 95.0%, BRANCH 76.0% |
| engine error codes with fr, en, zh texts | | 85 (63 + 22 HubRise) |

## Hashes and references

| Item | Value |
|------|-------|
| exit head before the exit commit | `96c99f16c6125aa5c58b5175736c03c6dc666eb1` |
| base | `30b32a07155774ee04ce17d8a70f25928d98872e` |
| freeze tag | `linux-v1.1.0-rc1` -> `65b567fdd2497277300340979c2c61ff17abc918` |
| Linux SHA-1 reproduced | `9d230ef1bba90a4edde0157bcb107e4bb4d23d6e` |
| `:app` release APK, unsigned, host build (not a reproducible artifact) | SHA-1 `b8a6df54f45e8198c4c986789208572527dbbdfe`, 15560056 bytes (`android/app/build/outputs/apk/release/app-release-unsigned.apk`) |
| `:app` badging | release `dev.nuagemagique.pilipos` 10000 `1.0.0-dev`, debug `dev.nuagemagique.pilipos.debug` |
| development minisign public key | `android/app/provisioning/dev-minisign.pub` (secret half only under `~/.cache/pili-android/dev-minisign/`) |

## Decisions taken while the user was away

PD99 to PD115 (planner, before the run, verbatim in `P4.md` and DECISIONS.md) and, at this exit:

- **PD116** a card already merged is gated against its fork point, not the moved `android/int`.
- **PD117** P4-T5's one-line `StubsTest.kt` change outside its regex is accepted (the card required START_STICKY).
- **PD118** the e2e restart is a real process kill through the debug `DebugKillReceiver`; `am force-stop` is ignored for a device owner on API 33, so P6 must not rely on it.
- **PD119** the error-code count is 85 (63 + 22), not 107; PD105 and `P4.md` carry the wrong figure.
- **PD120** Kotlin treats any text starting with `FDM` as an FDM error; the Linux Vue regex probably misses `FDM_ANSWER_PENDING` (question for the user).
- **PD121** no freeze step at the P4 exit; the chain keeps `linux-v1.1.0-rc1` until the user re-freezes as `linux-v1.0.0-rc1`.
- **PD122** the non-blocking review findings in foundation files go to one foundation fix card that runs alone before the lanes branch; the restart-related ones go to P6.

## Risks

Non-blocking review findings, carried forward (PD122):

- **Restart and lifecycle (P6).** `EngineEvents.pump`: when the event stream ends with null while the state is still Ready (a timed-out shutdown finishing in the background), nothing reacts until the next call fails. `ShellState.kt:536` maps `Closed(Shutdown)` to Opening unconditionally, so if `EngineService.onDestroy` closes the engine while the activity is up, the shell spins until START_STICKY restarts the service.
- **Provisioning crash windows.** A crash between the two swap renames leaves NotProvisioned until the next import (only `BundleImporter.recoverInterruptedSwap` restores `bundle-old`, `ProvisioningStore.resolve` does not). A crash between the bundle move and `records.write` leaves the previous record with a stale manifest hash; the device binding is unaffected. Both are safe, not silent.
- **Kiosk.** `KioskController.release()` drops the stored suspended list even when the unsuspend step threw, and sets NotOwner while still owner. Debug teardown only today.
- **Shell.** `ShellViewModel.kt:604` reads the owner status only on a state change (OwnerRequired does not refresh after `dpm set-device-owner` until restart); `ShellViewModel.kt:622-625` launches `holder.open()` with no exception handler (an exception from `launchConfig()` would crash the process).
- **Tooling.** `ffi-boundary` checks one line at a time (a call split across lines or a function reference under a star import gets past it); `kotlin_test_inventory.py` gives every test of a file the file's first top-level class; `LANES.md` locale pattern `values(-[a-z]+)?` does not cover `values-zh-rCN` or `values-b+zh+Hans` (fails loudly as unowned); a lane rule with unbalanced parentheses would split the anchoring; `TokensMatchCssTest` compares only counts of mapped properties; the `app_e2e.sh` header comment still says force-stop; the force-stop bullet of `APP.md` section 10 sits in the hardware-residue list.
- **Process.** Integrator shells must export `ANDROID_HOME` and `ANDROID_NDK_HOME`; without them 12 Gradle checks fail with "SDK location not found". The e2e `--keep` debugging run of P4-T12 left a temporary bundle directory (dev bundle only, no key); the planner deleted it.
- **Hardware residue** (APP.md section 10): the real Sunmi serial and `getSerial()` on the D3 Pro, setup wizard and QR provisioning, safe-boot and recovery under the `DISALLOW_*` set, the real USB stick, the 24 hour survival run, the real panel's density and usable height, Chinese on the ticket.
- **Disk.** 31 GB free at the exit; `int/target` holds `android-rust` 3.7 GB, `android-rust-mismatch` 1.0 GB, `android-mocks` 0.8 GB (kept as warm-start sources for P5 and P6).

## What the next phases (P5, P6) must know

- **Start from the exit commit** of `android/int`. The lanes' ownership and `--allow` regexes come from `python3 scripts/android/lane_paths.py --regex <owner>` (`docs/android/LANES.md`); the contracts are in `docs/android/APP.md` section 7: a screen is a `FeatureEntry` `@IntoSet`, a Settings row a `SettingsEntry`, an overlay a `ShellOverlay`; engine calls through `EngineGateway.call { }`, helpers through `EngineHelpers`, events from `EngineEvents.events`; texts through `MessageCatalog`, `ErrorTexts`, `FdmTexts`; theme through `PiliTheme`. No lane edits a foundation file; the shared build files change only through a `shared` card alone in its wave.
- **Run one foundation fix card first** (PD122), alone in its wave, for the findings in `engine/`, `provisioning/`, `kiosk/`, `shell/` and `scripts/android/`, each fix with a regression test.
- **Gates.** Card: `p4_gate.sh --app --fast --base android/int --allow '<regex>'` per commit and `p4_gate.sh --app ...` at the end; a reviewer of an already merged card uses the card's fork point (PD116). Phase: the P4 phase gate with the next phase's base. The Kover floor 80/60 may only rise; branch coverage is 76.0%, so a lane adding untested branches can fail it.
- **Freeze.** Keep `check_linux_frozen.sh` in tag mode against `linux-v1.1.0-rc1` (PD121). If the nightly export feature (plan update 2026-10-08) lands in the engine, it changes Linux build inputs and pos-ffi; that needs the user and a re-freeze as `linux-v1.0.0-rc1`, and the P4 gates' freeze checks would then point at the new tag.
- **Emulator.** Restart checks kill the process (PD118); `am force-stop` does nothing for the device owner. `app_e2e.sh` leaves no owner, package or reverse rule behind; reuse a running AVD, never wipe it.
- **Fonts.** P5 run A bundles Noto Sans SC, JetBrains Mono and Archivo as `res/font` (PD106); P4 uses the system fonts.
- **Release.** Release builds refuse every bundle until `piliProvisioningPublicKey` is given (PD108); the real key and its holder are the user's.

## Questions for the user

1. **PD108 re-provision by signed bundle.** After a serial mismatch, the only way out is importing a bundle signed by the producer's key, standing in for DECISIONS 3.2's manager re-provision (the manager PIN needs an open engine). Is possession of a signed bundle acceptable as the re-provision proof?
2. **PD109 lock task features.** P4 keeps GLOBAL_ACTIONS (the power menu) in Lock Task; Q16 has P6 set NONE at every start. Confirm that the staff lose the power menu in production.
3. **Provisioning release key.** Who holds the minisign key that signs production bundles, and where (DECISIONS 9)? Release builds refuse every bundle until it exists.
4. **Fonts.** Bundle Noto Sans SC, JetBrains Mono and Archivo in the APK (size, licence notices), or rely on the system fonts that covered 439 of 439 code points in S4?
5. **PD120.** The Linux Vue UI probably shows `err_unexpected` for `FDM_ANSWER_PENDING`. Fix it with the v1.0 re-freeze?
6. **Re-freeze.** The plan renames the freeze to `linux-v1.0.0-rc1` after the nightly export feature. When should that feature and the re-freeze happen relative to P5 and P6?
7. **Merge into main.** Approve P4 for merging `android/int` into main (protocol step 5), or keep it on `android/int` until a later phase?

## The relayed SD card question

The user asked whether the SQLite database would live on an SD card in the Sunmi D3 Pro and how big the card should be. From the code at `96c99f1`: no. The engine opens `filesDir/engine/pos.db`, the installed bundle is under `filesDir/provisioning/bundle` and the logs under `filesDir/logs` (`provisioning/ProvisioningPaths.kt`), all app-private internal storage, excluded from backup (`allowBackup` false, an exclude-everything `data_extraction_rules.xml`). That is deliberate: a fiscal journal on removable media can be pulled out, swapped, corrupted or read by other apps. Removable media are only planned for the manager's SAF export of logs, backups and the SPF files (DECISIONS Q16, Q17; P6), and `DISALLOW_MOUNT_PHYSICAL_MEDIA` stays off for that. The database holds text rows for one till, so it should grow by megabytes per year, not gigabytes (not measured yet); a good-quality 16 or 32 GB card is ample for exports and backups. The largest card the D3 Pro accepts was not checked.
