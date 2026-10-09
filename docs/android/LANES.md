# Lane ownership (Android port)

This file says who may edit which path of the Android port once the `:app` foundation (phase P4) is done. It is read by humans and by `scripts/android/lane_paths.py`, which turns the rules below into the path regex that every lane card passes to its gate and which fails when the rules and the tree disagree.

## Purpose

After P4, two lanes work in parallel on the same `android/app` module:

- the UI lane: P5 run A (shared components, theme) and P5 run B (one worker per view);
- the platform lane: P6 (SPF export, About, updater, device-owner restrictions, watchdog, release tooling).

The app is one Gradle module, `:app`, with one Kotlin package per area (decision PD101). One module keeps the Hilt graph, the manifest and the build files in a single place, and the packages give each lane a directory it owns outright. Two workers therefore never edit the same file: every file has exactly one owner (table below), and the few files that many lanes need are either append-only (`shared`) or reached through a contract (section "Contracts") instead of an edit.

Lanes start after P4. Until the P4 exit report is written, the foundation owner may touch any foundation path; from then on the foundation is frozen for the lanes.

## Owners

| Owner | Who | Rule |
| --- | --- | --- |
| `foundation` | P4 | Frozen for the lanes. Changed only through a card marked shared (section "Shared changes"). |
| `ui-common` | P5 run A | Theme, shared components, fonts, `strings_ui_common.xml`, the P5 gate, the parity index. |
| `ui-login`, `ui-tables`, `ui-order`, `ui-checkout`, `ui-reports`, `ui-settings`, `ui-webshop` | P5 run B, one worker each | The `feature/<view>/` package, its tests, its screenshots, its `strings_ui_<view>.xml`, its parity file. |
| `platform` | P6 | The `platform/` package, `strings_plat_*.xml`, platform XML and drawables, Docker, release tooling, the CI workflow. |
| `nightly` | X1 (export lane) | The `nightly/` package, its tests, its screenshots, `strings_nightly.xml`. |
| `e2e-parity` | P7 (end-to-end lane) | UC scenarios and runners, `android/uccheck`, soak, power cut, GMS run, runbook and parity docs. |
| `shared` | every lane | Append-only. Today: `docs/android/DECISIONS.md`. |
| `frozen` | nobody | Linux build inputs, the FFI and the P3 modules. A change is an escalation to the user, never a card. |

The owner of a path is the owner of the single rule that matches it. A path matching rules of two different owners is a defect in this file and `lane_paths.py --check` fails. A path matching two rules of the same owner is fine.

### Machine-readable rules

One rule per line: `<owner> <regex>`, separated by whitespace. Blank lines and lines starting with `#` are ignored. The regex is a POSIX ERE fragment that the checker anchors as `^(regex)$`. It must work both in Python `re` and in bash `[[ =~ ]]`: no lookahead, no backslash classes, a literal dot is written `[.]`.

