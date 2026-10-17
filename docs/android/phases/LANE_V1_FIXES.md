# Lane 1 (v1 fixes): plan

Branch `android/lane-v1-fixes`, worktree `moyun-pos-wt/lane-v1-fixes`, base `android/int` at `b3248d2d`. Planner: Opus 5.5. Workers: Sonnet 5.5. Reviewer: Opus 5.5. Written 2026-10-17.

These items go into the certified v1.0.0 (Linux and Android). The orchestrator makes the rc3 freeze after the merge; this lane never tags, pushes or merges.

## 1. Goal

1. **A. Presence of staff who never use the till.** A manager clocks any active employee in and out from the employees panel of Settings, on both UIs, with an S event signed by the FDM in that employee's INSZ. Kitchen staff can be registered as "presence only" users who can never log in.
2. **B. Meal-voucher cards on the linked terminal.** A card the linked terminal reports as Edenred, Pluxee (Sodexo), Monizze or Payflip is signed as `CHEQUE_MEAL` (`AUTOMATIC`, brand as `provider`, terminal reference), counted under meal vouchers in X/Z, printed as meal vouchers on the ticket and its copies, and labelled as meal vouchers on both checkout screens.
3. **C. Dossier consistency.** Reserved migration range, the "v1.1" invoice promise, the Advanced Set coverage figures, the CLAUDE.md UI tree.

## 2. Scope and non-goals

In scope: the engine (`crates/pos-db`, `crates/pos-core`, `crates/pos-fiscal`, `crates/pos-eft`, `crates/pos-engine`), the Tauri shell (`src-tauri`), the FFI (`android/rust/pos-ffi`, `android/pos-core/api/pos-core.api`), both UIs (`ui/`, `android/app`), their tests (cargo, FFI, Playwright, Kotlin unit, Roborazzi goldens that change), both dossiers and their PDFs, `CLAUDE.md`, `docs/android/{DECISIONS,PARITY,ENGINE,FFI}.md`, `docs/android/parity/{settings,checkout,login}.md`.

These paths are `frozen` in `docs/android/LANES.md` (Linux build inputs, FFI, dossier). The user asked for this lane (v1 must-fix before rc3), so the lane edits them; nothing else under `frozen` is touched (no `scripts/build_and_hash.sh`, no `scripts/check_repro.sh`, no `docs/android/baseline/`).

Non-goals (record them, do not build them):
- Editing an existing user's role or presence-only flag (deactivate, then create again with the same INSZ: the existing reactivation path).
- Showing who registered a clock-in on screen (stored for audit, not displayed).
- Routing the "Chèques-repas" method through the linked terminal: the manual meal-voucher path stays exactly as today (`MANUAL`, `CHEQUE_MEAL`, no provider).
- Eco-cheques, gift and sport/culture cards from the same issuers (`CHEQUE_OTHER`): unchanged (signed as today), recorded as an open point.
- A manual override "this terminal card was a meal voucher" when the terminal reports a scheme brand (Mastercard) for a meal-voucher card.
- Reordering payment lines so meal vouchers come first (the till rounds cash only, spec §7.1, so the order changes no amount; lines stay in the order taken, as for manual meal vouchers today).
- Reversal of an approved terminal payment, refunds to a card (refunds stay cash).
- Retaking dossier screenshots (section 10 lists them for the orchestrator).
- Version bumps, tags, rc3, `LINUX_FREEZE.md`, the `plat_about_engine_ref` string (still `linux-v1.0.0-rc2` until the orchestrator re-freezes).

## 3. Spec references

| Topic | Reference |
|---|---|
| Presence must be possible for users who do not use the POS | `docs/20250307_POS_CERTIFICATION_MANUEAL_FRV1.1/20250307_FR - Manuel de certification_POS_V11.md` §II.E "Enregistrement de la présence" (lines 799 to 825) |
| WorkIn/WorkOut input: only `employeeId` names a person | `docs/20260821_UC_BasicSet_POS_FDM_1.8/M140_signWorkIn/*`, `M141_signWorkOut/*` |
| Training: every registration made in training is a training event, label T | POS spec v2.2 §1.5 (`docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md` lines 312 to 320); `M170_signWorkIn_Training` (response `eventLabel` "T") |
| Meal vouchers: fixed value, never rounded, no change, processed first | POS spec v2.2 §7 "Arrondissement des paiements" (lines 605 to 690, the "Attention" paragraph and §7.1) |
| `CHEQUE_MEAL`, `provider` (example "Edenred"), `inputMethod` `AUTOMATIC` ("le terminal de paiement renvoie lui-même le type de carte utilisé") | POS spec v2.2 `paymentLine`, `tb_paymentTypes`, `tb_inputMethod` (lines 1730 to 1800) |
| A meal voucher line with a provider | `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/UC4119_SplitPayment/UC4119_2_Sale/UC4119_2_signSale_GQL-Variables.json` (`"provider": "PLUXEE"`) |
| Advanced Set folder count | `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/`: 58 UC folders plus `CHANGELOG-UC_AdvancedSet_1.6.md` (59 entries) |
| Reserved migrations | `crates/pos-db/src/migrate.rs` (comment after version 11: "Versions 12 to 19 are kept for the certified build") |
| No Linux v1.1 filing | `docs/android/DECISIONS.md` §10 |

## 4. Item A: presence registration

### 4.1 Current state

