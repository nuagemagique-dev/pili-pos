# The :app foundation (reference for the UI and platform lanes)

This is the map of the Android app after phase P4. The UI lane (P5) and the platform lane (P6) start from it. Who may edit which file is in `LANES.md`; why things are the way they are is in `DECISIONS.md` (PD101 to PD113 and Q13 to Q20). This file says what exists, how to use it and how it is checked.

## 1. Module and packages

One Gradle module, `:app` (decision PD101): package `dev.nuagemagique.pilipos`, debug applicationId `dev.nuagemagique.pilipos.debug`. The engine is reached only through `:pos-core` (the UniFFI facade); `:app` never calls `PosEngine.open` or a free FFI function outside `engine/` and `provisioning/` (gate check `ffi-boundary`).

| Package | What it holds | Owner (`LANES.md`) |
| --- | --- | --- |
| `di/` | coroutine scopes and dispatchers (`@ApplicationScope`, `@IoDispatcher`) | foundation |
| `engine/` | `EngineHolder`, `EngineGateway`, `EngineEvents`, `EngineHelpers`, `EngineService`, `LaunchConfig` | foundation |
| `provisioning/` | device owner, serial, bundle import, record, paths | foundation |
| `kiosk/` | Lock Task, launcher suspension, immersive bars, user activity forwarder | foundation |
| `shell/` | `ShellRoot`, `ShellViewModel`, provisioning and fatal screens | foundation |
| `nav/` | the navigation contracts (section 7) | foundation |
| `i18n/` | the strings pipeline runtime (section 6) | foundation |
| `ui/theme/` | tokens, `PiliTheme`, `PiliDensity` (section 6) | ui-common (P5 run A) |
| `feature/<view>/` | one screen area each: login, tables, order, checkout, reports, settings, webshop | ui-`<view>` (P5 run B) |
| `platform/` | About, updater, boot and package-replaced receivers, watchdog | platform (P6) |
| `debugtools/` (debug source set) | `DebugTeardownReceiver`, `DebugKillReceiver`, `E2eEntryPoint` | foundation |

`MainActivity` is the single activity (kiosk home). `PiliApplication` is the Hilt root and plants the Timber debug tree in debug builds only. Every feature is a placeholder screen until P5 fills it; the contracts around it are final.

## 2. Engine runtime

`EngineHolder` (singleton) owns the one engine of the process and publishes `state: StateFlow<EngineState>`:

| State | Meaning |
| --- | --- |
| `Closed(reason)` | no engine; reason is `Initial`, `NotProvisioned`, `DeviceMismatch(stored, actual)`, `DeviceIdUnavailable(detail)` or `Shutdown` |
| `Opening` | database, migrations and lock are being opened |
| `Ready(posSwVersion)` | the engine answers calls; the version is `pili-pos-android/<pos-ffi version>` |
| `Fatal(reason)` | an open failure or a Rust panic; no in-process reopen |

Rules:

- `open()` is idempotent and runs on the I/O dispatcher. `Ready`, `Opening` and `Fatal` are not touched. Without a provisioned launch configuration the engine is never started (no silent TEST identity). The event pump is started before `Ready` is published.
- `close()` calls the engine shutdown (its own bound is 30 s, the outer one 35 s). A refusal gives `CloseResult.StillOpen` and the engine keeps running; a clean close gives `Closed(Shutdown)`. `close()` on a non-Ready engine is a no-op.
- `markFatal(reason, cause)` is idempotent and starts a best-effort shutdown in the application scope.
- After `Fatal` the process must restart. P6 adds the restart (watchdog); until then the shell shows the fatal screen (`shell_fatal`).

`EngineGateway.call { engine -> ... }` is the only way to an engine method. It runs on the I/O dispatcher, throws `EngineUnavailableException(state)` when the engine is not Ready, lets `PosException` pass through unchanged (screens turn it into text with `ErrorTexts`) and turns an `InternalException` (Rust panic) into `Fatal`. `gateway.ready` is a `StateFlow<Boolean>`.

`EngineHelpers` wraps the stateless FFI helpers on the I/O dispatcher: `settlePreview`, `cashDueCents`, `vatSplit`, `qrMatrix`, `posSwVersion`. The UI computes no money; it calls these. They work before `Ready`.

`EngineEvents.events` is a hot `SharedFlow<PosEvent>` with no replay: subscribe before the call that causes the event. When a collector is slow the pump suspends, the Rust broadcast channel lags and the stream answers `PosEvent.Resync`; a collector treats `Resync` as "re-fetch the state" (PD107).

