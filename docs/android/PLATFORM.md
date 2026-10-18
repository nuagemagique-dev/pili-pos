# Platform lane reference (P6)

Owner: `platform` (`docs/android/LANES.md`). This file describes what the platform lane adds to the Android till on top of the `:app` foundation: lifecycle hooks, device-owner policies, the audit log, the crash watchdog, the About screen, the SPF export, the updater, release tooling and the storage rules. The decisions are PD201 to PD221 (`docs/android/DECISIONS.md`, "Decided while the user was away (review)").

How to read it: every section starts with a `Status:` line. `Status: design (P6-Tn)` means the text is the plan written before the card ran; the card `P6-Tn` that builds the feature replaces the line by `Status: as built (P6-Tn)` and corrects the text where the build differs. Each section is edited by one card only, so the cards touch disjoint hunks.

## 1. Overview and routes

Status: design (P6-T2)

The platform lane is the `platform/` package of `:app` (one Gradle module, PD101), its `strings_plat_<area>.xml` files, platform XML and drawables, the Docker and release tooling, and this file. It never edits `feature/`, `ui/` or the UI lane's strings, screenshots and parity files.

Three screens are registered by one `PlatformEntry` (a `FeatureEntry` bound with `@IntoSet`). Route constants live in `PlatformRoutes`, never in `nav/`. The UI lane's Settings screen lists the rows from the `SettingsEntry` set (PD205):

| Route | Settings row id | Order | Who sees the row |
| --- | --- | --- | --- |
| `platform/about` | `platform.about` | 900 | everyone (`managerOnly = false`) |
| `platform/export` | `platform.export` | 910 | managers only |
| `platform/update` | `platform.update` | 920 | managers only |

Packages under `platform/`: `backup` (the no-cloud-backup policy test), `policy` (device owner), `audit`, `watchdog`, `about`, `export`, `update`. Strings: `strings_plat_about.xml`, `strings_plat_policy.xml`, `strings_plat_watchdog.xml`, `strings_plat_export.xml`, `strings_plat_update.xml`; French in `values/`, English in `values-en/`, Chinese in `values-zh/`, the same names in all three (PD204). Strings that must not be translated carry `translatable="false"` and exist only in `values/`.

Fixed rules for everything in the lane:
- no money or fiscal logic in Kotlin; every engine call goes through `EngineGateway.call { }` off the main thread;
- the lane never calls the UniFFI free functions (the `ffi-boundary` gate check), never touches `EngineHolder`, never uses `am force-stop` (PD118) or `killBackgroundProcesses`;
- nothing leaves the till on its own: no cloud backup, no device transfer, and the audit log is never sent anywhere.

## 2. Lifecycle hooks used

Status: design (P6-T1)

The lane reaches the foundation only through the hooks added by the shared card P6-T1 (PD203). Details are in `APP.md` section 11.

