# Lane 2: v1 features restaurants use daily (table transfer, payment correction, discount, kitchen notes)

Written by the lane planner (Opus 5.5) on 2026-10-17. Branch `android/lane-v1-features`, worktree `moyun-pos-wt/lane-v1-features`, created from `android/int` at `b3248d2d`. Two other lanes run in parallel (`lane-v1-fixes`, `lane-v11`); the orchestrator merges all three into `android/int`, then makes the rc3 freeze. This lane never pushes, tags, merges or touches another worktree.

Planner decisions: V1F-D1 to V1F-D24, appended at the end of `docs/android/DECISIONS.md` under "## Lane 2-v1-features (2026-10-17)". Workers append their own decisions under the same heading, at the end of the file. This file repeats what a worker needs, so a task can be done from this file plus `CLAUDE.md`.

---

## 1. Goal, scope, non-goals

**Goal.** Four features a restaurant uses every service, in the certified v1.0.0 (both products, Linux `pili-pos` and Android `pili-pos-android`), each signed exactly as the SPF spec and the UC fixtures say, on both UIs (Vue in `ui/`, Compose in `android/`), with tests and both dossiers updated:

| Item | What | Fiscal event | Fixtures replayed |
|---|---|---|---|
| A | Move an open table order to another (free) table | P `signCostCenterChange` (COST_CENTER_CHANGE) | M122 (Basic Set 1.8) |
| B | Correct the payment method of a sale already signed | F `signPaymentCorrection` (PAYMENT_CORRECTION) | M132 (Basic Set 1.8) |
| C | Manager discount at the till: order-level percent / amount / 100 % "Offert", and one free line | `priceChanges` (EVENT scope, or LINE scope for a free line), PUBLIC, inside P / N | UC223, UC225A step 2, UC225B step 2, UC229B (Advanced Set 1.6) through the till path |
| D | Free-text kitchen note per order line | none (never in fiscal data) | none |

**In scope:** engine commands (`crates/pos-engine`), fiscal builders (`crates/pos-fiscal`), schema (`crates/pos-db`), slip printing (`crates/pos-print`), Tauri wrappers (`src-tauri/src/ipc`), UniFFI exports (`android/rust/pos-ffi`), the `:pos-core` API dump, Vue screens and stores, Compose screens and view models, strings (fr, zh, en), Playwright specs, Kotlin unit and Roborazzi tests, engine docs (`docs/android/ENGINE.md`, `FFI.md`, `PARITY.md`, `docs/android/parity/*.md`), `CLAUDE.md` (command counts, migrations, short feature notes), both dossiers (`docs/SUBMISSION_ARTIFACTS`, `docs/android/dossier`), `docs/android/DECISIONS.md` (append only, own heading).

**Non-goals (explicit):**
- Table merge or split (UC240 to UC252), moving to a ROOM / CUSTOMER cost center (UC4101), moving one line or one chair (UC4123): refused or absent. A transfer onto a table that has an open order with lines is refused in v1 (V1F-D2).
- Payment correction of a sale from an earlier booking period (after its Z), of an online order, of a refund, of a sale closed unpaid, or of a card the linked terminal approved; split corrections (several lines at once); CUSTOMER_CREDIT (UC4118). See V1F-D7 to V1F-D11.
- Linking a correction to a terminal payment (an AUTOMATIC target line). The target of a correction is always typed by hand (MANUAL).
- Discount on part of a line's quantity, several order-level discounts stacked, a percent with decimals, a surcharge (positive price change), INTERNAL price changes, discount codes / vouchers (UC260 to UC271).
- Preset note chips, notes on online orders, notes on the customer's VAT ticket or on the pre-bill, a kitchen display.
- Running the device UC scenarios on an emulator, retaking dossier screen captures, rebuilding dossier PDFs, regenerating `docs/SUBMISSION_ARTIFACTS/evidence/uc/` (all orchestrator follow-ups, section 12).
- No new crate dependency: the root `Cargo.toml` and `Cargo.lock` must not change (gate `hygiene-root-lock`).

---

## 2. Spec references (read these, they are authoritative)

POS description v2.2 = `docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md`. FAQ = `docs/20260717_FAQ_PRODUCTEURS_FR.md`.

| Topic | Where |
|---|---|
| Mutations per label (P: signCostCenterChange; F: signPaymentCorrection) | POS v2.2 §2.1, lines ~965 to 990 |
| `costCenter` object, `tb_costCenterType`, `reference` = the session | POS v2.2 §2.2.2 "costCenter", lines ~1225 to 1300 |
| `transfer` object (from / to arrays of `transferItem`), one-to-one / many-to-one / one-to-many, many-to-many forbidden | POS v2.2 lines ~1300 to 1345 |
| `transaction` mandatory for COST_CENTER_CHANGE | POS v2.2 lines ~1356 to 1375 |
| `negQuantityReason` values (COST_CENTER_CHANGE, PRICE_CHANGE, CORRECTION, ...) and the PRICE_CHANGE definition ("ligne artificielle ... sur une ligne déjà comptabilisée") | POS v2.2 lines ~1515 to 1545 |
| `vat.priceChanges`, `priceChange` (groupingId, id, name, scope LINE/EVENT, type PUBLIC/INTERNAL, amount) | POS v2.2 lines ~1590 to 1665 |
| `lineTotal` = Σ(vat.price + Σ priceChange.amount) | POS v2.2 line ~1440 |
| `fdmRefs` / `fdmRef` "le ticket de caisse TVA dont le mode de paiement est corrigé (event F)" | POS v2.2 lines ~1205 to 1212 |
| F event "modification du mode de paiement d'une transaction de vente déjà clôturée"; `paymentLine` fields, `tb_paymentTypes`, `inputMethod`, `amountType` | POS v2.2 lines ~1720 to 1830 |
| Report `payments[].amount[]`: normalAmount, negativeCorrections, positiveCorrections, correctionsCount, totalAmount | POS v2.2 lines ~2072 to 2135 |
| `tb_eventOperation` (COST_CENTER_CHANGE P, PAYMENT_CORRECTION F) | POS v2.2 lines ~2370 to 2390 |
| GraphQL inputs `CostCenterChangeInput`, `TransferInput`, `TransferItemInput`, `PaymentCorrectionInput` | POS v2.2 lines ~3160 to 3355 |
| Printing: kitchen/bar tickets never show amounts; P and T prints; consolidation rules | POS v2.2 §8.1 and §8.2, lines ~715 to 800 |
| Interrupted sale (P), one `reference` per session | POS v2.2 §1.3, lines ~273 to 295; FAQ Q211 (unique reference) |
| groupingId increments for successive discounts and **stays unchanged on a costCenterChange** | FAQ Q304 a) to e) |
| negQuantity counted on signOrder at a table; a PRICE_CHANGE cancelling an ordered item "n'apparaît que dans la commande et est donc compté sur le signOrder" | FAQ Q303 a) to d) |
| 100 % discount: N at 0,00 EUR, a VAT ticket is delivered (medium never NONE) | FAQ Q206, Q209 |
| An option without surcharge is not transmitted to the FDM; it is handled by the POS for the kitchen/bar ticket | FAQ Q210 b) |
| M122 fixture | `docs/20260821_UC_BasicSet_POS_FDM_1.8/M122_signCostCenterChange/` (scenario, Variables, Response) |
| M132 fixture | `docs/20260821_UC_BasicSet_POS_FDM_1.8/M132_signPaymentCorrection/` |
| UC223 (EVENT discount 50 %, direct sale) | `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/UC223_signSale_EventPriceChange/` |
| UC225A step 2 (P: -2 PRICE_CHANGE then +2 with change), UC225B step 2 (N journal: +2, -2 PRICE_CHANGE, +2 with change) | `.../UC225A_PriceChange_IndirectSale/2_ORDER/`, `.../UC225B_PriceChange_IndirectSale/2_SALE/` |
| UC229B (one free article, LINE scope, lineTotal 0) | `.../UC229B_FreeArticle_DirectSale/` |
| UC4118 (payment correction a week later, CUSTOMER_CREDIT), UC4101 (table to room) | read only, out of scope |
| Cert manual checklist rows | `docs/SUBMISSION_ARTIFACTS/REPONSES_MANUEL_CERT_FR.md` §III.F (lines ~455 to 500) |