```lanes
# foundation, ui-common, ui-<view>, platform, shared, frozen
foundation android/(settings[.]gradle[.]kts|build[.]gradle[.]kts|gradle[.]properties|[.]editorconfig|[.]gitignore|gradlew|gradlew[.]bat)
foundation android/gradle/.*
foundation android/config/.*
foundation android/app/(build[.]gradle[.]kts|proguard-rules[.]pro|lint[.]xml|[.]gitignore)
foundation android/app/provisioning/.*
foundation android/app/src/main/AndroidManifest[.]xml
foundation android/app/src/debug/.*
foundation android/app/src/main/kotlin/dev/nuagemagique/pilipos/[A-Za-z]+[.]kt
foundation android/app/src/main/kotlin/dev/nuagemagique/pilipos/(di|engine|provisioning|kiosk|shell|nav|i18n)/.*
foundation android/app/src/test/kotlin/dev/nuagemagique/pilipos/[A-Za-z]+[.]kt
foundation android/app/src/test/kotlin/dev/nuagemagique/pilipos/(di|engine|provisioning|kiosk|shell|nav|i18n|testing)/.*
foundation android/app/src/androidTest/(AndroidManifest[.]xml|kotlin/dev/nuagemagique/pilipos/(e2e|testing)/.*)
foundation android/app/src/test/resources/(robolectric[.]properties|vue_strings[.]json)
foundation android/app/src/main/res/values(-[a-z]+)?/strings_(vue_[a-z]+|shell|engine|provisioning|kiosk)[.]xml
foundation android/app/src/main/res/xml/(device_admin|network_security_config)[.]xml
foundation android/app/src/main/res/(mipmap[^/]*|drawable[^/]*)/(ic_launcher|shell_)[^/]*
foundation scripts/android/(p4_gate[.]sh|test_p4_gate[.]sh|heavy[.]sh|test_heavy[.]sh|lane_paths[.]py|test_lane_paths[.]py|kotlin_test_inventory[.]py|test_kotlin_test_inventory[.]py|kotlin_test_moves[.]tsv|vue_to_android[.]mjs|test_vue_to_android[.]mjs|make_dev_bundle[.]sh|test_make_dev_bundle[.]sh|app_e2e[.]sh)
foundation scripts/android/testdata/vue_to_android/.*
foundation docs/android/(LANES|APP)[.]md
foundation docs/android/phases/P4[^/]*[.]md
ui-common android/app/src/(main|test|androidTest)/kotlin/dev/nuagemagique/pilipos/ui/.*
ui-common android/app/src/test/screenshots/(theme|components)/.*
ui-common android/app/src/main/res/values(-[a-z]+)?/strings_ui_common[.]xml
ui-common android/app/src/main/res/font/.*
ui-common android/app/src/main/res/(drawable[^/]*)/ui_[^/]*
ui-common scripts/android/(p5_gate[.]sh|test_p5_gate[.]sh|check_ui_parity[.]py|test_check_ui_parity[.]py)
ui-common docs/android/PARITY[.]md
ui-common docs/android/phases/P5[^/]*[.]md
ui-login android/app/src/(main|test|androidTest)/kotlin/dev/nuagemagique/pilipos/feature/login/.*
ui-login android/app/src/test/screenshots/login/.*
ui-login android/app/src/main/res/values(-[a-z]+)?/strings_ui_login[.]xml
ui-login docs/android/parity/login[.]md
ui-tables android/app/src/(main|test|androidTest)/kotlin/dev/nuagemagique/pilipos/feature/tables/.*
ui-tables android/app/src/test/screenshots/tables/.*
ui-tables android/app/src/main/res/values(-[a-z]+)?/strings_ui_tables[.]xml
ui-tables docs/android/parity/tables[.]md
ui-order android/app/src/(main|test|androidTest)/kotlin/dev/nuagemagique/pilipos/feature/order/.*
ui-order android/app/src/test/screenshots/order/.*
ui-order android/app/src/main/res/values(-[a-z]+)?/strings_ui_order[.]xml
ui-order docs/android/parity/order[.]md
ui-checkout android/app/src/(main|test|androidTest)/kotlin/dev/nuagemagique/pilipos/feature/checkout/.*
ui-checkout android/app/src/test/screenshots/checkout/.*
ui-checkout android/app/src/main/res/values(-[a-z]+)?/strings_ui_checkout[.]xml
ui-checkout docs/android/parity/checkout[.]md
ui-reports android/app/src/(main|test|androidTest)/kotlin/dev/nuagemagique/pilipos/feature/reports/.*
ui-reports android/app/src/test/screenshots/reports/.*
ui-reports android/app/src/main/res/values(-[a-z]+)?/strings_ui_reports[.]xml
ui-reports docs/android/parity/reports[.]md
ui-settings android/app/src/(main|test|androidTest)/kotlin/dev/nuagemagique/pilipos/feature/settings/.*
ui-settings android/app/src/test/screenshots/settings/.*
ui-settings android/app/src/main/res/values(-[a-z]+)?/strings_ui_settings[.]xml
ui-settings docs/android/parity/settings[.]md
ui-webshop android/app/src/(main|test|androidTest)/kotlin/dev/nuagemagique/pilipos/feature/webshop/.*
ui-webshop android/app/src/test/screenshots/webshop/.*
ui-webshop android/app/src/main/res/values(-[a-z]+)?/strings_ui_webshop[.]xml
ui-webshop docs/android/parity/webshop[.]md
platform android/app/src/(main|test|androidTest)/kotlin/dev/nuagemagique/pilipos/platform/.*
platform android/app/src/main/res/values(-[a-z]+)?/strings_plat_[a-z]+[.]xml
platform android/app/src/main/res/xml/(data_extraction_rules|plat_[a-z_]+)[.]xml
platform android/app/src/main/res/(drawable[^/]*)/plat_[^/]*
platform android/docker/.*
platform scripts/android/(p6_gate[.]sh|test_p6_gate[.]sh|check_repro[.]sh|apk_entry_diff[.]py|test_apk_entry_diff[.]py|build_image[.]sh)
platform scripts/release/.*
platform [.]github/workflows/android[.]yml
platform docs/android/(REPRO|PLATFORM)[.]md
platform docs/android/phases/P6[^/]*[.]md
nightly android/app/src/(main|test|androidTest)/kotlin/dev/nuagemagique/pilipos/nightly/.*
nightly android/app/src/test/screenshots/nightly/.*
nightly android/app/src/main/res/values(-[a-z]+)?/strings_nightly[.]xml
platform scripts/android/(test_)?plat_[a-z0-9_]+[.](sh|py)
platform scripts/android/testdata/plat_[a-z0-9_]+/.*
platform android/app/src/test/screenshots/platform/.*
platform android/app/src/test/resources/platform/.*
shared docs/android/DECISIONS[.]md
frozen android/(rust|pos-core|smoke)/.*
frozen (Cargo[.]toml|Cargo[.]lock|rust-toolchain[.]toml)
frozen (src-tauri|ui|crates|deploy)/.*
frozen scripts/(build_and_hash|check_repro)[.]sh
frozen docs/android/baseline/.*
frozen docs/SUBMISSION_ARTIFACTS/.*
e2e-parity android/app/src/androidTest/kotlin/dev/nuagemagique/pilipos/(uc|soak|powercut)/.*
e2e-parity android/app/src/androidTest/assets/uc/.*
e2e-parity android/uccheck/.*
e2e-parity scripts/android/(test_)?(p7|uc|soak|powercut|fdm_delay)_[a-z0-9_]+[.](sh|py|toml|txt)
e2e-parity scripts/android/testdata/(p7|uc|soak|powercut|fdm_delay)_[a-z0-9_]+/.*
e2e-parity docs/android/(UC|UC_MATRIX_ANDROID|UC_RESULTS|SOAK|POWERCUT|GMS|RUNBOOK_D3PRO|LIVE_MODULE2)[.]md
e2e-parity docs/android/uc/.*
e2e-parity docs/android/phases/P7[^/]*[.]md
```

