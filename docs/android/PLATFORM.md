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
2. the 13 restrictions one by one: `no_factory_reset`, `no_safe_boot`, `no_uninstall_apps`, `no_install_unknown_sources`, `no_install_unknown_sources_globally`, `no_config_date_time`, `no_add_user`, `no_user_switch`, `no_remove_user`, `no_modify_accounts`, `no_usb_file_transfer`, `no_network_reset`, `no_system_error_dialogs`. One `getUserRestrictions` read comes first, and a restriction already listed is not added again (the step counts as ok and is marked skipped), so a normal start makes few calls;
3. `setUninstallBlocked` for the own package;
4. stay awake while plugged in (`STAY_ON_WHILE_PLUGGED_IN` = "7");
5. keyguard disabled, status bar disabled (a `false` return from the system is a failed step);
6. the postpone system update policy;
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

Status: design (P6-T5)

`filesDir/platform/audit.jsonl`, one JSON object per line (PD208): `ts` (ISO 8601 with the Europe/Brussels offset), `kind`, `event`, `detail` (string map), `versionCode`, `versionName`, `pid`.

- Every append is fsynced.
- Rotation at 1 MiB keeps 5 files (`audit.jsonl.1` to `audit.jsonl.5`).
- It is not the fiscal journal, it is never sent anywhere, and it is part of the SPF export (section 7).
- Writers: the policies, the watchdog, the export and the updater.

## 5. Crash watchdog

Status: design (P6-T5)

`platform/watchdog/EngineWatchdog` watches `EngineGateway.state` from the application scope (PD207).

- **On Fatal:** it writes an audit row, waits 3 s (the logs flush and the fatal screen is seen), records the restart and kills its own process (`Process.killProcess(myPid())` behind an injectable killer). The system restarts the persistent HOME application and the `START_STICKY` service. It never uses `am force-stop`, which the system ignores for the device-owner package (PD118).
- **Budget:** 3 restarts in 10 minutes (wall clock, persisted in `filesDir/platform/restarts.json`). Beyond that it stays on the fatal screen and audits "budget exhausted".
- **`Closed(Shutdown)` for more than 60 s,** while no platform operation (update, export) runs: it calls `EngineService.start(context)` and audits it.
- **`BootReceiver`:** audits the boot and starts `MainActivity` as a fallback to HOME.

Strings: `strings_plat_watchdog.xml`.

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

The engine reference is `pili-pos <version>, <freeze tag> (<short sha>)`, for example `pili-pos 1.1.0, linux-v1.1.0-rc1 (65b567f)`. The gate check `plat-engine-ref` compares it with the freeze tag, so a re-freeze must update the string.

`AboutScreen` has four sections: Version (version, code, engine version, engine reference), Integrity (APK SHA-1 and SHA-256, library SHA-1, signer SHA-256), Device (deviceId, posId, terminalId, Android, model) and Storage (free space, backups count and size). Hashes use the monospace figure style and the text is selectable. A warning line shows under Storage when less than 2 GiB are free or when the backups take more than half of the free space (PD219: the platform deletes nothing). Test tags: `about_root`, `about_apk_sha1`, `about_so_sha1`, `about_device_id`, plus `about_refresh`, `about_free_space`, `about_backups`, `about_warning_low_space`, `about_warning_backups`.

`AboutScreen` takes its `AboutViewModel` from an entry point of the Hilt graph (not `hiltViewModel()`), so it renders under any host activity. The golden is `android/app/src/test/screenshots/platform/about.png` (Roborazzi, 960 px window at density 0.5, the 1920 px till).

Strings: `strings_plat_about.xml`.

## 7. SPF export

Status: design (P6-T6)

The tax authority (SPF) may ask for the data of the till (AM Art. 16 and 102). The Linux practice is the full `pos.db`, the logs and the SHA-1 of the binary. The Android flow follows Q17 (PD210):

- a manager-only screen, refused while an order or a takeaway is open;
- a pre-check that a removable, non-primary volume is mounted ("Insert the USB stick or SD card");
- the picker `StorageVolume.createOpenDocumentTreeIntent()` from the locked activity (never `FLAG_ACTIVITY_NEW_TASK`); the root of the volume is refused with the hint to create a folder `PILI-EXPORT`;
- a persisted grant per volume uuid; "Change folder" releases the old grant.

