# SPF package of September 2026: analysis and next actions

Date: 2026-09-26. Branch `feat/cert-dossier-kiosk-image`. Internal document, not part of the submission.

Sources analysed: everything imported under `docs/` from
https://www.systemedecaisseenregistreuse.be/fr/sce-20-0 (FR, plus the ENG mirror under
`docs/original_pdfs/ENG/`), cross-checked against the code, the tests and
`docs/SUBMISSION_ARTIFACTS/`.

## 1. What the SPF changed

| Document | Change | Effect on us |
|---|---|---|
| Basic Set 1.8 (20260821) | Timestamps now carry milliseconds; M121 `terminalId` trailing space removed; Q100/Q110 `fdmDateTime` .514 becomes .000 (not in the changelog); `UC_TestSetup_20240806.md` dropped from the Basic Set (still in Reports) | Tests: folder paths only. Code: none |
| Advanced Set 1.6 | UC226, UC243 ticket numbers and times corrected; UC243 `signPrebill` renamed `signPreBill` (we already use that); UC4119bis_2 gains `negQuantityReason`; UC4121_3 VAT A 21% becomes B 12% | Tests: folder paths only. Code: none |
| Reports 1.3 (repackaged, same number) | Sub-second parts of some timestamps set to .000; no changelog entry | Tests: folder paths only |
| Intro procedure (20260903) | 7 steps; a `Report_Testing` folder with the JSON request and response of every test, plus a photo or PDF of every printed ticket; 8 items to submit; test results must come from the official test setup; register as distributor after the certificate; up to 5 client test sites after certifier approval; the old 3-month limit and 15-day update SLA are gone | Dossier and packaging work (section 4) |
| Example tickets (20260924) | 6 official layouts: VAT ticket, copy, refund, pre-bill, bar ticket, Z turnover report | Several ticket gaps (section 2) |
| FAQ producers (20260717) | 33 questions | 5 code gaps, 1 dossier contradiction (section 2) |
| API FinCloud 1.10 | 3 changes, all between the FDM and the SPF cloud | None for the POS |
| Manual, form, POS/FDM specs, Addendum, test setup | Byte-identical | None |

## 2. Gaps found in the code (each changes the binary)

Ranked by certification risk. All verified against the source, with file references.

### 2.1 Tickets (manual section VI, official example tickets)