- `commands/social.rs`: `do_work_in` / `do_work_out` require the logged-in operator to be the employee (`access::operator_named`, `Access::Staff`). The presence worker clocks in at login, `do_end_shift` clocks out. No UI calls `work_in` / `work_out` (`docs/android/PARITY.md` declares them excluded). So a manager cannot clock in anyone else: the dossier claim in `REPONSES_MANUEL_CERT_FR.md` §III.E ("L'écran de gestion des utilisateurs ... permet à un manager de pointer des employés") is false today, and the same section mentions a `users.social_status` column that does not exist.
- `staff_shifts` (migration 9) is the local index of open shifts, written in the sign commit (`signing.rs`, `PendingContext::Work`). It is written for training S events too: a waiter who logs in while training mode is on gets a T WorkIn that opens a local shift, so after training is switched off the next real login skips the real WorkIn (`clock_in_if_off_shift` sees "on shift"). That is a real presence never registered.
- A waiter without a PIN logs in by tapping their tile: a kitchen worker created as a waiter could take orders.

### 4.2 Decisions (recorded by T1 in DECISIONS)

- **L1F-A1 Presence-only users.** A flag, not a new role: `users.presence_only` (0/1). A presence-only user has role `waiter` in the database (least rights, never used), cannot log in (`login`, `login_with_new_pin`, `end_shift` all refuse with `presence_only_user`), cannot get a PIN (`set_pin` refuses with `presence_only_user`), is not shown on the login tiles of either UI, and is clocked in and out only by a manager from Settings. Their S events appear in the user reports like anyone's. Rationale: a fourth `UserRole` needs a rebuild of `users` (SQLite cannot alter a CHECK) and touches the role matrix, the FFI enum and every role label; a nullable-free flag added with `ALTER TABLE ADD COLUMN` keeps the model simple.
- **L1F-A2 Manager clock-in/out commands.** `clock_in_employee(employee_insz)` and `clock_out_employee(employee_insz)`, managers only (`Access::Admin`). The S event is the employee's (`employeeId` and `fiscal_events.user_insz` = the employee, as M140/M141); the manager who did it is stored in `staff_shifts.started_by` / `ended_by` (audit) and logged.
- **L1F-A3 Refusals, in this order** (nothing signed, no counter moved): not a manager (`forbidden`); training mode on (`presence_training`); the system identity (`system_user`); unknown or inactive employee (`user_unknown`); the operator themself (`presence_self`: own presence goes through login and "Fin de service"); clock-in while the local index says on shift (`presence_already_in`); clock-out while not on shift (`presence_not_in`).
- **L1F-A4 Training mode.** A manager-driven clock-in or out is refused in training mode. The spec allows a training WorkIn (M170, label T) and the login path keeps doing it, but a T event registers no presence, so registering someone else's presence in training would be misleading; refusing is the conservative reading. In addition, **training S events never touch `staff_shifts`**: the local index reflects real (S-labelled) presence only. Consequences, accepted: in training a login sends a T WorkIn at every login (harmless, never counted), and an end of shift in training sends a T WorkOut and leaves the real shift open (the person was really present).
- **L1F-A5 FDM down.** No extra status query: the command signs like the login clock-in. When the FDM is unreachable or refuses, the command returns `success: false` with the FDM error (text starting with "FDM", shown as an FDM message), nothing is archived and the local index does not change. Both UIs also disable the two presence buttons and show "FDM indisponible : pointage impossible" while their FDM store says the FDM is not connected.
- **L1F-A6 Deactivation of an employee on shift** is refused (`deactivate_on_shift`): otherwise the open shift could never be closed (an inactive user cannot be clocked out). The manager records the departure first.
- **L1F-A7 Migration number 28** (`presence-registration`), in the base `MIGRATIONS` list (not feature-gated): 12 to 19 stay reserved for certified hotfixes, 20 to 27 are taken. If another lane also claims 28, the orchestrator renumbers at the merge (references: `migrate.rs`, `SCHEMA_BD_FR.md` §3.8 and §6, `SCHEMA_ER_FR.md` §7, `CLAUDE.md` migrations sentence).

### 4.3 Data model (migration 28)

```sql
ALTER TABLE users ADD COLUMN presence_only INTEGER NOT NULL DEFAULT 0
    CHECK (presence_only IN (0, 1));
ALTER TABLE staff_shifts ADD COLUMN started_by TEXT;  -- INSZ of who registered the clock-in
ALTER TABLE staff_shifts ADD COLUMN ended_by TEXT;    -- INSZ of who registered the clock-out
```

Rows from before the migration keep NULL in `started_by` / `ended_by`. Add `MIGRATION_028_PRESENCE_REGISTRATION` to `crates/pos-db/src/schema.rs`, its entry in `MIGRATIONS` (version 28, name `presence-registration`), and migration tests (column defaults, CHECK refuses 2, existing users keep 0, a reopened DB does not re-run it).

### 4.4 Engine