Fixture facts the design relies on:
- **M122**: order T1 (Burger of the Chef 22001, VAT B, 28; Dry Martini 10006, VAT A, 12) moved to T5. `ticketMedium: NONE`. `transfer.from[0]`: costCenter `{id T1, type TABLE, reference O158}`, lines quantity -1 with `negQuantityReason: COST_CENTER_CHANGE`, `vats[].price` negative, `lineTotal` negative, `transactionTotal -40`. `transfer.to[0]`: costCenter `{T5, TABLE, O158}` (same reference), the same lines positive, total 40. Response label P, operation COST_CENTER_CHANGE.
- **M132**: corrects N 14/50 (52 EUR): `financials` = CASH MANUAL -52 PAYMENT, then CARD_CREDIT AUTOMATIC +52 PAYMENT; `fdmRef` of the corrected N; `ticketMedium: NONE`; no transaction. Response label F. M110 is a 52 EUR cash sale, so M110 is the base sale of the replay (as M111 does).
- **UC223**: no costCenter (direct sale), each line's VAT entry carries `{id, name, scope EVENT, type PUBLIC, amount, groupingId 1}`, 50 % of 52 = 26 spread 20 + 6.
- **UC229B**: the free Coca Cola line carries a LINE change of -3.04, lineTotal 0. The fixture labels Coca Cola VAT **B** while its scenario says A (the seed catalogue has A): a listed deviation (V1F-D17).

---

## 3. Cross-cutting rules for every task

### 3.1 Environment (mandatory, from the lane brief)

Every Bash call starts with:

```
cd /Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/lane-v1-features && export ANDROID_HOME=$HOME/Library/Android/sdk CARGO_TARGET_DIR=/Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/shared-target CARGO_INCREMENTAL=0 PILI_HEAVY_SLOTS=2 && 
```

Every heavy command (cargo build/test/clippy, gradle, the p5/p6 gates, playwright) runs through `scripts/android/heavy.sh`. Check `df -g /` before heavy builds (under 10 GB: `cargo clean --target-dir target` in this worktree and `(cd android && ./gradlew clean)`; under 6 GB after that: stop, report `DISK_LOW`). Never delete outside the worktree, never touch Docker, `~/.gradle`, `~/.cache` or the shared target with `rm`. No emulator, no adb, no device, no network call to a real vendor. `(cd ui && npm ci)` once if `ui/node_modules` is missing. Commits: Conventional Commits, English body, ending with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` (workers). No em-dash anywhere (code, comments, docs, UI strings, commits). Dossier text French and brand-neutral; comments and internal docs English.

### 3.2 Adding an engine command end to end (checklist, follow commit `c3488d84` as the template)

1. Engine: `crates/pos-engine/src/commands/<module>.rs`, a `do_*` function; refusals return codes from `commands/errors.rs` (add each new code to its list `ALL`, keep the doc comment style).
2. Tauri: one `#[tauri::command]` wrapper per command in `src-tauri/src/ipc/<module>.rs`, a **single** engine call; register it in the `invoke_handler` of `src-tauri/src/main.rs`. `python3 scripts/android/check_ipc_wrappers.py` must pass (and update the count in `CLAUDE.md`: today "96 in all, 94 in the cert build").
3. FFI: `android/rust/pos-ffi/src/api/<module>.rs`, `async`, same parameter names and order as the Tauri wrapper, `self.spawn` for an async engine function, `self.blocking` for a sync one; records in `android/rust/pos-ffi/src/records/` (derive Serialize + Deserialize, `deny_unknown_fields`, no unsigned ints). Tests in `android/rust/pos-ffi/tests/ffi/`. Update the hard-coded count of engine error codes in `android/rust/pos-ffi/src/error.rs` (`every_code_of_both_lists_maps_to_coded`). Then `./gradlew --no-daemon :pos-core:apiDump` (from `android/`, through `heavy.sh`) and commit the `android/pos-core/api/pos-core.api` diff. `python3 scripts/android/check_command_parity.py --strict` and `--hygiene` must pass.
4. Vue: typed wrapper in `ui/src/api/ipc.ts`, types in `ui/src/types/index.ts`, store action (components call stores, stores call ipc), view/component; strings in the Vue catalogs (`ui/src/i18n.ts` core, `i18n-checkout.ts`, `i18n-reports.ts`) for fr, zh, en; error texts `err_<code>` in fr, zh, en; the e2e mock `ui/e2e/fixtures/mock-ipc.ts` answers the new commands; a Playwright spec per feature.
5. Strings to Android: `node scripts/android/vue_to_android.mjs --write` regenerates `strings_vue_*.xml`, `VueStringIds.kt`, `EngineErrorCodes.kt` and `android/app/src/test/resources/vue_strings.json`; commit them (the gate runs `--check`). Android-only texts go to the owning view's hand-written file with its prefix (`strings_ui_order.xml` names start `ui_order_`, and so on, `docs/android/LANES.md` "Hand-written strings").
6. Compose: the call goes through `EngineGateway.call { it.<camelName>(...) }` in the view model or store of the owning package (`feature/<view>/`, or `ui/state/OrderStore.kt` for cart operations); texts through `MessageCatalog` / `ErrorTexts`; theme through `PiliTheme`. Unit tests (view model, content), Roborazzi goldens only for screens this change affects: record with the Roborazzi record task filtered to the affected test classes, open every new or changed PNG and look at it, commit them.
7. Parity: `python3 scripts/android/check_ui_parity.py --strict` must pass; add a row `| Vue | Compose | Commands |` to the view's page in `docs/android/parity/<view>.md` naming the new commands; `docs/android/PARITY.md` only if its index changes.
8. Docs: `docs/android/ENGINE.md` (command list), `docs/android/FFI.md` (command table), `CLAUDE.md` (counts, one or two lines per feature where the existing sections describe features), both dossiers (section 9).

New files land in existing ownership rules (`feature/<view>/`, `ui/state/`, `crates/`, `ui/`): no `LANES.md` change is expected. If a new path matches no rule, run `python3 scripts/android/lane_paths.py --owner <path>` and stop to ask rather than editing `LANES.md`.

### 3.3 Signed operations

Each new signed operation follows `commands/signing.rs`: a new `Kind` (add it to `Kind::ALL` and `name()`), `signing::begin(state, &Op::new(kind, subject))` to recognise a retry, numbering with `peek` under the guard, a `Prepared` with its `ArchiveSpec`, and a new `PendingContext` variant (serde tag, `#[serde(default)]` on any optional field) whose side effects on local tables run **in the archive transaction of `commit`**, so an answer that arrives late (recovery at start, `Begin::AlreadyDone`) has the same effect. Training mode signs the label `T` (`EventLabel::X.in_mode(config.training_mode)`). The FDM mock (`crates/pos-fdm-mock`), the testkit drop list and `pos-fiscal::response_check` already know both mutations; do not change their tables except to add tests.

### 3.4 Migrations

Core migrations are 1 to 11; 12 to 19 are reserved for certified hotfixes; 20 to 27 are taken (HubRise 20, 22 to 25; catalog 21, 26; EFT 27). This lane uses **30** (item C) and **31** (item D), in a new file `crates/pos-db/src/schema_till.rs` and a new list `TILL_MIGRATIONS` registered in `registry()` like `EFT_MIGRATIONS` (not feature-gated). 28 and 29 are left free on purpose for the parallel lanes (V1F-D21). Each migration is append-only friendly: new columns with a CHECK and a trigger that forbids rewriting them, new tables with no-delete and no-update triggers. Add migration tests in `migrate.rs` (applied once, idempotent, triggers refuse delete/update) and update any test that lists the registry's versions.

### 3.5 Gates (names used by the tasks)

All heavy steps through `scripts/android/heavy.sh`.

- **G-RUST**: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo clippy -p pili-pos --no-default-features --features cert --all-targets -- -D warnings`; `cargo test --workspace`; `cargo test -p pili-pos --no-default-features --features cert`; `cargo test -p pos-engine --no-default-features --features cert`.
- **G-FFI**: in `android/rust`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`.
- **G-PY**: `python3 scripts/android/check_ipc_wrappers.py`; `python3 scripts/android/check_command_parity.py --strict`; `python3 scripts/android/check_command_parity.py --hygiene`; `python3 scripts/android/check_ui_parity.py --strict`; `node scripts/android/vue_to_android.mjs --check`; `python3 scripts/android/lane_paths.py --check`.
- **G-UI**: `(cd ui && npm run build && npx playwright test)`.
- **G-ANDROID**: `bash scripts/android/p6_gate.sh` (full mode: p4/p3/p2 checks, `:pos-core` lint, unit tests, apiCheck, AAR, `:app` lint, ktlint, detekt, unit tests, Roborazzi verify, kover, release assemble). **Expected FAILs in this lane, by design:** `linux-frozen` (p2) and `freeze-tag` (p4), because the lane changes Linux frozen paths against tag `linux-v1.0.0-rc2` and the orchestrator re-freezes as rc3 after the merge. Every other check line must be PASS or SKIP. Report the full `GATE |` line list in the task summary.
- **G-DOSSIER**: `python3 scripts/check_dossier.py` and `PILI_DOSSIER_DIR=docs/android/dossier python3 scripts/check_dossier.py` (rules: backticked Rust identifiers and repo paths must exist, brand words, em-dash, placeholders).

A task commits only when its listed gates pass (with the expected FAILs above). A flaky test seen under load is rerun alone; if it passes alone, record it in the task summary, do not change it.

---

## 4. Item A: table transfer (M122, `signCostCenterChange`)

