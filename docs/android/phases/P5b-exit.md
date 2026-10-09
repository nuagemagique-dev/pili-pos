# P5b exit report: UI lane, completion of run B

Phase cards: `docs/android/phases/P5b.md`. Lane branch `android/lane-ui` (worktree `moyun-pos-wt/lane-ui`), lane base `B` = `ba57adf06191a1f386863f98f24930b6f5aa543c` (`android/int`), run start `S` = `e2fc5064956fe17bbe3fd90253115ac729220ab3`. Written by the P5b planner on 2026-10-09 while the user was away. Nothing was pushed, no tag was created, no GitHub Actions run, no live module2, HubRise, Scaleway or SPF call, no `declare.sh`, no release key. main (`96728e1`) and `cert/v1.0` (`d6210ca`) are untouched.

## Result in one paragraph

The run closes **complete**. All twelve cards are merged with an APPROVE verdict and no escalation: the FDM store, banner and veil (T1), the delivery store, ticket medium components and cash display helpers (T2), the webshop orders store and order chime (T5), the Reports and Settings view models (T3, T4), the scaffold, header, status lights and the Tables view (T6), the Order view in two parts (T7, T12), and the Checkout (T8), Reports (T9), Settings with HubRise QR and typed-code OAuth (T10) and WebshopOrders (T11) screens. Every Vue command now has a Kotlin call site or a recorded exclusion: `check_ui_parity.py --strict` prints `called 73, excluded 9, pending 0, stale 0, missing 0 (82 commands)`. The lane head builds, passes the phase gate and can be merged into `android/int`. Nothing in the run touched Rust, `:pos-core`, the foundation or a frozen Linux path.

## Branch state

| Item | Value |
|------|-------|
| lane head | the commit that adds this file, on top of `47afe44` (`docs(android): record the P5b wave 4 integration`) |
| last code merge | `deabf9a` (`chore(android): merge P5b-T12`) |
| commits in this run | 61 from `S` to `47afe44` (12 merges, 42 card commits, 5 planner or integrator docs commits, 2 integrator fix commits), plus this report |
| diff vs `B` | 318 paths, all inside `lane_paths.py --regex ui` plus `DECISIONS.md` (lane scope check prints nothing) |
| diff vs `S` | 223 paths |
| frozen refs | main `96728e1325f5e11d69b7571d17280e3027974e06`, `cert/v1.0` `d6210caabb32ca09f54be0f15410ce8447960be2`, `linux-v1.1.0-rc1` -> `65b567fdd2497277300340979c2c61ff17abc918`, no `linux-v1.1.0` tag |

## Per-card status

| Card | Wave | Verdict | Card commits | Merge commit |
|------|------|---------|--------------|--------------|
| P5b-T1 FDM store, banner, veil, login probe | 1 | APPROVE | `9800aa5`, `272d6d7`, `c907387`, `2ccf504` | `feebc02` |
| P5b-T2 delivery store, medium picker, e-mail keypad, outcome panel, `CashDisplay` | 1 | APPROVE | `fac8372`, `57b429b`, `bba7f6a`, `45fcc24` | `1a1fb7e` |
| P5b-T5 webshop orders store, order chime, session and order store follow-ups | 1 | APPROVE | `1e5d002`, `c18740b`, `4a416df` | `d466401` |
| P5b-T3 Reports view model | 1 | APPROVE | `7de2a63`, `a687092` | `580f4bc` |
| P5b-T4 Settings and HubRise view models | 1 | APPROVE | `1ab3df5`, `708b4a3` | `43da42a` |
| P5b-T6 scaffold, header, status lights, Tables | 2 | APPROVE | `18f5ff3`, `8d8cf45`, `7ab32ea`, `5565d2b`, `8bb842a` | `4514b96` |
| P5b-T7 Order part 1 (menu, modifiers, Z reminder) | 3 | APPROVE (review round 3) | `9bfc3fe`, `013199f`, `1ad9c5c`, `30ec638`, `f43782b` | `e066695` |
| P5b-T8 Checkout | 3 | APPROVE | `40dc985`, `73e581e`, `aa5a99a`, `745a39c`, `10e3327` | `e1c4b07` |
| P5b-T9 Reports screen | 3 | APPROVE | `7dcfa1f`, `6ffdaaf`, `b869454` | `43004e7` |
| P5b-T10 Settings screen, HubRise section and connect dialog | 3 | APPROVE | `14c0b83`, `ca17690`, `c0fa6a8` | `eabf347` |
| P5b-T11 WebshopOrders screen | 3 | APPROVE | `e48a7de`, `20c7234`, `3e507da` | `4fbd4d5` |
| P5b-T12 Order part 2 (full cart) and parity close | 4 | APPROVE | `2cd491b`, `3fe9e04`, `7eb417a` | `deabf9a` |