The export folder is `pili-export-<yyyyMMdd-HHmmss>-<deviceId>` and holds:

| Entry | Content |
| --- | --- |
| `pos.db` | a fresh `backupDatabase()` |
| `backups/` | the 7 newest older backups |
| `backups.txt` | inventory of every backup: name, bytes, SHA-256 |
| `logs/` | the engine logs |
| `audit/` | the platform audit files |
| `identity.json` | the identity report (section 6) |
| `LISEZMOI.txt` | French, brand-neutral: what the files are and how to check `MANIFEST.sha256` with `shasum -a 256 -c` |
| `MANIFEST.sha256` | `sha256sum` format, written last |

There is no staging copy: each file is hashed while it streams to the volume, `fd.sync()` is called, then the file is read back through the content resolver and hashed again. A mismatch fails the export. Nothing on the volume is ever deleted, even after an error. A write failure on a file of 4 GiB or more says that the stick is probably FAT32 and that exFAT is needed. Every export is audited (files, bytes, result). Strings: `strings_plat_export.xml`.

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

The token and the manifest are fetched once per source object, so the descriptor and the APK come from the same manifest. Every blob is hashed while it streams and compared with its `sha256:` digest: a wrong digest is `sha256_mismatch` for the APK and `bad_descriptor` for the descriptor files. A layer larger than its limit is `too_large` before the blob is requested, and a blob that turns out longer than the manifest says is cut at the limit. Timeouts: 15 s to connect, 60 s to read. Redirects (the registry sends blobs to a CDN) are followed by hand, at most 5, and the `Authorization` header goes only to the origin (scheme, host and port) of the base URL, never to the target of a redirect. TLS is required, apart from the debug override below. Any HTTP error or transport failure is `network`.

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

Any refusal, any error and a cancellation after the download began delete the downloaded file (and its `.part`). A returned `VerifiedUpdate` leaves the file for part 2. Calls are serialised (one verification at a time).

Audit rows, kind `update`: `check_started` (`source` = `ghcr` or `usb`), then `refused` (`code`, `detail`) or `verified` (`version`, `sha1`).

### 8.6 Refusal codes and strings

`UpdateRefusalCode` has twelve codes, each with `plat_update_err_<code>` in `strings_plat_update.xml` (French in `values/`, `values-en/`, `values-zh/`) plus `plat_update_title`: `key_missing`, `network`, `bad_descriptor`, `bad_signature`, `wrong_package`, `not_newer`, `too_large`, `sha256_mismatch`, `sha1_mismatch`, `archive_mismatch`, `signer_mismatch`, `usb_missing_file`. P6-T8 adds the other strings of the screen to the same file.

### 8.7 Tests

Fixtures `src/test/resources/platform/declared.json`, `declared.json.minisig` (signed with the development key) and `declared.tampered.json`. The JVM tests cannot load the Rust library, so `RealMinisignNatives` (test sources) re-implements the check (Ed25519 over BLAKE2b-512, as `minisign -S` does) behind `EngineNatives`; its own test pins BLAKE2b to the RFC 7693 vectors. `GhcrSource` is tested against a `com.sun.net.httpserver.HttpServer` on `127.0.0.1`: no test reaches another host.

## 9. Updater: preconditions and install

Status: design (P6-T8)

**Preconditions (PD213, interim).** The FFI has no engine-side update precheck and the engine is frozen. The updater composes the check from engine answers only; it computes no money and signs nothing:
- the current user is Admin (`getCurrentUser`);
- `listOpenOrders()` and `listOpenTakeaways()` are empty;
- `needsZReport()` is false;
- no fiscal event newer than the newest Z report's `fiscalEventId` (`listFiscalEvents(200)`) has a type other than the presence types (work in, work out) of `crates/pos-fiscal`, or the till has no fiscal event at all;
- `EngineGateway.close()` returns `Closed`; `StillOpen` refuses the update.

