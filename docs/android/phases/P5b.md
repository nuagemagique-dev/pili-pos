# P5b: UI port, completion of run B (UI lane)

Phase P5b of the Android port of moyun-pos (pili-pos) to the Sunmi D3 Pro (plan `~/.claude/plans/i-d-like-you-to-vast-koala.md`, section "P5: UI port (two runs)", the invariants and the orchestration protocol). This run is the **UI lane**: it runs at the same time as the platform lane (`android/lane-plat`) on its own integration branch `android/lane-ui`, and both lanes are merged into `android/int` afterwards. It finishes run B of the plan: the five P5 cards that delivered nothing (P5-T6 FDM, P5-T8 scaffold, P5-T10 Tables, P5-T11 and P5-T12 Order, see `P5-exit.md`) and the views Checkout, Reports, Settings (with the HubRise QR and typed-code OAuth) and WebshopOrders. It ends with `check_ui_parity.py --strict` green: every Vue command has a Kotlin call site or a recorded exclusion.

- **Base:** `android/lane-ui` at `e2fc5064956fe17bbe3fd90253115ac729220ab3` (`docs(android): P5 UI lane exit report and exit decisions`), called `S` below. The lane forked from `android/int` at `ba57adf06191a1f386863f98f24930b6f5aa543c`, called `B`; the lane-scope and inventory checks keep `B`. This file and the planner block in `DECISIONS.md` are committed on top of `S` (docs only). main (`96728e1`) and `cert/v1.0` (`d6210ca`) are not touched. Nothing is pushed; no GitHub Actions run; no live module2, HubRise, Scaleway or SPF call; no `declare.sh`; no release key; no tag.
- **Rule of the phase:** the UI lane changes only the paths that `python3 scripts/android/lane_paths.py --regex ui` prints (owners `ui-common` and `ui-<view>`) plus the append-only `docs/android/DECISIONS.md` (planner and integrator only). No foundation file (`engine/`, `i18n/`, `nav/`, `shell/`, `kiosk/`, `provisioning/`, `di/`, `MainActivity`, `PiliApplication`, their tests, build files, version catalog, `verification-metadata.xml`, manifests, `src/debug`, `docs/android/LANES.md`), no platform file (`platform/`, device owner, restrictions, updater, SPF export, watchdog, Docker, release tooling, `AndroidManifest.xml` permissions), no Linux build input, no `android/pos-core`, `android/rust`, `android/smoke`.
- **Settings and the platform screens:** Settings lists the `SettingsEntry` rows that the Hilt set provides and opens `row.route`; it names no platform route and imports nothing from `platform/` (UI-PD308). The route names are the ones the platform lane records in the "Platform routes" table of its `docs/android/LANES.md` (`platform/about` row 900, `platform/export` row 910, `platform/update` row 920). `LANES.md` exists since P4-T4 and is a foundation file, so this lane reads it and does not create or edit it.
- **Inputs read by the planner:** the plan; `phases/P5.md`, `phases/P5-exit.md`; `DECISIONS.md` up to PD249 (UI lane) and the platform lane block; `LANES.md` (both lanes); `APP.md`, `FFI.md`; the code at `S` (`android/app`, `android/pos-core/api/pos-core.api`, `scripts/android/p5_gate.sh`, `check_ui_parity.py`, `lane_paths.py`); the Vue sources `ui/src/views/*.vue`, `components/*.vue`, `stores/*.ts`, `composables/useOrderChime.ts`, `api/ipc.ts`.

## Measured starting point (S = e2fc506)

- `check_linux_frozen.sh`: `frozen against linux-v1.1.0-rc1 (65b567f): 16 paths, clean`. Tags: `linux-v1.1.0-rc1` only (no `linux-v1.1.0`, no `linux-v1.0.0-rc1`).
- `kotlin_test_inventory.py --base B`: `base 318, head 616, removed 0`. `:app` host unit tests 573, Kover LINE 97.1 / BRANCH 76.1 percent (floor 80/60). 18 goldens.
- `check_ui_parity.py`: `called 28, excluded 9, pending 45, stale 0, missing 0 (82 commands)`. Target at the exit with `--strict`: `called 73, excluded 9, pending 0, stale 0, missing 0`.
- Landed run A: `ui/components/` (PiliButton, NumPad, PinPad, PinDialog, PiliDialog, ConfirmDialog, QrCode, VatSummary, AmountDialog with `AmountEntryState`, TrainingBanner, TrainingWatermark, ConfigProblemsBanner), `ui/feedback/` (ToastStore, ToastOverlay, ErrorPresenter), `ui/state/` (PosSettingsStore, SessionStore, IntegrationsStore, OrderStore), `ui/text/`, `ui/di/` (UiCommonEntryPoint, rememberEntryPoint), `ui/format/` (Money, PosDateTime, CartAmounts, ProductNames). Login view done. Not landed: `ui/fdm/`, `ui/scaffold/`, every other view.
- Foundation tests that constrain the views (PD204): `nav/FeatureEntriesTest.kt` (Hilt, plain `ComponentActivity`, engine closed) asserts exactly the eight P4 routes, eight `FeatureEntry` bindings, `placeholder_<area>` displayed on every route and `settings_row_platform.about` displayed on the settings route; `shell/ShellContentTest.kt` builds `LoginEntry()` with no argument.
- Host: 14 GB free on the data volume at planning time, the platform lane building in parallel.