- `pos_core::user::User`: add `presence_only: bool` (serde field `presence_only`, like `has_pin`). `pos-db` `get_user`, `list_users`, `create_user` (insert and the reactivation `UPDATE` both set it) read/write the column. `do_list_users` still returns every active user (the settings list needs them); the login screens filter.
- `CreateUserInput`: add `#[serde(default)] presence_only: bool`. When true: `role` must be `waiter` (else `role_invalid`), `initial_pin` / `operator_pin` are ignored, no PIN is stored.
- `auth::authenticate`: after the user lookup, a presence-only user is refused with `presence_only_user` (before any PIN check, so no PIN throttle entry). `do_login_with_new_pin` and `do_set_pin` refuse a presence-only target with `presence_only_user`.
- `auth::do_deactivate_user`: refuse with `deactivate_on_shift` when the user has an open shift.
- `social.rs`: factor the signing of `do_work_event` into `sign_work_event(state, employee_insz, is_in, by_insz)`; `do_work_in` / `do_work_out` keep their access check and pass the employee as `by`; add `do_clock_in_employee(state, employee_insz)` and `do_clock_out_employee(state, employee_insz)` with the checks of L1F-A3 (training checked from `state.config`, on-shift from `queries::shifts`), returning `SocialResult` (`Err(code)` for the refusals, `Ok(SocialResult{success:false,..})` for an FDM failure, exactly like `do_work_in`).
- `PendingContext::Work` gains `#[serde(default)] by: Option<String>` (the context JSON of rows already in `pending_signs` must still parse). The commit opens/closes the shift only when the event is not a training event, and records `by` in `started_by` / `ended_by`. `queries::shifts::open_shift` / `close_shift` take `by: Option<&str>`.
- New error codes in `commands/errors.rs` (and `errors::ALL`): `presence_only_user`, `presence_training`, `presence_self`, `presence_already_in`, `presence_not_in`, `deactivate_on_shift`. The FFI test `every_code_of_both_lists_maps_to_coded` counts 113 today: it becomes 119.
- Shell: `src-tauri/src/ipc/social.rs` wrappers `clock_in_employee`, `clock_out_employee` (one engine call each, `async`), registered in `src-tauri/src/main.rs`. Wrapper count 96 to 98 (94 to 96 without peppol).
- FFI: `PosEngine::clock_in_employee` / `clock_out_employee` in `android/rust/pos-ffi/src/api/reports.rs` next to `work_in` (same `spawn` pattern), `User` and `CreateUserInput` records gain `presence_only` (`android/rust/pos-ffi/src/records/auth.rs`), records index comment in `records/mod.rs`, FFI tests (`android/rust/pos-ffi/tests/ffi/`), error count 119, `android/pos-core/api/pos-core.api` regenerated (`(cd android && ../scripts/android/heavy.sh ./gradlew :pos-core:apiDump)`), the generated table of `docs/android/FFI.md` §11 regenerated with `python3 scripts/android/check_command_parity.py --markdown` (and the command count in §1/§11 prose).
- Template for an end-to-end command: commit `c3488d84` (`list_unsettled_card_payments`).

### 4.5 UIs (identical behaviour, FR/ZH/EN)

Employees panel of Settings (Vue `ui/src/views/SettingsView.vue`; Android `feature/settings/SettingsScreen.kt` `EmployeesPanel` / `WorkerRow`, `SettingsViewModel.kt`, `SettingsState.kt`):
- The panel loads `list_users` and `list_open_shifts` together and refreshes both after every presence action, PIN change, creation and deactivation.
- Each row of another active employee shows, for a manager: the presence state ("En service depuis 14:02" with the Brussels local time of `startedAt`, or "Hors service") and one button: "Arrivée" when off shift, "Départ" when on shift. The operator's own row shows the state but no presence button. Success: toast "Arrivée enregistrée · <nom>" / "Départ enregistré · <nom>". `success: false`: the FDM error shown like other FDM failures (existing mapping), state unchanged. A refusal code: its error text.
- Buttons disabled, with the note "FDM indisponible : pointage impossible", while the FDM store says not connected; disabled with the `presence_training` text as note while training mode is on.
- A presence-only row: profile label "Présence seule", no "Code PIN" button, "Désactiver" kept.
- Add form: a fourth profile choice "Présence seule"; it sends `role: "waiter", presenceOnly: true` and never opens the PIN dialog.
- Login screens (Vue `LoginView.vue`, Android `feature/login/LoginViewModel.kt`): presence-only users are not on the tiles. "Autre (INSZ)" with a presence-only INSZ shows the `presence_only_user` error.
- A one-line panel note: "Personnel sans accès à la caisse (cuisine) : le Gérant enregistre son arrivée et son départ ici."

Strings (Vue sources, then `node scripts/android/vue_to_android.mjs --write` for Android; Android-only strings in `strings_ui_settings.xml` / `strings_ui_login.xml` with en and zh twins). Proposed texts, adjust ZH to the existing terminology of `i18n.ts` (for example the words already used for "Fin de service" and "en service"):

| Key | fr | en | zh |
|---|---|---|---|
| `role_presence` (i18n.ts) | Présence seule | Presence only | 仅考勤 |
| `settings_presence_in` | Arrivée | Clock in | 上班打卡 |
| `settings_presence_out` | Départ | Clock out | 下班打卡 |
| `settings_on_shift_since` | En service depuis {time} | On shift since {time} | {time} 起在岗 |
| `settings_off_shift` | Hors service | Off shift | 不在岗 |
| `settings_presence_done_in` | Arrivée enregistrée · {name} | Clock-in registered · {name} | 已登记上班 · {name} |
| `settings_presence_done_out` | Départ enregistré · {name} | Clock-out registered · {name} | 已登记下班 · {name} |
| `settings_presence_fdm_down` | FDM indisponible : pointage impossible | FDM unavailable: clocking in or out is not possible | FDM 不可用，无法打卡 |
| `settings_presence_note` | Personnel sans accès à la caisse (cuisine) : le Gérant enregistre son arrivée et son départ ici. | Staff without till access (kitchen): the manager registers their clock-in and clock-out here. | 不使用收银机的员工（厨房）：由经理在此登记上下班。 |
| `err_presence_only_user` | Employé enregistré pour la présence seulement : pas de connexion à la caisse | This employee is registered for presence only and cannot log in | 该员工仅用于考勤，不能登录收银机 |
| `err_presence_training` | Mode formation : le pointage d'un employé est impossible. Quittez le mode formation. | Training mode: an employee cannot be clocked in or out. Leave training mode. | 培训模式下不能为员工打卡，请先退出培训模式。 |
| `err_presence_self` | Votre arrivée et votre départ se font à la connexion et par « Fin de service » | Your own clock-in and clock-out happen at login and with "End of shift" | 您本人的上下班在登录时和"结束工作"时登记 |
| `err_presence_already_in` | Cet employé est déjà en service | This employee is already on shift | 该员工已在岗 |
| `err_presence_not_in` | Cet employé n'est pas en service | This employee is not on shift | 该员工不在岗 |
| `err_deactivate_on_shift` | Cet employé est en service : enregistrez d'abord son départ | This employee is on shift: clock them out first | 该员工在岗，请先登记下班 |

