# Lane 4-v12: owner features for the v1.2 certified update

Planner: Opus 5.5, 2026-10-17. Branch `android/lane-v12`, from `android/int` at `b3248d2d`.
Worktree `/Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/lane-v12`.

This branch is NOT part of the v1.0.0 certified binary (lanes 1 and 2, rc3 freeze) nor of v1.1
(lane 3, `android/lane-v11`). It merges after both. Every change it makes to the till is part of a
later SPF update (one month notice, at most 12 updates a year, the certificate covers only documented
functions), so each item says below whether it touches the certified binary.

## 1. Goal

Five owner features, in this order of work (independent, low risk first; fiscal risk last, planned
most carefully):

- **A. Accountant export** (outside the certified binary): a monthly or date-range export for the
  accountant, from signed archived data only, as detail CSV files plus one balanced journal file
  per day that Belgian accounting packages import.
- **B. Owner alerts** (in the binary, non-fiscal): a Telegram message to the owner's phone when the
  FDM is unreachable or blocked, a printer is out of paper or offline, the linked card terminal is
  offline, the day has no Z by a set hour, or an online order waits unaccepted. Off by default.
- **C. The 14 EU allergens** (in the binary, non-fiscal): data per product and per option, HubRise
  import, numbered badges on the order screen, a product allergen sheet, a printable allergen sheet.
- **D. Courses and "Envoyer la suite"** (in the binary, kitchen printing only, never in the signed
  payload): each line has a course, the kitchen slip prints the first course at once and holds the
  next ones until staff release them. Off by default.
- **E. Gift vouchers, multi-purpose (MPV)** (in the binary, FISCAL): sale as a VAT X line, redemption
  as a `VOUCHER_STORE` payment, balance and expiry, printed voucher, lookup, partial redemption.
  Replays UC260, UC261, UC262. Single-purpose vouchers (SPV) and MPV-to-cash are deferred.

