# P5 exit report: UI lane, run A and the first views (partial)

Phase cards: `docs/android/phases/P5.md`. Lane branch `android/lane-ui` (worktree `moyun-pos-wt/lane-ui`), base `ba57adf06191a1f386863f98f24930b6f5aa543c` (= `android/int`). Written by the P5 planner on 2026-10-09 while the user was away. Nothing was pushed, no tag was created, no GitHub Actions run, no live module2, HubRise, Scaleway or SPF call, no `declare.sh`, no release key. main (`96728e1`) and `cert/v1.0` (`d6210ca`) are untouched.

## Result in one paragraph

The run closes **partial** (PD240). Seven of the twelve cards are merged: the gate and parity tools (T1), the keypads and dialogs (T2), toasts, error presenter and QR (T3), the session, settings and integrations stores with the text and entry-point helpers (T4), the order store and product names (T5), money, Brussels time, VAT and the banners (T7), and the Login view (T9). Five cards delivered nothing: P5-T6 (FDM) stopped under the 8 GB disk rule, and P5-T8 (scaffold), T10 (Tables), T11 and T12 (Order) depend on it in a chain. No card failed review on its code. The lane head builds, passes the phase gate and can be merged into `android/int` as it is; the five open cards run unchanged in a short continuation run before P5b (PD241).

## Branch state

| Item | Value |
|------|-------|
| lane head | the commit that adds this file, on top of `3d351dc` (`docs(android): P5 phase cards and prune the called parity lines`) |
| last code merge | `c3f555c` (`chore(android): merge P5-T9`) |
| diff vs base | 110 paths (`git diff --name-only ba57adf...HEAD`), all inside `lane_paths.py --regex ui` plus `DECISIONS.md` |
| frozen refs | main `96728e1325f5e11d69b7571d17280e3027974e06`, `cert/v1.0` `d6210caabb32ca09f54be0f15410ce8447960be2`, `linux-v1.1.0-rc1` -> `65b567fdd2497277300340979c2c61ff17abc918`, no `linux-v1.1.0` tag |

## Per-card status

| Card | Wave | Verdict | Status | Card commits | Merge commit |
|------|------|---------|--------|--------------|--------------|
| P5-T1 gate and parity tools | 1 | APPROVE | merged | tip `685464c` | `03348ca` |
| P5-T2 keypads and dialogs | 1 | ESCALATE (disk only), accepted by the planner | merged | `20d0c97`, `59bc9aa` | `ec62e90` |
| P5-T3 toasts, errors, QR | 1 | APPROVE | merged | `5379b2f`, `1c5e8a7` | `b52383a` |
| P5-T4 stores, texts, entry points | 1 | APPROVE | merged | `0b78174`, `53b5674` | `92d449e` |
| P5-T5 order state, product names | 1 | APPROVE | merged | `8639acd`, `d7ed7d7`, `4c6885d` | `ae22c22` |
| P5-T6 FDM | 2 | REJECTED (no branch) | not built | none | none |
| P5-T7 money, time, VAT, banners | 2 | APPROVE | merged | `b8d762a`, `2a270fc` | `1c29582` |
| P5-T8 scaffold | 3 | REWORK (empty, blocked on T6) | not built | none (branch at `22b1b0d`) | none |
| P5-T9 Login | 3 | APPROVE | merged | `d9a19d0`, `9431a4e`, `76708ce`, `6003e47` | `c3f555c` |
| P5-T10 Tables | 4 | REWORK (empty, blocked on T8) | not built | none (branch at `c3f555c`) | none |
| P5-T11 Order part 1 | 4 | REWORK (empty, blocked on T8) | not built | none (branch at `c3f555c`) | none |
| P5-T12 Order part 2 | 5 | REWORK (no branch, blocked on T11) | not built | none | none |