### 4.1 Behaviour

Any logged-in staff member (`Access::Staff`) moves the open dine-in order of table X to table Y. The order keeps its id, its lines, its `reference` (`O<id>`, `pos_fiscal::event::order_reference`), its pre-bill flag and any approved terminal payment. The fiscal record is one P event COST_CENTER_CHANGE whose `transfer` takes every consolidated line off `T<X>` and puts it on `T<Y>`.

Refusals (new error codes in `errors.rs`, names are suggestions, keep them consistent):
- `table_transfer_same_table`: X = Y.
- `table_transfer_target_busy`: table Y has an open dine-in order **with at least one line** (V1F-D2; merge is UC250, out of scope). An open order on Y with no line (a ghost order from a tap-and-walk-away) does not block: it is voided in the transfer's commit (`pos_db::queries::orders::void_if_empty`).
- `table_transfer_target_invalid`: Y outside 1 to 999, or not in the current table plan (`super::table_plan::plan_numbers`). A table out of the plan that still shows because of an open order is a valid **source**, never a target.
- Not dine-in, no table, online order (`hubrise_order_id IS NOT NULL`), closed order: reuse `ORDER_NOT_OPEN` / `TAKEAWAY_NOT_REGISTERED` style existing codes where they fit, else add `table_transfer_not_a_table`.
- No line: `ORDER_NO_LINES`.
- An event of this order waits for the FDM: `ORDER_SIGN_PENDING` (as `unchanged_while_pending`).
- FDM not connected: the usual FDM failure path (no event, nothing moved).

### 4.2 Engine

New `do_transfer_table(state, order_id: i64, to_table: i32) -> Result<TableTransferResult, String>` in `crates/pos-engine/src/commands/tables.rs` (or a new `table_transfer.rs` next to it; keep `tables.rs` readable). `TableTransferResult { order_id, from_table, to_table, kitchen_slip: SlipOutcome, transfer_slip: SlipOutcome }` (serde camelCase).

Under one signing guard (`Kind::TableTransfer`, subject = order id), in this order:
1. Checks of 4.1, read under the guard.
2. **Register the unsent entries first** on the old table with `sign_table_order_locked` (the same P ORDER as leaving the table, exactly as `do_sign_pre_bill` does), so the transfer moves everything the FDM knows and nothing is left unregistered (V1F-D3). If this P fails, stop with its error.
3. Build `CostCenterChangeInput` with a new builder `pos_fiscal::event::build_cost_center_change_input(config, identity, order, lines, line_changes, from_table, to_table, ticket_number, booking)`:
   - `common` with `TicketMedium::None` (the fixture's medium; the event itself is not printed, V1F-D4) and the operator's INSZ as `employeeId`;
   - `transfer.from = [ { costCenter: build_table_cost_center(from, ref), transaction: lines negated } ]`, `transfer.to = [ { costCenter: build_table_cost_center(to, ref), transaction: lines } ]`, one-to-one;
   - lines = the **consolidated** order lines (`get_order_lines`, quantity > 0), same `productId`, names, department, `unitPrice`; the from side has quantity `-q`, `negQuantityReason: COST_CENTER_CHANGE`, `vats[].price` and `lineTotal` negative; the to side positive;
   - price changes already on a line (an online order never reaches a table, but item C adds till discounts) are **carried** on both sides with the same `id`, `name`, `scope`, `type` and `groupingId` (FAQ Q304 d), negated on the from side, through `pos_fiscal::price_change::apply` with the consolidated map (`price_changes::load(...).1`, line keyed). `lineTotal` and `transactionTotal` follow (from = -net, to = +net) (V1F-D5);
   - reuse `build_transaction` internals: refactor it to take a per-line reason instead of duplicating it.
4. Sign (`SignMutation::CostCenterChange`), with `PendingContext::TableTransfer { order_id, from_table, to_table, operator_insz }`. Its commit, inside the archive transaction: void the target's empty open order if any, then `UPDATE orders SET table_number = to WHERE id = order_id AND status = 'open'`. The P ORDER of step 2 keeps its own `sent_entries`; the transfer archives with `sent_entries: None` (it adds no journal entry: the N later sends the order's journal under `costCenter T<Y>` with the same reference).
5. After the guard: print the bar slip of step 2's entries (`print_registered`, table X, as today) and a **transfer notice** on the bar / kitchen printer: new `pos_print::bar_ticket::format_transfer_slip(&TransferSlip { date_time, operator, from_table, to_table, lang, is_training })`, same header, title and disclaimer as the bar slip, the line "Table X -> Table Y" (labels in `labels.rs` for fr, nl, de, en), no item, no amount (V1F-D6). Printing never undoes the signing; both outcomes go back to the UI.

Reports (`crates/pos-fiscal/src/report_totals.rs`) (V1F-D19):
- `transactions`: a COST_CENTER_CHANGE event counts once under label P (or T in training) with amount Σ of every line of `transfer.from` and `transfer.to` (0 for a full move). Today `has_transaction` only looks at a top-level `transaction`: extend it, and `lines()` for the amount, to the transfer items.
- `negQuantities`: the from-side negative lines count under reason COST_CENTER_CHANGE (one ticket per event), like an order registration. They are never resent by the N (the N carries the journal, not the transfer), so no `registered_negatives` change.
- departments, vats, payments: unchanged (label N only).

`CLAUDE.md` mentions the command, `docs/android/ENGINE.md` lists it.

### 4.3 UIs (both, same behaviour)

- **Order view (cart)**: for a dine-in order with lines, a "Changer de table" action in the cart header area (Vue `CartSidebar.vue`, Compose `feature/order/OrderCartColumn.kt` + `OrderStore.kt` / `MenuViewModel.kt`). It opens a table picker dialog listing the **free** tables of the plan (from `list_configured_tables`, status `free`), large touch buttons, current table excluded. Confirm calls `transfer_table`. On success the cart header shows the new table, a toast "Commande déplacée vers la table Y", and the slip outcomes are reported like "Envoyer en cuisine" (a failed slip shows the existing bar slip warning).
- **Tables view**: a header toggle "Déplacer une table" (Vue `TablesView.vue`, Compose `feature/tables/TablesScreen.kt` + `TablesViewModel.kt`): first tap an occupied table (source), then a free table (target), then a confirmation "Déplacer la table X vers la table Y ?". Cancel leaves the mode. Busy and invalid targets are not tappable in the mode.
- Strings (fr / zh / en) in the Vue core catalog, e.g. `transfer_table`, `transfer_pick_target`, `transfer_confirm`, `transfer_done`, `transfer_mode`, `transfer_pick_source`, error texts `err_table_transfer_*`.
- Training mode works the same (signs T).

### 4.4 Tests (A)

