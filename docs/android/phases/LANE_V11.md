# Lane 3-v11: v1.1 features (first certified update after v1.0.0)

Branch `android/lane-v11`, worktree `moyun-pos-wt/lane-v11`, base `android/int` at `b3248d2d`.
Planner: Opus 5.5 (2026-10-17). Workers: Sonnet 5.5. Reviewer: Opus 5.5.

This branch is NOT part of the v1.0.0 certified binary or of its rc3 freeze. It is merged
only after certification, as the content of the first certified update (v1.1). Every
existing feature keeps its certified behaviour; the v1.0.0 dossiers
(`docs/SUBMISSION_ARTIFACTS`, `docs/android/dossier`) are not edited. v1.1 dossier drafts
(French, brand-neutral) go under `docs/internal/v1.1/`; design notes (English) under
`docs/internal/v1.1/design/`.

## 1. Goal

Ship, on both UIs (Vue in `ui/`, Compose in `android/`), with the same behaviour, the same
fr/zh/en strings and green parity checks:

- **A.** B2B invoices (event I, M150) with customer VAT entry, an offline-safe Peppol lookup,
  the printed invoice, and Peppol delivery through a queue with retries against a mock
  access point.
- **B.** A sold-out ("Épuisé") switch on the order screen, refused by the till, pushed to
  HubRise inventory when HubRise is on.
- **C.** An on-screen menu editor (categories, products, option groups, options) for
  installations without HubRise, every change in an append-only log, locked while HubRise
  manages the menu.
- **D.** Non-fiscal sales reports (by product, by hour, day or date range), on screen and
  printed, from signed archived events only, reconciled with the Z reports.
- **E.** Design documents only (no code) for six later features.

Order of work: A, B, C, D, E. If time or disk runs short, A and B are finished fully
(both UIs, tests, docs) before C and D start.

## 2. Scope and non-goals

In scope: section 1, the v1.1 dossier drafts, the decisions in
`docs/android/DECISIONS.md` (section "## Lane 3-v11 (2026-10-17)", appended at the very end
of the file, all lane decisions kept together), Roborazzi goldens and Playwright tests of the
changed screens.

Non-goals (record any that a worker is tempted to cross as a decision, do not build):

- No workspace version bump (`[workspace.package] version` stays `1.0.0`; the v1.1 release
  manager bumps it at the v1.1 freeze). No tag, no push, no merge, no
  `scripts/release/declare*.sh`, no `scripts/android/check_repro.sh`.
- No emulator, adb or device run; no live call to module2, HubRise, Worldline, CCV,
  Storecove, Scaleway or any Peppol service. Live Storecove verification is an owner task
  (section 10, Q2).
- A: no credit notes as a separate document type (a refund N is invoiced like a sale, as in
  UC4122), no deferred payment / customer accounts (`CUSTOMER_CREDIT`, UC4100, UC4121's
  monthly account): v1.1 invoices sales already paid at the till; no invoice for a private
  person without a VAT number; no Peppol delivery to non-Belgian participants (they get the
  printed invoice only); no digital (email/PDF) invoice ticket; no invoice in training mode.
- B: no sold-out for options (HubRise `option_ref`), no import of inventory changes made in
  the HubRise back office, no effect on online orders already received.
- C: no product deletion (deactivate only), no image, no price schedule, no menu import or
  export file, no editing while a catalog source manages the menu.
- D: no export file (CSV/PDF), no per-user report (the fiscal user X/Z exist), no figures
  from mutable tables (`orders`, `order_lines`, `payments`).
- E: documents only.

## 3. Spec references (read before coding)

| Topic | File and place |
| --- | --- |
| Invoices based on prior N events | `docs/20260417_DESCRIPTIONDETAILLEES_POS_FR_2.2.md` §1.7 (around line 327): lines of the N tickets printed, NOT sent in the GraphQL message; payment lines not copied; print says FACTURE and, at the bottom, CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE; Peppol obligation remark (line ~346) |
| Prints | same file §8.1 (line ~708): N and I always printed; copy of an invoice identical to the original plus fdmRef and posDateTime of the copy event ("copie établie le jj.mm.aaaa hh:mm") |
| Fields | same file: `customerVatNo` mandatory for I (line ~1191, Belgian format of `vatNo` or the foreign equivalent), `invoiceNo` (line ~1199), `fdmRefs` (line ~1205), `costCenter` and `tb_costCenterType` incl. `CUSTOMER` (line ~1225), Z `invoices[]` (line ~2219), `input InvoiceInput` (line ~3221), `InvoiceTotalInput` (line ~3483) |
| Law on invoices | `docs/legislation/20240429_AM_aspects_techniques_certification_FR.md` art. 31 (an invoice only after an N; if offered it is part of certification), art. 32 (always printed; FACTURE in capitals; all mentions of AR n°1 art. 5 incl. customer name, address and VAT number; a separate continuous invoice numbering combined with the continuous event number; references to the original VAT tickets; their product lines; FDM control data), art. 33 (all invoice data exportable as an outgoing invoices sub-journal) |
| Forbidden functions | same AM art. 43 (no digital signature on prints other than the VAT ticket), art. 44 (programmed data may not be changed between entry and finalisation so that signed records change), art. 45 |
| Reports | same AM art. 29 (turnover report contents, `w` invoices); certification manual `docs/20250307_POS_CERTIFICATION_MANUEAL_FRV1.1/20250307_FR - Manuel de certification_POS_V11.md` §L "Rapports" (line ~1097: other reports must be described with how they match the fiscal reports, and the queries given); `docs/20260717_FAQ_PRODUCTEURS_FR.md` Q508 (internal reports must match the Z data) |
| Fixtures | `docs/20260821_UC_BasicSet_POS_FDM_1.8/M150_signInvoice/` (scenario, variables with `costCenter {id:"100", type:"CUSTOMER", reference:"FIN"}`, `invoiceNo "20240005"`, two `fdmRefs`); `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/UC4121CustomerInvoice/` (invoice of 3 sales), `UC4122_Creditnote/` (invoice of a refund N and 2 sales) |
| Tips, deposits (E) | Advanced Set 1.6 `UC292_Tip`, `UC296_PaymentForeignCurrencyTip`, `UC4110_advancedPayment`, `UC4111_advancedPayment%of price`, `UC4113_prepaymentReservation`, `UC4114_festivalDeposit` (bottle deposit, refund on card). Note: `UC4112` is the weighted product, not a deposit |
| Peppol dossier (v1.0.0) | `docs/SUBMISSION_ARTIFACTS/DECISION_PEPPOL_AP_FR.md` (Storecove recommended, invoicing announced as the v1.1 update) |
| Programming manual | `docs/SUBMISSION_ARTIFACTS/MANUEL_PROGRAMMATION_FR.md` §3 to §5 (menu by SQL today), §4.4 (deactivate = rupture), §4.5 (menu managed by HubRise) |
| HubRise inventory API | `crates/pos-hubrise-types/src/inventory.rs`, `crates/pos-hubrise-mock/README.md` and the mock routes in `crates/pos-hubrise-mock/src/lib.rs`. Shape (HubRise docs, catalogs §15 Inventories): `PATCH /catalogs/:catalog_id/location/inventory` (location token) with a JSON array of `{"sku_ref": "...", "stock": "0", "expires_at": "<ISO 8601>"}`; `stock` is a decimal string, `null` removes the entry (unlimited); `expires_at` only when `stock` is `"0"`; the response lists the modified entries |
| Migration rules | `crates/pos-db/src/migrate.rs` header and comment at line ~107: 1 to 11 certified, 12 to 19 reserved for certified hotfixes, 20 to 27 used (HubRise 20/22/23/24/25, catalog 21/26, EFT 27); strictly increasing, gaps allowed; `check_applied_names` refuses a version applied under another name |
| UX | `docs/UX-DESIGN-GUIDE.md` (touch sizes, no hover, §2.1 tabs), `docs/BRAND.md` |

