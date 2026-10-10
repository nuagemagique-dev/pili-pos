# Runbook: the Android till on a real Sunmi D3 Pro

Card P7-T12 (phase P7, end-to-end parity). English. This is the procedure to follow with a real Sunmi D3 Pro (GMS
variant) and a real Epson TM-m30III. Nothing in it has been run on a real unit: no D3 Pro and no Epson is attached
to the build host, and the chain never scans the LAN (P7D14). Each section therefore says three things: what to do
(commands), what you must see and keep (expected result and evidence), and which emulator result it mirrors, so that
a difference between the unit and the emulator is visible at once. The last section is a table to fill in.

Conventions: `$S` is the serial of the unit (`adb devices`), `adb` means `adb -s $S`, `PKG` is
`dev.nuagemagique.pilipos` for the release APK and `dev.nuagemagique.pilipos.debug` for the debug APK. The
emulator stand-in is the AVD `sunmi_d3_pro_api33` (1920x1080 landscape, 160 dpi, 4 GB, API 33 arm64,
`google_apis`, no Play) and, for the Play Protect items, `sunmi_d3_pro_gms_api33`.

Documents this runbook leans on:

| Document | Used for |
|----------|----------|
| `UC_RESULTS.md` and `UC_MATRIX_ANDROID.md` | the 48 scenarios and the 47 use-case ids already proven on the emulator against the reference |
| `PLATFORM.md` sections 3, 7, 8, 9, 12 | policies, SPF export, updater, storage and the SD card |
| `APP.md` sections 3, 4, 9, 10 | provisioning, kiosk, the end-to-end run, the known limits (hardware residue) |
| `SOAK.md` | the 24 h soak on the emulator, its figures and the pass rules |
| `POWERCUT.md` | the power-cut cases (card P7-T6). **Not delivered at this head**: the card did not run (DECISIONS P7D21), so section 8 below states the checks itself |
| `GMS.md` | the GMS emulator run and Play Protect findings (card P7-T11). **Not delivered at this head** (DECISIONS, wave 5 record), so section 10 lists what to look at without an emulator result to compare with |
| `LIVE_MODULE2.md` | the live run against the module2 simulator (separate, needs the user) |
| `spikes/S4.md` | the device-owner, Lock Task and restriction evidence this runbook repeats on hardware |

## 0. Prerequisites

| Item | Needed |
|------|--------|
| Unit | Sunmi D3 Pro (GMS variant), charged, on its charger for the soak, Android 13 (API 33), arm64 |
| Host | `adb` (platform-tools), `shasum` or `sha256sum`, `sqlite3`, `minisign`; the repository at the commit under test |
| Network | the till, the FDM (module2 FDM on the LAN, or the online simulator `http://fdm.module2.be/graphql/`) and the Epson on one LAN with fixed addresses; the Epson answers on raw TCP 9100 |
| Files | the release APK (or the debug APK for the test runs), a provisioning bundle signed with the release key (a development bundle for the debug APK: `scripts/android/make_dev_bundle.sh`), the FDM token file if the FDM needs one |
| Media | an exFAT microSDXC card (64 GB high-endurance) and an exFAT USB stick, both empty except for a folder named `PILI-EXPORT` |
| Paper | one roll for the Epson, a second roll for the soak |

Record before you start: the unit serial (label and `adb shell getprop ro.serialno`), `adb shell getprop
ro.build.fingerprint`, `ro.build.type`, `ro.build.version.security_patch`, the free internal storage (`adb shell
df -h /data`), the date.

## 1. Provisioning without a Google account (the adb path, the supported one)

Mirrors: `spikes/S4.md` section 1, `APP.md` section 9 (`app_e2e.sh` steps `precheck`, `install`), `plat_e2e.sh`.

Rule of the platform: `dpm set-device-owner` is refused when an account exists or when there is more than one
user. The unit must therefore reach the home screen without any Google or Sunmi account.

1. Factory reset the unit. At the welcome screen choose the language, connect Wi-Fi (or Ethernet), and **skip**
   every account step (Google sign-in, Sunmi account, "Add account"). Do not restore from a backup.