**Manager PIN (PD214).** The "manager PIN" of the plan is the logged-in Admin session. The updater and the export check `getCurrentUser()` again right before they act. There is no second PIN prompt, because a new `login` would queue a clock-in.

**Install flow (PD215, Q18).** Order: preconditions; verification (minisign, JSON, package, `versionCode` strictly greater than the installed one, download, SHA-256 and SHA-1, archive info: package, `versionCode`, `versionName`, signer set equal to the installed one and to `signerSha256`); `backupDatabase()`; `EngineGateway.close()`; a fsynced marker `filesDir/platform/update/pending.json`; the audit row "started"; a `PackageInstaller` session (`MODE_FULL_INSTALL`, `USER_ACTION_NOT_REQUIRED`, `INSTALL_REASON_POLICY`) committed with an explicit mutable `PendingIntent` to `InstallResultReceiver`.

Afterwards:
- **Refusal:** no session, the downloaded file is deleted.
- **Failure status:** audit, marker deleted, the engine is reopened with `EngineService.start`.
- **`STATUS_PENDING_USER_ACTION`:** the session is abandoned and recorded as an error.
- **Success:** the "done" row is written by the new process (`UpdateCompletionHook`, from the marker); `PackageReplacedReceiver` starts `MainActivity`.
- **No rollback:** a faulty version is fixed by a newer declared version (DECISIONS 6).

Strings: `strings_plat_update.xml`.

## 10. Release tooling

Status: design (P6-T9)

`scripts/release/declare.sh` (Linux) lives on the unmerged `feat/usb-autoinstall` branch, and a copy here would conflict when that branch merges. The Android declaration is therefore its own script, `scripts/release/declare_android.sh`; `declare.sh --android` gets a one-line dispatch when that branch is merged (a step for the user) (PD216). The script:
- signs the reproducible unsigned APK (RSA only, v1 off, v2 and v3 on, build-tools 35.0.0 pinned);
- checks the SHA-1 that the SPF accepted;
- writes and signs `declared.json` and pushes the tag `declared-android` with `oras`.

The tag scheme is `android-vX.Y.Z`, equal to `versionName`. The script refuses `-dev` versions and EC keys. The chain only runs it with `--dry-run`, against throwaway keys in `mktemp` directories, with stub `oras` and `gh`. The real run, the real signing key and the real declaration key are the user's.

## 11. Reproducible APK

Status: design (P6-T10)

The container build (`android/docker/build_in_container.sh`) builds `:app:assembleRelease` instead of `:smoke` (PD217, PD80, PD114). `check_repro.sh` compares two `app-release-unsigned.apk` files built from the same commit; `:smoke` stays in the host gates. `android.yml` builds and checks the `:app` APK as well (the chain never triggers it). `REPRO.md` records the SHA-1 and the image digest.

## 12. Storage and the SD card

Status: design (P6-T11)

**Where the database lives.** Not on the SD card. The engine database is `filesDir/engine/pos.db` in app-private internal storage, next to `filesDir/engine/backups/`, `filesDir/provisioning/` and `filesDir/logs/` (`provisioning/ProvisioningPaths.kt`). It stays there by design (PD220):
- a fiscal journal on removable media can be pulled out, swapped, corrupted or read by another device;
- the manifest has no `android:installLocation`, so Android never moves the data to an SD card adopted as internal storage, and it requests no storage permission (`READ_EXTERNAL_STORAGE`, `WRITE_EXTERNAL_STORAGE`, `MANAGE_EXTERNAL_STORAGE`, `READ_MEDIA_*`) and no `REQUEST_INSTALL_PACKAGES`. The gate check `plat-manifest` keeps it so;
- cloud backup and device transfer are off (`allowBackup="false"`, `fullBackupContent="false"`, the exclude-everything `data_extraction_rules.xml`). The gate check `plat-backup-rules` and `BackupPolicyTest` keep it so.

Removable media are reached only through SAF grants (the export, the update from a stick).