### Probes

Each probe is `<path> <owner>`. The checker requires every probe to match exactly one owner and that owner to be the one written here. The probes pin the intent of the rules: a regex edit that moves a file to another lane fails the check.

```lane-probes
android/app/build.gradle.kts foundation
android/gradle/libs.versions.toml foundation
android/gradle/verification-metadata.xml foundation
android/gradle/wrapper/gradle-wrapper.properties foundation
android/config/detekt/detekt.yml foundation
android/app/src/main/AndroidManifest.xml foundation
android/app/src/debug/AndroidManifest.xml foundation
android/app/src/debug/kotlin/dev/nuagemagique/pilipos/debugtools/DebugTeardownReceiver.kt foundation
android/app/src/main/kotlin/dev/nuagemagique/pilipos/MainActivity.kt foundation
android/app/src/main/kotlin/dev/nuagemagique/pilipos/PiliApplication.kt foundation
android/app/src/main/kotlin/dev/nuagemagique/pilipos/di/CoroutinesModule.kt foundation
android/app/src/main/kotlin/dev/nuagemagique/pilipos/engine/LaunchConfig.kt foundation
android/app/src/main/kotlin/dev/nuagemagique/pilipos/engine/EngineService.kt foundation
android/app/src/main/kotlin/dev/nuagemagique/pilipos/engine/EngineHolder.kt foundation
android/app/src/main/kotlin/dev/nuagemagique/pilipos/provisioning/PiliAdminReceiver.kt foundation
android/app/src/main/kotlin/dev/nuagemagique/pilipos/provisioning/ProvisioningStore.kt foundation
android/app/src/main/kotlin/dev/nuagemagique/pilipos/shell/ShellRoot.kt foundation
android/app/src/main/kotlin/dev/nuagemagique/pilipos/nav/AppNavHost.kt foundation
android/app/src/main/kotlin/dev/nuagemagique/pilipos/kiosk/KioskController.kt foundation
android/app/src/main/kotlin/dev/nuagemagique/pilipos/i18n/generated/VueStringIds.kt foundation
android/app/src/test/kotlin/dev/nuagemagique/pilipos/MainActivityTest.kt foundation
android/app/src/test/kotlin/dev/nuagemagique/pilipos/engine/EngineHolderTest.kt foundation
android/app/src/test/resources/robolectric.properties foundation
android/app/src/test/resources/vue_strings.json foundation
android/app/src/main/res/values/strings_shell.xml foundation
android/app/src/main/res/values-zh/strings_vue_core.xml foundation
android/app/src/main/res/values-en/strings_provisioning.xml foundation
android/app/src/main/res/xml/device_admin.xml foundation
android/app/src/main/res/xml/network_security_config.xml foundation
android/app/provisioning/dev-minisign.pub foundation
android/app/src/androidTest/kotlin/dev/nuagemagique/pilipos/e2e/ProvisioningE2eTest.kt foundation
scripts/android/p4_gate.sh foundation
scripts/android/vue_to_android.mjs foundation
docs/android/LANES.md foundation
docs/android/phases/P4-exit.md foundation
android/app/src/main/kotlin/dev/nuagemagique/pilipos/ui/theme/PiliTheme.kt ui-common
android/app/src/main/kotlin/dev/nuagemagique/pilipos/ui/components/NumPad.kt ui-common
android/app/src/test/screenshots/theme/swatch.png ui-common
android/app/src/main/res/values-zh/strings_ui_common.xml ui-common
android/app/src/main/res/font/noto_sans_sc_regular.ttf ui-common
scripts/android/p5_gate.sh ui-common
docs/android/PARITY.md ui-common
docs/android/phases/P5b.md ui-common
android/app/src/main/kotlin/dev/nuagemagique/pilipos/feature/login/LoginRoutes.kt ui-login
android/app/src/test/kotlin/dev/nuagemagique/pilipos/feature/tables/TablesScreenTest.kt ui-tables
android/app/src/test/screenshots/order/order_empty.png ui-order
android/app/src/main/kotlin/dev/nuagemagique/pilipos/feature/checkout/CheckoutViewModel.kt ui-checkout
android/app/src/main/res/values/strings_ui_reports.xml ui-reports
docs/android/parity/settings.md ui-settings
android/app/src/main/kotlin/dev/nuagemagique/pilipos/feature/webshop/WebshopScreen.kt ui-webshop
android/app/src/main/kotlin/dev/nuagemagique/pilipos/platform/update/InstallResultReceiver.kt platform
android/app/src/main/kotlin/dev/nuagemagique/pilipos/platform/PlatformRoutes.kt platform
android/app/src/androidTest/kotlin/dev/nuagemagique/pilipos/platform/UpdaterDeviceTest.kt platform
android/app/src/main/res/values-en/strings_plat_about.xml platform
android/app/src/main/res/xml/data_extraction_rules.xml platform
android/docker/Dockerfile platform
scripts/android/check_repro.sh platform
.github/workflows/android.yml platform
docs/android/phases/P6-exit.md platform
android/app/src/main/kotlin/dev/nuagemagique/pilipos/nightly/NightlyScreen.kt nightly
android/app/src/main/res/values-zh/strings_nightly.xml nightly
scripts/android/plat_e2e.sh platform
scripts/android/test_plat_registry_mock.py platform
scripts/android/testdata/plat_registry/manifest.json platform
android/app/src/test/screenshots/platform/about.png platform
android/app/src/test/resources/platform/declared.json platform
android/app/src/main/kotlin/dev/nuagemagique/pilipos/di/AppLifecycleHook.kt foundation
docs/android/DECISIONS.md shared
android/rust/pos-ffi/src/core.rs frozen
android/pos-core/build.gradle.kts frozen
src-tauri/src/main.rs frozen
crates/pos-engine/src/engine.rs frozen
scripts/check_repro.sh frozen
Cargo.lock frozen
android/app/src/androidTest/kotlin/dev/nuagemagique/pilipos/uc/UcScenarioTest.kt e2e-parity
android/app/src/androidTest/kotlin/dev/nuagemagique/pilipos/soak/SoakDriverTest.kt e2e-parity
android/app/src/androidTest/kotlin/dev/nuagemagique/pilipos/powercut/PowercutSteps.kt e2e-parity
android/app/src/androidTest/assets/uc/00_smoke/001_SMOKE.json e2e-parity
android/uccheck/Cargo.toml e2e-parity
android/uccheck/pos-uc-check/src/main.rs e2e-parity
scripts/android/p7_gate.sh e2e-parity
scripts/android/test_p7_gate.sh e2e-parity
scripts/android/uc_e2e.sh e2e-parity
scripts/android/uc_scope_android.toml e2e-parity
scripts/android/uc_lock_parity_allow.txt e2e-parity
scripts/android/fdm_delay_proxy.py e2e-parity
docs/android/UC_MATRIX_ANDROID.md e2e-parity
docs/android/uc/results.json e2e-parity
docs/android/phases/P7.md e2e-parity
```