2. Settings, About, tap the build number seven times; Developer options, switch on USB debugging; accept the
   host's RSA prompt on the unit.
3. Check that the device is clean:
   ```sh
   adb devices                                  # $S shows "device", not "unauthorized"
   adb shell dpm list-owners                    # prints nothing
   adb shell pm list users                      # exactly one user (UserInfo{0:...})
   adb shell dumpsys account | grep -c "Account {"   # 0 (no account)
   adb shell pm list packages | grep -c pilipos      # 0
   ```
4. Install and make the app device owner (the exit code of `dpm` is 0 even when it refuses: read the text):
   ```sh
   adb install -r app-release.apk
   adb shell dpm set-device-owner dev.nuagemagique.pilipos/.provisioning.PiliAdminReceiver
   adb shell dpm list-owners
   ```
   Expected: `Success: Device owner set to package ...` and `User 0: admin=...PiliAdminReceiver,DeviceOwner`.
   A transient `Trying to set an admin which is being removed` is retried after a few seconds (S4 finding; the
   scripts retry up to 8 times with a 3 s pause).
5. Launch the app once (`adb shell am start -n dev.nuagemagique.pilipos/.MainActivity`). The shell says the till
   is not provisioned (`PiliShell state=NotProvisioned` in logcat) and shows the bundle import screen.
6. Flush before you call it done: `adb shell sync`, wait about 12 s, or reboot (S4: a policy write within about
   10 s of an owner change is lost on an abrupt stop).

Evidence to keep: the text of the `set-device-owner` answer, `dpm list-owners`, `pm list users`, the
`ro.*` properties above, a photo of the screen after step 5.

If `set-device-owner` is refused with "already several users" or "there are already some accounts": remove the
account or the extra user, or factory reset again and skip the account. A unit that cannot be cleared is a
finding: write the exact refusal text into the table of section 11.

### 1b. QR provisioning on a unit that already has an account (not usable with the app as built)

Mirrors: nothing; `APP.md` section 10 and `spikes/S4.md` list it as hardware residue.

On a unit that has just been reset, Android can make an app the device owner from a QR code, even when an account
screen would otherwise be in the way:

1. At the very first welcome screen tap the same spot 6 times; the unit asks for Wi-Fi and opens the QR scanner.
2. The QR code carries a JSON payload. For this app it would be:
   ```json
   {
     "android.app.extra.PROVISIONING_DEVICE_ADMIN_COMPONENT_NAME": "dev.nuagemagique.pilipos/.provisioning.PiliAdminReceiver",
     "android.app.extra.PROVISIONING_DEVICE_ADMIN_PACKAGE_DOWNLOAD_LOCATION": "https://<release host>/pili-pos.apk",
     "android.app.extra.PROVISIONING_DEVICE_ADMIN_PACKAGE_CHECKSUM": "<SHA-256 of the APK, base64url, no padding>",
     "android.app.extra.PROVISIONING_SKIP_ENCRYPTION": true,
     "android.app.extra.PROVISIONING_TIME_ZONE": "Europe/Brussels",
     "android.app.extra.PROVISIONING_LOCALE": "fr_BE"
   }
   ```
   The checksum is the SHA-256 of the APK file: `openssl dgst -sha256 -binary app-release.apk | openssl base64 -A
   | tr '+/' '-_' | tr -d '='`. The URL must be reachable from the unit over HTTPS and serve exactly that file.
   Turn the JSON into a QR code with any generator (one that does not upload the text: the URL is not secret, but
   keep the habit).
3. Scan it. The unit downloads the APK, checks the checksum and installs it as the device owner.

**Finding, not tried on hardware:** the app declares the `DEVICE_ADMIN_ENABLED` receiver only. Android 9 and later
also expects a device policy controller to answer the provisioning handshake (activities for
`android.app.action.GET_PROVISIONING_MODE` and `android.app.action.ADMIN_POLICY_COMPLIANCE`), and the manifest
has neither (`grep` of `android/app/src` finds no reference). Expect the QR route to fail on the unit with a
"can't set up" message until a card adds them. There is also no release host for the APK yet. So the adb path of
section 1 is the way to provision a till, and the QR route stays on the residue list. If you try it, keep the
photo of the failure and `adb logcat -d | grep -i provision` (the unit allows adb only after setup).

