# Android port: decisions (P0.7)

Draft written at the P0 exit (2026-10-06) from the P0 reports on the branches
`android/p0-P0.1`, `android/p0-S1`, `android/p0-S2`, `android/p0-S3` and
`android/p0-S5` (all reviewed APPROVE), the approved plan, and the repo at
main `96728e1`. Spike S4 (D3 Pro profile on the emulator) ran on the
`sunmi_d3_pro_api33` AVD on 2026-10-07 (report `docs/android/spikes/S4.md`);
what rested on it was marked PENDING-S4 in the draft and is now settled
(DECIDED, S4 emulator) or recorded as hardware residue.

**Status labels**

- **DECIDED:** settled by the approved plan, an earlier owner decision, or P0
  evidence. The evidence is cited. The user's approval of this file confirms it.
- **PROPOSED:** a recommendation that needs the user's explicit choice.
- **PENDING-S4:** cannot be settled until spike S4 has run on the
  `sunmi_d3_pro_api33` AVD (and, for hardware residue, on a real D3 Pro).

**Phase exit rule (plan, P0):** P0 exits when the user approves this file.

## Summary

| # | Item | Status |
|---|------|--------|
| 1 | Application id | PROPOSED |
| 2 | Version-string scheme (posSwVersion, versionName, versionCode, tags) | PROPOSED |
| 3 | deviceId source | PROPOSED; mechanism DECIDED (S4 emulator); real serial format HARDWARE RESIDUE |
| 4 | TLS approach for `HttpTls` | DECIDED (S3 evidence) |
| 5 | Pinned toolchain and library versions | DECIDED (S1, S2, S3, S5 evidence) |
| 6 | Updater trigger policy | DECIDED (policy); install mechanics DECIDED (S4 emulator); real image HARDWARE RESIDUE |
| 7 | No remote support in v1 | DECIDED |
| 8 | Admission FDM = module2 online simulator | DECIDED; test-machine questions PROPOSED |
| 9 | Signing-key custody | PROPOSED (technical constraints DECIDED by S5) |
| 10 | When to file Linux v1.1 | PROPOSED |
| 11 | Other decisions carried from P0 evidence | DECIDED (technical) |
| 12 | S4 results (was: items waiting for S4) | DECIDED (S4 emulator), HARDWARE RESIDUE or OPEN per item |

## 1. Application id: PROPOSED

`dev.nuagemagique.pos` belongs to the Pili handheld (moyun-handheld,
`app/build.gradle.kts:43`, published on Google Play), so the till needs its
own id.

**Proposal.**
- Release: `dev.nuagemagique.pilipos`. It follows the producer's domain and the
  existing product name `pili-pos`, and names no client (the dossier stays
  brand-neutral).
- Development builds: `dev.nuagemagique.pilipos.debug` (`applicationIdSuffix`,
  as moyun-handheld does). They are never installed on a till that signs for real.
- The Kotlin namespace can follow the id (`dev.nuagemagique.pilipos`).

**Why it matters.** The application id and the signing key are permanent for
an installed till. Changing either means uninstalling the app, and an
uninstall deletes app-private storage, which holds the fiscal database. A
change after the first real installation is therefore treated as a reinstall
with a full SPF export first. The id is also written into the device-owner
provisioning (component name), the dossier and the update channel.

**Needs from the user:** confirm the id, or pick another one, before P3.1
creates the Gradle project.

## 2. Version-string scheme: PROPOSED

**Facts.**
- Today `posSwVersion` is a compile-time constant:
  `concat!("pili-pos/", env!("CARGO_PKG_VERSION"))`
  (`crates/pos-fiscal/src/event.rs:12`), and the workspace version is `1.0.0`
  (`Cargo.toml:20`).
- POS spec v2.2: `posSwVersion` is a mandatory string, 1 to 36 characters
  (`docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md:1097`).
- AM Art. 47: every software version carries a unique version mention. P1a
  turns the constant into the runtime `SoftwareIdentity` so that Android does
  not send the Linux string.

**Proposal.**

| Item | Linux | Android |
|------|-------|---------|
| Product line in `posSwVersion` | `pili-pos/<X.Y.Z>` (unchanged format) | `pili-pos-android/<X.Y.Z>` |
| First version | `1.1.0` after the engine extraction | `1.0.0` |
| Example | `pili-pos/1.1.0` (14 chars) | `pili-pos-android/1.0.0` (22 chars) |
| Git tag | `vX.Y.Z` (today), `linux-v1.1.0-rc1` / `linux-v1.1.0` for the freeze tags in the plan | `android-vX.Y.Z` (plan P6) |
| Release artifact | GHCR `:declared` | GHCR `:declared-android`, `"platform":"android"` in `declared.json` |

- **Single source.** The Android version string comes from one place: the
  `pos-ffi` crate version (`CARGO_PKG_VERSION` in `android/rust`), which
  `pos-ffi` passes to `Engine::open` as `SoftwareIdentity`. A Gradle check fails
  the build when `versionName` differs from it. The value is never read from
  `pos.json` or any other configuration, so it cannot be edited on a till.
- **versionCode** = `MAJOR * 10000 + MINOR * 100 + PATCH` (1.0.0 -> 10000,
  1.2.3 -> 10203). It is derived from the tag, never from a CI run number as
  moyun-handheld does: a run number is a build input that would break the
  reproducible APK (S5), and the updater requires versionCode to go up.
  MINOR and PATCH stay below 100.
- **Development builds** carry the suffix `-dev`
  (`pili-pos-android/1.0.0-dev`, at most 29 chars for any 2-digit parts), so a
  development build can never be taken for a declared version on a ticket or
  in an FDM journal.
- **Engine version.** The engine's own version (the Linux workspace version
  of the frozen engine tag) is not part of `posSwVersion`. It is shown on the
  About screen and in the Android build report next to the `base.apk` and
  `libpos_ffi.so` SHA-1 (plan P6).

**Needs from the user:** approve the product-line name `pili-pos-android` and
the versionCode formula.

## 3. deviceId source: PROPOSED, mechanism DECIDED (S4 emulator)

**Facts.**
- Linux: `device_id` is the real MAC address of the default-route interface,
  written to `pos.json` at first boot by `pili-provision`
  (`docs/SUBMISSION_ARTIFACTS/MANUEL_INSTALLATION_FR.md:229`). It is printed on
  every ticket.
- FAQ producteurs Q505 (`docs/20260717_FAQ_PRODUCTEURS_FR.md:461-471`):
  a) in general the MAC address; b) another unique hardware identifier, for
  example a serial number, is allowed.