## Created by vs owned by

A few files are created by a P4 card but belong to a lane. The creating card writes them as stubs, then ownership passes to the lane at once (the rules above already say so).

| Path | Created by | Owned by |
| --- | --- | --- |
| `ui/theme/` (`PiliTheme` and tokens) | P4-T8 | `ui-common` |
| `feature/<view>/` placeholders (route object, empty screen) | P4-T9 | `ui-<view>` |
| `platform/` stubs and `PlatformRoutes` | P4-T2, P4-T9 | `platform` |
| `res/xml/data_extraction_rules.xml` | P4-T2 | `platform` |

## Contracts the lanes use instead of editing foundation files

The foundation exposes extension points so that a lane adds behaviour by adding files in its own package.

- A screen: a `FeatureEntry` bound `@IntoSet` in a Hilt module of the lane's package. Route constants live in the lane's own `<Feature>Routes` object, never in `nav/`.
- A Settings row: a `SettingsEntry` bound `@IntoSet`.
- An overlay above every screen: a `ShellOverlay` bound `@IntoSet`.
- Engine calls: only through `EngineGateway.call { }` and `EngineHelpers`. No lane reaches the FFI or `EngineHolder` directly.
- Engine events: from `EngineEvents.events`.
- Vue texts: through `MessageCatalog`. Error texts through `ErrorTexts` and `FdmTexts`.
- Theme: through `PiliTheme`. No lane hard-codes colors, spacing or type sizes.