`EngineService` is a `specialUse` foreground service (Q15: typed `startForeground`, plain call as fallback). `MainActivity.onResume` starts it after the Lock Task relock, while the activity is visible. It opens the engine on start and closes it on destroy; the work runs in the application scope.

## 3. Provisioning

- **Device owner is required** on every build (PD108). `Build.getSerial()` needs it (Q13) and there is no non-owner path. `DeviceOwner` reads the role and grants the app its own runtime permissions (`READ_PHONE_STATE`, `POST_NOTIFICATIONS`). Without the role the shell shows `NeedsDeviceOwner`.
- **deviceId** is `Build.getSerial()`, read at import and at every start, never a stored copy. Unavailable (empty, `unknown`, longer than 600 characters, `SecurityException`) keeps the engine closed (`DeviceIdUnavailable`).
- **Mismatch.** The record keeps the serial of the import. A different serial now gives `DeviceMismatch(stored, actual)` and the engine stays closed. The only way out is a new signed import, which records the new serial.
- **Bundle layout** (a folder): `pos.json`, the files it names, an optional `seed.sql`, `MANIFEST.json` (format 1, files sorted with size and SHA-256) and `MANIFEST.json.minisig` (prehashed minisign, trusted comment).
- **Import steps** (`BundleImporter`, run by `ProvisioningCoordinator`: close engine, import, open engine): copy into `staging-<millis>` under limits (regular files, plain names, bounded depth, count and size, no symlink), `verifyBundle` against the build-time key, `checkConfigBundle` on `pos.json`, apply `seed.sql` with `installInitDb` only to a database that does not exist yet, then swap into `bundle/`. A failure at any step keeps the previous bundle and leaves no staging directory. An interrupted swap is repaired at the next import. Problems are `ProvisioningProblem` codes with the text `prov_err_<code>`.
- **Sources:** a SAF tree (the folder picker) and, in debug builds only, the app's external files directory `provisioning-inbox` (`ProvisioningPaths.debugInboxDir()`), which `adb push` can write without permissions.
- **Files under `filesDir`:**

```
provisioning/bundle/            the verified bundle in use (pos.json, MANIFEST.json, ...)
provisioning/staging-<millis>/  an import in progress
provisioning/record.json        deviceId, manifest SHA-256, trusted comment, import time
engine/                         engine data directory (pos.db, backups, engine.lock)
logs/                           engine log directory
```

- **Keys.** Debug builds embed the committed development public key `android/app/provisioning/dev-minisign.pub` (`PROVISIONING_PUBLIC_KEY`, `PROVISIONING_KEY_IS_DEV=true`). Its secret half lives only in `$HOME/.cache/pili-android/dev-minisign/` and never in a git work tree. Release builds take the Gradle property `piliProvisioningPublicKey`; without it every bundle is refused (`prov_err_key_missing`). The release key and its holder are the user's (DECISIONS section 9).
- **`scripts/android/make_dev_bundle.sh`** makes a development bundle (`--seed` adds `src-tauri/seed_spf_test.sql`; `--fdm-url` and `--printer` set the mock addresses). It prints `DEVBUNDLE | PASS | <dir>`. See `android/app/provisioning/README.md`.

## 4. Kiosk

`KioskController.prepare()` runs at every start of `MainActivity` and applies the owner policies, each step isolated (a refusal is logged, the rest still runs):

1. the app is the only Lock Task package;
2. Lock Task features = `GLOBAL_ACTIONS` (the power menu); the OS reverts them at every boot, so they are set every time;
3. the app is the persistent preferred HOME;
4. every other HOME package and `com.android.settings` are suspended with `setPackagesSuspended` (reversible; the list is stored so `release()` can undo it);
5. the kiosk flag is stored.

`MainActivity.onResume` calls `relock(activity)`: when the app is owner, the flag is set, Lock Task is permitted and the mode is `NONE`, it calls `startLockTask()`; `refreshState` then reads the mode (`KioskState` is `NotOwner`, `Prepared` or `Locked`). Entering is asynchronous, so a test waits for `ActivityManager.lockTaskModeState == LOCK_TASK_MODE_LOCKED`. Lock Task starts before provisioning, because the SAF picker works inside it. `applyImmersive(window)` hides the system bars on resume and on every focus gain (996 of 1080 px usable otherwise).

