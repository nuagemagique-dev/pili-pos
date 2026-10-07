# Mapping de conformité - pili-pos SCE 2.0

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
| 2 | 8 types d'événements supportés : N/P/T/C/I/F/S/R | POS v2.2 §II.4 | `crates/pos-fiscal/src/types.rs::EventLabel` | `crates/pos-fiscal/src/event.rs::tests` |
| 3 | Codes TVA A/B/C/D/X avec taux corrects | POS v2.2 §II.5 | `crates/pos-core/src/vat.rs::VatCode` | `crates/pos-core/src/vat.rs::tests` |
| 4 | Ticket avec en-tête (nom, adresse, TVA) + TVA par ligne + données de contrôle + QR | POS v2.2 §III.3 | `crates/pos-print/src/receipt.rs` | `crates/pos-print/tests/receipt_matrix.rs` (FR/NL/DE, chaque type de ticket) |
| 5 | QR Code Model 2, ECC M, URL 38 caractères (version 3, 29×29 : le 25×25 ne peut pas contenir l'URL en mode octet) | POS v2.2 §6.5 | `crates/pos-fiscal/src/qr.rs` | `crates/pos-fiscal/src/qr.rs::tests::spec_example_url_needs_version_3_with_ecc_m` |
| 6 | Compteurs fiscaux strictement séquentiels, sans trou | POS v2.2 §II.6 | `crates/pos-fiscal/src/counter.rs::FiscalCounter::next` | `crates/pos-fiscal/src/counter.rs::tests::counter_is_strictly_sequential_with_no_gaps` |
| 7 | Allocation de compteur rollback sur échec FDM | POS v2.2 §II.6 | `crates/pos-engine/src/commands/signing.rs` (le numéro est lu sous le verrou par `peek`, sans avancer le compteur ; `FiscalCounter::next` n'est appelé que dans la transaction d'archivage, après la réponse du FDM) | `src-tauri/tests/spf_e2e_no_gap.rs::checkout_fdm_failure_does_not_advance_counter` |
| 8 | Tous les événements fiscaux ajoutés à l'archive locale, jamais supprimés | POS v2.2 §II.7 | `crates/pos-db/src/schema.rs` (table fiscal_events, déclencheurs SQLite qui refusent tout DELETE et UPDATE) | `crates/pos-fiscal/src/archive.rs::tests::{archived_event_cannot_be_deleted, archived_event_cannot_be_rewritten}` |
| 9 | Le POS bloque les transactions quand le FDM est injoignable | POS v2.2 §III.6 | `crates/pos-engine/src/commands/signing.rs` et `checkout.rs` (FDM non joignable : la vente est refusée, aucun mode hors-ligne) | `src-tauri/tests/spf_e2e_fdm_failures.rs::checkout_unreachable_fdm_blocks_the_sale_without_archiving` |
| 10 | Mode formation visuellement distinct, événements T non comptés normalement | POS v2.2 §II.4.3 | `ui/src/components/TrainingBanner.vue` + flag `is_training` dans les inputs | `src-tauri/tests/spf_e2e_basic.rs::m171_sale_in_training_uses_t_counter_and_keeps_n_at_zero` |
| 11 | Chaque réponse FDM est contrôlée avant archivage (écho de la requête, opération et label, shortSignature = SHA-1 de digitalSignature pour N) ; la signature ECDSA est conservée telle quelle pour vérification par l'administration | POS v2.2 §2.2.4 et §III.5 | `crates/pos-fiscal/src/response_check.rs::check_sign_result` | `crates/pos-fiscal/src/response_check.rs::tests`, `src-tauri/tests/spf_e2e_fdm_response_check.rs` |
| 12 | Un échec FDM connu (injoignable, refusé) ne laisse aucune trace : rien n'est archivé, le compteur n'avance pas, le numéro reste libre. Une réponse perdue laisse la requête stockée (`pending_signs`) ; elle est renvoyée à l'identique avant toute autre signature et archivée une seule fois, sous son premier numéro | POS v2.2 §III.7 | `crates/pos-engine/src/commands/signing.rs` (verrou unique, numéro lu sous le verrou, compteur avancé seulement après la réponse, renvoi octet pour octet) | `src-tauri/tests/spf_e2e_fdm_failures.rs::restoring_the_fdm_url_after_unreachable_lets_the_next_call_succeed` ; `src-tauri/tests/spf_e2e_signing_serialized.rs` (14 tests : courses et réponses perdues) |
| 13 | Requêtes envoyées au FDM en JSON canonique (clés triées, sans espaces) : mêmes données, mêmes octets, journal de requêtes comparable. Le FDM applique sa propre canonicalisation pour signer | POS v2.2 §III.5 | `crates/pos-fiscal/src/canonical.rs` | `crates/pos-fiscal/src/canonical.rs::tests::canonical_is_deterministic` |
| 14 | Les événements C portent un `fdmRef` vers l'original | POS v2.2 §II.4.6 | `crates/pos-engine/src/commands/copy.rs` | e2e `spf_e2e_basic::{m160_*, m161_*, m172_*}` + fixture UC_BasicSet M160/M161 |
| 15 | Événements I avec n° TVA client + cost-center optionnel | POS v2.2 §II.4.5 | `crates/pos-engine/src/commands/invoice.rs::do_sign_invoice_fiscal` (existe, `#[cfg(feature = "peppol")]` - **non compilé dans la build cert v1.0.0**, event I hors périmètre v1.0.0, prévu v1.1, cf. *DECISION_PEPPOL_AP_FR.md*) | e2e `spf_e2e_basic::m150_*` + fixture UC_BasicSet M150 (features par défaut, pas la build cert) |
| 16 | F-WAR ouverture tiroir émis à chaque kick tiroir | POS v2.2 §II.4.7 | `crates/pos-engine/src/commands/financial.rs::do_open_drawer_fiscal` | e2e `spf_e2e_basic::m131_*` + fixture UC_BasicSet M131 |
| 17 | S pointage entrée/sortie pour Dimona ONSS | POS v2.2 §II.4.8 | `crates/pos-engine/src/commands/social.rs::{do_work_in,do_work_out}` | e2e `spf_e2e_basic::{m140_*, m141_*}` + fixtures UC_BasicSet M140/M141 |
| 18 | Rapport Z atomique (Turnover + User) | Reports v1.3 §II.3 | `crates/pos-engine/src/commands/report.rs::do_generate_z_report` (mutation M184 combinée + tx unique) | e2e `spf_e2e_reports::m184_*` + `reports_fixtures.rs` |

## Manuel de certification §III.H - Ticket numérique par e-mail

| # | Exigence | Spec | Code | Test |
|---|---|---|---|---|
| 18b | Support du ticket choisi et refusé avant signature, encodé dans la requête signée | Manuel §III.H | `crates/pos-engine/src/commands/delivery.rs::{resolve, resolve_with}` | `delivery.rs::tests::*` (16 tests) + e2e `spf_e2e_digital::{email_without_setup_is_refused_before_signing, a_bad_address_is_refused_before_signing}` |
| 18c | Mise en file d'attente dans la même transaction que l'archive fiscale | Manuel §III.H (garantie de remise) | `delivery::enqueue_in_tx` dans la transaction d'archivage de `signing.rs` (appelée pour la vente, le remboursement et la copie) | e2e `spf_e2e_digital::a_failed_outbox_write_rolls_the_whole_sale_back` |
| 18d | Envoi immédiat borné, repli papier automatique de l'original | Manuel §III.H | `delivery::deliver`, `INLINE_SEND_TIMEOUT = 8s` | e2e `spf_e2e_digital::{digital_only_emails_without_printing, a_failed_digital_email_prints_the_original_once}` |
| 18e | Réessais avec recul exponentiel en tâche de fond | Manuel §III.H | `pos_mail::outbox::{DeliveryPolicy, run_once}`, câblé toutes les 30s (`commands::delivery::run_outbox_once`) | `outbox.rs::tests::backoff_follows_the_documented_schedule` |
| 18f | Original imprimé une seule fois (POS v2.2 §8.1) | POS v2.2 §8.1 | `crates/pos-db/src/queries/prints.rs`, `delivery::print_original_once` | `prints.rs::tests::a_print_is_recorded_once` + e2e `spf_e2e_digital::a_printed_original_can_not_be_printed_again` |
| 18g | Adresse purgée sous 72h, jamais exposée en clair dans l'UI | Manuel §III.M (données) + RGPD | `pos_db::queries::deliveries::{mark_sent, purge_recipients_older_than}` ; `DeliveryView` sans champ `recipient` | `deliveries.rs::tests::purge_nulls_old_recipients` |

Détail complet (34 tests e2e dans `spf_e2e_digital.rs`) : *voir TICKET_NUMERIQUE_FR.md* §12.

## Spec FDM v1.4 + Addendum 2.1

| # | Exigence | Spec | Code | Test |
|---|---|---|---|---|
| 19 | Le POS ne parle PAS directement à SPF Finances | FDM v1.4 §I.1 | (architectural - le FDM est le seul canal sortant ; aucun appel SPF Finances nulle part dans `pili-pos`) | grep vérifié : `grep -rni "finances\.fed\.be\|minfin\.fed\.be" crates/ src-tauri/ --exclude-dir=target` → 0 hit |
| 20 | La short signature FDM est affichée sur le ticket | FDM v1.4 §IV.2 | `crates/pos-print/src/receipt.rs::format_receipt` | `src-tauri/tests/spf_e2e_peripherals.rs::the_sale_receipt_carries_the_fdm_control_data` |
| 21 | Le statut FDM est remonté à l'UI | FDM v1.4 §V | `crates/pos-engine/src/state.rs` (mirror dans store Pinia) | E2E manuel (kill du mock FDM → voyant FDM rouge et voile) |

## Basic Set v1.8

Les 22 UC du Basic Set v1.8 sont chargés par les tests de fixtures
(`crates/pos-fiscal/tests/uc_fixtures.rs`). 18 ont un test automatisé ;
les 4 autres sont exclus et déclarés : M150, M122 et M132 hors
périmètre v1.0.0, M112A non applicable (le remboursement partiel est
toujours un ticket séparé, M112B). Voir `COUVERTURE_FIXTURES_FR.md`
pour la matrice pass/fail des fixtures ; les exclusions sont déclarées
dans `scripts/uc_scope.toml`.

## Reports v1.3

Les 5 UC (M180 à M184) ont des tests de fixtures, et les 8 tickets
d'entrée T1 à T8 sont rejoués. Voir `COUVERTURE_RAPPORTS_FR.md`. Tests
dans `crates/pos-fiscal/tests/reports_fixtures.rs` et
`src-tauri/tests/spf_e2e_report_fixtures.rs`.

## Advanced Set v1.6

Les 141 fixtures se chargent et se désérialisent proprement
(`crates/pos-fiscal/tests/advanced_set_fixtures.rs`) ; voir
`COUVERTURE_ADVANCED_SET_FR.md` pour le détail par UC. Sur les 58
identifiants de test, 5 ont un test dédié : UC223, UC230, UC4109 et
UC4119 comparent la requête construite par pili-pos à la fixture, UC201
contrôle les lignes de paiement et d'arrondi du scénario. Les 53 autres sont déclarés hors périmètre v1.0.0
(`scripts/uc_scope.toml`). UC223 ne couvre que les remises de portée
EVENT des commandes en ligne (`crates/pos-fiscal/tests/price_change_uc.rs`) ;
UC220 et UC224 n'ont pas de test de fixture.

## Remboursements & corrections

Voir `FLUX_REMBOURSEMENT_FR.md`. M111 (remboursement complet),
M112B (remboursement partiel comme événement séparé - le pattern
déclaré, cf. e2e `spf_e2e_basic::m112b_*`) sont couverts dans
`crates/pos-engine/src/commands/refund.rs`. Chaque ligne remboursée est
valorisée au prix de la ligne de vente d'origine qu'elle désigne ; si
le produit a été vendu à plusieurs prix, la ligne doit être désignée
(code `refund_line_ambiguous`). UC4122 (note de crédit) : reporté avec
l'event I, hors périmètre v1.0.0. M132 (correction de paiement) :
reporté, hors périmètre, non déclaré.

## Peppol B2B (obligation parallèle, hors SCE-2.0)

Voir `DECISION_PEPPOL_AP_FR.md`. Sérialiseur UBL + trait AccessPoint
implémentés dans `crates/pos-peppol` ; IPC dans
`crates/pos-engine/src/commands/invoice.rs`. Ce code est derrière le feature
flag `peppol`, absent de la build cert v1.0.0 (event I hors périmètre,
prévu v1.1).

## Hygiène de build

| Préoccupation | Preuve |
|---|---|
| Périmètre du binaire cert | `cargo build --release --no-default-features --features cert` : commandes en ligne présentes (activées par installation, `COMMANDES_EN_LIGNE_FR.md`), facturation B2B absente (`nm pili-pos | grep -c pos_peppol` → 0) ; vérifié par `scripts/build_and_hash.sh` |
| Tous les tests verts | `cargo test --workspace` et la build cert : 0 échec (totaux dans RAPPORT_TESTS_BASELINE_FR.md) |
| Lint propre | `cargo clippy --workspace -- -D warnings` (à exécuter avant soumission) |
| Format cohérent | `cargo fmt --all` |
| Build reproductible | `Cargo.lock` committé ; Rust 1.99.0 épinglé dans `rust-toolchain.toml` |
