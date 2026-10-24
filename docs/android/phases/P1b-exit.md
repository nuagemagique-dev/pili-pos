# P1b exit report: UI fiscal triggers into the engine

> Correction (2026-10-10): the Linux build is not filed as v1.1 under Art. 113. Nothing was filed, so Linux and Android are both version 1.0.0 initial filings (Art. 99), and the freeze tag is `linux-v1.0.0-rc1`, not `linux-v1.1.0-rc1` (`docs/android/DECISIONS.md`, "Decided by the user"). The report below is unchanged.

Written by the P1b planner on 2026-10-08, at the end of the P1b run.

- **Base:** `android/int` at `7b8a2da` (`docs(android): P1a phase cards, exit report and exit decisions`).
- **Exit head:** `android/int` at `042d911` (`chore(android): merge P1b-T10`) before the exit commit, then the commit that adds `phases/P1b.md`, this report and the exit decisions (PD47 to PD50).
- **Cards:** `docs/android/phases/P1b.md`. Decisions: `docs/android/DECISIONS.md`, "Decided while the user was away (review)" (PD30 to PD46 from P1b-T1, PD47 to PD50 at this exit). Engine reference: `docs/android/ENGINE.md` (sections 6 and 7).
- main (`96728e1`) and `cert/v1.0` (`d6210ca`) were not touched. Nothing was pushed, no GitHub Actions run, no live module2, HubRise or SPF call, no `declare.sh`.

## Verdict

- **All ten cards are APPROVE and merged** into `android/int` with `--no-ff`, one merge per card, no conflicts, no rework round, no escalation.
- **`android/int` builds and the phase gate passes** at `042d911`: `p1b_gate.sh --ui` 0 failed of 36 checks and `p1b_gate.sh --exit` 0 failed of 38 checks (integrator, wave 6), and the same exit gate on the identical tree `e90acb6` (T10 reviewer). `042d911` and `e90acb6` have the same tree (`8b03a8a9...`). The planner's own rerun of the gate at the exit was refused by the session's permission system (see "What the planner could not do"); the figures below are the integrator's and the reviewers'.
- **The moved rules now live in the engine.** Leaving an order (P event, takeaways excluded), the clock-in at login and the clock-out at end of shift, the 5 minute idle lock, and the FDM status polling and classification are engine functions and background loops; the Vue UI only calls the seven new commands and listens to the three new events. `p1b_gate.sh --exit` checks that the old Vue rules are gone from `ui/src` (`ui-rules` PASS).
- **No behaviour change was found in the FDM requests.** The FDM journal of the exit commit passes PD40: business view equal to baseline run1 after the version mapping, the five counter triples equal, every `posSwVersion` `pili-pos/1.1.0`, and even the `Status`-only list is equal.
- **Version 1.1.0** in `Cargo.toml`, `src-tauri/tauri.conf.json`, `ui/package.json` and `ui/package-lock.json`; `get_settings` returns `posSwVersion` `pili-pos/1.1.0`, shown on the login screen.
- **The soft freeze was NOT set.** `scripts/check_repro.sh` stops at step 3/5 of `scripts/build_and_hash.sh` (the fiscal archive mutation scan) before any SHA-1 is computed. No `linux-v1.1.0-rc1` tag exists, `docs/android/LINUX_FREEZE.md` is not created, and `check_linux_frozen.sh` still prints SKIP. This needs the user (PD47, question 1).

**Planner recommendation:** P2 can start on `android/int` (it builds in `android/rust` and needs no Linux change), with `042d911` as the provisional freeze point until the user settles the scan question and the rc1 tag is set (PD48). Merging into main waits for the user (PD45).

## Task status