- Spec: `deviceId` is 1 to 600 characters.
- Android hides the MAC from ordinary apps.

**Proposal (plan, point 11 and P4.4).**
1. The deviceId is the Sunmi hardware serial, read during provisioning by the
   device-owner app with `Build.getSerial()` (READ_PHONE_STATE granted by the
   owner). If that is unavailable, provisioning fails: S4 measured that
   `ro.serialno` is empty inside the app process, so there is no in-app
   fallback (was: fallback `ro.serialno`, see "Measured by S4").
2. It is read once at provisioning and stored in the engine configuration, as
   Linux does with `pos.json`. At every `Engine::open` the host reads it again
   and compares. A mismatch (for example a restored backup on other hardware)
   blocks signing with a coded error and needs a manager re-provision, which
   is logged.
3. If no hardware identifier can be read, provisioning fails. There is no
   generated or random id, and no `ANDROID_ID` (that value depends on the
   signing key and the user, and changes on a factory reset, so it is not a
   hardware identifier in the Q505 sense).
4. Format: the serial as the OS returns it, trimmed, with no prefix.

**Alternative evaluated by S4 and not chosen.** A device owner can call
`DevicePolicyManager.getWifiMacAddress(admin)`. That gives the factory Wi-Fi
MAC, which matches Q505 a) and the Linux practice.

**Measured by S4** (emulator, `docs/android/spikes/S4.md` section 3; this was
the PENDING-S4 list of the draft):
- `Build.getSerial()` works as device owner on API 33 (tested with
  READ_PHONE_STATE granted, not without the grant). It returned
  `EMULATOR37X2X12X0` (17 characters) with Wi-Fi on, with Wi-Fi off and after a
  reboot, so it is stable. Before the owner is set it throws
  `SecurityException`.
- `ro.serialno` and `ro.boot.serialno` are empty inside the app process
  (reflection and `exec getprop` both return an empty string, no exception),
  while `adb shell getprop ro.serialno` shows `EMULATOR37X2X12X0`.
  `Build.SERIAL` is `unknown`. There is no in-app `ro.serialno` fallback.
- The emulator's fake serial, recorded for the dossier: `EMULATOR37X2X12X0`.
- `getWifiMacAddress(admin)` returned `02:15:b2:00:00:00` (a fixed fake) with
  Wi-Fi on and after a reboot, and `null` with Wi-Fi off.

**Outcome of the rule (Q13).** `Build.getSerial()` works for the device owner
and is stable across reboot, so the hardware serial stays the deviceId (Q4
stands). `getWifiMacAddress` is evaluated and not chosen: it is `null` while
Wi-Fi is off (a till on Ethernet, or with its radio off, depends on a state the
installer does not control), the AVD value is a fixed fake, and Q505 b) accepts
a hardware serial.

The real Sunmi serial format, and whether the D3 Pro firmware returns it to the
device owner, stay hardware residue (checked on a real D3 Pro if one becomes
available).

## 4. TLS approach for `HttpTls`: DECIDED (S3)

**Decision: Option B1c.**
- The engine gets `HttpTls::{Platform, RootsOnly(Vec<Vec<u8>>)}` with a single
  `client_builder()` factory.
- `RootsOnly` hands DER roots to reqwest through `tls_certs_only`. An empty
  list is refused at open.
- On Android, `pos-ffi` passes `RootsOnly` with the roots of
  `webpki-root-certs` compiled in (`HttpTlsMode::BundledRoots`).
- Linux/Tauri passes `Platform`, which makes no `tls_*` call, so the Linux code
  path is today's.

**Evidence (S3, reviewed APPROVE, rerun by the reviewer on pili_api33).**
- **Baseline panic confirmed.** A default reqwest client builds on Android and
  panics at the first HTTPS handshake: `Expect rustls-platform-verifier to be
  initialized`, `rustls-platform-verifier-0.7.1/src/android.rs:90`. The panic
  runs on a tokio worker, so it would silently kill the HubRise poller or the
  mail task. S1 hit the same panic from `pos-fiscal`.
- **All B variants reach both services.** B1, B1c, B2 and B2c each return
  `HTTP/2.0 404` from `https://api.hubrise.com` and `HTTP/2.0 200` from
  `https://api.scaleway.com`. Each of them still refuses the four badssl
  negative controls (expired, self-signed, wrong host, untrusted root).
- **Option A also works but is fragile.** A (JNI init of the platform
  verifier, AAR, R8 release APK) works, including a `.so` loaded first by JNA.
  - It fails on a skipped init (panic) or a missing keep rule
    (`failed to call native verifier`).
  - The AAR is only on the GitHub `maven-archive` branch (group `org.rustls`,
    10,295 bytes) and ships no consumer rules.
- **No feature-flag way out.** No reqwest 0.13.5 feature set keeps rustls
  without compiling rustls-platform-verifier. The lever is the runtime builder
  call.
- **No new lock entry.** `webpki-root-certs 1.0.9` is already in the root
  `Cargo.lock`, so B1c adds no crate there.

**Rules that come with it.**
- **Three production call sites.** `HttpTls` must cover:
  - `src-tauri/src/state.rs:98` (`shared_http`);
  - `src-tauri/src/commands/delivery.rs:77` (mail);
  - the `None` fallback of `crates/pos-fiscal/src/fdm_client.rs:427`. An
    https FDM URL without a usable CA file would otherwise reach the platform
    verifier and panic in the middle of a signing call.
- **FDM transports.** `fdm_tls.rs` (mTLS, custom verifier for module2's X.509
  v1 certificates) is unchanged and is proven on Android (S1: `fdm_tls` 6/6,
  `fdm_https_mock` 3/3). Plain `http://` FDM never handshakes.
- **Where `HttpTls` lives.** `pos-fiscal` does not depend on the engine (its
  dependencies are `pos-core` and libraries), so the type cannot live only in
  `pos-engine`. Decision for P1a: define `HttpTls` in `pos-fiscal` (the lowest
  crate that already depends on reqwest; `pos-core` has no reqwest), and have
  `pos-engine` re-export it.
- **Linux stays byte-for-byte on `Platform`.** The Linux build never uses
  `RootsOnly`.
- **Fallback.** Option A stays a documented recipe in S3. If the owner ever
  wants "trust the device store", it can be added later as a third mode
  `HttpTlsMode::PlatformJni`, in `pos-ffi` only.