An app in Lock Task cannot `finish()` its root activity. A test that closes an activity calls `stopLockTask()` first (the app is allowlisted, so it may end its own Lock Task).

`release()` undoes the policies (empty Lock Task list, unsuspend, clear the persistent HOME, clear the flag). The debug `DebugTeardownReceiver` (action `dev.nuagemagique.pilipos.debug.TEARDOWN`, permission `DUMP`, held by the adb shell) calls it and then `clearDeviceOwnerApp`, so an emulator can be provisioned again.

**What P6 adds:** the `DISALLOW_*` restrictions, uninstall block, the time policy (auto time, `Europe/Brussels`), stay awake, keyguard and status bar off, the update policy, adb off as the last step (Q16), the restart after `Fatal`, the `MY_PACKAGE_REPLACED` and boot receivers (Q18).

## 5. User activity forwarding

`MainActivity.onUserInteraction()` calls `UserActivityForwarder.onInteraction()` for every touch, key and trackball event. The forwarder keeps a conflated channel (a burst is one pending signal) and one consumer in the application scope calls `userActivity()` through the gateway while the engine is Ready, then waits one second. Failures are logged at debug level, never shown. The engine owns the idle timer and the lock rule (P1b); the app only navigates to login when the engine announces `PosEvent.SessionLocked` (`ShellEffect.NavigateToLogin`). `forwardedCount` is the number of calls the engine accepted (the probe of the end-to-end test). `onUserInteraction` runs from `Activity.dispatchTouchEvent`, so a test must inject events through the window (`Instrumentation.sendPointerSync`); Compose test gestures go straight to the Compose view and do not reach it.

## 6. Strings, theme and fonts

**Strings pipeline.** `scripts/android/vue_to_android.mjs` reads the Vue texts (`ui/src/i18n*.ts`, `ui/src/utils/fdmMessages.ts`) and the engine error registry (`crates/pos-engine/src/commands/errors.rs`) and writes, under `android/app`: `res/values{,-en,-zh}/strings_vue_<area>.xml` for the areas `core`, `errors`, `checkout`, `reports`, `settings`, `webshop`; `strings_vue_fdm.xml` (French and Chinese FDM texts, not translatable); the generated Kotlin `i18n/generated/{VueStringIds,FdmMessageTables,EngineErrorCodes}.kt`; and the snapshot `src/test/resources/vue_strings.json`. `--write` regenerates, `--check` (gate check `vue-strings`) compares byte for byte. Generated files are never edited by hand.

- Placeholders keep the Vue form `{name}`. The texts are never passed to `getString` with format arguments (they hold braces and may hold `%`); the fill is a plain replace.
- `MessageCatalog.text(key, lang, vars)` looks up a Vue key in the language chosen in the app (not the system language): unknown key gives the key, a missing language falls back to French. `MessageCatalog.text(R.string.x, lang, vars)` does the same for hand-written resources. `AppLanguage` is `FR`, `EN`, `ZH` (tag `zh-Hans`, Q19).
- `ErrorTexts.describe(raw | Throwable, lang)` is `describeError` of the Vue UI: a text that starts with `FDM` goes to `FdmTexts`, a known `err_<code>` is filled from `code|name=value`, anything else is the unexpected-error text. `FdmTexts.describe(raw, lang)` returns the French text (always) and the Chinese line when the till runs in Chinese; these messages are mandatory and their banner never auto-dismisses.
- **Hand-written strings** use reserved prefixes that Vue keys may not use: `shell_`, `engine_`, `prov_`, `kiosk_` (foundation), `ui_` (UI lane, files `strings_ui_common.xml` and `strings_ui_<view>.xml`), `plat_` (platform, `strings_plat_<area>.xml`), plus `fdm_msg_`, `fdm_zh_`, `fdm_fallback_` for the generated FDM tables.

**Theme.** `ui/theme/` ports `ui/src/styles/tokens.css`: `PiliColors` (`Standard` and the violet `Training` palette), `PiliDimens` (spacing 4 to 32, radii, header 64, cart width 670, tap scale and durations), `PiliTypography` (sizes in sp). `PiliTheme(training) { }` provides the tokens and a dark Material 3 scheme mapped from them; read them with `PiliTheme.colors`, `PiliTheme.dimens`, `PiliTheme.typography`. A unit test compares every token with `tokens.css`. `PiliDensity { }` provides `Density(widthPx / 1920f, 1f)` so 1 dp is 1 CSS px of the Vue design, with the font scale fixed at 1 (Q19). `ShellRoot` wraps everything in `PiliTheme { PiliDensity { ... } }`.

