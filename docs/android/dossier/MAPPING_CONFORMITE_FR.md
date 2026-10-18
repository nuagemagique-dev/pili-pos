<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/MAPPING_CONFORMITE_FR.md @ 86088e4 -->
# Mapping de conformité - pili-pos-android SCE 2.0

**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)

Mappe chaque exigence des spécifications SPF Finances vers le code qui
l'implémente et le test qui prouve son fonctionnement. C'est le cœur
du dossier cert : c'est ce que le relecteur utilisera pour vérifier
la couverture de chaque règle obligatoire.

Le moteur fiscal est le même code source que sous Linux (étiquette de
gel `linux-v1.0.0-rc2`, `CADRE_DEMANDE_FR.md` §4) : les lignes du moteur
(1 à 20) sont celles du dossier Linux, avec les mêmes fichiers et les
mêmes tests. Seules changent les lignes qui touchent l'interface, la
plateforme et la construction (10, 19, 21, hygiène de build) ; les
exigences que l'arrêté impose au support Android sont aux lignes 22 à 26.

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
| 7 | Allocation de compteur rollback sur échec FDM | POS v2.2 §II.6 | `crates/pos-engine/src/commands/signing.rs` (le numéro est lu sous le verrou par `peek`, sans avancer le compteur ; `FiscalCounter::next` n'est appelé que dans la transaction d'archivage, après la réponse du FDM) | `crates/pos-engine/tests/spf_e2e_no_gap.rs::checkout_fdm_failure_does_not_advance_counter` |
| 8 | Tous les événements fiscaux ajoutés à l'archive locale, jamais supprimés | POS v2.2 §II.7 | `crates/pos-db/src/schema.rs` (table fiscal_events, déclencheurs SQLite qui refusent tout DELETE et UPDATE) | `crates/pos-fiscal/src/archive.rs::tests::{archived_event_cannot_be_deleted, archived_event_cannot_be_rewritten}` |
| 9 | Le POS bloque les transactions quand le FDM est injoignable | POS v2.2 §III.6 | `crates/pos-engine/src/commands/signing.rs` et `checkout.rs` (FDM non joignable : la vente est refusée, aucun mode hors-ligne) | `crates/pos-engine/tests/spf_e2e_fdm_failures.rs::checkout_unreachable_fdm_blocks_the_sale_without_archiving` |
| 10 | Mode formation visuellement distinct, événements T non comptés normalement | POS v2.2 §II.4.3 | `android/app/src/main/kotlin/dev/nuagemagique/pilipos/ui/components/TrainingBanner.kt` et `TrainingWatermark.kt` (même dossier) + flag `is_training` dans les inputs | `crates/pos-engine/tests/spf_e2e_basic.rs::m171_sale_in_training_uses_t_counter_and_keeps_n_at_zero` ; affichage : `android/app/src/test/kotlin/dev/nuagemagique/pilipos/ui/components/TrainingBannerTest.kt` |
| 11 | Chaque réponse FDM est contrôlée avant archivage (écho de la requête, opération et label, shortSignature = SHA-1 de digitalSignature pour N) ; la signature ECDSA est conservée telle quelle pour vérification par l'administration | POS v2.2 §2.2.4 et §III.5 | `crates/pos-fiscal/src/response_check.rs::check_sign_result` | `crates/pos-fiscal/src/response_check.rs::tests`, `crates/pos-engine/tests/spf_e2e_fdm_response_check.rs` |
| 12 | Un échec FDM connu (injoignable, refusé) ne laisse aucune trace : rien n'est archivé, le compteur n'avance pas, le numéro reste libre. Une réponse perdue laisse la requête stockée (`pending_signs`) ; elle est renvoyée à l'identique avant toute autre signature et archivée une seule fois, sous son premier numéro | POS v2.2 §III.7 | `crates/pos-engine/src/commands/signing.rs` (verrou unique, numéro lu sous le verrou, compteur avancé seulement après la réponse, renvoi octet pour octet) | `crates/pos-engine/tests/spf_e2e_fdm_failures.rs::restoring_the_fdm_url_after_unreachable_lets_the_next_call_succeed` ; `crates/pos-engine/tests/spf_e2e_signing_serialized.rs` (14 tests : courses et réponses perdues) |
| 13 | Requêtes envoyées au FDM en JSON canonique (clés triées, sans espaces) : mêmes données, mêmes octets, journal de requêtes comparable. Le FDM applique sa propre canonicalisation pour signer | POS v2.2 §III.5 | `crates/pos-fiscal/src/canonical.rs` | `crates/pos-fiscal/src/canonical.rs::tests::canonical_is_deterministic` |
| 14 | Les événements C portent un `fdmRef` vers l'original | POS v2.2 §II.4.6 | `crates/pos-engine/src/commands/copy.rs` | e2e `spf_e2e_basic::{m160_*, m161_*, m172_*}` + fixture UC_BasicSet M160/M161 |
| 15 | Événements I (facture) avec n° TVA client, centre de coût CUSTOMER et références des ventes (`fdmRefs`) | POS v2.2 §II.4.5 | `crates/pos-engine/src/commands/invoice.rs::do_sign_invoice_fiscal` (numéro `<année><séquence>` sans trou, une vente une facture) ; impression `crates/pos-print/src/invoice.rs` ; cf. *FACTURATION_PEPPOL_FR.md* | `crates/pos-engine/tests/spf_e2e_fixtures.rs::m150_invoice`, `invoice_rules.rs`, `invoice_advanced.rs` (UC4121, UC4122) + fixture UC_BasicSet M150 |
| 16 | F-WAR ouverture tiroir émis à chaque kick tiroir | POS v2.2 §II.4.7 | `crates/pos-engine/src/commands/financial.rs::do_open_drawer_fiscal` | e2e `spf_e2e_basic::m131_*` + fixture UC_BasicSet M131 |
| 17 | S pointage entrée/sortie pour Dimona ONSS | POS v2.2 §II.4.8 | `crates/pos-engine/src/commands/social.rs::{do_work_in,do_work_out,do_clock_in_employee,do_clock_out_employee}` | e2e `spf_e2e_basic::{m140_*, m141_*}` + fixtures UC_BasicSet M140/M141 |
| 17b | S pointage par badge : la routine d'inscription est libre pourvu que l'event S soit correct ; le POS relie son identification interne au NISS, jamais imprimé (option désactivée par défaut, badge lié par un Gérant avec son PIN, empreinte HMAC seule conservée, Gérant toujours par PIN) | POS v2.2 §1.9 et §6.4 ; Manuel de certification §II.E et §II.C ; AM du 29 avril 2024 art. 1, 9° et art. 9 | `crates/pos-engine/src/commands/badges.rs::{do_badge_tap,do_link_badge,do_revoke_badge}` (signature : `crates/pos-engine/src/commands/social.rs::sign_work_event`) | `badges.rs::{a_scan_while_a_cashier_is_logged_in_clocks_the_badge_holder_in_and_keeps_the_session, an_fdm_failure_on_a_badge_scan_archives_nothing, in_training_a_badge_scan_is_refused_and_signs_nothing, a_double_tap_never_clocks_in_and_out, a_manager_badge_opens_no_session_and_manager_commands_stay_refused}` |
| 17c | RGPD : le numéro du badge n'est jamais conservé (empreinte HMAC-SHA256 avec une clé qui reste sur l'appareil, hors base, sauvegardes et exports), pas de biométrie, finalité limitée à la présence et à la connexion, badge facultatif | RGPD art. 5 et 32 ; POS v2.2 §6.4 | `crates/pos-engine/src/badge_key.rs::{load_or_create,hash_token}` ; migration 70 `staff-badges` (`crates/pos-db/src/schema_badges.rs`) | `badges.rs::the_raw_badge_number_is_nowhere_in_the_database_file` ; `PROFIL_ENTREPRISE_FR.md` §6 |
| 18 | Rapport Z atomique (Turnover + User) | Reports v1.3 §II.3 | `crates/pos-engine/src/commands/report.rs::do_generate_z_report` (mutation M184 combinée + tx unique) | e2e `spf_e2e_reports::m184_*` + `reports_fixtures.rs` |