**Accepted trade-offs.**
- No CRL checking.
- A TLS-inspecting proxy with a private root is not trusted. The answer is an
  extra root supplied in the signed USB bundle (`RootsDir` mode, B2 loader).
- Refreshing the bundled roots means a new APK, so a new declared Android
  version and a filing.
  - HubRise (ISRG Root X1) and Scaleway (USERTrust RSA) chain to long-lived
    roots, so this is expected to be rare.
  - A stale set fails closed with `UnknownIssuer`.

**Open (not blocking):**
- B1c was not run on a real D3 Pro.
- The claim that Android 13 has no Mainline root update path was not verified.

## 5. Pinned versions: DECIDED

All versions below were used together in S2 and S5 (built, run on pili_api33,
and reproduced in a container). The Rust crates shared with Linux match the root
`Cargo.lock` (checked at `96728e1`), as required by the future
`check_lock_parity.sh`.

| Component | Pin | Evidence |
|-----------|-----|----------|
| Rust | 1.99.0 (`rust-toolchain.toml`), target `aarch64-linux-android` | S1, S2, S3, S5 |
| NDK | `27.1.12297006` (r27b), also AGP `ndkVersion` | S1, S2, S5 (sdkmanager sha1 `6fc476b2e57d7c01ac0c95817746b927035b9749`) |
| cargo-ndk | 4.1.2 (`cargo install --locked --version 4.1.2`) | S1, S2, S5 |
| uniffi / uniffi-bindgen | `=0.32.2` in `pos-ffi` and in the bindgen binary, one workspace lockfile | S2 (latest stable on 2026-10-06) |
| JNA | `net.java.dev.jna:jna:5.19.1@aar` (app, 16 KB aligned `libjnidispatch.so`); plain `jna:5.19.1` jar for JVM tests only | S2 |
| AGP | 8.9.1 | S2, S5, moyun-handheld `libs.versions.toml` |
| Kotlin | 2.2.20 | S2, moyun-handheld |
| R8 | 8.13.25, forced through the root `buildscript { classpath(libs.r8) }` (the R8 bundled with AGP 8.9.1 cannot read Kotlin 2.2 metadata) | S2 (`./gradlew buildEnvironment`), moyun-handheld |
| Gradle | 8.11.1 wrapper, `distributionSha256Sum=f397b287023acdba1e9f6fc5ea72d22dd63669d59ed4a289a29b1a76eee151c6` | S2, S5 |
| compileSdk / targetSdk / minSdk | 36 / 36 / 33; ABI `arm64-v8a` only | S2, plan point 10 |
| JDK | 17 (container: Temurin 17.0.20.1+1, sha256 `3808d1d1...e08e`) | S2, S5 |
| Build-tools | 35.0.0 in the reference container (AGP 8.9.1 default) | S5 |
| kotlinx-coroutines | 1.9.0 (as moyun-handheld) | S2 |
| androidx.annotation | 1.9.1 (required by `android_cleaner = true` bindings) | S2 |
| tokio | 1.53.1 | S2, root `Cargo.lock` |
| reqwest / rustls / rustls-platform-verifier | 0.13.5 / 0.23.45 / 0.7.1 | S3, root `Cargo.lock` |
| aws-lc-rs | 1.18.1 (aws-lc-sys 0.45.0, built with the NDK clang, no cmake or bindgen) | S1, S3 |
| webpki-root-certs | 1.0.9 (already in the root `Cargo.lock`) | S3 |
| Reference build image base | `debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587`, apt from snapshot `20261001T000000Z`, rustup-init 1.29.1 (sha256 pinned), cmdline-tools 13114758 (sha256 pinned) | S5 |