## 2. The bundle import

Mirrors: `APP.md` section 3, `app_e2e.sh` (`push`, `instrumented`), `plat_e2e.sh`.

The bundle is a folder: `pos.json`, the files it names, an optional `seed.sql`, `MANIFEST.json` and
`MANIFEST.json.minisig`. A release build verifies the signature against the key compiled into it
(`piliProvisioningPublicKey`); a debug build uses the development key.

1. Fill `pos.json` for the restaurant: `pos_id`, `terminal_id`, `restaurant_*`, `fdm_url`, `fdm_token_file` (if the
   FDM needs a token, a file of the bundle), `printer_address` (the Epson, `<ip>:9100`), `table_count`.
2. Sign it with the release tooling (not described here: the release key is the user's). Copy the folder to the SD
   card or the USB stick.
3. On the unit open the shell, choose the bundle folder with the picker (`prov_import_*`). Debug build only: push
   the folder to `provisioning-inbox` instead:
   `adb push <bundle>/. /sdcard/Android/data/dev.nuagemagique.pilipos.debug/files/provisioning-inbox/`.
4. Expected: the engine opens (state `Ready`), the login screen appears, Lock Task is `LOCKED`.
   ```sh
   adb shell dumpsys activity activities | grep -i mLockTaskModeState        # LOCKED
   adb logcat -d -s PiliShell:I | tail -n 5                                  # state=Ready
   ```
5. Identity: the record keeps `deviceId`, which must be the unit serial as `Build.getSerial()` returns it to the
   device owner. Compare it with the label and `ro.serialno` (hardware residue: the real format and length, 1 to
   600 characters).

Evidence: the `PiliShell` lines, the About screen (`platform/about`) with the app SHA-1 and the `deviceId`, a photo
of the login screen. Failure texts are `prov_err_<code>` (for example `prov_err_key_missing` for a release build
without a key, `prov_err_signature` for a wrong key): keep the exact text.

## 3. Lock Task and the restrictions

Mirrors: `PLATFORM.md` section 3, `APP.md` section 4, `spikes/S4.md` sections 2 and 8, `plat_e2e.sh`.

Check on the unit, in this order, with the app in front:

| Check | How | Expected |
|-------|-----|----------|
| Lock Task | `adb shell dumpsys activity activities \| grep mLockTaskModeState` | `LOCKED` |
| Home and Recents | press Home and Recents on the unit | nothing leaves the app |
| Status bar | swipe from the top edge | no shade (disabled), the system bars stay hidden |
| Power key | long press the power key | the power menu only (`GLOBAL_ACTIONS`), no Assistant |
| Settings | `adb shell am start -a android.settings.SETTINGS` | the Settings app does not come to the front (suspended) |
| Restrictions | `adb shell dumpsys device_policy \| grep -i -A2 "Enabled restrictions"` | the 13 keys of `PLATFORM.md` section 3 |
| Time | Settings is out of reach; `adb shell settings get global auto_time` and `adb shell getprop persist.sys.timezone` | `1` and `Europe/Brussels` |
| Stay awake | `adb shell settings get global stay_on_while_plugged_in` | `7` |
| Uninstall | `adb shell pm uninstall dev.nuagemagique.pilipos` | refused |
| Safe boot | hold the volume key while powering on | safe mode refused (hardware residue) |
| Recovery | find the recovery key combination, look at the menu | write the last-resort path into the table of section 11 |
| Vendor layer | `adb shell pm list packages \| grep -i sunmi` | list them, test that none bypasses Lock Task |

Evidence: the command output, a photo of each key test. The last four rows are the "NOT PROVEN ON EMULATOR" items
of S4; do not skip them.

## 4. A sale, a refund and a Z printed on the Epson over LAN

Mirrors: `UC_RESULTS.md` (M110, M111, M181 on `pos-printer-mock`; the ticket texts are equal to the reference),
`spikes/S4.md` (DLE EOT on the mock).

On this host no Epson TM-m30III is attached and the chain does not scan the LAN: this section is a procedure only.

1. Network: the Epson has a fixed address (print its self-test: hold FEED while powering on). From the host, prove
   the port is open: `nc -vz <epson-ip> 9100`. `printer_address` in `pos.json` is `<epson-ip>:9100`. The till
   prints over raw TCP 9100 only (no USB, no Bluetooth).
2. Printer status (DLE EOT, the engine reads two answers after a print): open Settings, the printer check
   (`check_printer`). Expected: online, paper ok. Then open the cover, check again (cover open), remove the paper
   (paper end), pull the cable (unreachable). Write the text the till shows for each. Record the answer time: the
   mock answers instantly, the emulator path takes about 0.5 s, the real value is unknown (the engine waits
   500 ms by default, `printer_status_read`).
3. Print, in the order of the UC scenarios:
   - a takeaway sale paid in cash with two products (M110): the receipt prints with the control data (`fdmId`,
     counters, `shortSignature`, the QR code), and the drawer opens (cash);
   - the refund of that sale (M111) and its ticket;
   - an X report (M180) and then a Z report (M181) at the end of the test day, with its automatic copy;
   - a pre-bill on a table order (M123);
   - a ticket with a Chinese product name: the Epson code page prints `?` for the characters (CP858, a product
     decision of P6), check that the line is still readable.
4. Compare with the emulator: the texts of `docs/android/uc/journals/android/` for the same scenarios are the
   expected text up to the normalisation of `UC.md` section 8. Hold the printed receipt next to the journal text:
   same lines, same order, same totals; only dates, signatures, the QR payload, the software line and identities
   differ.
5. The QR code must scan to the 38-character verification URL; scan one with a phone and read the URL.

Evidence: a photo of each ticket, `nc` output, the screen text of the printer checks, the answer times. Keep the
tickets (they are fiscal documents): do not reuse a test day's tickets for a customer.

## 5. The UC runs

Mirrors: `UC_RESULTS.md` (48 scenarios in 8 groups, 48 PASS on both sides, `UCCHECK | total | PASS`),
`UC_MATRIX_ANDROID.md`.

`scripts/android/uc_e2e.sh` runs the scenarios on the emulator: it asks `emulator.sh ensure` for its device and
does not read `ANDROID_SERIAL`, so on the unit it cannot be pointed at the hardware as it stands. (`device_engine_tests.sh`
does read `ANDROID_SERIAL`.) Two ways:

- **Preferred, once a card adds a serial option to `uc_e2e.sh`:** `ANDROID_SERIAL=$S bash scripts/android/uc_e2e.sh
  --compare --group 00_smoke --group 10_basic --out target/d3pro-uc`. Write this finding into the table of
  section 11 as an open item.
- **By hand, with the debug APKs, the same steps the script does:**
  1. `bash scripts/android/make_dev_bundle.sh --seed --out /tmp/d3-bundle --fdm-url http://127.0.0.1:19380/graphql
     --printer 127.0.0.1:19390` (the debug bundle, signed with the development key).
  2. Start the mocks on the host: `PORT=8081 target/android-mocks/debug/pos-fdm-mock` and
     `PORT=9100 HTTP_PORT=9180 target/android-mocks/debug/pos-printer-mock`; reverse the ports to the unit:
     `adb reverse tcp:19380 tcp:8081`, `adb reverse tcp:19390 tcp:9100`, `adb reverse tcp:19391 tcp:9180`.
  3. Install both debug APKs, make the debug package device owner (section 1, with `.debug`), push the bundle to
     the inbox, run `adb shell am instrument -w -r -e class dev.nuagemagique.pilipos.uc.UcProvisionTest
     dev.nuagemagique.pilipos.debug.test/androidx.test.runner.AndroidJUnitRunner`.
  4. For each scenario: clear the mock logs, then `adb shell am instrument -w -r -e class
     dev.nuagemagique.pilipos.uc.UcScenarioTest -e group 10_basic -e scenario 010_M110 -e fdmPort 19380 -e
     printerHttpPort 19391 <runner>`; pull the steps file and build the journal with `scripts/android/uc_journal.py`.
  5. Compare with `pos-uc-check record` and `pos-uc-check compare --fixtures` (`UC.md` sections 5 and 6).

Expected: every scenario PASS on the unit, `UCCHECK | total | PASS`. A difference between the unit and the
emulator is an ESCALATE, never a new volatile path (P7D8). Evidence: the journals and the comparison report.

## 6. The SPF export to an SD card or a USB stick

Mirrors: `PLATFORM.md` sections 7 and 12 (spike S4 and `plat_e2e.sh` proved the SAF export on a virtual SD card).

The database never lives on the card (PD220, `PLATFORM.md` section 12): `pos.db` is in the app's private internal
storage. The card is a removable volume for the export and for the update from a stick, like a USB stick.

1. Format the card and the stick **exFAT** (FAT32 refuses a file of 4 GiB or more, which the database reaches
   after about 3 years at 300 sales a day). On a Mac: Disk Utility, ExFAT, GUID. Create a folder `PILI-EXPORT`.
2. Insert the card in the slot of the unit. Check it mounts as a public volume:
   `adb shell sm list-volumes public` shows `mounted`; `adb shell sm list-disks` shows it. Repeat with the stick on
   the OTG port (the port may need enabling; write down what it takes, and whether `DISALLOW_USB_FILE_TRANSFER`
   or `DISALLOW_MOUNT_PHYSICAL_MEDIA` changes it).
3. Log in as a manager with no order open and run Settings, Export. First time: choose the `PILI-EXPORT` folder
   (the root of the volume is refused on purpose: `plat_export_err_root`).
4. Expected: the Done screen with the folder `pili-export-<yyyyMMdd-HHmmss>-<deviceId>`, the file count, the total
   size and the SHA-256 of `MANIFEST.sha256`.
5. Verify on a computer, as an inspector would:
   ```sh
   cd pili-export-...
   shasum -a 256 -c MANIFEST.sha256            # every line OK (sha256sum -c on Linux)
   sqlite3 pos.db "PRAGMA integrity_check;"    # ok
   shasum -a 256 MANIFEST.sha256               # equals the hash shown on the Done screen
   ```
   `identity.json` must carry the application hashes of the installed build, `backups.txt` the list of the other
   backups, `audit/audit.jsonl` the row "started" and "done" of this export.
6. Failure cases to try, and write down the exact screen text: pull the card in the middle of a write (a database
   of a few MB takes seconds: add a large file to the backups directory to lengthen it, or export a database
   after a long soak); insert a FAT32 card and export (expected `plat_export_err_fat32_hint` only when a file of 4
   GiB or more fails; a small export succeeds on FAT32); export with an order open (blocked); log in as a cashier
   (not a manager).
7. Size: note the free space of the card and the total export size, and compare with the soak figures of
   `SOAK.md` (one export is `pos.db` plus the 7 newest backups, about 8 times the database).
8. The largest card the D3 Pro accepts is a hardware fact: try a 128 GB and, if you own one, a 256 GB card, and
   record what the slot does.

Evidence: the Done screen, the three verification outputs, the photos of the failure screens, the mount
lines. The export never deletes anything on the card or on the till: the manager clears old folders by hand.

## 7. The declared-only update

Mirrors: `PLATFORM.md` sections 8 and 9, `plat_e2e.sh` (the updater and the crash restart on the emulator).

Updates are a manager action from a stick (and from the registry when the release exists). Only a **declared**
release installs: the descriptor is signed with the update key (`plat_update_keys.xml`, PD212) and carries the
APK's SHA-256 and the version. `scripts/release/declare_android.sh` makes the declaration; **the chain never runs
it for real** (it needs the release key, which is the user's).

1. Prepare a stick with the declared layout of `PLATFORM.md` section 8.3 (OCI layout: descriptor, signature, the
   APK blob) for a version one higher than the installed one.
2. On the unit, as a manager with no open order and the day's Z done, open Settings, Update, choose the stick
   (`update_from_usb`). Expected: the screen shows the version, `versionCode`, the APK SHA-1 and the signer;
   the Install button is enabled only when the preconditions are met (`z_due`, `z_not_done`, `open_order`,
   `not_manager` are listed when they are not).
3. Negative cases: an APK that is not the declared one (flip a byte), a descriptor signed with another key, an
   older version, an APK signed by another certificate. Expected: refused with the codes of `PLATFORM.md`
   section 8.6, nothing installed.
4. Install the good one. Expected: a backup is made, the engine closes, the system installs silently
   (`USER_ACTION_NOT_REQUIRED`, device owner), the app restarts by itself in Lock Task
   (`MY_PACKAGE_REPLACED`), and the audit row `done` appears with the running version equal to the target.
   ```sh
   adb shell dumpsys package dev.nuagemagique.pilipos | grep versionName
   ```
5. On the unit check what the Sunmi `PackageInstaller` does under a device owner (silent or a prompt:
   `silent_install_refused` is the failure code), and that the sale numbering continues across the update.

Evidence: the audit file (`filesDir/platform/audit.jsonl`, in the next export's `audit/`), the versions before and
after, the refusal texts. No rollback exists: a faulty version is fixed by a newer declared version.

## 8. The power loss: a real plug pull

Mirrors: `spikes/S4.md` section 10 (`adb emu kill`: LOCKED 3 s after boot, integrity ok). The power-cut card P7-T6
(`POWERCUT.md`, cut points C, A1, A2 and B of P7D11, with a host delay proxy) did not run at this head, so the
emulator side of this section has only the S4 result. `adb emu kill` ends qemu without a guest shutdown but the
host page cache still holds qemu's writes, so it is milder than a flash power loss: the real pull is the real test.

The four cut points (P7D11) and the expected behaviour, which is the behaviour of
`crates/pos-engine/tests/spf_e2e_crash_safety.rs`:

| Cut | How | Expected after the cold boot |
|-----|-----|------------------------------|
| C | a few committed sales, then pull the plug while the till is idle | the sales are all there; the WAL replays |
| A1 | pull the plug while a sale is being sent to the FDM and before the FDM has it (put a delaying proxy between the till and the FDM, or unplug the FDM's network at the same time) | no ticket number is used; the next sale takes the next number |
| A2 | pull the plug after the FDM has signed the sale and before the till has committed the answer (the key case) | the signed request stays open; the next signing operation sends it again under the same number; the FDM counts it once; no number is lost or doubled |
| B | pull the plug during a Z report (A2 style) | the Z is completed or re-sent once; one Z per day; the post-Z backup exists |

A real FDM answers in about a second, so A2 needs several tries, or a proxy that holds the answer for a few seconds.
The delay proxy of card P7-T6 (`fdm_delay_proxy.py`) was not delivered; a few lines of Python between the till and
the FDM do the same.

For each cut:
1. The D3 Pro has a battery, so pulling the charger alone leaves the unit running and proves nothing. A real power
   cut is the battery removed with the charger unplugged (ask the supplier whether the cover opens), or the
   power key held until the unit dies. Record the method you used.
2. Reboot. Time how long it takes to reach the login screen in `LOCKED`, and whether `BOOT_COMPLETED` started the
   app by itself (`APP.md` and S4: not observed on the emulator after the power cut).
3. Checks: `PRAGMA integrity_check` returns `ok` on the exported `pos.db` (section 6); the fiscal events are
   contiguous (no gap in the per-label counters of the FDM and in the till's `fiscal_events`, compare with the
   FDM's own display or the simulator's Input/Output log); the sale signed at the cut shows once; the open table
   order is still open; the engine log shows the recovery (`filesDir/logs`); no second signing of the same
   number.
4. Repeat the update and the export with a pull in the middle (the update is excluded by its preconditions in
   normal use, but the pull may happen): after the boot either the old or the new version runs, never a broken
   install, and the marker `pending.json` is dealt with (`not_completed` audit row).

Evidence: for each cut the times, the export set of the next day, the FDM counters before and after, the
`integrity_check` output.

## 9. The 24 hour soak on the unit

Mirrors: `SOAK.md` (the same load profile on the emulator, the pass rules of P7D12 and the figures).

`scripts/android/p7_soak.sh` drives the emulator only (it asks `emulator.sh ensure`). On the unit use the same
instrumented driver by hand, plugged in:

1. Install the debug APKs and provision as in sections 1 and 2 (debug build, development bundle, mocks on the host
   with `adb reverse`, or the real FDM and printer).
2. Start the driver without waiting for it, so that it survives an adb disconnect:
   `adb shell am instrument -r -e class dev.nuagemagique.pilipos.soak.SoakDriverTest -e hours 24 -e dayMinutes 60
   -e salesPerDay 100 -e seed <n> -e idleGapMinutes 6 <runner>` (no `-w`; `<runner>` is
   `dev.nuagemagique.pilipos.debug.test/androidx.test.runner.AndroidJUnitRunner`, the call is in `p7_soak.sh`).
   Keep the unit plugged in, screen on (`adb shell svc power stayon true`; the device-owner policy sets
   `stay_on_while_plugged_in` anyway).
3. Sample from the host every minute, like `soak_sampler.py` does: `dumpsys meminfo <pkg>` (TOTAL PSS and Native
   Heap), `/proc/<pid>/status` (Threads, VmRSS), the number of entries of `/proc/<pid>/fd` through `run-as`, `df
   /data`, the size of `pos.db`, its WAL and `backups/`, and the pid.
4. Pass rules (P7D12): one pid, no crash or ANR, every sale signed, counters continuous, one Z per compressed day,
   PSS and native heap slope after the first hour at most 5 MB/h, threads at most baseline plus 20, open fds at
   most baseline plus 50, largest heartbeat gap at most 120 s, free internal storage at least 1 GiB.
5. What the emulator cannot show and the unit must: deep sleep (the SoC suspends; compare the heartbeat gaps with
   the emulator's), thermal behaviour (fanless: note the case temperature and any throttling in `dumpsys
   thermalservice`), the battery state while plugged in (`dumpsys battery`), and the survival of the foreground
   service. Decide from the gaps whether a partial wake lock or an alarm-based heartbeat is needed.
6. Repeat once with the screen off for a night (the till in a closed restaurant), then compare.

Evidence: the sampler files, the driver file, `dumpsys thermalservice` at the start, the middle and the end.

## 10. Play Protect and the developer verification items

Mirrors: `GMS.md` (card P7-T11, **not delivered**): there is no emulator result to compare with. The emulator for
this section is the GMS AVD `sunmi_d3_pro_gms_api33`.

The D3 Pro GMS variant ships Google Play services and Play Protect. A till with a self-signed or privately signed
APK meets them at three points.

1. **No Google account before device owner** (section 1): Settings of the unit show no account.
2. **Play Protect on install.** After `adb install`, the unit may show "App blocked to protect your device" or
   "Scan app" for an app not from Play. Note whether it appears for `adb install`, for an install from a stick
   (updater path), and for the PackageInstaller session of the updater. Look at: Play Store, Play Protect, the
   gear: "Scan apps with Play Protect" and "Improve harmful app detection". The Play Store is hidden by the user
   (it is not a policy of the app: a platform card of PD230 would hide it and set `DISALLOW_INSTALL_APPS`).
   `adb shell settings get global package_verifier_enable` and `verifier_verify_adb_installs` show whether adb
   installs are verified.
3. **Developer verification.** Google announced that apps on certified Android devices will have to come from a
   developer who has verified their identity, with an exception path for private distribution to a controlled
   fleet. Check, on the date of the test, what the unit does with this APK (registered package name
   `dev.nuagemagique.pilipos`, signing certificate) and whether a device-owner install is exempt. Write down the
   Android security patch level, the Google Play services version
   (`adb shell dumpsys package com.google.android.gms | grep versionName`) and the exact text of any warning.
   This item can change with every Play services update: record the version.
4. **Play services policy interference.** `adb logcat -d | grep -i -E "verifier|gms|protect"` after an update
   through the updater: no prompt must reach the screen in Lock Task, because nobody is there to answer it.

Evidence: photos of every warning, the properties above, the logcat lines. A warning that needs a click is a
blocker for the unattended updater: write it up first.

## 11. The hardware residue checklist (to fill in)

Everything below is open until a real D3 Pro (and a real Epson and USB stick) is available. Fill in the last two
columns on the unit; keep the evidence next to this file.

| # | Item | Emulator result it mirrors | Result on the unit | Evidence file |
|---|------|----------------------------|--------------------|---------------|
| 1 | Real serial: format, length, `Build.getSerial()` as device owner equals the label and `ro.serialno` | fake serial of 17 characters (`APP.md` section 10) | | |
| 2 | `set-device-owner` on the unit without account, one user; the refusal text with an account | `Success` on the AVD; account case not run (S4) | | |
| 3 | QR provisioning (6 taps) and the missing provisioning handshake of the app | not run | | |
| 4 | Epson TM-m30III over LAN: sale, refund, X, Z, copy, pre-bill, drawer kick, Chinese line, QR scan | `pos-printer-mock` (`UC_RESULTS.md`) | | |
| 5 | Epson DLE EOT: answer time, paper end, cover open, unreachable | mock answers instantly; emulator path about 0.5 s | | |
| 6 | SD card: auto-mount, exFAT export, `fd.sync` time, throughput | virtual vfat SD disk (S4, `plat_e2e.sh`) | | |
| 7 | USB stick: OTG port enabling, auto-mount, picker, exFAT and FAT32, the 4 GiB limit | virtual SD stands in | | |
| 8 | Removal of the card or stick during an export write | not testable | | |
| 9 | Largest card accepted by the slot (64, 128, 256 GB) | n/a | | |
| 10 | Free internal storage of the unit at delivery, and after the soak | emulator `df` (see `SOAK.md`) | | |
| 11 | Deep sleep and heartbeat gaps over 24 h, screen on and off | emulator CPU is never suspended | | |
| 12 | Thermal behaviour over 24 h (case temperature, throttling) | none | | |
| 13 | Plug pull at cut points C, A1, A2 and B; boot time to `LOCKED`; `BOOT_COMPLETED` | `adb emu kill` only (S4 section 10) | | |
| 14 | Safe boot with the volume key and the recovery menu under the restriction set | power-menu route only | | |
| 15 | Sunmi vendor layer and launcher: crash and ANR dialogs, auto-start allowlist | Google launcher on the AVD | | |
| 16 | System OTA under the postpone policy; can it reboot while the till is open | postpone policy set and read back | | |
| 17 | Play Protect warnings on install and on the updater path (GMS variant) | `GMS.md` not delivered | | |
| 18 | Developer verification status of the APK on the unit | none | | |
| 19 | Display: `wm size`, `wm density`, usable height in Lock Task, fonts, 8.6 mm touch targets | profile declared, 996 of 1080 px usable | | |
| 20 | bcrypt cost 12 PIN check on the Qualcomm CPU (`cmd bcrypt`) | 295 ms on the AVD, 232 ms on the host | | |
| 21 | Wi-Fi and Ethernet MAC behaviour (randomisation) | fixed fake MAC | | |
| 22 | `uc_e2e.sh` against the unit (needs a serial option) | AVD only | | |
| 23 | Update on the unit: Sunmi `PackageInstaller` under device owner, silent install | emulator install works silently | | |
| 24 | adb off after provisioning: nothing can install or downgrade, developer options cannot re-enable adb (`user` build) | AVD is userdebug | | |

When the table is full, copy its figures into the P7 exit report (or the dossier note on the test environment).
Until then the dossier states that the tests ran on an emulated profile of the D3 Pro.