## Cards and waves

| Card | Wave | Depends on (files that must exist on the lane) | Owner | Scope | Needs human |
|------|------|-----------------|-------|-------|-------------|
| P5b-T1 | 1 | none | ui-common (+ ui-login) | `ui/fdm/`: FdmStore, UiEventToasts, FdmBanner + FdmBannerOverlay, FdmBlockingVeil (was P5-T6); login probe over `FdmStore.checkNow()` | no |
| P5b-T2 | 1 | none | ui-common | `ui/state/DeliveryStore`, `ui/components/` TicketMediumPicker, EmailKeypad, DeliveryOutcomePanel; `ui/format/CashDisplay` and its `ui-money` exemption in `p5_gate.sh` | no |
| P5b-T3 | 1 | none | ui-reports | Reports view model and state (day, cash, tickets tabs), no screen | no |
| P5b-T4 | 1 | none | ui-settings | Settings and HubRise view models and state, no screen | no |
| P5b-T5 | 1 | none | ui-common | `ui/state/WebshopOrdersStore`, `ui/feedback/OrderChime` (overlay), SessionStore and OrderStore follow-ups | no |
| P5b-T6 | 2 | `ui/fdm/FdmStore.kt`, `ui/fdm/FdmBlockingVeil.kt`, `ui/state/WebshopOrdersStore.kt` | ui-common + ui-tables | `ui/scaffold/` (HeaderArea, HeaderActions, HeaderViewModel, StatusLights, AppHeader, PosScaffold), `res/drawable/ui_logo.xml`; the Tables view (was P5-T8 and P5-T10) | no |
| P5b-T7 | 3 | `ui/scaffold/PosScaffold.kt` | ui-order | Order part 1: menu, categories, items, free price, modifiers, Z reminder, screen with a minimal cart column (was P5-T11) | no |
| P5b-T8 | 3 | `ui/scaffold/PosScaffold.kt`, `ui/state/DeliveryStore.kt`, `ui/components/TicketMediumPicker.kt`, `ui/components/DeliveryOutcomePanel.kt`, `ui/format/CashDisplay.kt` | ui-checkout | Checkout view, complete | no |
| P5b-T9 | 3 | `ui/scaffold/PosScaffold.kt`, `feature/reports/ReportsViewModel.kt`, the T2 files | ui-reports | Reports screen on the T3 view model | no |
| P5b-T10 | 3 | `ui/scaffold/PosScaffold.kt`, `feature/settings/SettingsViewModel.kt`, `feature/settings/HubriseViewModel.kt`, `ui/state/DeliveryStore.kt` | ui-settings | Settings screen, HubRise section and connect dialog | no |
| P5b-T11 | 3 | `ui/scaffold/PosScaffold.kt`, `ui/state/WebshopOrdersStore.kt` | ui-webshop | WebshopOrders screen on the T5 store | no |
| P5b-T12 | 4 | `feature/order/OrderScreen.kt`, `feature/order/OrderCartColumn.kt`, and every wave 3 card merged | ui-order + ui-common (PARITY.md) | Order part 2: the full cart (was P5-T12); parity close with `--strict` | no |

Twelve cards, four waves, no card needs a human, no card uses the emulator or Docker. Paths in the table are relative to `android/app/src/main/kotlin/dev/nuagemagique/pilipos/`.

**Merge order inside a wave:** wave 1: T1, T2, T5, T3, T4. Wave 2: T6. Wave 3: T7, T8, T9, T10, T11. Wave 4: T12. **Start order** (UI-PD313): the same order, a card starting only while at least 11 GB are free; T1 is the critical path (T6, and through it every screen, needs it).

**Mapping to P5:** P5-T6 is P5b-T1; P5-T8 and P5-T10 are P5b-T6; P5-T11 is P5b-T7; P5-T12 is P5b-T12. The P5 pending owners in `PARITY.md` (`pending:P5-T6`, `P5-T8`, `P5-T10`, `P5-T11`) and the `pending:P5b-<view>` lines stay as they are until P5b-T12 deletes them all (UI-PD305).

## Names that later cards rely on