**Build profile (from S2).**
- `[profile.android-release]`: `inherits = "release"`, `panic = "unwind"`,
  `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `strip = "none"`,
  `debug = "line-tables-only"`.
- Link argument `-Wl,-z,max-page-size=16384`. Under `RUSTFLAGS` it must be
  repeated, because `RUSTFLAGS` replaces `target.*.rustflags`.
- AGP strips the packaged copy.

**Bumps.** Any later change of a pin is a deliberate commit that also updates
this table. For the APK it means a new declared version.

## 6. Updater trigger policy: DECIDED (policy), mechanics DECIDED (S4 emulator)

**Policy (approved plan, P6; same principle as `deploy/UPDATE-STRATEGY.md`:
"a self-updating terminal ... is ruled out").**
1. **Operator-started only.** The manager enters a PIN in Settings. There is no
   background check, no scheduled install and no push trigger.
2. **Preconditions** checked by the engine (not by Kotlin):
   - the day's Z report is done;
   - no order is open;
   - no sign is in flight.
   The update is refused with a coded error otherwise.
3. **Source.** An anonymous pull of `declared-android` from the public GHCR
   package (`declared.json` with `"platform":"android"`, plus the minisign
   signature). Nothing else is ever installed.
4. **Checks before install:**
   - the minisign signature against the public key compiled into the app;
   - the SHA-1 and SHA-256 of the APK against `declared.json`;
   - versionCode strictly greater than the installed one (no downgrade);
   - the APK signing certificate equal to the installed one.
5. **Install** through a PackageInstaller session as device owner. Before the
   install, the engine makes a consistent DB backup.
6. **Audit.** Every attempt (refused or done) is written to the audit log.
   The About screen then shows the new version and SHA-1.

**PROPOSED addition (needs the user):** a second source with identical checks:
the same signed `declared.json` + APK from a USB stick through SAF. Linux
deploys from a USB stick on site too, and it covers a till without internet.

**Measured by S4** (emulator, `docs/android/spikes/S4.md` section 7; this was
the PENDING-S4 list of the draft):
- **Silent install.** A PackageInstaller session as device owner, while in Lock
  Task, returns `STATUS_SUCCESS` with no installer UI: only our activity was on
  top in 57 of 57 samples over 30.2 s. The app is back on top and LOCKED 0.60 s
  after the commit. The device owner, the Lock Task packages, the persistent
  HOME and the database all survive.
- **Restart path.** The install force-stops the app. The system starts HOME
  (our persistent preferred activity) and `onResume` re-locks from the persisted
  kiosk flag: 10 of 10 trials back at +0.67 to +0.75 s with only our task on the
  device. With a stock launcher task below ours the app never came back by
  itself (16.6 s observed); the `MY_PACKAGE_REPLACED` receiver that starts the
  main activity brought it back at +1.35 s, so the receiver stays in P6. The
  install status arrives in the NEW process, which therefore writes the audit
  "done" row.
- **Unknown sources.** `DISALLOW_INSTALL_UNKNOWN_SOURCES` and
  `DISALLOW_INSTALL_UNKNOWN_SOURCES_GLOBALLY` do not stop the owner's session,
  and do not stop `adb install` either (userdebug image).
- **OS refusals (second line behind the app checks).** Downgrade:
  `INSTALL_FAILED_VERSION_DOWNGRADE` (`EXTRA_STATUS=4`, legacy -25). Other
  signing key: `INSTALL_FAILED_UPDATE_INCOMPATIBLE` (`EXTRA_STATUS=5`, legacy
  -7). A same-versionCode reinstall is **accepted** by the OS, so "strictly
  greater" is checked by the app only.
- **Escape window.** The final run saw nothing but our activity. Earlier runs
  saw the stock launcher once, a black frame of about 0.5 s and, once, the stock
  home screen for about 2 s; with a stock launcher task below ours the home
  screen with a Settings icon stays visible until the receiver fires.
- **`REQUEST_INSTALL_PACKAGES`.** Not needed: an update installed silently
  without the permission declared. It stays out of the production manifest.

Hardware residue: whether the Sunmi PackageInstaller behaves as AOSP, the
install time on the real SoC, and `adb install -d` on a user build.

Rollback is not offered by the app: Android refuses a downgrade without an
uninstall, and an uninstall deletes the fiscal DB. A faulty declared version is
fixed by a newer declared version.

## 7. No remote support in v1: DECIDED

**Source:** the approved plan (`support_program: None` on Android, "no
RustDesk").

**Consequences, to be written into the dossier variant (P8):**
- The Android engine is opened with `support_program = None`. The
  support-session command returns a coded "not available" error, and the UI
  hides the entry.
- No listening port and no permanent outbound channel other than FDM, HubRise,
  TEM mail and the operator-started update pull.
- adb is off on a provisioned till (`DISALLOW_DEBUGGING_FEATURES`, the last
  provisioning step, P6). The measured restriction set and the adb-off behaviour
  (adb goes offline within 4 s; only the app can switch it back on, with
  `dpm.setGlobalSetting(ADB_ENABLED, "1")`, which works on API 33) are in
  `docs/android/spikes/S4.md` section 8 and Q16.
- Support is on site:
  - the manager-PIN Settings screens;
  - the signed USB provisioning bundle (configuration, FDM token, mTLS
    certificates);
  - SAF export of logs and backups to a USB stick.
- Adding remote support later is a new Android version and a dossier change.

## 8. Admission FDM = module2 online simulator: DECIDED, with open test-machine questions

**Decision.** This was decided for Linux on 2026-09-27 and is reused here.
- The dossier names module2 as the production FDM.
- The module2 online simulator (`FDM02000425`, endpoint
  `http://fdm.module2.be/graphql/` or `https://...:443`) is delivered with the
  test machine instead of a physical FDM (manual §IV, step 1).

**Android specifics.**
- **Transport.** Port 80 is plain HTTP, which `HttpTls` does not touch. Port
  443 goes through `fdm_tls` with module2's CA (X.509 v1, custom verifier),
  proven on Android against the mock (S1). Neither path uses the bundled web
  roots.
- **Token and certificates.** The bearer token and any mTLS material travel
  only in the minisign-signed USB provisioning bundle into app-private storage
  (P4.4). They are never in the APK, the repo or logs. Development keeps it in
  the gitignored `.env.live`.
- **Live runs.** Every live run against module2 needs the user's explicit
  go-ahead (plan), including from the emulator. P0 made no live call.

**PROPOSED (needs the user):**
- **Which identity.** Which posId and terminalId the Android test machine uses
  on the simulator. The simulator allows `CPOS0031234567` (official
  UC_TestSetup identity) with one free slot. Proposal: same posId, its own
  `terminalId` (for example `POS-2-ANDROID`).
- **Which machine.** What is handed over as the "test machine" for the Android
  filing: a real D3 Pro (needs hardware) or the emulated profile (the plan
  allows the dossier to say tests ran on an emulated profile).

## 9. Signing-key custody: PROPOSED (technical constraints DECIDED)