**Fonts** are left to P5 run A: P4 uses the system fonts (439 of 439 code points covered, S4). Bundling Noto Sans SC (text and Chinese), JetBrains Mono (figures) and Archivo (brand) as `res/font` belongs to the UI lane (PD106).

## 7. Navigation contracts

`ShellRoot` shows one screen per `ShellState` (`NeedsDeviceOwner`, `NotProvisioned`, `DeviceMismatch`, `DeviceIdUnavailable`, `Opening`, `Ready`, `Fatal`) and, when Ready, the `AppNavHost` with every `ShellOverlay` above it. Every state change is logged under the tag `PiliShell` as `state=<Name>`.

Three Hilt sets are declared (possibly empty) by `NavModule`; a feature contributes to them from its own module, so adding a screen never edits a shared file. `NavRegistry` validates the sets at construction and fails with a clear message when a route is registered twice, the start route `login` is registered by none or by several entries, two settings rows share an id, or a settings row points to an unregistered route.

Test tags the shell and e2e rely on: `shell_root`, `shell_provisioning`, `prov_import_inbox` (debug only), `shell_opening`, `shell_fatal`, `placeholder_login`.

**Add a screen** (`FeatureEntry` + `<Feature>Routes` + module). In `feature/tables/`:

```kotlin
object TablesRoutes { const val TABLES = "tables" }

internal class TablesEntry @Inject constructor() : FeatureEntry {
    override val area = "tables"
    override val routes = listOf(TablesRoutes.TABLES)
    override fun register(builder: NavGraphBuilder, navigator: AppNavigator) {
        builder.composable(TablesRoutes.TABLES) { TablesScreen(onBack = navigator::back) }
    }
}

@Module @InstallIn(SingletonComponent::class)
internal abstract class TablesModule {
    @Binds @IntoSet abstract fun bindTablesEntry(entry: TablesEntry): FeatureEntry
}
```

Routes are lowercase `<area>` or `<area>/<sub>` with `{arg}` segments. `AppNavigator` offers `navigate`, `back`, `toLogin` and `replaceAll`; a screen never sees the `NavController`.

**Add a Settings row** (`SettingsEntry`, from any module; the target route must be registered by some `FeatureEntry`):

```kotlin
@Provides @IntoSet
fun provideAboutSettings(): SettingsEntry =
    SettingsEntry(id = "platform.about", titleRes = R.string.plat_about_title,
        route = PlatformRoutes.ABOUT, order = 900, managerOnly = false)
```

Rows sort by `order`, then `id`. `managerOnly` defaults to true.

**Add an overlay** (`ShellOverlay`: offline banner, update prompt, lock screen):

```kotlin
class OfflineBanner @Inject constructor() : ShellOverlay {
    override val order = 100
    @Composable override fun Content() { /* draws nothing when there is nothing to show */ }
}

@Module @InstallIn(SingletonComponent::class)
abstract class OfflineBannerModule {
    @Binds @IntoSet abstract fun bind(overlay: OfflineBanner): ShellOverlay
}
```

Overlays are drawn above the navigation host by ascending `order`.

## 8. Gates

`scripts/android/p4_gate.sh` prints one `GATE | <check> | PASS|FAIL|SKIP | <detail>` line per check and ends with `GATE | total | PASS|FAIL | <n> failed (<m> checks)`; exit 1 on any FAIL.

| Profile | Use |
| --- | --- |
| `--app --fast --base android/int --allow '<card regex>'` | every commit of an `:app` card: hygiene (paths, em-dash, artifacts, Conventional Commits, `Cargo.lock`), the Android freeze tag (`freeze-tag`, `android-v1.0.0-rc3`; the `linux-frozen` check is retired, `android-frozen` guards the APK inputs), `lanes`, `vue-strings`, `kotlin-tests`, `ffi-boundary`, `app-manifest` |
| `--app` (without `--fast`) | adds the self-tests and the `:app` Gradle checks: lint, ktlint, detekt, unit tests, Roborazzi verify, Kover, `assembleRelease`, `check_apk.sh` |
| `--exit` | adds the plan's literal command `lint ktlintCheck detekt test koverVerifyDebug assembleRelease` |
| `--app-e2e` | adds `scripts/android/app_e2e.sh` (section 9) |
| `--linux-repro` | Linux history, not part of the v1 freeze (INT-D41): builds the frozen Linux tag `linux-v1.0.0-rc2` twice in Docker and compares the SHA-1 with `LINUX_FREEZE.md` |