## 4. Decisions taken by this plan (copy each into DECISIONS.md in the task that implements it)

- **V11-D1 Invoicing and Peppol join every build.** The `peppol` cargo feature is removed:
  `commands/invoice.rs`, `src-tauri/src/ipc/invoice.rs` and `pos-peppol` are compiled in every
  build (Linux cert, dev, Android FFI). Rationale: both UIs must offer invoices, the Android
  engine is built with `cert`, and HubRise already set the rule "one binary, switched per
  installation". Invoices themselves need no switch (an I event is only made when a cashier
  asks for one); Peppol delivery runs only when the installation has a `peppol` section in
  `pos.json` / the Android bundle. Consequences handled in T1: `src-tauri/src/main.rs` keeps a
  single `generate_handler!` list; `check_command_parity.py` drops hygiene rule H7 and its
  test data; `check_ipc_wrappers.py` keeps working with zero peppol-gated modules;
  `p2_gate.sh` check `ffi-deps` no longer refuses `pos-peppol` (and `test_p2_gate.sh` follows);
  `scripts/build_and_hash.sh` no longer requires 0 `pos_peppol` symbols (it reports the count);
  `scripts/uc_scope.toml` drops `[M150]`; comments in `.github/workflows/test.yml`,
  `android/rust/pos-ffi/Cargo.toml`, `android/uccheck/pos-uc-check/Cargo.toml`,
  `android/rust/pos-ffi/src/records/mod.rs`, `CLAUDE.md` are corrected. The v1.0.0 dossiers are
  NOT edited: they describe v1.0.0.
- **V11-D2 Migrations 40 and up.** The next free number at this base is 28, but the v1.0.x
  lanes (`lane-v1-fixes`, `lane-v1-features`) run in parallel and may claim 28+. The v1.1 lane
  takes its own block: 40 `invoicing`, 41 `sold-out`, 42 `menu-editor` (gaps are allowed by the
  runner). They live in a new `crates/pos-db/src/schema_v11.rs`, registered in a new
  `V11_MIGRATIONS` slice of `migrate.rs`, in every build (no feature gate), with a test
  `the_v11_migrations_are_40_to_42_and_in_every_build`. If the orchestrator wants consecutive
  numbers at merge time, only `schema_v11.rs`, the registry and that test change.
- **V11-D3 Invoice lines come from the archive, never from the UI.** The engine builds the
  invoice (lines, VAT breakdown, totals) from the archived signed requests and FDM responses of
  the referenced N events. The UI only sends sale event ids and a customer id. Supplier data
  comes from the configuration.
- **V11-D4 One invoice per sale.** An N event may be referenced by at most one invoice
  (`invoice_sales.sale_fiscal_event_id UNIQUE`). A refund N may be invoiced (UC4122). Training
  (T) events, copies and anything not labelled N are refused.
- **V11-D5 Invoice numbering.** `invoiceNo = <issue year YYYY><sequence, at least 4 digits>`
  where the sequence is ONE continuous counter over all invoices of the installation, never
  reset (AM art. 32 "numérotation continue séparée"); the year prefix keeps the shape of the
  M150 fixture (`20240005`). Allocated under the sign guard as `MAX(seq)+1` from the `invoices`
  table; the number travels in the pending sign context (`PendingContext::Invoice`) so a
  recovered sign keeps it; it commits with the archive (no gap on FDM failure). The print shows
  the invoice number and the I event counter (`I eventCounter/totalCounter`).
- **V11-D6 Cost center.** The I request carries `costCenter {id: "C<customer id>", type:
  CUSTOMER, reference: <customer name, max 600 chars>}` like M150 and UC4121.
- **V11-D7 No invoice in training mode.** `sign_invoice` refuses with `invoice_training_mode`.
  Conservative: a training invoice would reference training sales and make a T event; nothing in
  the spec requires it.
- **V11-D8 Who.** Invoices: `Access::Cashier` (issued at the counter like the sale). Retrying a
  Peppol delivery and the Peppol panel: `Access::Admin`. Sold-out: `Access::Cashier`. Menu
  editor and sales reports: `Access::Admin`.
- **V11-D9 Peppol identifiers.** A Belgian participant is `0208:<10-digit enterprise number>`
  (the VAT number without `BE`); the current `0208:BE...` value in `invoice.rs` is a bug. Only
  Belgian customers (`BE` VAT) are delivered over Peppol in v1.1; others are printed only and
  their delivery row is not created. The supplier id comes from `peppol.sender_id` in the
  configuration.
- **V11-D10 Offline-safe lookup.** Customer VAT entry is validated offline (Belgian check digits:
  the 10-digit number, last two digits = 97 minus the first eight modulo 97; foreign: country
  prefix plus 2 to 13 alphanumerics). The Peppol lookup is an access point discovery call with a
  3 s timeout whose result is cached on the customer (`registered`, `not_registered`, `unknown`
  plus `checked_at`); a failure or no network gives `unknown` and NEVER blocks the invoice (the I
  event only needs the FDM). The queue worker redoes the lookup before submitting when the
  status is `unknown` or older than 30 days.
- **V11-D11 UBL in cents, multi-VAT, already paid.** `pos-peppol` computes in i64 cents (no f64
  money): line net amount = round(line VAT-inclusive amount / (1 + rate)); per VAT category one
  `TaxSubtotal` whose taxable amount is the sum of its line net amounts and whose tax amount is
  round(taxable x rate); event-level price changes become document `AllowanceCharge`s with their
  tax category; `PrepaidAmount` = VAT-inclusive total of the referenced tickets' product lines and
  price changes (payment rounding excluded, payment lines are not copied); `PayableRoundingAmount`
  absorbs the cent differences so that `PayableAmount` is `0.00`; `BuyerReference` = invoice
  number; no `DueDate`. `vat_label_to_bis` keeps its mapping (A/B/C S, D Z, X O).