| Item | Touches the certified binary | Fiscal data | Default state |
| --- | --- | --- | --- |
| A | No: separate CLI crate, never a dependency of `pos-engine`, `src-tauri` or `pos-ffi` | reads signed data only, writes nothing | n/a |
| B | Yes: engine worker, settings, both UIs | none | off |
| C | Yes: schema, catalog import, display, print | none | shows data when present |
| D | Yes: schema, bar slip, both UIs | none (kitchen print only) | off (today's slips unchanged) |
| E | Yes: payment method, sale and refund paths, both UIs | yes: N events with VAT X lines and `VOUCHER_STORE` payments | off |

## 2. Scope and non-goals

**In scope:** engine commands (`crates/pos-engine`), `pos-fiscal` (E only), schema (`crates/pos-db`,
migrations 60 to 63), printing (`crates/pos-print`), catalog mapping (`crates/pos-catalog`,
`crates/pos-hubrise`), a new tool crate `crates/pos-accounting`, Tauri wrappers, UniFFI exports, the
`:pos-core` API dump, Vue screens and stores, Compose screens and view models, strings (fr, zh, en;
nl for printed allergen names), Playwright specs, Kotlin unit and Roborazzi tests, engine docs
(`docs/android/ENGINE.md`, `FFI.md`, `PARITY.md`, `docs/android/parity/*.md`), `CLAUDE.md` (counts,
migrations, short feature notes), `scripts/uc_scope.toml` (UC260 to UC262 become covered), the French
update-dossier drafts in `docs/internal/v1.2/`, `docs/android/DECISIONS.md` (append only, own heading).

**Non-goals (explicit):**

- No edit of the v1.0.0 dossiers (`docs/SUBMISSION_ARTIFACTS/**`, `docs/android/dossier/**`). Never
  run `scripts/uc_matrix.py --write` (it writes into `docs/SUBMISSION_ARTIFACTS`); run it in check
  mode only.
- No version bump, no tag, no freeze, no release script, no `check_repro.sh`, no push, no merge.
- SPV (UC270, UC271) and MPV-to-cash (UC4105) are deferred (section 5.E, V12-D24).
- Refunding a sale to a gift voucher (re-crediting the voucher) is deferred: refunds stay cash.
- Voucher expiry breakage accounting is the accountant's decision; A lists signed movements only.
- Email alert channel: deferred (the channel trait allows it later); Telegram only.
- A local allergen editor UI and a local course-per-category editor inside the menu editor: lane 3's
  menu editor is not on this branch. C and D ship the engine commands; C's editor UI is a merge hook
  (section 13). D ships its own small Settings panel for the category mapping because the feature
  is useless without it.
- No KDS, no multi-terminal course sync, no allergen filter on online orders.
- No change to the nightly statistics file schema (`crates/pos-export/STATS_SCHEMA.md` v1): gift
  voucher payments stay `other` there.

## 3. Spec references (read before coding)

| Topic | File, section |
| --- | --- |
| VAT X for an MPV sale | `docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md` §5 "Codes TVA", lines ~486-491: "Utilisation du code TVA X ... la vente d'un bon d'achat Multi-Purpose-Voucher (MPV)" |
| Payment types | same file, `paymentLine` and `tb_paymentTypes` (~line 1740-1775): `VOUCHER_STORE` "Paiement avec bon d'entreprise"; enum `PaymentType` (~line 2960) |
| negQuantityReason VOUCHER | same file, ~line 1515-1530 (only for SPV use, deferred) |
| Prints, kitchen tickets | same file, §8.1 "Impressions" (~line 700-735): every event gives a message; N and I always printed; other prints carry "CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE"; "Les tickets de cuisine et de bar ne mentionnent JAMAIS de montants. S'ils sont imprimés suite à l'établissement d'un event PRO FORMA, ils doivent contenir cette mention." |
| Consolidation of kitchen prints | same file, §8.2 (~line 760-800): "L'imprimé associé (facture provisoire ou ticket cuisine/bar) peut être consolidé et regroupé"; consolidation never applies to FDM messages |
| POS-level kitchen data | `docs/20260717_FAQ_PRODUCTEURS_FR.md`, Q210 b): an option without surcharge is not transmitted to the FDM, "l'option éventuelle est gérée au niveau du POS pour l'affichage du ticket cuisine ou bar" (the course follows the same reading) |
| Every online order signed | FAQ Q203 (B's online-order alert never changes signing) |
| MPV fixtures | `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/UC260_MPV_Sale` (line "Gift Voucher General", dept "Vouchers", VAT X 50, paid `CARD_DEBIT` AUTOMATIC VISA); `UC261_MPV_Use` (P order then N 116 EUR paid `VOUCHER_STORE` "MPV" MANUAL 50 + CASH 66, composite product); `UC262_MPV_Use` (A 12 + B 28 + C 2.50, one `VOUCHER_STORE` 42.50) |
| Deferred fixtures | `UC270_SPV_Sale` (SPV sold at VAT B 75), `UC271_SPV_Use` (negative line `negQuantityReason VOUCHER` with VAT split A -18 / B -40 for a voucher sold at B, see V12-D24), `UC4105_MPVtoCash` (`signMoneyInOut` VOUCHER_STORE +50 / CASH -50) |
| Test setup catalogue | `docs/20260821_UC_Reports_1.3/UC_TestSetup_20240806.md` lines 66-72 and 124-129: MPV 40001-40004 VAT X, SPV 40020 VAT B, department 400 "Vouchers" |
| Report fixtures | `docs/20260821_UC_Reports_1.3/` (A's reconciliation tests replay a report day like `crates/pos-engine/tests/spf_e2e_report_fixtures.rs`) |
| Migration rules | `crates/pos-db/src/migrate.rs` header and the comment after version 11: 1 to 11 certified, 12 to 19 reserved for certified hotfixes, 20 to 27 used; strictly increasing, gaps allowed; `check_applied_names` refuses a version applied under another name. Lanes 1 to 3 use 28, 30, 31 and 40 to 4x. This lane: 60 to 63. |
| Allergens | Regulation (EU) 1169/2011 Annex II (14 substances, numbered 1 to 14 in Annex order); Belgian AFSCA: allergen information for non-prepacked food available on request, written or oral backed by a written record |
| Gift vouchers (consumer law) | Belgian Code de droit économique, book VI, "titres-cadeaux": at least one year of validity and the end date on the voucher. Not checked against the text by the planner: owner to confirm (Q3). |
| MPV VAT treatment | Directive (EU) 2016/1065 (single-purpose / multi-purpose vouchers) as transposed in the Belgian VAT Code; the POS rule that matters is spec §5 (VAT X at sale, VAT on the goods at redemption) |

Code anchors read by the planner (workers re-read them):

- `crates/pos-engine/src/commands/signing.rs`: `PendingContext::Sale` (with `#[serde(default)]
  eft_payment_ids`, the pattern E copies), `commit`, `apply`, `print_slip_of`.
- `crates/pos-engine/src/commands/checkout.rs` (`do_checkout_split`, `sell`), `crates/pos-fiscal/src/
  payment.rs` (`PaymentMethod`, `payment_line_identity`, `PaymentPart`, `settle`).
- `crates/pos-engine/src/commands/eft.rs` (`mark_used` in the sale commit: model for voucher debits).
- `crates/pos-engine/src/commands/bar_slip.rs`, `crates/pos-print/src/bar_ticket.rs`,
  `crates/pos-engine/src/commands/tables.rs` (`do_sign_table_order`), `commands/session.rs`
  (`do_leave_order` signs the P).
- `crates/pos-catalog/src/apply.rs` (catalog sync deactivates every row it does not list; a system
  row must start inactive), `crates/pos-hubrise/src/catalog_map.rs` (the `pos:` tag convention).
- `crates/pos-db/src/queries/orders.rs` `CartLine` (UI view of a line; `OrderLine` feeds the fiscal
  builders and must not gain kitchen-only fields).
- `crates/pos-export/src/stats.rs` (how signed requests and FDM answers are parsed; A reuses the ideas,
  not the crate's file format).
- `crates/pos-engine/src/background.rs` (worker loops), `commands/fdm_monitor.rs` (`FdmHealth`),
  `commands/print.rs` (`do_check_printer`), `commands/eft.rs` (`do_check_payment_terminal`),
  `commands/report.rs` (`do_needs_z_report`), `pos-core/src/config.rs` (`day_end_hour`, `*_file`
  secrets resolved against the config directory).

## 4. Decisions taken by this plan (copy each into DECISIONS.md in the task that implements it)

All decisions go to `docs/android/DECISIONS.md`, at the very end of the file, under one heading
`## Lane 4-v12 (2026-10-17)` (T1 creates it; later tasks append bullets under it).

**Cross-cutting**

- **V12-D1 Migrations 60 to 63.** 60 `owner-settings-alerts` (B), 61 `allergens` (C), 62 `courses`
  (D), 63 `gift-vouchers` (E), in a new `crates/pos-db/src/schema_v12.rs`, registered in a new
  `V12_MIGRATIONS` slice of `migrate.rs`, in every build (no cargo feature), with a test
  `the_v12_migrations_are_60_to_63_and_in_every_build` (each task adds its version to the test).
  Renumbering at merge only touches `schema_v12.rs`, the registry and that test.
- **V12-D2 One owner-settings store.** Migration 60 creates `owner_settings(key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL, changed_at TEXT NOT NULL, changed_by TEXT)` and the append-only
  `owner_settings_log(id, key, value_json, changed_by, changed_at)` (no-update, no-delete triggers).
  Keys: `alerts`, `courses`, `gift_vouchers`. No `CHECK` on the key (adding a key needs no table
  rebuild). Values are validated in Rust (`serde(deny_unknown_fields)`), an absent key means the
  feature's defaults (off). Every set writes the row and a log row in one transaction.
- **V12-D3 Who.** Settings of B, D, E and allergen edits: `Access::Admin`. Sending the next course,
  changing a line's course, selling and redeeming a voucher, voucher lookup, printing allergen sheets:
  any logged-in staff that may already do the underlying action (`Access::Staff` for kitchen and
  allergen actions, `Access::Cashier` for voucher sale and payment, as for any payment). The voucher
  list and duplicates: `Access::Admin`.
- **V12-D4 Off by default, today's behaviour kept.** With B, D and E off (the default), every
  existing test passes unchanged and the till behaves exactly as v1.0.0/v1.1: no alert worker HTTP
  call, slips print as today, no voucher button. C only shows data the catalog or an admin entered.

**A. Accountant export**

- **V12-D5 A separate tool, outside the certified binary.** New crate `crates/pos-accounting`
  (`publish = false`, library + binary `pili-accounting`), a workspace member that no shipped crate
  depends on. A unit test reads `crates/pos-engine/Cargo.toml`, `src-tauri/Cargo.toml` and
  `android/rust/pos-ffi/Cargo.toml` and fails if any of them names `pos-accounting`. Rationale: an
  on-till screen would grow the certified binary and need an SPF update for every format tweak an
  accountant asks for; the export can live off the till because the source data is already the
  signed archive the till keeps and exports.
- **V12-D6 Input = a copy of `pos.db`.** The tool opens a `pos.db` file read-only
  (`SQLITE_OPEN_READ_ONLY`, URI `?mode=ro&immutable=1`), never the live database. Sources: the SPF
  export folder (Android `platform/export` writes a consistent `pos.db` copy; Linux practice is the
  full `pos.db`), or the owner's decrypted nightly backup `pos-<id>.db.age` (X1). `--db` accepts a
  file or an SPF export folder (it then uses the `pos.db` inside and checks `MANIFEST.sha256` when
  present). It refuses a file whose `_migrations_applied` lacks version 4 (append-only archive).
- **V12-D7 Signed data only.** Rows read: `fiscal_events` with an FDM answer (`fdm_response_json`
  not null) and labels N, I, F, R (never T, never P, S, C), and `z_reports` joined to their R event.
  Never `orders`, `order_lines`, `payments` or any other unsigned state. Amounts are the signed
  request's lines and payments and the FDM's `vatCalc`; nothing is recomputed from the menu.
  Dine-in vs takeaway comes from the signed request (`costCenter.type` `TABLE` = dine-in; no cost
  center = takeaway/direct sale; any other cost center type = online), not from `orders.order_type`.
- **V12-D8 Period = closed booking periods.** `--month YYYY-MM` or `--from/--to` (booking dates,
  inclusive). Only booking periods closed by a real Z are exported; an open period in the range is
  reported in `LISEZMOI.txt` and left out. Each day's totals must equal its Z exactly (the tool
  fails with exit code 2 and a `RECONCILIATION_FAILED` line naming the day and the field when they
  do not, and writes nothing).
- **V12-D9 Formats.** Detail CSVs (UTF-8 with BOM, `;` separator, CRLF, amounts with a decimal comma
  by default, `--decimal point` available, a cents column next to every amount): `jours.csv` (per
  booking date: Z number, tickets, refunds, total), `tva.csv` (per date, channel dine-in / takeaway /
  online, VAT label: base, VAT, total), `paiements.csv` (per date and method: cash, card, meal
  voucher, gift voucher, app, online, other; payment, rounding, tip), `remboursements.csv` (each
  refund ticket), `factures.csv` (each I event: number, date, customer VAT and name, base/VAT per
  label, the referenced tickets; informative: already counted in the sales), `bons_cadeaux.csv`
  (vouchers sold = X lines with productId `GV`, redeemed = `VOUCHER_STORE` payments, refunded),
  `mouvements_caisse.csv` (F money in/out), `rapports_z.csv` (signed Z totals). Journal: one
  balanced entry per booking date (document number = Z number) in `journal.csv` with the
  **generic** profile (columns: date `YYYY-MM-DD`, journal, document, account, description, debit,
  credit, VAT code; `;`, decimal point), plus `--profile winbooks` (Winbooks miscellaneous-operation
  CSV: journal type `PRI`, dates `YYYY/MM/DD`, decimal point) and `--profile yuki` (Yuki general
  journal CSV: dates `dd-mm-yyyy`, `;`). Account numbers come from a JSON mapping file
  (`--accounts`), default PCMN values: 700000/700100/700200 sales per VAT label, 451000 VAT payable per
  rate (451021/451012/451006), 570000 cash, 580000 card transit, 580100 meal vouchers, 580200 app,
  580300 online, 489000 gift vouchers outstanding (credit at sale, debit at redemption), 657000/757000
  rounding. Exact Online has no stable public CSV journal layout (it imports through its own XML or a
  mapped CSV): the generic profile is the one to map there. Why generic first: a balanced journal of
  plain columns is accepted by all three packages through their column-mapping import, while the
  native layouts must be confirmed by a test import at the accountant (Q1).
- **V12-D10 Output folder.** `export-comptable-<from>-<to>/` with the files above, `LISEZMOI.txt`
  (French: source file, its SHA-256, periods included and left out, how each file reconciles with the
  Z) and `MANIFEST.sha256`. No INSZ, no operator names, no ticket email; B2B customer name and VAT
  number appear only in `factures.csv` (the accountant needs them).

**B. Owner alerts**

- **V12-D11 In the engine, its own loop, never in a command's path.** Module
  `crates/pos-engine/src/alerts/` (`mod.rs` worker, `telegram.rs` channel, `rules.rs` pure
  dedupe/rate-limit state machine) and `commands/alerts.rs` (commands). No new crate (no lock churn in
  `android/rust`). The loop runs from `start_background` only when the setting is enabled; it never
  takes `state.db` or the signing lock while an HTTP request is in flight (reads under the lock, sends
  after), uses `state.shared_http` with a 10 s timeout, and drops a message after 3 attempts.
- **V12-D12 Conditions (each one a kind).** `fdm` (the monitor's `FdmHealth` is not operational for
  2 minutes; only while a session is open, because the monitor only polls then), `printer` (ticket
  or kitchen printer ping fails, or reports paper out / cover open, for 2 consecutive probes; probes
  every 5 minutes while a session is open, skipped while a print or a sale is running), `terminal`
  (`check_payment_terminal` not connected for 2 probes, only when one is configured; never probes
  while `eft_line` is held), `z_missing` (the booking period that should have closed is still open
  at `z_deadline_hour`, default `day_end_hour + 1`, Europe/Brussels), `online_waiting` (an online
  order waits unaccepted for more than `online_wait_minutes`, default 5, only while the HubRise switch
  is on). Each kind has a resolved message when the condition clears.
- **V12-D13 Dedupe and rate limit.** One open alert per kind: raised once, a reminder every
  `reminder_minutes` (default 60, 0 = never) while open, resolved once. A global cap of 20 messages per
  rolling hour; above it messages are dropped and logged `suppressed`. State survives a restart from
  `owner_alert_log` (no second "raised" after a restart if the last row for the kind is an open
  raise younger than the reminder period).
- **V12-D14 Data minimisation.** Message text: an installation label (setting, default the
  `terminal_id`), the kind, the local time since when, and only: for `online_waiting` the number of
  orders and the channel name (platform or webshop), no customer, no amount, no address; for
  `z_missing` the booking date. Never an INSZ, a user name, an amount, a ticket or an order content.
  Language per setting (fr, zh, en; default fr).
- **V12-D15 Secrets.** The bot token lives in a file named by `pos.json` (`alerts.telegram_bot_token_file`,
  resolved like `fdm_token_file`; Android: the provisioning bundle), never in the database, never in
  a log, never returned to the UI (the UI shows "jeton configuré / absent"). `alerts.telegram_api_base`
  (default `https://api.telegram.org`) exists for tests. Error texts strip the request URL
  (`reqwest::Error::without_url`) so the token cannot leak. `check_config` validates the file like the
  other secrets. The chat id and the switches are owner settings (V12-D2), editable on both UIs.
- **V12-D16 Tables (migration 60).** `owner_settings`, `owner_settings_log` (V12-D2) and
  `owner_alert_log(id, kind, transition CHECK IN ('raised','reminder','resolved','test'), detail,
  outcome CHECK IN ('sent','failed','suppressed'), http_status, created_at)`, append-only, rows older
  than 90 days are not deleted (the table is small; documented).

**C. Allergens**

- **V12-D17 Model.** `pos-core/src/allergen.rs`: `enum Allergen` with the 14 Annex II substances in
  Annex order (`gluten`, `crustaceans`, `eggs`, `fish`, `peanuts`, `soybeans`, `milk`, `nuts`,
  `celery`, `mustard`, `sesame`, `sulphites`, `lupin`, `molluscs`), `number()` 1 to 14, `code()`,
  `name(lang)` for fr, nl, en, zh (and de when the establishment language is de). Two levels:
  `contains` and `may_contain` (traces). A product is `declared` (its list is complete, possibly
  empty) or `unknown` (no information: the UI and the sheet say "information non disponible,
  demandez au responsable"). Migration 61: `menu_item_allergens(menu_item_id, allergen, level,
  PRIMARY KEY(menu_item_id, allergen))`, `modifier_allergens(modifier_id, allergen, level, PK)`,
  `allergen_declarations(entity_type CHECK IN ('menu_item','modifier'), entity_id, source CHECK IN
  ('catalog','local'), declared_at, declared_by, PK(entity_type, entity_id))` and the append-only
  `allergen_change_log(entity_type, entity_id, before_json, after_json, source, changed_by,
  changed_at)` (traceability for AFSCA). A line's allergens are its product's plus its chosen
  options'.
- **V12-D18 HubRise import.** Tags on the product, the SKU or the option, following the `pos:` tag
  convention: `allergen:<code>` (contains), `traces:<code>` (may contain), `allergen:none` (declared,
  none). Any allergen tag on product or SKU makes the item declared. An unknown code is a catalog
  issue `UnknownAllergen` (the catalog is refused whole, as every invalid catalog: a silent typo on
  an allergen is a food-safety risk). `pos-catalog::Item` and `Choice` gain
  `allergens: Option<AllergenSet>` with `#[serde(default)]` (stored `catalog_json` of older versions
  still parses; the catalog hash changes once, so the first sync after the upgrade re-applies).
  If the POS uploads its own catalog to HubRise anywhere (`pos-hubrise/src/catalog_mapper.rs`),
  it writes the same tags.
- **V12-D19 Local edit = engine command now, UI after lane 3.** `set_allergens(input
  {entity_type, entity_id, declared, contains[], may_contain[]})`, Admin, refused with
  `hubrise_menu_managed` while a catalog source manages the menu (same lock as the menu). Its UI is
  the lane 3 menu editor hook (section 13).
- **V12-D20 Display.** Numbered badges (Annex number in a small circle, the international practice on
  Belgian menus; no hand-drawn icons) on the product tile (contains only, at most 6 then "+n") and in
  the option picker; an "Allergènes" mode button on the order screen (like lane 3's "Épuisé" mode):
  in this mode a tap on a product opens the product allergen sheet (names in the UI language and in
  French, contains, traces, per option, or "information non disponible") instead of adding it. A
  legend lists the 14 numbers. No long press (touch guide).
- **V12-D21 Printing.** `pos-print/src/allergen_sheet.rs`: title "ALLERGÈNES", establishment name,
  date, per product its name, "Contient" list, "Traces possibles" list or "Information non
  disponible"; footer "Selon le règlement (UE) n° 1169/2011, annexe II. Informations fournies par
  l'exploitant." and "CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE". No price. One product or the
  whole active menu (hidden and system items excluded). Language: the establishment language or a
  chosen one (fr, nl, en, de).

**D. Courses**

- **V12-D22 The course stays out of the signed payload.** The course is kitchen data: it is never in
  a product name, a department, a P/N/T request or the VAT ticket (FAQ Q210 b reading, spec §8.2:
  kitchen prints may be regrouped, FDM messages never are). A test asserts it for P, N, T, pre-bill
  and (after lane 2's merge) transfer requests. `OrderLine` (fiscal builders) does not change;
  `CartLine` gains `course` and `kitchen_state`.
- **V12-D23 Rules.** Courses: 0 "Direct" (drinks, printed at once), 1 "Entrées", 2 "Plats", 3
  "Desserts" (names fr, zh, en, nl on the slip). Setting `courses {enabled, category_courses:
  {category_id: 0..3}}`; an unmapped category is 0. With the switch off, every line is 0 and the slip
  is exactly today's. A new line takes its category's course; staff can change it while nothing of
  the line was printed (`course_already_sent` otherwise). Each table order has a released course,
  starting at 1. When a P (or the N of a direct sale) is signed, the slip prints the entries just
  signed whose course is 0 or not above the released course, grouped by course with a header; the
  others are held. "Envoyer la suite" (`send_next_course(order_id)`): if the order has unsent entries
  it first signs them through the existing table-order path (FDM down = refused, nothing printed,
  as today: a kitchen slip only follows a signed P); then it releases the lowest held course and
  prints all its signed, unprinted entries, consolidated per line (net quantity; a line corrected to
  0 prints nothing). A correction of an already printed line prints at once (today's rule); a
  correction of a held line stays held and is netted at release. A direct sale (N without a table)
  prints every course at once with headers. A sale that closes a table with held courses does not
  print them: both UIs warn before checkout ("Suite non envoyée : Plats") with a "Envoyer maintenant"
  button. `reprint_course_slip(order_id, course)` reprints what was printed for that course with the
  mention "DUPLICATA". The release slip carries the same title and mentions as today's bar slip
  (no amount; the "pas un ticket de caisse TVA" and PRO FORMA mentions exactly as the current
  `bar_ticket.rs` prints them after a P).
- **Tables (migration 62).** `order_line_courses(line_id PRIMARY KEY REFERENCES order_lines(id),
  order_id, course, set_by, set_at)`, `order_course_releases(order_id, course, released_by,
  released_at, PRIMARY KEY(order_id, course))`, `kitchen_slip_entries(entry_id PRIMARY KEY
  REFERENCES order_entries(id), course, printed_at, slip TEXT CHECK IN ('immediate','release','sale'))`
  (an entry is recorded only when its slip reports `Printed`; recorded whatever the switch, so a
  switch turned on mid-service is consistent). Kitchen-only, never fiscal.

**E. Gift vouchers**

- **V12-D24 MPV only.** Implemented: multi-purpose vouchers (sale VAT X, no VAT at issue; VAT on the
  goods at redemption, the redemption is a payment). Deferred, recorded in DECISIONS and in
  `docs/internal/v1.2/BONS_CADEAUX_FR.md`: (1) SPV: VAT due at sale on the goods, redemption as a
  negative product line `negQuantityReason VOUCHER` whose VAT split must mirror the sale; the UC271
  fixture splits a voucher sold at B 75 EUR into A -18 / B -40 at use (a different total and a
  different VAT mix than the sale), which we cannot reproduce from a clean rule, so the reading is
  ambiguous; (2) MPV to cash (UC4105, `signMoneyInOut` VOUCHER_STORE +x / CASH -x): no business need,
  and Belgian vouchers are not exchangeable for cash; (3) refund to a voucher. `scripts/uc_scope.toml`:
  remove UC260, UC261, UC262 (covered by tests); keep UC270, UC271, UC4105 `hors_perimetre` with the
  reason updated to "différé en v1.2" wording (French).
- **V12-D25 Sale.** A system product (migration 63) `menu_items` row `external_id = 'pili:gift-voucher'`,
  in a system category `external_id = 'pili:system'`, `vat_code = 'X'`, `vat_code_takeaway = 'X'`,
  `vat_category = 'out_of_scope'`, `kitchen_printer = 0`, `open_price = 1`, `active = 0` (never on
  the grid; catalog sync deactivating every row is a no-op on it). Command
  `add_gift_voucher_line(order_id, amount_cents)` adds one line, quantity 1, name "Bon cadeau" in the
  establishment language, `productId` the system item's id, department id `GV` and name "Bons
  cadeaux" (establishment language). Amount 1,00 to `max_amount_cents` (setting, default 500,00 EUR).
  The line's quantity cannot grow (`gift_voucher_line_fixed`), it can be removed (correction) before
  the sale. No discount or price change applies to it (lane 2 merge note). An order with a voucher
  line cannot be paid with meal vouchers or a gift voucher (`gift_voucher_payment_invalid`); cash and
  card (manual or EFT) are fine.
- **V12-D26 Issue at the N commit.** At prepare (under the sign guard) the engine draws one code per
  voucher line and stores `gift_vouchers_issued: Vec<{line_id, code, face_value_cents, expires_on}>`
  in `PendingContext::Sale` (`#[serde(default)]`), so a recovered or late answer issues the same
  codes. The vouchers and their `issue` movements are written in `apply` of the archive transaction.
  Training (T): vouchers are written with `training = 1`, usable only in training. Code: 12 digits
  (10 random digits from the OS RNG + 2 check digits, 97 - (n mod 97), Belgian structured-communication
  style), shown `1234 5678 9012`, so both UIs enter it on the existing numeric keypad. Lookups that
  fail are throttled (5 per minute per terminal, `gift_voucher_lookup_throttled`).
- **V12-D27 Redemption = `VOUCHER_STORE` payment.** `PaymentMethod::GiftVoucher` maps to
  `PaymentType::VoucherStore`, identity ("8", "Gift voucher"), `inputMethod MANUAL`, never rounded
  (like meal vouchers). `PaymentPart` gains `gift_voucher_id: Option<i64>` (`#[serde(default)]`; the
  UI gets it from `lookup_gift_voucher`). Checks at prepare under the sign guard: voucher exists, same
  training mode, not cancelled, not expired at the local date, balance >= the part, the part > 0, at
  most one part per voucher. The debit is a `redeem` movement written in `apply` (unconditionally:
  signing is serialized and nothing is signed while an answer is awaited, so the balance cannot move
  between prepare and commit; no SQL CHECK that could fail a commit after signing). If the voucher is
  worth more than the bill, the part equals the rest of the bill and the balance stays on the voucher
  (no change given). Several vouchers may pay one sale. Card and EFT parts keep their rules.
- **V12-D28 Refunds.** A refund of a sale paid partly by voucher stays cash (unchanged refund flow,
  manager-confirmed); the voucher is not re-credited (deferred, V12-D24). A refund of a voucher SALE
  line cancels that voucher in the refund's archive transaction (`PendingContext::Refund` gains
  `gift_vouchers_cancelled: Vec<i64>`, `#[serde(default)]`), only if the voucher was never redeemed
  (balance = face value) and is not cancelled; otherwise refused `gift_voucher_used`.
- **V12-D29 Expiry and settings.** Setting `gift_vouchers {enabled, validity_months (12 to 36,
  default 12), max_amount_cents}`. `expires_on` = issue local date + validity months; printed on the
  voucher. Expiry is not an event: an expired voucher is refused at payment (`gift_voucher_expired`);
  an admin can still look it up. No VAT effect (MPV).
- **V12-D30 Printed voucher.** After the N is committed, a non-fiscal slip `pos-print/src/
  gift_voucher.rs`: "BON CADEAU", establishment name and address, value, code (large) and a QR of the
  code, issue date, "Valable jusqu'au jj.mm.aaaa", "Utilisable en une ou plusieurs fois, sans rendu de
  monnaie", the ticket reference (fdmRef of the sale), "CECI N'EST PAS UN TICKET DE CAISSE TVA
  VALABLE". A duplicate (`print_gift_voucher(voucher_id)`, Admin) says "DUPLICATA" and is logged in
  `gift_voucher_prints`. When a payment leaves a balance, a small "Solde du bon" slip prints after the
  VAT ticket (code masked to its last 4 digits, remaining balance, expiry). The VAT ticket layout
  itself does not change (the payment line reads "Bon cadeau").
- **Tables (migration 63).** The system category and item (V12-D25); `gift_vouchers(id, code TEXT NOT
  NULL UNIQUE, face_value_cents INTEGER NOT NULL CHECK(> 0), issued_on, expires_on, training INTEGER
  CHECK IN (0,1), issue_event_id REFERENCES fiscal_events(id) NOT NULL, issue_order_id, issue_line_id
  UNIQUE, created_at)` with no-update and no-delete triggers; `gift_voucher_movements(id, voucher_id,
  kind CHECK IN ('issue','redeem','cancel'), amount_cents (issue +face, redeem -x, cancel -balance),
  fiscal_event_id REFERENCES fiscal_events(id) NOT NULL, order_id, created_at)` append-only;
  `gift_voucher_prints(id, voucher_id, kind CHECK IN ('original','duplicate','balance'), printed_by,
  printed_at)` append-only. Balance and status are computed (`SUM(amount_cents)`, expiry, cancel).
- **V12-D31 Reports and echo.** X/Z need no change (the FDM's `vatCalc` carries X; payments are
  grouped by id, name and type); the Z printout gets a French/zh/en label for `VOUCHER_STORE`
  ("Bon cadeau"). The HubRise echo of a till sale leaves voucher lines out and maps a voucher payment
  to HubRise's generic "other" payment (worker checks `hubrise_push.rs`; if the echo cannot be kept
  consistent, a sale with a voucher line is not echoed, recorded).

## 5. Design per item

### A. Accountant export (`crates/pos-accounting`)

- `src/lib.rs` (`export(ExportRequest) -> Result<ExportSummary, AccountingError>`), `src/source.rs`
  (open read-only, check migrations, list periods from `booking_periods` + `z_reports` + R events),
  `src/parse.rs` (signed request and FDM answer of N, I, F, R events: lines with VAT label, payments
  with type and amountType, `vatCalc`, `costCenter`, refund detection as in `pos-export/src/stats.rs`:
  references an earlier ticket or a negative total), `src/aggregate.rs` (per day, channel, label,
  method), `src/reconcile.rs` (each day vs its Z: ticket count, turnover, VAT per label, payments per
  type; any gap fails), `src/csv.rs` (tiny writer, no new dependency unless `csv` is already in
  `Cargo.lock`), `src/journal.rs` (balanced entry per day, profiles, account mapping JSON with
  defaults), `src/bin/pili-accounting.rs` (args: `--db`, `--month` or `--from/--to`, `--out`,
  `--accounts`, `--profile`, `--decimal`, `--lang fr`), `README.md` (English).
- Gift voucher identification is fixed now so A can be built before E: a sold voucher is a signed
  line whose `departmentId` is `GV`; a redemption is a payment line of type `VOUCHER_STORE`. Invoices
  (I events) come from lane 3 later; A parses I events generically (the M150 shape: `invoiceNo` when
  present, `customerVatNo`, `costCenter` CUSTOMER, `fdmRefs`) and lists them; tests use the M150
  fixture.
- Errors: `db_unreadable`, `not_a_pili_db`, `period_open`, `reconciliation_failed`, `out_exists`
  (never overwrite an existing folder).

### B. Owner alerts

**Engine.** `alerts/rules.rs`: pure `AlertBook` (per kind: open since, last sent, reminder due; global
hourly window) with a fixed clock in tests. `alerts/mod.rs`: `run_owner_alerts(state)` loop
(`background.rs`, tick `timers.owner_alerts`, default 30 s; probes on their own cadence), reading the
setting each tick (switching off stops sending at once). `alerts/telegram.rs`: `TelegramChannel {
api_base, token }`, `send(chat_id, text)` = `POST {api_base}/bot{token}/sendMessage` JSON `{chat_id,
text, disable_web_page_preview: true}`, 10 s timeout; outcome from HTTP status and `ok`.

| Command | Access | Does |
| --- | --- | --- |
| `get_owner_alerts()` | Admin | `OwnerAlerts {enabled, chat_id, label, language, kinds {fdm, printer, terminal, z_missing, online_waiting}, z_deadline_hour, online_wait_minutes, reminder_minutes, token_configured}` |
| `set_owner_alerts(input)` | Admin | validate (chat id: optional `-` and 1 to 20 digits; hour 0 to 12; minutes 1 to 120; reminder 0 to 1440), store (V12-D2) |
| `send_test_alert()` | Admin | one test message now, returns `{sent, detail}`; logged `test` |
| `list_owner_alert_log(limit)` | Admin | the log, newest first, max 200 |

Errors: `alerts_invalid|field=<name>`, `alerts_token_missing`, `alerts_send_failed`.
Config: `pos.json` `alerts: {telegram_bot_token_file, telegram_api_base}` (both optional);
`pos-core/src/config.rs` resolves the file and `check_config` reports an unreadable or empty token.

**UIs.** Vue: Settings panel "Alertes sur téléphone" (`ui/src/components/OwnerAlertsSettings.vue`,
store `ui/src/stores/ownerAlerts.ts`): switch, chat id on `NumPad` (with a minus key), label, language,
one switch per kind, hours and minutes on `NumPad`, "jeton configuré / absent", "Envoyer un test",
last 20 log rows. Strings in `ui/src/i18n-settings.ts`. Android: `feature/settings/alerts/`
(`OwnerAlertsSection.kt`, `OwnerAlertsViewModel.kt`, `OwnerAlertsState.kt`), wired in
`SettingsScreen.kt`. Same behaviour, same strings (through `vue_to_android.mjs`).

### C. Allergens

**Engine.** `MenuItem` gains `allergens: Vec<String>` (codes), `traces: Vec<String>`,
`allergens_declared: bool`; `Modifier` (option) gains the same. New `commands/allergens.rs`:

| Command | Access | Does |
| --- | --- | --- |
| `get_allergen_sheet(menu_item_id: Option<i64>, lang: Option<String>)` | Staff | `AllergenSheet {lang, products[{id, name, name_fr, declared, contains[{code, number, name}], traces[..], options[{group, name, contains, traces}]}]}`; `None` = whole active menu |
| `print_allergen_sheet(menu_item_id: Option<i64>, lang: Option<String>)` | Staff | prints V12-D21, returns the slip outcome |
| `set_allergens(input)` | Admin | V12-D19 |
| `list_allergens()` | Staff | the 14 entries `{code, number, names{fr, nl, en, zh}}` for the legend |

Errors: `allergen_unknown`, `allergen_target_not_found`, `hubrise_menu_managed` (existing).
Catalog: `pos-catalog` model + validation issue, `pos-hubrise/src/catalog_map.rs` tags, `apply.rs`
writes `menu_item_allergens`, `modifier_allergens`, `allergen_declarations` (source `catalog`) and a
log row per change, inside the same catalog transaction.

**UIs.** Vue: badges in `MenuItemButton.vue` and `ModifierDialog.vue`; "Allergènes" mode button and
legend in `OrderView.vue`; `ui/src/components/AllergenSheetDialog.vue` (product sheet, "Imprimer cette
fiche", "Imprimer la carte complète"); store `ui/src/stores/allergens.ts`; strings in `ui/src/i18n.ts`
(the 14 names in fr, zh, en). Android: `feature/order/AllergenBadges.kt`,
`feature/order/AllergenSheetDialog.kt`, mode in `MenuViewModel.kt` / `OrderState.kt`, badges in
`MenuGrid.kt` and `ModifierDialog.kt`.

### D. Courses

**Engine.** `commands/courses.rs` and changes in `bar_slip.rs`, `signing.rs::print_slip_of`,
`order.rs` (line creation takes the category's course), `pos_print::bar_ticket` (`BarItem` gains
`course: Option<i32>`; headers per course; title "SUITE : <course>" on a release slip; "DUPLICATA" on
a reprint).

| Command | Access | Does |
| --- | --- | --- |
| `get_course_settings()` | Staff | `{enabled, category_courses[{category_id, course}]}` |
| `set_course_settings(input)` | Admin | V12-D2 store, categories must exist, courses 0 to 3 |
| `set_line_course(line_id, course)` | Staff | V12-D23, `course_already_sent`, `course_invalid`, `courses_disabled` |
| `list_held_courses(order_id)` | Staff | `[{course, lines, quantity}]` held (signed or not) |
| `send_next_course(order_id)` | Staff | V12-D23, returns `{course: Option<i32>, signed: Option<SlipOutcome of the P>, slip: SlipOutcome}`; `no_course_held` |
| `reprint_course_slip(order_id, course)` | Staff | V12-D23, `nothing_printed` |

`CartLine` gains `course: i32` and `kitchen_state` (`unsent`, `held`, `printed`); `TableInfo` gains
`held_course: Option<i32>` (next course waiting) for the table plan.

**UIs.** Vue: Settings panel "Service en plusieurs temps" (`CoursesSettings.vue`: switch, a row per
category with four buttons 0 to 3); cart (`CartSidebar.vue`, `OrderLineItem.vue`) groups lines by
course with headers, a course chip per line (tap opens a 4-button picker; disabled once printed), an
"Envoyer la suite (Plats)" button when a course is held; `TablesView.vue` shows "Suite : Plats" on a
table with a held course; `CheckoutView.vue` warns about held courses with "Envoyer maintenant".
Android: `feature/settings/CoursesSection.kt`, `feature/order/` (cart grouping in
`OrderCartColumn.kt`, `CoursePickerDialog.kt`, button), `feature/tables/TablesScreen.kt` badge,
`feature/checkout/CheckoutPanels.kt` warning. Store work in `ui/src/stores/order.ts` and the Compose
`ui/state/OrderStore.kt` (or `MenuViewModel.kt`, whichever owns cart calls today).

### E. Gift vouchers

**pos-fiscal.** `PaymentMethod::GiftVoucher` (`payment_type` = `VoucherStore`, `is_offered_at_till`
true), `payment_line_identity(VoucherStore)` = ("8", "Gift voucher"), `PaymentPart.gift_voucher_id`,
`settle` treats it like meal vouchers (exact, never rounded, never above what is left). Unit tests for
each rule.

**Engine.** `commands/gift_vouchers.rs`; changes in `checkout.rs` (prepare checks, issued codes),
`signing.rs` (`PendingContext::Sale.gift_vouchers_issued`, `.gift_voucher_redemptions: Vec<{voucher_id,
amount_cents}>`, `PendingContext::Refund.gift_vouchers_cancelled`, all `#[serde(default)]`; `apply`
writes vouchers and movements; follow-up prints the voucher and balance slips), `refund.rs`
(V12-D28), `report.rs` / `pos-print/src/labels.rs` (payment label).

| Command | Access | Does |
| --- | --- | --- |
| `get_gift_voucher_settings()` | Staff | `{enabled, validity_months, max_amount_cents}` |
| `set_gift_voucher_settings(input)` | Admin | V12-D29 |
| `add_gift_voucher_line(order_id, amount_cents)` | Cashier | V12-D25, `gift_vouchers_disabled`, `gift_voucher_amount_invalid` |
| `lookup_gift_voucher(code)` | Cashier | `GiftVoucher {id, code_masked, face_value_cents, balance_cents, issued_on, expires_on, status (active, used, expired, cancelled), training, movements[{kind, amount_cents, date, ticket}]}`; `gift_voucher_not_found`, `gift_voucher_code_invalid` (check digits), `gift_voucher_lookup_throttled` |
| `list_gift_vouchers(status: Option<String>)` | Admin | outstanding first, max 500 |
| `print_gift_voucher(voucher_id)` | Admin | duplicate (V12-D30) |

Checkout errors: `gift_voucher_not_usable` (training mismatch, cancelled), `gift_voucher_expired`,
`gift_voucher_insufficient`, `gift_voucher_payment_invalid`, `gift_voucher_line_fixed`,
`gift_voucher_used` (refund).

**UIs.** Vue: order screen "Bon cadeau" button (only when enabled) opening `PriceDialog.vue` for the
amount; the line shows a voucher badge, no quantity buttons; `CheckoutView.vue` payment button "Bon
cadeau" opening `GiftVoucherDialog.vue` (12-digit `NumPad` entry, lookup result: balance, expiry,
status, the amount to use = min(balance, rest), confirm adds the part); after the sale the outcome
panel shows "Bon imprimé" / "Solde imprimé" or the print failure; `ReportsView.vue` new tab "Bons
cadeaux": lookup by code, admin list (outstanding, used, expired), duplicate; Settings panel "Bons
cadeaux" (switch, validity, max amount). Stores `ui/src/stores/giftVouchers.ts`, `order.ts`
(payment parts). Strings in `i18n.ts`, `i18n-checkout.ts`, `i18n-reports.ts`, `i18n-settings.ts`.
Android: `feature/order/` button and line, `feature/checkout/GiftVoucherDialog.kt` + view model
changes, `feature/reports/ReportsVouchersTab.kt`, `feature/settings/GiftVoucherSection.kt`.

## 6. Test strategy

- **A:** unit tests per module; `crates/pos-accounting/tests/reconcile_report_day.rs` builds a real
  archive with `pos-engine-testkit` (dev-dependency only): the Reports 1.3 day (as in
  `spf_e2e_report_fixtures.rs`) plus a refund, a meal-voucher sale, a takeaway sale, a training sale
  (must be absent), an unsigned open order (must be absent), a P event (absent), money in/out, a Z;
  copies the DB file and runs the export: every day equals its Z to the cent (turnover, VAT per label,
  payments per type, counts); the journal entries balance (debit = credit per day); CSVs golden-tested
  (byte exact, CRLF, BOM); an open period is left out and named; a tampered file (a modified amount
  in a copied DB through a raw `UPDATE` with triggers dropped) fails reconciliation; `--db` on an
  export folder; read-only proven (file mtime and hash unchanged); synthetic GV line and
  `VOUCHER_STORE` payment counted in `bons_cadeaux.csv`; the M150 fixture's I event listed in
  `factures.csv`; the dependency-isolation test (V12-D5).
- **B:** `crates/pos-engine/tests/owner_alerts.rs` against a local mock HTTP endpoint (wiremock or an
  axum server, already dev-dependencies): no request with the switch off or no token; raise after the
  debounce, reminder, resolved, dedupe across a restart, global cap and `suppressed`, test message,
  token absent from every log line, error and log row (assert on captured tracing output), message
  texts contain no INSZ, name or amount (fixtures with recognizable INSZ and amounts), a stalled
  endpoint (sleep 30 s) never delays a sale or a Z (time a checkout while the send hangs), each kind's
  probe with the FDM mock, `pos-printer-mock` paper-out state, scripted EFT terminal, a fixed clock for
  `z_missing`, the HubRise mock for `online_waiting` (and nothing when the switch is off). Never the
  real Telegram API. `rules.rs` unit tests with a fixed clock. `access_control.rs`, `error_codes.rs`,
  `config_check.rs` rows.
- **C:** `pos-core` allergen table (14 entries, numbers, names in 4 languages); `pos-hubrise`
  catalog_map tests (tags on product, SKU, option; `allergen:none`; unknown code rejects the catalog);
  `crates/pos-engine/tests/allergens.rs` (catalog import writes rows and log, a second identical sync
  writes no log row, a changed tag logs before/after, local edit refused while managed, allowed
  otherwise, sheet content, whole-menu sheet excludes hidden and system items, print to
  `pos-printer-mock`, no allergen data in any archived request); `pos-print` golden test of the sheet.
- **D:** `crates/pos-engine/tests/courses.rs`: switch off = slips byte-identical to today's (compare
  with `spf_e2e_bar_slip.rs` expectations); course 0 and 1 print at the P, 2 and 3 held; release
  prints course 2 then 3; release with unsent entries signs a P first; FDM down = refused and nothing
  printed; release of already-signed course with FDM down still prints; correction rules (printed
  line prints at once, held line nets, net 0 prints nothing); course change refused once printed;
  direct sale prints all with headers; printer failure leaves entries unprinted and a retry prints
  them; reprint says DUPLICATA; training slips keep TRAINING; the course never appears in P, N, T,
  pre-bill requests (search the archived JSON and the VAT ticket text). `pos-print` golden tests.
- **E (fiscal):** `crates/pos-fiscal` unit tests (identity, settle rules). New
  `crates/pos-engine/tests/spf_e2e_gift_vouchers.rs` with `uc260_mpv_sale`, `uc261_mpv_use`,
  `uc262_mpv_use` replayed field by field with `assert_matches_fixture` (as `spf_e2e_fixtures.rs`;
  listed `Deviation`s only: product and payment ids and names, `inputMethod` of UC260 if the card is
  typed, UC261's composite product built as plain lines with the same 116 EUR total, UC262's
  takeaway line labelled from the order type); the payment lines `VOUCHER_STORE` / `MANUAL` and
  amounts must match exactly. New `crates/pos-engine/tests/gift_vouchers.rs`: issue at commit only
  (FDM failure issues nothing; a lost answer recovered at start issues the same code once,
  `spf_e2e_crash_safety.rs` patterns); balance after partial use; two vouchers on one sale; voucher
  larger than the bill; expired refused; training voucher refused in normal mode and vice versa;
  meal voucher or gift voucher payment of a voucher purchase refused; quantity increase refused;
  refund of an unused voucher sale cancels it, of a used one refused; refund of a sale paid by voucher
  is cash and leaves the balance; X/Z totals (VAT X turnover, `VOUCHER_STORE` payment row) and the Z
  printout label; check digits; lookup throttle; printed voucher, balance slip and duplicate on
  `pos-printer-mock`; the code never appears in any archived request. `access_control.rs`,
  `error_codes.rs`, migration tests. `python3 scripts/uc_matrix.py` (check mode) passes with UC260 to
  UC262 covered.
- **FFI:** `android/rust/pos-ffi/tests/ffi/` one file per item (round trip of the new records,
  `deny_unknown_fields`, the new `PaymentPart` field), the error-code count in `pos-ffi/src/error.rs`.
- **Vue:** Playwright `ui/e2e/owner-alerts.spec.ts`, `allergens.spec.ts`, `courses.spec.ts`,
  `gift-vouchers.spec.ts`, handlers in `ui/e2e/fixtures/mock-ipc.ts`; no existing test removed.
- **Android:** unit tests of each new view model and state, Roborazzi goldens only for affected
  screens (record filtered to the affected classes, open every new or changed PNG), Kover thresholds
  kept, `kotlin_test_inventory.py --base android/int` removes 0.

## 7. Gates

Every Bash call starts with:

```
cd /Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/lane-v12 && export ANDROID_HOME=$HOME/Library/Android/sdk CARGO_TARGET_DIR=/Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/shared-target CARGO_INCREMENTAL=0 PILI_HEAVY_SLOTS=2 &&
```

Check `df -g /` before heavy builds (under 10 GB: `cargo clean --target-dir target` and
`(cd android && ./gradlew clean)` in this worktree, retry; under 6 GB after that: stop, report
`DISK_LOW`). Every heavy command through `scripts/android/heavy.sh`. Long runs in the background
(tool calls time out at 10 minutes). Never delete outside the worktree, never touch Docker,
`~/.gradle`, `~/.cache` or the shared target with `rm`.

- **R (Rust):** `cargo fmt --all -- --check`;
  `scripts/android/heavy.sh cargo clippy --workspace --all-targets -- -D warnings`;
  `scripts/android/heavy.sh cargo clippy -p pili-pos --no-default-features --features cert --all-targets -- -D warnings`;
  `scripts/android/heavy.sh cargo test --workspace`;
  `scripts/android/heavy.sh cargo test -p pili-pos --no-default-features --features cert`;
  `scripts/android/heavy.sh cargo test -p pos-engine --no-default-features --features cert`.
- **F (FFI):** in `android/rust`: `cargo fmt --all --check`,
  `../../scripts/android/heavy.sh cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `../../scripts/android/heavy.sh cargo test --workspace --locked`. This lane adds no dependency to
  the engine graph; if a task must, refresh `android/rust/Cargo.lock` and `android/uccheck/Cargo.lock`
  offline and run `bash scripts/android/check_lock_parity.sh` and `bash scripts/android/uc_lock_parity.sh`.
- **P (parity):** `python3 scripts/android/check_ipc_wrappers.py`;
  `python3 scripts/android/check_command_parity.py --strict`;
  `python3 scripts/android/check_command_parity.py --hygiene`;
  `python3 scripts/android/check_ui_parity.py --strict`; `node scripts/android/vue_to_android.mjs --check`;
  `python3 scripts/android/lane_paths.py --check`.
- **D (docs):** `python3 scripts/check_dossier.py`;
  `PILI_DOSSIER_DIR=docs/android/dossier python3 scripts/check_dossier.py`;
  `PILI_DOSSIER_DIR=docs/internal/v1.2 python3 scripts/check_dossier.py`; `python3 scripts/uc_matrix.py`
  (check mode, never `--write`); no em-dash in changed files:
  `git diff --name-only --diff-filter=d android/int | xargs python3 -c 'import sys; bad=[f for f in sys.argv[1:] if chr(0x2014) in open(f, errors="ignore").read()]; print(bad); sys.exit(bool(bad))'`.
- **U (Vue):** `scripts/android/heavy.sh bash -c 'cd ui && npm run build && npx playwright test'`
  (`(cd ui && npm ci)` once if `ui/node_modules` is missing).
- **A (Android):** from `android/`, with `$HOME/.cargo/bin` first on PATH:
  `PATH=$HOME/.cargo/bin:$PATH ../scripts/android/heavy.sh ./gradlew --no-daemon --console=plain --offline -Dorg.gradle.jvmargs='-Xmx2g -XX:MaxMetaspaceSize=1g -Dfile.encoding=UTF-8' lint ktlintCheck detekt :pos-core:apiCheck :app:testDebugUnitTest :app:verifyRoborazziDebug :app:koverVerifyDebug`
  (after an FFI change: `:pos-core:apiDump` first and review the diff; new goldens:
  `:app:recordRoborazziDebug --tests '<pattern>'`, then verify). Never `./gradlew --stop`.
  `python3 scripts/android/kotlin_test_inventory.py --base android/int` removes 0.
- **G (full gate, T6):** R, F, P, D, U, then `bash scripts/android/p5_gate.sh` and
  `bash scripts/android/p6_gate.sh` through `heavy.sh` (background, poll), plus
  `bash scripts/android/test_inventory.sh --diff`.

**Base reference.** `/Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/gates-rc2.log`
was taken at `6a7b5fc8` and its FFI line reads `FFI passed 23 failed 1`
(`error::tests::every_code_of_both_lists_maps_to_coded`) and `ANDROID_GATE_FAIL`, although the brief
calls it all green. T1 runs **F** at the base before any change and records the result in its summary
and in DECISIONS; a failure that exists at the base is not this lane's to hide, and each later task
updates the code count in `pos-ffi/src/error.rs` for the codes it adds.

**Expected-by-design failures of p5/p6 on this lane** (record them in DECISIONS, never weaken a
script): `hygiene-paths` (changes outside `android/|scripts/android/|docs/android/`),
`hygiene-root-lock` (root `Cargo.toml`/`Cargo.lock` change for `pos-accounting`), `linux-frozen` and
`freeze-tag` (the Linux build differs from `linux-v1.0.0-rc2` on purpose). Every other check: PASS or
SKIP.

**Adding an engine command end to end** (template commit `c3488d84`): engine `do_*` with codes in
`commands/errors.rs` (`ALL`); one Tauri wrapper per command in `src-tauri/src/ipc/<module>.rs`, single
engine call, registered in `src-tauri/src/main.rs`; FFI method in `android/rust/pos-ffi/src/api/`
(async, same parameter names and order, `self.spawn` for async, `self.blocking` for sync), records in
`pos-ffi/src/records/` (Serialize + Deserialize, `deny_unknown_fields`, no unsigned ints), FFI tests,
error count, `:pos-core:apiDump`; Vue wrapper in `ui/src/api/ipc.ts`, types in `ui/src/types/index.ts`,
store action, view, strings fr/zh/en and `err_<code>` texts, `mock-ipc.ts` handlers, Playwright spec;
`node scripts/android/vue_to_android.mjs --write` and commit the generated files; Compose call through
`EngineGateway.call { it.<camelName>(...) }` in the owning package, tests, goldens; a `| Vue | Compose |
Commands |` row in `docs/android/parity/<view>.md`; `docs/android/ENGINE.md`, `FFI.md`, `CLAUDE.md`
counts (take them from `python3 scripts/android/check_ipc_wrappers.py --map`). New files stay inside
existing ownership rules (`crates/`, `ui/`, `src-tauri/`, `feature/<view>/`); if a path matches no
rule (`python3 scripts/android/lane_paths.py --owner <path>`), stop and report instead of editing
`LANES.md`.

**Signed operations (E):** follow `commands/signing.rs`: side effects only in `apply` of the archive
transaction (so a late answer has the same effect), new `PendingContext` fields `#[serde(default)]`,
training signs T. Do not change the FDM mock or `response_check` tables except to add tests.

## 8. Tasks

Each task leaves the tree compiling with its gates green, commits in small Conventional Commits
(items never share a commit), body in English, ending with
`Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`, appends its decisions under
`## Lane 4-v12 (2026-10-17)` at the end of `docs/android/DECISIONS.md`, adds its section to
`docs/internal/v1.2/NOTE_MISE_A_JOUR_V1_2_FR.md`, and appends a short "Worker notes" entry at the end
of this file (what was delivered, deviations, gate results).

| Task | Content | Gates |
| --- | --- | --- |
| V12-T1 | A: `crates/pos-accounting`, CLI, reconciliation tests, dossier draft; DECISIONS heading; base F run | F at the base first; then R, P, D |
| V12-T2 | B: migration 60, alerts engine and worker, config, commands to both UIs | R, F, P, U, A, D |
| V12-T3 | C: migration 61, allergen model, HubRise import, display, sheets, print, both UIs | R, F, P, U, A, D |
| V12-T4 | D: migration 62, courses, bar slip holds, release, both UIs | R, F, P, U, A, D |
| V12-T5 | E: migration 63, MPV sale, redemption, refunds, prints, UC260-262, both UIs | R, F, P, U, A, D |
| V12-T6 | Full gate + fixes | G |

## 9. Dossier changes (v1.2 drafts, French, brand-neutral, `docs/internal/v1.2/`)

- `NOTE_MISE_A_JOUR_V1_2_FR.md` (created by T1; one section per item): for the update declaration
  (manual intro, post-certification procedure): what changes, whether it is in the certified binary,
  fiscal impact, tests, UC, screens.
- `EXPORT_COMPTABLE_FR.md` (T1): tool outside the certified binary, input, signed data only,
  reconciliation with the Z, file formats, account mapping.
- `ALERTES_PROPRIETAIRE_FR.md` (T2): non-fiscal, never blocks the till, data sent (exhaustive list),
  secret handling, off by default.
- `ALLERGENES_FR.md` (T3): non-fiscal, model, HubRise tags, sheet, traceability log.
- `SERVICES_CUISINE_FR.md` (T4): course rules, why nothing reaches the FDM (spec §8.1, §8.2, FAQ
  Q210 b), slips and mentions, behaviour with the switch off.
- `BONS_CADEAUX_FR.md` (T5): MPV rules (spec §5 VAT X, `VOUCHER_STORE`), UC260 to UC262 results with
  deviations, balance and expiry, refunds, prints, what is deferred (SPV, UC270, UC271, UC4105,
  refund to voucher) and why; database tables (for the "description de la base de données" annex).
- `SCHEMA_BD_V1_2_FR.md` (T5 completes it; each task adds its migration): tables of migrations 60 to 63.
- Brand-neutral: never the restaurant's or a client's name. No em-dash.

## 10. Risks and open questions (with the recommended decision)

- **Q1 Accounting import layouts not verified on real packages.** Recommended: ship the generic
  balanced journal (V12-D9) as the reference; the Winbooks and Yuki profiles are best effort from
  their public import descriptions; the owner's accountant does one test import per package before
  use; Exact Online through its column-mapping import of the generic file.
- **Q2 Telegram as a channel.** It sends text to a third party (Telegram servers). Recommended:
  accept with V12-D14 minimisation and off by default; the dossier draft lists the exact fields.
- **Q3 Gift voucher validity.** Recommended: minimum and default 12 months, end date printed; the
  owner confirms the consumer-law minimum with the official text before enabling the feature.
- **Q4 SPV and UC271's VAT split.** Recommended: deferred (V12-D24); revisit if the SPF publishes an
  FAQ on SPV redemption VAT.
- **Q5 Voucher code security.** 10^10 codes, check digits, throttled lookups, code never in signed
  data or logs (only masked). Recommended: accept; no online validation exists anyway (single till).
- **Q6 Refund of a voucher-paid sale in cash** effectively turns voucher value into cash.
  Recommended: accept with the existing manager confirmation; document; re-credit later.
- **Q7 Courses and the P mention.** Release slips print after a P that was signed earlier.
  Recommended: same title and mentions as the bar slip printed right after that P; the slip is a
  regrouped print of that P's entries (§8.2).
- **Q8 Held courses at checkout.** Recommended: warn, never auto-print (V12-D23); the staff decides.
- **Q9 Allergen typo rejects the whole catalog.** Recommended: accept (V12-D18), the rejection screen
  names the product and the tag.
- **Q10 Merge conflicts with lanes 2 and 3** in `bar_slip.rs`, `bar_ticket.rs`, `checkout.rs`,
  `signing.rs`, `migrate.rs`, settings screens, `errors.rs` counts, i18n catalogs, goldens.
  Recommended: keep changes additive, new files where possible, section 13 lists each hook.
- **Q11 Base gate log not green for F/A** (section 7). Recommended: T1 measures; the orchestrator
  decides whether the base failure is fixed upstream.

## 11. Captures to retake (orchestrator, after the merge; both products)

1. Settings: "Alertes sur téléphone" panel (off, on with token configured, token absent), test result.
2. Order screen with allergen badges; "Allergènes" mode; product allergen sheet dialog; option picker
   with badges; legend.
3. Printed allergen sheet (one product, whole menu).
4. Settings: "Service en plusieurs temps" panel.
5. Cart grouped by course with chips and "Envoyer la suite"; course picker dialog; table plan with
   "Suite : Plats"; checkout warning about a held course.
6. Printed kitchen slips: first course at the P, release slip "SUITE : Plats", direct sale with course
   headers (the course reprint is engine-only, no screen: no DUPLICATA capture).
7. Order screen "Bon cadeau" button and amount dialog; cart line with a voucher.
8. Checkout "Bon cadeau" payment dialog (lookup active, expired, insufficient); split voucher + cash.
9. VAT ticket of a voucher sale (VAT X) and of a sale paid by voucher; printed voucher; balance slip;
   duplicate.
10. Reports "Bons cadeaux" tab (lookup, list); Settings "Bons cadeaux" panel, with the refusal of the
    switch-off while a voucher can still be redeemed (`gift_vouchers_outstanding`).
11. Z report printout with a "Bon cadeau" payment row and VAT X.
12. Accountant export: `LISEZMOI.txt` and a sample of `journal.csv` (text, no screen).

## 12. Identifiers that must keep existing

`scripts/check_dossier.py` rule `rust-ident` checks backticked identifiers of the v1.0.0 dossiers.
Do not rename or delete any existing public function, type, table or column; this lane only adds.
Run both `check_dossier.py` runs after any change in `crates/`.

## 13. Merge notes (for the orchestrator)

- **Lane 3 menu editor (V11-C):** add an "Allergènes" section to the product and option edit panels
  calling `set_allergens` (V12-D19), showing "information non disponible" vs "aucun allergène"; add a
  course picker (0 to 3) to the category edit panel writing `set_course_settings`; hide the system
  category and item (`external_id LIKE 'pili:%'`) from the editor lists; `menu_change_log` may also
  log allergen edits or leave them in `allergen_change_log` (recommended: keep the allergen log).
- **Lane 3 sales reports:** voucher sales are product "Bon cadeau" VAT X; keep them in totals (they
  are in the Z), optionally a separate row. **Lane 3 invoices:** an invoice of a voucher sale has an X
  line (UBL category O, already mapped).
- **Lane 2 (v1 features):** till discounts must skip lines whose department is `GV`; payment
  correction (M132) must refuse to change a `VOUCHER_STORE` line (or re-credit; recommended refuse);
  kitchen notes (`BarItem.note`) and course headers both extend `BarItem`: merge both fields; the
  transfer slip is unaffected; the course test asserting nothing in the transfer request is added
  after the merge; PRICE_CHANGE entries are never held or printed.
- **Migration numbers:** this lane 60 to 63; lanes 1 to 3: 28, 30, 31, 40 to 4x. The registry tests
  of each lane must be combined.
- **Counts:** Tauri wrapper totals in `CLAUDE.md`, FFI error-code count, Vue key counts in the gate.
- **Goldens:** order, settings, checkout, reports, tables screenshots of all lanes must be re-recorded
  once after the merge.
- **Nightly stats file:** unchanged (gift voucher payments are `other` in schema v1); a schema v2 is a
  separate decision.

## 14. Worker notes

(Each task appends its entry here.)

### V12-T1 (Sonnet 5.5, 2026-10-17): A, accountant export

Delivered: `crates/pos-accounting` (workspace member, `publish = false`, library `pos_accounting` plus
binary `pili-accounting`; modules `source`, `parse`, `aggregate`, `reconcile`, `csv`, `files`,
`journal`, `readme`, `error`, `bin/pili-accounting`), its tests (`tests/reconcile_report_day.rs`,
`synthetic.rs` with `tests/golden/*.csv`, `cli.rs`, `isolation.rs`, 32 unit tests), README (English),
a line in `CLAUDE.md`, the drafts `docs/internal/v1.2/NOTE_MISE_A_JOUR_V1_2_FR.md` (section A) and
`docs/internal/v1.2/EXPORT_COMPTABLE_FR.md`, and the heading `## Lane 4-v12 (2026-10-17)` at the end of
`docs/android/DECISIONS.md` with V12-D1 to V12-D10 and the base F result.

- **Base gate F** (at `c35b7d46`): fmt, clippy `--locked` and `cargo test --workspace --locked` in
  `android/rust` all green. `every_code_of_both_lists_maps_to_coded` passes at this base (it failed at
  `6a7b5fc8` in `gates-rc2.log`, fixed upstream since). Nothing to hide for later tasks.
- **Deviations from the plan (all recorded in DECISIONS):** the tool has no POS crate as a run-time
  dependency (stricter than the plan); a non-empty `pos.db-wal` beside the file is refused (immutable
  mode would ignore it); output rows are per booking period (a Z), not per calendar date; three
  accounts added to the default list (tips 489100, unpaid sales 400000, sales D 700300 and X 700400)
  so that every day balances; Yuki uses a decimal comma; Winbooks and Yuki layouts are best effort (Q1).
- **Engine tests build the archive with `pos-engine-testkit`:** the Reports 1.3 day with the published
  responses (T1 to T8, M184 Z) and a mixed day on the admission mock FDM. The synthetic tests build an
  archive row by row in a migrated database file (for exact goldens, dates, tampering and the M150 and
  gift voucher shapes). The snapshot of an in-memory archive is `VACUUM INTO`.
- **Gates run:** R (fmt, clippy workspace, clippy `cert`, `cargo test --workspace` with 148 passing suites, `cargo test -p pili-pos --no-default-features --features cert`, `cargo test -p pos-engine --no-default-features --features cert`: all green), P (ipc wrappers 96/96,
  command parity strict and hygiene, UI parity strict, Vue to Android strings, `lane_paths --check`
  all PASS), D (`check_dossier.py` for the Linux dossier, the Android dossier and `docs/internal/v1.2`
  all exit 0; no em-dash in changed files; `uc_matrix.py` in check mode). F was run at the base only:
  this task changes nothing under `android/`.
- **Ownership:** `lane_paths.py --owner` says `frozen` for `crates/`, `Cargo.toml`, `Cargo.lock`
  (expected, plan section 7) and gives no owner to `docs/internal/v1.2/`; the plan names these paths,
  `LANES.md` was not edited.
- **Captures to retake (orchestrator):** none on a screen; item 12 of section 11 (text sample): run
  `pili-accounting` on a demo archive and keep `LISEZMOI.txt` and a few lines of `journal.csv`.
- **For T6 / the merge:** root `Cargo.lock` gained the `pos-accounting` package (only workspace
  crates and crates already in the lock); `android/rust/Cargo.lock` and `android/uccheck/Cargo.lock`
  are untouched (the tool is not in their graph; `tests/isolation.rs` checks it). The testkit's
  `Cargo.toml` comment now names the second dev-dependent.
- **Environment finding (shared target):** worktrees that hold the same crate names and versions share
  cargo artifact file names in `shared-target` (path dependencies are hashed relative to the workspace
  root), and cargo's mtime check then treats another lane's fresh artifact as fresh. Here `cargo test
  --workspace` linked `pos-db` against lane 3's `pos-core` (which has `sold_out`) and failed with
  `E0063 missing fields sold_out and sold_out_until in MenuItem` although this tree has no such field.
  Fix used: `touch` the sources of `crates/` and `src-tauri/` of this worktree (mtime only), which makes
  cargo rebuild the workspace crates from this tree. Expect lanes to flip each other's artifacts when
  they build one after the other; a failure that names a field of another lane is this, not a bug.
  `ui/dist` was also missing in the fresh worktree (`pili-pos` needs it: `(cd ui && npm ci && npm run
  build)` once; both untracked).

### V12-T2 (Sonnet 5.5, 2026-10-17): B, owner alerts

Delivered: migration 60 (`crates/pos-db/src/schema_v12.rs`, `V12_MIGRATIONS`, `queries/owner_settings.rs`,
`integrations::waiting_online_orders`); engine module `crates/pos-engine/src/alerts/` (`rules.rs`,
`telegram.rs`, `text.rs`, `mod.rs`) and `commands/alerts.rs`, `commands/owner_settings.rs`; the `alerts`
section of `pos.json` (`pos-core` config, `config_alerts` in `check_config`); four Tauri wrappers
(100 in all, 98 in the cert build) and the UniFFI facade (5 records, 4 methods, error count 117); the Vue
panel (`OwnerAlertsSettings.vue`, `NumberEntryDialog.vue`, `stores/ownerAlerts.ts`, strings, mock handlers,
`ui/e2e/owner-alerts.spec.ts`); the Compose panel (`feature/settings/alerts/`) with tests and three
goldens; the drafts `docs/internal/v1.2/ALERTES_PROPRIETAIRE_FR.md`, `SCHEMA_BD_V1_2_FR.md` and section B of
`NOTE_MISE_A_JOUR_V1_2_FR.md`; parity docs, ENGINE.md, FFI.md, PARITY.md, CLAUDE.md. DECISIONS got the T2
entry under the lane heading (V12-D2 to D4 were already written by T1 and are unchanged; the T2 entry
records the precisions and deviations from D11 to D16).

- **Deviations from the plan text (recorded in DECISIONS):** the loop is spawned in every start and idles
  while the settings are off (so a switch-on needs no restart; `the_loops_tick_and_stop_on_shutdown`
  counts 8 background tasks now); tick and probe interval are module constants, not `EngineTimers` fields;
  the worker builds its own HTTP client with a 10 s timeout instead of `shared_http`; the Vue panel sits in
  the third column (with `panel-grow`, it scrolls inside itself) because the first column already
  overflows with the export panel; the title is "Alertes sur téléphone" (plan section 11 item 1).
- **Tests:** engine 27 scenarios against a mock Bot API (`tests/owner_alerts.rs`) plus 40 unit tests in
  the module; pos-db 80; pos-core 149; FFI 5; Playwright 6 (`owner-alerts.spec.ts`, 360 in all); Kotlin 44
  new tests (number entry rules, view model, section, dialog, goldens). `kotlin_test_inventory.py`: base
  2081, head 2125, removed 0.
- **Gates run:** R (fmt, clippy workspace, clippy `cert`, `cargo test --workspace` including
  `error_codes`, `access_control`, `config_check`, `cargo test -p pili-pos --no-default-features
  --features cert`, `cargo test -p pos-engine --no-default-features --features cert`: all green), F (fmt,
  clippy `--locked`, `cargo test --workspace --locked`: green; `check_lock_parity.sh` identical), P (ipc
  wrappers 100/100, command parity strict 98/98 and hygiene, UI parity strict, Vue to Android strings
  `--check`, `lane_paths --check`: PASS), U (`npm run build` and the whole Playwright suite: 360 passed),
  A (`ktlintCheck detekt :pos-core:apiCheck :app:testDebugUnitTest :app:verifyRoborazziDebug
  :app:koverVerifyDebug` green; `lint` green on its own run, see below), D (`check_dossier.py` three
  dossiers exit 0, `uc_matrix.py` check mode, no em-dash in the changed files).
- **Environment findings:** (1) `./gradlew lint` failed once inside the same invocation with "Unexpected
  failure during lint analysis of ProvisioningE2eTest.kt" (a Kotlin FIR resolution error in lint, androidTest,
  a file this task does not touch); the same task passes when run alone, so it is a lint flake under the
  2 GB heap, not a defect. (2) `bash scripts/android/uc_lock_parity.sh` fails at this lane's base,
  before and after T2: `android/uccheck/Cargo.lock` records the workspace crates at version 1.1.0 while this
  lane's crates are 1.0.0 (the lock came from the v1.1 line); `cargo tree --locked` therefore wants to
  rewrite it. Left untouched; resolved by the merge with the v1.1 version bump. `android/rust/Cargo.lock`
  is unchanged (`check_lock_parity.sh`: 341 packages, identical). (3) Gradle's own cargo build writes
  `target/android-rust` inside the worktree (untracked, ignored).
