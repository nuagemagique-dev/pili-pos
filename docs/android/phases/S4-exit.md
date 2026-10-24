# S4 exit report: D3 Pro platform spike on the emulator

> Correction (2026-10-10): the Linux build is not filed as v1.1 under Art. 113. Nothing was filed, so Linux and Android are both version 1.0.0 initial filings (Art. 99), and the freeze tag is `linux-v1.0.0-rc1`, not `linux-v1.1.0-rc1` (`docs/android/DECISIONS.md`, "Decided by the user"). The report below is unchanged.

Written by the S4 planner on 2026-10-07, at the end of the S4 run.
- **Base:** `android/int` at `0b68587` (`docs(android): P0 exit report, phase cards and decisions`).
- **Integration branch:** `android/int`, docs only. main was not touched.
- **Spike branch:** `android/s4-spike` at `80a25eb`, also tagged locally as
  `android/p0-spike-S4` (annotated, same commit). Nothing was pushed.
- **Cards:** `docs/android/phases/S4.md`. Report: `docs/android/spikes/S4.md`.
  Decisions: `docs/android/DECISIONS.md` (Q13 to Q20, the wave records and
  the "S4 exit" bullet under "Decided while the user was away (review)").
- **Exit rule (cards):** P4 planning may start once `android/int` carries
  S4.md and the updated DECISIONS.md, unless the deviceId item is OPEN (PD4
  rule 3). The user reviews the "Decided while the user was away" bullets
  when back.

## Verdict

- **All eight cards are done and APPROVE.** Each reviewer reran the card's
  gates and the run script on the AVD (S4-T7 with `SOAK_MINUTES=5`) and
  reproduced the worker's must-PASS results. S4-T2 went through one rework
  round (QR URL length and VAT consistency of the sample ticket, `1cd85fb`).
  The results do not report a rework round for the other cards.
- **No item failed on the emulator.** Every item of the plan's S4 list ran,
  except the 24 h survival test, which the user replaced by a 30 minute soak
  (PD2). The 24 h run moves to the P7 runbook.
- **The phase gate passes on `android/int`** (all checks below), rerun by the
  planner at the exit.
- **deviceId (PD4): rule 1 applies.** `Build.getSerial()` works for the device
  owner on API 33 and is stable across reboot and with Wi-Fi off. The hardware
  serial stays the deviceId (Q4 stands, Q13). P4.4 is not blocked.
- **`android/int` is a valid start point for the next phase.** It differs from
  `0b68587` only in four files under `docs/android/`.

**Planner recommendation:** go on with P1a (engine extraction), which needs
nothing from S4. P4 planning may use S4.md and DECISIONS §12 as they stand.
The hardware residue stays open until a real D3 Pro is available.

## Task status

| Card | Wave | Branch tip | Review | Integrated as |
|------|------|------------|--------|---------------|
| S4-T1 kiosk base app, device owner, Lock Task, deviceId | 1 | `android/S4-S4-T1` `86ce333` | APPROVE | `android/s4-spike` merge `8a97083` |
| S4-T2 Rust probe library (host only) | 1 | `android/S4-S4-T2` `1cd85fb` | APPROVE (after 1 rework) | `android/s4-spike` merge `d2f08a8` |
| S4-T3 printing with DLE EOT, display, CJK, bcrypt | 2 | `android/S4-S4-T3` `912f919` | APPROVE | `android/s4-spike` merge `ca30bdd` |
| S4-T4 SAF export to a removable volume | 3 | `android/S4-S4-T4` `adfa14a` | APPROVE | `android/s4-spike` merge `af50e8d` |
| S4-T5 silent self-update as device owner | 4 | `android/S4-S4-T5` `a71467b` | APPROVE | `android/s4-spike` merge `9b9c227` |
| S4-T6 restrictions, time lock, boot, adb off | 5 | `android/S4-S4-T6` `011ed99` | APPROVE | `android/s4-spike` merge `d68a708` |
| S4-T7 30 minute soak, power cut, teardown | 6 | `android/S4-S4-T7` `246416b` | APPROVE | `android/s4-spike` merge `e1c1101` |
| S4-T8 report and the PENDING-S4 decisions | 7 | `android/S4-S4-T8` `acee008` | APPROVE | `android/s4-spike` merge `80a25eb`; docs on `android/int` as `2daabdc` |