- `crates/pos-fiscal`: builder unit tests (lines negated with reason COST_CENTER_CHANGE, same reference both sides, ids `T01`/`T05`, totals, medium NONE, price changes carried and negated with the same groupingId, quantity -0 never produced).
- `crates/pos-engine/tests/spf_e2e_fixtures.rs`: **`m122_cost_center_change`**: seed, order on table 1 from the M122 lines (`order_from` with M122's `transfer.to[0].transaction` lines), `mock_uc(M121)` for the registration P, `mock_uc(M122)`, `do_transfer_table(order, 5)`, then compare the COST_CENTER_CHANGE request with the fixture. Extend `pos_engine_testkit::compare::business_view` with a `transfer` view: for each of from/to: `costCenter.type` and the transaction view (same mapping as `transaction`); assert in the test that from and to share the same `reference` and differ in `id`. Target: **no deviation**. Also assert the request order (P ORDER then P COST_CENTER_CHANGE) and that the following sale's costCenter is `T05` with the same reference. `app.dump_evidence("M122")`.
- New `crates/pos-engine/tests/table_transfer.rs`: every refusal of 4.1; ghost order on the target is voided; a target out of the plan refused; an order whose entries are partly unsent registers them first (two events); the FDM down leaves the table unchanged and signs nothing; a late answer (pending sign recovered at start, see `spf_e2e_crash_safety.rs` patterns) moves the table; training signs T; access (waiter allowed); the transfer notice and the kitchen slip reach the printer mock (`spf_e2e_bar_slip.rs` patterns); the N after a transfer carries `costCenter T05` and the journal; with price changes recorded on lines (insert through `pos_db::queries::price_changes::add_to_line` in the test) the transfer carries them negated / positive with the same groupingId.
- `report_totals.rs` unit tests for the P count, amount 0 and the COST_CENTER_CHANGE negQuantities; `spf_e2e_reports.rs`: an X after a transfer.
- `pos-print`: `format_transfer_slip` text test in fr and one other language, training header, no amount characters.
- `crates/pos-engine/tests/access_control.rs`, `error_codes.rs`: the new codes and command.
- FFI test for `transfer_table`; Vue Playwright `ui/e2e/table-transfer.spec.ts` (both entry points, refusals shown in French); Compose unit tests for the picker, the move mode and the view models; Roborazzi goldens for the picker dialog, the Tables move mode and the cart header with the new action.

---

## 5. Item B: payment correction (M132, `signPaymentCorrection`)

### 5.1 Behaviour

A manager, confirmed by their PIN (`access::confirmed_operator(state, insz, pin, Access::Admin)`, as `do_close_unpaid`), corrects the method of **one payment line** of a sale signed today: "this was paid by card, not cash". The F event carries the line removed (its type, -amount) and the same amount under the new type (+amount), and `fdmRef` of the corrected N. No ticket is printed (`ticketMedium: NONE`, as the fixture), the drawer is not opened (V1F-D10).

Rules (conservative, V1F-D7 to V1F-D11):
- The corrected event is a sale **N** (or **T** while the till is in training; a T sale only in training, an N sale only outside training), of the **current booking period** (not covered by a Z yet), not a refund (negative total), not closed unpaid (empty `financials`), not an online order (APP / ONLINE lines, or `hubrise_order_id`), not refunded in whole or in part since.
- **One correction per sale.** A second one is refused (`payment_correction_already_done`). Checked from the archive under the signing lock (an F/T PAYMENT_CORRECTION whose `fdmRef` equals the sale's), no new table.
- The corrected line is a `PAYMENT` line typed by hand (`inputMethod: MANUAL`). A card the linked terminal approved (`AUTOMATIC`, an `eft_payments` row) is never corrected: `payment_correction_terminal_card` (reversing a terminal payment is not supported, EFT-D1).
- The new method is one of the till's methods: cash, card (`CARD_DEBIT`, MANUAL, id "2"), meal voucher (`CHEQUE_MEAL`, id "3"); it differs from the line's type. Identity from `pos_fiscal::payment::payment_line_identity`, same `id` / `name` as a sale would write.
- Rounding: refused when the corrected line is cash and the sale has a ROUNDING line, and when the new method is cash and the amount is not a multiple of 5 cents (`payment_correction_rounding`). No ROUNDING line is ever written by a correction.
- FDM down: nothing signed, the usual failure path.

### 5.2 Engine

New module `crates/pos-engine/src/commands/payment_correction.rs`:
- `do_list_sale_payments(state, fiscal_event_id) -> SalePayments` (Admin): `{ fiscalEventId, ticketNumber, totalCents, lines: [ { index, paymentType, name, amountCents, inputMethod, amountType, brand } ], correctable: bool, refusal: Option<String> (an error code), corrected: Option<{ fromType, toType, amountCents, at }> }`. The UI shows the refusal text when `correctable` is false.
- `do_correct_payment(state, fiscal_event_id, line_index: i32, method: PaymentMethod, insz: String, pin: String) -> FinancialResult` (reuse `commands::financial::FinancialResult`).
- Builder `pos_fiscal::event::build_payment_correction_input(config, identity, ticket_number, employee_insz, booking, original_ref: &FdmReference, from: &PaymentLineInput-like spec, to: PaymentType, amount_cents)`, MANUAL lines, `drawer: None`, `TicketMedium::None`. The original reference comes from the archived N (`fdm_id`, `event_label`, counters from its response, as `copy.rs` does for C).
- Signed with `Kind::PaymentCorrection` (subject = the N's archive id) and `PendingContext::PaymentCorrection { corrected_event_id }` (no local table write is needed; the order's `payments` history table is not changed, the reports read the archive).

Reports (`report_totals.rs`, `commands/report.rs`) (V1F-D9):
- `payments()` takes the sales **and** the PAYMENT_CORRECTION events of the same scope (label F for the turnover, T kept apart as today). For each correction line, per (id, name, type) and amountType: a negative amount adds to `negativeCorrections`, a positive one to `positiveCorrections`, each line counts 1 in `correctionsCount`; `totalAmount = normal + negative + positive`. A method that appears only through a correction gets an entry with `normalAmount 0`. Update the doc comment ("Payment corrections are not supported in v1.0.0").
- User report: the correction counts in the payments of the **employee who signed the F** (its `employeeId`), not of the original cashier (V1F-D9).
- Printed X/Z: the per-method amounts are the `totalAmount` (they already are), so the correction shows; add a "Corrections de paiement" detail line (count and Σ of positive) on the printout through `ReportDetails` and `pos-print/src/report.rs` labels (fr, nl, de, en). The count of payments per method keeps counting N lines only.
- No change to the existing report fixtures (M180 to M184 have no correction): `spf_e2e_report_fixtures.rs` must stay green untouched.

### 5.3 UIs (both)

- Reports view, "Tickets" tab (Vue `ReportsView.vue`; Compose `feature/reports/ReportsTicketsTab.kt`, `ReportsViewModel.kt`, new `ReportsPaymentCorrectionDialog.kt`): on a sale, an action "Corriger le paiement" (managers only). The dialog calls `list_sale_payments`, lists the payment lines (method, amount, "terminal" mark for AUTOMATIC lines, not selectable), the manager picks the line and the new method (three large buttons), sees "Carte 52,00 EUR -> Espèces 52,00 EUR", then confirms with the PIN dialog (`PinDialog.vue`, Compose PIN panel used by close unpaid). Refusals show the error text; a corrected sale shows "Paiement corrigé : Espèces -> Carte".
- Strings in `ui/src/i18n-reports.ts` (fr / zh / en), error texts in the core catalog.

### 5.4 Tests (B)

- `spf_e2e_fixtures.rs` **`m132_payment_correction`**: M110 sale with `mock_uc(M110, Some(&m132["fdmRef"]))`, `mock_uc(M132)`, `do_correct_payment(n_event, 0, Card, insz, pin)` (the fixture operator needs a PIN in the test: set one through the users query as `access_control.rs` does), compare request 1 with the fixture. Expected listed deviation only: the card line's `type` (`CARD_DEBIT` vs `CARD_CREDIT`), reason "without a linked terminal the operator cannot tell debit from credit, a card typed by hand is CARD_DEBIT MANUAL (REPONSES §III.F)". `fdmRef` must be equal. `app.dump_evidence("M132")`.
- New `crates/pos-engine/tests/payment_correction.rs`: every refusal of 5.1 (wrong PIN, waiter, previous period after a Z, refund, unpaid, online order, terminal card via the scripted terminal of `eft_signing.rs`, second correction, same method, rounding both ways, training mismatch, refunded sale, unknown line index); a split sale (card + cash) corrects one line; FDM down; late answer recovered; X and Z totals after a correction (normal, negative, positive, count, total per method) and the user report attribution; the printout line.
- `report_totals.rs` unit tests for the correction arithmetic, a method only present through a correction, training kept apart.
- `access_control.rs`, `error_codes.rs`; FFI tests for both commands; Playwright `ui/e2e/payment-correction.spec.ts`; Compose dialog and view model tests; Roborazzi goldens for the dialog states (choose, confirm, refused).

---

## 6. Item C: discount at the till ("Remise" / "Offert", manager PIN)

### 6.1 Behaviour

Before payment, a manager (logged-in `Admin`, PIN confirmed) applies to an open till order:
- an **order discount**: a percent (integer 1 to 99), an amount (cents, 1 to the order's net total), or **Offert** (100 %); or
- a **free line** ("Offert" on one line, its whole quantity, UC229B).

A reason is required, chosen from a fixed list (`commercial_gesture`, `staff_meal`, `service_issue`, `other`) with an optional short free text; it is stored locally for audit, never in the signed payload (V1F-D14). A manager can also **remove** the discount (order or line). The discount shows in the cart, on the checkout total, on the pre-bill, on the VAT ticket and in the X/Z reductions.

Refusals: not a manager or wrong PIN; online order (it has its own discounts); closed order; pending sign; no line; order discount when one already applies (`discount_already_applied`: remove it first); free line when the line already has a change or an order discount applies; amount above the net total or not positive; percent out of range; the new net total would fall under the approved unused terminal card payments (`eft::charged_floor` / `keep_charged`, existing `eft_below_charged`); remove when there is none (`discount_none`).

### 6.2 Fiscal shape (the core decision, V1F-D12, V1F-D13)

Price changes live in `order_price_changes` (migration 23) keyed by journal entry and line; `pos_fiscal::price_change::apply` puts them on the transaction built from the entries; `ticket_lines` prints a line's public changes under it. Till discounts reuse this model:

- **Amounts.** Order discount: `D` = round half up of `net_total * pct / 100` (percent), the typed amount, or `net_total` (Offert); spread over the lines with a positive net amount with `pos_core::money::distribute` (weights = each line's net amount), so the shares add up to `D` exactly (UC223: 26 = 20 + 6). Free line: `D` = the line's net amount.
- **Change identity.** `scope EVENT` for an order discount, `LINE` for a free line; `type PUBLIC`; `id` language-neutral: `TILL-DISCOUNT` (percent or amount), `TILL-FREE` (order Offert), `TILL-FREE-LINE` (free line); `name` in the establishment's receipt language (`ReceiptLanguage` of `config.language`): fr "Remise 10 %", "Remise", "Offert"; nl "Korting 10 %", "Korting", "Gratis"; de "Rabatt 10 %", "Rabatt", "Gratis"; en "Discount 10 %", "Discount", "Free" (a helper in `pos-fiscal/src/price_change.rs`). `groupingId` = 1 + the highest groupingId of the order's changes (FAQ Q304 b).
- **Where the change goes**, per affected line:
  - the line has **exactly one journal entry and it is unsent**: the change is recorded on that entry (`price_changes::add_to_line`), no new entry. This is the direct sale shape of UC223 and UC229B;
  - otherwise (the line was registered in a P, or it has several entries): append a **correction entry** `-q` marked PRICE_CHANGE (carrying the negation of the line's existing changes, as `negate_on_correction` does) and a **re-entry** `+q` marked PRICE_CHANGE carrying the line's new changes. The FDM then receives `-q` with `negQuantityReason: PRICE_CHANGE` and `+q` with the `priceChanges` (UC225A step 2, UC225B step 2).
- **Registration.** When the order is dine-in and at least one of its entries was already sent (the table was registered), the command signs a **P ORDER right away** under the same guard (the unsent entries, the PRICE_CHANGE pair included), because FAQ Q303 d) says such a line "n'apparaît que dans la commande et est donc compté sur le signOrder" (UC225A step 2). A takeaway or an unregistered table is registered by its N (direct sale, FAQ Q303 b).
- **Removal** is the same mechanism in reverse: correction `-q` PRICE_CHANGE carrying the negated changes, re-entry `+q` with no change; a line whose single unsent entry got a direct change gets the same pair (append-only: a recorded change is never deleted); then the immediate P under the same rule.
- **Kitchen.** PRICE_CHANGE entries never print on the bar / kitchen slip (they order nothing) and do not count in the cart's "not sent to the kitchen" quantity.
- **Refund.** A line with a change can only be refunded with the whole order (existing rule of `refund.rs`, "Ligne vendue avec une remise : remboursez la commande entière"); the refund gives back the net amount (existing behaviour for online orders, test it for till discounts).
- **Transfer.** A discounted order moves with its changes (item A, FAQ Q304 d).
- **Pre-bill.** `sign_pre_bill_locked` must apply the consolidated line changes to the PreBillInput transaction and print the change rows (today it ignores them; online orders never use a pre-bill, a till discount does).
- **HubRise echo** of a till sale (`hubrise_push::build_hubrise_order_body`): add the discount as HubRise `discounts` (`name`, `price_off`) so the echoed order's total matches its payments; test with the mock.

### 6.3 Data model, migration 30 `till-discounts`

In `crates/pos-db/src/schema_till.rs`:
- `ALTER TABLE order_entries ADD COLUMN till_op TEXT CHECK (till_op IS NULL OR till_op = 'price_change');` plus a trigger refusing any UPDATE OF `till_op` (the existing `order_entries_no_rewrite` trigger lists its columns explicitly and does not cover a new one). Set on both entries of a PRICE_CHANGE pair.
- `till_discounts` (append-only, no-delete and no-update triggers): `id`, `order_id`, `line_id` (NULL for an order discount), `action` (`apply` / `remove`), `kind` (`percent` / `amount` / `free`, NULL on remove), `value` (percent or cents, NULL on remove), `amount_cents` (the total change, negative), `change_id`, `grouping_id`, `reason_code`, `reason_text`, `manager_insz`, `created_at`.

Queries (`pos-db/src/queries/`): insert an entry with `till_op`, the set of PRICE_CHANGE entry ids of an order (for the builders), the audit rows. `get_cart_lines` returns, per line, `discountCents` (Σ of its changes) and `priceChanges` (`[{ id, name, scope, amountCents }]` after consolidation, `by_line`), so both UIs show net amounts and the VAT summary uses net amounts. The FFI `CartLine` record and the Vue type follow.

Builders: `price_change::apply` stays; a new helper marks `negQuantityReason: PRICE_CHANGE` on the transaction lines of the entries in the PRICE_CHANGE set (negative ones), used by the P ORDER and the N builders (`sell_locked`, `sign_table_order_locked`). `build_transaction`'s per-event `neg_reason` stays the default.

The engine module `crates/pos-engine/src/commands/discount.rs` is compiled under `#[cfg(feature = "hubrise")]` like `price_changes.rs` (the price change store is part of the HubRise schema; every shipped build, Linux cert and Android cert, has the feature) (V1F-D15). Its Tauri wrappers and FFI exports follow the same gate; `check_command_parity.py` counts them in the cert set.

Commands:
- `do_apply_discount(state, order_id, input: DiscountInput { target: order | line(line_id), kind: percent | amount | free, value: Option<i64>, reason_code, reason_text: Option<String> }, insz, pin) -> DiscountResult { order: Order, kitchen_slip: SlipOutcome }`
- `do_remove_discount(state, order_id, line_id: Option<i64>, insz, pin) -> DiscountResult`

Both run under the signing guard (`Kind::TableOrder` subject order id is fine for the immediate P, or a new `Kind::Discount`; the worker decides and records it), use `confirmed_operator(.., Access::Admin)`, write the journal in one transaction with `keep_charged`, then the immediate P if the rule says so.

### 6.4 Reports and ticket

- Ticket: `ticket_lines` already prints each line then its public changes; check the totals block uses the net total and the VAT block comes from the FDM `vatCalc`. A sale at 0,00 EUR (Offert) is signed and printed like any other (FAQ Q206, Q209; the existing zero-sale path pays cash 0).
- X/Z printout: `report_details` today shows every price change as line reductions and `event_reductions: (0, 0)`. Split them by scope from the archived N lines (EVENT scope -> event reductions, LINE scope -> line reductions), count = number of tickets with such a change, amount = Σ. The signed report payload (`turnover.priceChanges`) does not change.
- `negQuantities` gets PRICE_CHANGE lines automatically (P at a table, N for a direct sale, `registered_negatives` keeps the N from counting the P's lines again); test it.

### 6.5 UIs (both)

- Cart (Vue `CartSidebar.vue` + `OrderLineItem.vue`; Compose `OrderCartColumn.kt` + `OrderLineItem.kt` + `OrderStore.kt`): a "Remise" button for managers when the order is a till order with lines; a discount dialog: tabs or segmented buttons "%", "Montant", "Offert"; quick percents 5, 10, 15, 20, 50; the `NumPad` for a custom percent or amount; the reason buttons (four codes) and an optional text; a preview "Total 52,00 EUR -> 26,00 EUR"; confirm opens the PIN dialog. Each line shows its public changes under it ("Remise 50 %  -20,00"); a line action "Offrir" (manager, PIN) on a line without change; "Retirer la remise" on a discounted order / offered line. The cart total and the VAT summary use net amounts.
- Adding the same product again must create a **new line** when the existing line has a change (engine refuses quantity changes on such a line; `addItem` in `ui/src/stores/order.ts` and `OrderStore.kt` must skip lines with changes, as they skip lines with options).
- Checkout (Vue `CheckoutView.vue`, Compose `feature/checkout/`): the amount due is the net total; the summary shows the discount rows.
- On-screen text entry on Vue for the optional reason text: reuse the on-screen keyboard of `EmailKeypad.vue` (generalise it into a text keypad component if needed, keep the email use unchanged); Android uses the system keyboard.
- Strings fr / zh / en in the core and checkout catalogs.

### 6.6 Tests (C)

- `spf_e2e_fixtures.rs` (Advanced Set root, like `uc230`):
  - **`uc223_event_discount_at_the_till`**: dine-in table 1, 2 Carpaccio 20001 + 1 Dry Martini 10006, `do_apply_discount(percent 50)`, card checkout; compare with UC223: allowed deviations `costCenter.type` (TABLE_SALE) and, if the fixture's change `id`/`name` are compared, none (they are free text, presence only). Card type matches (CARD_DEBIT).
  - **`uc225a_discount_after_registration`**: order of 2 Carpaccio registered by a P (mock UC225A 1_ORDER), discount 20 %: the immediate P compared with UC225A 2_ORDER; listed deviation: `scope` EVENT (pili-pos, an order discount) vs LINE (fixture); then the cash N compared with UC225B 2_SALE (same deviation).
  - **`uc229b_free_line_at_the_till`**: 2 Burger 22001 + 1 Coca Cola 12009, free line on the Coca Cola, cash; compare with UC229B; listed deviations: the Coca Cola VAT label (catalogue A, fixture B, scenario A) and TABLE_SALE.
  - Extend `business_view` to compare each VAT entry's `priceChanges` as `[{scope, type, amount}]` (id and name: presence only). Existing fixture tests have no priceChanges and must stay green; rerun `hubrise_discounts.rs`.
- New `crates/pos-engine/tests/till_discounts.rs`: amounts and spread to the cent (odd totals, 1 cent lines), percent rounding, amount = total, Offert at 0,00 EUR (N signed, ticket printed, cash 0), every refusal, groupingId increments after remove + re-apply, direct attach vs PRICE_CHANGE pair choice, immediate P only when the table was registered, bar slip ignores PRICE_CHANGE entries, cart unsent quantity unchanged, refund of a discounted sale (whole order only, net amount), transfer of a discounted order (item A), pre-bill after a discount, EFT floor, training, audit rows, HubRise echo discounts, X/Z event vs line reductions and negQuantities PRICE_CHANGE.
- Migration 30 tests in `migrate.rs`; `access_control.rs`, `error_codes.rs`; FFI tests; Playwright `ui/e2e/till-discount.spec.ts`; Compose tests and Roborazzi goldens for the dialog, the cart with a discount, the checkout summary.

---

## 7. Item D: kitchen notes per order line

### 7.1 Behaviour and rules (V1F-D16)

Any staff member writes a short note on a cart line ("sans wasabi", "allergie arachide"). The note:
- applies to the whole line; adding the same product again creates a new line when the existing line has a note (as for options and discounts), so "1 with, 1 without" is two lines;
- is **never** part of fiscal data: not in any FDM message (P, N, transfer), not in the product name, not on the VAT ticket, the pre-bill, a copy or the export (FAQ Q210 b; POS v2.2 §8.1). It prints on the bar / kitchen slip under its item, and shows in the cart;
- is at most 60 characters after trim, one line, no control character, and only characters the slip printer prints (CP858 after `pos_print::escpos::printable`): otherwise `note_too_long` / `note_unprintable` (a Chinese note would print "???" in the kitchen). An empty note clears it;
- on an open till order only (not online orders), not while an event of the order waits for the FDM.

**A note changed after the line went to the kitchen** (the line has sent quantity): the command prints a **note slip** right away on the bar / kitchen printer: the bar slip header (date, operator, table or direct sale), the title "MODIFICATION", the product name and "Note : <text>" (or "Note supprimée"), no quantity, no amount, the "not a VAT ticket" mention. It is not an order (no quantity, nothing new to prepare) and no fiscal event goes with it. A note set before the line is sent prints with the line's item on the next slip.

### 7.2 Data model, migration 31 `kitchen-notes`

`ALTER TABLE order_lines ADD COLUMN kitchen_note TEXT;` and `ALTER TABLE order_lines ADD COLUMN kitchen_note_printed TEXT;` (the text last printed on a slip, NULL if never). `pos_core::order::OrderLine` (a fiscal input struct) does **not** get the note; `CartLine` does (`kitchenNote`), and the slip code reads it by line id.

### 7.3 Engine and printing

- `do_set_line_note(state, line_id, note: Option<String>) -> SlipOutcome` in `commands/order.rs` (Staff). Validate, store, then: if the line has sent quantity and `note != kitchen_note_printed`, print the note slip; on `Printed` set `kitchen_note_printed`. On failure the outcome goes back to the UI (same warning as a failed bar slip) and the printed copy stays stale, so confirming the same note again retries.
- Bar slip (`commands/bar_slip.rs`, `pos_print::bar_ticket`): `BarItem` gains `note: Option<String>`, printed on the next line indented ("   > sans wasabi"); after a slip prints, set `kitchen_note_printed` of the printed lines. Entries map to their line through `order_entries.line_id` (the journal `OrderLine.id` is the entry id). PRICE_CHANGE entries (item C) are skipped. New `format_note_slip` in `bar_ticket.rs` with labels in `labels.rs` (fr, nl, de, en).

### 7.4 UIs (both)

- A "Note" action on each cart line (Vue `OrderLineItem.vue`; Compose `OrderLineItem.kt`), a dialog with the text field and a character counter; Vue uses the on-screen text keypad of item C (or `EmailKeypad.vue` generalised), Android the system keyboard. The cart shows the note under the line name (italic, muted token colour). The dialog says when the note will print ("Imprimée en cuisine avec la commande" / "Imprimée en cuisine maintenant").
- Strings fr / zh / en in the core catalog; errors `err_note_too_long`, `err_note_unprintable`.

### 7.5 Tests (D)

- `crates/pos-engine/tests/kitchen_notes.rs`: validation; the note never appears in any archived request (P, N, transfer, pre-bill) nor on the VAT ticket text; it prints on the bar slip after the P with the item; a change after sending prints one note slip and none when unchanged; failure then retry; takeaway prints the note with the N's slip; adding the same product to a noted line creates a new line (store tests on both UIs, engine refuses nothing here).
- `pos-print` text tests for the item note and the note slip (fr and one other language, CP858 output).
- Migration 31 tests; FFI test; Playwright `ui/e2e/kitchen-notes.spec.ts`; Compose tests and Roborazzi goldens for the dialog and a cart line with a note.

---

## 8. Test strategy (summary)

- Every fiscal path has (1) a fixture replay compared field by field with the official fixture (`spf_e2e_fixtures.rs`, deviations listed with their reason, a listed deviation that no longer occurs fails), (2) a dedicated integration test file for refusals, recovery, training and access, (3) unit tests of the builders and report arithmetic, (4) report tests (X/Z totals and printout), (5) FFI tests, (6) UI tests on both UIs.
- `business_view` grows twice (transfer in T1, priceChanges in T3); after each change rerun the whole `spf_e2e_*` and `hubrise_*` sets.
- Crash safety: each new signed op gets a "late answer" test (pending sign resolved at start, `Begin::AlreadyDone`) in its own test file, using the patterns of `spf_e2e_crash_safety.rs`.
- No test is removed (gate `kotlin-tests` and the workspace inventory).

---

## 9. Dossier changes (French, brand-neutral; both dossiers unless marked)

Write the dossier after the code, so every backticked Rust identifier and repo path exists (`check_dossier.py`). Linux dossier `docs/SUBMISSION_ARTIFACTS/`, Android dossier `docs/android/dossier/` (same file names where both exist). The orchestrator rebuilds the PDFs.

| File | A (M122) | B (M132) | C (remise) | D (notes) |
|---|---|---|---|---|
| `REPONSES_MANUEL_CERT_FR.md` §III.F | row "Gestion des cost centers": ☑ for **Attribution** (transfert de table, M122), division / fusion stay ☐ hors périmètre; §IV coverage sentence (18 -> 19 of 22) | row "Changement de paiement (M132)": ☑ with the rules; coverage 19 -> 20 of 22 | rows "Réductions" (till discount added to online discounts), "Modification de prix ligne" (☑ only for "Offert" on a whole line, UC229B) | if a row about kitchen tickets / options exists, add the note rule; else a sentence under P printing |
| `RESULTATS_TESTS_UC_FR.md` | intro list of excluded UCs, M122 section title and status (rejoué, comparé, test name), coverage table line (Basic Set 22 / 20 tested), "16 UC rejoués" -> 17 | same for M132, -> 18 | Advanced Set lines for UC223 (till path added), UC225A/B, UC229B | none |
| `COUVERTURE_FIXTURES_FR.md` (Linux) | M122 "E2e applicatif": `transfer_table` | M132: `correct_payment` | - | - |
| `MATRICE_TRACABILITE_FR.md` (Linux), `docs/android/UC_MATRIX_ANDROID.md` | M122 row | M132 row (Android device column "À effectuer") | - | - |
| `MAPPING_CONFORMITE_FR.md` | excluded list | excluded list, M132 paragraph | - | - |
| `DECLARATION_FONCTIONS_INTERDITES_FR.md` | - | M132 now in scope: manager + PIN, signed F, never hides a sale, one per sale, current period | discounts: manager + PIN, signed priceChanges, audit table | notes never change a fiscal record |
| `FLUX_REMBOURSEMENT_FR.md` | - | header and §2 rewritten (rules, event, reports); the refund + new sale workaround stays for other cases | refund of a discounted till sale (whole order, net amount) | - |
| `LISTE_FONCTIONNALITES_FR.md` | add | add | add | add |
| `MANUEL_UTILISATEUR_FR.md` | §5 / §6 new subsection "Changer de table"; profiles table §1 | §9 or §11 new subsection "Corriger un moyen de paiement"; §1 | §5.3 "Il n'y a pas de bouton de remise" replaced by the discount procedure; §7 checkout; §9 refund note; §1 | §5.3 notes; §6.1 kitchen slip |
| `PROFILS_UTILISATEURS_FR.md` | staff | manager + PIN | manager + PIN | staff |
| `COUVERTURE_ADVANCED_SET_FR.md` | UC4101 stays out (room) | UC4118 stays out (customer credit) | UC223 / UC225A / UC225B / UC229B: till path covered | - |
| `TICKETS_EXEMPLES_FR.md` | §8 add the transfer notice slip | - | a VAT ticket with an order discount and one with an offered line (from the pos-print test output) | §8 bar slip with a note, the note slip |
| `LIGNES_PRODUITS_FR.md` | transfer lines | - | PRICE_CHANGE pair, EVENT / LINE changes | notes are not product data (FAQ Q210 b) |
| `DOCUMENT_PROCESSUS_TECHNIQUE_FR.md`, `FLUX_DONNEES_FR.md` | P COST_CENTER_CHANGE in the flows | F PAYMENT_CORRECTION | priceChanges at the till | notes stay local |
| `SCHEMA_BD_FR.md`, `SCHEMA_ER_FR.md` (Linux) | - | - | migration 30 | migration 31 |
| `docs/android/dossier/INDEX_DOSSIER_FR.md` | row about "pas d'IPC encore" for M122/M132 updated | same | - | - |
| `docs/android/dossier/RESULTATS_TESTS_UC_FR.md` | M122 skeleton entry and coverage table (device run "À effectuer (EV-06)") | M132 | - | - |

Counts to keep consistent across both dossiers after T2: Basic Set 22 UCs, 20 with an automated test (exclusions M150 event I, M112A not applicable); fixture replays in `spf_e2e_fixtures.rs` 18 Basic Set UCs.

---

## 10. Risks, open questions and recommended decisions

| # | Question | Recommended decision (recorded) |
|---|---|---|
| R1 | Target table has an open order | Refuse in v1 unless the open order has no line (voided). Merge is UC250/251, out of scope (V1F-D2). |
| R2 | Unsent entries at transfer time | Register them on the old table first (P ORDER), then transfer, under one guard (V1F-D3). |
| R3 | Is the transfer printed / ticketMedium | NONE as the fixture; a non-fiscal transfer notice goes to the kitchen like the existing bar slip, which already follows a P with medium NONE (V1F-D4, V1F-D6). |
| R4 | Discounted order moved | Carry the price changes with the same groupingId (FAQ Q304 d) (V1F-D5). |
| R5 | Reports for COST_CENTER_CHANGE | Count it under P with amount 0, count its negative lines under COST_CENTER_CHANGE. The spec's "transactions" wording is about events carrying transaction objects; a transfer carries them inside `transfer`. Conservative reading recorded (V1F-D19). |
| R6 | Payment correction of an older sale | Current booking period only; UC4118 (a week later, customer credit) out of scope (V1F-D8). |
| R7 | Terminal card corrected to cash | Refused: a terminal-approved card cannot be turned into another method; no reversal support (V1F-D7). |
| R8 | Correction target typed as AUTOMATIC like M132 | No: the POS did not take that card through the link, so the line is MANUAL CARD_DEBIT; listed deviation (V1F-D7). |
| R9 | Rounding in a correction | Refuse the two cases that would need a ROUNDING line (V1F-D10). |
| R10 | Whose user report gets a correction | The employee who signed the F (V1F-D9). |
| R11 | Order discount on an already registered table | PRICE_CHANGE correction + re-entry, registered right away by a P (FAQ Q303 d, UC225A step 2) (V1F-D12, V1F-D13). |
| R12 | Scope of an order discount | EVENT (UC223, the hubrise path); UC225 uses LINE for a one-line order: listed deviation (V1F-D13). |
| R13 | Single free item (UC229B) | Included: LINE scope, whole line, same mechanism, low extra risk. The fixture's VAT label B for Coca Cola is a fixture error (scenario says A): listed deviation (V1F-D17). |
| R14 | Discount reason in the signed payload | No: the `name` is the customer-visible label; the reason is local audit (V1F-D14). |
| R15 | Feature gate of the discount code | `hubrise`, the feature of the price change store; every shipped build has it (V1F-D15). |
| R16 | Notes in fiscal data | Never (FAQ Q210 b); note slip after sending is non-fiscal, without quantity or amount (V1F-D16). |
| R17 | Migration numbers collide with the parallel lanes | 30 and 31, leaving 28 and 29 free; `check_applied_names` refuses a reused number. The orchestrator checks the three lanes' registries at merge (V1F-D21, follow-up F5). |
| R18 | Gate FAILs on frozen Linux paths | Expected (`linux-frozen`, `freeze-tag`); the rc3 freeze is the orchestrator's (V1F-D22). |
| R19 | Evidence JSON and RESULTATS embedding | Not regenerated in the lane (it rewrites every UC's evidence and would conflict with the other lanes); the orchestrator runs `scripts/capture_uc_evidence.sh` and `scripts/embed_uc_evidence.py` after the merge (V1F-D23). |
| R20 | A worker cannot make a fixture match | Stop and record the exact field, the spec text and two options in DECISIONS; never silence a deviation without a reason line. |

---

## 11. Captures to retake (orchestrator, after the merge; both products unless marked)

1. Order view, cart header with "Changer de table" (dine-in order with lines).
2. Table picker dialog (free tables).
3. Tables view in "Déplacer une table" mode: source selected, target choice, confirmation.
4. Tables view after a transfer (order shown on the new table).
5. Reports, Tickets tab: a sale with the "Corriger le paiement" action.
6. Payment correction dialog: line choice, new method, confirmation summary; PIN step.
7. Payment correction refused (terminal card) and a sale marked "Paiement corrigé".
8. X report screen / printout with a payment correction (per-method totals, corrections line).
9. Cart with the "Remise" button (manager).
10. Discount dialog: percent, amount, Offert tabs, reasons, preview; PIN step.
11. Cart with an order discount (rows under the lines, net total, VAT summary) and with an offered line.
12. Checkout with a discount (amount due, summary rows).
13. Printed VAT ticket with an order discount and with an offered line; pre-bill with a discount.
14. Z report printout with event and line reductions.
15. Note dialog on a cart line; cart line showing a note.
16. Printed bar slip with a note, the note slip, the transfer notice slip.
17. Manual §1 profiles table screenshot, if the dossier shows it as a capture.

## 12. Follow-ups for the orchestrator (outside the lane)

- F1: regenerate the UC evidence and embed it (`scripts/capture_uc_evidence.sh`, `scripts/embed_uc_evidence.py`) after the three lanes are merged.
- F2: rebuild the dossier PDFs and `CAPTURES_ECRAN_FR.pdf` after the captures of section 11.
- F3: Android device UC scenarios (`android/app/src/androidTest/assets/uc/10_basic/` M122, M132; `40_advanced/` UC223 till path, UC229B) and `docs/android/uc/results.json`, run on the emulator (`uc_e2e.sh`), owner e2e-parity.
- F4: rc3 freeze (tag, `LINUX_FREEZE.md`, freeze SHA in `p4_gate.sh`), which clears the expected `linux-frozen` / `freeze-tag` FAILs.
- F5: check migration numbers 28 to 31 across the three lanes before merging.

---

## 13. Tasks (sequential; each leaves the tree compiling and its gates green)

Every task: read `CLAUDE.md`, this file, and the spec sections of its item; commit in small Conventional Commits (engine, FFI, Vue, Android, docs can be separate commits); append its decisions under "## Lane 2-v1-features (2026-10-17)" at the end of `docs/android/DECISIONS.md` (ids V1F-D25 onward, continue the numbering); list in the summary the gate lines, the deviations listed in fixture tests, and any screen that needs a capture beyond section 11.

### T1: table transfer (M122), engine to both UIs and dossiers
Section 4 in full: builder, `do_transfer_table`, signing kind and context, reports, transfer notice slip, `business_view` transfer view, M122 replay with no deviation, `table_transfer.rs`, Tauri wrapper `transfer_table`, FFI export, API dump, Vue (cart action, picker, Tables move mode, store, Playwright), Compose (cart, picker, Tables move mode, tests, goldens), parity pages, ENGINE/FFI/CLAUDE docs, dossiers column A of section 9.
**Gates before the last commit:** G-RUST, G-FFI, G-PY, G-UI, G-ANDROID, G-DOSSIER.

### T2: payment correction (M132), engine to both UIs and dossiers
Section 5 in full: builder, `do_list_sale_payments`, `do_correct_payment`, signing kind and context, report corrections (turnover, user report, printout), M132 replay (only the CARD type deviation), `payment_correction.rs`, Tauri, FFI, API dump, Vue (Reports tickets action, dialog, PIN, Playwright), Compose (dialog, view model, tests, goldens), parity, docs, dossiers column B (coverage 20 of 22, 18 replays).
**Gates:** G-RUST, G-FFI, G-PY, G-UI, G-ANDROID, G-DOSSIER.

### T3: till discount, engine, schema, reports, fixtures (no UI yet)
Sections 6.2 to 6.4 and the engine part of 6.6: migration 30 (`schema_till.rs`, `TILL_MIGRATIONS`), queries, PRICE_CHANGE marking in the P and N builders, `discount.rs` (`do_apply_discount`, `do_remove_discount`, immediate P rule), pre-bill changes, bar slip skips PRICE_CHANGE entries, cart line fields (`discountCents`, `priceChanges`), HubRise echo discounts, report printout split, `business_view` priceChanges, UC223 / UC225A / UC225B / UC229B replays, `till_discounts.rs`, transfer of a discounted order, Tauri wrappers, FFI exports and records (including the `CartLine` fields), API dump, ENGINE/FFI/CLAUDE docs. The Vue and Compose screens are not changed in this task except what the new `CartLine` fields require to compile (Vue types, Kotlin mapping); the commands have no UI call site yet, so declare them `pending:` in the `ui-parity` block of `docs/android/PARITY.md` only if `check_ui_parity.py --strict` needs it (it reads the Vue `ipc.ts`; if the wrappers are not in `ipc.ts` yet, nothing is needed).
**Gates:** G-RUST, G-FFI, G-PY (without `--strict` pending issues: none expected), G-ANDROID, G-DOSSIER (G-UI too if Vue types changed).

### T4: till discount, both UIs and dossiers
Section 6.5 and the UI part of 6.6: Vue (`ipc.ts`, types, store `addItem` rule, cart "Remise" / "Offrir" / "Retirer la remise", dialog, PIN, net totals and VAT summary, checkout summary, on-screen text keypad for the reason, Playwright), Compose (same, `OrderStore.kt`, order and checkout packages, tests, goldens), parity pages (remove any pending line added in T3), dossiers column C of section 9 (manual §5.3, REPONSES rows, COUVERTURE_ADVANCED_SET, TICKETS_EXEMPLES, FLUX_REMBOURSEMENT, PROFILS, DECLARATION).
**Gates:** G-RUST (Rust untouched: fmt and the cert test are enough if no Rust file changed), G-PY, G-UI, G-ANDROID, G-DOSSIER.

### T5: kitchen notes, engine to both UIs and dossiers
Section 7 in full: migration 31, `do_set_line_note`, bar slip notes, note slip, `printed` tracking, cart field `kitchenNote`, store rule for new lines, `kitchen_notes.rs`, pos-print tests, Tauri `set_line_note`, FFI, API dump, Vue (line action, dialog with the text keypad, cart display, Playwright), Compose (same, tests, goldens), parity, docs, dossiers column D.
**Gates:** G-RUST, G-FFI, G-PY, G-UI, G-ANDROID, G-DOSSIER.

### T6: full gate and fixes
Run the complete gate of the lane brief, in this order, and fix what fails (fix the cause; never weaken a test or a check): `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo clippy -p pili-pos --no-default-features --features cert --all-targets -- -D warnings`; `cargo test --workspace`; `cargo test -p pili-pos --no-default-features --features cert`; `cargo test -p pos-engine --no-default-features --features cert`; the FFI tests in `android/rust` (fmt, clippy, `cargo test --workspace --locked`); `python3 scripts/android/check_ipc_wrappers.py`; `python3 scripts/android/check_command_parity.py` (and `--strict`, `--hygiene`); `python3 scripts/android/check_ui_parity.py` (and `--strict`); `(cd ui && npm run build && npx playwright test)`; `bash scripts/android/p5_gate.sh`; `bash scripts/android/p6_gate.sh`; `python3 scripts/check_dossier.py`; `PILI_DOSSIER_DIR=docs/android/dossier python3 scripts/check_dossier.py`. All through `heavy.sh`. Then a consistency pass: the counts of section 9 in both dossiers, the command counts in `CLAUDE.md`, the decision list complete, no em-dash in the lane diff (`git diff b3248d2d...HEAD | grep -n $'\xe2\x80\x94'` finds nothing), the "Captures to retake" list of section 11 completed with anything the tasks added. Write the gate results (every `GATE |` line, the expected FAILs `linux-frozen` and `freeze-tag` named as such) to `docs/android/phases/LANE_V1_FEATURES-exit.md` and commit it.

## Worker notes

### T1 (table transfer, M122)

- Engine: `do_transfer_table` in `crates/pos-engine/src/commands/table_transfer.rs`; decisions V1F-D25 to D30. The `PendingContext::TableTransfer` side effect moves the order and voids an empty order on the target in the archive transaction.
- Extra Android captures added by T1 beyond section 11: none. Two new Roborazzi goldens (`tables_move`, `order_table_picker`) and the re-recorded `tables_grid` and `tables_takeaways` (new move bar) are committed.
- Command counts after T1: 97 Tauri commands in all, 95 in the cert build (`CLAUDE.md`, `check_ipc_wrappers.py`).
- `scripts/uc_scope.toml` no longer excludes M122; `MATRICE_TRACABILITE_FR.md` and `docs/android/UC_MATRIX_ANDROID.md` were regenerated (M122 now "Couvert" on Linux, "Couvert (tests moteur exécutés sur l'appareil)" on Android; no device scenario was added, so the lane-final task may add `m122` to a device scenario if wanted).
- Dossier counts after T1 (both dossiers): Basic Set 19 of 22 with an automated test, 17 fixture replays in `spf_e2e_fixtures.rs`. The evidence blocks (`evidence/uc/M122_*`) are not regenerated here (V1F-D23).
- Worktree build note: the shared cargo target is clobbered by other lanes' crates of the same names; an untracked per-worktree `.cargo/config.toml` (distinct `codegen-units` for the workspace crates) keeps artifacts apart. It is not committed.

### T2 (payment correction, M132)

- Engine: `crates/pos-engine/src/commands/payment_correction.rs` (`do_list_sale_payments`, `do_correct_payment`), builder `build_payment_correction_input` in `crates/pos-fiscal/src/event.rs`, `Kind::PaymentCorrection` and `PendingContext::PaymentCorrection`, reports in `report_totals.rs` and `commands/report.rs`, label and `ReportDetails.payment_corrections` in `pos-print`. Decisions V1F-D31 to D35.
- Tests: `crates/pos-engine/tests/payment_correction.rs` (23), `m132_payment_correction` in `spf_e2e_fixtures.rs`, FFI tests, `ui/e2e/payment-correction.spec.ts` (8), Compose view model, content and screenshot tests.
- Extra Android captures added by T2 beyond section 11: none. Two new Roborazzi goldens (`reports_tickets_manager`, `reports_payment_correction`); the existing `reports_tickets` golden is unchanged (no manager flag).
- Command counts after T2: 99 Tauri commands in all, 97 in the cert build (`CLAUDE.md`, `check_ipc_wrappers.py`, `docs/android/ENGINE.md` section 6).
- `scripts/uc_scope.toml` no longer excludes M132; both matrices regenerated. Dossier counts after T2 (both dossiers): Basic Set 20 of 22 with an automated test, 18 fixture replays in `spf_e2e_fixtures.rs`. Evidence blocks (`evidence/uc/M132_*`) are not regenerated here (V1F-D23).
- Build notes: Compose builds need `ANDROID_HOME=~/Library/Android/sdk`; the Gradle Kotlin checks (`ktlintCheck detekt`) gate the Compose files.

### T3 (till discount, engine, no screens)

- Engine: `crates/pos-engine/src/commands/discount.rs` (`do_apply_discount`, `do_remove_discount`), migration 30 in `crates/pos-db/src/schema_till.rs`, helpers in `crates/pos-fiscal/src/price_change.rs`; decisions V1F-D36 to D44. Tests: `crates/pos-engine/tests/till_discounts.rs` (31), `uc223_event_discount_at_the_till`, `uc225a_discount_after_registration`, `uc229b_free_line_at_the_till` in `spf_e2e_fixtures.rs` (listed deviations only), FFI conversion tests.
- Tauri `apply_discount` and `remove_discount`, FFI exports, API dump regenerated, Vue types, mock and i18n error texts (regenerated Android mirrors). No screens: T4.
- Command counts after T3: 101 Tauri commands in all, 99 in the cert build.
- Captures to retake: none new beyond section 11 items 9 to 14 (the screens arrive with T4).
- Not committed: the untracked per-worktree `.cargo/config.toml`.

### T4 (till discount, screens and dossiers)

- Vue: `DiscountDialog.vue`, `TextKeypad.vue` (shared with `EmailKeypad.vue`), `utils/cartAmounts.ts`, store `applyDiscount` / `removeDiscount`, cart, line, VAT summary and checkout changes; `ui/e2e/till-discount.spec.ts` (20 tests). Android: `feature/order/DiscountDialog.kt`, `OrderStore`, cart, line and checkout panels; unit, Compose and Roborazzi tests (goldens `order_discount_dialog`, `order_cart_discount`, `order_cart_offered`, `checkout_discount`). Parity pages `order.md` and `checkout.md` updated. Decisions V1F-D45 to D50.
- Dossiers (both): manual 5.3, cert answers (Reductions, Modification de prix ligne and event, Article offert, XV), tickets, lines, refund, profiles, forbidden functions, feature list, process, data flow; schema docs for migration 30 (Linux only).
- Captures to retake: section 11 items 9 to 14, plus the Vue reason-text keypad (`discount-reason-text`) and the Android discount dialog, cart with a discount, offered line and checkout with a discount.