| Card | Files (under `android/app/src/main/kotlin/dev/nuagemagique/pilipos/`) | Public names |
|------|------|------|
| T1 | `ui/fdm/FdmStore.kt`, `ui/fdm/UiEventToasts.kt`, `ui/fdm/FdmBanner.kt`, `ui/fdm/FdmBlockingVeil.kt`, `ui/fdm/FdmModule.kt` | `FdmStore` (`state: StateFlow<FdmUiState>`, `suspend fun checkNow(): Boolean`), `FdmUiState`, `FdmBanner`, `FdmBannerOverlay` (order 200), `FdmBlockingVeil(state, onCheckNow, modifier)`, `UiEventToasts` |
| T2 | `ui/state/DeliveryStore.kt`, `ui/components/TicketMediumPicker.kt`, `ui/components/EmailKeypad.kt`, `ui/components/DeliveryOutcomePanel.kt`, `ui/format/CashDisplay.kt` | `DeliveryStore` (`mailStatus`, `request`, `refreshMailStatus()`, `setPaper()`, `setEmail(medium, email)`, `reset()`, `printMissingOriginal(fiscalEventId)`), `TicketMediumPicker`, `EmailKeypad`, `DeliveryOutcomePanel`, `CashDisplay.changeCents`, `equalShares`, `quickAmountsCents`, `sumCents` |
| T3 | `feature/reports/ReportsViewModel.kt`, `ReportsState.kt`, `ReportsEntryPoint.kt` | `ReportsViewModel`, `ReportsState`, `ReportsTab` |
| T4 | `feature/settings/SettingsViewModel.kt`, `SettingsState.kt`, `HubriseViewModel.kt`, `HubriseState.kt`, `SettingsFeatureEntryPoint.kt` | `SettingsViewModel`, `HubriseViewModel` |
| T5 | `ui/state/WebshopOrdersStore.kt`, `ui/feedback/OrderChime.kt`, `ui/feedback/OrderChimeModule.kt` | `WebshopOrdersStore` (`orders`, `pendingCount`, the actions), `OrderChimePlayer`, `OrderChimeOverlay` (order 10) |
| T6 | `ui/scaffold/HeaderArea.kt`, `HeaderActions.kt`, `HeaderViewModel.kt`, `StatusLights.kt`, `AppHeader.kt`, `PosScaffold.kt`, `ScaffoldEntryPoint.kt`; `feature/tables/TablesViewModel.kt`, `TablesScreen.kt`, `TablesEntryPoint.kt` | `HeaderArea` (`TABLES`, `ORDER`, `CHECKOUT`, `REPORTS`, `SETTINGS`, `WEBSHOP`, `isSales`), `PosScaffold(area, navigator, modifier, overlay, content)` |
| T7 | `feature/order/MenuViewModel.kt`, `OrderScreen.kt`, `OrderEntryPoint.kt`, `OrderCartColumn.kt`, `CategoryTabs.kt`, `MenuGrid.kt`, `ModifierDialog.kt` | `OrderCartColumn(state, actions, modifier)`: the seam T12 fills |

## Planner decisions (decided while the user was away)

The planner appended this block to `docs/android/DECISIONS.md` at the end of the section "Decided while the user was away (review)" in the same commit as this file. Numbers carry the lane prefix (PD245).