- **V11-D12 Peppol queue.** `peppol_deliveries` (migration 1, never shipped with rows) becomes
  the queue and audit row; migration 40 adds `attempts`, `next_attempt_at`, `last_attempt_at`,
  `parked_at`, `lookup_status`; statuses stay `queued` (waiting or retrying), `accepted`,
  `failed` (parked: refused by the AP, recipient not on Peppol, or authentication error). Every
  attempt is appended to `peppol_delivery_log` (append-only, triggers). Backoff 30 s doubling,
  capped at 30 min, retried forever while transient (timeout, 429, 5xx); 4xx refusals and "not
  registered" park the row with a reason; 401 parks and stops the pass. An admin can re-queue a
  parked row (`retry_peppol_delivery`). A background loop (`background.rs`, wake on new invoice,
  `timers.peppol_outbox`, default 60 s) drains the queue, only when `peppol` is configured.
- **V11-D13 Access point implementation.** `AccessPoint` gains `lookup(participant)`; a
  `StorecoveAccessPoint` speaking the Storecove API v2 shape (`POST /api/v2/discovery/receives`,
  `POST /api/v2/document_submissions` with the base64 UBL in `document.rawDocumentData`,
  `Authorization: Bearer <key>`) is tested ONLY against a local mock: routes added to
  `crates/pos-fdm-mock` (`/storecove/api/v2/...`, controllable answers, request log), reused by
  `pos-engine-testkit`. `LoggingAccessPoint` stays (dev default when `peppol.provider` is
  `logging`). Existing identifiers cited by the v1.0.0 dossiers are kept (see section 9).
- **V11-D14 Sold-out model.** `menu_item_sold_out` holds the current state per product
  (`until` = end of the business day, or NULL for "until re-enabled"); expiry is by time, not by
  Z. Every set, clear and expiry is appended to `sold_out_log`. The till refuses
  `add_order_line`, `add_order_line_with_modifiers` and a quantity increase with
  `product_sold_out|product=<id>`; decreases, removals and lines already on an order are not
  affected; online orders are not affected.
- **V11-D15 Sold-out to HubRise.** Only when HubRise is enabled AND the product has an
  `external_id` (SKU ref). A row in `hubrise_inventory_sync` (one per product, upserted, `dirty`)
  is drained by the existing HubRise outbox loop, which sends the CURRENT state at send time:
  sold out today -> `{"sku_ref", "stock": "0", "expires_at": <end of business day, Brussels
  offset>}`; until re-enabled -> `{"sku_ref", "stock": "0"}`; cleared -> `{"sku_ref",
  "stock": null}`, by `PATCH /catalogs/{catalog_id}/location/inventory`. Same error policy as
  the order outbox (transient retry with backoff, 4xx park with reason, 401 stops the pass).
  HubRise off: nothing is queued; switching it on later does not replay old changes.
- **V11-D16 Sold-out UI.** A "Épuisé" mode button on the order screen (cashier and admin only):
  in this mode a tap on a product opens a dialog "Épuisé jusqu'à la fin de la journée / jusqu'à
  réactivation / Disponible"; out of the mode a sold-out product is greyed with an "Épuisé"
  badge and a tap shows the refusal. A "Produits épuisés (n)" list lets the operator re-enable
  any product. No long press (touch guide).
- **V11-D17 Menu editor model.** Edits go to the existing tables (`categories`, `menu_items`,
  `modifier_groups`, `modifiers`, `translations`); nothing is deleted (an `active` flag). Every
  create or update appends `{entity_type, entity_id, action, before_json, after_json,
  changed_by, changed_at}` to `menu_change_log` (append-only triggers, like `table_plan_log`).
  Locked with `hubrise_menu_managed` while `pos_catalog::apply::managed_by` returns a source.
  Order lines keep their snapshot (name, price, VAT, department) so a change never alters an
  open order or a signed event (AM art. 44): a test proves it.
- **V11-D18 Modifiers without HubRise.** The `cfg(feature = "hubrise")` on `commands/modifiers.rs`
  is a schema dependency (migration 21 columns), not a runtime one; every shipped build has the
  feature (`src-tauri/src/main.rs` has a `compile_error!` without it), so the gate stays and the
  menu editor module uses the same gate. The UIs stop keying the option picker on the HubRise
  switch: `MenuItem` gains `has_modifiers` (product has at least one active group with an active
  option) and the picker opens when it is true. With HubRise on, the outcome is the same as
  today (the picker already added plainly when a product had no group).
- **V11-D19 Text entry on the Linux kiosk.** The kiosk has no OS keyboard: a new Vue
  `TextKeyboard.vue` (Belgian AZERTY, upper case, digits, accents é è ê à ç ù, space, `-'.,/&`)
  is used by the invoice customer form and the menu editor. Chinese has no input method on the
  kiosk: on Linux the zh name field shows the current value and is editable only from a
  hardware keyboard paste; on Android it uses the system IME. This is the one documented
  behaviour difference between the UIs (see Q4).
- **V11-D20 Sales reports.** Read-only, never signed, no event, no counter. Computed from the
  archived N events (training excluded, refunds as negative amounts) of the booking dates in
  the range (max 92 days, dates are booking dates). Product rows = signed product lines (incl.
  line price changes); event price changes and payment rounding as separate rows so that the
  grand total equals the sum of `transactionTotal`, i.e. the turnover N of the X/Z. Each closed
  booking date shows its Z number and a "concordance" mark (equal / gap with the amount); the
  open period is marked "période en cours". Hours are Brussels local hours of the sale's
  `posDateTime`, listed from `day_end_hour`. Title "RAPPORT DE VENTES", sub-title "DOCUMENT NON
  FISCAL", footer "CECI N'EST PAS UN RAPPORT FISCAL" and "CECI N'EST PAS UN TICKET DE CAISSE TVA
  VALABLE"; never the words X, Z or CHIFFRE D'AFFAIRES in the title. A fourth Reports tab
  "Ventes".
- **V11-D21 Invoice print.** New `pos-print` layout: FACTURE in capitals, invoice number and I
  counter, seller (name, address, VAT, establishment number), customer (name, address, VAT),
  issue date, references of the original tickets (`fdmId-N eventCounter/totalCounter` and their
  ticket numbers and dates), the product lines of those tickets (consolidation allowed, AM art.
  34), VAT breakdown per rate (base, VAT, total), total, "déjà payé" mention, FDM control data of
  the I event WITHOUT the digital signature (AM art. 43) and without QR, the footer CECI N'EST PAS
  UN TICKET DE CAISSE TVA VALABLE. A copy (event C referencing the I) prints the same layout with
  COPIE and "copie établie le ...". Invoices are always printed on paper in v1.1.