- **Expected failures of the p5/p6 gates:** `hygiene-paths`, `hygiene-root-lock` (root `Cargo.lock` gained
  the edge `pos-engine` to `tracing-subscriber`, a dev-dependency, no new package), `linux-frozen`,
  `freeze-tag`.
- **Ownership:** `lane_paths.py --owner` says `frozen` for the `crates/`, `src-tauri/`, `ui/` and `android/rust/pos-ffi` paths this
  task edits (the Linux code and the FFI facade) and gives none to `docs/internal/v1.2/`; the plan names them, `LANES.md` was not edited.
- **Captures to retake (orchestrator, plan section 11 item 1; both products):** Settings, panel "Alertes sur
  téléphone": (a) off with the token absent ("Robot Telegram absent", test button off); (b) on with the
  token configured and a chat id (group id with the minus sign), switches, thresholds; (c) the number dialog
  for the chat id; (d) after "Envoyer un message test": the green "Message test envoyé" line and the log
  with a TEST row and an ALERTE row held back ("Retenu (limite horaire)"); (e) a refused test (error line
  with the HTTP code). The Compose goldens `settings/alerts_*.png` cover a, b and c on the 1920 dp window.
  Also keep one real message of each language (fr, zh, en) from the live check below.
- **For the merge / the live check (needs the owner, never done here):** create a bot with BotFather, write
  its token in a file named by `alerts.telegram_bot_token_file`, put the chat id in the panel and press the
  test button once. The tests never reach the real Telegram API.