- **UI-PD301 One run instead of two.** PD241 planned a separate "P5 completion" run for P5-T6, T8, T10, T11 and T12 before P5b. This run folds them into P5b so that the lane reaches parity in one run of twelve cards in four waves (mapping above). The empty branches `android/P5-P5-T8`, `T10`, `T11` and their worktrees are not reused; the orchestrator removes the worktrees.
- **UI-PD302 State before screens.** To use wave 1 while the critical path (FDM, then scaffold) runs, the Reports and Settings view models and the webshop store are built in wave 1 without a screen; their screens come in wave 3, by the same owner. A state card leaves its feature's placeholder screen and entry untouched.
- **UI-PD303 More shared state in ui-common (amends PD206).** `stores/delivery.ts` (used by checkout, refund, copy and settings) becomes `ui/state/DeliveryStore`, and `stores/webshopOrders.ts` (header badge, global chime, webshop view) becomes `ui/state/WebshopOrdersStore`. `TicketMediumPicker`, `EmailKeypad` and `DeliveryOutcomePanel` (checkout and reports) become ui-common components. The menu store stays in `feature/order`; Settings does not reload the menu, the order screen reloads its categories when the language changes or `PosEvent.CatalogUpdated` or `Resync` arrives.
- **UI-PD304 The order chime is an overlay.** `useOrderChime()` (App.vue) becomes `OrderChimeOverlay`, a `ShellOverlay` (order 10) that draws nothing and plays C5 then E5 (150 ms each, sine, fading) through `android.media.AudioTrack` with generated samples, behind an `OrderChimePlayer` interface (a fake in tests), each time `WebshopOrdersStore.pendingCount` goes up. No sound file, no new dependency.
- **UI-PD305 Single writers in this run.** `docs/android/PARITY.md`: P5b-T12 only; the other cards leave their `pending:` lines, which the checker reports as stale (a warning, an error only with `--strict`). `strings_ui_common.xml` (three locales): P5b-T2 in wave 1 and P5b-T6 in wave 2. `scripts/android/p5_gate.sh` and `test_p5_gate.sh`: P5b-T2 only. `ui/di/UiEntryPoints.kt`: nobody; every package declares its own `@EntryPoint`. `docs/android/DECISIONS.md`: planner and integrator only; a worker reports its deviations in its card report.
- **UI-PD306 Checkout display arithmetic (question for the user).** The Vue checkout and refund compute four amounts for which the engine offers no helper: the change from the cash handed over, the equal shares of a split, the quick amounts above the cash due, and the refund preview total (sum of the selected lines). The engine is frozen (any change means a re-freeze and an escalation). The planner allows exactly one file, `ui/format/CashDisplay.kt`, with four integer functions ported from `CheckoutView.vue` and `ReportsView.vue`: `changeCents`, `equalShares`, `quickAmountsCents`, `sumCents`. No rounding, no floating point, each arithmetic line ending with `// money-display-only`; `ui-money` exempts those lines in that file only (P5b-T2 extends the gate and its self-test). The 5-cent cash rounding and every signed amount stay in the engine: the rest to pay and the rounded cash due come from `EngineHelpers.settlePreview` and `cashDueCents`, the sale from `checkout`, the refund from `refund_checkout`, and the engine refuses any inconsistent set of parts. Alternatives for the user: engine helpers in a later engine change (re-freeze), or no change and split display.
- **UI-PD307 Refund preview.** The refund confirmation shows the selected lines (each line amount is the single `vatSplit` row of that line, as `CartAmounts.lineAmount`), the VAT summary of the selection, and a preview total: `CashDisplay.sumCents` of the line totals, then `EngineHelpers.cashDueCents(total, [cash part])` for the cash rounding. When a selected line has VAT code X (left out by `vatSplit`), no preview total is shown; the engine's refund result gives the amount.
- **UI-PD308 Settings reaches the platform screens through rows only.** The settings screen lists `NavRegistry.settingsEntries` (sorted by order, then id), hides `managerOnly` rows from non-managers, keeps `testTag("settings_row_<id>")` on every row (the foundation test asserts `settings_row_platform.about` with the engine closed), and calls `navigator.navigate(row.route)`. No import from `dev.nuagemagique.pilipos.platform`, no platform route literal. The agreed names are the platform lane's "Platform routes" table in its `LANES.md`; until that lane merges, only `platform.about` appears.
- **UI-PD309 No third-party platform logos.** `WebshopOrdersView.vue` shows the Deliveroo, Uber Eats and Takeaway logos. Android shows the channel as a text chip. This keeps trademark artwork out of a declared APK and the dossier brand-neutral; `ui-webshop` owns no drawable path.
- **UI-PD310 The PD204 tags stay.** No foundation card runs in this lane (the foundation is frozen for the lanes and question 2 of `P5-exit.md` is open). Every screen keeps `testTag("placeholder_<area>")` on its root; sub-screens are in-screen state or dialogs.
- **UI-PD311 Follow-ups taken in this run.** P5b-T1 replaces the login `GatewayFdmProbe` by a probe over `FdmStore.checkNow()` (PD243 item 4) and corrects `docs/android/parity/login.md` (an unanswered FDM gives `check_not_configured`, PD244). P5b-T5 makes `SessionStore` forget the user when `gateway.ready` turns false, and makes `OrderStore.resetLocal()` safe against a refresh in flight (PD244). The other PD244 items (21-module QR placeholder, toast slide-in, `ui-money` heuristic gaps) stay open.
- **UI-PD312 Icons.** Icons come from Material Icons Extended (`Icons.*`, already a dependency). The header logo `res/drawable/ui_logo.xml` is a conversion of the paths of the existing `ui/src/components/PiliLogo.vue`, nothing drawn by hand.
- **UI-PD313 Disk staging.** 14 GB were free at planning time; a card worktree takes about 3 GB with its Gradle outputs. The orchestrator starts a card only with at least 11 GB free, in the start order above, and deletes each merged card's `target` and `android/app/build` at once. A worker below 8 GB free waits up to 30 minutes (polling every 5 minutes) before it stops and reports; in P5, P5-T6 stopped at once and stalled four cards.
- **UI-PD314 Dependencies by file presence** (PD241 rule). Before a wave the orchestrator checks with `git ls-tree android/lane-ui -- <path>` the files listed in the card table, never branch ancestry.
- **UI-PD315 Gates.** Cards run `p5_gate.sh` (PD216). The strict parity runs in P5b-T12 and at the phase exit: `check_ui_parity.py --strict` must print `called 73, excluded 9, pending 0, stale 0, missing 0`. The root Rust test inventory (`test_inventory.sh --diff`) and the Linux SHA-1 reproduction run once at the exit when at least 20 GB are free; otherwise they are recorded as not run and done at the lanes' merge into `android/int`, as at the P5 exit (the UI lane changes no Rust and no frozen path, which the frozen-path diff proves).
- **UI-PD316 No emulator in the cards** (PD217). The integrator runs `p4_gate.sh --fast --app-e2e` once after wave 4, reusing a running emulator and never stopping one it did not start.
- **UI-PD317 Webshop amounts.** The webshop screen shows the amounts of the engine's webshop records through `formatCents` only; a webshop refund is `webshop_refund_order` with the engine's own amounts. Time arithmetic (preparation minutes, delays, ages) is not money and is allowed, in `Europe/Brussels`.
- **UI-PD318 The SD card question, relayed again.** The answer of PD248 stands (section at the end of this file); this run adds no measurement.