- **V11-D22 Invoice journal (AM art. 33).** Migration 40 creates the SQL view
  `v_invoice_journal` (invoice no, date, customer VAT and name, base and VAT per rate, total, I
  event FDM reference, Peppol status) over append-only tables; the database is the export (as in
  the v1.0.0 answer "SQLite lisible directement"), and the v1.1 dossier draft gives the query.

## 5. Design per item

### A. B2B invoice and Peppol (event I, M150)

**Data (migration 40 `invoicing`, `schema_v11.rs`):**

- `b2b_customers(id, vat_number TEXT NOT NULL UNIQUE, name, street, postal_code, city,
  country_code, peppol_status TEXT CHECK IN ('unknown','registered','not_registered') DEFAULT
  'unknown', peppol_checked_at, created_at, updated_at, updated_by)`. Not fiscal; editable.
- `invoices(id, invoice_seq INTEGER NOT NULL UNIQUE, invoice_no TEXT NOT NULL UNIQUE,
  fiscal_event_id INTEGER NOT NULL UNIQUE REFERENCES fiscal_events(id), customer_id, customer_json
  (snapshot printed), issue_date, total_cents, vat_json (per label base/vat/total cents),
  created_by, created_at)`, append-only triggers (no UPDATE, no DELETE).
- `invoice_sales(invoice_id, sale_fiscal_event_id INTEGER NOT NULL UNIQUE, position)`,
  append-only.
- `peppol_deliveries` new columns (V11-D12) and `peppol_delivery_log(id, delivery_id, attempt,
  outcome, http_status, detail, at)` append-only.
- View `v_invoice_journal` (V11-D22).
- The invoice rows are written in the SAME transaction as the I archive (signing commit hook /
  `PendingContext::Invoice` carries sale ids, customer id and number), so a crash never leaves
  an I without its invoice row or the reverse. Check `crates/pos-engine/src/commands/signing.rs`
  for where kind-specific rows commit (look at how refunds or pre-bills write their own rows) and
  follow that pattern.

**Configuration:** `PosConfig.peppol: Option<PeppolConfig>` (`crates/pos-core/src/config.rs`):
`provider` (`storecove` or `logging`), `base_url`, `api_key_file`, `legal_entity_id`,
`sender_id` (`0208:<number>`), `supplier_street`, `supplier_postal_code`, `supplier_city`,
`supplier_country` (default `BE`). Validated by `install::check_config` (missing file, bad id).
Absent: invoices still work, no delivery rows. HTTP client built like the HubRise one (TLS from
`EngineConfig`, so Android uses its webpki roots).

**Engine commands** (`crates/pos-engine/src/commands/invoice.rs`, keep the file and the cited
names, section 9):

| Command | Access | Does |
| --- | --- | --- |
| `list_invoiceable_sales(limit, query)` | Cashier | recent N events (not training) with ticket no, date, total, fdm ref, `invoiced_by` (invoice no or null) |
| `invoice_preview(sale_event_ids)` | Cashier | lines, VAT breakdown, total, problems (`invoice_sale_already_invoiced`, ...) without signing |
| `lookup_b2b_customer(vat_number)` | Cashier | offline VAT check, cached customer if any, then the Peppol lookup with a 3 s timeout (async; failure = `unknown`) |
| `search_b2b_customers(query)` | Cashier | by VAT or name prefix, max 20 |
| `save_b2b_customer(input)` | Cashier | create or update (validation `customer_incomplete|field=`) |
| `sign_invoice(sale_event_ids, customer_id, employee_insz)` | Cashier | checks (V11-D4, D7), allocates the number, signs I through `do_sign_invoice_fiscal`, writes `invoices`/`invoice_sales`, prints the invoice, queues the Peppol delivery when configured and Belgian; returns `InvoiceResult {fiscal_event_id, invoice_no, printed, delivery_status}` |
| `list_invoices(limit)` | Cashier | recent invoices with customer, total, delivery status |
| `list_peppol_deliveries(limit)` | Admin | queue rows with attempts, next attempt, reason |
| `retry_peppol_delivery(delivery_id)` | Admin | re-queues a parked row (`do_issue_b2b_invoice` becomes the internal "build UBL and queue" step) |

`do_sign_invoice_fiscal` keeps its role (sign the I from sale ids) and gains the customer id
and cost center; the old UI-supplied `B2bLine` / `SupplierInput` path is removed from the IPC
surface. Copy of an invoice: the existing `copy_ticket` on the I event, printed with the
invoice layout (check `crates/pos-engine/src/commands/copy.rs` already maps `"I"`). The Z/X
`invoices[]` already sums the referenced sales (`report.rs::period_events`); add a test with a
two-sale invoice.

**Errors** (`crates/pos-engine/src/commands/errors.rs`, each in `ALL`, each with `err_<code>`
fr/en/zh in `ui/src/i18n.ts`, then `vue_to_android.mjs --write`; update the count in
`android/rust/pos-ffi/src/error.rs` test `every_code_of_both_lists_maps_to_coded`):
`invoice_no_sales`, `invoice_sale_not_found|event=`, `invoice_not_a_sale|event=`,
`invoice_sale_already_invoiced|event=|invoice=`, `invoice_training_mode`,
`customer_vat_invalid`, `customer_incomplete|field=`, `customer_not_found`,
`peppol_not_configured`, `peppol_delivery_not_found`, `peppol_delivery_not_parked`.

**Peppol crate** (`crates/pos-peppol`): `ubl.rs` rewritten per V11-D11 (keep `to_ubl_xml`,
`vat_label_to_bis`, `InvoiceUbl`); `access_point.rs` adds `lookup`, `LookupResult`,
`StorecoveAccessPoint` real HTTP (keep the names `AccessPoint`, `AccessPoint::submit`,
`LoggingAccessPoint`, `StorecoveAccessPoint`, `DeliveryReceipt`, `PeppolError`); new
`participant.rs` (VAT check digits, `0208` id). No f64 for money.

**Tauri:** one wrapper per command in `src-tauri/src/ipc/invoice.rs` (single engine call), the
single handler list in `main.rs`. **FFI:** `android/rust/pos-ffi/src/api/invoice.rs` +
`records/invoice.rs` (new domain file; update the ownership comment of `records/mod.rs`),
contract tests, `tests/ffi/invoice.rs` against the FDM mock; regenerate
`android/pos-core/api/pos-core.api` with `:pos-core:apiDump` and review the diff (additions only).

**Vue (T2):** Reports, tab "Tickets": a "Facture" action with multi-select of sales
(checkboxes), then a dialog: customer step (VAT entry on `NumPad`/`TextKeyboard` with live
check, search of known customers, new-customer form name/street/postal code/city/country on
`TextKeyboard`, Peppol chip "inscrit / non inscrit / inconnu (hors ligne)"), preview step (lines,
VAT, total, the "already paid" mention), confirm (signs, prints), result. A "Factures" list under
it (number, customer, total, Peppol status, "Copie"). Settings (admin): a "Peppol" panel shown
only when configured (provider, sender id, queue counts, parked rows with reason and
"Réessayer"). New store `ui/src/stores/invoice.ts`, wrappers in `ui/src/api/ipc.ts`, strings
in `ui/src/i18n-reports.ts` (invoice) and `ui/src/i18n-settings.ts` (Peppol), new
`ui/src/components/TextKeyboard.vue`.