### V12-T3 (Sonnet 5.5, 2026-10-17): C, the 14 EU allergens

Delivered: migration 61 (`schema_v12.rs`, `V12_MIGRATIONS`, `queries/allergens.rs`); `pos-core/src/allergen.rs`
and the allergen fields of `MenuItem` and `Modifier`; the HubRise tag import (`pos-hubrise/src/catalog_map.rs`,
`pos-catalog` model, validation `UnknownAllergen`, `apply.rs`); the slip `pos-print/src/allergen_sheet.rs`;
the engine commands `commands/allergens.rs` (`list_allergens`, `get_allergen_sheet`, `print_allergen_sheet`,
`set_allergens`) with the errors `allergen_unknown` and `allergen_target_not_found`; four Tauri wrappers (104 in
all, 102 in the cert build) and the UniFFI facade (error count 119); the Vue UI (`AllergenBadges.vue`,
`AllergenSheetDialog.vue`, `stores/allergens.ts`, `utils/allergens.ts`, mode and legend in `OrderView.vue`,
strings, mock handlers, `ui/e2e/allergens.spec.ts`); the Compose UI (`feature/order/AllergenBadges.kt`,
`AllergenBar.kt`, `AllergenSheetDialog.kt`, view model, grid and option picker) with tests; the drafts
`docs/internal/v1.2/ALLERGENES_FR.md`, `SCHEMA_BD_V1_2_FR.md` (migration 61) and section C of
`NOTE_MISE_A_JOUR_V1_2_FR.md`; parity docs, ENGINE.md, FFI.md, PARITY.md, CLAUDE.md. DECISIONS got the T3 entry.