## Shared environment (all cards)

- Worker worktree `$WT` = `/Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/P5b-P5b-Tn` on branch `android/P5b-P5b-Tn` from `android/lane-ui`. The worktrees `moyun-pos-wt/lane-ui` (integrator) and `moyun-pos-wt/int` are read only for workers.
- Every shell: `export PATH="$HOME/.cargo/bin:$PATH"; unset CARGO_TARGET_DIR; export RUSTC_WRAPPER=/opt/homebrew/bin/sccache CARGO_INCREMENTAL=0 SCCACHE_CACHE_SIZE=25G CARGO_BUILD_JOBS=3 ANDROID_HOME=/Users/brunomoyse/Library/Android/sdk ANDROID_NDK_HOME=/Users/brunomoyse/Library/Android/sdk/ndk/27.1.12297006 PILI_GRADLE_OFFLINE=1 JAVA_HOME=$(/usr/libexec/java_home -v 17)`.
- Warm start before the first build: `mkdir -p $WT/target/android-rust && cp -cR /Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/int/target/android-rust/aarch64-linux-android $WT/target/android-rust/` (an APFS clone).
- Gradle from `$WT/android`, always through the slots: `../scripts/android/heavy.sh ./gradlew --no-daemon --console=plain --offline -Dorg.gradle.jvmargs='-Xmx2g -XX:MaxMetaspaceSize=1g -Dfile.encoding=UTF-8' <tasks>`; one Gradle build per agent at a time; retry once with `-Xmx3g` after an OutOfMemoryError; never `./gradlew --stop`. Tool calls time out after 10 minutes: long builds run in the background and are polled. Goldens: `:app:recordRoborazziDebug --tests '<pattern>'`, then `:app:verifyRoborazziDebug`.
- Disk: UI-PD313. No emulator, no Docker, no new Gradle dependency (a card that needs one stops and reports).

## Rules (reviewer checks on every card)

- Allowed paths per card (`--allow`, section "Card summaries"); nothing outside `lane_paths.py --regex ui`.
- Freeze: `check_linux_frozen.sh` exit 0 (`linux-v1.1.0-rc1`, 16 paths clean); no Rust and no `:pos-core` change.
- Engine and FFI hygiene: engine calls only inside `EngineGateway.call { engine -> ... }` (that is where `check_ui_parity.py` finds them), helpers only through `EngineHelpers`, events only from `EngineEvents.events`; `ffi-boundary` PASS.
- No fiscal rule and no money arithmetic in Kotlin (PD208, UI-PD306): no product, quotient, remainder, sum or difference of cents outside the marked lines of `ui/format/Money.kt` and `ui/format/CashDisplay.kt`; totals come from engine records or helpers; the 5-cent rounding only from `settlePreview`/`cashDueCents`.
- Contracts: no `hiltViewModel`; dependencies from the package's own `@EntryPoint` through `rememberEntryPoint<T>()`, view models built with `viewModel { }`; `FeatureEntry` constructors and routes unchanged, no new route or entry; `placeholder_<area>` on each screen root; screens render with the engine closed and never navigate at start (PD204, PD205); theme tokens only (no `Color(0x`, no numeric `.sp` outside `ui/theme/`); icons from `Icons.*` only (UI-PD312); every time shown in `Europe/Brussels` through `ui/format/PosDateTime`.
- Tests never drop: `kotlin-tests` removed 0 (a card that changes a component keeps every earlier test function, updating bodies); Kover floor 80/60; goldens recorded and verified at `w1920dp-h1080dp-mdpi` (PD215), at most four per card unless the brief says otherwise.
- Strings: Vue texts through `vueText(key)` / `MessageCatalog`; hand-written strings only in the card's own `strings_ui_<area>.xml`, with `values/` (fr), `values-en/` and `values-zh/` and the `ui_<area>_` prefix.
- No secrets, keystores, `local.properties`, `.env` or build outputs committed. No push, tag, GitHub Actions, live call, `declare.sh` or release key.
- No em-dash anywhere; Conventional Commits (`feat(android): ...`, `test(android): ...`, `docs(android): ...`, merge commits `chore(android): merge P5b-Tn`); docs in English; no client named.