## Manuel de certification §III.H - Ticket numérique par e-mail

| # | Exigence | Spec | Code | Test |
|---|---|---|---|---|
| 18b | Support du ticket choisi et refusé avant signature, encodé dans la requête signée | Manuel §III.H | `crates/pos-engine/src/commands/delivery.rs::{resolve, resolve_with}` | `delivery.rs::tests::*` (16 tests) + e2e `spf_e2e_digital::{email_without_setup_is_refused_before_signing, a_bad_address_is_refused_before_signing}` |
| 18c | Mise en file d'attente dans la même transaction que l'archive fiscale | Manuel §III.H (garantie de remise) | `delivery::enqueue_in_tx` dans la transaction d'archivage de `signing.rs` (appelée pour la vente, le remboursement et la copie) | e2e `spf_e2e_digital::a_failed_outbox_write_rolls_the_whole_sale_back` |
| 18d | Envoi immédiat borné, repli papier automatique de l'original | Manuel §III.H | `delivery::deliver`, `INLINE_SEND_TIMEOUT = 8s` | e2e `spf_e2e_digital::{digital_only_emails_without_printing, a_failed_digital_email_prints_the_original_once}` |
| 18e | Réessais avec recul exponentiel en tâche de fond | Manuel §III.H | `pos_mail::outbox::{DeliveryPolicy, run_once}`, câblé toutes les 30s (`commands::delivery::run_outbox_once`, démarré dans `crates/pos-engine/src/background.rs`) | `outbox.rs::tests::backoff_follows_the_documented_schedule` |
| 18f | Original imprimé une seule fois (POS v2.2 §8.1) | POS v2.2 §8.1 | `crates/pos-db/src/queries/prints.rs`, `delivery::print_original_once` | `prints.rs::tests::a_print_is_recorded_once` + e2e `spf_e2e_digital::a_printed_original_can_not_be_printed_again` |
| 18g | Adresse purgée sous 72h, jamais exposée en clair dans l'UI | Manuel §III.M (données) + RGPD | `pos_db::queries::deliveries::{mark_sent, purge_recipients_older_than}` ; `DeliveryView` sans champ `recipient` | `deliveries.rs::tests::purge_nulls_old_recipients` |