Planner and integrator commits: `8b216e1` (phase cards and UI-PD301 to UI-PD318), `371e2ff` (wave 1 fix: two P5b-T3 tests aligned with the P5b-T5 rule that the session forgets the user when the engine goes away, test files only), `eec214f`, `31bb9ad`, `cd770b9`, `47afe44` (wave 1 to 4 integration records), `4239a90` (wave 3 fix: the 45 stale `pending:` lines of `PARITY.md` deleted once every view called its commands), and the commit that adds this report and the exit decisions.

No merge had a conflict. Rework: P5b-T7 was approved at its third review; its two `fix(android)` commits come from those rounds, within the limit of two rework rounds. No card was rejected or escalated.

## Gate evidence

Allow regex everywhere: `$(lane_paths.py --regex ui)|^docs/android/DECISIONS[.]md$`.

Static phase checks, rerun by the planner at `47afe44` (all pass):

| Check | Result |
|-------|--------|
| clean tree, main, `cert/v1.0`, `linux-v1.1.0-rc1`, no `linux-v1.1.0` | PASS |
| `check_linux_frozen.sh` | `frozen against linux-v1.1.0-rc1 (65b567f): 16 paths, clean` |
| frozen-path `git diff --quiet $B -- ...` (Cargo, ui, crates, deploy, android/rust, android/pos-core, build files, manifest, `src/debug`, `LANES.md`, `.github`) | empty |
| lane scope (`git diff --name-only $B...HEAD` minus the allow regex) | empty |
| `lane_paths.py --check` | `LANES | total | PASS | 3695 files, 73 rules, 66 probes` |
| `kotlin_test_inventory.py --base B` | `base 318, head 1514, removed 0` |
| `kotlin_test_inventory.py --base S` | `base 616, head 1514, removed 0` |
| `check_ui_parity.py --strict` | `called 73, excluded 9, pending 0, stale 0, missing 0 (82 commands)` |

Exit gate, rerun by the planner at `47afe44` (`scripts/android/p5_gate.sh --exit --parity-strict --base $B --allow "$UI"`, native libraries cloned from the `lane-plat` target, the Rust inputs being identical): `GATE | total | PASS | 0 failed (35 checks)`, exit 0, with 1 SKIP (`frozen-provisional`, retired).

| Check | Result |
|-------|--------|
| hygiene-paths (318 paths, all allowed), emdash, artifacts, subjects (91 Conventional Commits), root-lock, keys | PASS |
| linux-frozen, freeze-tag | PASS, 16 paths clean against `linux-v1.1.0-rc1` |
| app-only-scope, ffi-boundary, app-manifest (6 permissions) | PASS |
| lanes, vue-strings (481 keys), kotlin-tests (318 to 1514, removed 0) | PASS |
| self-tests: p4-gate 75, heavy 11, p5-gate 193, ui-parity 46, kotlin-tests, lanes, vue-strings | PASS |
| app-lint, app-ktlint, app-detekt | PASS |
| app-unit-tests | PASS, 1471 tests |
| app-screenshots (verifyRoborazziDebug) | PASS |
| app-kover | PASS, LINE 98.2 percent, BRANCH 74.7 percent (floor 80/60) |
| app-assemble-release, app-apk-check | PASS, 6 checks, `324d6d81fc28` |
| plan-gradle (`./gradlew lint ktlintCheck detekt test koverVerifyDebug assembleRelease`) | PASS |
| ui-parity (strict), ui-contracts, ui-money, ui-strings (8 files) | PASS |

Run by the wave 4 integrator at `deabf9a`/`47afe44` (same code): `p5_gate.sh --exit --parity-strict` 35 checks, 0 failed; `p4_gate.sh --fast --app-e2e` 22 checks including `app-e2e` 11 of 11 on the reused `emulator-5554`; `:app:testDebugUnitTest --rerun :app:verifyRoborazziDebug --rerun :app:koverVerifyDebug` BUILD SUCCESSFUL. The emulator was not started by this run and was not stopped.

Root Rust test inventory: `test_inventory.sh --diff` ran once in this run, at the wave 3 integration with 25 GB free (`ui/dist` copied from the `int` worktree, the `ui` tree being identical to `B`): no test disappeared, `ws 1482 >= 1355`. It was not repeated after wave 4 (9.4 GB free, below the 20 GB of UI-PD315); wave 4 changed only Kotlin and docs under `feature/order` and `PARITY.md`, so the Rust inventory is the same.

