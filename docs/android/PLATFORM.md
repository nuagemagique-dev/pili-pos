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

Status: design (P6-T3)

`platform/policy/` applies, at every application start and every activity start, each step isolated and its failures audited (PD206):

- the Q16 set: 12 user restrictions plus `no_system_error_dialogs`;
- `setUninstallBlocked` for the application;
- auto time on, auto time zone off, time zone `Europe/Brussels`;
- stay awake while plugged in (value 7);
- keyguard and status bar disabled;
- the postpone system update policy;
- `setLockTaskFeatures(NONE)`, applied after `KioskController.prepare()`, so it overrides the global actions the kiosk enables. In production the staff lose the power menu (question for the user).

Exceptions:
- `DISALLOW_DEBUGGING_FEATURES` (adb off) is applied only in release builds and only once a provisioning record exists. Debug builds never apply it, because the emulator loop needs adb. There is no timed adb (DECISIONS 7).
- `DISALLOW_MOUNT_PHYSICAL_MEDIA` and `DISALLOW_CONFIG_WIFI` stay off, so an SD card or a USB stick mounts and the Wi-Fi can be fixed on site.

The policies end when the debug teardown clears the device owner. Strings: `strings_plat_policy.xml`.

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

Status: design (P6-T4)

`platform/about/IdentityProvider` builds an `IdentityReport` (PD209):

- application id, `versionName`, `versionCode`, `posSwVersion` (`EngineHelpers.posSwVersion`) and the engine reference;
- SHA-1 and SHA-256 of the installed `base.apk` (`applicationInfo.sourceDir`), SHA-1 of the `lib/arm64-v8a/libpos_ffi.so` entry read from that APK (stored uncompressed, not extracted), SHA-256 of the signing certificates;
- `deviceId` (the provisioning record), `posId` and `terminalId` (`getSettings` when Ready);
- Android release, SDK level, build fingerprint, manufacturer and model;
- free internal bytes, count and bytes of `engine/backups/`.

Hashes are cached per `(sourceDir, lastUpdateTime)`. The engine reference is the non-translatable string `plat_about_engine_ref`, of the form `pili-pos <version>, <freeze tag> (<short sha>)`, for example `pili-pos 1.1.0, linux-v1.1.0-rc1 (65b567f)`. The gate check `plat-engine-ref` compares it with the freeze tag, so a re-freeze must update it. The About screen shows the report; the export writes it as `identity.json`.

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

Status: design (P6-T7)

One signed descriptor `declared.json` per declared Android version (PD211):

`{"format":1,"platform":"android","package":"dev.nuagemagique.pilipos","version":"X.Y.Z","versionCode":N,"asset":"pili-pos-android-X.Y.Z.apk","sha1":"..","sha256":"..","signerSha256":"..","declared_at":".."}`

with `declared.json.minisig` (trusted comment `pili-pos-android X.Y.Z declared <ts>`).

- **Online:** an OCI artifact, tag `declared-android` of the public package `ghcr.io/brunomoyse/moyun-pos` (the same package as the Linux `:declared`), pulled anonymously: a token from `/token?scope=repository:brunomoyse/moyun-pos:pull&service=ghcr.io`, then the manifest (`application/vnd.oci.image.manifest.v1+json`), whose layers are found by their `org.opencontainers.image.title` annotation, then the blobs, each checked against its digest. Redirects are followed by hand and the bearer token is never sent to another host.
- **USB (Q8):** the same three files in a folder chosen through SAF.
- **Limits:** APK 200 MB, descriptor 64 KB. HTTPS only in release builds. Debug builds read an optional `filesDir/platform/registry.json` (`{"baseUrl":"http://127.0.0.1:<port>"}`) so the emulator test can reach a host mock through `adb reverse`. The gate check `plat-boundary` allows an `http://` literal only on a line with `127.0.0.1` or `localhost`.
- **Keys (PD212):** `res/xml/plat_update_keys.xml` holds the public minisign key per build type. `debug` is the committed development key (`android/app/provisioning/dev-minisign.pub`; the gate check `plat-keys` compares them). `release` is empty until the user creates the declaration key (DECISIONS 9, Q7). With an empty key every update is refused (`plat_update_err_key_missing`).

The verification core only reads and checks; it installs nothing. Signature verification goes through `EngineHelpers.verifyMinisign`.

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