| Card | Wave | Branch tip | Worker | Review | Merge on `android/int` |
|------|------|------------|--------|--------|------------------------|
| P1b-T1 gate tooling, Playwright inventory, journal compare, PD30 to PD46 | 1 | `android/P1b-P1b-T1` `fa9079e` | done | APPROVE | `b752020` |
| P1b-T2 `leave_order`, `set_active_order`, `lock_session`, `session-locked` | 1 | `android/P1b-P1b-T2` `e7e8c40` | done | APPROVE | `0976f83` |
| P1b-T3 DLE EOT seam, `posSwVersion` in `get_settings` | 1 | `android/P1b-P1b-T3` `49eba9c` | done | APPROVE | `264d809` |
| P1b-T4 clock-in inside login, `end_shift`, `presence-failed` | 2 | `android/P1b-P1b-T4` `656306b` | done | APPROVE | `45bd439` |
| P1b-T5 Vue on leave, active order, lock; version on the login screen | 2 | `android/P1b-P1b-T5` `e6dfcdf` | done | APPROVE | `c444155` |
| P1b-T6 engine idle lock, `user_activity` | 3 | `android/P1b-P1b-T6` `838d66b` | done | APPROVE | `67f417f` |
| P1b-T7 FDM monitor, `fdm_health`, `fdm_check_now`, `fdm-status` | 4 | `android/P1b-P1b-T7` `8eb45ae` | done | APPROVE | `abda3a4` |
| P1b-T8 Vue on the engine presence and idle lock | 4 | `android/P1b-P1b-T8` `b9c68d3` | done | APPROVE | `a463f36` |
| P1b-T9 Vue FDM store on the engine monitor | 5 | `android/P1b-P1b-T9` `380aa56` | done | APPROVE | `6c62f1b` |
| P1b-T10 version 1.1.0, ENGINE.md, dossier prose, FDM journal | 6 | `android/P1b-P1b-T10` `e90acb6` | done | APPROVE | `042d911` |

The card branches are kept locally. The task worktrees are removed except `moyun-pos-wt/P1b-P1b-T7` and `moyun-pos-wt/P1b-P1b-T8`: their removal was refused by the permission system at the wave 4 integration. Both are clean and fully merged, and their `target` directories are already gone; removing them is left to the user (question 4).

### What each card delivered (from the reviews)

- **T1.** `p1a_gate.sh --allow-ui` (default unchanged), `p1b_gate.sh` (`--fast`, `--exit`, `--ui`, `--base`), `playwright_inventory.py` with 18 unit tests, `playwright_list.txt` (280 tests, source `7b8a2da`), `compare_fdm_journals.sh` with a Docker-free self-test (7 cases), a Docker target volume per checkout in `capture_fdm_journals.sh` (PD43), PD30 to PD46 in DECISIONS.md byte for byte as the cards.
- **T2.** `commands/session.rs` (`SessionRules`, `do_leave_order`, `do_set_active_order`, `lock`, `do_lock_session`), `session-locked`, three wrappers, `session_rules.rs` (14 tests). Leave signs P only for an open dine-in order; a lock registers the active order while the session is still open, never blocked by an FDM failure.
- **T3.** `PrinterStatus.status_known`, `DEFAULT_STATUS_READ_TIMEOUT` 500 ms, `with_status_timeout`, `send_to_printer_with`; `EngineTimers.printer_status_read` passed at every status-reading engine print site; `posSwVersion` in `get_settings`. No Linux behaviour change (same bytes, same 500 ms, `PrinterStatus` never persisted).
- **T4.** `auth::authenticate` split out of `do_login` with the same checks and order; login and first-PIN login queue a clock-in; presence worker (`presence-failed` on failure); `do_end_shift` (wrong PIN refused without FDM call, one `signWorkOut`, session always closed); `presence.rs` (8 tests), background task count 5.
- **T5.** `ipc.ts` (`leaveOrder`, `setActiveOrder`, `lockSession`, `posSwVersion`); order store `leave(reason)`; router guard, AppHeader and CartSidebar on `leave`; `auth.lock` on `lock_session` (with a `locking` flag so the own lock event is not handled twice); version on LoginView. Playwright 289 (+9).
- **T6.** `EngineTimers.idle_lock` 300 s; `touch` on every `open_session`; `do_user_activity`; idle loop calling `session::lock(Idle)`; tests stable over 6 consecutive runs; background task count 6.
- **T7.** `commands/fdm_monitor.rs` (`classify`, `pick_primary` equal to the Vue store branch by branch, `poll_delay` 30/60/120/240/300/300 s), monitor loop polling only while a session is open, `fdm-status`, `fdm_health`, `fdm_check_now`; 21 unit tests and 10 loop tests.
- **T8.** Auth store on the engine presence (`presence-failed` toast, `end_shift`), `App.vue` forwards touch, mouse and key at most once per second, `useIdleTimer.ts` deleted. Playwright 295 (+15 in all), 4 renames declared.
- **T9.** FDM store applies `FdmHealth` from `fdm-status` and `fdm_check_now`; the UI poller and classification are gone (no timer left in `stores/fdm.ts`); 3 renames declared.
- **T10.** Version 1.1.0 (Cargo.lock: the 15 workspace member version lines only), ENGINE.md final pass with section 7 "UI rules owned by the engine", dossier prose where a moved rule was said to live in the UI (no fiscal statement, figure, evidence or `pili-pos/1.0.0` string changed; 146 such strings before and after), CLAUDE.md and README.md, FDM journal capture and compare.