**Android (T2):** `feature/reports/` (`ReportsInvoiceDialog.kt`, `InvoiceState.kt`, calls in
`ReportsViewModel.kt` or a new `InvoiceViewModel.kt`), `feature/settings/` (`PeppolSection.kt`),
texts through `MessageCatalog` from the generated `strings_vue_*` (no hand-written duplicates),
Roborazzi goldens under `android/app/src/test/screenshots/reports/` and `settings/`. Parity docs
`docs/android/parity/reports.md` and `settings.md` (tables `| Vue | Compose | Commands |`).

### B. Sold-out ("Épuisé")

**Data (migration 41 `sold-out`):** `menu_item_sold_out(menu_item_id PRIMARY KEY REFERENCES
menu_items(id), until TEXT NULL, set_by, set_at)`; `sold_out_log(id, menu_item_id, action CHECK
IN ('set','clear','expired'), until, by, at)` append-only; `hubrise_inventory_sync(menu_item_id
PRIMARY KEY, sku_ref, dirty INTEGER, attempts, next_attempt_at, last_attempt_at, parked_at,
error_msg, synced_at)`.

**Engine** (`crates/pos-engine/src/commands/sold_out.rs`, not gated; the HubRise enqueue part
`#[cfg(feature = "hubrise")]`): `set_sold_out(menu_item_id, until_end_of_day: bool)`,
`clear_sold_out(menu_item_id)`, `list_sold_out()`; refusal in `order.rs`
(`do_add_order_line`, quantity increase) and `modifiers.rs`; expiry computed with
`business_date`/`day_end_hour` (`commands/mod.rs`) in Europe/Brussels, an `expired` log row
written lazily on read. `MenuItem` (`crates/pos-core/src/menu.rs`) gains `sold_out: bool` and
`sold_out_until: Option<String>` (`#[serde(default)]`), filled by
`pos_db::queries::menu::get_menu_items`. The FFI record `MenuItem`
(`android/rust/pos-ffi/src/records/orders.rs`, `deny_unknown_fields`) MUST gain the same fields
in the same commit or the Android grid fails at runtime; the TS type too. HubRise:
`crates/pos-hubrise/src/inventory.rs` (drain pass called from the outbox loop of
`outbox.rs::run_loop`, catalog and location ids from `token_store`), error code
`product_sold_out|product=`.

**UIs:** Vue `OrderView.vue`, `MenuGrid.vue`, `MenuItemButton.vue`, `stores/menu.ts`, a
`SoldOutDialog.vue`, strings in `ui/src/i18n.ts`; Android `feature/order/` (`MenuGrid.kt`,
`MenuViewModel.kt`, `OrderScreen.kt`, a `SoldOutDialog.kt`), goldens under `screenshots/order/`,
parity doc `docs/android/parity/order.md`.

### C. Menu editor (no HubRise)

**Data (migration 42 `menu-editor`):** `menu_change_log` (V11-D17) with triggers
`menu_change_log_no_update` / `_no_delete`.

**Engine** (`crates/pos-engine/src/commands/menu_admin.rs`, `#[cfg(feature = "hubrise")]` per
V11-D18), all `Access::Admin`, all refused with `hubrise_menu_managed` while a source manages the
menu:

| Command | Does |
| --- | --- |
| `menu_admin_get()` | `MenuAdmin {managed_by, languages, categories, products, groups, options}` incl. inactive, with names per language (fr, nl, en, zh, and de when the establishment language is de) |
| `menu_save_category(input)` | create (id null) or update: names, sort_order, color, active |
| `menu_save_product(input)` | names, category, price_cents (0 to 999 999), vat_code, vat_code_takeaway, kitchen_printer, code, open_price, sort_order, active |
| `menu_save_modifier_group(input)` | product, names, min_select, max_select (max >= 1, min <= max), sort_order, active |
| `menu_save_modifier(input)` | group, names, price_cents (>= 0), sort_order, active |
| `list_menu_changes(limit)` | the log, newest first |

Validation: the name in the establishment's fiscal language is required (`config.language`),
others optional; VAT codes A B C D X; errors `menu_invalid|field=<name>`, `menu_not_found`.
Each save writes the row(s), the translations and the log row in one transaction. `MenuItem`
gains `has_modifiers` (V11-D18); both UIs open the option picker on it instead of the HubRise
switch.

**UIs:** Vue: a Settings panel "Carte" (admin; read-only notice when managed) opening a new view
`ui/src/views/MenuEditorView.vue` (route `/menu-editor`, router guard admin): categories list
(left), products of the category (middle), edit panel (right) with `TextKeyboard`, `NumPad` for
prices, VAT pickers, toggles, option groups and options of the product, "Historique" list.
Strings in `ui/src/i18n-settings.ts`. Android: `feature/settings/menu/` (screen, view model,
state; route constant in `SettingsRoutes`, a second `FeatureEntry` bound in `SettingsModule`),
goldens under `screenshots/settings/`, parity doc `docs/android/parity/settings.md`.

### D. Sales reports (non-fiscal)

**No migration.** Engine `crates/pos-engine/src/commands/sales_report.rs`, `Access::Admin`:
`sales_report(from_date, to_date)` -> `SalesReport {from, to, generated_at, days[{booking_date,
z_number, total_cents, z_total_cents, matches_z}], open_period: bool, by_product[{product_id,
name, department, quantity, amount_cents}], by_hour[{hour, tickets, amount_cents}],
price_changes_cents, rounding_cents, refunds_cents, ticket_count, total_cents}`;
`print_sales_report(from_date, to_date)` -> printed or the printer error. Reads only
`fiscal_events` (via `pos_fiscal::archive`), `booking_periods` and `z_reports`; reuse line
parsing from `pos_fiscal::report_totals` (make the needed helpers `pub`), not from
`pos-export/src/stats.rs` (export crate is for the nightly file). Errors
`sales_report_range_invalid`. Print layout `crates/pos-print/src/sales_report.rs` (V11-D20).

**UIs:** Vue `ReportsView.vue` fourth tab "Ventes" (date pickers: today, yesterday, this week,
this month, custom range with `NumPad` dates), two tables (by product sortable by amount or
quantity, by hour as bars drawn with CSS), the concordance line per day, "Imprimer". Strings in
`ui/src/i18n-reports.ts`. Android `feature/reports/ReportsSalesTab.kt` (+ state in
`ReportsState.kt`), goldens `screenshots/reports/reports_sales*.png`.