Not run here, for the merge of the lanes into `android/int` (PD249, UI-PD315): `p4_gate.sh --exit --root --android` and the Linux SHA-1 reproduction (`p4_gate.sh --fast --linux-repro`, expected `9d230ef1bba90a4edde0157bcb107e4bb4d23d6e`). The UI lane changes no Rust and no frozen path, which the frozen-path diff proves.

Gate incidents in the run, none caused by lane code: sibling `./gradlew --stop` calls killed two gate runs (rerun unchanged, PASS); a full host disk (ENOSPC in the gate's own temp logs) failed the first review runs of P5b-T8 and P5b-T11 (rerun after the disk recovered, PASS); a timing-sensitive Rust test `pos-ffi lifecycle::cancelled_shutdown_then_shutdown_ends_the_streams` failed once under host load (3 reruns 212/212); `pos-core EndToEndTest.refundOfTheSaleIsSignedAndPrinted` failed once with "printer mock holds 0 job(s)" under load (passes alone and on the rerun); one `dpm set-device-owner` refusal in an e2e run (cleared, rerun PASS); Homebrew's cargo picked by the Gradle JVM when `$HOME/.cargo/bin` is not first on PATH (E0463, then E0514 from its partial output until deleted).

## Figures

| Figure | Base `B` | Run start `S` | Lane head |
|--------|----------|---------------|-----------|
| Kotlin test inventory | 318 | 616 | 1514 (removed 0 against both) |
| `:app` host unit tests | 275 | 573 | 1471 |
| Kover LINE / BRANCH (floor 80/60) | 95.0 / 76.0 | 97.1 / 76.1 | 98.2 / 74.7 |
| goldens | 2 | 18 | 50: 21 components, 5 order, 4 login, 4 reports, 4 settings, 4 webshop, 3 checkout, 3 tables, 2 theme |
| parity | none | called 28, excluded 9, pending 45, stale 0, missing 0 | called 73, excluded 9, pending 0, stale 0, missing 0 |
| parity pages `docs/android/parity/` | none | `login.md` | `login.md`, `tables.md`, `order.md`, `checkout.md`, `reports.md`, `settings.md`, `webshop.md` |

## Exit steps done

1. Phase gate at the lane head: see above.
2. `docs/android/PARITY.md` closed: no `pending:` line left (deleted in `4239a90`, confirmed by P5b-T12 in `7eb417a`), the 9 exclusions unchanged, the run A rows updated.
3. This report and the exit decisions UI-PD319 to UI-PD324 in `DECISIONS.md`. `phases/P5b.md` is the phase file of `8b216e1`, unchanged.
4. Freeze: none required at the P5b exit (UI-PD320, as PD121 and PD247). No tag. `docs/android/LINUX_FREEZE.md` is outside the UI lane's paths and is unchanged; the decision is recorded in `DECISIONS.md` instead.

## What the next phase must know

- **Start point.** `android/lane-ui` at this report's commit, ready to be merged into `android/int` together with the platform lane (`android/lane-plat`) and the export lane. The append rule of `DECISIONS.md` keeps both lanes' blocks.
- **Merge gate on `android/int`.** Repeat `p4_gate.sh --app` with Roborazzi verify and Kover (P5-T2 caveat), `p4_gate.sh --exit --root --android`, the Linux reproduction (`9d230ef1...`), `check_ui_parity.py --strict`, and `FeatureEntriesTest` with the platform lane's real settings rows (`platform/about` 900, `platform/export` 910, `platform/update` 920; until now only `platform.about` was exercised on the UI lane, UI-PD308).
- **Integration-entry placement.** The integrators appended the P5 and P5b wave entries at the end of `DECISIONS.md`, which is inside "Decided by the user". They are decisions taken while the user was away; the append-only rule keeps them where they are, and UI-PD319 says so.
- **Host environment.** Start Gradle with `$HOME/.cargo/bin` first on PATH. Only one gate per host at a time when disk is under 15 GB; free space dropped to 0 GB twice in this run because of other lanes' Rust `target` directories (about 16 GB in `lane-plat`, 14 GB in `X1-X1-T8`). Call `./gradlew --stop` only when no other Gradle build runs.
- **Stale worktrees.** `moyun-pos-wt/P5-P5-T8`, `P5-P5-T10`, `P5-P5-T11` (empty P5 branches, UI-PD301) still exist; the orchestrator may remove them. The P5b card worktrees are removed and their branches kept.
- **Flaky tests owned by the platform and engine lanes.** `pos-ffi` `lifecycle::cancelled_shutdown_then_shutdown_ends_the_streams` and `pos-core` `EndToEndTest.refundOfTheSaleIsSignedAndPrinted` fail under host load; they should be made deterministic before the `android/int` gate relies on them.
- **Follow-ups for a later UI polish card** (UI-PD322, none blocking).
- **P7 device checks** that the host tests cannot prove: the order chime uses `USAGE_NOTIFICATION` (a kiosk's notification volume or Do Not Disturb can silence it; Vue plays on the media output), the FDM banner shows on the login screen too, the status lights against a real Epson and FDM.

## Risks

- **Display arithmetic in Kotlin (UI-PD306).** `ui/format/CashDisplay.kt` holds four integer functions (change, equal shares, quick amounts, sum for the refund preview). Every signed amount, the 5-cent rounding and the refund amount stay in the engine; the gate `ui-money` exempts only the marked lines of that file. The user has not yet approved this exception.
- **Checkout recompute window.** The checkout amounts are recomputed asynchronously; a validate tap within milliseconds of a method or part change can show a change off by up to 2 cents. The engine still settles and rounds the sale, so nothing fiscal is wrong, but the display can be briefly stale (UI-PD322 item 1).
- **Races of the same class as the fixed `OrderStore` one** remain in four places (UI-PD322 items 2 to 5): small windows, no fiscal impact, the engine refuses commands after a lock.
- **`placeholder_<area>` tags (UI-PD310).** Every real screen still carries the foundation's placeholder tag on its root; renaming needs a foundation card.
- **Card size.** The view cards were well above the plan's 600-line guide (2,300 to 4,500 added lines per card with tests and goldens); review quality held, but the merge into `android/int` is large (318 paths).
- **Unmerged lanes.** The UI lane has never been built together with the platform lane's settings rows and manifest; the first `android/int` gate is the first time.

## Questions for the user

1. **UI-PD306 display arithmetic.** Accept the four `CashDisplay` functions, or move them into engine helpers with the next engine change (a re-freeze)?
2. **UI-PD309 channel logos.** Android shows Deliveroo, Uber Eats and Takeaway as text chips, not logos. Agree?
3. **UI-PD310 placeholder tags.** May a foundation card rename `placeholder_<area>` to a feature-root tag after the lanes merge?
4. **PD203 fonts.** No font is bundled; the system fonts cover all the code points used. Agree, or bundle a CJK font?
5. **PD211 Tables after login.** Login ends on Tables. Agree?
6. **PD219 no option total.** The modifier dialog shows the item price and each option's price, with no total. Agree, or should the engine provide a priced preview?
7. **UI-PD301 one run.** The P5 completion was folded into P5b. Agree after the fact?
8. **UI-PD321 accepted deviations from Vue** (list in `DECISIONS.md`). In particular: Settings shows the employees panel to managers only (a cashier changes the own PIN from the header, as in Vue's header), "New takeaway" registers the current order from every screen, and reject and refund on the webshop board ask for confirmation. Agree?

## The SD card question (relayed from the user)

**Will the SQLite database be stored on the SD card? No.** The engine's database is `filesDir/engine/pos.db`, with its backups (`filesDir/engine/backups/`) and `engine.lock`; the installed configuration bundle is `filesDir/provisioning/bundle` and the logs `filesDir/logs` (`provisioning/ProvisioningPaths.kt`, `EnginePaths::under` in the engine). All of this is the app's private internal storage, excluded from any backup (`allowBackup="false"`, an exclude-everything `data_extraction_rules.xml`, platform lane PD220). That is deliberate: a fiscal journal on removable media could be pulled out mid-write, swapped, corrupted or read by another app, and the SCE rules want the journal under the till's control.

**What the card is for.** Only exports: the manager's SAF export of the database backups, logs and SPF files (P6). One export copies the 7 newest backups plus the logs, so an export set is about seven database sizes.

**How big?** Not measured yet, so this is an estimate, not a figure: one till writes text rows, so the database should grow by megabytes per year rather than gigabytes, and a good-quality 16 or 32 GB card (class A1, known brand) should hold years of exports. Larger cards bring nothing for this use. What grows over time is internal storage, not the card, because the engine keeps every post-Z backup there (platform PD219). Research card SD-1 (no code, outside this lane) still has to measure `pos.db` bytes per ticket and per Z, the D3 Pro's free internal storage and the largest card it accepts. This run added no measurement (UI-PD318).