Détail complet (34 tests e2e dans `spf_e2e_digital.rs`) : *voir TICKET_NUMERIQUE_FR.md* §12.

## Spec FDM v1.4 + Addendum 2.1

| # | Exigence | Spec | Code | Test |
|---|---|---|---|---|
| 19 | Le POS ne parle PAS directement à SPF Finances | FDM v1.4 §I.1 | (architectural - le FDM est le seul canal sortant ; aucun appel SPF Finances nulle part dans `pili-pos-android`) | grep vérifié : `grep -rni "finances\.fed\.be\|minfin\.fed\.be" crates/ src-tauri/ android/ --exclude-dir=target` → 0 hit |
| 20 | La short signature FDM est affichée sur le ticket | FDM v1.4 §IV.2 | `crates/pos-print/src/receipt.rs::format_receipt` | `crates/pos-engine/tests/spf_e2e_peripherals.rs::the_sale_receipt_carries_the_fdm_control_data` |
| 21 | Le statut FDM est remonté à l'UI | FDM v1.4 §V | `crates/pos-engine/src/commands/fdm_monitor.rs` (état classé par le moteur) et `android/app/src/main/kotlin/dev/nuagemagique/pilipos/ui/fdm/FdmStore.kt` (relais vers l'écran) | `android/app/src/test/kotlin/dev/nuagemagique/pilipos/ui/fdm/FdmStoreTest.kt` et `FdmBlockingVeilTest.kt` (même dossier) ; E2E sur émulateur : À effectuer (EV-10 : parcours sur les deux profils du D3 Pro) |

## Basic Set v1.8