- **`heavy.sh`** gives every Gradle or cargo build a global slot (`PILI_HEAVY_SLOTS`, default 2) because several agents share one 16 GB Mac. Start builds as `scripts/android/heavy.sh ./gradlew --no-daemon --console=plain --offline -Dorg.gradle.jvmargs='-Xmx2g -XX:MaxMetaspaceSize=1g -Dfile.encoding=UTF-8' <tasks>`; `p4_gate.sh` takes its slots itself. Never stop a daemon you did not start.
- **Kover floor** on `:app`: 80 percent of lines and 60 percent of branches (`koverVerifyDebug`; generated code, Hilt and previews are excluded). A lane may raise it, never lower it.
- **`kotlin_test_inventory.py`** (check `kotlin-tests`) fails when a Kotlin test present on the base branch disappears.
- **`lane_paths.py --check`** (check `lanes`) fails when `LANES.md` and the tree disagree; it also prints the path regex a lane card passes as `--allow`.
- A bug found in `engine/`, `provisioning/`, `kiosk/` or `shell/` is fixed in its own `fix(android):` commit with a regression unit test.

## 9. The end-to-end run and its teardown

`bash scripts/android/app_e2e.sh [--keep]` proves the foundation on `sunmi_d3_pro_api33` (PD113). One line per check, `APPE2E | <check> | PASS|FAIL | <detail>`, then `APPE2E | total | PASS|FAIL | <n> failed (<m> checks)`:

| Check | What it does |
| --- | --- |
| `emulator` | `emulator.sh ensure` reuses the running AVD or starts it headless (exit 3 when another AVD runs); the script never stops it |
| `precheck` | `dpm list-owners` shows no owner, one user, no pilipos package; otherwise it stops and leaves the device alone |
| `build` | `:app:assembleDebug :app:assembleDebugAndroidTest` through `heavy.sh` |
| `mocks` | `pos-fdm-mock` and `pos-printer-mock` on free host ports; `adb reverse` of device ports 19380 (FDM), 19390 (printer), 19391 (printer HTTP); a request must pass through the tunnel |
| `bundle` | `make_dev_bundle.sh --seed` into a fresh temporary directory outside the repository |
| `install` | `adb install -r` of both APKs, then `dpm set-device-owner` (PASS only when the text says `Success`; the exit code is 0 even on refusal; retried up to 8 times) |
| `launch` | `am start`, then `PiliShell state=NotProvisioned` in logcat since the launch |
| `push` | the bundle files into `/sdcard/Android/data/dev.nuagemagique.pilipos.debug/files/provisioning-inbox` |
| `instrumented` | `am instrument` of `ProvisioningE2eTest` (below) |
| `restart` | the debug-only `DebugKillReceiver` broadcast (`...debug.KILL`, SIGKILL of its own process; not `am force-stop`, `am crash` or `run-as kill`, see section 10); the old pid must be gone, a new pid must appear (the system restarts the HOME app, `am start` is the fallback), and only that pid's `PiliShell` lines count: `state=Ready` within 30 s with no `NotProvisioned`, then `mLockTaskModeState=LOCKED`. The PASS detail reports `old pid -> new pid` |
| `teardown` | the debug teardown broadcast, owner cleared, both packages uninstalled, this run's reverse rules removed, mocks stopped, bundle deleted |