- **Deviations from the plan text (recorded in DECISIONS):** no HubRise upload code (the mapper has none); the
  catalog never clears a local list it says nothing about; zh prints in French; no Vue wrapper and no screen for
  `set_allergens` (lane 3 hook).
- **Tests:** engine 15 scenarios (`tests/allergens.rs`, with a HubRise import submodule) and one access-control
  test; unit tests in pos-core, pos-db, pos-catalog, pos-hubrise and pos-print (exact French golden of the slip);
  FFI suite; Playwright 6 new (366 in all); Kotlin: 3 test classes, `kotlin_test_inventory.py` against the merge
  base: base 2081, head 2152, removed 0 (against the current `android/int` it lists tests that lane 2 of the
  integration added after the merge base, which this branch does not yet have).
- **Gates run:** R, F, P, U, A, D as in the Delivered list of T2; see the final report of the task for the
  results of each run. The four order-screen goldens (`order_menu`, `order_cart`, `order_cart_dialogs`,
  `order_modifiers`) were re-recorded: the only change is the new "Allergènes" button row.
- **Captures to retake (orchestrator, plan section 11 items 2 and 3; both products):** order screen with
  badges on tiles; the "Allergènes" mode with its hint and legend; the product sheet dialog (contains and traces,
  declared empty, information not available, options); the option picker with badges; the printed sheet for one
  product and for the whole menu.
