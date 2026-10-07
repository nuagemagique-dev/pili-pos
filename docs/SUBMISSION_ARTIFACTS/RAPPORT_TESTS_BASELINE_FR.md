# Rapport de tests baseline (couverture de la build cert)

**Date et révision :** voir l'en-tête du bloc « Totaux de tests » (généré).
**Toolchain :** Rust 1.99.0 épinglée (`rust-toolchain.toml`), edition 2024, workspace Cargo, Tauri v2.
**Objet :** Photographier la posture de tests à la date de soumission,
afin que tout mouvement de tests ultérieur puisse être tracé et
attribué à des modifications futures.

---

## Vérification de build

| Configuration | Commande | Résultat |
|---|---|---|
| Par défaut (dev) | `cargo check --package pili-pos` | OK vert |
| Certification | `cargo check --package pili-pos --no-default-features --features cert` | OK vert |

**Vérification du périmètre du binaire cert** (`scripts/build_and_hash.sh`) :

```
strings target/release/pili-pos | grep -c integration_switch_log   # > 0 : commandes en ligne présentes
nm target/release/pili-pos | grep -c pos_peppol                    # 0 : facturation B2B absente
```

Les commandes en ligne font partie du binaire certifié et s'activent
par installation (`COMMANDES_EN_LIGNE_FR.md`) ; la facturation B2B
(feature `peppol`) en est absente.

---|---|
| Par défaut | **493** |
| Cert (`--no-default-features --features cert`) | **0** |

Les colonnes de schéma liées aux intégrations webshop (noms de colonnes
du schéma SQL embarqué dans `pos-db`) restent présentes dans la
définition du schéma pour compatibilité de lecture avec des bases
créées par une build dev. Aucun chemin de code vivant n'écrit ces
colonnes dans la build cert.

---

## Totaux de tests

Les totaux ci-dessous ne sont jamais recopiés à la main : ils sont
régénérés par `scripts/generate_test_report.py`, qui exécute les deux
suites et réécrit le bloc. La colonne « Couche » reprend la première
phrase du commentaire de module (`//!`) de chaque binaire de test. Le
script est relancé sur la révision soumise, avant l'envoi ; la CI exécute les mêmes suites à chaque push.

<!-- TEST_TOTALS:BEGIN (scripts/generate_test_report.py) -->

Généré le 2026-10-04 11:07 (Europe/Brussels) sur la révision `459ae3c`.

### Workspace, fonctionnalités par défaut (`cargo test --workspace`)