Error keys follow the existing `err_<code>` convention of `ui/src/i18n.ts` (check `ui/src/utils/errorMessages.ts`).

### 4.6 Tests

- `pos-db`: migration 28 tests; `shifts` open/close with `by`; `users` presence-only read/write and reactivation.
- Engine (`crates/pos-engine/tests/presence.rs`, or a new `presence_manager.rs`): a manager clocks a presence-only employee in (one `signWorkIn` with `employeeId` = the employee, label S, `staff_shifts.started_by` = manager) and out; every refusal of L1F-A3 signs nothing (FDM wiremock receives nothing, counters unchanged); an FDM failure returns `success: false`, archives nothing, leaves the index; training: the manager command is refused, and a login in training sends a T WorkIn that leaves `staff_shifts` unchanged (then, training off, the next login signs a real WorkIn); a presence-only user's login, first-PIN login, end of shift and `set_pin` are refused with `presence_only_user`; `deactivate_user` refused while on shift; a pending sign stored with the old `Work` context JSON (no `by`) still recovers; the user X report lists the presence-only employee with their presence. `access_control.rs`: the two commands refused for cashier and waiter. `error_codes.rs` (or wherever codes are enumerated): the six codes.
- FFI tests: the two methods, the new record fields, error count 119.
- Playwright (`ui/e2e/settings.spec.ts`, `login.spec.ts`, `fixtures/mock-ipc.ts`): presence state and buttons, clock in then out, FDM down disables, training disables, presence-only add, row without PIN button, login tiles without presence-only users.
- Kotlin: `SettingsViewModelTest`, `SettingsContentTest`, `SettingsDialogsTest` if touched, `LoginViewModelTest`; re-record only the Roborazzi goldens that change (`android/app/src/test/screenshots/settings/settings_users.png`, maybe `settings_main.png`) and look at them.

### 4.7 Documentation (both dossiers unless noted)

- `REPONSES_MANUEL_CERT_FR.md` §III.E: rewrite the description (login clock-in, end of shift, the manager registration from "Config > Employés", the presence-only profile, training and FDM rules, `staff_shifts` with `started_by`/`ended_by` as the local index, drop `users.social_status`), name the real functions (`commands::social::do_clock_in_employee`, ...; `scripts/check_dossier.py` checks backticked Rust identifiers).
- `MANUEL_UTILISATEUR_FR.md` §3 (presence), §12.3 (Employés), §14.2 table (new messages).
- `PROFILS_UTILISATEURS_FR.md`: the presence-only profile (no login, no PIN, no rights), the role table of §3, §6 if a limitation changes.
- Linux only: `SCHEMA_BD_FR.md` §3.8 `users` and §3.8 bis `staff_shifts` (new columns), §6 (twenty migrations: 1 to 11, 20 to 28), `SCHEMA_ER_FR.md` §7 (same count and list). Android `INDEX_DOSSIER_FR.md` if it cites the migration count.
- Grep both dossiers for `signWorkIn`, `pointage`, `work_in` and fix any other description of presence (`DOCUMENT_PROCESSUS_TECHNIQUE_FR.md`, `LISTE_FONCTIONNALITES_FR.md`, `MAPPING_CONFORMITE_FR.md`, `FLUX_DONNEES_FR.md`).
- `docs/android/PARITY.md`: a row for the employees panel with `clock_in_employee clock_out_employee list_open_shifts`; keep the `work_in` / `work_out` exclusion lines (still no Vue caller) with the note updated. `docs/android/parity/settings.md`, `docs/android/parity/login.md`. `docs/android/ENGINE.md` (command list, presence rules). `CLAUDE.md`: wrapper counts, migrations sentence (28), presence in "UI rules owned by the engine" if relevant.
- Rebuild the PDFs of every changed annex: `scripts/make_dossier_pdfs.sh <files>` and `PILI_DOSSIER_DIR=docs/android/dossier scripts/make_dossier_pdfs.sh <files>` (an Android `commun:` document has no Android PDF: see its MANIFEST).

## 5. Item B: meal-voucher cards on the linked terminal

### 5.1 Current state

`crates/pos-eft/src/brand.rs::normalize` knows no meal-voucher brand, so an Edenred card comes back as brand "EDENRED" (raw), card type `Unknown`, and the sale signs a `CARD_UNKNOWN` line (id "7"). `eft_payments.card_type` has `CHECK(card_type IN ('debit','credit','unknown'))` and an append-only trigger, so a new card type value would need a table rebuild.

### 5.2 Decisions (recorded by T3 in DECISIONS, under a "### EFT" heading of the lane section)