Planner and integrator commits on the lane: `beb9ea5` and `b00c0f8` (wave 1 decisions and gate result), `22b1b0d` (wave 2 decisions and the P5-T6 rejection), `3d351dc` (this phase file and the parity prune), and the commit that adds this report and the exit decisions.

Why the chain stalled: P5-T6 never started because the host disk was at 3.4 to 4.0 GB free (other lanes' Rust `target` directories, about 18 GB in `lane-plat` and 14 GB in `lane-export`). Its rerun was scheduled (DECISIONS, P5-T6 entry) but not executed in this run, so T8 had nothing to build on, and T10 and T11 then saw "P5-T8 merged" only because the empty T8 branch is an ancestor of the lane. The workers of T8, T10, T11 and T12 stopped correctly instead of stubbing a dependency outside their paths, and the reviewers confirmed that nothing was fabricated.

## Gate evidence

All runs on `android/lane-ui`, worktree `moyun-pos-wt/lane-ui`, base `ba57adf`, allow regex `$(lane_paths.py --regex ui)|^docs/android/DECISIONS[.]md$`.

Static phase checks at `3d351dc` (this run, all pass): clean tree; the four ref checks; `check_linux_frozen.sh` prints `frozen against linux-v1.1.0-rc1 (65b567f): 16 paths, clean`; the frozen-path `git diff --quiet` is empty; lane scope empty; `LANES | total | PASS | 3493 files, 73 rules, 66 probes`; `KTESTS | total | PASS | base 318, head 616, removed 0`; `UIPARITY | total | PASS | called 28, excluded 9, pending 45, stale 0, missing 0 (82 commands)`.

`scripts/android/p5_gate.sh --exit --base ba57adf... --allow "$UI"` at `3d351dc` (this run): 34 PASS, 1 SKIP (`frozen-provisional`, retired), 1 FAIL. The FAIL was `plan-gradle` with `Gradle build daemon has been stopped: stop command received`: another agent ran `./gradlew --stop` while the check compiled `pos-ffi`, as already seen in the P5-T7 review. The plan's literal command `./gradlew lint ktlintCheck detekt test koverVerifyDebug assembleRelease` was then rerun alone through `heavy.sh` on the same tree: `BUILD SUCCESSFUL in 2m 42s`, exit 0. Taken together the exit gate is green. Selected lines of the gate run:

| Check | Result |
|-------|--------|
| hygiene-paths, emdash, artifacts, subjects (29 Conventional Commits), root-lock, keys | PASS |
| linux-frozen, freeze-tag | PASS, 16 paths clean against `linux-v1.1.0-rc1` |
| app-only-scope, ffi-boundary, app-manifest (6 permissions) | PASS |
| lanes, vue-strings (481 keys), kotlin-tests (318 to 616, removed 0) | PASS |
| self-tests: p4-gate 75, heavy 11, p5-gate 183, ui-parity 46, kotlin-tests, lanes, vue-strings | PASS |
| app-lint, app-ktlint, app-detekt | PASS |
| app-unit-tests | PASS, 573 tests |
| app-screenshots (verifyRoborazziDebug) | PASS |
| app-kover | PASS, LINE 97.1 percent, BRANCH 76.1 percent (floor 80/60) |
| app-assemble-release, app-apk-check | PASS, 6 checks, `e98a482dac5d` |
| plan-gradle | FAIL in the run (daemon stopped by another agent), PASS on the rerun |
| ui-parity, ui-contracts, ui-money, ui-strings (2 files) | PASS |

Run by the wave 5 integrator at `c3f555c` (the same code; this run only adds docs): `p5_gate.sh` 34 checks and `p5_gate.sh --exit` 35 checks, 0 failed; `p4_gate.sh --fast --app-e2e --base ba57adf`: 22 of 22, including `app-e2e` 11 of 11 on the reused `emulator-5554` (the integrator did not start it, so it left it running); the rerun `:app:testDebugUnitTest --rerun :app:verifyRoborazziDebug --rerun :app:koverVerifyDebug`: BUILD SUCCESSFUL. I did not repeat the e2e run: no code changed since, and the emulator belongs to another agent.

Not run here, as planned for the P5 exit on `android/int` after the lanes merge: the int-worktree checks, `p4_gate.sh --exit --root --android` (root test inventory) and `p4_gate.sh --fast --linux-repro` (Linux SHA-1 `9d230ef1bba90a4edde0157bcb107e4bb4d23d6e`). That merge gate must also repeat `p4_gate.sh --app` with Roborazzi verify and Kover (the P5-T2 caveat).

## Figures

| Figure | Base (`ba57adf`) | Lane head |
|--------|------------------|-----------|
| Kotlin test inventory | 318 | 616 (removed 0) |
| `:app` host unit tests | 275 | 573 |
| Kover LINE / BRANCH | 95.0 / 76.0 | 97.1 / 76.1 |
| goldens | 2 (theme) | 18: 12 components (`amount_dialog`, `button_variants`, `config_problems`, `dialog_confirm`, `numpad_default`, `pinpad_enter`, `pinpad_error`, `qr_sample`, `toast_variants`, `training_banner`, `training_watermark`, `vat_summary`), 4 login (`login_first_pin`, `login_numpad`, `login_pin`, `login_tiles`), 2 theme |
| engine commands called from `:app` | 1 (`user_activity`) | 28 |
| parity | none | called 28, excluded 9, pending 45, stale 0, missing 0 |

The plan expected called 38 and pending 35 at the exit. The 10 missing calls are exactly the open cards' commands: `fdm_health` (T6), `hubrise_status` and `set_pin` (T8), `list_tables` and `list_open_takeaways` (T10), `get_categories`, `get_menu_items`, `get_item_modifiers`, `add_order_line_with_modifiers` and `needs_z_report` (T11).

## Exit steps done

1. Phase gate at the lane head: see above.
2. `docs/android/PARITY.md` pruned (PD242): the 27 `pending:P5-T<n>` lines whose command is CALLED were deleted, so `stale` is now 0. Left: 9 exclusions, 35 `pending:P5b-*` lines and 10 `pending:P5-T<n>` lines of the open cards. A note under the run A table says that the `ui/fdm/` and `ui/scaffold/` rows have not landed, and that until P5-T6 lands the login start-up checks call `fdm_check_now`, `check_printer` and `check_kitchen_printer`.
3. This report, `phases/P5.md` and the exit decisions PD240 to PD249 in `DECISIONS.md`.
4. Freeze: none required at the P5 exit (PD247). No tag, `LINUX_FREEZE.md` unchanged.

## What the next phase must know

- **Start point.** `android/lane-ui` at this report's commit. The continuation run (PD241) runs P5-T6, then T8, then T10 and T11, then T12, with the card texts unchanged. Check every dependency by the presence of its files on the lane, never by branch ancestry. Reset the empty branches `android/P5-P5-T8`, `T10`, `T11` to the lane head; their worktrees (`moyun-pos-wt/P5-P5-T8`, `T10`, `T11`, clean, about 61 MB each) can be removed.
- **Disk first.** At least 15 GB free before two lanes build in parallel; today 14 GB are free. Only the idle lanes' `target` directories may be cleaned, by the orchestrator.
- **Daemon stops.** Another agent's `./gradlew --stop` killed a running gate check twice in this run. Integrators stop daemons only when no other Gradle build runs (PD249).
- **Small follow-up after T6.** A ui-login card switches `GatewayFdmProbe` to `FdmStore.checkNow()` (PD243 item 4); T6 itself may not edit `feature/login/`.
- **Follow-ups** listed in PD244 (session clear on engine loss, `OrderStore.resetLocal` race, QR placeholder size, toast animation, `ui-money` heuristic gaps, small test and style nits).
- **Decision numbers.** The platform lane also used PD201 to PD221; qualify references by lane, and use lane-prefixed numbers from P5b on (PD245).
- **P5b** (Checkout, Reports, Settings, WebshopOrders) starts after the continuation run, as planned in `phases/P5.md`.

## Risks

- **PD204 placeholder tags.** Every real screen keeps `testTag("placeholder_<area>")` on its root because the frozen foundation tests require it; the tag now means "feature root". A shared foundation card should rename it before more views land.
- **PD208 money heuristic.** `ui-money` is a line heuristic with known blind spots (PD244). No violation was found by the reviewers in the merged code; the reviewer stays the authority on every card.
- **Card size.** The view cards (Login was 23 paths and about 3,000 added lines with tests and goldens) are at or above the plan's 600-line guide; Order is split over T11 and T12 for that reason.
- **Chained dependencies.** One blocked card (T6) stalled four more. The continuation run keeps the chain but gates each wave on file presence.
- **Host capacity.** Three lanes on a 16 GB Mac with 14 to 15 GB of free disk is the binding limit of this run, not the code.
- **Unmerged lanes.** The UI lane has not yet been merged with the platform and export lanes; the PD201 to PD219 numbering overlap with the platform lane is a documentation hazard at that merge (append rule keeps both blocks).

## Questions for the user

1. **PD203 fonts:** no font is bundled; the system fonts cover all 439 code points on the image. Agree, or bundle a CJK font?
2. **PD204 tag rename:** may a shared foundation card, alone in its wave at the start of P5b, rename the `placeholder_<area>` tags and let features own sub-routes?
3. **PD211 Tables after login:** login ends on Tables (Vue pushes Order, which bounces to Tables). Agree?
4. **PD219 no option total:** the modifier dialog shows the item price and each option's own price, with no total on the button, because Kotlin may not add amounts. Agree, or should the engine provide a priced preview?
5. **P5-T6 rejection and PD240/PD241:** agree to close this run partial and to run the five open cards in a continuation run before P5b?
6. **PD245 numbering:** agree to lane-prefixed decision numbers from P5b on?

## The SD card question (relayed from the user)

**Is the SQLite database on the SD card? No.** The engine's database is `filesDir/engine/pos.db`, with its backups (`filesDir/engine/backups/`) and `engine.lock`; the installed configuration bundle is `filesDir/provisioning/bundle` and the logs `filesDir/logs` (`provisioning/ProvisioningPaths.kt:39-42`, `crates/pos-engine/src/engine.rs` `EnginePaths::under`). All of this is the app's private internal storage, excluded from any backup (`allowBackup` false, an exclude-everything `data_extraction_rules.xml`); the platform lane also pins it there (no `installLocation`, no storage permission, platform PD220). This is deliberate: a fiscal journal on removable media could be pulled out mid-write, swapped, corrupted or read by another app, and the SCE rules want the journal under the till's control. The only external path in the app is a debug-only inbox. Four reviewers checked this in the code.

**What the card is for.** Only exports: the manager's SAF export of the database backups, logs and SPF files (P6, DECISIONS Q16 and Q17; the S4 spike used an emulated SD card as a stand-in for the USB stick). One export copies the 7 newest backups plus the logs, so an export set is roughly seven database sizes.

**How big?** Not measured yet, so this is an estimate: one till writes text rows, so the database should grow by megabytes per year, not gigabytes, and a good-quality 16 or 32 GB card (class A1, from a known brand) should hold years of exports. Two caveats. The reviewers' own rough guesses ranged from a few MB to over 1 GB a year, so treat the figure as unconfirmed. And the engine keeps every post-Z backup on internal storage (platform PD219), so internal storage is what grows over time, not the card. Research card SD-1 (no code) will measure `pos.db` bytes per ticket and per Z, multiply by the retention period, and check the D3 Pro's free internal storage and the largest card it accepts, which has not been checked on real hardware (PD248).