## Phase gate evidence

Recorded by the wave 6 integrator on `042d911` (clean tree, `main` and `cert/v1.0` unchanged, only the allowed tag names, 40 GB free, `ui/dist` present) and by the T10 reviewer on `e90acb6` (same tree).

| Check | Result |
|-------|--------|
| `p1b_gate.sh --ui --base 7b8a2da...` | PASS, 0 failed (36 checks) |
| `p1b_gate.sh --exit` | PASS, 0 failed (38 checks) |
| tests (workspace) | 1466 passed, 0 failed |
| tests-cert-pos-engine / tests-cert-pili-pos | 576 passed / 8 passed |
| test-inventory | ok, no test disappeared (ws 1479 >= 1355) |
| ui-build, ui-e2e | PASS, 295 passed |
| ui-e2e-inventory | added 15, removed 0, total 295 |
| ipc-wrappers | `commands: 86 (84 without peppol), single-call: 86/86` |
| ipc-signatures-additive | PASS against `7b8a2da` (every one of its 79 signatures kept; the cards added 3, 1, 1, 2 for the seven new commands) |
| engine-tauri-free, engine-tree, engine-globals | PASS (0 globals) |
| hygiene (em-dash, subjects, artifacts, untouchable, Cargo.lock, baseline-p1b) | PASS |
| dossier-check, uc-matrix, dossier-paths | PASS |
| ui-rules (`--exit`) | PASS: all moved rules are gone from `ui/src` |
| version (`--exit`) | PASS: 1.1.0 in all three places |
| linux-frozen | SKIP: no freeze tag (see below) |
| ENGINE.md section 6 against `check_ipc_wrappers.py --map` | equal, 86 rows, 84 cert (T10 reviewer, compared programmatically) |

The integrator ran the phase gate after every wave: 36 of 36 checks after waves 1 to 5 (`--ui`). Incidents, none caused by a merge: one Playwright flake in `checkout-change.spec.ts` (2 tests, 5 s timeout under two concurrent cargo builds) during the T1 review, passed on rerun (PD46); the free disk fell to 14 GB (T1 review) and 18 GB (wave 4) while workers built, and was back above 25 GB before each integration step.

## Test inventories

| Figure | Start (`7b8a2da`) | Exit (`042d911`) |
|--------|-------------------|------------------|
| Rust ws tests listed | 1405 | 1479 (+74), 0 removed against the baseline (ws floor 1355) |
| Rust workspace tests passed | 1392 (P1a exit) | 1466 |
| cert pos-engine tests passed | 508 (P1a exit) | 576 |
| cert pili-pos tests passed | 8 | 8 |
| Playwright tests | 280 in 22 files | 295, 15 added, 0 removed |
| Playwright renames | none | 8 rules in `playwright_moves.tsv` (T5 1, T8 4, T9 3), all used |

Added Rust tests by card (reviews): T2 +15, T3 +9, T4 +11, T6 +7, T7 +32 (ws 1405, 1420, 1429, 1440, 1447, 1479 after each). Added Playwright tests: T5 +9, T8 +6. No test was deleted, renamed outside a declared move, or newly ignored.

## FDM journal (PD40)

Captured in Docker against the local mocks only (`capture_fdm_journals.sh`, volume `pili-e2e-p0-target-60733939`), suite 8/8, 9 FDM calls, binary `pili-pos v1.1.0`.

| Capture | Commit | Business view (no `Status`, version mapped) | Counters | `posSwVersion` | `Status` only | Normalised sha256 |
|---------|--------|---------------------------------------------|----------|----------------|---------------|-------------------|
| baseline run1 (P0.1, mapped to `pili-pos/1.1.0`) | `96728e1` | `b92750aa...` | `S 1 1, N 1 2, N 2 3, P 1 4, N 3 5` | `pili-pos/1.0.0` (mapped) | `cfc66440...` (4 entries) | `68a09ee3...` |
| P1b-T10 worker | `e90acb6` | `b92750aa...` PASS | equal PASS | 5 of 5 `pili-pos/1.1.0` PASS | `cfc66440...` same | `839393661f959c30...` (raw `45c63b71...`) |
| P1b-T10 reviewer, fresh out-dir | `e90acb6` | `b92750aa...` PASS | equal PASS | 5 of 5 PASS | same | `839393661f959c30...` (raw `4950c78e...`) |
| wave 6 integrator (exit) | `042d911` | equal to baseline PASS | equal PASS | 5 of 5 PASS | reported | `target/journals/fdm_journal_p1b_exit*.json` (not committed) |