`ProvisioningE2eTest` (`androidTest/.../e2e`) runs the real application and Hilt graph. `E2eEntryPoint` lives in the debug source set (an entry point declared only in the test APK is not part of the app's component) and gives the test `EngineHolder`, `EngineGateway`, `EngineEvents`, `UserActivityForwarder`, `KioskController` and `ProvisioningStore`. It checks: `shell_provisioning` shown; click `prov_import_inbox`; within 90 s `Ready` with `pili-pos-android/1.0.0-dev`; the record's `deviceId` equals `Build.getSerial()` and is not blank; Lock Task LOCKED; a real touch raises `forwardedCount` within 3 s; with `EngineEvents` subscribed first, the manager login succeeds and an `FdmStatus` event with `health.connected` arrives within 20 s; logout. Instrumentation arguments: `managerInsz` (default 94060242576), `managerPin` (default 1111), `fdmPort` (default 19380). A missing mock fails the test; it is never skipped.

The test runs with `am instrument`, not with a Gradle connected task: Gradle uninstalls the app before and after, which a device owner forbids (PD113). The teardown is an `EXIT` trap, so it also runs after an error or a signal; `--keep` skips it (the next run's precheck then refuses until you tear down by hand: `adb shell am broadcast -a dev.nuagemagique.pilipos.debug.TEARDOWN -n dev.nuagemagique.pilipos.debug/dev.nuagemagique.pilipos.debugtools.DebugTeardownReceiver`, then `adb uninstall` both packages). Only P4-T12 and the phase exit use the emulator in P4.

## 10. Known limits (hardware residue)

The emulator proves the code paths, not the hardware. Still open, and tracked in `DECISIONS.md` and `spikes/S4.md`:

- the real Sunmi serial format and that `Build.getSerial()` returns it to the device owner (the emulator serial is a fake of 17 characters);
- setup wizard, QR provisioning and an account that blocks `set-device-owner` (the image has no account and no Play);
- the safe-boot volume-key route and the recovery menu under the `DISALLOW_*` set;
- the USB stick for the SPF export (an SD card stood in) and the real update image;
- the 24 hour survival run with the foreground service and deep sleep (P7; the emulator ran a 30 minute soak);
- display facts of the real panel (density, usable height) and whether Chinese prints on the ticket (CP858 prints `?`, a P6 product decision).

- `am force-stop` is ignored for the device-owner package on API 33 (logcat: `Ignoring request to force stop protected package`). Neither the e2e script nor P6's restart and watchdog design can rely on it: a restart is a real process death (the debug `KILL` broadcast of `DebugKillReceiver`, or the OS); `am crash` is ignored for the protected package too and `run-as kill` cannot see the pid (separate pid namespace). The system then restarts the persistent HOME app. After `am instrument` ends, the system also starts a fresh process for the top activity, so a restart check must compare pids and read only the new pid's log. This replaces the brief's `am force-stop` (decision to be copied into `DECISIONS.md` under "Decided while the user was away (review)": that file is outside this card's allowed paths).

The emulator is not wiped by any card. A debug run leaves it without owner, without the packages and without this run's reverse rules.

## 11. Lifecycle hooks (P6)

Added by the shared card P6-T1 (PD203) so that the platform lane never edits foundation files.

- **`AppLifecycleHook`** (`di/AppLifecycleHook.kt`): `order` plus three optional callbacks, `onApplicationCreated(app)`, `onActivityCreated(activity)`, `onActivityResumed(activity)`. `HooksModule` declares the `Set<AppLifecycleHook>` with `@Multibinds`, so it is empty when no lane binds a hook; a lane adds one with `@IntoSet` in a module of its own package.
- **Call points.** `PiliApplication.onCreate`: at the end, after `installLogging`. `MainActivity.onCreate`: right after `kiosk.prepare()`, before `setContent`. `MainActivity.onResume`: after `kiosk.relock(this)` and `applyImmersive(window)`, before `EngineService.start(this)`. All on the main thread.
- **Isolation.** `AppLifecycleHooks` (the `@Singleton` runner) sorts by `order`, then class name, and calls each hook in its own try/catch: a failure is logged with `Timber.w` and never rethrown, and the next hook still runs. A hook cannot veto or abort the start.
- **`EngineGateway.state`** is the read-only `StateFlow<EngineState>` of the holder, for the watchdog and for screens. **`EngineGateway.close()`** delegates to `EngineHolder.close()`; only the updater uses it, before a package install, and the engine reopens through `EngineService.start` (there is no in-process reopen).
- **Event stream end.** When the event stream ends while the engine is still `Ready` and stays `Ready` for 40 s (longer than the 35 s shutdown bound, so a shutdown in progress is not mistaken for a crash), `EngineEvents` marks the engine `Fatal("event stream ended while ready")` and the watchdog can restart the process (PD122). The holder reaches `EngineEvents` through a lazily member-injected `holder` (the two depend on each other).
- **`EngineHelpers.verifyMinisign(publicKey, data, signature)`** wraps the UniFFI free function on the I/O dispatcher and returns `MinisignCheck` (`valid`, `trustedComment`, `problem`); failures are values. The `ffi-boundary` gate check forbids calling the free function outside `engine/` and `provisioning/`, so the updater goes through this helper.