| Binaire de test | Couche | Passants | Échecs | Ignorés |
|---|---|---:|---:|---:|
| `pili_pos` (unit) | Library facade for the pili-pos binary. | 55 | 0 | 0 |
| `access_control` (intégration) | Who may do what (`PROFILS_UTILISATEURS_FR.md` §2–3), enforced by the backend whatever the UI shows | 20 | 0 | 0 |
| `admission_mock` (intégration) | The admission image runs pili-pos against `pos-fdm-mock` (the examiners' machine has no real FDM until the kit arrives). | 4 | 0 | 0 |
| `config_check` (intégration) | A fiscal identity the FDM would refuse is reported by the settings the UI reads at start-up, so it is fixed before the first sale. | 2 | 0 | 0 |
| `error_codes` (intégration) | Every error code the backend can return has a text in the three languages of the screen (`ui/src/i18n.ts`, key `err_<code>`), so a refusal is never shown as… | 4 | 0 | 0 |
| `fdm_status_cache` (intégration) | The header polls the FDM status from every screen that shows it. | 3 | 0 | 0 |
| `hubrise_cancellations` (intégration) | Online orders HubRise cancels or changes after the POS got them. | 13 | 0 | 0 |
| `hubrise_catalog` (intégration) | The POS menu mirrors the HubRise catalog: applied at start, on a catalog event and by the drift check, with stable product ids, the VAT labels of the… | 12 | 0 | 0 |
| `hubrise_config` (intégration) | The installation's HubRise client comes from the `hubrise` section of `pos.json`, its secret from a file. | 4 | 0 | 0 |
| `hubrise_discounts` (intégration) | Discounts on online orders: each is spread over the product lines (the cent to the largest share) and sent as a priceChange of scope EVENT on every line it… | 4 | 0 | 0 |
| `hubrise_modifiers` (intégration) | Products with options on the order screen: the groups come from the mirrored catalog, the backend checks each group's minimum and maximum, prices the… | 6 | 0 | 0 |
| `hubrise_orders_ingest` (intégration) | Online orders from the mock reach the board resolved against the menu, acknowledged, and never among the POS's tables and takeaways. | 9 | 0 | 0 |
| `hubrise_orders_signing` (intégration) | Online orders signed by the FDM: prepaid ones end in an N at acceptance (APP or ONLINE), unpaid ones in a P at acceptance then an N at handover with the… | 16 | 0 | 0 |
| `hubrise_print_access` (intégration) | Webshop slips are reprinted by logged-in staff only. | 1 | 0 | 0 |
| `hubrise_switch` (intégration) | The installation's HubRise switch. | 5 | 0 | 0 |
| `hubrise_transport` (intégration) | HubRise transport against the in-process mock: callback registration, event polling, acknowledgements, the outbox, errors. | 17 | 0 | 0 |
| `live_combined` (intégration) | Online orders end to end, live on both sides: the orders come from the real HubRise API (test location), and their events are signed by a live FDM (the… | 0 | 0 | 2 |
| `live_fdm` (intégration) | An operator day signed by a live FDM (the module2 cloud simulator). | 0 | 0 | 6 |
| `live_hubrise` (intégration) | The HubRise integration against the real HubRise API, on a test location, with the FDM mock signing. | 0 | 0 | 3 |
| `order_commands` (intégration) | The order commands the order screen calls on every tap: start an order, add / change / remove lines, reload it. | 22 | 0 | 0 |
| `refund_guards` (intégration) | A refund pays money out: whatever stops it must leave nothing a cashier could complete later. | 7 | 0 | 0 |
| `remote_support` (intégration) | Remote support (RustDesk) is started from the POS by the logged-in manager, confirmed by their PIN, one session at a time. | 14 | 0 | 0 |
| `screen_state` (intégration) | State the screens read back from the backend, beyond the fiscal record itself | 3 | 0 | 0 |
| `spf_e2e_auto_z` (intégration) | Producers FAQ Q501c: "En cas de dépassement, la caisse créera automatiquement un event R par période de booking date manquante." | 5 | 0 | 0 |
| `spf_e2e_bar_slip` (intégration) | Bar / kitchen order slip (traceability ID VI_BAR, SPF example bar ticket of 2026-09-24). | 11 | 0 | 0 |
| `spf_e2e_basic` (intégration) | Application-level e2e tests for the SPF Finances Basic Set v1.8. | 17 | 0 | 0 |
| `spf_e2e_concurrency` (intégration) | Two sign calls firing at once on the same order/label. | 3 | 0 | 0 |
| `spf_e2e_copy_refund_tickets` (intégration) | What the copy and refund tickets show (SPF example tickets of 2026-09-24, POS description v2.2 §1.8, §6.5, §6.6). | 9 | 0 | 0 |
| `spf_e2e_crash_safety` (intégration) | Crash safety: what happens when something goes wrong strictly AFTER the FDM has successfully signed an event, but BEFORE the local commit finishes. | 2 | 0 | 0 |
| `spf_e2e_digital` (intégration) | Ticket delivery end to end: paper, email, or both (cert manual §III.H), through the real checkout, refund and copy commands. | 34 | 0 | 0 |
| `spf_e2e_faq` (intégration) | Rules of the SPF producers' FAQ (20260717) that no Basic Set or Reports scenario exercises on its own, played through the commands the UI calls. | 7 | 0 | 0 |
| `spf_e2e_fdm_failures` (intégration) | Exhaustive FDM-failure matrix for every fiscal command. | 27 | 0 | 0 |
| `spf_e2e_fdm_response_check` (intégration) | Every fiscal command refuses an FDM response that does not answer its request. | 13 | 0 | 0 |
| `spf_e2e_fixtures` (intégration) | SPF Finances Basic Set v1.8, replayed from the official fixtures. | 21 | 0 | 0 |
| `spf_e2e_journal` (intégration) | Order-journal semantics through the command layer. | 4 | 0 | 0 |
| `spf_e2e_no_gap` (intégration) | Regression test: SCE 2.0 "strictly sequential, no gaps" counter. | 2 | 0 | 0 |
| `spf_e2e_payments` (intégration) | How a sale's payment reaches the FDM. | 8 | 0 | 0 |
| `spf_e2e_peripherals` (intégration) | Printer, cash drawer and back-office commands, end to end. | 18 | 0 | 0 |
| `spf_e2e_report_fixtures` (intégration) | SPF Reports v1.3, replayed end to end through the real commands. | 14 | 0 | 0 |
| `spf_e2e_report_printout` (intégration) | The printed X and Z turnover reports carry what the AM of 29 April 2024 art. | 2 | 0 | 0 |
| `spf_e2e_reports` (intégration) | Application-level e2e tests for the SPF Finances Reports v1.3 set. | 4 | 0 | 0 |
| `spf_e2e_rounding` (intégration) | Certification manual, "Arrondissement des méthodes de paiement": the five situations, with their evidence. | 6 | 0 | 0 |
| `spf_e2e_service_day` (intégration) | A long, realistic service day driven entirely through the Tauri command layer: several tables, interleaved orders/corrections/… | 1 | 0 | 0 |
| `spf_e2e_signing_serialized` (intégration) | One signing lock, and requests whose answer was lost. | 14 | 0 | 0 |
| `spf_e2e_split_payment` (intégration) | A sale paid in several parts: card, meal vouchers and cash for the rest, through the command the checkout screen calls. | 9 | 0 | 0 |
| `spf_e2e_unpaid` (intégration) | A sale closed without payment: the customer left without paying (Advanced Set UC4109, `commands/unpaid.rs`). | 10 | 0 | 0 |
| `spf_e2e_validation` (intégration) | Input validation and auth checks across the command layer: unknown or inactive menu items, non-positive quantities, empty/zero-total checkouts, unknown ids… | 16 | 0 | 0 |
| `ticket_copies` (intégration) | The copy picker lists what a customer can ask a copy of (manual §III, event C): sales, refunds, pre-bills and training tickets the FDM signed. | 4 | 0 | 0 |
| `pos_catalog` (unit) | The POS menu as a mirror of an external catalog. | 12 | 0 | 0 |
| `apply` (intégration) | Applying a catalog to a real POS database. | 16 | 0 | 0 |
| `pos_core` (unit) | Business types of the POS: configuration, catalogue, orders, tables, users and VAT. | 61 | 0 | 0 |
| `pos_db` (unit) | SQLite persistence: schema, migrations and typed queries, with the fiscal archive append-only. | 124 | 0 | 0 |
| `pos_fdm_mock` (unit) | Mock FDM (SCE 2.0 GraphQL) and mock Scaleway TEM, for development, the admission image, screenshots and the certification evidence. | 30 | 0 | 0 |
| `module2_shape` (intégration) | The mock answers in the shape of module2's simulator. | 1 | 0 | 0 |
| `pos_fiscal` (unit) | SCE 2.0 protocol: event building, FDM client, counters, archive, tickets and reports. | 167 | 0 | 0 |
| `advanced_set_fixtures` (intégration) | SCE 2.0 Advanced Set v1.6 fixture validation. | 1 | 0 | 0 |
| `client_auth` (intégration) | FdmClient bearer token: a cloud FDM (the module2 simulator) needs `Authorization: Bearer <token>`; a LAN FDM gets no such header. | 4 | 0 | 0 |
| `client_status` (intégration) | FdmClient status-query contract tests (Q100 / Q110). | 5 | 0 | 0 |
| `e2e_staging` (intégration) | Pre-submission rehearsal against a real virtual FDM. | 0 | 0 | 2 |
| `fdm_client_errors` (intégration) | fdm_client error-handling contract tests. | 21 | 0 | 0 |
| `fdm_https_mock` (intégration) | The POS's HTTPS to the FDM, offline: the mock FDM serves TLS like module2 (an X.509 v1 server certificate without names, signed by its CA) and here requires… | 3 | 0 | 0 |
| `fdm_tls` (intégration) | The FDM's certificate is trusted on its CA's signature and its dates (`pos_fiscal::fdm_tls`), with module2's real certificates:… | 6 | 0 | 0 |
| `full_service` (intégration) | Full Service Integration Test | 1 | 0 | 0 |
| `module2_recorded` (intégration) | What module2's simulator really answered (recorded on 2026-09-27 in `crates/pos-fdm-mock/fixtures/module2/`) goes through the POS's own parsing and response… | 3 | 0 | 0 |
| `price_change_uc` (intégration) | Price changes against the SPF's own examples: the transaction the POS builds for Advanced Set UC223 (a 50 % ticket discount spread over two lines at B and… | 2 | 0 | 0 |
| `report_totals_fixtures` (intégration) | The SPF Reports v1.3 scenario, replayed through `report_totals`: the eight input tickets T1–T8 (their published requests and FDM responses) must add up to… | 6 | 0 | 0 |
| `reports_fixtures` (intégration) | SCE 2.0 Reports v1.3 fixture validation. | 8 | 0 | 0 |
| `sign_messages` (intégration) | POS description v2.2, Message: a warning the FDM attaches to a mutation's response is shown at least once per bookingDate. | 1 | 0 | 0 |
| `sign_request` (intégration) | Sign requests are kept byte for byte, so one whose answer was lost is sent again identically (POS description v2.2, duplicate requests). | 6 | 0 | 0 |
| `spf_basic_set` (intégration) | SPF BasicSet Integration Tests | 10 | 0 | 0 |
| `uc_fixtures` (intégration) | SCE 2.0 Basic Set v1.8 fixture validation. | 6 | 0 | 0 |
| `webshop_resolve` (intégration) | Online orders resolved against a POS menu that mirrors the mock's seed catalog: every template, and every reason an order is held. | 23 | 0 | 0 |
| `hubrise_shape` (intégration) | The mock answers in the shape of the real HubRise API. | 2 | 0 | 0 |
| `router` (intégration) | The mock's HTTP API, driven in-process with `tower::oneshot`. | 24 | 0 | 0 |
| `doc_samples` (intégration) | The JSON samples of the HubRise API documentation (copied to `tests/fixtures/`) parse into the wire types, keep their `ref` fields, and survive a round trip. | 10 | 0 | 0 |
| `pos_mail` (unit) | Digital ticket delivery: renders a signed [`pos_fiscal::ticket::FiscalTicket`] into an email, sends it via the Scaleway Transactional Email HTTP API, and… | 46 | 0 | 0 |
| `outbox` (intégration) | End-to-end outbox tests: `send_now` / `run_once` driving real SQLite (`pos-db`) rows against a mocked TEM endpoint (`wiremock`). | 13 | 0 | 0 |
| `tem_client` (intégration) | `TemClient` contract tests against a mocked Scaleway TEM endpoint. | 15 | 0 | 0 |
| `pos_print` (unit) | ESC/POS printing: tickets, reports and cash drawer, in CP858. | 116 | 0 | 0 |
| `official_examples` (intégration) | Golden tests of the six SPF example tickets of 2026-09-24 (`docs/20260904_Example_tickets/`): the example's own data goes through the real print pipeline,… | 6 | 0 | 0 |
| `receipt_matrix` (intégration) | Every ticket type (N sale, N refund, P, C, T) in every ticket language (FR/NL/DE), rendered through the real `format_receipt` pipeline: each must fit the 80… | 9 | 0 | 0 |
| `report_matrix` (intégration) | Every report type (X, Z, user X, user Z) in every ticket language (FR/NL/DE), rendered through the real `format_report` / `format_user_report` pipeline:… | 8 | 0 | 0 |
| `ticket_examples` (intégration) | §VI ticket-example generator. | 12 | 0 | 0 |
| `pos_printer_mock` (unit) | A network thermal printer for development, demos and tests, standing in for the Epson TM-m30III the POS prints to (raw TCP, port 9100). | 7 | 0 | 0 |
| Crates exclues de la build cert (unit) | intégrations hors v1.0.0 | 72 | 0 | 0 |
| **Total** | | **1339** | **0** | **13** |

### Application, build cert (`cargo test -p pili-pos --no-default-features --features cert`)

| Binaire de test | Couche | Passants | Échecs | Ignorés |
|---|---|---:|---:|---:|
| `pili_pos` (unit) | Library facade for the pili-pos binary. | 55 | 0 | 0 |
| `access_control` (intégration) | Who may do what (`PROFILS_UTILISATEURS_FR.md` §2–3), enforced by the backend whatever the UI shows | 20 | 0 | 0 |
| `admission_mock` (intégration) | The admission image runs pili-pos against `pos-fdm-mock` (the examiners' machine has no real FDM until the kit arrives). | 4 | 0 | 0 |
| `config_check` (intégration) | A fiscal identity the FDM would refuse is reported by the settings the UI reads at start-up, so it is fixed before the first sale. | 2 | 0 | 0 |
| `error_codes` (intégration) | Every error code the backend can return has a text in the three languages of the screen (`ui/src/i18n.ts`, key `err_<code>`), so a refusal is never shown as… | 4 | 0 | 0 |
| `fdm_status_cache` (intégration) | The header polls the FDM status from every screen that shows it. | 3 | 0 | 0 |
| `hubrise_cancellations` (intégration) | Online orders HubRise cancels or changes after the POS got them. | 13 | 0 | 0 |
| `hubrise_catalog` (intégration) | The POS menu mirrors the HubRise catalog: applied at start, on a catalog event and by the drift check, with stable product ids, the VAT labels of the… | 12 | 0 | 0 |
| `hubrise_config` (intégration) | The installation's HubRise client comes from the `hubrise` section of `pos.json`, its secret from a file. | 4 | 0 | 0 |
| `hubrise_discounts` (intégration) | Discounts on online orders: each is spread over the product lines (the cent to the largest share) and sent as a priceChange of scope EVENT on every line it… | 4 | 0 | 0 |
| `hubrise_modifiers` (intégration) | Products with options on the order screen: the groups come from the mirrored catalog, the backend checks each group's minimum and maximum, prices the… | 6 | 0 | 0 |
| `hubrise_orders_ingest` (intégration) | Online orders from the mock reach the board resolved against the menu, acknowledged, and never among the POS's tables and takeaways. | 9 | 0 | 0 |
| `hubrise_orders_signing` (intégration) | Online orders signed by the FDM: prepaid ones end in an N at acceptance (APP or ONLINE), unpaid ones in a P at acceptance then an N at handover with the… | 16 | 0 | 0 |
| `hubrise_print_access` (intégration) | Webshop slips are reprinted by logged-in staff only. | 1 | 0 | 0 |
| `hubrise_switch` (intégration) | The installation's HubRise switch. | 5 | 0 | 0 |
| `hubrise_transport` (intégration) | HubRise transport against the in-process mock: callback registration, event polling, acknowledgements, the outbox, errors. | 17 | 0 | 0 |
| `live_combined` (intégration) | Online orders end to end, live on both sides: the orders come from the real HubRise API (test location), and their events are signed by a live FDM (the… | 0 | 0 | 2 |
| `live_fdm` (intégration) | An operator day signed by a live FDM (the module2 cloud simulator). | 0 | 0 | 6 |
| `live_hubrise` (intégration) | The HubRise integration against the real HubRise API, on a test location, with the FDM mock signing. | 0 | 0 | 3 |
| `order_commands` (intégration) | The order commands the order screen calls on every tap: start an order, add / change / remove lines, reload it. | 22 | 0 | 0 |
| `refund_guards` (intégration) | A refund pays money out: whatever stops it must leave nothing a cashier could complete later. | 7 | 0 | 0 |
| `remote_support` (intégration) | Remote support (RustDesk) is started from the POS by the logged-in manager, confirmed by their PIN, one session at a time. | 14 | 0 | 0 |
| `screen_state` (intégration) | State the screens read back from the backend, beyond the fiscal record itself | 3 | 0 | 0 |
| `spf_e2e_auto_z` (intégration) | Producers FAQ Q501c: "En cas de dépassement, la caisse créera automatiquement un event R par période de booking date manquante." | 5 | 0 | 0 |
| `spf_e2e_bar_slip` (intégration) | Bar / kitchen order slip (traceability ID VI_BAR, SPF example bar ticket of 2026-09-24). | 11 | 0 | 0 |
| `spf_e2e_basic` (intégration) | Application-level e2e tests for the SPF Finances Basic Set v1.8. | 15 | 0 | 0 |
| `spf_e2e_concurrency` (intégration) | Two sign calls firing at once on the same order/label. | 3 | 0 | 0 |
| `spf_e2e_copy_refund_tickets` (intégration) | What the copy and refund tickets show (SPF example tickets of 2026-09-24, POS description v2.2 §1.8, §6.5, §6.6). | 9 | 0 | 0 |
| `spf_e2e_crash_safety` (intégration) | Crash safety: what happens when something goes wrong strictly AFTER the FDM has successfully signed an event, but BEFORE the local commit finishes. | 2 | 0 | 0 |
| `spf_e2e_digital` (intégration) | Ticket delivery end to end: paper, email, or both (cert manual §III.H), through the real checkout, refund and copy commands. | 34 | 0 | 0 |
| `spf_e2e_faq` (intégration) | Rules of the SPF producers' FAQ (20260717) that no Basic Set or Reports scenario exercises on its own, played through the commands the UI calls. | 7 | 0 | 0 |
| `spf_e2e_fdm_failures` (intégration) | Exhaustive FDM-failure matrix for every fiscal command. | 27 | 0 | 0 |
| `spf_e2e_fdm_response_check` (intégration) | Every fiscal command refuses an FDM response that does not answer its request. | 13 | 0 | 0 |
| `spf_e2e_fixtures` (intégration) | SPF Finances Basic Set v1.8, replayed from the official fixtures. | 21 | 0 | 0 |
| `spf_e2e_journal` (intégration) | Order-journal semantics through the command layer. | 4 | 0 | 0 |
| `spf_e2e_no_gap` (intégration) | Regression test: SCE 2.0 "strictly sequential, no gaps" counter. | 2 | 0 | 0 |
| `spf_e2e_payments` (intégration) | How a sale's payment reaches the FDM. | 8 | 0 | 0 |
| `spf_e2e_peripherals` (intégration) | Printer, cash drawer and back-office commands, end to end. | 18 | 0 | 0 |
| `spf_e2e_report_fixtures` (intégration) | SPF Reports v1.3, replayed end to end through the real commands. | 14 | 0 | 0 |
| `spf_e2e_report_printout` (intégration) | The printed X and Z turnover reports carry what the AM of 29 April 2024 art. | 2 | 0 | 0 |
| `spf_e2e_reports` (intégration) | Application-level e2e tests for the SPF Finances Reports v1.3 set. | 4 | 0 | 0 |
| `spf_e2e_rounding` (intégration) | Certification manual, "Arrondissement des méthodes de paiement": the five situations, with their evidence. | 6 | 0 | 0 |
| `spf_e2e_service_day` (intégration) | A long, realistic service day driven entirely through the Tauri command layer: several tables, interleaved orders/corrections/… | 1 | 0 | 0 |
| `spf_e2e_signing_serialized` (intégration) | One signing lock, and requests whose answer was lost. | 14 | 0 | 0 |
| `spf_e2e_split_payment` (intégration) | A sale paid in several parts: card, meal vouchers and cash for the rest, through the command the checkout screen calls. | 9 | 0 | 0 |
| `spf_e2e_unpaid` (intégration) | A sale closed without payment: the customer left without paying (Advanced Set UC4109, `commands/unpaid.rs`). | 10 | 0 | 0 |
| `spf_e2e_validation` (intégration) | Input validation and auth checks across the command layer: unknown or inactive menu items, non-positive quantities, empty/zero-total checkouts, unknown ids… | 16 | 0 | 0 |
| `ticket_copies` (intégration) | The copy picker lists what a customer can ask a copy of (manual §III, event C): sales, refunds, pre-bills and training tickets the FDM signed. | 4 | 0 | 0 |
| **Total** | | **480** | **0** | **11** |

<!-- TEST_TOTALS:END -->

Les tests ignorés de `e2e_staging` (niveau client FDM) et de
`live_fdm` (journée d'exploitation par les vraies commandes)
constituent le harness de répétition pré-soumission ; ils requièrent
un vrai FDM, le simulateur en ligne module2, et s'exécutent via
`scripts/live_fdm.sh` (ou
`cargo test --package pos-fiscal --test e2e_staging -- --ignored`).
Ils ne font PAS partie de la barrière CI hors-ligne.

L'interface est couverte à part, par Playwright (`ui/e2e/`, IPC Tauri
simulée) : `cd ui && npx playwright test`. Dernier passage (2026-09-26) :
157 passants, 0 échec, 0 ignoré.

Le vrai binaire de certification, avec sa base SQLite et son IPC réels,
est piloté à part par WebDriver (`tauri-driver` + `WebKitWebDriver`) :
`ui/e2e/real-binary/integration.test.mjs`, job CI `integration`
(connexion, tables, menu du jeu d'essai, ajout d'une ligne et total).

### Familles de tests

| Finalité | Binaires de test |
|---|---|
| Domaine et noyau fiscal (unit) | `pos_core`, `pos_db`, `pos_fiscal`, `pos_print`, `pos_mail`, `pos_fdm_mock` |
| Formes des fixtures SPF Finances (requêtes et réponses officielles) | `uc_fixtures`, `reports_fixtures`, `advanced_set_fixtures` |
| Client FDM : aller-retour par mutation, statut, erreurs | `spf_basic_set`, `full_service`, `client_status`, `fdm_client_errors` |
| Rejeu des UC officiels à travers les commandes réelles (requête construite par pili-pos comparée à la fixture) | `spf_e2e_fixtures`, `spf_e2e_report_fixtures`, `report_totals_fixtures` |
| Orchestration applicative (commande → FDM → transaction → archive) | `spf_e2e_basic`, `spf_e2e_reports`, `spf_e2e_journal`, `spf_e2e_service_day`, `order_commands` |
| Numérotation sans trou, pannes FDM, crash, concurrence | `spf_e2e_no_gap`, `spf_e2e_fdm_failures`, `spf_e2e_crash_safety`, `spf_e2e_concurrency` |
| Profils utilisateurs, validation des entrées, remboursements, configuration | `access_control`, `spf_e2e_validation`, `refund_guards`, `config_check` |
| Ticket numérique, impression, tiroir | `spf_e2e_digital`, `outbox`, `tem_client`, `spf_e2e_peripherals`, `receipt_matrix`, `ticket_examples` |
| Image d'admission (POS contre `pos-fdm-mock`) | `admission_mock` |

---

## Rot de tests pré-existant corrigé lors du baselining

Les deux tests d'intégration pos-fiscal ne compilaient plus contre
l'API actuelle. Corrigé :

| Fichier | Sites | Raison |
|---|---:|---|
| `crates/pos-fiscal/tests/full_service.rs` | 5 | Champ `backup_dir` manquant sur `PosConfig` ; champ `rounding` manquant sur `TicketParams` (×2) ; 8e argument `Option<&FdmReference>` manquant sur `build_sale_input` (×2). |
| `crates/pos-fiscal/tests/spf_basic_set.rs` | 6 | `backup_dir` manquant sur `PosConfig` ; 8e argument manquant sur `build_sale_input` (×5). |

Toutes les corrections étaient mécaniques - `None` pour les nouveaux
champs/arguments optionnels. Aucun changement de sémantique de test.

---

## Mises en conformité lors de cette baseline

| Fichier | Changement |
|---|---|
| `src-tauri/Cargo.toml` | Feature `cert` = `custom-protocol` + `hubrise` ; la facturation B2B (`peppol`) reste hors du binaire cert. |
| `src-tauri/src/main.rs` | Commandes Tauri de la facturation B2B gardées derrière `#[cfg(feature = "peppol")]`. |
| `crates/pos-engine/src/commands/checkout.rs` | **Bug fix** : passait `restaurant_est_no: ""` à `TicketParams`, passe désormais `&config.restaurant_est_no`. |
| `ui/src/router/index.ts` | Écran des commandes en ligne accessible seulement si l'intégration est activée pour l'installation. |

---

## Limitations connues / suivis

- **Commandes en ligne** : présentes dans le binaire cert, coupées par
  défaut (`hubrise_enabled: false`) ; un Gérant les active dans
  Réglages, chaque changement est journalisé.
- **Endpoint healthcheck** (`127.0.0.1:9999`) : une variante
  liveness-only cert-friendly est planifiée pour un futur polissage UI.
- **Playwright** : hors du tableau ci-dessus (généré depuis `cargo
  test`), totaux donnés plus haut ; la CI (`.github/workflows/test.yml`)
  le relance à chaque push.

---

## Reproductibilité

```bash
cd pili-pos

# Build dev (features par défaut)
cargo test --workspace --no-fail-fast

# Build certification
cargo build --package pili-pos --no-default-features --features cert
nm target/debug/pili-pos | grep -c pos_peppol   # → 0

# Build UI (un seul bundle, commandes en ligne activées à l'exécution)
npm --prefix ui run build
```