Full values: business view `b92750aa569fd856baf9021cda30434f035edbd3f02ac3ac407be2f72c6af497`; `Status` only `cfc6644062c7baf8fcec6621bb43174238a743e9044e6da56cb9eb2fa14f8b45`; normalised journal of `e90acb6` `839393661f959c30c12b073aa42dfdbd67dbbe997ad01108c9109f3c8fe9b44d`. The normalised hash differs from the baseline's `68a09ee3...` only through the version string (and, as in P1a, possibly the place of a `Status` poll); the raw hashes always differ by timestamps and signatures. The baseline files are not edited.

## Reproducible build and soft freeze (PD44): not done

What happened at the exit (wave 6 integrator):

- `scripts/check_repro.sh` was run from a clean local clone of `042d911` (a worktree cannot be used: Docker in the build container cannot follow the worktree's `.git` file).
- Run 1 compiled the cert build, then `scripts/build_and_hash.sh` stopped at step 3/5, the fiscal archive mutation scan ("no source file may contain a DELETE or UPDATE statement targeting fiscal_events"), with two hits:
  - `crates/pos-engine/tests/spf_e2e_crash_safety.rs:154`, a test helper that lifts the append-only trigger to remove the test's own poison row;
  - `crates/pos-engine/tests/spf_e2e_peripherals.rs:415`, a test assertion that a DELETE on the backup copy is refused.
- Both lines are integration test code, compiled only by `cargo test`, never part of the cert binary. They existed at v1.0 in `src-tauri/tests`, which the scan never covered (it scans `crates` and `src-tauri/src`). P1a commit `aeee41a` moved those tests to `crates/pos-engine/tests` (PD1), which is inside the scanned tree. The P1b cards did not touch them. P1a had no `check_repro.sh` run (PD11), so the effect shows up only now.
- No SHA-1 was produced, so there is no `target/repro-p1b/sha1`, no `linux-v1.1.0-rc1` tag and no `docs/android/LINUX_FREEZE.md`. `check_linux_frozen.sh` prints SKIP (exit 2).

Why the run did not fix it: the scan is a control declared in the dossier (`docs/SUBMISSION_ARTIFACTS/GARANTIE_MAJ_LANGUES_FR.md` section 1.2), `scripts/build_and_hash.sh` is a never-edited Linux build input, and the test files are inside the freeze paths. The integrator's attempt to change the scan's scope and the planner's attempt to change the two test spots were both refused by the session's permission system as weakening a security control. The decision is the user's (PD47, question 1).

## What the planner could not do

At this exit the session's permission system refused, besides the test change above: a rerun of `p1b_gate.sh --exit`, reading the exit journal hashes, and some read-only shell commands. The planner therefore relies on the integrator's and the T10 reviewer's gate and journal evidence (both independent reruns, consistent with each other). The planner checked with git only that `042d911` and `e90acb6` have the same tree, that the working tree is clean, that main and `cert/v1.0` are unchanged and that no `linux-*` tag exists.

## Hashes and references

| Item | Value |
|------|-------|
| P1b base | `7b8a2da406a676d5839e46f07c56db7af4fdd75b` |
| last card merge (`chore(android): merge P1b-T10`) | `042d911` (tree `8b03a8a9dd0fd86d2d057506a61023bf4be8aeea`, same as `e90acb6`) |
| main | `96728e1325f5e11d69b7571d17280e3027974e06` |
| `cert/v1.0` | `d6210caabb32ca09f54be0f15410ce8447960be2` |
| diff `7b8a2da..042d911` | 77 files, 5413 insertions, 478 deletions, 54 commits (10 merges) |
| Linux baseline SHA-1 (P0.1, main `96728e1`) | `83313be28027d2e8d9e84cbddff4e584799c2fbc` |
| Linux 1.1.0 SHA-1 | none: `check_repro.sh` stopped at the mutation scan |
| local tags | `android/p0-spike-S1` to `-S5` only; no `linux-*` tag |
| free disk at the exit | 34 GB |

## Decisions taken while the user was away

All in `docs/android/DECISIONS.md` under "Decided while the user was away (review)".

- **PD30 to PD46** (planner, before wave 1, copied by P1b-T1): card layout, the seven commands, the three events, leaving an order, presence, idle lock, FDM monitor, the DLE EOT seam, the version, what is not moved, the journal rule, the Playwright inventory, the gate, the capture volume, the soft freeze rule, the P1a exit questions 4 and 5, flakes and disk.
- **PD47 to PD50** (this exit): the soft freeze is not set and the mutation scan question goes to the user (PD47); the provisional freeze point for P2 (PD48); the review notes accepted as they are and carried forward (PD49); exit records (PD50).

## Risks

1. **No soft freeze yet.** Until the user settles the scan and `check_repro.sh` passes, nothing mechanically stops a later phase from changing the Linux build inputs. PD48 makes `042d911` the provisional freeze point; settling the scan will itself touch a frozen path, so the rc1 commit will be a later commit than `042d911`.
2. **The Linux 1.1.0 SHA-1 is unknown.** Reproducibility of the extracted engine has not been shown yet; the PD44 rule (two equal SHA-1 or stop) still applies once the scan is settled. Each `check_repro.sh` takes about 80 minutes here.
3. **End-of-shift monitor query** (T7 review): `do_end_shift` opens a short technical session, so the FDM monitor sends one `Status` query and one `fdm-status` event around the clock-out. Allowed by PD36 and softened by the 20 s cache; a later card may skip it.
4. **Shutdown during an idle lock** (T6 review): a shutdown can drop an idle lock in the middle of an FDM sign; the `pending_signs` journal resends it before the next sign, so nothing is lost.
5. **Concurrent lock paths** (T6 review): an idle lock and a manual `lock_session` at the same moment could emit two `session-locked` events. Unlikely, because a tap touches the timer first.
6. **Timer tests.** The idle and monitor tests passed 3 to 6 consecutive runs in review; PD46 applies to any later flake.
7. **Dossier drift.** The dossier still shows the `pili-pos/1.0.0` evidence (146 strings, PD38); P8 regenerates it. The v1.1 filing must not use the v1.0 SHA-1.
8. **Disk.** 34 GB free at the exit, `int/target` 27 GB; a repro clone adds a full Linux build; PD46 stays.

## Open items for the next phases

- **Freeze:** the user's answer to question 1, then `check_repro.sh` (two equal SHA-1), the local tag `linux-v1.1.0-rc1`, `docs/android/LINUX_FREEZE.md`, and `check_linux_frozen.sh` PASS on `android/int`.
- **P2:** the parity target is 84/84 cert commands (PD31). `work_in`, `work_out`, `check_fdm_status`, `sign_table_order` and `logout` stay cert commands (P5 records them as UI parity exclusions). Android passes `printer_status_read` of about 2 s and shows "printer state unknown" when `status_known` is false (PD37). The engine has three new loops (presence worker, idle lock, FDM monitor) whose shutdown order matters for the PD27a close sequence. `idle_lock` and `fdm_poll` are `EngineTimers` inputs.
- **P4.5 / P5:** forward `user_activity` from the root pointer interceptor (throttled like the Vue UI), listen to `session-locked`, `presence-failed` and `fdm-status`, and keep MANDATORY FDM messages on screen; the once-per-bookingDate display stays a UI convenience (PD39).
- **Docs nits** (T7, T9, T10 reviews): an ENGINE.md sentence about payloads sits in the wrong subsection; the "Background loops at a glance" table at ENGINE.md:61 breaks the numbered list; the header of `playwright_moves.tsv` still says "No rule yet."; the `fdm` store could compare `checkedAt` before applying an older reply. All cosmetic; for a later docs or Vue card.

## Questions for the user

1. **Mutation scan and the soft freeze (PD47).** `scripts/build_and_hash.sh` step 3 now flags two integration test lines that P1a moved into the scanned tree. How do you want the scan and these tests to relate, given that the scan is declared in the dossier (section 1.2 of `GARANTIE_MAJ_LANGUES_FR.md`)? Both fixes tried during the run were refused by the permission system, so the change needs your go-ahead (and, if it touches the scan, your wording for the dossier). After that: may the chain run `check_repro.sh` and set the local tag `linux-v1.1.0-rc1` on success (PD44)?
2. **Provisional freeze point (PD48).** Do you accept that P2 starts with `042d911` as the provisional freeze point, any change to the Linux freeze paths during P2 being an escalation?
3. **Journal rule (PD40).** Do you accept the business-view comparison as the journal parity rule for the v1.1 evidence?
4. **Worktrees.** May the clean, merged worktrees `moyun-pos-wt/P1b-P1b-T7` and `moyun-pos-wt/P1b-P1b-T8` be removed (branches kept)?
5. **Merge to main.** `android/int` now carries P1a and P1b. When do you want it reviewed and merged into main (PD45)?