**What the SD card is for.** It is a removable volume like a USB stick, so it is a target of the SPF export (section 7), of the update source "from a stick" (section 8) and later of the nightly export lane. Spike S4 proved the SAF export on this kind of volume (a virtual SD card stood in for the stick). `DISALLOW_MOUNT_PHYSICAL_MEDIA` stays off so the card mounts.

**Size, first estimate.** A test database of this repository holds about 1.6 KB per fiscal event and 3.5 KB per pending sign (19 events, measured with `dbstat`). With the order, its lines and its payments that is roughly 10 to 15 KB per sale. At 100 to 300 sales a day that is roughly 0.4 to 1.5 GB a year. The P4 exit report said "megabytes per year", which was too low.

Measured figure: to be measured by P6-T11 (growth per sale on the emulator, then written here).

**A finding that matters more than the card size.** The engine writes a full copy of `pos.db` to `backups/` after every Z (`crates/pos-engine/src/commands/signing.rs:761`) and never deletes one (PD219). After a year that is about 365 copies of a database that keeps growing, which is far more than the database itself and can fill the internal storage of the till within months. The platform lane deletes nothing, because the engine is frozen and deleting is a decision for the user. The About screen shows the free internal space and the size of `backups/`, and the export copies only the 7 newest backups. A retention rule (for example "keep 14 days on the till") is an engine change, best made with the re-freeze that the nightly export lane needs anyway.

**Recommendation.** A good-quality microSDXC card of 64 GB, "high endurance" or class A1/U3, formatted exFAT:
- exFAT because FAT32 refuses files of 4 GiB or more, which a database plus its copies can reach over the years;
- 64 GB holds many full exports even after several years.

Not checked, hardware residue: the largest card the D3 Pro accepts and its slot behaviour (there is no real D3 Pro here). The SD card is never the only copy: the SPF export is a manager action, and the off-till copy belongs to the nightly export lane.

## 13. End-to-end run

Status: design (P6-T11)

`scripts/android/plat_e2e.sh` proves the platform features on the AVD `sunmi_d3_pro_api33` (PD218). One line per check, `PLATE2E | <check> | PASS|FAIL | <detail>`, then `PLATE2E | total | PASS|FAIL | <n> failed (<m> checks)`. `p6_gate.sh --plat-e2e` runs it and passes on the total line.

- Only P6-T11 and the phase exit start or reuse the emulator, never while another AVD runs (`scripts/android/emulator.sh ensure`). The AVD is never wiped.
- The removable volume is the virtual SD disk of vold (S4-T4, Q20; `sm set-virtual-disk true` if absent).
- The update test reaches a host mock registry (`plat_registry_mock.py`) through `adb reverse`, using the debug-only registry override of section 8.
- Teardown as in `app_e2e.sh` (`APP.md` section 9): no device owner, no package, no reverse rule, export folders removed.
- It also measures the database growth per sale (section 12).

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
| `plat-engine-ref` | SKIP while `plat_about_engine_ref` does not exist; else it equals `pili-pos <V>, <TAG> (<SHORT>)` for the freeze tag (`PILI_P6_FREEZE_TAG`, default `linux-v1.1.0-rc1`) |
| `selftest-p6-gate` | full mode only: `scripts/android/test_p6_gate.sh` |
| `selftest-declare` | full mode only: `scripts/release/test_declare_android.sh`, SKIP while absent |
| `plat-e2e` | `--plat-e2e` only: `scripts/android/plat_e2e.sh`, SKIP while absent |

Profiles (PD202): `p6_gate.sh --app --fast --base android/lane-plat --allow "$(python3 scripts/android/lane_paths.py --regex platform)"` for every commit of a card, without `--fast` at the end; the card that changes `android/docker/` drops `--app`; P6-T11 adds `--plat-e2e --app-e2e`. The Kover floor stays at 80 percent of lines and 60 percent of branches.

Self-test: `bash scripts/android/test_p6_gate.sh` builds temporary git repositories, uses a stub sub-gate and prints `TESTP6 | total | PASS | <n> tests`. The `plat_engine_ref` check follows a re-freeze: when the freeze tag changes, the string and `PILI_P6_FREEZE_TAG` (and `p4_gate.sh`) must follow.