- **L1F-B1 Classification by brand, at signing time, no migration.** `pos_eft::brand::normalize` maps the meal-voucher spellings to four canonical names, and `pos_eft::brand::is_meal_voucher(brand: Option<&str>) -> bool` is true for exactly those four. `eft_payments` keeps storing card type `unknown` and the normalized brand; `commands/eft.rs::terminal_cards` builds `TerminalCard { card_type: PaymentType::ChequeMeal, .. }` when the stored brand is a meal-voucher brand. The brand is normalized when the approval is stored, so the classification is fixed by what is stored.
- **L1F-B2 Spellings** (key = lowercase ASCII alphanumerics of the raw value, as `normalize` already does): `edenred`, `ticketrestaurant`, `edenredticketrestaurant`, `edenredmeal`, `edenredmealvoucher`, `edenredrestaurant` -> "Edenred"; `pluxee`, `sodexo`, `sodexopass`, `sodexolunchpass`, `lunchpass`, `sodexomeal`, `sodexomealvoucher`, `sodexocard`, `pluxeemeal`, `pluxeemealvoucher`, `pluxeecard`, `pluxeelunchpass` -> "Pluxee"; `monizze`, `monizzemeal`, `monizzemealvoucher`, `monizzecard` -> "Monizze"; `payflip`, `payflipmeal`, `payflipmealvoucher` -> "Payflip". Anything with another qualifier (`eco`, `ecocheque`, `gift`, `cadeau`, `sport`, `culture`, `consumption`) stays raw and is not a meal voucher. `receipt::brand_in` also recognizes EDENRED, TICKET RESTAURANT, PLUXEE, SODEXO, MONIZZE, PAYFLIP (CCV cloud reads the brand from the receipt when the circuit is missing). Exact matches only, never a prefix match: an eco-cheque card must never become `CHEQUE_MEAL`. These spellings are an assumption until a trace of a real Belgian terminal is available (same caveat as the OPI driver header); record it.
- **L1F-B3 The signed line.** `pos_fiscal::payment::SettledPayment::payment_type` accepts `CHEQUE_MEAL` from the terminal (`CardDebit | CardCredit | ChequeMeal` kept, anything else `CardUnknown`). The line is `id "3"`, `name "Meal voucher"` (the identity of the manual meal voucher, `payment_line_identity`), `type CHEQUE_MEAL`, `inputMethod AUTOMATIC`, `provider` = brand, `reference` = terminal reference. Same id and name as the manual path, so X/Z and user reports count both in one meal-voucher row (they differ by `inputMethod` only), exactly like the manual and terminal debit cards share id "2". The part stays method `Card` in `PaymentPart` (the engine still requires `part.method == Card` for an `eft_payment_id`), exact amount, never rounded, no change.
- **L1F-B4 Local payment history.** `checkout.rs` stores the order's `payments` row as `meal_voucher` (not `card`) when the settled payment's type is `CHEQUE_MEAL`.
- **L1F-B5 Ticket and copies.** The original ticket prints "Chèques-repas Edenred" (method label then brand, as for cards). `pos_fiscal::ticket::printed_payments` (used by copies) must print the same: the brand is printed for a `CARD*` line or for any `AUTOMATIC` line with a provider (today only `CARD*`, so a copy would drop the brand and differ from the original). The existing test `a_copy_of_a_terminal_card_sale_prints_the_brand_too` keeps its expectations (its `CHEQUE_MEAL` row has no `inputMethod`).
- **L1F-B6 Screens.** `commands/eft.rs::EftPayment` gains `meal_voucher: bool` (from `is_meal_voucher(brand)`); `card_type` keeps `debit|credit|unknown`. Both checkout screens label an approved terminal part with the meal-voucher method label when `meal_voucher` is true ("Part 1 · Chèques-repas · Edenred"), the card label otherwise. The unsettled-card warning of the Z step and the table badge are unchanged (it is still money charged on the terminal).
- **L1F-B7 Scheme-branded meal cards.** A meal-voucher card that the terminal reports under a scheme brand (for example "Mastercard") is signed as a card: the spec says the terminal returns the card type itself. Open point for the user (section 9).

### 5.3 Driver check (tests in `crates/pos-eft`)

- Worldline: `Card.CardBrand` (`worldline.rs` approval parse, around line 595).
- CCV cloud: `details.cardCircuit` (escaped JSON string or object), fallback `receipt::brand_in` (`ccv_cloud.rs` around line 479 to 560).
- CCV OPI: `CardCircuit` attribute of the authorisation (`ccv_opi.rs` around line 646).
- Add one test per driver approving with an Edenred spelling (for example "EDENRED", "Ticket Restaurant", "SODEXO") that yields the canonical brand, and a `mock_tests.rs` test through `pos-eft-mock` with brand "Edenred" on the three protocols. Add a brand test list for every spelling of L1F-B2 and for the eco/gift negatives. `pos-eft-mock` needs no change (the brand is free text); mention in its startup hint that `"brand":"Edenred"` simulates a meal-voucher card if useful.

### 5.4 Tests