### E. Design documents (English, no code)

`docs/internal/v1.1/design/` one file each: `TIPS_ON_CARD.md` (UC292, UC296),
`DEPOSITS_AND_ADVANCE_PAYMENTS.md` (UC4110, UC4111, UC4113, UC4114; note UC4112),
`CARD_REFUND_TO_CARD.md` (EFT drivers `crates/pos-eft`, today refunds stay cash, UC4114 refund on
card, reversal rules of `eft_payments`), `KITCHEN_DISPLAY.md` (station routing, `kitchen_printer`
flag, P events, no amounts on kitchen tickets), `QR_TABLE_ORDERING.md` (orders and payments from
a phone, costCenter, P/N signing, HubRise or own channel), `WAITER_HANDHELD_SYNC.md` (multi
terminal: `terminalId`, one FDM, booking periods and Z per POS, the single-terminal assumptions
of today). Each: what the spec requires (file, section, UC), data model, fiscal events and
payloads, UI on both platforms, effort estimate (worker-days per layer), risks, open questions.

## 6. Test strategy

- **Fiscal (A):** `crates/pos-engine/tests/spf_e2e_fixtures.rs` gains `m150_invoice` replayed
  field by field like the other UCs (business fields equal: `costCenter.type`, `fdmRefs` count
  and labels, `customerVatNo`; `invoiceNo` checked for format; listed `Deviation`s only);
  `spf_e2e_basic.rs` M150 tests lose their `cfg` and keep their names; new
  `crates/pos-engine/tests/invoice_advanced.rs` replays UC4121 (three sales) and UC4122 (refund
  N + two sales); new `invoice_rules.rs` (one invoice per sale, training refused, non-N refused,
  numbering continuous across a year change and an FDM failure, crash between sign and commit
  recovers number and rows, Z `invoices[]` amount of a two-sale invoice, copy of an invoice);
  `access_control.rs` and `error_codes.rs` rows for every new command and code.
- **Peppol:** `pos-peppol` unit tests (golden UBL for a two-label invoice, sums per BIS rules
  BR-CO-10/13/15/16 and BR-S-08 asserted in code, allowances, prepaid and rounding, endpoint ids,
  check digits incl. `BE0308357159` valid and a wrong one); `crates/pos-engine/tests/peppol_queue.rs`
  against the mock AP (accepted, 503 then accepted with backoff, 422 parked, 401 parked and pass
  stopped, not registered parked, retry re-queues, offline lookup = unknown and the invoice still
  signed, nothing sent without `peppol` config, foreign customer not queued, log rows appended
  and immutable).
- **Sold-out (B):** `crates/pos-engine/tests/sold_out.rs` (refusals, quantity increase refused,
  decrease allowed, expiry at `day_end_hour` with a fixed clock, log rows, access) and
  `hubrise_inventory.rs` against `pos-hubrise-mock` (PATCH body and path, `expires_at`, `null` on
  clear, coalescing of several changes into the current state, fault then retry, switch off = no
  request, local product without SKU = no request).
- **Menu editor (C):** `crates/pos-engine/tests/menu_editor.rs` (create and update of the four
  entities, validation codes, log rows and triggers, lock while managed, admin only, art. 44: a
  price, name and VAT change while an order is open leaves its lines and the signed P/N
  unchanged), `migrate.rs` tests for 40 to 42.
- **Sales reports (D):** `crates/pos-engine/tests/sales_report.rs` (Reports 1.3 scenario day
  replayed as in `spf_e2e_report_fixtures.rs`: the sales report total equals the Z turnover N of
  that day; hours from `day_end_hour`; refunds negative; training excluded; range > 92 days
  refused; no row added to `fiscal_events`, counters unchanged, nothing sent to the FDM); print
  tests in `pos-print` (title, both disclaimers, no X/Z words).
- **FFI:** `android/rust/pos-ffi/tests/ffi/` one file per new domain, contract tests of the new
  records (round trip, `deny_unknown_fields`).
- **Vue:** Playwright specs `ui/e2e/invoice.spec.ts`, `peppol.spec.ts`, `sold-out.spec.ts`,
  `menu-editor.spec.ts`, `sales-report.spec.ts` with handlers in `ui/e2e/fixtures/`; no existing
  test removed (`test_inventory.sh` and the Playwright inventory must not drop a test).
- **Android:** unit tests of each new view model and state, Roborazzi goldens for each new
  screen or dialog (record only the affected tests, look at every new PNG), Kover thresholds kept
  (`koverVerifyDebug`), no Kotlin test removed (`kotlin_test_inventory.py`).

## 7. Gates

Every Bash call starts with:

```
cd /Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/lane-v11 && export ANDROID_HOME=$HOME/Library/Android/sdk CARGO_TARGET_DIR=/Users/brunomoyse/Development/applications/PERSO/tokyosushi/moyun-pos-wt/shared-target CARGO_INCREMENTAL=0 PILI_HEAVY_SLOTS=2 &&
```

Check `df -g /` before heavy builds (under 10 GB: `cargo clean --target-dir target` and
`(cd android && ./gradlew clean)` in this worktree, retry; under 6 GB after that: stop, report
DISK_LOW). Long runs go in the background (tool calls time out at 10 minutes).

- **R (Rust):** `cargo fmt --all -- --check`;
  `scripts/android/heavy.sh cargo clippy --workspace --all-targets -- -D warnings`;
  `scripts/android/heavy.sh cargo clippy -p pili-pos --no-default-features --features cert --all-targets -- -D warnings`;
  `scripts/android/heavy.sh cargo test --workspace`;
  `scripts/android/heavy.sh cargo test -p pili-pos --no-default-features --features cert`;
  `scripts/android/heavy.sh cargo test -p pos-engine --no-default-features --features cert`.