- **Environment findings:** the shared cargo target reuses another lane's artifacts, so `touch` the sources of
  `crates/` and `src-tauri/` before a gate to force a rebuild; `ANDROID_HOME=$HOME/Library/Android/sdk` is needed
  for gradle in this worktree (no `local.properties`).

### V12-T4 (Sonnet 5.5, 2026-10-17): D, courses and "Envoyer la suite" (kitchen only)

Delivered: migration 62 (`schema_v12.rs`: `order_line_courses`, `order_course_releases`, `kitchen_slip_entries`,
backfill; `queries/courses.rs`; `CartLine.course` and `kitchen_state`); the `courses` setting; six engine
commands (`commands/courses.rs`) with the errors `course_already_sent`, `course_invalid`, `courses_disabled`,
`no_course_held`, `nothing_printed`; the V12-D23 correction rules; the release slip "SUITE : <course>" and its
DUPLICATA (`pos-print`); Tauri wrappers and the UniFFI facade; the Vue UI (`CoursePickerDialog.vue`,
`CoursesSettings.vue`, course groups in the cart, held course on the table plan and at checkout, mock handlers,
`ui/e2e/courses.spec.ts`); the Compose UI (`CoursePickerDialog.kt`, `CartLines.kt`, `settings/courses/*`, held tags
on the table card and the checkout) with tests and six new goldens; the drafts `SERVICES_CUISINE_FR.md`, section D
of the NOTE and migration 62 in `SCHEMA_BD_V1_2_FR.md`; parity docs, ENGINE.md, FFI.md, PARITY.md, CLAUDE.md
counts. DECISIONS got V12-D22, V12-D23 and the deviations under "Lane 4-v12 (2026-10-17)".
The course never enters `OrderLine`, product names, departments, P/N/T/pre-bill requests or the VAT ticket.