Les 22 UC du Basic Set v1.8 sont chargés par les tests de fixtures
(`crates/pos-fiscal/tests/uc_fixtures.rs`). 21 ont un test automatisé ;
le dernier est exclu et déclaré : M112A non applicable (le remboursement partiel est
toujours un ticket séparé, M112B). Voir `COUVERTURE_FIXTURES_FR.md`
pour la matrice pass/fail des fixtures ; les exclusions sont déclarées
dans `scripts/uc_scope.toml`.

## Reports v1.3

Les 5 UC (M180 à M184) ont des tests de fixtures, et les 8 tickets
d'entrée T1 à T8 sont rejoués. Voir `COUVERTURE_RAPPORTS_FR.md`. Tests
dans `crates/pos-fiscal/tests/reports_fixtures.rs` et
`crates/pos-engine/tests/spf_e2e_report_fixtures.rs`.

## Advanced Set v1.6

Les 141 fixtures se chargent et se désérialisent proprement
(`crates/pos-fiscal/tests/advanced_set_fixtures.rs`) ; voir
`COUVERTURE_ADVANCED_SET_FR.md` pour le détail par UC. Sur les 58
identifiants de test, 13 ont un test dédié : UC223, UC225A et UC225B
(un même rejeu), UC229B, UC230, UC260, UC261, UC262, UC4109, UC4119,
UC4121 et UC4122 comparent la requête construite par pili-pos à la
fixture, UC201 contrôle les lignes de paiement et d'arrondi du
scénario. Les 45 autres sont déclarés hors périmètre v1.0.0 (fonctions
non offertes) (`scripts/uc_scope.toml`). UC223
couvre la remise de portée EVENT des commandes en ligne
(`crates/pos-fiscal/tests/price_change_uc.rs`) et celle de la caisse
(`uc223_event_discount_at_the_till`) ; UC220 et UC224 n'ont pas de test
de fixture (la caisse n'a pas de remise partielle sur une ligne).

## Rejeu sur Android

Les UC des trois jeux sont rejoués contre le moteur d'`android/rust` sur
l'émulateur du D3 Pro, requête et réponse JSON par UC (résultats :
`RESULTATS_TESTS_UC_FR.md`) : À effectuer (EV-06 : résultats des UC sur
l'émulateur Android). Le journal des requêtes FDM est comparé à celui de
Linux pour les mêmes scénarios : À effectuer (EV-07 : comparaison des
journaux Android et Linux).

## Bons cadeaux, fonctions du propriétaire et allergènes

| # | Exigence | Spec | Code | Test |
|---|---|---|---|---|
| G1 | La vente d'un bon d'achat à usage multiple (MPV) est une ligne de code T.V.A. X (hors champ) : aucune T.V.A. à la vente, elle est due sur les marchandises achetées avec le bon | POS v2.2, utilisation du code T.V.A. X ; UC260 | `crates/pos-engine/src/commands/gift_vouchers.rs::do_add_gift_voucher_line` (produit système, département `GV`) | `crates/pos-engine/tests/spf_e2e_gift_vouchers.rs::uc260_mpv_sale` |
| G2 | L'utilisation d'un bon est une ligne de paiement de type `VOUCHER_STORE` (« Gift voucher », saisie `MANUAL`), au montant exact, jamais arrondi | POS v2.2, types de paiement ; UC261, UC262 | `crates/pos-fiscal/src/payment.rs` (règles de paiement), `crates/pos-engine/src/commands/checkout.rs::do_checkout_split` | `crates/pos-engine/tests/spf_e2e_gift_vouchers.rs::{uc261_mpv_use,uc262_mpv_use}` |
| G3 | Les contrôles d'un bon (existence, mode, expiration, solde) sont faits sous le verrou de signature, avant l'envoi au module fiscal ; le débit s'écrit dans l'opération d'archive qui suit la signature | POS v2.2 §II.6 (séquentialité), UC261 | `crates/pos-engine/src/commands/gift_vouchers.rs`, `crates/pos-engine/src/commands/signing.rs::apply` | `crates/pos-engine/tests/gift_vouchers.rs` (38 scénarios) |
| G4 | Une vente qui vend un bon ne se facture pas (catégorie O, EN 16931) ; la correction d'un moyen de paiement ne touche jamais une ligne `VOUCHER_STORE` | EN 16931 BR-O-02, BR-O-11 à BR-O-14 ; M132 | `crates/pos-engine/src/commands/invoice.rs`, `crates/pos-engine/src/commands/payment_correction.rs` | `crates/pos-engine/tests/lane_interactions.rs` |
| G5 | Les alertes du propriétaire ne signent rien, ne lisent aucun ticket ni rapport et ne bloquent jamais une vente ou un Z | POS v2.2 §III.6 (le blocage ne vient que du module fiscal) | `crates/pos-engine/src/alerts/` | `crates/pos-engine/tests/owner_alerts.rs` |
| G6 | Le temps d'une ligne (service en plusieurs temps) n'entre dans aucune demande signée ni ticket de T.V.A. ; les tickets de cuisine et de bar ne portent aucun montant | POS v2.2 §8.1 et §8.2 (consolidation des imprimés), FAQ Q210 b | `crates/pos-engine/src/commands/courses.rs`, `crates/pos-print/` | `crates/pos-engine/tests/courses.rs` |
| G7 | Les allergènes sont une information d'affichage et d'impression : aucun allergène dans une demande signée | Règlement (UE) n° 1169/2011, annexe II (information) | `crates/pos-engine/src/commands/allergens.rs` | `crates/pos-engine/tests/allergens.rs` |

## Remboursements & corrections

Voir `FLUX_REMBOURSEMENT_FR.md`. M111 (remboursement complet),
M112B (remboursement partiel comme événement séparé - le pattern
déclaré, cf. e2e `spf_e2e_basic::m112b_*`) sont couverts dans
`crates/pos-engine/src/commands/refund.rs`. Chaque ligne remboursée est
valorisée au prix de la ligne de vente d'origine qu'elle désigne ; si
le produit a été vendu à plusieurs prix, la ligne doit être désignée
(code `refund_line_ambiguous`). UC4122 (facture d'un remboursement) :
couvert, voir *FACTURATION_PEPPOL_FR.md* et `FLUX_REMBOURSEMENT_FR.md` §3. M132 (correction de paiement) :
livré pour un moyen de paiement saisi à la main (Gérant + PIN, événement F
`signPaymentCorrection`, une correction par vente, journée en cours),
couvert par `m132_payment_correction` et `crates/pos-engine/tests/payment_correction.rs`.

## Peppol B2B (obligation parallèle, hors SCE-2.0)

Voir `DECISION_PEPPOL_AP_FR.md` et `FACTURATION_PEPPOL_FR.md`.
Sérialiseur UBL + trait AccessPoint implémentés dans `crates/pos-peppol` ;
commandes dans `crates/pos-engine/src/commands/invoice.rs`, file d'envoi
dans `crates/pos-engine/src/commands/peppol_queue.rs`, enveloppes IPC dans
`src-tauri/src/ipc/invoice.rs`. Ce code est compilé dans tous les
binaires (aucune fonction Cargo) ; l'envoi ne fonctionne que dans une
installation qui a une section `peppol`.

## Exigences de l'arrêté ministériel propres à la plateforme Android

Ces lignes n'ont pas d'équivalent dans le moteur : elles portent sur le
support Android. Le texte de référence est l'arrêté ministériel du
29 avril 2024 (`docs/legislation/20240429_AM_aspects_techniques_certification_FR.md`).

| # | Exigence | Spec | Où c'est décrit | Code et test |
|---|---|---|---|---|
| 22 | Art. 16 et 102 : l'opérateur n'a qu'un accès en lecture ; le SPF Finances copie les données sur simple demande, par un port externe de la caisse. Sur Android : l'export du Gérant vers une clé USB ou une carte SD, seule voie d'accès, sans compte de contrôle | AM Art. 16 et 102 | `SECURITE_BD_FR.md` §8 (droits §8.1, export §8.2, contrôleur §8.3, aucune autre voie §8.4) | `android/app/src/main/kotlin/dev/nuagemagique/pilipos/platform/export/Exporter.kt` ; `android/app/src/test/kotlin/dev/nuagemagique/pilipos/platform/export/ExporterTest.kt` ; déroulé sur les AVD : À effectuer (EV-10), vraie clé et vraie carte : À effectuer (EV-15) |
| 23 | Art. 47 : numéro de version unique, lisible, visible à l'écran d'ouverture de session et envoyé au FDM à chaque communication | AM Art. 47 | `CADRE_DEMANDE_FR.md` §3 (posSwVersion pili-pos-android/1.0.0, écrans de connexion et À propos) | `android/rust/pos-ffi/Cargo.toml` (source unique de la version) ; `android/app/src/test/kotlin/dev/nuagemagique/pilipos/feature/login/LoginContentTest.kt` (version affichée) ; envoi au FDM : tests du moteur, lignes 1 et 20 |
| 24 | Art. 112 : garantie de fabrication, l'exemplaire livré est identique à l'exemplaire certifié ; hash SHA-1 au moins | AM Art. 112 | `GARANTIE_MAJ_LANGUES_FR.md` §1 (build reproductible §1.1, SHA-1 de l'APK signé §1.2) | `scripts/android/check_repro.sh` ; `android/app/src/main/kotlin/dev/nuagemagique/pilipos/platform/about/Hashes.kt` ; `android/app/src/test/kotlin/dev/nuagemagique/pilipos/platform/about/HasherTest.kt` ; valeurs : À effectuer (EV-01, EV-02, EV-05) |
| 25 | FAQ producteurs Q505 : identifiant unique de l'appareil, adresse MAC ou, réponse b, identifiant matériel comme le numéro de série | FAQ Q505 | `PLATEFORME_ANDROID_FR.md` §4 (source, lecture à chaque ouverture, blocage en cas de différence) | `android/app/src/main/kotlin/dev/nuagemagique/pilipos/provisioning/DeviceIdReader.kt` ; `android/app/src/test/kotlin/dev/nuagemagique/pilipos/provisioning/DeviceIdReaderTest.kt` ; format réel : À effectuer (EV-15), valeur de la machine de test : `EMULATOR37X2X12X0` (EV-16) |
| 26 | Art. 113 : toute modification d'un système certifié suit la procédure de mise à jour ; l'APK n'est installé que par l'outil de mise à jour qui n'accepte que l'artefact déclaré | AM Art. 113 | `GARANTIE_MAJ_LANGUES_FR.md` §2 | `android/app/src/main/kotlin/dev/nuagemagique/pilipos/platform/update/UpdateVerifier.kt` ; `android/app/src/test/kotlin/dev/nuagemagique/pilipos/platform/update/UpdateVerifierTest.kt` |

## Hygiène de build

| Préoccupation | Preuve |
|---|---|
| Périmètre du moteur cert | `android/rust/pos-ffi/Cargo.toml` compile `pos-engine` avec la seule fonction `cert` : commandes en ligne présentes (activées par installation, `COMMANDES_EN_LIGNE_FR.md`), facturation B2B (événement I) et file d'envoi Peppol présentes, sans fonction Cargo |
| Tous les tests verts | moteur (`cargo test --workspace` et build cert), adaptateur `android/rust` et application Kotlin : 0 échec ; totaux au commit de release : À effectuer (EV-17), détail dans `RAPPORT_TESTS_BASELINE_FR.md` |
| Forme de l'APK | `scripts/android/check_apk.sh` : alignement 16 Ko, une architecture, `libpos_ffi.so` présente |
| Build reproductible | `Cargo.lock` et `android/rust/Cargo.lock` commités ; Rust 1.99.0 épinglé dans `rust-toolchain.toml` ; conteneur épinglé, `scripts/android/check_repro.sh` ; rapport : À effectuer (EV-05) |