### Gates

- Every commit: `scripts/android/p5_gate.sh --fast --base android/lane-ui --allow '<card regex>'`.
- Last commit: `scripts/android/p5_gate.sh --base android/lane-ui --allow '<card regex>'`, `python3 scripts/android/check_ui_parity.py` (the card's commands CALLED at the expected files), `python3 scripts/android/kotlin_test_inventory.py --base android/lane-ui` (removed 0), `bash scripts/android/check_linux_frozen.sh`, `python3 scripts/android/lane_paths.py --check`.
- P5b-T12 adds `python3 scripts/android/check_ui_parity.py --strict` and `scripts/android/p5_gate.sh --parity-strict --base android/lane-ui --allow '<card regex>'`.
- Reviewer of an already merged card: the same gate with `--base <fork point>` (PD116).

## Integration procedure

1. Before each wave: `git -C moyun-pos-wt/lane-ui status` clean; the dependency files of the wave's cards present on `android/lane-ui` (UI-PD314); free disk per UI-PD313.
2. Merge in the order listed per wave with `git merge --no-ff -m "chore(android): merge P5b-Tn"` into `android/lane-ui`.
3. After each wave: the phase gate below (without the exit-only lines); `(cd android && ./gradlew --stop)` only for a daemon this run started and only when no other Gradle build runs (PD249); delete the merged worktrees' `target` and `android/app/build`.
4. An append conflict in `docs/android/DECISIONS.md` keeps both blocks. Any other conflict is a REWORK of the later card.
5. REWORK at most twice per card; a third failure, a freeze violation or two escalations stop the run. With the user away, the planner decides and records in `DECISIONS.md` under "Decided while the user was away (review)" with `UI-PD3nn` numbers.

## Phase gate (integrator, worktree moyun-pos-wt/lane-ui)

```bash
cd /Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/lane-ui
export ANDROID_HOME=/Users/brunomoyse/Library/Android/sdk ANDROID_NDK_HOME=/Users/brunomoyse/Library/Android/sdk/ndk/27.1.12297006 JAVA_HOME=$(/usr/libexec/java_home -v 17) RUSTC_WRAPPER=/opt/homebrew/bin/sccache CARGO_INCREMENTAL=0 SCCACHE_CACHE_SIZE=25G PILI_GRADLE_OFFLINE=1
B=ba57adf06191a1f386863f98f24930b6f5aa543c
S=e2fc5064956fe17bbe3fd90253115ac729220ab3
UI="$(python3 scripts/android/lane_paths.py --regex ui)|^docs/android/DECISIONS[.]md$"
test -z "$(git status --porcelain)"
test "$(git rev-parse main)" = 96728e1325f5e11d69b7571d17280e3027974e06
test "$(git rev-parse cert/v1.0)" = d6210caabb32ca09f54be0f15410ce8447960be2
test "$(git rev-parse 'linux-v1.1.0-rc1^{commit}')" = 65b567fdd2497277300340979c2c61ff17abc918
test -z "$(git tag -l linux-v1.1.0)"
bash scripts/android/check_linux_frozen.sh
git diff --quiet $B -- Cargo.toml Cargo.lock rust-toolchain.toml src-tauri ui crates deploy scripts/build_and_hash.sh scripts/check_repro.sh docs/android/baseline docs/SUBMISSION_ARTIFACTS android/rust android/smoke android/docker android/pos-core android/settings.gradle.kts android/build.gradle.kts android/gradle.properties android/gradle android/config android/app/build.gradle.kts android/app/proguard-rules.pro android/app/src/main/AndroidManifest.xml android/app/src/debug docs/android/LANES.md .github
test -z "$(git diff --name-only $B...HEAD | grep -Ev "$UI")"                 # lane scope
python3 scripts/android/lane_paths.py --check
python3 scripts/android/kotlin_test_inventory.py --base $B                    # removed 0
python3 scripts/android/kotlin_test_inventory.py --base $S                    # removed 0 in this run
python3 scripts/android/check_ui_parity.py                                    # after waves 1 to 3
scripts/android/p5_gate.sh --base $B --allow "$UI"                            # after waves 1 to 3
python3 scripts/android/check_ui_parity.py --strict                           # after wave 4
scripts/android/p5_gate.sh --exit --parity-strict --base $B --allow "$UI"     # after wave 4
scripts/android/p4_gate.sh --fast --app-e2e --base $B                         # after wave 4, once (UI-PD316)
(cd android && ../scripts/android/heavy.sh ./gradlew --no-daemon --console=plain --offline :app:testDebugUnitTest --rerun :app:verifyRoborazziDebug --rerun :app:koverVerifyDebug)   # after wave 4
bash scripts/android/test_inventory.sh --diff                                 # after wave 4, with >= 20 GB free (UI-PD315)
scripts/android/emulator.sh stop                                              # only if this run started it
```

## Exit procedure (planner, after wave 4)

1. Phase gate PASS at the lane head: `p5_gate.sh --exit --parity-strict` (contains the plan gate `./gradlew lint ktlintCheck detekt test koverVerifyDebug assembleRelease`), freeze clean, frozen-path diff empty, Kotlin inventory 0 removed against `B` and `S`, lanes, strings drift, `check_ui_parity.py --strict` with `called 73, excluded 9, pending 0, stale 0, missing 0`, the app e2e run with teardown, and the root inventory or its recorded deferral.
2. Write `docs/android/phases/P5b-exit.md`: per-card status and commits, gate evidence, Kover figures, test counts (Kotlin inventory, `:app` unit tests, goldens), parity totals, risks, questions for the user (UI-PD306 display arithmetic, UI-PD309 logos, UI-PD310 tags, PD203 fonts, PD211, PD219, UI-PD301). Commit it with the exit decisions appended to `DECISIONS.md` (docs only).

## Card summaries (allowed regex for `--allow`)

- **P5b-T1 FDM.** `^(android/app/src/(main|test)/kotlin/dev/nuagemagique/pilipos/(ui/fdm|feature/login)/.*|android/app/src/test/screenshots/components/fdm_[a-z0-9_]+[.]png|docs/android/parity/login[.]md)$`
- **P5b-T2 delivery and cash display.** `^(android/app/src/(main|test)/kotlin/dev/nuagemagique/pilipos/ui/(state/Delivery[A-Za-z]*[.]kt|components/(TicketMediumPicker|EmailKeypad|DeliveryOutcome)[A-Za-z]*[.]kt|format/CashDisplay[A-Za-z]*[.]kt)|android/app/src/test/screenshots/components/(delivery|email|medium)_[a-z0-9_]+[.]png|android/app/src/main/res/values(-en|-zh)?/strings_ui_common[.]xml|scripts/android/(p5_gate|test_p5_gate)[.]sh)$`
- **P5b-T3 Reports state.** `^android/app/src/(main|test)/kotlin/dev/nuagemagique/pilipos/feature/reports/.*$`
- **P5b-T4 Settings and HubRise state.** `^android/app/src/(main|test)/kotlin/dev/nuagemagique/pilipos/feature/settings/.*$`
- **P5b-T5 webshop store and chime.** `^android/app/src/(main|test)/kotlin/dev/nuagemagique/pilipos/ui/(state/(WebshopOrdersStore|SessionStore|OrderStore)[A-Za-z]*[.]kt|feedback/OrderChime[A-Za-z]*[.]kt)$`
- **P5b-T6 scaffold and Tables.** `^(android/app/src/(main|test)/kotlin/dev/nuagemagique/pilipos/(ui/scaffold|feature/tables)/.*|android/app/src/test/screenshots/(components/(header|scaffold|status)_[a-z0-9_]+[.]png|tables/.*)|android/app/src/main/res/drawable/ui_[a-z0-9_]+[.]xml|android/app/src/main/res/values(-en|-zh)?/strings_ui_(common|tables)[.]xml|docs/android/parity/tables[.]md)$`
- **P5b-T7 Order part 1.** `^(android/app/src/(main|test)/kotlin/dev/nuagemagique/pilipos/feature/order/.*|android/app/src/test/screenshots/order/.*|android/app/src/main/res/values(-en|-zh)?/strings_ui_order[.]xml|docs/android/parity/order[.]md)$`
- **P5b-T8 Checkout.** `^(android/app/src/(main|test)/kotlin/dev/nuagemagique/pilipos/feature/checkout/.*|android/app/src/test/screenshots/checkout/.*|android/app/src/main/res/values(-en|-zh)?/strings_ui_checkout[.]xml|docs/android/parity/checkout[.]md)$`
- **P5b-T9 Reports screen.** `^(android/app/src/(main|test)/kotlin/dev/nuagemagique/pilipos/feature/reports/.*|android/app/src/test/screenshots/reports/.*|android/app/src/main/res/values(-en|-zh)?/strings_ui_reports[.]xml|docs/android/parity/reports[.]md)$`
- **P5b-T10 Settings screen.** `^(android/app/src/(main|test)/kotlin/dev/nuagemagique/pilipos/feature/settings/.*|android/app/src/test/screenshots/settings/.*|android/app/src/main/res/values(-en|-zh)?/strings_ui_settings[.]xml|docs/android/parity/settings[.]md)$`
- **P5b-T11 Webshop screen.** `^(android/app/src/(main|test)/kotlin/dev/nuagemagique/pilipos/feature/webshop/.*|android/app/src/test/screenshots/webshop/.*|android/app/src/main/res/values(-en|-zh)?/strings_ui_webshop[.]xml|docs/android/parity/webshop[.]md)$`
- **P5b-T12 Order part 2 and parity close.** `^(android/app/src/(main|test)/kotlin/dev/nuagemagique/pilipos/feature/order/.*|android/app/src/test/screenshots/order/.*|android/app/src/main/res/values(-en|-zh)?/strings_ui_order[.]xml|docs/android/parity/order[.]md|docs/android/PARITY[.]md)$`

The full task briefs, acceptance criteria and gates are in the orchestrator's task cards (one per card, same ids).

## Commands per card (expected CALLED file at the card's end)

| Card | Commands |
|------|----------|
| T1 | `fdm_health`, `fdm_check_now` (both only in `ui/fdm/FdmStore.kt`) |
| T2 | `print_missing_original` (`ui/state/DeliveryStore.kt`); `check_mail_status` also there |
| T3 | `generate_x_report`, `generate_user_x_report`, `generate_z_report`, `needs_z_report`, `list_z_reports`, `money_in_out`, `open_drawer_fiscal`, `list_copyable_tickets`, `copy_ticket`, `list_refundable_events`, `list_order_lines`, `refund_checkout` (`feature/reports/`) |
| T4 | `create_user`, `deactivate_user`, `set_pin`, `list_ticket_deliveries`, `hubrise_start_oauth_flow`, `hubrise_complete_oauth`, `hubrise_disconnect`, `hubrise_list_channels`, `hubrise_set_channel`, `hubrise_adoption_preview`, `hubrise_adopt_catalog`, `hubrise_sync_catalog` (`feature/settings/`) |
| T5 | `hubrise_list_webshop_orders`, `hubrise_patch_order_status`, `hubrise_set_confirmed_time`, `print_webshop_customer`, `print_webshop_kitchen`, `webshop_accept_order`, `webshop_close_cancelled_order`, `webshop_complete_order`, `webshop_handover_order`, `webshop_refund_order`, `webshop_reject_order` (`ui/state/WebshopOrdersStore.kt`) |
| T6 | `hubrise_status`, `set_pin`, `check_printer`, `check_kitchen_printer` (`ui/scaffold/`); `list_tables`, `list_open_takeaways` (`feature/tables/`) |
| T7 | `get_categories`, `get_menu_items`, `get_item_modifiers`, `add_order_line_with_modifiers`, `needs_z_report` (`feature/order/`) |
| T8 | `checkout` (`feature/checkout/`) |
| T9 to T11 | no new command; the screens use the T3, T4, T5 calls |
| T12 | none new; deletes every `pending:` line of `PARITY.md` |

## The relayed SD card question

The user asked again whether the SQLite database is stored on the SD card of the Sunmi D3 Pro and how big the card should be. The answer of PD248 stands. **No:** the engine's database is `filesDir/engine/pos.db` (with its backups `filesDir/engine/backups/` and `engine.lock`), the installed bundle `filesDir/provisioning/bundle` and the logs `filesDir/logs` (`provisioning/ProvisioningPaths.kt`, `EnginePaths::under`), all in the app's private internal storage, excluded from any backup (`allowBackup` false, an exclude-everything `data_extraction_rules.xml`, platform lane PD220). A fiscal journal on removable media could be pulled out mid-write, swapped, corrupted or read by another app, and the SCE rules want the journal under the till's control. **The card is only an export medium:** the manager's SAF export of the backups, logs and SPF files (P6; `no_mount_physical_media` is deliberately never set). One export copies the 7 newest backups plus the logs, so an export set is about seven database sizes. **Size:** not measured yet; one till writes text rows, so the database should grow by megabytes per year rather than gigabytes, and a good-quality 16 or 32 GB card (class A1, known brand) should hold years of exports. Since the engine keeps every post-Z backup on internal storage (platform PD219), internal storage, not the card, is what grows. Research card SD-1 (no code, outside this lane) still has to measure `pos.db` bytes per ticket and per Z, the D3 Pro's free internal storage and the largest card it accepts.