- **F (FFI):** in `android/rust`: `cargo fmt --all --check`,
  `../../scripts/android/heavy.sh cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `../../scripts/android/heavy.sh cargo test --workspace --locked`. When the engine's dependency
  graph changes (T1), first refresh `android/rust/Cargo.lock` and `android/uccheck/Cargo.lock`
  offline (`cargo update --offline -w` or a non-`--locked` `cargo metadata --offline` in each),
  then `bash scripts/android/check_lock_parity.sh` and `bash scripts/android/uc_lock_parity.sh`.
- **P (parity):** `python3 scripts/android/check_ipc_wrappers.py`;
  `python3 scripts/android/check_command_parity.py --strict`;
  `python3 scripts/android/check_command_parity.py --hygiene`;
  `python3 scripts/android/check_ui_parity.py --strict`; `node scripts/android/vue_to_android.mjs --check`;
  `python3 scripts/android/lane_paths.py --check`.
- **D (docs):** `python3 scripts/check_dossier.py`;
  `PILI_DOSSIER_DIR=docs/android/dossier python3 scripts/check_dossier.py`;
  `PILI_DOSSIER_DIR=docs/internal/v1.1 python3 scripts/check_dossier.py` (once the folder
  exists; brand, em-dash, identifiers); `python3 scripts/uc_matrix.py`;
  `git diff --name-only --diff-filter=d android/int | xargs python3 -c 'import sys; bad=[f for f in sys.argv[1:] if chr(0x2014) in open(f, errors="ignore").read()]; print(bad); sys.exit(bool(bad))'` finds no em-dash.
- **U (Vue):** `scripts/android/heavy.sh bash -c 'cd ui && npm run build && npx playwright test'`
  (run `(cd ui && npm ci)` once if `ui/node_modules` is missing).
- **A (Android):** from `android/`, with `$HOME/.cargo/bin` first on PATH:
  `PATH=$HOME/.cargo/bin:$PATH ../scripts/android/heavy.sh ./gradlew --no-daemon --console=plain --offline -Dorg.gradle.jvmargs='-Xmx2g -XX:MaxMetaspaceSize=1g -Dfile.encoding=UTF-8' lint ktlintCheck detekt :pos-core:apiCheck :app:testDebugUnitTest :app:verifyRoborazziDebug :app:koverVerifyDebug`
  (after an FFI change: `:pos-core:apiDump` first and review the diff; new goldens:
  `:app:recordRoborazziDebug --tests '<pattern>'`, then verify). Never `./gradlew --stop`.
  `python3 scripts/android/kotlin_test_inventory.py --base android/int` removes 0.
- **G (full gate, T6 only):** R, F, P, D, U, then `bash scripts/android/p5_gate.sh` and
  `bash scripts/android/p6_gate.sh` through `heavy.sh` (background, poll), plus
  `bash scripts/android/test_inventory.sh --diff`.

**Expected-by-design failures of p5/p6 on this lane** (record them in DECISIONS, do not "fix"
them by weakening the scripts): `hygiene-paths` (the lane changes `crates/`, `ui/`,
`src-tauri/`, `scripts/`, `docs/internal/`, outside `android/|scripts/android/|docs/android/`),
`hygiene-root-lock` if `Cargo.toml`/`Cargo.lock` changed, `linux-frozen` and `freeze-tag`
(the Linux build differs from `linux-v1.0.0-rc2` on purpose). Every other check must PASS.

## 8. Tasks

Each task leaves the tree compiling with every listed gate green, commits in small Conventional
Commits per feature (A, B, C, D, E never share a commit), body in English, ending with
`Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`, and appends its decisions to
`docs/android/DECISIONS.md` under the single section `## Lane 3-v11 (2026-10-17)` at the end of
the file (T1 creates the heading; later tasks append bullets under it).

| Task | Content | Gates before each commit / at the end |
| --- | --- | --- |
| V11-T1 | A engine: V11-D1 build flip, migration 40, invoice engine, print, Peppol crate, AP mock, queue worker, Tauri, FFI, scripts | R, F, P, D at the end; R subset (`cargo test -p pos-engine`, `-p pos-peppol`, `-p pos-print`, `-p pos-db`) per commit |
| V11-T2 | A UIs (Vue + Android), `TextKeyboard.vue`, Peppol panel, v1.1 dossier draft for invoicing | U, A, P, D; R if Rust touched |
| V11-T3 | B sold-out, engine to both UIs, HubRise inventory push | R, F, P, U, A, D |
| V11-T4 | C menu editor, engine to both UIs, `has_modifiers` | R, F, P, U, A, D |
| V11-T5 | D sales reports, engine to both UIs; E design documents | R, F, P, U, A, D |
| V11-T6 | Full gate + fixes | G |

## 9. Identifiers that must keep existing

`scripts/check_dossier.py` rule `rust-ident` checks that backticked Rust identifiers of the
v1.0.0 dossiers exist in the code. Do not rename or delete: `do_sign_invoice_fiscal`,
`do_issue_b2b_invoice`, `sign_invoice_fiscal` (wrapper), `LoggingAccessPoint`,
`StorecoveAccessPoint`, `AccessPoint` and its method `submit`, `to_ubl_xml`, `PeppolError`,
`FdmClient::sign_invoice`, the test `m150_sign_invoice_archives_i_event_referencing_the_sale`, the
files `crates/pos-engine/src/commands/invoice.rs`, `src-tauri/src/ipc/invoice.rs`,
`crates/pos-peppol/src/ubl.rs`, `crates/pos-peppol/src/access_point.rs`, the table
`peppol_deliveries` and its columns `ubl_xml`, `customer_endpoint_id`, `status`. Run both
`check_dossier.py` runs after any rename anywhere.

## 10. Dossier changes (v1.1 drafts, French, brand-neutral, `docs/internal/v1.1/`)

- `NOTE_MISE_A_JOUR_V1_1_FR.md` (created by T1, one section appended per feature): the list of
  functional changes for the update declaration (manual intro §C, post-certification
  procedure), per change: what, why, fiscal impact, tests, UC.
- `FACTURATION_PEPPOL_FR.md` (T2): event I, numbering, print, invoice journal query
  (`v_invoice_journal`), M150/UC4121/UC4122 results, Peppol (AP, identifiers, queue, offline),
  replaces `DECISION_PEPPOL_AP_FR.md` for v1.1.
- `PRODUIT_EPUISE_FR.md` (T3): rule, log, HubRise push, no fiscal effect.
- `EDITEUR_CARTE_FR.md` (T4): replaces `MANUEL_PROGRAMMATION_FR.md` §3 to §5 for v1.1, log,
  art. 44 argument.
- `RAPPORTS_VENTES_FR.md` (T5): the answer to manual §L "autres rapports": description,
  correspondence with the Z (the concordance line), the SQL/archive queries used, FAQ Q508.

## 11. Risks and open questions (with the recommended decision)

- **Q1 Peppol in the cert graph changes the certified binary's shape.** Recommended: accept
  (V11-D1); it is the v1.1 update content. Risk: reviewers of the update expect the v1.0.0 guard
  scripts; the update note explains the change.
- **Q2 Storecove API not verified live.** Recommended: implement the documented v2 shape against
  the mock only; before the first production use the owner runs a sandbox submission (needs an
  account and a legal entity); keep `provider: logging` as the default in examples.
- **Q3 UBL validity without the official Schematron.** Recommended: assert the main BIS rules in
  unit tests; the Storecove sandbox validates on submission (owner task with Q2).
- **Q4 Chinese names on the Linux kiosk.** Recommended: V11-D19 (no IME on the kiosk; zh names
  from Android or from a hardware keyboard paste; the grid falls back to fr). Ask the owner
  whether a pinyin input is wanted later.
- **Q5 Android IME in lock task mode.** The menu editor and the customer form use the system
  keyboard on Android; device-owner policies may restrict IMEs. Recommended: verify at the
  capture retake on the device; fall back to a Compose port of `TextKeyboard` if blocked.
