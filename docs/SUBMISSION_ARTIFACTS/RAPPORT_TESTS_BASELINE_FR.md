# Rapport de tests baseline (couverture de la build cert)

**Date et révision :** voir l'en-tête du bloc « Totaux de tests » (généré).
**Toolchain :** Rust 1.96.0 épinglée (`rust-toolchain.toml`), edition 2024, workspace Cargo, Tauri v2.
**Objet :** Photographier la posture de tests à la date de soumission,
afin que tout mouvement de tests ultérieur puisse être tracé et
attribué à des modifications futures.

---

## Vérification de build

| Configuration | Commande | Résultat |
|---|---|---|
| Par défaut (dev) | `cargo check --package moyun-pos` | OK vert |
| Certification | `cargo check --package moyun-pos --no-default-features --features cert` | OK vert |

**Vérification au niveau symboles** que le binaire cert n'embarque aucun
code d'intégrations tierces désactivées :

```
nm target/debug/moyun-pos | grep -c pos_hubrise
```

| Configuration | Symboles d'intégrations tierces |
|---|---|
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

Généré le 2026-09-25 23:11 (Europe/Brussels) sur la révision `2c4e308`.

### Workspace, fonctionnalités par défaut (`cargo test --workspace`)

| Binaire de test | Couche | Passants | Échecs | Ignorés |
|---|---|---:|---:|---:|
| `moyun_pos` (unit) | Library facade for the moyun-pos binary. | 43 | 0 | 0 |
| `access_control` (intégration) | Who may do what (`PROFILS_UTILISATEURS_FR.md` §2–3), enforced by the backend whatever the UI shows | 13 | 0 | 0 |
| `admission_mock` (intégration) | The admission image runs moyun-pos against `pos-fdm-mock` (the examiners' machine has no real FDM until the kit arrives). | 3 | 0 | 0 |
| `config_check` (intégration) | A fiscal identity the FDM would refuse is reported by the settings the UI reads at start-up, so it is fixed before the first sale. | 2 | 0 | 0 |
| `order_commands` (intégration) | The order commands the order screen calls on every tap: start an order, add / change / remove lines, reload it. | 22 | 0 | 0 |
| `refund_guards` (intégration) | A refund pays money out: whatever stops it must leave nothing a cashier could complete later. | 5 | 0 | 0 |
| `spf_e2e_basic` (intégration) | Application-level e2e tests for the SPF Finances Basic Set v1.7. | 16 | 0 | 0 |
| `spf_e2e_concurrency` (intégration) | Two sign calls firing at once on the same order/label. | 3 | 0 | 0 |
| `spf_e2e_crash_safety` (intégration) | Crash safety: what happens when something goes wrong strictly AFTER the FDM has successfully signed an event, but BEFORE the local commit finishes. | 2 | 0 | 0 |
| `spf_e2e_digital` (intégration) | Ticket delivery end to end: paper, email, or both (cert manual §III.H), through the real checkout, refund and copy commands. | 30 | 0 | 0 |
| `spf_e2e_fdm_failures` (intégration) | Exhaustive FDM-failure matrix for every fiscal command. | 26 | 0 | 0 |
| `spf_e2e_fixtures` (intégration) | SPF Finances Basic Set v1.7, replayed from the official fixtures. | 16 | 0 | 0 |
| `spf_e2e_journal` (intégration) | Order-journal semantics through the command layer. | 4 | 0 | 0 |
| `spf_e2e_no_gap` (intégration) | Regression test: SCE 2.0 "strictly sequential, no gaps" counter. | 3 | 0 | 0 |
| `spf_e2e_peripherals` (intégration) | Printer, cash drawer and back-office commands, end to end. | 18 | 0 | 0 |
| `spf_e2e_report_fixtures` (intégration) | SPF Reports v1.3, replayed end to end through the real commands. | 5 | 0 | 0 |
| `spf_e2e_reports` (intégration) | Application-level e2e tests for the SPF Finances Reports v1.3 set. | 4 | 0 | 0 |
| `spf_e2e_service_day` (intégration) | A long, realistic service day driven entirely through the Tauri command layer: several tables, interleaved orders/corrections/… | 1 | 0 | 0 |
| `spf_e2e_validation` (intégration) | Input validation and auth checks across the command layer: unknown or inactive menu items, non-positive quantities, empty/zero-total checkouts, unknown ids… | 16 | 0 | 0 |
| `ticket_copies` (intégration) | The copy picker lists what a customer can ask a copy of (manual §III, event C): sales, refunds, pre-bills and training tickets the FDM signed. | 4 | 0 | 0 |
| `pos_core` (unit) | Business types of the POS: configuration, catalogue, orders, tables, users and VAT. | 57 | 0 | 0 |
| `pos_db` (unit) | SQLite persistence: schema, migrations and typed queries, with the fiscal archive append-only. | 104 | 0 | 0 |
| `pos_fdm_mock` (unit) | Mock FDM (Checkbox-compatible GraphQL) and mock Scaleway TEM, for development, the admission image and screenshots. | 15 | 0 | 0 |
| `pos_fiscal` (unit) | SCE 2.0 protocol: event building, FDM client, counters, archive, tickets and reports. | 113 | 0 | 0 |
| `advanced_set_fixtures` (intégration) | SCE 2.0 Advanced Set v1.5 fixture validation. | 1 | 0 | 0 |
| `client_status` (intégration) | FdmClient status-query contract tests (Q100 / Q110). | 3 | 0 | 0 |
| `e2e_staging` (intégration) | Pre-submission rehearsal against a real virtual FDM. | 0 | 0 | 2 |
| `fdm_client_errors` (intégration) | fdm_client error-handling contract tests. | 12 | 0 | 0 |
| `full_service` (intégration) | Full Service Integration Test | 1 | 0 | 0 |
| `report_totals_fixtures` (intégration) | The SPF Reports v1.3 scenario, replayed through `report_totals`: the eight input tickets T1–T8 (their published requests and FDM responses) must add up to… | 6 | 0 | 0 |
| `reports_fixtures` (intégration) | SCE 2.0 Reports v1.3 fixture validation. | 8 | 0 | 0 |
| `spf_basic_set` (intégration) | SPF BasicSet Integration Tests | 10 | 0 | 0 |
| `uc_fixtures` (intégration) | SCE 2.0 Basic Set v1.7 fixture validation. | 6 | 0 | 0 |
| `pos_mail` (unit) | Digital ticket delivery: renders a signed [`pos_fiscal::ticket::FiscalTicket`] into an email, sends it via the Scaleway Transactional Email HTTP API, and… | 46 | 0 | 0 |
| `outbox` (intégration) | End-to-end outbox tests: `send_now` / `run_once` driving real SQLite (`pos-db`) rows against a mocked TEM endpoint (`wiremock`). | 13 | 0 | 0 |
| `tem_client` (intégration) | `TemClient` contract tests against a mocked Scaleway TEM endpoint. | 15 | 0 | 0 |
| `pos_print` (unit) | ESC/POS printing: tickets, reports and cash drawer, in CP858. | 92 | 0 | 0 |
| `receipt_matrix` (intégration) | Every ticket type (N sale, N refund, P, C, T) in every ticket language (FR/NL/DE), rendered through the real `format_receipt` pipeline: each must fit the 80… | 8 | 0 | 0 |
| `ticket_examples` (intégration) | §VI ticket-example generator. | 7 | 0 | 0 |
| Crates exclues de la build cert (unit) | intégrations hors v1.0.0 | 24 | 0 | 0 |
| **Total** | | **777** | **0** | **2** |

### Application, build cert (`cargo test -p moyun-pos --no-default-features --features cert`)

| Binaire de test | Couche | Passants | Échecs | Ignorés |
|---|---|---:|---:|---:|
| `moyun_pos` (unit) | Library facade for the moyun-pos binary. | 43 | 0 | 0 |
| `access_control` (intégration) | Who may do what (`PROFILS_UTILISATEURS_FR.md` §2–3), enforced by the backend whatever the UI shows | 13 | 0 | 0 |
| `admission_mock` (intégration) | The admission image runs moyun-pos against `pos-fdm-mock` (the examiners' machine has no real FDM until the kit arrives). | 3 | 0 | 0 |
| `config_check` (intégration) | A fiscal identity the FDM would refuse is reported by the settings the UI reads at start-up, so it is fixed before the first sale. | 2 | 0 | 0 |
| `order_commands` (intégration) | The order commands the order screen calls on every tap: start an order, add / change / remove lines, reload it. | 22 | 0 | 0 |
| `refund_guards` (intégration) | A refund pays money out: whatever stops it must leave nothing a cashier could complete later. | 5 | 0 | 0 |
| `spf_e2e_basic` (intégration) | Application-level e2e tests for the SPF Finances Basic Set v1.7. | 14 | 0 | 0 |
| `spf_e2e_concurrency` (intégration) | Two sign calls firing at once on the same order/label. | 3 | 0 | 0 |
| `spf_e2e_crash_safety` (intégration) | Crash safety: what happens when something goes wrong strictly AFTER the FDM has successfully signed an event, but BEFORE the local commit finishes. | 2 | 0 | 0 |
| `spf_e2e_digital` (intégration) | Ticket delivery end to end: paper, email, or both (cert manual §III.H), through the real checkout, refund and copy commands. | 30 | 0 | 0 |
| `spf_e2e_fdm_failures` (intégration) | Exhaustive FDM-failure matrix for every fiscal command. | 26 | 0 | 0 |
| `spf_e2e_fixtures` (intégration) | SPF Finances Basic Set v1.7, replayed from the official fixtures. | 16 | 0 | 0 |
| `spf_e2e_journal` (intégration) | Order-journal semantics through the command layer. | 4 | 0 | 0 |
| `spf_e2e_no_gap` (intégration) | Regression test: SCE 2.0 "strictly sequential, no gaps" counter. | 3 | 0 | 0 |
| `spf_e2e_peripherals` (intégration) | Printer, cash drawer and back-office commands, end to end. | 18 | 0 | 0 |
| `spf_e2e_report_fixtures` (intégration) | SPF Reports v1.3, replayed end to end through the real commands. | 5 | 0 | 0 |
| `spf_e2e_reports` (intégration) | Application-level e2e tests for the SPF Finances Reports v1.3 set. | 4 | 0 | 0 |
| `spf_e2e_service_day` (intégration) | A long, realistic service day driven entirely through the Tauri command layer: several tables, interleaved orders/corrections/… | 1 | 0 | 0 |
| `spf_e2e_validation` (intégration) | Input validation and auth checks across the command layer: unknown or inactive menu items, non-positive quantities, empty/zero-total checkouts, unknown ids… | 16 | 0 | 0 |
| `ticket_copies` (intégration) | The copy picker lists what a customer can ask a copy of (manual §III, event C): sales, refunds, pre-bills and training tickets the FDM signed. | 4 | 0 | 0 |
| **Total** | | **234** | **0** | **0** |

<!-- TEST_TOTALS:END -->

Les tests ignorés de `e2e_staging` constituent le harness de
répétition pré-soumission ; ils requièrent `MOYUN_STAGING_FDM_URL`
pointant vers un vrai FDM (émulateur ou FDM de développement
Checkbox), et sont exécutables via
`cargo test --package pos-fiscal --test e2e_staging -- --ignored`.
Ils ne font PAS partie de la barrière CI hors-ligne.

L'interface est couverte à part, par Playwright (`ui/e2e/`, IPC Tauri
simulée) : `cd ui && npx playwright test`.

### Familles de tests

| Finalité | Binaires de test |
|---|---|
| Domaine et noyau fiscal (unit) | `pos_core`, `pos_db`, `pos_fiscal`, `pos_print`, `pos_mail`, `pos_fdm_mock` |
| Formes des fixtures SPF Finances (requêtes et réponses officielles) | `uc_fixtures`, `reports_fixtures`, `advanced_set_fixtures` |
| Client FDM : aller-retour par mutation, statut, erreurs | `spf_basic_set`, `full_service`, `client_status`, `fdm_client_errors` |
| Rejeu des UC officiels à travers les commandes réelles (requête construite par moyun-pos comparée à la fixture) | `spf_e2e_fixtures`, `spf_e2e_report_fixtures`, `report_totals_fixtures` |
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
| `src-tauri/Cargo.toml` | Intégrations tierces marquées `optional = true` ; feature `cert` (default-off) exclut ces intégrations du binaire cert. |
| `src-tauri/src/main.rs` | Commandes Tauri des intégrations tierces gardées derrière `#[cfg(feature = "hubrise")]`. |
| `src-tauri/src/commands/checkout.rs` | **Bug fix** : passait `restaurant_est_no: ""` à `TicketParams`, passe désormais `&config.restaurant_est_no`. |
| `ui/src/router/index.ts` | Vue de gestion webshop exclue du bundle cert via variable d'environnement Vite. |

---

## Limitations connues / suivis

- **Composants UI webshop** : la vue de gestion webshop est exclue du
  bundle cert via Vite. Le côté IPC est désactivé dans la build cert
  (aucune commande webshop enregistrée).
- **Endpoint healthcheck** (`127.0.0.1:9999`) : une variante
  liveness-only cert-friendly est planifiée pour un futur polissage UI.
- **Pas de run Playwright/E2E** dans cette baseline ; le workflow CI
  existant les lance séparément.

---

## Reproductibilité

```bash
cd moyun-pos

# Build dev (features par défaut)
cargo test --workspace --no-fail-fast

# Build certification
cargo build --package moyun-pos --no-default-features --features cert
nm target/debug/moyun-pos | grep -c pos_hubrise   # → 0

# Build UI certification (tree-shake WebshopOrdersView)
VITE_FEATURE_HUBRISE=false npm --prefix ui run build
```