- **Tests:** engine `tests/courses.rs` (20 scenarios) and one access-control test; unit tests in pos-db and
  pos-print; FFI suite; Playwright `courses.spec.ts` (12); Kotlin: new classes `CoursesCartTest`,
  `CoursesFlowTest`, `CoursesViewModelTest`, `CoursesSectionTest`, `CoursesScreenshotTest`,
  `CheckoutHeldCourseTest`, `TablesHeldCourseTest`, plus 9 cases in `OrderStoreTest`.
- **Fixes made on the way to earlier tasks:** the allergen slip golden test (`\x20` indent) and the
  `cargo fmt` drift in `android/rust/pos-ffi` allergens files. Stray tracked `.kt-E` files from T3 in the order
  feature tests were noticed (to be removed by the orchestrator).
- **Captures to retake (orchestrator; both products):** Settings "Service en plusieurs temps" panel (off and on);
  the cart grouped by course with chips, the picker and "Envoyer la suite"; the table plan "Suite : Plats";
  the checkout warning for a held course; printed slips: first course at P, "SUITE : Plats", direct-sale
  headers, DUPLICATA.
- **Note:** the Tables tag may wrap when two tags share a card (held course plus another tag).

### V12-T5 (Sonnet 5.5, 2026-10-17): E, multi-purpose gift vouchers (UC260, UC261, UC262)