**Constraints DECIDED by S5 (reviewed APPROVE).**
- **The key must be RSA.** apksig v2/v3 signatures with RSA (PKCS#1 v1.5) are
  deterministic: re-signing the same APK twice gave the same SHA-1. With ECDSA
  (secp256r1) they gave two different SHA-1, because of the random nonce. If
  the SHA-1 of the signed APK is the value of record, the key must be RSA.
  - Only RSA-2048 was measured. A larger RSA key should behave the same, but
    `spikes/s5/probe_signing.sh` must be rerun with the chosen size before the
    key is created.
- **Signature schemes:** v1 off, v2 and v3 on. No v3 key-rotation lineage.
  minSdk stays fixed at 33, because it changes which blocks apksigner checks.
- **The key never enters the build container.**
  - The container builds the unsigned, zipaligned release APK, and
    `scripts/android/check_repro.sh` compares two unsigned builds byte for
    byte, plus the entry manifest.
  - Signing happens afterwards with the pinned build-tools `apksigner`.
  - The certificate SHA-256 digest is then pinned in this file, in the
    dossier and in the updater.

**Why custody is critical.** Android only accepts an update signed with the
same key.
- **Losing the key** means no installed till can ever be updated without an
  uninstall, and an uninstall deletes the fiscal DB.
- **Leaking the key** lets anyone build an APK that installs over a till. The
  minisign check in the updater still blocks undeclared versions in the app's
  own update path, but not adb or other paths (adb is disabled by device
  owner, see 7).

**Proposal (the plan reserves the creation and holder choice to the user).**
- **Who and when.** The producer (Bruno Moyse) creates the key personally,
  offline, after this file is approved and before the first Android tag. The
  plan requires an explicit go-ahead for this step.
- **Key parameters:**
  - RSA 4096 (subject to the probe rerun), validity 30 years;
  - PKCS12 keystore with a strong passphrase kept in a password manager.
- **Storage:**
  - the keystore lives on an encrypted offline medium;
  - two encrypted backups in two separate physical places.
- **Never** in the repo, in CI secrets, on GHCR or on a till.
- **Separate from minisign.** The key is separate from the minisign
  declaration key (`declare.sh`). Proposal: reuse the existing offline minisign
  key for `declared-android`, so one declaration identity covers both
  platforms. The alternative is a second minisign key; the user picks.
- **Signing step.** `declare.sh --android` (P6) does the signing on the
  producer's machine only.

## 10. When to file Linux v1.1: PROPOSED

**Facts.**
- The engine extraction (P1) changes the Linux binary, so it ships as Linux
  v1.1 under AM Art. 113 with its own SHA-1.
- The plan freezes it softly at the P1b exit (`linux-v1.1.0-rc1`) and hard at
  the P3 exit (`linux-v1.1.0`).
- A Linux update can be filed at most 12 times a year.
- P8 coordinates the Android filing with v1.1 "so one engine version backs
  both".
- **Baseline vs cert branch.** The P0.1 Linux baseline SHA-1
  `83313be28027d2e8d9e84cbddff4e584799c2fbc` is that of main `96728e1`. The
  branch `cert/v1.0` exists and main is 47 commits ahead of it.

**Options.**
- **A. File v1.1 at the P3 hard freeze, on its own.** It gets the Linux update
  out of the way early. But any engine bug found during P4 to P7 then becomes
  a Linux v1.2 filing.
- **B. Freeze at P3, file v1.1 together with Android at P8 (recommended).**
  - Until it is filed, a re-freeze for an engine bug found in P4 to P7 costs no
    SPF filing, only a new rc tag and a new check.
  - Both filings cite the same engine tag.
  - It uses one filing slot for the pair.
- **C. If v1.0 has not been filed yet, skip v1.1.** File the extracted engine
  directly as the first Linux version, which avoids an Art. 113 update
  altogether. This contradicts the plan's precondition that v1.0 stays frozen
  on `cert/v1.0`, so it is only an option if the user confirms v1.0 is not
  filed.

**Needs from the user:**
- Has v1.0 been filed with the SPF, and which SHA-1 is its value of record
  (`cert/v1.0` or main)?
- Choose A, B or C, plus the deadline and freeze cadence the plan asks for.

## 11. Other decisions carried from P0 evidence: DECIDED (technical)

These are not on the P0.7 list but bind later phases. The P1 to P3 planners
copy them into their task cards.

**Gates**
1. **Test baseline.** The baseline is the measured inventory: `ws` 1355
   (1342 passed + 13 ignored), `cert` 491, 1846 lines, sha256
   `586834f05390ed4535e34b22747d2a34c0f003542676684e0de8d8b0ff7f99a2`. The
   plan's "1,356" (plan lines 96 and 333) has no source. The gate is
   `scripts/android/test_inventory.sh --diff` (no baseline line removed)
   together with `ws >= 1355`.
2. **FDM journal parity.** P1 compares the normalised journals (sha256
   `68a09ee3...`). It also compares `eventCounter` and `totalCounter` from the
   raw journals: they were identical across the two runs and are fiscal
   contract data.

**Builds**
3. **No concurrent builds in one tree.** `scripts/check_repro.sh` must never
   run at the same time as another build in the same tree. It runs
   `rm -rf ui/dist`, which is the likely cause of the one-off `ui/dist` loss in
   P0.1.
4. **rustup cargo first on PATH.** Homebrew's cargo/rustc shadow rustup on the
   dev Mac and have no Android std (E0463). Every script, Gradle Exec task and
   CI step puts `~/.cargo/bin` first or calls `rustup run 1.99.0 cargo`.

**UniFFI, R8 and the panic path**
5. **UniFFI load check.** `EngineGateway` calls `uniffiEnsureInitialized()`
   once at open. Without it a mismatched `.so` crashed the JVM (SIGSEGV).
   Keep a JVM test that loads a deliberately mismatched library.
6. **Bindings source.** Bindings are generated in library mode from the
   unstripped cargo output (or a `--strip-debug` copy), never from the `.so`
   inside the APK. The `--config` file uses `[crates.<name>.bindings.kotlin]`;
   the old flat format is silently ignored.
7. **R8 consumer rules of `:pos-core`.** S2 showed, by removing them one at a
   time, that these three are needed:
   - `-dontwarn java.awt.*`
   - `-keep class com.sun.jna.** { *; }`
   - `-keep class uniffi.<crate>.** { *; }`, as a full keep (members-only
     fails), naming every generated package.

   Also kept: `-keep interface * extends com.sun.jna.Library`. S3 needed it
   for a hand-written JNA interface (without it, `ClassCastException`). For
   generated code the uniffi keep already covers it, so it is kept as a
   safety net.
8. **Panics.** Release builds use `panic = "unwind"`. A panic reaches Kotlin as
   `InternalException`, a separate channel from `PosException`, and the
   gateway marks the engine Fatal.
   - A panic inside a spawned task shows up only as a `JoinError`, which
     `pos-ffi` maps itself.
   - `pos-ffi` installs a panic hook that writes to logcat and the daily log
     file, because stderr is not forwarded on Android.
9. **Cancellation.** All engine work is spawned on the engine runtime and
   awaited through the JoinHandle. A cancelled coroutine detaches the task
   rather than aborting it, so a sign in flight completes (S2 proof). The
   engine must make the effect recoverable (boot recovery, idempotent
   reads).
10. **Runtime drop.** The runtime is held in an `Option` and dropped with
    `shutdown_background()`, because dropping a tokio Runtime inside an async
    context panics.

**Engine paths and tests on Android**
11. **Temp dir.** `std::env::temp_dir()` is `/data/local/tmp` on Android when
    `TMPDIR` is unset, and an app cannot write there. The engine takes every
    path from `EnginePaths` (app-private `filesDir`/`cacheDir`). P1 confirms
    that no production path calls `temp_dir()`; the uses found are in
    `#[cfg(test)]` code.
12. **Fixture paths in device tests.** The 35 `pos-fiscal` tests that read
    fixtures through `env!("CARGO_MANIFEST_DIR")` cannot run on a device as
    built. P3.3 runs the engine subset that reads no host fixture path, or adds
    a fixture-root override to the test helpers. With the fixtures redirected,
    S1 got 250 passed, 0 failed, 2 ignored. The certification evidence keeps
    coming from host runs.

**Reproducible APK (S5)**
13. **P3 reproducibility work:**
    - keep `--remap-path-prefix` for source, `CARGO_HOME`, `RUSTUP_HOME` and
      the target dir. Without it, 16 `/opt/cargo` strings stay in the stripped
      `.so`;
    - add `SOURCE_DATE_EPOCH` from the commit time and `CARGO_INCREMENTAL=0`;
    - set `vcsInfo` and `dependenciesInfo` off;
    - generate and commit Gradle `verification-metadata.xml` (sha256);
    - pre-fetch dependencies and run the build step with `--network none`;
    - push the image and record its digest next to every APK SHA-1;
    - run the exit check once on a native x86_64 Linux host.
14. **16 KB pages.** Every LOAD segment is aligned to `0x4000`, and
    `zipalign -c -P 16 -v 4` must pass on every release APK.

## 12. S4 results (was: items waiting for S4, PENDING-S4)

S4 ran on `sunmi_d3_pro_api33` on 2026-10-07 (`docs/android/spikes/S4.md`). None
of these items blocked P1a, P1b, P2 or P3. The emulator results unblock P4 and
P6; the hardware residue below is checked on a real D3 Pro before filing.

| Item | S4 result (emulator) | Status | Blocks |
|------|----------------------|--------|--------|
| deviceId mechanism | `Build.getSerial()` works as device owner and is stable (`EMULATOR37X2X12X0`); `ro.serialno` is empty in the app; `getWifiMacAddress` is `null` with Wi-Fi off and was not chosen (S4.md section 3, Q13) | DECIDED (S4 emulator); real serial format HARDWARE RESIDUE | §3, P4.4 (mechanism unblocked) |
| Device owner through `adb shell dpm set-device-owner`; refusals | set owner works; second user refused with `Not allowed to set the device owner because there are already several users on the device.`; the account refusal was NOT RUN (no account on the image); the adb exit code is 0, the text is the signal (S4.md section 1) | DECIDED (S4 emulator, adb path); account refusal HARDWARE RESIDUE | P4.4 (unblocked) |
| Lock Task entry and exit, recovery path | entry, exit, HOME, recents, back and Settings launch behave; crash and reboot recovery in 1 to 3 s; features reset to GLOBAL_ACTIONS at every boot; a second crash freezes the app unless `no_system_error_dialogs`; a stock launcher task blocks recovery (S4.md section 2) | DECIDED (S4 emulator); Sunmi launcher HARDWARE RESIDUE | P4.5 (unblocked) |
| `DISALLOW_*` set, time lock, Europe/Brussels | the 12 restrictions, uninstall block, time policy, stay awake, keyguard, status bar and update policy are accepted and enforced; adb off works and comes back through the app (S4.md section 8, Q16) | DECIDED (S4 emulator); safe-boot volume-key route and recovery menu HARDWARE RESIDUE | §6, §7, P6 (unblocked) |
| Silent PackageInstaller self-update, restart after install | silent session install in Lock Task, back and LOCKED at +0.60 s; OS refuses downgrade and other key but not a same-versionCode reinstall (S4.md section 7, Q18) | DECIDED (S4 emulator); real image HARDWARE RESIDUE | §6, P6 (unblocked) |
| SAF export to the emulated SD card (stand-in for USB) | works in Lock Task with a persisted tree grant; the volume root is refused, a sub-folder is needed; survives unmount, remount and reboot; the AVD SD card could not attach, a virtual SD disk stood in (S4.md section 4, Q17) | DECIDED (S4 emulator); real USB stick HARDWARE RESIDUE | P6 SPF export, §6 USB source (unblocked) |
| Printing to `pos-printer-mock` at `10.0.2.2:9100` with DLE EOT status | six states read correctly, typed error when unplugged, no main-thread call; the 500 ms DLE EOT read is unreliable on the emulator path and hides a late answer; Chinese prints as `?` (S4.md section 5) | DECIDED (S4 emulator); DLE EOT timeout and Chinese ticket OPEN (P6 decisions); real Epson HARDWARE RESIDUE | P5, P7 |
| Layout density at 1920x1080 / 160 dpi, CJK font | density 1.0, usable height 996 px unless immersive, 439 of 439 code points covered, Han variant follows the locale tag (S4.md section 6, Q19) | DECIDED (S4 emulator); real display facts HARDWARE RESIDUE | P4.1, P5 |
| 24 h process survival with the foreground service | replaced in S4 by a 30 minute soak, all must-PASS items passed; typed `specialUse` start works on API 33; the 24 h run moves to P7 (S4.md section 9, Q14, Q15) | DECIDED (S4 emulator, 30 minute soak); 24 h run and deep sleep HARDWARE RESIDUE | P4.2 (unblocked), P7 |
| Power cut recovery | `adb emu kill`: LOCKED 3 s after boot, service and beats back, integrity ok, no committed row lost (S4.md section 10) | DECIDED (S4 emulator); real plug pull HARDWARE RESIDUE | P7 |
| bcrypt cost 12 | 295 ms on the emulator, 232 ms on the host (S4.md section 11) | HARDWARE RESIDUE | P7 |

**Hardware residue** (needs a real D3 Pro, otherwise the dossier states an
emulated profile). The list of the draft, extended with the S4 findings:
- the real Sunmi serial, and whether `Build.getSerial()` returns it to the
  device owner on the firmware;
- Wi-Fi and Ethernet MAC behaviour (informational, the serial is the deviceId);
- the SUNMI kiosk extension, and any vendor keep-alive or auto-start whitelist;
- the recovery key combination, and what the recovery menu can wipe or sideload
  with the restrictions on (`DISALLOW_FACTORY_RESET` only blocks the Settings
  entry); `DISALLOW_SAFE_BOOT` through the volume-key route;
- the setup wizard, QR provisioning, and an account blocking `set-device-owner`;
- the stock Sunmi launcher and system apps: crash recovery, the update gap with
  a stock launcher task below ours, crash and ANR dialogs;
- USB OTG: a real stick (FAT32 and exFAT, the 4 GiB FAT32 file limit, auto-mount,
  removal during a write, throughput, DocumentsUI on the Sunmi image);
- bcrypt timing on the Qualcomm CPU;
- the real Epson TM-m30III: DLE EOT answer latency, real paper, cover and error
  states, a Chinese line;
- B1c and the whole stack on the real image (including whether
  `/system/etc/security/cacerts` exists, relevant only to the optional
  `RootsDir` mode);
- system OTA behaviour under the system update policy;
- display facts (`wm density`, bar sizes in Lock Task, fonts, touch comfort) and
  the power key behaviour in Lock Task;
- deep sleep of the SoC, thermal behaviour and the 24 h run, a real plug pull
  and the boot time to LOCKED, `BOOT_COMPLETED` delivery;
- adb behaviour on a `user` build (`adb install -d`, `adb root`, developer
  options).

## Decided while the user was away (review)

The user asked on 2026-10-06 to chain all phases without stopping. These answers to the P0 exit questions follow the planner's recommendations and are open for the user's review.

- **Q1 Exit timing:** P0 approved with S4 deferred; S4 runs on the `sunmi_d3_pro_api33` emulator right after P0, before P4.
- **Q2 Application id:** `dev.nuagemagique.pilipos` (debug `dev.nuagemagique.pilipos.debug`).
- **Q3 Version scheme:** `pili-pos-android/X.Y.Z` from 1.0.0, versionCode `MAJOR*10000 + MINOR*100 + PATCH`, `-dev` suffix for development builds.
- **Q4 deviceId:** Sunmi serial as device owner, read at provisioning and checked at every open, no generated fallback. S4 also evaluates the device-owner Wi-Fi MAC (`getWifiMacAddress`) and records which one the dossier names.
- **Q5 TLS:** bundled roots (B1c) accepted; a root refresh ships as a new declared APK.
- **Q6 Linux v1.1:** option B, filed together with Android at P8. Whether v1.0 was filed from `cert/v1.0` or main stays for the user.
- **Q7 Signing key:** constraints as proposed (RSA 4096, 30 years, offline, two encrypted backups, never in CI). Creation, holder and whether `declared-android` reuses the minisign key stay with the user. Until then only a throwaway dev keystore is used and never committed.
- **Q8 Updater sources:** the USB stick through SAF is accepted as a second source, with the same signature and hash checks.
- **Q9 Spike evidence:** spike branches kept locally and tagged `android/p0-spike-S1`, `-S2`, `-S3`, `-S5` (local tags, not pushed).
- **Q10 Admission:** proposed posId `CPOS0031234567` with terminalId `POS-2-ANDROID` on the module2 simulator; no live call without the user. The SPF test machine is the emulated profile unless a real D3 Pro becomes available.
- **Q11 Hardware:** unknown; hardware residue tracked in S4.
- **Q12 Docker cache:** workers remove only images and caches they created; no global prune.
- **S4 spike branch (wave 1 integration):** S4-T1 and S4-T2 only add files under `spikes/s4/`, and the phase gate forbids any `spikes/` path on `android/int` (spike code is never merged, only reports). They were therefore merged with `--no-ff` into a separate local branch `android/s4-spike` (created from 0b68587), not into `android/int`. `android/int` stays at 0b68587 plus docs only; later S4 tasks merge into `android/s4-spike` too, and the S4 report lands on `android/int` as `docs/android/spikes/S4.md`. Local tag `android/p0-spike-S4` to follow when the spike closes (not pushed).
- **S4 spike branch (wave 3 integration):** S4-T4 (SAF export, picker in and out of Lock Task) only adds files under `spikes/s4/`, so by the same rule it was merged with `--no-ff` into `android/s4-spike` (merge af50e8d, no conflicts), not into `android/int`. `android/int` carries docs only. The gate checks on `docs/android/spikes/S4.md` and the S4-T7 evidence are expected to stay red until the later waves (T5 to T7 and the report) land.
- **S4 spike branch (wave 4 integration):** S4-T5 (silent self-update as device owner in Lock Task) only adds files under `spikes/s4/`, so by the same rule it was merged with `--no-ff` into `android/s4-spike` (merge 9b9c227, no conflicts), not into `android/int`. A first merge into `android/int` was reverted before any gate run (hard reset to 7b92a9a, nothing pushed). `android/int` still carries docs only. The `docs/android/spikes/S4.md` and S4-T7 evidence gate checks stay red until the remaining waves (T6, T7 and the report) land.
- **S4 spike branch (wave 5 integration):** S4-T6 (device-owner restrictions, time lock, stay awake, boot into the kiosk, adb off) only adds files under `spikes/s4/`, so by the same rule it was merged with `--no-ff` into `android/s4-spike` (merge d68a708, no conflicts), not into `android/int`. A first merge into `android/int` was reverted before any gate run (hard reset to 67921ad, nothing pushed). `android/int` still carries docs only. The `docs/android/spikes/S4.md` and S4-T7 evidence gate checks stay red until the remaining waves (T7 and the report) land.
- **S4 spike branch (wave 6 integration):** S4-T7 (foreground service soak, power cut and teardown scripts) only adds files under `spikes/s4/`, so by the same rule it was merged with `--no-ff` into `android/s4-spike` (merge e1c1101, no conflicts), not into `android/int`. `android/int` still carries docs only. The S4-T7 notes and the `TEARDOWN | PASS` evidence line now exist on `android/s4-spike`. The `docs/android/spikes/S4.md` gate check stays red until the S4 report lands.
- **Q13 deviceId outcome (S4.md section 3):** the S4 rule gives case (1). `Build.getSerial()` works for the device owner on API 33 and is stable across reboot and with Wi-Fi off (emulator value `EMULATOR37X2X12X0`, 17 characters), so the hardware serial stays the deviceId and Q4 stands. `getWifiMacAddress` is evaluated and not chosen (`null` with Wi-Fi off, fixed fake `02:15:b2:00:00:00` on the AVD). `ro.serialno` is empty in the app process, so there is no in-app fallback: provisioning fails when `getSerial` throws. The real Sunmi serial format stays hardware residue.
- **Q14 Survival test (S4.md section 9):** the 24 h process survival test is replaced in S4 by a 30 minute soak, on the user's request during the run. All must-PASS items passed (31 samples, one pid, LOCKED 31 of 31, 179 beats, largest gap 10.0 s, PSS -2.1 percent, integrity ok). The 24 h run moves to the P7 runbook on the real D3 Pro: `SOAK_MINUTES=1440 caffeinate -i spikes/s4/run_t7.sh` (adb serial in place of the emulator, a real plug pull).
- **Q15 Foreground service start on API 33 (S4.md section 9.1):** P4.2 uses the typed call `ServiceCompat.startForeground(..., FOREGROUND_SERVICE_TYPE_SPECIAL_USE)` with `foregroundServiceType="specialUse"`, the `PROPERTY_SPECIAL_USE_FGS_SUBTYPE` property and the permissions in the manifest, and the plain `startForeground(id, notification)` as the try/catch fallback (both worked, neither threw). `POST_NOTIFICATIONS` is granted by the device owner. The service starts from `onResume` after the Lock Task relock; the `BOOT_COMPLETED` receiver is optional because it was not observed after the power cut.
- **Q16 P6 restriction set (S4.md section 8):** P6 sets the 12 measured restrictions (`no_factory_reset`, `no_safe_boot`, `no_uninstall_apps`, `no_install_unknown_sources`, `no_install_unknown_sources_globally`, `no_config_date_time`, `no_add_user`, `no_user_switch`, `no_remove_user`, `no_modify_accounts`, `no_usb_file_transfer`, `no_network_reset`), plus `no_system_error_dialogs`, `setUninstallBlocked`, auto time on, auto time zone off, `Europe/Brussels`, `STAY_ON_WHILE_PLUGGED_IN=7`, keyguard and status bar disabled and the postpone update policy. `DISALLOW_DEBUGGING_FEATURES` is the last provisioning step. `setLockTaskFeatures(NONE)` is called at every start because it reverts to GLOBAL_ACTIONS at every boot. Stays off: `DISALLOW_MOUNT_PHYSICAL_MEDIA` (the USB export needs the volume mounted). `DISALLOW_CONFIG_WIFI` stays open (off): inside Lock Task with features NONE no Settings screen is reachable, and the manager fixes a Wi-Fi password through a PIN-gated screen in the app without reprovisioning. No timed adb in v1 unless the dossier accepts it.
- **Q17 SAF export flow (S4.md section 4):** the manager picks a folder once per stick from a PIN-gated screen with no open order. Pre-check `StorageManager.storageVolumes` for a mounted removable non-primary volume and show "Insert the USB stick" otherwise. Open the picker with `StorageVolume.createOpenDocumentTreeIntent()` from the locked activity, never with `FLAG_ACTIVITY_NEW_TASK`; DocumentsUI stays off the Lock Task allowlist. The volume root is refused, so the screen tells the manager to create a sub-folder (suggested `PILI-EXPORT`). Persist the tree grant; export without a picker, write the SHA-256 manifest last, delete nothing on error, and report a missing stick from the pre-check.
- **Q18 Update restart path and prechecks (S4.md section 7):** the app keeps three ways back into Lock Task (persistent preferred HOME, a `MY_PACKAGE_REPLACED` receiver that starts the main activity, relock in `onResume` from the persisted kiosk flag). The updater checks, before any session: Z done, no open order, no sign in flight, minisign, SHA-256 and SHA-1, package name, versionCode strictly greater (the OS accepts a same-versionCode reinstall), same signer; then a DB backup and an fsynced marker, then `commit`. The audit "done" row is written in the new process. `STATUS_PENDING_USER_ACTION` is an error. `REQUEST_INSTALL_PACKAGES` is not declared, and production uses HTTPS only.
- **Q19 CJK font and dp scale (S4.md sections 5 and 6):** bundle Noto Sans SC (Regular and Bold, subset) as app resources with the system font as fallback, and tag every Chinese run `zh-Hans` and French `fr` per `Text`. 1 dp equals 1 CSS px of the Vue design (the till reports density 1.0 at 160 dpi), with a `LocalDensity` wrapper `Density(widthPx / 1920f, 1f)` and `fontScale` 1.0, and immersive mode on every screen (the usable height is 996 px otherwise). Whether Chinese appears on the printed ticket (it prints as `?` through CP858) stays a P6 product decision.
- **Q20 AVD state (S4.md, State left and cleanup):** S4 left the `sunmi_d3_pro_api33` AVD without a device owner and without the spike app (`TEARDOWN | PASS`, only user 0, no factory reset). It keeps the virtual SD disk from S4-T4. P4 provisions from that clean state, and the spike code stays on the local tag `android/s4-spike`.
- **S4 spike branch (wave 7 integration):** S4-T8 (the S4 report and the settled PENDING-S4 decisions) carries the whole S4 spike history, so by the same rule it was merged with `--no-ff` into `android/s4-spike` (merge 80a25eb, no conflicts), not into `android/int`. Only its two documentation files reach `android/int`: `docs/android/spikes/S4.md` as is, and `docs/android/DECISIONS.md` by a three-way merge against 0b68587 that keeps the wave 1 to 6 records above. No `spikes/` path is on `android/int`; the spike code and the S4-T7 evidence stay on `android/s4-spike`.
- **S4 exit (planner, 2026-10-07):**
  - *Spike history.* It stays on the local branch `android/s4-spike` (80a25eb) and, following the P0 convention and the wave 1 note, on the local annotated tag `android/p0-spike-S4` at the same commit. No tag named `android/s4-spike` was created, because it would make that name ambiguous with the branch; the S4.md references to `android/s4-spike` resolve through the branch. Nothing is pushed.
  - *Merge procedure deviations.* The phase cards `docs/android/phases/S4.md` were committed at the exit (02860ee), not before wave 1; their content is unchanged. The two S4-T8 report commits landed as one commit (2daabdc) with a three-way merge of this file, because `android/int` already carried the wave records: `docs/android/spikes/S4.md` is byte-identical to the S4-T8 branch, and this file differs from it only by the wave 7 record.
  - *Review record.* There is no separate `S4-review.md`; the eight card reviews (all APPROVE) are summarised in `docs/android/phases/S4-exit.md`.
  - *Freeze.* S4 has no freeze step. The plan's first freeze is the soft freeze `linux-v1.1.0-rc1` at the P1b exit, which needs the user's go-ahead, so no Linux tag exists and `docs/android/LINUX_FREEZE.md` is not created yet. `scripts/check_repro.sh` was not rerun: no Linux build input changed, and the test inventory is byte-identical to the baseline (sha256 `586834f0...`).
  - *DLE EOT timeout.* `pos-print` returns an optimistic "ready" when the DLE EOT answer misses its 500 ms read (S4.md section 5). The engine change (treat a missing answer as unknown, longer timeout) is a Linux behaviour change in a shared crate: it is offered to the P1a/P1b planner for the window before the soft freeze, and goes to the user as a question. Until it lands, P5 and P6 treat a missing answer as "printer state unknown" at the app level.
  - *Escape window after a silent update.* The S4-T5 reviewer rerun caught one frame of about 0.9 s showing the stock launcher's taskbar with tappable icons in the single-task case. P4.5 therefore hides or suspends the stock launcher and Settings through the device owner (`setApplicationHidden` or `setPackagesSuspended`) on every till, not only when a stock launcher task exists.
  - *Q14 wording.* The 30 minute soak was the user's request, recorded in the phase cards (PD2). The S4-T7 notes call it a planner decision; the S4.md wording stands.
  - *AVD hygiene.* A dangling `sdcard.img.qcow2` overlay (its backing file deleted) left in the AVD folder by an S4-T4 attempt was moved out of the AVD folder; the AVD is otherwise as described in Q20.