- `pos-fiscal`: `SettledPayment::payment_type` with a `ChequeMeal` terminal card; `financial_lines` writes `CHEQUE_MEAL`, `AUTOMATIC`, provider, reference; `printed_payments` prints the brand of an `AUTOMATIC` `CHEQUE_MEAL` line; report totals group a manual and an automatic meal voucher in one row (`report_totals.rs`).
- Engine (`crates/pos-engine/tests/eft_signing.rs`, scripted terminal): a sale paid by an Edenred approval plus cash signs `financials` [CHEQUE_MEAL AUTOMATIC provider Edenred reference, CASH (+ ROUNDING)]; a sale with a manual meal voucher, a terminal Edenred card and a terminal Bancontact card; the X and Z (turnover and user) totals have one meal-voucher row with both amounts; the printed ticket text and a copy's text both show "Cheques-repas Edenred" (printer labels drop accents: check `pos-print/src/labels.rs`); the order's `payments` row is `meal_voucher`; `list_eft_payments` shows `meal_voucher: true`; the HubRise echo names the payment "Meal voucher" (if the echo is testable there). Training never charges the terminal (unchanged).
- FFI: `EftPayment` record field `meal_voucher`; FFI eft tests.
- Playwright `ui/e2e/eft.spec.ts` + `fixtures/mock-ipc.ts` (add `meal_voucher` to the mocked payments, an Edenred step): the locked part reads "Chèques-repas · Edenred". Kotlin: `CheckoutViewModelTest` / panel test for the label; re-record an `eft/` golden only if one changes.

### 5.5 Documentation

- Both `MANUEL_UTILISATEUR_FR.md` §7.2 ("Carte sur le terminal relié"): a meal-voucher card (Edenred, Pluxee, Monizze, Payflip) is paid with "Carte" when a terminal is linked; the till recognizes it by the brand the terminal returns and records it as meal vouchers; the "Chèques-repas" button stays the manual entry.
- Every dossier text describing the terminal card types (grep `CARD_UNKNOWN`, `terminal relié`, `CHEQUE_MEAL`): `DOCUMENT_PROCESSUS_TECHNIQUE_FR.md` (around line 305), `FLUX_DONNEES_FR.md` (around 175), `LISTE_FONCTIONNALITES_FR.md` (around 141), `REPONSES_MANUEL_CERT_FR.md` (payment rows around 485, meal voucher row), `COUVERTURE_RAPPORTS_FR.md` (around 158), `LIGNES_PRODUITS_FR.md` (around 92), Android `MANUEL_INSTALLATION_FR.md`, both `TICKETS_EXEMPLES_FR.md` (add or adjust an example if the wording of a terminal meal-voucher line is shown).
- `CLAUDE.md` "Payment terminal (EFT)": the signed line is `CHEQUE_MEAL` for a meal-voucher brand.
- `docs/android/ENGINE.md` / `FFI.md` if they describe `EftPayment`; `docs/android/parity/checkout.md`.
- Rebuild the changed PDFs (both dossiers).

## 6. Item C: dossier consistency (docs only)

