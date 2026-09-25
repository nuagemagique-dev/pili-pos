# Mapping de conformité - moyun-pos SCE 2.0

Mappe chaque exigence des spécifications SPF Finances vers le code qui
l'implémente et le test qui prouve son fonctionnement. C'est le cœur
du dossier cert : c'est ce que le relecteur utilisera pour vérifier
la couverture de chaque règle obligatoire.

Format :
- **Exigence.** Reformulation libre.
- **Spec.** Document source et section.
- **Code.** Fichier:lignes.
- **Test.** Fichier de test::nom du test.

## Spec technique POS v2.2

| # | Exigence | Spec | Code | Test |
|---|---|---|---|---|
| 1 | Le POS utilise GraphQL sur HTTP pour communiquer avec le FDM | POS v2.2 §III.1 | `crates/pos-fiscal/src/fdm_client.rs` | `crates/pos-fiscal/tests/spf_basic_set.rs` (10 tests wiremock) |
| 2 | 8 types d'événements supportés : N/P/T/C/I/F/S/R | POS v2.2 §II.4 | `crates/pos-fiscal/src/event.rs::EventLabel` | `crates/pos-fiscal/src/event.rs::tests` |
| 3 | Codes TVA A/B/C/D/X avec taux corrects | POS v2.2 §II.5 | `crates/pos-core/src/vat.rs::VatCode` | `crates/pos-core/src/vat.rs::tests` |
| 4 | Ticket avec en-tête (nom, adresse, TVA) + TVA par ligne + données de contrôle + QR | POS v2.2 §III.3 | `crates/pos-print/src/receipt.rs` | `crates/pos-print/tests/receipt_layout.rs` |
| 5 | QR Code Model 2, ECC M, URL 38 caractères (version 3, 29×29 : le 25×25 ne peut pas contenir l'URL, à confirmer avec le SPF) | POS v2.2 §6.5 | `crates/pos-fiscal/src/qr.rs` | `crates/pos-fiscal/src/qr.rs::tests::spec_example_url_needs_version_3_with_ecc_m` |
| 6 | Compteurs fiscaux strictement séquentiels, sans trou | POS v2.2 §II.6 | `crates/pos-fiscal/src/counter.rs::FiscalCounter::next` | `crates/pos-fiscal/src/counter.rs::tests::counter_is_sequential` |
| 7 | Allocation de compteur rollback sur échec FDM | POS v2.2 §II.6 | `src-tauri/src/commands/checkout.rs` (tx unique) | `src-tauri/tests/spf_e2e_no_gap.rs::checkout_fdm_failure_does_not_advance_counter` |
| 8 | Tous les événements fiscaux ajoutés à l'archive locale, jamais supprimés | POS v2.2 §II.7 | `crates/pos-db/src/schema.rs::SCHEMA_SQL` (table fiscal_events, pas de DELETE/UPDATE dans le code applicatif) | `crates/pos-db/src/queries/fiscal.rs::tests::archive_is_append_only` |
| 9 | Le POS bloque les transactions quand le FDM est injoignable | POS v2.2 §III.6 | `src-tauri/src/commands/checkout.rs` (pré-vérification `check_fdm_status`) | `crates/pos-fiscal/tests/full_service.rs::blocks_on_fdm_disconnect` |
| 10 | Mode formation visuellement distinct, événements T non comptés normalement | POS v2.2 §II.4.3 | `ui/src/components/TrainingBanner.vue` + flag `is_training` dans les inputs | `crates/pos-fiscal/tests/spf_basic_set.rs::training_event_does_not_advance_n_counter` |
| 11 | Signature ECDSA secp256r1 vérifiable depuis la réponse FDM | POS v2.2 §III.5 | `crates/pos-fiscal/src/signature.rs::verify_signature` | `crates/pos-fiscal/src/signature.rs::tests` |
| 12 | Retry idempotent sur échecs FDM transitoires | POS v2.2 §III.7 | `crates/pos-fiscal/src/inputs.rs::idempotency_key` | `crates/pos-fiscal/src/inputs.rs::tests::idempotent_retry_returns_same_key` |
| 13 | JSON canonique pour reproductibilité de signature | POS v2.2 §III.5 (entrées de signature) | `crates/pos-fiscal/src/canonical.rs` | `crates/pos-fiscal/src/canonical.rs::tests::deterministic_50x` |
| 14 | Les événements C portent un `fdmRef` vers l'original | POS v2.2 §II.4.6 | `src-tauri/src/commands/copy.rs` | e2e `spf_e2e_basic::{m160_*, m161_*, m172_*}` + fixture UC_BasicSet M160/M161 |
| 15 | Événements I avec n° TVA client + cost-center optionnel | POS v2.2 §II.4.5 | `src-tauri/src/commands/invoice.rs::do_sign_invoice_fiscal` (existe, `#[cfg(feature = "peppol")]` - **non compilé dans la build cert v1.0.0**, event I hors périmètre v1.0.0, prévu v1.1, cf. *DECISION_PEPPOL_AP_FR.md*) | e2e `spf_e2e_basic::m150_*` + fixture UC_BasicSet M150 (features par défaut, pas la build cert) |
| 16 | F-WAR ouverture tiroir émis à chaque kick tiroir | POS v2.2 §II.4.7 | `src-tauri/src/commands/financial.rs::open_drawer_fiscal` | e2e `spf_e2e_basic::m131_*` + fixture UC_BasicSet M131 |
| 17 | S pointage entrée/sortie pour Dimona ONSS | POS v2.2 §II.4.8 | `src-tauri/src/commands/social.rs::{work_in,work_out}` | e2e `spf_e2e_basic::{m140_*, m141_*}` + fixtures UC_BasicSet M140/M141 |
| 18 | Rapport Z atomique (Turnover + User) | Reports v1.3 §II.3 | `src-tauri/src/commands/report.rs::do_generate_z_report` (mutation M184 combinée + tx unique) | e2e `spf_e2e_reports::m184_*` + `reports_fixtures.rs` |

## Spec FDM v1.4 + Addendum 1

| # | Exigence | Spec | Code | Test |
|---|---|---|---|---|
| 19 | Le POS ne parle PAS directement à SPF Finances | FDM v1.4 §I.1 | (architectural - le FDM est le seul canal sortant ; aucun appel SPF Finances nulle part dans `moyun-pos`) | grep vérifié : `grep -r "finances" crates/ src-tauri/ → 0 hit` |
| 20 | La short signature FDM est affichée sur le ticket | FDM v1.4 §IV.2 | `crates/pos-print/src/receipt.rs::render_control_block` | `crates/pos-print/tests/receipt_layout.rs::receipt_includes_short_signature` |
| 21 | Le statut FDM est remonté à l'UI | FDM v1.4 §V | `src-tauri/src/state.rs` (mirror dans store Pinia) | E2E manuel (kill du mock FDM → pilule de statut UI) |

## Basic Set v1.7

Les 22 UC obligatoires sont couverts. Voir `COUVERTURE_FIXTURES_FR.md`
pour la matrice pass/fail par UC. Tests fixture dans
`crates/pos-fiscal/tests/uc_fixtures.rs`.

## Reports v1.3

Les 5 UC sont couverts (M180-M184). Voir `COUVERTURE_RAPPORTS_FR.md`.
Tests dans `crates/pos-fiscal/tests/reports_fixtures.rs`.

## Advanced Set v1.5

Les 141 fixtures se chargent et se désérialisent proprement. Voir
`COUVERTURE_ADVANCED_SET_FR.md` pour le détail par UC. Tests dans
`crates/pos-fiscal/tests/advanced_set_fixtures.rs`.

## Remboursements & corrections

Voir `FLUX_REMBOURSEMENT_FR.md`. M111 (remboursement complet),
M112B (remboursement partiel comme événement séparé — le pattern
déclaré, cf. e2e `spf_e2e_basic::m112b_*`), UC4122 (note de crédit)
couverts dans `src-tauri/src/commands/refund.rs`. M132 (correction
de paiement) : reporté, non déclaré.

## Peppol B2B (obligation parallèle, hors SCE-2.0)

Voir `DECISION_PEPPOL_AP_FR.md`. Sérialiseur UBL + trait AccessPoint
implémentés dans `crates/pos-peppol` ; IPC dans
`src-tauri/src/commands/invoice.rs`.

## Hygiène de build

| Préoccupation | Preuve |
|---|---|
| Binaire cert sans intégrations tierces | `cargo build --release --no-default-features --features cert` ; `nm moyun-pos | grep -c pos_hubrise → 0` |
| Tous les tests verts | `cargo test --workspace` → 190 tests passent |
| Lint propre | `cargo clippy --workspace -- -D warnings` (à exécuter avant soumission) |
| Format cohérent | `cargo fmt --all` |
| Build reproductible | `Cargo.lock` committé ; Rust 1.95.0 épinglé dans `rust-toolchain.toml` |