**Commits on `android/int` since `0b68587`:**

| Commit | Subject | Content |
|--------|---------|---------|
| `d6320a5` | `docs(android): record S4 spike branch handling` | wave 1 record |
| `7b92a9a` | `docs(android): record S4 wave 3 spike branch handling` | wave 3 record |
| `67921ad` | `docs(android): record S4 wave 4 spike branch handling` | wave 4 record |
| `742e519` | `docs(android): record S4 wave 5 spike branch handling` | wave 5 record |
| `2e41eb4` | `docs(android): record S4 wave 6 spike branch handling` | wave 6 record |
| `2daabdc` | `docs(android): add the S4 report and settle the PENDING-S4 decisions` | S4.md, DECISIONS.md (Q13 to Q20, §3, §6, §7, §12), wave 7 record |
| `02860ee` | `docs(android): S4 phase cards` | `docs/android/phases/S4.md` |
| `f0302d9` | `docs(android): record the S4 exit decisions` | "S4 exit" bullet in DECISIONS.md |
| this commit | `docs(android): S4 exit report` | this file |

### Deviations from the merge procedure (recorded in DECISIONS.md)

1. **Spike branch instead of chained task branches only.** The cards said
   nothing is merged until S4-T8 and the tag `android/s4-spike` is set at the
   end. The integrators merged every approved card with `--no-ff` into a local
   branch `android/s4-spike` instead, because the phase gate forbids any
   `spikes/` path on `android/int`. The cards still chained their predecessors,
   so S4-T8 already held the whole history; the result is the same set of
   files. The name `android/s4-spike` is a branch, so S4.md references resolve
   through it. The planner added the local tag `android/p0-spike-S4` on the
   same commit (P0 convention) rather than a tag with the branch's name.
2. **One docs commit instead of two cherry-picks.** `android/int` already
   carried the wave records, so the two S4-T8 commits landed as `2daabdc`, with
   a three-way merge of DECISIONS.md. Checked at the exit: S4.md on
   `android/int` is byte-identical to `android/S4-S4-T8`, and DECISIONS.md
   differs from it only by the wave 7 record (before the "S4 exit" bullet).
3. **Phase cards committed late.** `docs/android/phases/S4.md` was committed at
   the exit (`02860ee`), not before wave 1. The workers worked from the same
   card text.
4. **No `S4-review.md`.** The reviews live in the workflow results; their
   findings are summarised below.
5. **Intermediate merges into `android/int` were undone.** In waves 4 and 5 a
   first merge into `android/int` was hard-reset before any gate run (to
   `7b92a9a` and `67921ad`). Nothing was pushed, so no published history was
   rewritten.

## Phase gate evidence (rerun by the planner on 2026-10-07)

Run in the integration worktree (`moyun-pos-wt/int`, the `$INT` of the cards)
on `android/int`, after `02860ee`. The text checks were repeated after this
report was committed.

| # | Check | Result |
|---|-------|--------|
| 1 | HEAD is `android/int` | PASS |
| 2 | only `docs/android/{spikes/S4.md,DECISIONS.md,phases/S4.md,phases/S4-exit.md}` differ from `0b68587` | PASS |
| 3 | no `spikes/` path in the `android/int` tree | PASS |
| 4 | no diff in `Cargo.toml Cargo.lock rust-toolchain.toml src-tauri ui crates deploy .github scripts` | PASS |
| 5 | no em-dash in the diff | PASS (0 hits) |
| 6 | Conventional Commit subjects | PASS |
| 7 | no client name in S4.md | PASS |
| 8 | `android/s4-spike:spikes/s4/notes/S4-T7.md` exists, its `run.log` has `TEARDOWN \| PASS` | PASS (`S4-T7 finished: exit 0, must-PASS failures 0, items: 37`) |
| 9 | `check_linux_frozen` | n/a (no script and no `linux-v1.1.0*` tag yet) |
| 10 | `ui`: `npm ci && npm run build` | PASS (89 packages, built in 266 ms) |
| 10 | `test_inventory.sh --diff` | PASS: `baseline 1846 lines, current 1846 lines: 0 added, 0 removed`, rc 0 |
| 11 | `test_inventory.sh --out`, `ws` count | PASS: 1355 (minimum 1355); 491 `cert`; 1846 lines |