Delivered: migration 63 (system category 990000 and item 990001, `gift_vouchers`, `gift_voucher_movements`,
`gift_voucher_prints`, append-only triggers); `PaymentMethod::GiftVoucher` (VOUCHER_STORE, MANUAL, exact, stored
as `other`); the code (10 digits plus 2 check digits) drawn at prepare and kept in the pending context; issue at the
N commit, redemption and refund cancellation in `signing::apply`; six engine commands (settings get/set, add line,
lookup with a 5 a minute throttle, list, print duplicate) with 12 error codes; voucher, balance and DUPLICATA slips
(`pos-print`); Tauri wrappers and the UniFFI facade (114/114); the Vue UI (order button and amount dialog, voucher
badge on the line, checkout method and code dialog, Reports tab, Settings panel, mock handlers,
`ui/e2e/gift-vouchers.spec.ts`); the Compose UI with the same behaviour; `uc_scope.toml`; drafts
`BONS_CADEAUX_FR.md`, section E of the NOTE, migration 63 in `SCHEMA_BD_V1_2_FR.md`; parity docs, ENGINE.md,
FFI.md, PARITY.md, CLAUDE.md. DECISIONS got V12-D24..D31 and the deviations. SPV, UC4105 and refund-to-voucher are
deferred (V12-D24).

- **Tests:** engine `tests/gift_vouchers.rs` (34) and `spf_e2e_gift_vouchers.rs` (UC260, UC261, UC262); FFI suite;
  Playwright `gift-vouchers.spec.ts` (14, passing); Kotlin: `GiftVoucherViewModelTest`, `GiftVoucherSectionTest`,
  `GiftVoucherScreenshotTest`, `CheckoutGiftVoucherTest`, `OrderGiftVoucherTest`, `ReportsVouchersTest`,
  `ReportsVouchersScreenshotTest`, `GiftVouchersTest`.
- **Captures to retake (orchestrator; both products):** order screen "Bon cadeau" button, the amount dialog and the
  voucher line with its badge; checkout "Bon cadeau" method, the code dialog (lookup, balance, amount) and the done
  screen notes about slips; Reports "Bons cadeaux" tab (list, filters, masked codes, reprint); Settings "Bons cadeaux"
  panel (off and on); printed slips: the voucher (value, code, QR, dates, rules), the balance slip (masked code),
  the DUPLICATA.
- **Environment findings:** shared heavy slots are often busy with other lanes; use a distinct `PW_PORT` for
  Playwright (the default port 1420 is shared); the test seed deletes the whole menu, hence the preserved system rows.

### V12-T6 (Sonnet 5.5, 2026-10-18): full gate

Ran the complete gate G (R, F, P, D, U, p5, p6, test inventory) at the head of the branch; everything is green
except the expected-by-design failures (`hygiene-paths`, `hygiene-root-lock`, `linux-frozen`, `freeze-tag`,
`app-only-scope`) and two failures inherited from the base or from `android/int` having moved (`kotlin-tests`
against `android/int`; `ui-money` on `WebshopOrdersStore.kt`, already fixed in `android/int`). No code needed a
fix, so no fix commit. Details and the GATE list are in DECISIONS under "V12-T6". v1.0.0 dossiers are untouched,
no em-dash was added, `CLAUDE.md` counts (116 wrappers, 114 in the cert build) are current.

The "Captures to retake" list of section 11 matches what T1 to T5 built (items 1 to 12; the Worker notes of T2,
T3, T4, T5 name the same screens). Disk note: under 10 GB free during the run, regenerable worktree build outputs
were removed (see DECISIONS).