1. **Copy ticket.** It must reproduce the original exactly (all fields and control data), carry "COPIE", add the copy event's own FDM block, and show an "Informations FDM d'origine" block. Our copy prints neither the original block nor the original QR. The original reference is already loaded (`copy.rs:77-123`) but never passed to `TicketParams`. Same for the refund ticket (`refund.rs:104-143`).
2. **Z report printout.** Missing against the official example: establishment unit number (`ReportPrintData` has no field), the department breakdown (computed in `report_totals.rs::departments`, never printed), separate refund count and amount, the reductions rows at 0, the invoice section, and the "CECI N'EST PAS UN TICKET DE CAISSE T.V.A." line. Also check that `build_print_data` fills `fdm_id` and `short_signature` (reported as hardcoded to `None`).
3. **Pre-bill.** Second title line "Addition Provisoire" missing. Done; printed "ADDITION PROVISOIRE" since 2026-09-27, as AM art. 38 writes it.
4. **Layout details** from the examples: comma decimals and 2-decimal VAT rates; the euro sign on the total only, "TOTAL :"; a "Mode de paiement :" header; a labelled "INFORMATIONS FDM" block (fdmId, fdmDateTime, Event, eventCounter, totalCounter, shortSignature); quantity printed also for 1; label wording (Ticketnr., POS Serial, Input device MAC, Logiciel de caisse v., Unité d'établissement); legal identity at the bottom; a "Brut" VAT column; the refund ticket prints REMBOURSEMENT twice.
5. **Header wording.** The examples write "TICKET DE CAISSE T.V.A.", the manual mostly "TICKET DE CAISSE TVA" Settled by the law (2026-09-27): "TICKET DE CAISSE T.V.A." (AR 2009 art. 2, AM art. 20 a) and "CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE" (AM art. 38), now printed. The 49-character mention wraps over two lines on the 48 columns.
6. **Bar ticket.** We have none for dine-in (the kitchen ticket is HubRise only). The spec makes it optional: we declare it absent.

### 2.2 FAQ contradictions

7. **Zero-total sale (Q206, Q209).** A sale that ends at 0,00 EUR must be signed as N with a VAT ticket, and `ticketMedium: NONE` is not allowed. `checkout.rs:116-128` refuses any total at or below 0, and `REPONSES_MANUEL_CERT_FR.md:343-350` presents this as a feature (manual III.D). Fix: refuse only an order with no lines or a negative total; sign 0,00 with a payment line of amount 0 (or an empty `financials`, per the FAQ). Rewrite the III.D answer.
8. **Rounding line (Q403).** Cash rounding must be its own `ROUNDING` line in `payments` (the M112A fixture shows it). We fold it into the single PAYMENT line (`event.rs` `build_sale_input`, `checkout.rs:174-178`). Fix it, then check the X/Z payment totals and the user totals (Q509: rounding stays out of `totalAmount`).
9. **Automatic Z on overrun (Q501c).** "En cas de dépassement, la caisse créera automatiquement un event R par période de booking date manquante." We only show a reminder (`report.rs` `needs_z_report`). Proposed: when a new booking date starts and earlier dates with activity have no Z, generate one Z per missing date before the first signing of the day. Settled by AM art. 22 (section 3).
10. **Cash count declaration (M133, Basic Set).** Not implemented: `MoneyInOutLineType::DrawerDeclaration` exists (`types.rs:133`) but `do_money_in_out` sends only money in or out. Small feature: a third direction plus a Settings or cash-screen entry. The alternative is to declare it out of scope like M122 and M132.
11. **negQuantities from signOrder (Q303).** For table service the FAQ counts corrections on `signOrder`; we count them only on `signSale`. The Reports replay passes either way (the scenario has no correction under P). Done 2026-09-27: counted on the P/ORDER event, never on a pre-bill, and a table sale leaves out the corrections its orders already sent, even before a Z (`report_totals.rs`, `a_table_correction_counts_on_its_order_registration`).

Checked and compliant: Q202 and Q207 (no artificial signOrder), Q211 and Q212 (unique, stable `reference`), Q401, Q405, Q502 (the official M181 counts the earlier X report, so our device ranges match), Q504, Q505, Q506, Q509.

### 2.3 Bar / kitchen ticket (decided 2026-09-26: build it)

The official example (`20260924_bar_ticket_example_FR`) is an order slip for the bar or the kitchen. Rules: no prices; the mention "CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALIDE."; content date and time, operator, table, then "quantity x article" lines (the example also shows the number of guests, which we do not record).

12. **When it prints.** Right after a successful signing that transmits new order lines: the table order (`sign_table_order_as`, which also runs before a pre-bill) and a direct sale at checkout (its unsent entries go into the N event). Only the newly sent lines, corrections included (printed with their negative quantity), and only items whose `menu_items.kitchen_printer` is set. Nothing to print, no slip. Training mode prints it with a TRAINING mark.
13. **Where.** New `pos.json` key `kitchen_printer_address`; without it the slip goes to `printer_address`; without either, no slip. A printer failure never undoes the signing: the command reports the slip outcome (printed, nothing to print, no printer, failed) and the UI shows a toast when it failed.
14. **Code.** New `crates/pos-print/src/bar_ticket.rs` (the webshop's `kitchen_ticket.rs` stays as it is), labels in FR/NL/DE in `labels.rs`, a `sends_to_kitchen` lookup in `pos_db::queries::menu`, the outcome returned by `sign_table_order` and carried in `CheckoutResult`, the toast in `stores/order.ts` and the checkout view, the new key in the installation manual and `provision.sh`.
15. **Tests.** Rust: layout (no price, the mention, FR/NL/DE, 48 columns); a table order prints exactly its new lines once; a second registration with nothing new prints nothing; a correction prints as a negative line; an item without the kitchen flag is left out; no printer and a failed printer both keep the signed event; training mark. Playwright: the toast on failure. Traceability ID `VI_BAR`; the slip joins the `Report_Testing` PDFs and the printer photos.
16. **Dossier.** `FLUX_DONNEES_FR.md:37-51`, `DOCUMENT_PROCESSUS_TECHNIQUE_FR.md:62,177`, `CHECKLIST_DEPLOIEMENT_KIOSK_FR.md:187` already describe a kitchen slip on "Envoyer en cuisine": align them with this behaviour, and add the slip to `TICKETS_EXEMPLES_FR.md`.

### 2.4 Webshop (Q203, Q204)

Every webshop order must end in an N event in the register. Update 2026-10-03: the certified build now includes the HubRise integration (`cert = ["custom-protocol", "hubrise"]`), switched on or off per installation (`hubrise_enabled` in `pos.json`, every change logged in `integration_switch_log`); only invoicing (`peppol`) stays out. Online orders therefore end in an N event through HubRise (CLAUDE.md, section HubRise). Two consequences, still to settle in the dossier wording:

- `REPONSES_MANUEL_CERT_FR.md:217` "Webshop existe mais hors périmètre fiscal" must be aligned with the HubRise integration present in the certified binary (not "no webshop integration").
- A restaurant running the certified build must still register its webshop sales. Decision for you (section 6).

### 2.5 Split payment and unpaid table (asked 2026-09-26: plan them; built 2026-09-26 as B10 and B11)

Before B10 a sale took one payment method (cash or card), and an open table whose customer left without paying stayed open for ever (its P events are in the FDM, no N event ever closes it).

**Status (2026-09-26): items 17 to 20 are done** on branch `feat/split-payment-unpaid`:
- `crates/pos-fiscal/src/payment.rs::settle` (item 17), `CHEQUE_MEAL` meal vouchers (item 18);
- `src-tauri/src/commands/unpaid.rs` (item 19);
- the two FDM messages (item 20).

`ROUND4`, `ROUND5`, `UC4119` and `UC4109` left `uc_scope.toml`; the matrix shows them covered, none missing.

Decisions taken while building:
- The unpaid sale is allowed in training, where it signs a T: training cannot be left while an order is open, so an abandoned training table must close too.
- Takeaways (FAQ Q205): leaving an unpaid takeaway sends nothing. It stays a draft, listed on the Tables screen, until it is paid or closed unpaid, and always ends in an N without a cost center. `sign_table_order` refuses a takeaway. Side effect: a takeaway's bar or kitchen slip now prints at payment only.
- A refund is always paid back in cash. An unpaid sale is never refundable (`refund_unpaid_sale`) and is not listed.

17. **Split payment.** A sale paid with several methods, for example part card and the rest cash.
    - Checkout keeps a list of payments. Each entry is a method and an amount; the screen shows what is left to pay, and the sale is signed once the rest is zero.
    - `financials` gets one PAYMENT line per entry, with its own id per method (the reports already total per id).
    - The ROUNDING line applies to the cash part only, computed on what is left when cash pays the rest (manual rounding test 3, situation 5). Change given back follows FAQ Q402.
    - Ticket and report print every payment line. The cash drawer opens when any part is cash.
    - Refund of a split sale: the refund is paid back cash or card as today; say so in the user manual.
    - Tests:
      - request equality with `UC4119_SplitPayment` (today declared out of scope);
      - the rounding case of situation 5 without meal vouchers;
      - the report payment totals;
      - Playwright for the payment screen (partial amounts, remainder, cancel).
    - Dossier: remove the "no split payment" statement (section 4).
18. **Meal vouchers (decided 2026-09-26: build them with item 17).** `ROUND4` and `ROUND5` need meal vouchers, which are never rounded. Adding a "chèques-repas" method on top of item 17 is small: a new `PaymentType`, no rounding, its own report row. Then `ROUND4` and `ROUND5` get real tests and leave `uc_scope.toml`.
19. **Unpaid table (UC4109).** A manager action "Client parti sans payer" on an open table.
    - The operator confirms with their PIN. It signs a `signSale` N event with the order's untransmitted lines and an empty `financials`, as in `UC4109_2_Sale`, closes the order, and prints the VAT ticket for the records.
    - The turnover counts the sale; the payments do not.
    - Tests:
      - request equality with `UC4109_1` and `UC4109_2` (a single product instead of the composite menu, a declared deviation);
      - report totals;
      - refused for a cashier or in training mode as the rules require;
      - Playwright for the confirmation.
    - Dossier: the user manual and the III answers on open tables.
    - FAQ Q205 says a direct sale ends in N. Today a takeaway left unpaid is registered as a P (costCenter table 0) by the router guard, like a table. Decide in this step: an unpaid takeaway either stays a draft with nothing sent, or ends through this same unpaid-sale action. Test it with a `faq_q205_` test.
20. **FDM codes `UPDATE_POS_VATNO` and `UPDATE_POS_ESTNO`.** POS spec v2.2 marks them mandatory display, but they are not in the manual's §V table. Today they show the generic "Erreur boîte noire" text. Give each its own message in `fdmMessages.ts`, with a Playwright case.

## 3. Former SPF questions: answered or decided (2026-09-27)

No email is sent to the SPF: the owner decided on 2026-09-27 to settle every point here. Eight were answered by the texts (`docs/legislation/`, the FAQ, the UC sets); the five left open were decided by us and are documented in the dossier.

**Answered by the texts:**

| # | Point | Answer | Source | Action |
|---|---|---|---|---|
| 1 | Automatic Z per missing booking date | The missing Z reports are made at the first report of the next period; generating them automatically is allowed | AM of 29/04/2024, art. 22, second paragraph | None: B4 complies |
| 2 | Table corrections: signOrder or signSale | Counted on the signOrder for table service | FAQ Q303; AM art. 29 v | Done (section 2.2 item 11) |
| 3 | Ticket titles and mentions | « TICKET DE CAISSE T.V.A. », « CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE », « ADDITION PROVISOIRE », « PRO FORMA », « Z CHIFFRE D'AFFAIRES » | AR of 30/12/2009 art. 2; AM art. 20, 29, 30, 38 | Done (section 2.1 items 3 and 5) |
| 4 | Payment total vs rounding | Payments and roundings are separate lines; our layout matches the official Z example | AM art. 29 n and o | None |
| 5 | M111 `fdmRef` | Our premise was wrong: the official M111 request does carry `fdmRef`, and the law requires it for a full refund | AM art. 5, 3°; FAQ Q301 | None |
| 6 | Sale at 0,00 EUR | Always an N with a VAT ticket | AM art. 5, 1°; FAQ Q206, Q209 | III.D answer rewritten |
| 7 | Reports T4 VAT label | T4 is an on-site sale at the counter, so A is right | UC TestSetup; the Z totals | None in code; see the note below |
| 8 | "Nombre de convives" on the bar ticket | The bar ticket is optional; no rule requires the number of guests | Manual, optional features | None |

T4 note: our replay rings T4 up as a takeaway (label C) and declares that deviation, because pili-pos has no on-site sale without a table. The mock FDM replays the scenario's VAT, so the reports still match. `spf_e2e_report_fixtures.rs` and `COUVERTURE_RAPPORTS_FR.md` document it.

**Decided by us (no text settles them):**

| # | Point | Decision | Why | Where documented |
|---|---|---|---|---|
| 9 | M180/M181 P turnover: 3 tickets, 198 EUR published | Count every P event of the period: 4 tickets, 250 EUR | POS spec §2.2.2 `ticketCount` and AM art. 29 t count every event of the label; no text excludes T1 | `COUVERTURE_RAPPORTS_FR.md`, `RESULTATS_TESTS_UC_FR.md` |
| 10 | Unpaid sale with empty `financials` | Follow UC4109_2: an N with `financials: []` and a non-zero total | The official use case is the only worked example; the spec line (:1714) is about a zero amount | `REPONSES_MANUEL_CERT_FR.md` §III.F (unpaid row) |
| 11 | Block logout while a table order is unregistered | No block | Every exit from an order registers it (P); a failure keeps the operator on the order; only screen locking goes through, and the entries are sent at the next registration. The law asks for an event (AM art. 6, FAQ Q205), not a block | `REPONSES_MANUEL_CERT_FR.md` §III.D |
| 12 | Sign of the quantity on a refund ticket | Negative: "-1 Cola -3,04" (owner's decision) | The ticket reproduces the signed data (M111 sends -1) | `FLUX_REMBOURSEMENT_FR.md` §1 step 7; golden test `vi_refund_the_official_refund_example` |
| 13 | QR code size | Version 3 (29x29), ECC M, byte mode | The mixed-case URL needs byte mode; version 2 holds 26 bytes at M (32 at L), less than 38 | `TICKET_NUMERIQUE_FR.md` §11, `MAPPING_CONFORMITE_FR.md` |

## 4. Dossier and procedure work (docs only, after the binary is final)

- Version references: `LISTE_FONCTIONNALITES_FR.md:189-193` (also API 1.09 to 1.10), `DOCUMENT_PROCESSUS_TECHNIQUE_FR.md:8`, `MAPPING_CONFORMITE_FR.md:31-34,58,69`, the three `COUVERTURE_*`, `RESULTATS_TESTS_UC_FR.md` (its JSON extracts come from the old folders), `RAPPORT_TESTS_BASELINE_FR.md`, `REPONSES_MANUEL_CERT_FR.md:416-639`, both evidence READMEs, `CHECKLIST_DEPLOIEMENT_KIOSK_FR.md:83` (test setup file now only in the Reports folder), and the form field "Versions de spec" in `docs/internal/DOSSIER_SOUMISSION_FR.md:27`.
- New `Report_Testing` folder (procedure 3.2): per test, the JSON request and response, plus a photo or PDF of every printed ticket. The UC harness already dumps request and response (`UC_EVIDENCE_DIR`); add a PDF render of each ticket (section 5) and keep the photos of the real printer for the module2 rehearsal.
- Test setup rule (procedure 3.1): every submitted result must use the official products, prices and VAT labels. Check `seed_spf_test.sql` against `UC_TestSetup_20240806.md`.
- `ASSEMBLAGE_FINAL_FR.md`: add step 8 "register as distributor", and your decision on the up-to-5 client test sites.
- III.D answer (zero registration), the webshop line, a "no tip, no split payment" statement, the ticket examples in `TICKETS_EXEMPLES_FR.md` (regenerated after section 2.1).
- `CLAUDE.md`: new folder names and API 1.10. `docs/internal/GAP_AUDIT_FR.md` is stale on the FDM error captures; mark it superseded.

## 5. One test per SPF test (your idea)

Most of the machinery exists. `spf_e2e_fixtures.rs` and `spf_e2e_report_fixtures.rs` already build each Basic Set and Reports request with our real commands and compare it field by field with the official file, with a declared list of deviations. What is missing is coverage of the other documents, a matrix that proves it, and tests that reach the real binary.

**Step 0, move to the new sets (repointing done: no test file or script refers to the 2025 folders any more, `common/fixtures.rs` uses `20260821_UC_BasicSet_POS_FDM_1.8`, `20260821_UC_Reports_1.3`, `20260821_UC_AdvancedSet_POS_FDM_1.6`; the superseded folders are still on disk, removal not done).** Repoint `common/fixtures.rs:57-67`, `uc_fixtures.rs:35`, `advanced_set_fixtures.rs:30`, `reports_fixtures.rs:37`, `report_totals_fixtures.rs:28` and `scripts/capture_fdm_error_evidence.sh:27`. The deep diff found no assertion that breaks; run it locally to confirm. Then remove the superseded folders.

**Step 1, a generated traceability matrix.** `scripts/uc_matrix.py` lists every test ID from the three sets (M1xx, Q1xx, T1-T8, M18x, UC2xx, UC4xxx), the manual's checklist items (rounding cases, section V messages, section VI ticket items) and the FAQ numbers. It finds the tests whose names carry the ID (`m110_`, `uc201_`, `faq_q206_`, Playwright titles tagged `[M110]`), reads the declared exclusions from `scripts/uc_scope.toml` (ID plus reason, for example M122, M132, M150, M112A and the Advanced Set features we do not have), and writes `docs/SUBMISSION_ARTIFACTS/MATRICE_TRACABILITE_FR.md`. It fails when an ID has neither a test nor a declared exclusion. It replaces the hand-written `COUVERTURE_*` files, which went stale three times.

**Step 2, Rust, one test per ID.**
- Basic Set: the 15 existing request-equality tests keep their names. Add M133 (after 2.2 item 10), M171B (the `@skip` query style, response parse), Q100 and Q110 through `do_check_fdm_status`.
- Reports: the T1-T8 replay stays; add M181 and M183 names pointing at the combined Z (we only emit M184, by design).
- Advanced Set: request-equality for the cases our features cover (UC201 rounding, UC230 correction, UC4109 no payment if it maps to our flow); the rest get a named "parses and is declared out of scope" entry.
- Tickets: golden-file tests that render each of the six official example tickets from the same data and compare them line by line with our output, plus the 17 items of manual section VI per ticket type.
- Manual: the five cash-rounding cases as numeric tests; FAQ rules as `faq_q202_`, `faq_q206_`, `faq_q211_`, `faq_q403_`, `faq_q501_`, `faq_q504_`, `faq_q509_` tests.
- The same run writes the `Report_Testing` folder: JSON request and response per ID, and a PDF of each printed ticket from the ESC/POS render.

**Step 3, Playwright.**
- Mocked UI specs: tag titles with the IDs they exercise (`[M121]`, `[M123]`, `[M160]`), so the matrix sees them.
- Real binary: extend `ui/e2e/real-binary/integration.test.mjs`, which stops at adding an item today, into a full operator day against `pos-fdm-mock`: login and work in (M140), table order (M121), pre-bill (M123), cash sale (M110), refund (M111), copy (M160), money in (M130), drawer (M131), X and Z (M180 to M184), work out (M141). This is the only layer that proves the shipped binary, UI included, sends the right requests. Run it locally in the Linux Docker image with Xvfb to keep Actions minutes.

Effort: step 0 half a day, step 1 one day, step 2 three to five days, step 3 two days.

## 6. Decisions (2026-09-26)

1. Webshop (2026-09-26, superseded 2026-10-03): the first decision kept webshop sales out of the POS. Since 2026-10-03 the HubRise integration is in the cert binary, switched per installation, and every online order ends in an N event (FAQ Q203).
2. M133 cash count declaration: implement.
3. Client test sites: use as many as allowed (up to 5).
4. Automatic Z (item 9): build now, as proposed.
5. Bar/kitchen ticket: build it (section 2.3).
6. Split payment and unpaid table: plan them (section 2.5), built as phase B10 and B11. Meal vouchers (item 18): build them in B10.

## 7. Order of work

The SHA-1 of record must come after the last binary change, so:

| Phase | Content | Changes the binary |
|---|---|---|
| A | Commit the docs import; test step 0 and step 1 (matrix); SPF questions settled without an email (section 3) | No |
| B | Section 2 fixes, each with its tests from section 5 step 2, one commit each, in this order: B1 zero-total sale (7), B2 rounding line (8), B3 cash count declaration M133 (10), B4 automatic Z (9), B5 bar ticket (12-16), B6 copy and refund tickets (1), B7 pre-bill and layout details (3, 4), B8 Z report printout (2), B9 golden tests of the six official example tickets, B10 split payment and meal vouchers (17, 18), B11 unpaid table (19) and the two FDM codes (20) | Yes |
| C | Section 5 step 3 (real-binary day); the operator day against the module2 FDM simulator (`scripts/live_fdm.sh`, see `LIVE_FDM_MODULE2.md`) once module2 gives the endpoint; local gate | No |
| D | Local reproducible build twice, local image and QEMU boot, record the SHA-1 | Output only |
| E | Section 4 dossier work, `Report_Testing`, PDFs, `check_dossier.py --final`, `make_submission_zip.sh` | No |
| F | Yours: module2 simulator rehearsal and ticket photos, DE wording, signed form, submission | No |

Everything runs locally. Before merging, check the remaining Actions minutes and decide whether one final CI run is worth it.