The inventory file `target/s4-inventory.txt` has sha256
`586834f05390ed4535e34b22747d2a34c0f003542676684e0de8d8b0ff7f99a2`, the same
as the P0.1 baseline, so no test was added, removed or renamed.

**AVD check.** No emulator was running at the exit (`adb devices` empty, no
`qemu-system` process), so teardown rests on the S4-T7 `run.log` line:
`TEARDOWN | PASS | no device owner (list-owners: no owners), spike app uninstalled, only user 0; time zone Europe/Brussels, stay_on=0, adb_enabled=1`.

**Repro.** `scripts/check_repro.sh` was not rerun: no Linux build input
changed (check 4), so the P0.1 SHA-1 `83313be28027d2e8d9e84cbddff4e584799c2fbc`
still holds. The plan requires the repro check only at exits after the P3
freeze.

**Freeze.** S4 has no freeze step. The first freeze in the plan is the soft
freeze `linux-v1.1.0-rc1` at the P1b exit, which needs the user's go-ahead.
No Linux tag was created and `docs/android/LINUX_FREEZE.md` does not exist
yet; P1b creates it.

## Hashes

| Object | Value |
|--------|-------|
| `android/int` before the exit commits | `2daabdce023b7afc07b50a91a22110ef303afa81` |
| `android/s4-spike` = tag `android/p0-spike-S4` | `80a25eb43ae0a4075bfbb6c08591aa090d739007` (33 commits over `0b68587`, 529 files under `spikes/s4/`) |
| `docs/android/spikes/S4.md` sha256 | `e1689f91048bf3a874e1d7a480afdfef75b43db6b88e7aab8c112d55d00bb602` |
| `docs/android/phases/S4.md` sha256 | `1ddaf5802a3624a251319c1b414a8e20083a0a38caa92587866dabda06124f9e` |
| `docs/android/DECISIONS.md` sha256 (after `f0302d9`) | `63dd9526712aff2dd44b44a344dedfa79d9107b50c63d94ee2ed2bd5aafe1574` |
| Test inventory sha256 | `586834f05390ed4535e34b22747d2a34c0f003542676684e0de8d8b0ff7f99a2` (unchanged) |
| Linux SHA-1 (P0.1, not rerun) | `83313be28027d2e8d9e84cbddff4e584799c2fbc` |
| S4 probe library (S4-T2 review rebuild) | `libs4_probe.so` unstripped 30,876,712 B, SHA-1 `a4050f9ff3fb98482d3e9eacb4b26421e910f884`, LOAD aligned to `0x4000` |

## Results in one table (details in S4.md)