1. **Reserved migrations: 12 to 19** (`migrate.rs` is the truth; 11 is the table plan). Fix: `docs/SUBMISSION_ARTIFACTS/REPONSES_MANUEL_CERT_FR.md` (line 187), `docs/android/dossier/REPONSES_MANUEL_CERT_FR.md` (line 195), `docs/android/dossier/GARANTIE_MAJ_LANGUES_FR.md` (line 149), `docs/android/dossier/INDEX_DOSSIER_FR.md` (line 83). Re-grep before committing: `git grep -n -E "11 (à|a|to) ?19|11-19"` across `docs/` (excluding the SPF sources `docs/202*`) and `CLAUDE.md`. Also check the Linux `GARANTIE_MAJ_LANGUES_FR.md`.
2. **No version number for the invoice update.** DECISIONS §10: the Linux build is filed as 1.0.0 and there is no Linux v1.1. Replace "mise à jour certifiée prévue v1.1", "prévu v1.1", "En attendant v1.1", "dossier de mise à jour v1.1", "déploiement v1.1" and similar by wording without a version ("une mise à jour certifiée ultérieure", "jusqu'à cette mise à jour", ...), keeping the meaning (event I and Peppol out of v1.0.0, a later certified update under the official procedure). Files (21 matches today): Linux `COUVERTURE_FIXTURES_FR.md`, `DECISION_PEPPOL_AP_FR.md`, `MAPPING_CONFORMITE_FR.md`, `REPONSES_MANUEL_CERT_FR.md`, `RESULTATS_TESTS_UC_FR.md` (line 2710); Android `MAPPING_CONFORMITE_FR.md` and any Android copy found by `git grep -n -E "v ?1\.1([^0-9]|$)" -- docs/SUBMISSION_ARTIFACTS docs/android/dossier`. Do not touch "Manuel ... V1.1" (the SPF manual's own version) nor FPSFIN "1.10".
3. **`COUVERTURE_ADVANCED_SET_FR.md`** (both): "59 dossiers UC" -> 58 (59 entries in the folder include the changelog; verify with `ls -d docs/20260821_UC_AdvancedSet_POS_FDM_1.6/*/ | wc -l`); verify the 141 fixtures figure by running `scripts/android/heavy.sh cargo test --package pos-fiscal --test advanced_set_fixtures -- --nocapture` and fix it if wrong; drop the stale "Total tests workspace : 190 passent" line (Linux) and point to `RAPPORT_TESTS_BASELINE_FR.md` for counts; Linux row "`ModifierDialog.vue`, `PriceChangeDialog.vue` | Non implémenté | Phase 10": `ModifierDialog.vue` shipped for HubRise options (same wording as the Android row "Options d'un produit"), price changes at entry stay not implemented; replace the internal "Phase 10" target in both tables and in the "Impact cert" paragraphs by "mise à jour certifiée ultérieure" (no phase name, no version).
4. **`CLAUDE.md` workspace tree**: list the real files. UI: views (`CheckoutView`, `LoginView`, `OrderView`, `ReportsView`, `SettingsView`, `TablesView`, `WebshopOrdersView`), components (the 27 real files of `ui/src/components/`, at least the ones a reader needs: `AppHeader`, `CartSidebar`, `CategoryTabs`, `MenuGrid`, `MenuItemButton`, `ModifierDialog`, `NumPad`, `OrderLineItem`, `PinDialog`, `PriceDialog`, `StatusLights`, `TrainingBanner`, `VatSummary`, `EftOverlay`, `FdmAlerts`, ... or all of them), stores (11 real files), `composables/useOrderChime.ts` (the three listed composables do not exist), `utils/`, the `i18n*.ts` files, no `public/fonts` (find where fonts really come from before writing it). Remove `ReceiptView.vue`, `FdmStatus.vue`, `PaymentButtons.vue`, `OrderLine.vue`, `useVat.ts`, `usePrint.ts`, `useCheckout.ts`, `public/fonts`. Verify the crate trees too (`pos-core`, `pos-fiscal`, `pos-print`, `pos-db`) and add one line per crate missing from the tree (`pos-eft`, `pos-catalog`, `pos-hubrise*`, `pos-mail`, `pos-export*`, `pos-peppol`, the mocks, `pos-eft-probe`).
5. Rebuild the PDFs of every annex changed in this item (both dossiers).

## 7. Task split (sequential)

Common to every task: the environment prefix of the lane brief on every Bash call; heavy commands through `scripts/android/heavy.sh`; `df -g /` before heavy builds (under 10 GB: clean this worktree's targets; under 6 GB after that: stop with `DISK_LOW`); no emulator, no adb, no network to real vendors; Conventional Commits ending with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`; never an em-dash; decisions appended only at the end of `docs/android/DECISIONS.md` under `## Lane 1-v1-fixes (2026-10-17)` (T1 creates the heading; later tasks append below it); `(cd ui && npm ci)` once if `ui/node_modules` is missing.

Gate shorthands used below (run from the worktree root):
- **G-RUST**: `scripts/android/heavy.sh cargo fmt --all -- --check`; `scripts/android/heavy.sh cargo clippy --workspace --all-targets -- -D warnings`; `scripts/android/heavy.sh cargo clippy -p pili-pos --no-default-features --features cert --all-targets -- -D warnings`; `scripts/android/heavy.sh cargo test --workspace`; `scripts/android/heavy.sh cargo test -p pili-pos --no-default-features --features cert`; `scripts/android/heavy.sh cargo test -p pos-engine --no-default-features --features cert`.
- **G-FFI**: `(cd android/rust && cargo fmt --all -- --check && ../../scripts/android/heavy.sh cargo clippy --workspace --all-targets --locked -- -D warnings && ../../scripts/android/heavy.sh cargo test --workspace --locked)` (see `scripts/android/p2_gate.sh` `in_rust` for the exact environment it uses).
- **G-PARITY**: `python3 scripts/android/check_ipc_wrappers.py`; `python3 scripts/android/check_command_parity.py`; `python3 scripts/android/check_ui_parity.py`; `node scripts/android/vue_to_android.mjs --check`; `python3 scripts/android/lane_paths.py --check`.
- **G-VUE**: `(cd ui && npm run build)`; `(cd ui && ../scripts/android/heavy.sh npx playwright test)`.
- **G-ANDROID**: `bash scripts/android/p5_gate.sh` and `bash scripts/android/p6_gate.sh` (both through heavy.sh internally; read their headers).
- **G-DOSSIER**: `python3 scripts/check_dossier.py`; `PILI_DOSSIER_DIR=docs/android/dossier python3 scripts/check_dossier.py`.

| Task | Title | Gates before its commits |
|---|---|---|
| T1 | Presence registration: database, engine, shell, FFI | G-RUST, G-FFI, G-PARITY, `(cd android && ../scripts/android/heavy.sh ./gradlew :pos-core:apiCheck :app:compileDebugUnitTestKotlin)` |
| T2 | Presence registration: both UIs, strings, tests, docs | G-PARITY, G-VUE, G-ANDROID, G-DOSSIER, `cargo test -p pos-engine --test presence` (sanity) |
| T3 | Meal-voucher cards on the terminal as CHEQUE_MEAL | G-RUST, G-FFI, G-PARITY, G-VUE, G-ANDROID, G-DOSSIER |
| T4 | Dossier consistency and CLAUDE.md tree | G-DOSSIER, `python3 scripts/android/check_ui_parity.py` (unchanged code, sanity) |
| T5 | Full gate + fixes | the complete gate (section 8) |

T1 leaves `check_ui_parity.py` green by declaring the two new commands `pending:P5b-settings` in the `ui-parity` block of `docs/android/PARITY.md` (the owner must match the script's owner pattern; non-strict mode accepts pending); T2 removes those lines when both UIs call them.

## 8. Full gate (T5)

`cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo clippy -p pili-pos --no-default-features --features cert --all-targets -- -D warnings`; `cargo test --workspace`; `cargo test -p pili-pos --no-default-features --features cert`; `cargo test -p pos-engine --no-default-features --features cert`; the FFI tests in `android/rust` (G-FFI); `python3 scripts/android/check_ipc_wrappers.py`; `python3 scripts/android/check_command_parity.py`; `python3 scripts/android/check_ui_parity.py`; `(cd ui && npm run build && npx playwright test)`; `bash scripts/android/p5_gate.sh`; `bash scripts/android/p6_gate.sh`; `python3 scripts/check_dossier.py`; `PILI_DOSSIER_DIR=docs/android/dossier python3 scripts/check_dossier.py`. Every heavy one through `scripts/android/heavy.sh`. Compare with `moyun-pos-wt/gates-rc2.log` (that log is from `6a7b5fc`, where the FFI error count test failed; `5b3f0b69` fixed it, so the base is expected green): the workspace test count must not go down (1813 at rc2), Kotlin tests removed 0. Write the results into this file under "T5 results" (counts, PASS/FAIL per step) in the final docs commit.

## 9. Risks and open questions

| # | Risk or question | Recommendation |
|---|---|---|
| 1 | Another lane also adds migration 28. | Keep 28; the orchestrator renumbers at the merge (references listed in L1F-A7). |
| 2 | Changing the commit of training S events (L1F-A4) changes behaviour seen by existing tests (M170, presence). | Expected: adjust only assertions on `staff_shifts` after a training WorkIn, never a signed payload; record the change. |
| 3 | `PendingContext` JSON in `pending_signs` written by rc2 databases. | `#[serde(default)]` on `by`; a test parses the old JSON. |
| 4 | Meal-voucher brand spellings are not confirmed by a real terminal trace. | Exact-match list (L1F-B2), negatives for eco/gift; record as an assumption; a field trace may extend the list in a certified hotfix. |
| 5 | Payflip (or another issuer) reported by the terminal as a scheme brand (Mastercard). | Accept for v1 (L1F-B7): the terminal decides the card type. Question for the user: is a manual override wanted later? |
| 6 | Eco-cheques via the terminal are still signed as cards (should be `CHEQUE_OTHER`). | Out of scope; record as a follow-up for the user. |
| 7 | A presence-only employee needs to become a till user. | Deactivate and create again with the same INSZ (existing reactivation), documented in the user manual. |
| 8 | FFI record changes (`User`, `CreateUserInput`, `EftPayment`) break Kotlin call sites and the API dump. | T1 and T3 fix the call sites and regenerate `pos-core.api` in the same commit. |
| 9 | The lane edits `frozen` paths of `LANES.md`. | Authorized by the user for this lane (v1 must-fix, rc3 afterwards); say so in the commit bodies. |
| 10 | `p6_gate.sh` plat-engine-ref still points at `linux-v1.0.0-rc2`. | Leave it; the orchestrator updates it at the rc3 freeze. |
| 11 | Heavy gates on a shared 16 GB machine. | heavy.sh for everything heavy, disk check first. |

## 10. Captures to retake (orchestrator, after the merge)

Linux (`ui/e2e/captures.spec.ts`, `npm run captures`; workers update the spec's scenario and mock data, not the PNGs):
- `19_admin_users.png`: employees with the presence state and "Arrivée"/"Départ", a presence-only employee, the fourth profile choice.
- New `19a_admin_presence.png` (proposed): an employee clocked in by the manager ("En service depuis ...", "Départ"), plus the FDM-down note variant if the runbook wants it.
- New `09l_eft_meal_voucher.png` (proposed): checkout with an approved terminal part "Chèques-repas · Edenred".
- `09_checkout_done.png` / ticket example only if the example sale is changed to include a terminal meal voucher.
- Update `docs/internal/CAPTURES_ECRAN_RUNBOOK_FR.md` entries for the new and changed captures (T2 and T3).

Android (`docs/android/dossier/screenshots`, emulator):
- `A46_employes.png` (presence state and buttons, presence-only row, fourth profile).
- New: manager clock-in confirmation / employee on shift in Settings (proposed `A124_pointage_par_le_gerant.png`).
- New: approved terminal meal-voucher part at checkout (proposed `A125_cheques_repas_terminal.png`).
- `A01_login_tuiles.png` only if the capture data includes a presence-only user (it should not appear).
- Any capture listed by `CAPTURES_ECRAN_FR.pdf` that shows the employees panel or an approved terminal part.

## Worker notes

- T1: see DECISIONS, section `Lane 1-v1-fixes`, L1F-A8 (create_user_flagged, uniffi defaults, i18n error texts and ipc.ts wrappers added in T1, Gradle PATH).
- T2: see DECISIONS, L1F-A9 (UI choices, ZH wording, `SettingsFeatureEntryPoint.fdm()`, goldens re-recorded, dossier files touched). Additional captures to retake (besides section 10): `19a_admin_presence.png` (Linux scenario added to `captures.spec.ts`, runbook entry written) and `A124_pointage_par_le_gerant.png` (Android, runbook row added); `A46_employes.png` and `19_admin_users.png` as listed above.

- T3 (meal-voucher cards): see DECISIONS, section `Lane 1-v1-fixes`, `### EFT`. Captures to retake besides section 10: `09l_eft_meal_voucher.png` (Linux scenario added to `captures.spec.ts`, runbook entry written; the Linux capture PDF must include it) and `A125_cheques_repas_terminal.png` (Android, runbook row added). `TICKETS_EXEMPLES_FR.md` (both dossiers) show no terminal meal-voucher line, so they are unchanged. The scripted terminal of the engine tests bypasses the drivers, so the engine now normalizes the brand once more when it stores an approval (`resolution` in `commands/eft.rs`); CCV's own circuit codes `EDRD`, `SODX`, `PAYF` (found in the CCV request fixture `cardCircuitCollection`) are accepted as exact spellings too.
- T4 (dossier consistency, CLAUDE.md tree): see DECISIONS, `### Dossier consistency (T4)`. PDFs rebuilt for the six Linux annexes and five Android annexes changed; no new capture needed. Docs only.

## Results

(Filled by T5.)