- **Q6 Category rename mid-period.** Departments are category ids with names; a rename during a
  booking period shows two names for one id in the X/Z departments. Recommended: accept (the Z
  shows what was signed); mention it in `EDITEUR_CARTE_FR.md`.
- **Q7 Migration numbers vs the v1.0.x lanes.** Recommended: V11-D2 (40+), renumber only if the
  orchestrator asks at merge.
- **Q8 B2B customer not on Peppol.** The law makes the e-invoice the invoice for Belgian B2B;
  the till can only print. Recommended: show "non inscrit sur Peppol" before signing and park
  the delivery with that reason; the operator handles it outside the till.
- **Q9 HubRise inventory of shared catalogs.** If several locations share a catalog, the
  location-scoped path is required. Recommended: always `/catalogs/{catalog_id}/location/inventory`
  with the location token (what the POS holds).
- **Q10 Gate scripts owned by other lanes** (`p2_gate.sh`, `check_command_parity.py`,
  `build_and_hash.sh`) change in T1. Recommended: minimal edits with their self-tests updated,
  recorded in DECISIONS.

## 12. Captures to retake (orchestrator, after the merge)

Both UIs (Vue on the Linux kiosk, Compose on the D3 Pro), French, then zh where the runbook
asks for it:

1. Reports, Tickets tab, sales multi-selected with the "Facture" action.
2. Invoice dialog, customer step: VAT entry with the check, known-customer search, new
   customer form with `TextKeyboard` (Vue) / system keyboard (Android), Peppol chip in each of
   its three states.
3. Invoice dialog, preview step (lines, VAT breakdown, "déjà payé").
4. Invoice result and the "Factures" list with Peppol statuses (queued, accepted, failed).
5. Printed invoice (printer mock render) and printed invoice copy.
6. Settings, Peppol panel with a parked delivery and "Réessayer".
7. Order screen in "Épuisé" mode, the sold-out dialog, a greyed product with its badge, the
   refusal message, the "Produits épuisés" list.
8. Settings, "Carte" panel (editable and the locked notice under HubRise).
9. Menu editor: categories / products / edit panel, product form, option group and options,
   "Historique".
10. Reports, "Ventes" tab: by product, by hour, concordance line (equal and gap), open period
    mark; printed sales report.
11. Settings and Order screens whose layout moved because of the new buttons (check the existing
    captures of these screens and retake those that changed).

## 13. Worker notes

### V11-T1 (A engine), 2026-10-17

**Delivered.** V11-D1 flip (no `peppol` feature; one `generate_handler!` list; parity scripts,
`p2_gate.sh`, `build_and_hash.sh`, `uc_scope.toml`, `test.yml`, comments and `CLAUDE.md`
follow); migration 40 (`schema_v11.rs`, `V11_MIGRATIONS`); `PosConfig.peppol` with
`check_config` problems (`config_peppol`); `pos-peppol` rewritten (cents UBL with golden file,
participant ids and offline VAT check, `AccessPoint` with `lookup` and `submit_to`, Storecove
API v2 shape, `LoggingAccessPoint`); Storecove routes in `pos-fdm-mock` and the testkit helper
`use_admission_mock_with_peppol`; `commands/invoice.rs` (9 commands), `commands/peppol_queue.rs`,
the `PendingContext::Invoice` path in `signing.rs`, the invoice copy in `copy.rs`, the queue
loop in `background.rs` (only with a `peppol` section); `pos-print` invoice layout; Tauri
wrappers; FFI facade `api/invoice.rs` and `records/invoice.rs`; tests of section 6 (M150 in
`spf_e2e_fixtures.rs` and `spf_e2e_basic.rs`, `invoice_advanced.rs`, `invoice_rules.rs`,
`peppol_queue.rs`, rows in `access_control.rs`, `error_codes.rs`, `config_check.rs`, FFI
`tests/ffi/invoice.rs`); French note `docs/internal/v1.1/NOTE_MISE_A_JOUR_V1_1_FR.md`;
decisions V11-D1 to D13, D21, D22 and the new D23 to D29 in `DECISIONS.md`.

**Deviations from the plan text.**

- The command is `sign_invoice_fiscal` (not `sign_invoice`), see V11-D23. T2 must call that name.
- X maps to E in the UBL (existing mapping, golden file), not O: V11-D24.
- `do_issue_b2b_invoice` is an engine function only (queues the delivery of an invoice that has
  none); the nine commands are `list_invoiceable_sales`, `invoice_preview`, `lookup_b2b_customer`,
  `search_b2b_customers`, `save_b2b_customer`, `sign_invoice_fiscal`, `list_invoices`,
  `list_peppol_deliveries`, `retry_peppol_delivery`. Total 103 commands.
- Numbering across a year change is not simulated (the clock is real): the sequence is global
  and the year prefix is the year of `posDateTime`; the test asserts continuity across an FDM
  failure and the shape `<year><4 digits>`.

**Hand-off to T2 (UI).** Records (camelCase) are in `android/rust/pos-ffi/src/records/invoice.rs`
and `crates/pos-engine/src/commands/invoice.rs`: `InvoiceableSale`, `InvoicePreview` (with
`problems`, coded errors such as `invoice_sale_already_invoiced|event=..|invoice=..`),
`CustomerLookup` (`peppolStatus`, `peppolChecked`, `deliverable`), `Customer`,
`B2bCustomerInput`, `InvoiceResult` (`success`, `fiscalEventId`, `invoiceNo`, `printed`,
`deliveryStatus`, `error`), `InvoiceSummary`, `PeppolDeliveries` / `Delivery`. The copy of an
invoice goes through the existing `copy_ticket` command on the I event id (admin only, prints
COPIE). Error texts `err_invoice_*`, `err_customer_*`, `err_peppol_*`, `err_config_peppol` are
already in `ui/src/i18n.ts` and in the generated Android strings. `ui/src/api/ipc.ts` is
unchanged on purpose (UI parity stays green until T2 adds the callers).

**Gate notes.** R (fmt, clippy workspace and cert, `cargo test --workspace` 1856 passed,
cert tests), F (fmt, clippy `--locked`, tests `--locked`), P (`check_ipc_wrappers.py`,
`check_command_parity.py --strict` 103/103 and `--hygiene`, `check_ui_parity.py --strict`,
`vue_to_android.mjs --check`, `lane_paths.py --check`) and D (three dossier checks, `uc_matrix.py`,
em-dash scan) are green. The Android A set was NOT run: the host had 4 GB free (other lanes fill
the shared target), under the 6 GB floor, so `:pos-core:apiDump` is still to run. The FFI added
methods and records change `android/pos-core/api/pos-core.api`; whoever has disk must run
`:pos-core:apiDump`, review the diff (only invoice additions expected), then the A set. The
`lock_parity` scripts pass (`android/uccheck/Cargo.lock` refreshed offline).