| Area | Outcome on the emulator | DECISIONS |
|------|-------------------------|-----------|
| Device owner via adb | works with one user; second user refused with the exact `IllegalStateException` text; the adb exit code is 0, the text is the signal; the account case NOT RUN (no account on the image) | §12, P4.4 |
| Lock Task | HOME, recents, back, Settings all contained; crash recovery 1 s, reboot to LOCKED 1 to 2 s after `boot_completed`; features revert to GLOBAL_ACTIONS at every boot; a second crash freezes the app unless `no_system_error_dialogs`; a stock launcher task blocks recovery | Q16, §12 |
| deviceId | `getSerial` as owner: `EMULATOR37X2X12X0` (17 chars), stable; `getWifiMacAddress` `null` with Wi-Fi off, fake `02:15:b2:00:00:00`; `ro.serialno` empty in the app | Q13, §3 |
| SAF export | works in Lock Task with a persisted tree grant; the volume root is refused; picker from the locked activity without `NEW_TASK`; virtual SD disk stood in for the SD card and the USB stick | Q17 |
| Printing, DLE EOT | six states read through the Rust engine, typed error when unplugged, no main-thread call; the 500 ms DLE EOT read misses late answers; Chinese prints as `?` | §12 OPEN items |
| Display, CJK | 1920x1080, 160 dpi, density 1.0; 996 px usable height unless immersive; 439 of 439 code points covered | Q19 |
| Silent update | silent session install in Lock Task, back and LOCKED at +0.60 s; OS refuses downgrade and other key, accepts a same-versionCode reinstall; no `REQUEST_INSTALL_PACKAGES` needed | Q18, §6 |
| Restrictions | 12 restrictions plus time lock, stay awake, keyguard, status bar and update policy accepted and enforced; adb off and back through the app | Q16 |
| Soak (30 min) | 31 samples, one pid, LOCKED 31 of 31, 179 beats, largest gap 10.0 s, PSS -2.1 percent, integrity ok; typed `specialUse` FGS start works | Q14, Q15 |
| Power cut | `adb emu kill`: LOCKED 3 s after boot, service back, 185 rows, integrity ok | §12 |
| bcrypt cost 12 | 295 ms on the emulator, 232 ms on the host | hardware residue |

**PENDING-S4 items:** all settled. Each row of DECISIONS §12 is now DECIDED
(S4 emulator), HARDWARE RESIDUE, or OPEN. The two OPEN items are P6 product
decisions (the DLE EOT timeout handling and Chinese on the customer ticket);
neither blocks P4. The remaining `PENDING-S4` strings in DECISIONS.md are the
legend or are explained in place.

## Decided while the user was away

All recorded in DECISIONS.md under "Decided while the user was away (review)":
- **Q13** deviceId: rule 1, the hardware serial; no in-app `ro.serialno`
  fallback; provisioning fails when `getSerial` throws.
- **Q14** 30 minute soak instead of 24 h in S4; the 24 h run in the P7 runbook.
- **Q15** typed `specialUse` foreground service start, plain call as fallback.
- **Q16** the P6 restriction set, `DISALLOW_DEBUGGING_FEATURES` last,
  `setLockTaskFeatures(NONE)` at every start, `DISALLOW_CONFIG_WIFI` off.
- **Q17** SAF export flow (PIN-gated folder pick, sub-folder, persisted grant).
- **Q18** update restart path (three ways back) and precheck order.
- **Q19** bundled Noto Sans SC, per-run locale tags, 1 dp = 1 CSS px.
- **Q20** the AVD left clean for P4 (no owner, no spike app, virtual SD kept).
- **Wave records 1 to 7:** spike code merged into `android/s4-spike`, never
  into `android/int`.
- **S4 exit:** the `android/p0-spike-S4` tag, the merge procedure deviations,
  no freeze at S4, the DLE EOT engine fix offered to P1a/P1b, launcher hiding
  on every till, the Q14 wording, the AVD overlay moved out.

## Reviewer findings carried forward (non-blocking)

**For later phases**
1. **No retry pattern in P6** (S4-T3): `run_t3.sh` retries each printer state
   up to 3 times to absorb the lost DLE EOT answer. That is a spike tolerance;
   production treats a missing answer as unknown.
2. **Escape window after an update** (S4-T5): the reviewer rerun caught about
   0.9 s of the stock launcher's taskbar with tappable icons in the
   single-task case, which the S4-T5 notes (line 86) say does not happen.
   P4.5 hides or suspends the stock launcher and Settings through the device
   owner on every till (DECISIONS "S4 exit").
3. **No exported receiver without permission** (S4-T5): the spike's
   `BgStartReceiver` is exported with no permission; P6 and P7 must not copy
   it.
4. **Power-cut checks as must** (S4-T7): the `power.*` items in `run_t7.sh` are
   recorded, not must-PASS; P4 and P7 make them must.
5. **`BOOT_COMPLETED` not observed** after the power cut (S4-T7, confirmed by
   the review rerun); the service starts from `onResume`. Hardware residue.
6. **End state not enforced** (S4-T5): `END.state` is not a must item.
7. **Lock Task features revert to GLOBAL_ACTIONS at boot** (S4-T6, consistent
   with AOSP `DevicePolicyData`): set NONE at every start.