## Hand-written strings

Each area has one hand-written strings file per locale folder, with a fixed name and a fixed resource-name prefix, so that two lanes never share a file and never collide on a resource name:

| File | Prefix of every `string`, `plurals` and `string-array` name |
| --- | --- |
| `strings_shell.xml` | `shell_` |
| `strings_engine.xml` | `engine_` |
| `strings_provisioning.xml` | `prov_` |
| `strings_kiosk.xml` | `kiosk_` |
| `strings_ui_<area>.xml` (`<area>` = `common` or a view) | `ui_<area>_` |
| `strings_plat_<area>.xml` | `plat_<area>_` |
| `strings_nightly.xml` | `nightly_` |

The files `strings_vue_*.xml` are generated by `scripts/android/vue_to_android.mjs` from the Vue catalogs. They are never edited by hand and are exempt from the prefix check.

`lane_paths.py --check` fails on a strings file with an unknown name and on an entry without its prefix.

## Shared changes

The build files (`build.gradle.kts`, `libs.versions.toml`, `gradle.properties`), `verification-metadata.xml`, the manifest, `src/debug` and the foundation packages change after P4 only through a card marked shared. A shared card runs alone in its wave. A lane that needs such a change stops and asks its planner; it does not edit the file and does not work around the rule.

## DECISIONS.md

`docs/android/DECISIONS.md` is `shared` and append-only. Each lane appends its own block at the end of the section "Decided while the user was away (review)". Existing text is never rewritten. When two lanes append at the same time, the merge keeps both blocks.

## Parity docs

P5 run B writes one file per view: `docs/android/parity/<view>.md`, owned by `ui-<view>`. `docs/android/PARITY.md` is the index and is owned by `ui-common`.

## Gate use

A lane card passes its own paths to the allowed-paths check of its gate:

```
--allow "$(python3 scripts/android/lane_paths.py --regex ui-login)"
```

and may add a card-specific regex next to it. `--regex ui` prints the regex of `ui-common` plus every `ui-<view>`; `--regex platform` the platform regex. The lane gates `p5_gate.sh` and `p6_gate.sh` build on `p4_gate.sh`, so a lane card still runs the whole foundation gate.

Other modes of the checker: `--check` (validates this file against the tree), `--owner PATH` (prints the owner, or `none`; exit 1 when two owners match), `--list-owners`.

## Platform routes