- **`AppLifecycleHook`** (set bound with `@IntoSet` from a module in the lane's own package): `onApplicationCreated(app)`, `onActivityCreated(activity)` and `onActivityResumed(activity)`, sorted by `order`, each hook isolated in its own try/catch. A hook can neither veto nor abort the start.
- **Call points:** the end of `PiliApplication.onCreate`; `MainActivity.onCreate` right after `kiosk.prepare()`; `MainActivity.onResume` after `kiosk.relock` and before `EngineService.start`.
- **`EngineGateway.state`** (read-only `StateFlow<EngineState>`) feeds the watchdog; **`EngineGateway.close()`** is used by the updater only.
- **`EngineHelpers.verifyMinisign(publicKey, data, signature)`** is the only way for the updater to check a minisign signature.
- **Event stream end:** the engine is marked `Fatal` when the stream ends while it stays `Ready`, so the watchdog can restart the process (PD122).

Used by: the device-owner policies (application and activity start), the watchdog (application start), the update completion hook (application start).

## 3. Device-owner policies

Status: as built (P6-T3)

`platform/policy/` holds four classes and a Hilt module:

- `PolicySet`: the constants (the restriction keys, `stay_on_while_plugged_in` = 7, `Europe/Brussels`, Lock Task features 0).
- `DevicePolicies`: an interface over `DevicePolicyManager` with exactly the calls used; `AndroidDevicePolicies` is the production implementation and reaches the manager through `provisioning.DeviceOwner.adminComponent` (read only use of the foundation helper). Tests use a fake.
- `PolicyApplier` (`@Singleton`): `apply(release, provisioned)` returns a `PolicyReport` (one `PolicyStep` per step: name, ok, error text). `applyLockTaskFeatures()` applies the last-but-one step alone.
- `PolicyHook` (an `AppLifecycleHook`, order 100, bound `@IntoSet` in `PolicyModule`): it runs the applier at the lifecycle points and writes the audit rows.

Order inside `apply`, each step in its own try/catch (a refusal is recorded and the next step still runs):

1. `setAutoTimeZoneEnabled(false)`, then `setTimeZone("Europe/Brussels")` (the zone is set only once the automatic zone is off), then `setAutoTimeEnabled(true)`;
2. the 14 restrictions one by one: `no_factory_reset`, `no_safe_boot`, `no_uninstall_apps`, `no_install_apps`, `no_install_unknown_sources`, `no_install_unknown_sources_globally`, `no_config_date_time`, `no_add_user`, `no_user_switch`, `no_remove_user`, `no_modify_accounts`, `no_usb_file_transfer`, `no_network_reset`, `no_system_error_dialogs`. One `getUserRestrictions` read comes first, and a restriction already listed is not added again (the step counts as ok and is marked skipped), so a normal start makes few calls;
3. `setUninstallBlocked` for the own package, then one `hide:<package>` step per entry of `PolicySet.HIDDEN_PACKAGES` (PD231: the Play Store and the user-facing Google apps, `setApplicationHidden`; a missing or already hidden package is a skipped step, never an error; `NEVER_HIDDEN` and the own package are refused);
4. stay awake while plugged in (`STAY_ON_WHILE_PLUGGED_IN` = "7");
5. keyguard disabled, status bar disabled (a `false` return from the system is a failed step);
6. the windowed system update policy, 03:00 to 05:00 local (Europe/Brussels) (PD231; a postpone policy only delays 30 days);
7. `setLockTaskFeatures(0)`;
8. last, only when the build is release and a provisioning record exists: `no_debugging_features` (adb off).

When the app is not the device owner nothing is called and the report is `NotOwner`.

Hook behaviour (PD206):

- **Application start:** the hook reads the provisioning record file (`ProvisioningStore.refresh`), then applies the whole set once in the application scope on the I/O dispatcher. In a release build with no record yet it keeps waiting for the record (the first import) and applies the set again when it appears, which adds adb off.
- **Activity create and resume:** `setLockTaskFeatures(0)` is called synchronously every time. It runs after `KioskController.prepare()` (the foundation calls the kiosk directly in `MainActivity.onCreate`, before any hook), so it overrides the power menu the kiosk enables, and it is repeated because the OS reverts the features at every boot (S4 2.3). If the previous full application had failed steps, the whole set is applied again, in the background.
- **Audit** (`PlatformAudit`, kind `policy`): one `step_failed` row per failed step (step name and error text), and one summary row per application start (`applied` with the ok count and the failed names, or `not_owner`). A successful resume writes nothing.
- In the production release the staff lose the power menu (question for the user).

Debug versus release: a debug build (`BuildConfig.DEBUG`) applies steps 1 to 7 and never adb off, because the emulator loop and Gradle need adb. A release build adds step 8 as soon as a record exists. There is no timed adb (DECISIONS 7).

Never applied, and a test asserts it: `no_mount_physical_media` (an SD card or a USB stick mounts, the export needs it) and `no_config_wifi` (the Wi-Fi can be fixed on site).

Teardown: the restrictions, the uninstall block and the other policies belong to the device owner. The debug `DebugTeardownReceiver` calls `KioskController.release()` and then `clearDeviceOwnerApp`; clearing the owner removes every restriction and policy the owner had set, so an emulator is usable again (including adb, which release builds switch off). The policy layer has its own teardown code in no place, and no strings: it has no screen, so `strings_plat_policy.xml` does not exist.

## 4. Audit log

Status: as built (P6-T11 observed)

`filesDir/platform/audit.jsonl`, one JSON object per line (PD208): `ts` (ISO 8601 with the Europe/Brussels offset), `kind`, `event`, `detail` (string map), `versionCode`, `versionName`, `pid`.

- Every append is fsynced.
- Rotation at 1 MiB keeps 5 files (`audit.jsonl.1` to `audit.jsonl.5`).
- It is not the fiscal journal, it is never sent anywhere, and it is part of the SPF export (section 7).
- Writers: the policies, the watchdog, the export and the updater.

Observed on the emulator (`plat_e2e.sh`, section 13): the file survives the restarts and the package replacement (the same `audit.jsonl` carries rows of `versionCode` 10000 and 10001), and after one full run it holds `policy/applied`, `export/started`, `export/done`, `update/check_started`, `update/verified`, `update/refused` (`not_newer`, `sha256_mismatch`), `update/started`, `update/package_replaced`, `update/done`, `watchdog/fatal`, `watchdog/restart` and, written by the new process, `watchdog/restarted` (31 rows in the run with the update). The step `check_audit` asserts these rows.

## 5. Crash watchdog

Status: as built (P6-T11 observed)

`platform/watchdog/EngineWatchdog` watches `EngineGateway.state` from the application scope (PD207).

- **On Fatal:** it writes an audit row, waits 3 s (the logs flush and the fatal screen is seen), records the restart and kills its own process (`Process.killProcess(myPid())` behind an injectable killer). The system restarts the persistent HOME application and the `START_STICKY` service. It never uses `am force-stop`, which the system ignores for the device-owner package (PD118).
- **Budget:** 3 restarts in 10 minutes (wall clock, persisted in `filesDir/platform/restarts.json`). Beyond that it stays on the fatal screen and audits "budget exhausted".
- **`Closed(Shutdown)` for more than 60 s,** while no platform operation (update, export) runs: it calls `EngineService.start(context)` and audits it.
- **`BootReceiver`:** audits the boot and starts `MainActivity` as a fallback to HOME.

Strings: `strings_plat_watchdog.xml`.

Observed (step `fatal` of `plat_e2e.sh`, `EngineHolder.markFatal("e2e")` from the debug entry point, no `am force-stop`): the audit rows `watchdog/fatal` then `watchdog/restart` are written, the process ends about 3 s later, the system starts a new process at once (the new pid is there when the instrumentation reports "Process crashed"), the new process logs `PiliShell state=Ready` without a second import, 3,646 ms after `markFatal` in the final run (4,011 ms in the first run; 3,000 ms of it is the wait of the watchdog, so about 0.6 to 1 s of restart), the kiosk is `mLockTaskModeState=LOCKED` within 1 s of that, and the new process writes `watchdog/restarted`. The budget file `restarts.json` held one entry.

## 6. About and identity

Status: as built (P6-T4)

`platform/about/IdentityProvider` (a `@Singleton`) builds an `IdentityReport` (PD209). `report()` never throws (apart from the cancellation of its caller): a field that cannot be read is null. The route `platform/about` (`PlatformRoutes.ABOUT`) shows `AboutScreen`; the settings row `platform.about` (order 900, not manager only) is unchanged. The export writes `IdentityReport.toJson()` as `identity.json`.

| Field | Source | How to recompute it on a host |
|---|---|---|
| `applicationId`, `versionName`, `versionCode` | `PackageManager.getPackageInfo` | `aapt2 dump badging base.apk` |
| `posSwVersion` | `EngineHelpers.posSwVersion()`, null when the native library does not load | Cargo version of `android/rust/pos-ffi` |
| `engineRef` | string `plat_about_engine_ref` (not translatable) | see below |
| `apkSha1`, `apkSha256` | `base.apk` (`applicationInfo.sourceDir`), streamed with a 64 KiB buffer on the I/O dispatcher | `shasum -a 1 base.apk`, `shasum -a 256 base.apk` (pull it with `adb shell pm path dev.nuagemagique.pilipos`, then `adb pull`) |
| `soSha1` | the zip entry `lib/arm64-v8a/libpos_ffi.so` read inside the APK with `java.util.zip.ZipFile`; null with `soProblem` when the entry is absent or the file is not a zip | `unzip -p base.apk lib/arm64-v8a/libpos_ffi.so \| shasum` |
| `signerSha256` | `PackageManager.GET_SIGNING_CERTIFICATES`, SHA-256 of the DER bytes of each signing certificate, lowercase hex | `apksigner verify --print-certs base.apk` (line "certificate SHA-256 digest") |
| `deviceId` | the provisioning record (`ProvisioningStore`), read only | the Sunmi serial, `adb shell getprop ro.serialno` |
| `posId`, `terminalId` | `getSettings` through `EngineGateway.call`, only while the gateway is ready; null otherwise or when the call fails | the `pos.json` of the bundle |
| `androidRelease`, `sdkInt`, `fingerprint`, `manufacturer`, `model` | `Build` | `adb shell getprop` |
| `freeInternalBytes` | `StatFs(filesDir).availableBytes` | `adb shell df /data` |
| `backupsCount`, `backupsBytes` | the `*.db` files of `filesDir/engine/backups` | `adb shell run-as <package> ls -l files/engine/backups` (debug builds) |
| `generatedAt` | ISO 8601 with the Europe/Brussels offset | |

The hashes are cached per `(sourceDir, lastUpdateTime)`, so a new install recomputes them and a second report of the same install reads nothing. A failed read of the APK is not cached. `toJson()` has a stable key order (the order above, fields as named in the table) and writes a missing value as JSON `null`, never as an absent key.

The engine reference is `pili-pos <version>, <freeze tag>` with no commit part (INT-D45: the tag is set on the commit that carries the string, so it cannot name its own commit), for example `pili-pos 1.0.0, android-v1.0.0-rc3`. The tag commit is recorded in `ANDROID_FREEZE.md`. There is no `(pending)` form any more. The gate check `plat-engine-ref` compares the string with the freeze tag, so a re-freeze must update it. v1 submits Android only (INT-D41), so the reference names the Android freeze tag.

`AboutScreen` has four sections: Version (version, code, engine version, engine reference), Integrity (APK SHA-1 and SHA-256, library SHA-1, signer SHA-256), Device (deviceId, posId, terminalId, Android, model) and Storage (free space, backups count and size). Hashes use the monospace figure style and the text is selectable. A warning line shows under Storage when less than 2 GiB are free or when the backups take more than half of the free space (PD219: the platform deletes nothing; since the 1.0.0 re-freeze the engine keeps only its 7 newest backups, so the backups warning is a safety net). Test tags: `about_root`, `about_apk_sha1`, `about_so_sha1`, `about_device_id`, plus `about_refresh`, `about_free_space`, `about_backups`, `about_warning_low_space`, `about_warning_backups`.

`AboutScreen` takes its `AboutViewModel` from an entry point of the Hilt graph (not `hiltViewModel()`), so it renders under any host activity. The golden is `android/app/src/test/screenshots/platform/about.png` (Roborazzi, 960 px window at density 0.5, the 1920 px till).

Strings: `strings_plat_about.xml`.

## 7. SPF export

Status: as built (P6-T6)

The tax authority (SPF) may ask for the data of the till (AM Art. 16 and 102). The Linux practice is the full `pos.db`, the logs and the SHA-1 of the binary. The Android flow follows Q17 (PD210). Route `platform/export` (`PlatformRoutes.EXPORT`), settings row `platform.export` (order 910, `managerOnly = true`), package `platform/export`, strings `strings_plat_export.xml` (fr, en, zh).

**Screen states** (`ExportViewModel`, `ExportScreen`; every state has a test):

| State | Shown when | Test tag |
|---|---|---|
| Checking | the first rights check runs | `export_checking` |
| NotManager | the logged-in user is not an Admin (`getCurrentUser()`, PD214) | `export_not_manager` |
| Blocked | an order or a takeaway is open, or the engine is not ready | `export_blocked` |
| NoStick | no mounted removable non-primary volume: "Insert the USB stick or SD card" | `export_no_stick` |
| NeedsFolder | the volume has no stored folder; the picker button; a refused pick shows its reason | `export_pick`, `export_error` |
| Ready | a folder is stored: its name, "Change folder", "Export" | `export_folder`, `export_start`, `export_change` |
| Running | progress (files and bytes, current file) | `export_progress` |
| Done | export folder, file count, total size, SHA-256 of `MANIFEST.sha256` | `export_done`, `export_manifest_sha` |
| Failed | step, file, reason from the strings, the technical detail, and "what was written is kept" | `export_failed`, `export_failed_reason` |

The root tag is `export_root`. The screen looks for a stick that was inserted or removed every 2 s while it is in front, and re-checks the rights each time it comes back to the front. When several removable volumes are mounted the screen works with the one that has a stored folder and offers the others as buttons.

**Volume and folder.**
- `VolumeProbe` lists `StorageManager.storageVolumes` filtered to `isRemovable && !isPrimary && state == MEDIA_MOUNTED` (and a uuid) and builds `StorageVolume.createOpenDocumentTreeIntent()`.
- The picker is started with `rememberLauncherForActivityResult(StartActivityForResult)` from the activity, never with `FLAG_ACTIVITY_NEW_TASK` (Lock Task refuses it; S4 case B).
- `ExportFolderStore` keeps the tree Uri per volume uuid in the app-private preferences `platform_export` and calls `takePersistableUriPermission(READ|WRITE)`; "Change folder" and a new choice release the old grant. It refuses the volume root (document id ending with ':', `plat_export_err_root`: "create a folder named PILI-EXPORT and choose it") and a tree of another volume (`plat_export_err_wrong_volume`). A stored folder whose grant is gone is read as no folder.
- No storage permission is added: the tree grant is enough, and the manifest is untouched by this card (`git diff android/lane-plat...HEAD -- android/app/src/main/AndroidManifest.xml` is empty).

**Export folder.** `pili-export-<yyyyMMdd-HHmmss>-<deviceId>` (Europe/Brussels; the deviceId keeps letters, digits, `.`, `_`, `-`), created in the folder the manager chose:

| Entry | Content |
| --- | --- |
| `pos.db` | a fresh `backupDatabase()`: a consistent SQLite copy made by the online backup API (it opens as is, `PRAGMA integrity_check` answers `ok`) |
| `backups/` | the 7 newest other `*.db` of `engine/backups` (never more; in practice at most 6, because the engine keeps its 7 newest backups and the fresh `pos.db` copy is one of them) |
| `backups.txt` | inventory of every other backup of the till, sorted by name, one line each: `<name><TAB><bytes><TAB><sha256>` (the fresh copy is `pos.db`, not listed) |
| `logs/` | every file of `filesDir/logs`, copied up to the size it had when the plan was made |
| `audit/` | the platform audit files (`PlatformAudit.files()`), including the row "started" of this export |
| `identity.json` | `IdentityReport.toJson()` (section 6) |
| `LISEZMOI.txt` | French, brand-neutral: what each file is, that `pos.db` is a consistent copy made by the online backup API, how to verify, export date, deviceId, posId |
| `MANIFEST.sha256` | `sha256sum` format, `<sha256>  <relative path>`, sorted by path, covering every other file, written last |

`backups/`, `logs/` and `audit/` always exist, empty when there is nothing to copy.

**How it runs** (`Exporter.run`, inside `PlatformOps.run("export")` so the watchdog stays out of the way):
1. Pre-check, repeated at every run: the volume is still mounted, the engine is ready, the current user is an Admin, `listOpenOrders()` and `listOpenTakeaways()` are empty. A refusal writes nothing and is audited as `refused`.
2. Audit row `started`, then the plan (`ExportPlanner`: the fresh `backupDatabase()`, the lists, the generated files).
3. For each file, in plan order: stream it in 1 MiB chunks through a SHA-256 `DigestOutputStream` to the target, `fd.sync()`, close, open it again through the target, hash it again; a different hash fails the export. Then `MANIFEST.sha256` the same way.
4. Audit row `done` (files, bytes, folder).

On any error the export stops, keeps what was written and reports the step (`precheck`, `plan`, `create_dir`, `copy`, `verify`, `manifest`), the file and the reason; the audit row `failed` carries `step`, `file`, `problem`, `reason`. A write error on a file of 4 GiB or more is reported as `plat_export_err_fat32_hint` (the stick is probably FAT32, use exFAT). The platform never deletes anything, neither on the volume nor in `filesDir`. The fresh `pos.db` copy stays in `engine/backups` like every backup the engine made; it is an engine backup, so the engine's retention (section 12) then keeps the 7 newest backups and may remove the oldest one.

**Targets.** All export logic is above `ExportTarget` (`createDirectory`, `createFile` returning an `ExportOutput` with `sync()`, `openForRead`). `SafTarget` uses `DocumentsContract.createDocument`, `openFileDescriptor("w")` with `FileOutputStream(fd)` and `fd.sync()`, and reads back through `ContentResolver.openInputStream`; every file is created with the MIME type `application/octet-stream` so that the provider never adds an extension to `MANIFEST.sha256`. `FileTarget` writes plain files (unit tests, and the P6-T11 device test on the removable volume's app-specific directory).

**For an inspector.** Plug the stick into a computer and, in the `pili-export-...` folder:
1. `shasum -a 256 -c MANIFEST.sha256` (Linux: `sha256sum -c MANIFEST.sha256`): every line must say `OK`, and a missing `MANIFEST.sha256` means the export is incomplete.
2. `sqlite3 pos.db "PRAGMA integrity_check;"` must say `ok`.
3. `identity.json` gives the application hashes to compare with the installed build (section 6); `backups.txt` lists the other backups; `audit/audit.jsonl` shows when exports ran and how they ended.
4. The hash shown on the Done screen is the SHA-256 of `MANIFEST.sha256`: `shasum -a 256 MANIFEST.sha256` must give the same value.

Limits: the gate cannot read a real stick; the real USB and SD behaviour (exFAT, removal during a write) is hardware residue (spike S4). A log or audit file that is rotated away between the plan and its copy fails the export with `plat_export_err_read` (rare: audit rotates at 1 MiB, the plan and the copy are seconds apart); the manager just runs it again.

## 8. Updater: source format and verification

Status: as built (P6-T7)

The package is `platform/update/`. Part 1 (this section) reads a declared release and verifies it; it installs nothing. Part 2 (section 9) installs. Nothing here touches `InstallResultReceiver` or `PackageReplacedReceiver`.

### 8.1 Descriptor

One signed descriptor `declared.json` per declared Android version (PD211), UTF-8, at most 64 KiB (`DeclaredParser.MAX_BYTES`):

`{"format":1,"platform":"android","package":"dev.nuagemagique.pilipos","version":"X.Y.Z","versionCode":N,"asset":"pili-pos-android-X.Y.Z.apk","sha1":"..","sha256":"..","signerSha256":"..","declared_at":".."}`

| Field | Rule (`DeclaredParser`, a refusal is `bad_descriptor`, or `too_large` above 64 KiB) |
| --- | --- |
| `format` | the integer `1` |
| `platform` | exactly `"android"` |
| `package` | non-empty string, must equal the installed application id (`wrong_package`) |
| `version` | `^[0-9]+\.[0-9]+\.[0-9]+$`, equal to the `versionName` of the APK |
| `versionCode` | integer greater than 0, strictly greater than the installed `versionCode` (`not_newer`) |
| `asset` | `^pili-pos-android-[0-9]+\.[0-9]+\.[0-9]+\.apk$` and equal to `pili-pos-android-<version>.apk` |
| `sha1` | 40 lowercase hex characters, SHA-1 of the APK |
| `sha256` | 64 lowercase hex characters, SHA-256 of the APK |
| `signerSha256` | 64 lowercase hex characters, SHA-256 of the DER signing certificate of the APK |
| `declared_at` | non-empty string |

A missing, mistyped or empty field, an unknown `format`, another `platform` or a hash in capitals refuses the release. Unknown extra keys are ignored (the whole file is signed). The file is written compactly with one trailing newline; the signature covers the exact bytes.

`declared.json.minisig` is a prehashed minisign signature (`minisign -S`, the default, algorithm `ED`) of `declared.json`, trusted comment `pili-pos-android X.Y.Z declared <ts>`. It is verified only through `EngineHelpers.verifyMinisign` (the Rust verifier of the engine). A key that is not a minisign key is `key_missing`, any other failure is `bad_signature`.

### 8.2 Sources

`UpdateSource` has two calls: `fetchDescriptor()` returns `DescriptorFiles(jsonBytes, minisigText)`, `fetchAsset(name, dest, maxBytes = 200 MiB)` writes the APK to `dest` (through `dest.part`, so a failed copy leaves nothing). A source moves bytes and trusts nothing; failures are an `UpdateException(code, detail)`.

**`GhcrSource` (online).** An OCI artifact, tag `declared-android` of the public package `ghcr.io/brunomoyse/moyun-pos` (the same package as the Linux `:declared`), pulled anonymously:

1. `GET {base}/token?scope=repository:brunomoyse/moyun-pos:pull&service=ghcr.io`, JSON `{"token":"..."}` (`access_token` is accepted too);
2. `GET {base}/v2/brunomoyse/moyun-pos/manifests/declared-android` with `Accept: application/vnd.oci.image.manifest.v1+json` and `Authorization: Bearer <token>`;
3. `GET {base}/v2/brunomoyse/moyun-pos/blobs/<digest>` for each layer, with the bearer.

The token and the manifest are fetched once per source object, so the descriptor and the APK come from the same manifest. Every blob is hashed while it streams and compared with its `sha256:` digest: a wrong digest is `sha256_mismatch` for the APK and `bad_descriptor` for the descriptor files. A layer larger than its limit is `too_large` before the blob is requested, and a blob that turns out longer than the manifest says is refused (`too_large`) as soon as it passes the limit, and nothing is kept. Timeouts: 15 s to connect, 60 s to read. Redirects (the registry sends blobs to a CDN) are followed by hand, at most 5, and the `Authorization` header goes only to the origin (scheme, host and port) of the base URL, never to the target of a redirect. TLS is required, apart from the debug override below. Any HTTP error or transport failure is `network`.

**`UsbSource` (Q8).** The same three files from a folder chosen through the Storage Access Framework (`UsbSource(context, treeUri)`, a `UsbFolder` over a `DocumentFile`); a file that is absent or cannot be opened is `usb_missing_file`. The same size limits apply.

**Limits:** APK 200 MiB, each descriptor file 64 KiB. HTTPS only in release builds. Debug builds read an optional `filesDir/platform/registry.json` (`{"baseUrl":"http://127.0.0.1:<port>"}`) so the emulator test can reach a host mock through `adb reverse`; `GhcrSource.forApp(filesDir, debug)` is the only reader. In the override, clear text is accepted only for `127.0.0.1` or `localhost` (an unacceptable file is ignored and logged), a release build never reads it, and a redirect to clear text on another host is refused. The gate check `plat-boundary` allows an `http://` literal only on a line with `127.0.0.1` or `localhost`; the code compares schemes and has no such literal.

### 8.3 OCI layout that P6-T9 must produce

`oras push ghcr.io/brunomoyse/moyun-pos:declared-android` with exactly these three files, each pushed as a layer whose `org.opencontainers.image.title` annotation is the file name (oras sets it from the file name):

| Layer title | Content |
| --- | --- |
| `declared.json` | the descriptor of 8.1 |
| `declared.json.minisig` | its minisign signature |
| `pili-pos-android-X.Y.Z.apk` | the signed APK of `asset` |

Each title appears once (two layers with the same title are refused). The manifest is `application/vnd.oci.image.manifest.v1+json`; the config blob and the media types of the layers are not read. The tag is moved to the new manifest at each declaration; the app never reads another tag. Layer digests must be `sha256:`.

### 8.4 Keys (PD212)

`res/xml/plat_update_keys.xml` holds the public minisign key per build type: `<plat-update-keys><key build="debug">RW...</key><key build="release"></key></plat-update-keys>`. `UpdateKeys.load` picks the element by `BuildConfig.DEBUG`. `debug` is the committed development key (`android/app/provisioning/dev-minisign.pub`; the gate check `plat-keys` compares them). `release` is empty until the user creates the declaration key (DECISIONS 9, Q7). With an empty key every update is refused (`key_missing`, string `plat_update_err_key_missing`).

### 8.5 Verification order

`UpdateVerifier.verify(source)` returns a `VerifiedUpdate(release, file, sha1, sha256)` or an `UpdateRefusal(code, detail)`. It stops at the first refusal, in this order (S4 section 7):

1. a public key for this build (`key_missing`);
2. `fetchDescriptor()` (source codes: `network`, `bad_descriptor`, `too_large`, `usb_missing_file`);
3. minisign over the exact bytes of `declared.json` (`bad_signature`);
4. the strict parse (`bad_descriptor`, `too_large`);
5. `package` equals the installed application id (`wrong_package`);
6. `versionCode` strictly greater than the installed one (`not_newer`; the OS accepts an equal one, S4 U5, so the app refuses it);
7. the download to `filesDir/platform/update/<asset>` (app-private; an older file is removed first);
8. SHA-256, then SHA-1, of the file equal to the descriptor (`sha256_mismatch`, `sha1_mismatch`), one pass through `platform/about/Hasher`;
9. `PackageManager.getPackageArchiveInfo(path, GET_SIGNING_CERTIFICATES)`: package, `versionCode` and `versionName` equal to the descriptor (`archive_mismatch`); the signer set (SHA-256 of each signing certificate) equal to the installed signer set and containing `signerSha256` (`signer_mismatch`).

Any refusal, any error and a cancellation after the download began delete the file downloaded by that run (and its `.part`). A refusal raised before the download (steps 1 to 6) deletes nothing, so the file of an earlier `VerifiedUpdate` stays in `filesDir/platform/update`. A returned `VerifiedUpdate` leaves its file for part 2. Calls are serialised (one verification at a time).

Audit rows, kind `update`: `check_started` (`source` = `ghcr` or `usb`), then `refused` (`code`, `detail`) or `verified` (`version`, `sha1`).

### 8.6 Refusal codes and strings

`UpdateRefusalCode` has twelve codes, each with `plat_update_err_<code>` in `strings_plat_update.xml` (French in `values/`, `values-en/`, `values-zh/`) plus `plat_update_title`: `key_missing`, `network`, `bad_descriptor`, `bad_signature`, `wrong_package`, `not_newer`, `too_large`, `sha256_mismatch`, `sha1_mismatch`, `archive_mismatch`, `signer_mismatch`, `usb_missing_file`. P6-T8 adds the other strings of the screen to the same file.

### 8.7 Tests

Fixtures `src/test/resources/platform/declared.json`, `declared.json.minisig` (signed with the development key) and `declared.tampered.json`. The JVM tests cannot load the Rust library, so `RealMinisignNatives` (test sources) re-implements the check (Ed25519 over BLAKE2b-512, as `minisign -S` does) behind `EngineNatives`; its own test pins BLAKE2b to the RFC 7693 vectors. `GhcrSource` is tested against `TestServer` (test sources), a minimal HTTP/1.1 server on plain sockets bound to `127.0.0.1`; the JDK `com.sun.net.httpserver.HttpServer` is not visible to the Kotlin test compilation of this module. No test reaches another host.

## 9. Updater: preconditions and install

Status: as built (P6-T8)

Package `platform/update` (verification is section 8). Everything below is `UpdatePreconditions`, `UpdateInstaller`, `InstallResultReceiver` (with `InstallStatusHandler`), `PackageReplacedReceiver`, `UpdateCompletionHook` (order 300, `@IntoSet` in `UpdateModule`) and the screen `UpdateScreen` / `UpdateViewModel` (route `platform/update`, settings row `platform.update`, order 920, managers only).

**Preconditions (PD213, interim).** The FFI has no engine-side update precheck and the engine is frozen. `UpdatePreconditions.check()` composes the check from engine answers only, through `EngineGateway.call`; it computes no money and signs nothing. It returns a list of `PreconditionFailure` (empty = go on):
- `not_manager`: `getCurrentUser()` is not an Admin (nothing else is asked then);
- `open_order`: `listOpenOrders()` or `listOpenTakeaways()` is not empty;
- `z_due`: `needsZReport()` is true;
- `z_not_done`: the newest `listZReports()` entry (highest `sequenceNumber`) has no `fiscalEventId`, or a fiscal event of `listFiscalEvents(200)` with an id greater than it has an event type outside `PRESENCE_TYPES`; with no Z report at all it is a failure unless `listFiscalEvents(1)` is empty (a till that never signed anything);
- `engine_not_ready`: the engine is not ready or a call failed.

`PRESENCE_TYPES = {"S"}`: the work-in and work-out events are the Social events, label `S` (`crates/pos-fiscal/src/response_check.rs:60-61`, `src/types.rs:42-51`; the label is stored as `event_type`, `src/archive.rs:57`). In training mode every event is `T` (`src/types.rs:53-59`), so training presence events are not accepted: the check stays strict.

**Manager PIN (PD214).** The "manager PIN" of the plan is the logged-in Admin session. The screen reads the preconditions when it comes to the front and again after a verification, and `UpdateInstaller` checks again right before it acts. There is no second PIN prompt, because a new `login` would queue a clock-in.

**Install flow (PD215, Q18), as a sequence.** The screen verifies a release (section 8) and shows its version, `versionCode`, SHA-1 and signer. The Install button is enabled only with a `VerifiedUpdate` and no precondition failure. Then `UpdateInstaller.install(verified)`, inside `PlatformOps.run("update")`:

```
UpdateScreen            UpdateInstaller                 engine / disk / system
  install() ----------> preconditions again ----------> getCurrentUser, listOpen*, needsZReport, listZReports, listFiscalEvents
                         (failure: Blocked, nothing touched, audit install_blocked)
                        gateway.call { backupDatabase() }   (error: Refused backup_failed)
                        gateway.close()  -> Closed          (StillOpen: Refused engine_busy, EngineService.start)
                        write filesDir/platform/update/pending.json  (tmp file, fsync, rename)
                        audit update/started {from, to, version, sha1}
                        clearUserRestriction(no_install_apps)   (PD231; set again in a finally, after the system answered or the hold ended)
                        PackageInstaller session: MODE_FULL_INSTALL, own package, INSTALL_REASON_POLICY,
                          USER_ACTION_NOT_REQUIRED, setSize, openWrite + fsync, commit(broadcast to InstallResultReceiver)
                        returns Committed (the result is not awaited)
```

`DISALLOW_INSTALL_APPS` (PD231) is lifted only around the session. If the install succeeds the process is replaced before the restriction is set again, and the next `PolicyApplier.apply` at application start adds it back (the same repair as after a crash). A refused clear or re-set is logged and does not change the outcome.

Any exception from the marker on, before the commit returns: abandon the session, delete the marker, `EngineService.start`, audit `failed` (code `install_failed`), result `Refused(INSTALL_FAILED)`. A cancellation does the same, then rethrows. After the commit the call only keeps the `update` operation open for at most 90 s (or until the receiver reports), so that the watchdog does not reopen the engine while the system installs; it decides nothing from the answer. The screen then shows "installing, the till restarts".

**After the commit.**
- **Failure status** (`InstallResultReceiver`, action `dev.nuagemagique.pilipos.platform.update.INSTALL_STATUS` only, `goAsync`): audit `failed` with `code`, `status` and the system message, marker deleted, `EngineService.start`.
- **`STATUS_PENDING_USER_ACTION`:** the session (`EXTRA_SESSION_ID`) is abandoned and recorded as `failed` with `code = silent_install_refused`; the confirmation intent is never started.
- **`STATUS_SUCCESS`:** audit `status_success` (rare: the install normally kills the old process first); the marker stays for the new process.
- **New process:** `PackageReplacedReceiver` (`MY_PACKAGE_REPLACED`) starts `MainActivity` with `FLAG_ACTIVITY_NEW_TASK` (the third way back into Lock Task, Q18) and audits `package_replaced`. `UpdateCompletionHook` (application start) reads `pending.json`: running `versionCode == toVersionCode` gives `done` (marker versions, `apkSha1` from `IdentityProvider`, the declared SHA-1) and deletes the marker; `== fromVersionCode` with a marker older than 5 minutes gives `not_completed` (younger: left alone, the install may still run); any other version gives `not_completed` (`unexpected_version`); an unreadable marker gives `marker_corrupt`. All delete the marker.
- **No rollback:** a faulty version is fixed by a newer declared version (DECISIONS 6).

**Audit rows** (file `audit.jsonl`, kind `update`; verification rows are in section 8):

| event | when | detail |
| --- | --- | --- |
| `install_blocked` | a precondition failed | `failures` (ids, comma separated) |
| `install_refused` | backup failed or the engine stayed open | `code` (`backup_failed`, `engine_busy`), `detail` |
| `started` | marker written, before the session | `from`, `to`, `version`, `sha1` |
| `failed` | error before the commit, or a failure status | `code` (`install_failed`, `silent_install_refused`), `status`, `message` / `detail` |
| `status_success` | the old process got the success status | `session` |
| `package_replaced` | new process, `MY_PACKAGE_REPLACED` | none |
| `done` | new process, running version is the target | `from`, `to`, `version`, `running`, `apkSha1`, `declaredSha1` |
| `not_completed` | marker left by an install that did not happen | `from`, `to`, `version`, `reason` |
| `marker_corrupt` | `pending.json` unreadable | `reason` |

**Screen.** Test tags: `update_root`, `update_installed` (version and APK SHA-1 from `IdentityProvider`), `update_check_online` (`GhcrSource`), `update_from_usb` (`UsbSource`; `VolumeProbe` pre-check and the volume's `createOpenDocumentTreeIntent` started from the activity, no `FLAG_ACTIVITY_NEW_TASK`; with several volumes the first one is used; the grant is the temporary one of the result, nothing is persisted), `update_result` (version, `versionCode`, SHA-1, signer), `update_refusal` (verification refused), `update_failures` (one `update_failure_<id>` text per failed precondition), `update_install`, `update_installing`, `update_install_failed`. Not an Admin: only `update_not_manager` (`plat_update_not_manager`).

Strings: `strings_plat_update.xml` (`plat_update_pre_<id>` for the preconditions, `plat_update_install_<id>` for the install refusals).

## 10. Release tooling

Status: as built (P6-T9)

`scripts/release/declare.sh` (Linux) lives on the unmerged `feat/usb-autoinstall` branch, and a copy here would conflict when that branch merges. The Android declaration is therefore its own script, `scripts/release/declare_android.sh` (PD216), for the producer only. Its self-test is `scripts/release/test_declare_android.sh` (gate check `selftest-declare`).

**Release procedure, in order (all of it the producer's, on the Mac):**
1. Tag the commit `android-vX.Y.Z` locally (the tag equals `versionName`) and run `scripts/android/check_repro.sh` on that tagged commit: two container builds of the unsigned `:app` APK must be byte-identical (section 11).
2. Sign a copy with the offline release key, with the same `apksigner` flags as the script (`--min-sdk-version 33 --v1-signing-enabled false --v2-signing-enabled true --v3-signing-enabled true --v4-signing-enabled false`), and submit it to the SPF; the SPF accepts the SHA-1 of the SIGNED APK. The RSA key is deterministic (DECISIONS 9), so the SHA-1 does not change when `declare_android.sh` signs the same unsigned APK again.
3. Run `scripts/release/declare_android.sh --apk <unsigned apk> --version X.Y.Z --sha1 <SHA-1 the SPF accepted> --keystore <offline keystore> --alias <alias> --minisign-key <offline declaration key>`, with `PILI_KS_PASS_FILE` set. Nothing is pushed unless every check passes. A real run needs `oras` and `gh` (scope `write:packages`).
4. Later, once `feat/usb-autoinstall` is merged, `declare.sh --android` gets a one-line dispatch to this script (PD216), a step for the user.

**What `declare_android.sh` does**, printing `DECLARE | <step> | PASS|FAIL | <detail>` per step and `DECLARE | total | ...` last (exit 0 declared, 1 a check failed, 2 usage):
- `tools`: apksigner, zipalign and aapt2 from build-tools 35.0.0 (pinned), keytool, minisign; `oras` and `gh` only without `--dry-run`. The keystore and the minisign key must lie outside any git work tree.
- `version`: `X.Y.Z` only (MINOR and PATCH below 100); a `-dev` version is refused unless `--allow-dev` is given together with `--dry-run`; without `--dry-run` the local tag `android-vX.Y.Z` must exist.
- `badging`: package `dev.nuagemagique.pilipos`, `versionName` equal to the version, `versionCode` equal to MAJOR*10000 + MINOR*100 + PATCH.
- `zipalign`: `zipalign -c -P 16 -v 4` passes on the unsigned APK.
- `key`: RSA of 2048 bits or more; an EC or DSA key is refused.
- `sign`: `apksigner sign` on a copy with `--min-sdk-version 33`, v1 off, v2 and v3 requested (apksigner folds v2 into v3 at that minSdk), v4 off; then `apksigner verify --print-certs` (v1 false, v3 true, one signer); the signer SHA-256 is read from its output.
- `sha1`: the SHA-1 of the signed copy must equal `--sha1`, else the run stops and nothing is written beyond its temporary directory.
- `descriptor`, `minisign-sign`, `minisign-verify`: `declared.json` in the section 8.1 schema (compact, one trailing newline), a prehashed `minisign -S` signature with trusted comment `pili-pos-android X.Y.Z declared <ts>`, then `minisign -V` with the public key. That key is `PILI_DECLARED_PUB` (a file or the base64 line, accepted only with `--dry-run`), else the release key of `plat_update_keys.xml`; an empty release key stops the run.
- `push`: with `--dry-run`, the exact commands are printed and the three files (`declared.json`, `declared.json.minisig`, `pili-pos-android-X.Y.Z.apk`) are written into `--out`; without it, `oras push ghcr.io/<repo lowercased>:declared-android --artifact-type application/vnd.pili-pos.declared-android.v1 <apk>:application/vnd.android.package-archive declared.json:application/json declared.json.minisig:text/plain`, run in the directory of the files so that each layer title is the bare file name (section 8.3), then `gh release upload android-vX.Y.Z -R <repo> --clobber` for the same three files.

**The chain never runs it for real.** It runs `test_declare_android.sh` only: every key (a throwaway RSA-2048 keystore, an EC keystore, two minisign pairs), every APK and a scratch git repository holding a copy of the script live in a `mktemp` directory outside this repository, deleted on exit; `oras` and `gh` are stubs that record their arguments and fail when called during a dry run. The real release signing key, the real declaration key and the real declaration are the user's.

## 11. Reproducible APK

Status: as built (P6-T10)

The container build (`android/docker/build_in_container.sh`) builds `:app:assembleRelease` instead of `:smoke` (PD217, PD80, PD114) and copies `app-release-unsigned.apk`, the R8 `mapping.txt` of `:app` and the unstripped `libpos_ffi.so` to `/out`; it prints `APK | sha256`, `SO | sha1` and `APK | sha1`. Everything that makes the build reproducible is unchanged: `SOURCE_DATE_EPOCH`, Gradle and cargo `--offline`, the path remaps, `vcsInfo.include = false`, no signing key. `check_repro.sh` compares two `app-release-unsigned.apk` files built from the same commit in fresh `--network none` containers (checks `build-1`, `build-2`, `apk-identical`, `so-identical`, `entry-manifest`, `apk-check`; `check_apk.sh` runs on the `:app` APK). `:smoke` stays in the host gates. `android.yml` also assembles `:app:assembleRelease` and runs `check_apk.sh` on `android/app/build/outputs/apk/release/app-release-unsigned.apk` (the chain never triggers it). The commit, the image digest, both APK hashes and the `libpos_ffi.so` SHA-1 of the run are in `REPRO.md`, section "P6: :app". The signed APK SHA-1 of record is produced later by `declare_android.sh` with the release key (DECISIONS 9).

## 12. Storage and the SD card

Status: as built (P6-T11)

**Where the database lives.** Not on the SD card. The engine database is `filesDir/engine/pos.db` in app-private internal storage, next to `filesDir/engine/backups/`, `filesDir/provisioning/` and `filesDir/logs/` (`provisioning/ProvisioningPaths.kt`). It stays there by design (PD220):
- a fiscal journal on removable media can be pulled out, swapped, corrupted or read by another device;
- the manifest has no `android:installLocation`, so Android never moves the data to an SD card adopted as internal storage, and it requests no storage permission (`READ_EXTERNAL_STORAGE`, `WRITE_EXTERNAL_STORAGE`, `MANAGE_EXTERNAL_STORAGE`, `READ_MEDIA_*`) and no `REQUEST_INSTALL_PACKAGES`. The gate check `plat-manifest` keeps it so;
- cloud backup and device transfer are off (`allowBackup="false"`, `fullBackupContent="false"`, the exclude-everything `data_extraction_rules.xml`). The gate check `plat-backup-rules` and `BackupPolicyTest` keep it so.

Removable media are reached only through SAF grants (the export, the update from a stick).

**What the SD card is for.** It is a removable volume like a USB stick, so it is a target of the SPF export (section 7), of the update source "from a stick" (section 8) and later of the nightly export lane. Spike S4 proved the SAF export on this kind of volume (a virtual SD card stood in for the stick). `DISALLOW_MOUNT_PHYSICAL_MEDIA` stays off so the card mounts.

**Size, first estimate (superseded by the measurement below).** A test database of this repository holds about 1.6 KB per fiscal event and 3.5 KB per pending sign (19 events, measured with `dbstat`). With the order, its lines and its payments that was estimated at 10 to 15 KB per sale, 0.4 to 1.5 GB a year at 100 to 300 sales a day. The P4 exit report said "megabytes per year", which was too low.

**Size, measured (P6-T11).** Step `sales` of `plat_e2e.sh` on the emulator, with the seeded test database (294,912 bytes at the start, real engine, FDM mock): 50 takeaway sales of one product, one line each, paid in cash, then 10 sales of 6 lines each, then the Z report. Three kinds of figure come out; only the second and third are the growth of the data:

| Measure | Result | Meaning |
| --- | --- | --- |
| size of `pos.db` plus `pos.db-wal`, no checkpoint forced | 319,664 to 4,746,888 bytes, 88,544 bytes per sale | the write-ahead log re-writes the changed pages at every commit until its next checkpoint (about 4 MB), so this is not growth; it is logged as `bytes_per_sale` of `PLATE2E_DBSIZE` for the record |
| `PRAGMA page_count` times `page_size` (4096), read-only connection, WAL frames included, sales of 1 line | 294,912 to 618,496 bytes: **6,471 bytes per sale** | the growth of the database |
| same, sales of 6 lines | **14,336 bytes per sale** | one more line costs about 1.6 KB: (14,336 - 6,471) / 5 = 1,573 (measured, not decomposed) |
| `engine/backups` after the Z (60 sales) | 1 file, **778,240 bytes** | the full copy the engine makes after every Z (`signing.rs:761`) |

The database has no free pages after the run, so the figures are the growth and not slack. A real sale of the restaurant has several lines, so the 6-line figure is the one to plan with; the 1-line figure is the floor.

**Yearly projection** (`bytes_per_sale x sales a day x 365`, from the measured figures):

| Sales a day | 1-line sale (6,471 bytes) | 6-line sale (14,336 bytes) |
| --- | --- | --- |
| 100 | 236 MB a year (225 MiB) | 523 MB a year (499 MiB) |
| 300 | 709 MB a year (676 MiB) | 1.57 GB a year (1.46 GiB) |

**Backups, if every Z copies the whole database and nothing deletes the copies (the engine before the 1.0.0 re-freeze; kept for the record).** The copy of day *d* weighs the database of day *d*, so after 365 days the copies together weigh `365 x 294,912 + daily growth x 365 x 366 / 2`:

| Sales a day | 1-line sale | 6-line sale |
| --- | --- | --- |
| 100 | 40 GiB a year; 32 GiB reached on day 325, 64 GiB on day 460 | 89 GiB a year; 32 GiB on day 219, 64 GiB on day 309 |
| 300 | 121 GiB a year; 32 GiB on day 188, 64 GiB on day 266 | 268 GiB a year; 32 GiB on day 126, 64 GiB on day 179 |

The database itself stays small (about 0.2 to 1.5 GiB a year); the backups are 100 times bigger than it after a year.

**Retention (user decision of 2026-10-10, "Keep the 7 newest", PD219).** The engine writes a full copy of `pos.db` to `backups/` after every Z (`crates/pos-engine/src/commands/signing.rs`, `report::backup_database_impl`) and, before the 1.0.0 re-freeze, never deleted one: the table above gave 40 to 268 GiB of backups after a year, which would fill the internal storage of the till within months. Since the re-freeze the engine keeps only the 7 newest of its own backups (`report::BACKUPS_KEPT`). It deletes only after the new copy is complete and `PRAGMA quick_check` answers `ok`, only files named `pos-<UTC timestamp>.db`, never `pos.db`, the nightly export folder (`exports/`) or a file someone else put there (`manual-*.db`, `pre-update-*.db` on Linux), and it logs each deletion. Every engine backup counts, whatever made it (the Z, the Backup button, the SPF export's fresh copy). The rule is the same on Linux and Android (no platform conditional) and is tested by `nine_z_in_a_row_leave_the_seven_newest_backups` (`crates/pos-engine/tests/export_nightly.rs`). Nothing legal is lost: the live `pos.db` keeps the whole fiscal journal; the off-till copy is the nightly export.

**Backups with retention.** The copies together weigh at most 7 times the database: about 2 MiB at the start, then at the end of the first year about 1.5 GiB at 100 sales a day of 1 line, 3.4 GiB at 100 sales of 6 lines, 4.6 GiB at 300 sales of 1 line and 10.2 GiB at 300 sales of 6 lines (7 x the yearly projection above plus the seed). The database and its 7 copies grow linearly, not quadratically.

**Recommendation.** The database stays in internal storage only (above). The SD card is for exports and updates:
- exFAT, because FAT32 refuses files of 4 GiB or more, which the database alone reaches after about 3 years at 300 sales a day of 6 lines;
- a good-quality microSDXC card, "high endurance" or class A1/U3;
- **64 GB** is enough for the first year at 100 sales a day: one export is `pos.db` plus at most 6 other backups, about 7 times the database (the figures here keep the earlier 8 as a margin), so about 0.2 GiB at the start and 3.9 GiB at the end of the first year, which leaves room for about 15 exports at the end of that year;
- **128 GB** at 300 sales a day or from the second year: one export weighs about 12 GiB at the end of the first year at 300 sales a day (6-line sales), and the app never deletes a folder on the card, so the manager has to clear old exports.

The measurement does not change "internal only" nor the card kind; it makes the size depend on the volume and shows that the card cannot be the answer to the backups: the retention rule on the till (the engine keeps the 7 newest backups, PD219) is what keeps the internal storage from filling.

Not checked, hardware residue: the largest card the D3 Pro accepts and its slot behaviour (there is no real D3 Pro here), the free internal space of the real device, and exFAT on a real card (the virtual SD disk of the emulator is vfat). The SD card is never the only copy: the SPF export is a manager action, and the off-till copy belongs to the nightly export lane.

## 13. End-to-end run

Status: as built (P6-T11)

`scripts/android/plat_e2e.sh` proves the platform features on the AVD `sunmi_d3_pro_api33` (PD218). One line per check, `PLATE2E | <check> | PASS|FAIL | <detail>`, then `PLATE2E | total | PASS|FAIL | <n> failed (<m> checks)`. `p6_gate.sh --plat-e2e` runs it and passes on the total line. The instrumented side is `PlatformE2eTest` (`androidTest/.../platform`), one `am instrument -e step <name>` per step so that the host can wait for the process deaths between steps; it reads the singletons through the debug `E2eEntryPoint` and the internal platform entry points, and prints `PLATE2E_*` lines to logcat (tag `PlatE2e`).

| Check | What it does |
| --- | --- |
| `emulator`, `precheck` | `emulator.sh ensure`; no owner, one user, no pilipos package; waits (at most 40 minutes) while another end-to-end run uses the shared emulator |
| `removable-volume` | `sm list-volumes public` shows a mounted public volume (the virtual SD disk, created with `sm set-virtual-disk true` and `sm partition ... public` when absent) |
| `build`, `newer-apk` | `:app:assembleDebug :app:assembleDebugAndroidTest`; then the same debug app as versionCode + 1 and the next patch version (`1.0.1`, no `-dev`, as the descriptor wants) |
| `mocks` | FDM and printer mocks as `app_e2e.sh`, plus `plat_registry_mock.py` (device ports 19392, and 19393 for its redirect target) |
| `bundle`, `install`, `launch`, `push` | as `app_e2e.sh` |
| `policies` | step `provision_and_policies`: import from the inbox, Ready, then through the system: the 14 restrictions of PD206 and PD231, `no_debugging_features` absent in debug, `no_physical_media` and `no_config_wifi` absent, lock task features 0, auto time on, auto time zone off, zone Europe/Brussels, uninstall blocked, stay awake 7, keyguard off |
| `sales` | step `sales`: manager login, 50 sales of one line then 10 of 6 lines, each `createOrder` (takeaway), `addOrderLine`, `checkout` with one cash part without an amount (the engine settles it, no arithmetic in the test), then `generateZReport`; the PASS detail carries `bytes_per_sale` and the sizes of section 12 |
| `export` | step `export`: `Exporter` with a `FileTarget` on the app-specific directory of the removable volume, every `MANIFEST.sha256` entry hashed again in the app; then `sha256sum -c` on the device, the folder pulled and `shasum -a 256 -c` on the host, and `PRAGMA integrity_check` of the exported `pos.db` |
| `declared` | the host copies the newer APK to the registry directory and writes `declared.json` (PD211) with the hashes and the signer from `apksigner verify --print-certs`, signed with the development minisign key (read in place) and verified with `dev-minisign.pub` |
| `update` | step `update`: `UpdateVerifier` with `GhcrSource` (the debug override `filesDir/platform/registry.json`), then `UpdateInstaller.install`; the install ends the process; the host checks a new pid, `dumpsys package` versionCode, `state=Ready` of the new pid, `mLockTaskModeState=LOCKED`, and that the registry log shows the bearer on the blob requests of the main port and none on the redirect target |
| `refusals` | step `update_refusals` twice with the registry swapped: a signed descriptor for the installed versionCode (`not_newer`) and a newer one with a wrong SHA-256 (`sha256_mismatch`); a refusal leaves no new file |
| `watchdog` | step `fatal`: `markFatal`; the watchdog kills the process; new pid, Ready, LOCKED, timings |
| `audit` | step `check_audit`: the rows of section 4 |
| `teardown` | the debug teardown broadcast, owner cleared, both packages uninstalled, the export folder on the volume, the reverse rules, the mocks, the temporary directories and the newer-apk copy removed |

Choices to know:
- Only P6-T11 and the phase exit start or reuse the emulator, never while another AVD runs (`scripts/android/emulator.sh ensure`). The AVD is never wiped. Other lanes' gates also use it, so the script waits while another `app_e2e.sh`, `plat_e2e.sh` or device-test script runs; a run that starts after the install is not prevented.
- The removable volume is the virtual SD disk of vold (S4-T4, Q20). It is vfat, so the exFAT and FAT32 behaviour stays hardware residue.
- The newer APK cannot come from the injected AGP properties (`-Pandroid.injected.version.code` and `.name` are ignored by the AGP of this repository: the APK kept its version), so it is built from a throwaway `git archive HEAD` copy under `target/p6-newer` of the work tree whose `build.gradle.kts` is patched there only (a registered `git worktree` under `~/.cache` lost its `target` directory in the middle of a build to the disk guard of the shared Mac). The copy is removed at the end; `git worktree list` never names it.
- The instrumented steps do not use `am force-stop` (PD118). The install and the watchdog end the instrumented process on purpose, so the restart checks compare pids and read only the log of the new pid.

**Run of 2026-10-09 (this card), after the fix of `UpdatePreconditions.zIsDone`: `PLATE2E | total | PASS | 0 failed (19 checks)`.** Every check passes, `emulator` to `teardown`. Lines (detail shortened; the full output of the run is kept in `target/android-plat-e2e/final2.out` of the work tree):
- `policies | PASS | PLATE2E_POLICIES ok restrictions=14 zone=Europe/Brussels`
- `sales | PASS | sales=50 ... bytes_per_sale=88544 (file sizes, WAL included); logical: bytes_per_sale=6471; multi-line: sales=10 lines=6 bytes_per_sale=14336; backups: after_z files=1 bytes=778240`
- `export | PASS | 8 files, 1562220 bytes ...; MANIFEST.sha256 verified on the device (sha256sum -c: 7 OK) and on the host (shasum -a 256 -c: 7 OK); pos.db integrity_check ok`
- `declared | PASS | pili-pos-android-1.0.1.apk ... declared.json signed with the development key and verified with dev-minisign.pub`
- `update | PASS | versionCode 10000 -> 10001, pid 21700 -> 21757, 1612 ms from the commit line to state=Ready of the new pid, mLockTaskModeState=LOCKED within 1s of the end of the instrumentation; registry: 3 blob request(s) with the bearer redirected, 3 on the redirect target, 0 with Authorization`
- `refusals | PASS | installed versionCode 10001; case=not_newer code=not_newer case=sha256_mismatch code=sha256_mismatch; the refused download left no APK in filesDir/platform/update`
- `watchdog | PASS | fatal sent, pid 22105 -> 22152, 3646 ms from markFatal to state=Ready of the new pid (3000 ms of it is the watchdog's wait), mLockTaskModeState=LOCKED within 0s`
- `audit | PASS | rows=31 kinds=13 update_done_version=1.0.1 versions=[10000, 10001]; policy, export, update (verified, refused, started, package_replaced, done) and watchdog (fatal, restart, restarted) rows`
- `teardown | PASS | owner cleared, packages uninstalled, export folder, reverse rules, mocks, temp dirs and newer-apk copy removed`

**Finding of the first run, fixed in the rework (history).** The first run ended with 2 FAIL of 19 (`update`, and because of it `audit`): `UpdatePreconditions.zIsDone` could never be true after a real Z (`Blocked(failures=[Z_NOT_DONE])`, `eventsAfter=63:R` after a Z at event 62). The engine signs the financial Z and the user Z in one operation (M184, `report.rs`), as two events of type `R` with consecutive ids, and the `z_reports` row points to the first one. `zIsDone` now ignores the `R` whose id is `fiscalEventId + 1` (`Z_PAIR_TYPE`); `UpdatePreconditionsTest` has the pair cases (second `R` ignored, a further `R` or a sale right after the Z still refused). The audit and update numbers quoted in sections 4 and 5 come from the run above, made with the committed code.

## 14. Gates

Status: as built (P6-T2)

`scripts/android/p6_gate.sh` runs `p4_gate.sh` with the same options, re-prints its check lines (not its total line) and adds the P6 checks. The output contract is the one of `p4_gate.sh`: one `GATE | <check> | PASS|FAIL|SKIP | <detail>` line per check, every check runs even after a failure, exactly one `GATE | total | PASS|FAIL | <n> failed (<m> checks)` line at the end, exit 1 on any FAIL, the output of a failed check on stderr.

| Check | What it verifies |
| --- | --- |
| `plat-manifest` | `allowBackup="false"`, `fullBackupContent="false"`, `dataExtractionRules="@xml/data_extraction_rules"`, no `android:installLocation` on `<manifest>`, no `REQUEST_INSTALL_PACKAGES`, `READ_EXTERNAL_STORAGE`, `WRITE_EXTERNAL_STORAGE`, `MANAGE_EXTERNAL_STORAGE` or `READ_MEDIA_*` |
| `plat-backup-rules` | `<cloud-backup>` and `<device-transfer>` each exclude `root`, `file`, `database`, `sharedpref`, `external`; no `<include>` |
| `plat-strings` | every `values/strings_plat_<area>.xml` has a twin in `values-en/` and `values-zh/` with the same names, each starting with `plat_<area>_`, no empty value; `translatable="false"` only in `values/` |
| `plat-boundary` | under `platform/` (comments ignored): no `EngineHolder`, `force-stop`, `forceStop`, `killBackgroundProcesses`, `settlePreview`, `vatSplit`, `cashDueCents`; `http://` only on a line with `127.0.0.1` or `localhost` |
| `plat-keys` | SKIP while `plat_update_keys.xml` is absent; else the `debug` key equals the key line of `dev-minisign.pub` and the `release` key is empty or matches `^RW[A-Za-z0-9+/]{54}$` |
| `plat-engine-ref` | SKIP while `plat_about_engine_ref` does not exist; else the value must equal `pili-pos <V>, <TAG>` with V from the working tree and TAG = `PILI_P6_FREEZE_TAG` (default `android-v1.0.0-rc3`); any other value (the old `(pending)` and `(SHORT)` forms too) is a FAIL. TAG absent: PASS "tag TAG not set yet". TAG present: FAIL unless `git show TAG:Cargo.toml` has version V and `TAG:android/app/src/main/res/values/strings_plat_about.xml` holds the same value, else PASS "matches TAG (<short>)" |
| `selftest-p6-gate` | full mode only: `scripts/android/test_p6_gate.sh` |
| `selftest-declare` | full mode only: `scripts/release/test_declare_android.sh`, SKIP while absent |
| `plat-e2e` | `--plat-e2e` only: `scripts/android/plat_e2e.sh`, SKIP while absent |

Profiles (PD202): `p6_gate.sh --app --fast --base android/lane-plat --allow "$(python3 scripts/android/lane_paths.py --regex platform)"` for every commit of a card, without `--fast` at the end; the card that changes `android/docker/` drops `--app`; P6-T11 adds `--plat-e2e --app-e2e`. The Kover floor stays at 80 percent of lines and 60 percent of branches.

Self-test: `bash scripts/android/test_p6_gate.sh` builds temporary git repositories, uses a stub sub-gate and prints `TESTP6 | total | PASS | <n> tests`. The `plat_engine_ref` check follows a re-freeze: when the freeze tag changes, the string and `PILI_P6_FREEZE_TAG` (and `p4_gate.sh`, `p7_gate.sh`) must follow.