8. **P4.4 hygiene** (S4-T1): drop the in-app `ro.serialno` fallback (done in
   §3); P4.4 removes or neutralises the stock launcher task after kiosk setup.

**Spike notes, cosmetic (left as is on the spike branch)**
- `spikes/s4/provision.sh:33` mentions a `dumpsys` fallback that is not
  implemented (S4-T1).
- `spikes/s4/notes/S4-T1.md:141`: the `run-as ... kill` claim is not in the
  script's evidence; the reviewer reproduced it (S4-T1).
- `deviceid_table.py` marks the sysfs wlan0 permission denial as stable
  (S4-T1).
- `Commands3.kt` bcrypt keeps the thread of the last run only (S4-T3).
- `LayoutProbeScreen.kt` cart lines add up to 29,00 while the totals show
  22,00; fix before reusing the screenshot in P5 docs (S4-T3).
- `spikes/s4/evidence/S4-T2/readelf-l.log` is from the round-0 `.so`
  (alignment rechecked on round 1).
- `spikes/s4/notes/S4-T4.md:162` says the SD card attempt left nothing; it
  left `sdcard.img.qcow2`, which the planner moved out of the AVD folder.
- `run_t6.sh:231` label counts 8 global restrictions next to a list of 12;
  `notes/S4-T6.md:164` cites end-state files from the run's exit trap, not a
  fresh boot (the reviewer's fresh-boot readback confirms the state).
- `notes/S4-T7.md` beats row: n=3 and n=4 are warm-up beats, not probe beats.
- `notes/S4-T7.md` line 8 calls the 30 minute soak a planner decision; it was
  the user's request (PD2).

## Risks

1. **Emulator only.** Every S4 result comes from an AOSP `google_apis`
   userdebug image on an M1. The Sunmi firmware can differ on identifiers,
   Lock Task, the vendor launcher, recovery, OTA and USB. The hardware residue
   list (S4.md, DECISIONS §12) is long; if no D3 Pro becomes available, the
   dossier must say tests ran on an emulated profile.
2. **Real serial format unknown.** The deviceId mechanism is decided, but the
   value a D3 Pro returns (format, length, stability after a factory reset) is
   not. If the firmware withholds `getSerial` from the device owner, PD4 rule 2
   or 3 applies and P4.4 changes.
3. **Printer status can be silently wrong.** `pos-print` reports "ready" when
   the DLE EOT answer is late. This also affects Linux. A fix in the engine is
   cheapest before the soft freeze (P1b exit); after it, it is a Linux v1.2
   change or an app-level workaround.
4. **Escape window during an update.** Lock Task ends when the installer kills
   the process; for up to about 1 s the stock launcher can show. Hiding the
   launcher and Settings closes most of it; a real plug pull during an update
   is not tested.
5. **Policy writes lost on abrupt power loss.** An emulator stopped within about
   10 s of an owner or package change lost the policy write. Provisioning must
   flush (or reboot) before reporting success; on a real till, a power cut
   during provisioning may need a re-provision.
6. **No account refusal tested.** If a Sunmi account or the setup wizard
   blocks `set-device-owner`, provisioning needs QR provisioning after a
   factory reset (the plan's known P0 risk).
7. **Host load.** S4-T7 swapped heavily on the 16 GB host (12.6 of 13.3 GB
   swap). The 24 h run and later Gradle plus cargo builds must not overlap.
   `int/target` holds 19 GB of cargo output (the inventory build); keep it for
   later gates or delete it when disk runs short.

## Questions for the user

1. **Review the "Decided while the user was away" bullets** Q13 to Q20, the
   wave records and the "S4 exit" bullet.
2. **DLE EOT engine fix.** May the P1a/P1b planner change `pos-print` (treat a
   missing DLE EOT answer as unknown, longer read timeout) before the soft
   freeze? It changes Linux behaviour too.
3. **Chinese on the customer ticket.** French only (current behaviour, Chinese
   becomes `?`), or a raster path for Chinese lines in P6?
4. **Hardware.** Is a real Sunmi D3 Pro (and the Epson TM-m30III) coming
   before filing? If not, the dossier states the emulated profile.
5. **Timed adb in v1.** S4 proved adb can be turned off and back on by the app.
   Should the manager have a PIN-gated "adb on for N minutes" mode, or is adb
   off for good after provisioning (Q16 default: no timed adb unless the
   dossier accepts it)?
6. **Spike tags.** Keep `android/s4-spike` (branch) and `android/p0-spike-S4`
   (tag) local only, like the other spike tags?

## What the next phases must know

**P1a/P1b (next in the plan; nothing from S4 is required)**
- `android/int` = `0b68587` plus docs. Start the engine extraction from it.
- Consider the `pos-print` DLE EOT fix before the soft freeze (question 2).
- The soft freeze at the P1b exit creates `linux-v1.1.0-rc1` and
  `docs/android/LINUX_FREEZE.md`, with the user's go-ahead.

**P2/P3**
- The S4 probe confirms the UniFFI and Gradle wiring of S2 with the real
  `pos-print` and rusqlite (WAL, `synchronous=FULL`, Backup API): Exec task
  running `cargo ndk` and library-mode bindgen on the unstripped `.so`,
  `~/.cargo/bin` first on PATH, JNA `5.19.1@aar`, R8 keep rules of DECISIONS
  §11.7. Engine open including `uniffiEnsureInitialized` took 34 ms.

**P4 (app foundation and provisioning)**
- Follow S4.md "Recommendations for the later phases" for P4.1, P4.2, P4.4 and
  P4.5. In short: typed `specialUse` foreground service started from
  `onResume`; deviceId from `Build.getSerial()` with `READ_PHONE_STATE`
  granted by the owner, no fallback; `setLockTaskFeatures(NONE)` before every
  `startLockTask()`; `no_system_error_dialogs`; persistent preferred HOME plus
  a `MY_PACKAGE_REPLACED` receiver plus relock in `onResume`; hide or suspend
  the stock launcher and Settings; flush before reporting provisioning done;
  never trust the `dpm` exit code.
- The AVD `sunmi_d3_pro_api33` is clean: no device owner, no spike app, only
  user 0, time zone Europe/Brussels, virtual SD disk `BFD1-0A11` kept
  (`persist.sys.virtual_disk=true`). It has no snapshot, so boots are cold.
- Emulator hazards for scripts: `sync; sleep 12; sync` before stopping the AVD
  after an owner or package change; check `airplane_mode_on`; send
  `KEYCODE_WAKEUP` first; wait for `init.svc.bootanim=stopped` before
  screenshots. `spikes/s4/emu.sh` and `spikes/s4/tools/ui.py` on
  `android/s4-spike` are templates.

**P5**
- 1 dp = 1 CSS px with `Density(widthPx / 1920f, 1f)`, `fontScale` 1.0,
  immersive on every screen; bundled Noto Sans SC; locale tags per `Text`.

**P6**
- The restriction set of Q16, the SAF flow of Q17, the updater order of Q18,
  printer state "unknown" when DLE EOT is missing, no spike retry pattern, no
  exported receiver without a permission.

**P7**
- 24 h soak: `SOAK_MINUTES=1440 caffeinate -i spikes/s4/run_t7.sh` adapted to
  the D3 Pro (adb serial instead of `emu.sh`, a real plug pull instead of
  `adb emu kill`), with the power-cut items as must-PASS, plus the hardware
  residue checks of S4.md.

## State left

- `android/int`: docs only over `0b68587`; working tree clean after this
  commit.
- Branches kept: `android/S4-S4-T1` to `android/S4-S4-T8`, `android/s4-spike`.
  Local tag `android/p0-spike-S4`. No S4 worktree remains apart from `int`.
- AVD stopped and clean (see P4 above). The dangling `sdcard.img.qcow2`
  overlay from an S4-T4 attempt was moved out of the AVD folder.
- No emulator, Gradle daemon or printer mock left running by the planner.
  Nothing pushed, no GitHub Actions, no live module2 or HubRise call, no
  release key, nothing sent to the SPF.