The platform lane registers its three screens through one `PlatformEntry` (a `FeatureEntry` bound `@IntoSet` in the `platform/` package). Route constants live in `PlatformRoutes`, never in `nav/`.

| Route | Row id | Order | Who sees it |
| --- | --- | --- | --- |
| `platform/about` | `platform.about` | 900 | everyone |
| `platform/export` | `platform.export` | 910 | managers |
| `platform/update` | `platform.update` | 920 | managers |

### AppLifecycleHook

`AppLifecycleHook` (`di/AppLifecycleHook.kt`, PD203) is the contract for behaviour that must run at the application and activity lifecycle points: `onApplicationCreated` at the end of `PiliApplication.onCreate`, `onActivityCreated` right after `KioskController.prepare()`, `onActivityResumed` after the Lock Task relock and the immersive bars and before the engine service starts. Any lane may bind a hook, `@Binds @IntoSet` in a Hilt module of its own package; nobody edits `PiliApplication`, `MainActivity` or `HooksModule` for it. The runner calls the hooks by `order`, then class name, each in its own try/catch, so a failing hook is logged and never stops the start or the next hook.

## End-to-end lane (P7)

The end-to-end lane (phase P7, parallel to the P8 dossier lane) owns the `e2e-parity` paths of the rules above:

- the `uc/`, `soak/` and `powercut/` packages of the androidTest source set and the scenario assets under `androidTest/assets/uc/`;
- the third Cargo workspace `android/uccheck` (the host reference run and the comparison tool `pos-uc-check`);
- the scripts `scripts/android/(test_)?(p7|uc|soak|powercut|fdm_delay)_*` and their fixtures `scripts/android/testdata/<same prefix>_*/`;
- the docs `UC`, `UC_MATRIX_ANDROID`, `UC_RESULTS`, `SOAK`, `POWERCUT`, `GMS`, `RUNBOOK_D3PRO` and `LIVE_MODULE2` (all `.md` under `docs/android/`), the folder `docs/android/uc/` and the phase file `docs/android/phases/P7*.md`.

The lane edits no foundation file after the card P7-T1 that registered it: no `:app` main or debug source set, no build file, no manifest, no foundation, UI, platform or nightly package. A lane card that needs such a change stops and asks its planner (section "Shared changes"). `docs/android/DECISIONS.md` stays append-only and shared.

### P7 emulator lock contract

Every P7 script that uses the emulator (`uc_e2e.sh`, `p7_soak.sh`, `p7_powercut.sh`, `p7_gms.sh`) takes the P7 lock from `scripts/android/p7_emulator_lock.sh` before it touches a device, by calling `p7_emulator_lock NAME SCRIPT_PATH [ARGS...]` at the top of the script:

- the lock file is `$HOME/.cache/pili-android/emulator/p7-use.lock` (override `PILI_P7_LOCK_DIR`), taken with `/usr/bin/lockf`;
- the call first waits while `app_e2e.sh`, `plat_e2e.sh`, `device_engine_tests.sh` or `emulator_smoke.sh` runs (a process that is not the caller or one of its ancestors), at most `PILI_P7_EMU_WAIT` seconds (default 5400), then re-executes the script under the lock with `PILI_P7_LOCK_HELD=1`;
- a nested P7 script (`PILI_P7_LOCK_HELD=1`) returns at once and runs under its parent's lock;
- on timeout it prints `P7LOCK | busy | held by <holder>` and exits 75;
- the holder (name, pid, start time) is written to `p7-use.holder`, removed by `p7_emulator_lock_cleanup`; `bash scripts/android/p7_emulator_lock.sh --status` prints `free` or the holder line.

### P7 output tags

Each P7 script ends with one `<TAG> | total | PASS|FAIL | ...` line and exits 1 on any FAIL: `UCE2E` (`uc_e2e.sh`), `UCREC` (`pos-uc-check record`), `UCCHECK` (`pos-uc-check compare`), `SOAK` (`p7_soak.sh report`), `POWERCUT` (`p7_powercut.sh`), `GMS` (`p7_gms.sh`), `GATE` (`p7_gate.sh`). The lock prints only its own `P7LOCK | busy | held by <holder>` line (exit 75) and `P7LOCK | wait | ...` progress lines on stderr. Ports on the host seen from the device through `adb reverse`: 19380 FDM (or the delay proxy), 19390 printer, 19391 printer HTTP jobs (P7.md section 4.7).
