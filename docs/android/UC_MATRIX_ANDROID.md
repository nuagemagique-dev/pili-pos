# Matrice de traçabilité des tests SPF, version Android

Ce document indique, pour chaque identifiant de test décrit par le SPF Finances (Basic Set, Reports, Advanced Set, checklist du manuel de certification, FAQ producteurs), comment le port Android de la caisse le couvre : par un scénario de bout en bout rejoué sur l'appareil et comparé au moteur de référence, par les tests du moteur (exécutés sur l'appareil, ou sur la machine hôte en attendant leur passage sur l'appareil), ou par une exclusion déclarée.

Généré automatiquement par `scripts/android/uc_matrix_android.py` à partir de la matrice Linux (`scripts/uc_matrix.py`, `scripts/uc_scope.toml`), des scénarios (`android/app/src/androidTest/assets/uc`), des résultats (`docs/android/uc/results.json`) et des exclusions propres à Android (`scripts/android/uc_scope_android.toml`). Une réexécution sans changement produit un fichier identique, à la ligne de date près.

Date de génération : 2026-10-18

Les 47 identifiants requis doivent chacun avoir un scénario Android au verdict PASS ; tant que ce n'est pas le cas, leur statut est En attente.

Légende du statut : Couvert (scénario Android) (un scénario liste l'identifiant et son résultat est PASS), Couvert (tests moteur exécutés sur l'appareil) (des tests Rust portent l'identifiant et au moins l'un d'eux a passé lors du dernier passage sur l'appareil), Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire) (des tests Rust portent l'identifiant, mais aucun n'existait lors du dernier passage sur l'appareil : ils ont passé sur la machine hôte seulement), En attente (identifiant requis sans scénario au verdict PASS), Exclu sur Android (comportement d'écran couvert sur Linux par un test d'interface seulement, prouvé sur Android autrement ; raison en fin de document), Hors périmètre, Non applicable et Couvert par X (exclusions de la matrice Linux reprises à l'identique), MANQUANT (aucune couverture ni exclusion).

Passage sur l'appareil : le dernier passage des tests du moteur sur l'émulateur date du 2026-10-10 (`docs/android/uc/device_engine_tests.md`, commit `778e9968`). Un test qui n'existait pas encore à ce commit a passé sur la machine hôte seulement : son exécution sur l'appareil reste à faire (`scripts/android/device_engine_tests.sh`). Un identifiant dont aucun test n'existait à ce commit a le statut Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire) ; pour un identifiant déjà couvert sur l'appareil, les tests ajoutés depuis sont listés à part (« sur l'hôte seulement »).

## Synthèse

| Statut | Nombre |
|---|---|
| Couvert (scénario Android) | 47 |
| Couvert (tests moteur exécutés sur l'appareil) | 27 |
| Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire) | 8 |
| En attente | 0 |
| Exclu sur Android | 24 |
| Hors périmètre | 48 |
| Non applicable | 1 |
| Couvert par un autre identifiant | 0 |
| MANQUANT | 0 |
| Total | 155 |

## Basic Set

| Id | Libellé | Scénario Android | Résultat Android | Tests moteur | Statut |
|---|---|---|---|---|---|
| M110 | Vente directe (ticket de caisse TVA) | `10_basic/010_M110.json` | PASS | `m110_sale`, `m110_sign_sale_archives_n_event_and_closes_order`, `m110_sign_sale_response_shape` | Couvert (scénario Android) |
| M111 | Remboursement total d'une vente | `10_basic/020_M111.json` | PASS | `m111_full_refund`, `m111_full_refund_archives_negative_n_with_fdm_ref`, `vi_refund_m111_the_refund_ticket_shows_the_sale_it_reverses` | Couvert (scénario Android) |
| M112A | Remboursement partiel combiné à une nouvelle vente | - | - | - | Non applicable |
| M112B | Remboursement partiel en ticket séparé | `10_basic/030_M112B.json` | PASS | `m112b_partial_refund_as_a_separate_ticket`, `m112b_partial_refund_is_separate_event_chained_to_original` | Couvert (scénario Android) |
| M121 | Commande en gestion de table | `10_basic/040_M121.json` | PASS | `m121_sign_order_archives_p_event_and_keeps_order_open`, `m121_sign_order_response_omits_n_only_fields`, `m121_table_order` | Couvert (scénario Android) |
| M122 | Changement de centre de coût (transfert de table) | - | - | `m122_cost_center_change` | Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire) |
| M123 | Addition provisoire (pre-bill) | `10_basic/050_M123.json` | PASS | `m123_pre_bill`, `m123_sign_pre_bill_archives_p_event_and_keeps_order_open`, `vi_proforma_m123_the_pre_bill_says_addition_provisoire_without_vat` | Couvert (scénario Android) |
| M130 | Mouvement de caisse (argent entrant/sortant) | `10_basic/060_M130.json` | PASS | `m130_money_in`, `m130_money_in_out_archives_f_event` | Couvert (scénario Android) |
| M131 | Ouverture du tiroir-caisse | `10_basic/070_M131.json` | PASS | `m131_drawer_open`, `m131_drawer_open_archives_f_event` | Couvert (scénario Android) |
| M132 | Correction de paiement | - | - | `m132_payment_correction` | Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire) |
| M133 | Déclaration de fond de caisse (comptage) | `10_basic/080_M133.json` | PASS | `m133_drawer_declaration` | Couvert (scénario Android) |
| M140 | Entrée en service de l'utilisateur | `10_basic/090_M140.json` | PASS | `m140_work_in`, `m140_work_in_archives_s_event` | Couvert (scénario Android) |
| M141 | Sortie de service de l'utilisateur | `10_basic/100_M141.json` | PASS | `m141_work_out`, `m141_work_out_archives_s_event` | Couvert (scénario Android) |
| M150 | Facturation | - | - | `m150_invoice_fdm_failure_does_not_advance_i_counter`, `m150_sign_invoice_archives_i_event_referencing_the_sale` ; sur l'hôte seulement : `m150_invoice` | Couvert (tests moteur exécutés sur l'appareil) |
| M160 | Copie d'un ticket de vente | `10_basic/110_M160.json` | PASS | `m160_copy_of_a_sale`, `m160_copy_of_sale_archives_c_event_with_ref_to_original`, `vi_copy_m160_a_copy_reproduces_the_original_sale` | Couvert (scénario Android) |
| M161 | Copie d'une commande | `10_basic/120_M161.json` | PASS | `m161_copy_of_an_order`, `m161_copy_of_order_archives_c_event_with_ref_to_p_event`, `vi_copy_m161_a_copy_of_a_pre_bill_is_not_a_vat_ticket` | Couvert (scénario Android) |
| M170 | Entrée en service en mode formation | `20_training/010_M170.json` | PASS | `m170_work_in_in_training`, `m170_work_in_training_does_not_advance_normal_s_counter` | Couvert (scénario Android) |
| M171 | Vente en mode formation | `20_training/020_M171.json` | PASS | `m171_sale_in_training`, `m171_sale_in_training_uses_t_counter_and_keeps_n_at_zero`, `m171_training_sale_has_no_vat_calc` | Couvert (scénario Android) |
| M171B | Vente en mode formation (style de requête @skip) | `20_training/030_M171B.json` | PASS | `m171b_sale_in_training_with_skipped_fields` | Couvert (scénario Android) |
| M172 | Copie en mode formation | `20_training/040_M172.json` | PASS | `m172_copy_in_training`, `m172_copy_in_training_mode_archives_c_event` | Couvert (scénario Android) |
| Q100 | Requête de statut du FDM | `10_basic/130_Q100.json` | PASS | `q100_status_response_shape` | Couvert (scénario Android) |
| Q110 | Requête de statut, messages multilingues | `10_basic/140_Q110.json` | PASS | `q110_multi_lang_status_response_shape`, `status_multi_lang_parses_q110_envelope` | Couvert (scénario Android) |

## Reports

| Id | Libellé | Scénario Android | Résultat Android | Tests moteur | Statut |
|---|---|---|---|---|---|
| M180 | Rapport X du chiffre d'affaires | `30_reports/090_M180.json` | PASS | `m180_generate_x_report_archives_r_event_without_resetting_counter`, `m180_the_x_report_is_not_numbered`, `m180_turnover_x`, `m180_turnover_x_of_the_scenario_day` +1 | Couvert (scénario Android) |
| M181 | Rapport Z du chiffre d'affaires | `30_reports/100_M181.json` | PASS | `m181_the_z_report_prints_the_article_29_content`, `m181_turnover_z_has_the_same_totals`, `m181_turnover_z_response_parses` | Couvert (scénario Android) |
| M182 | Rapport X des utilisateurs | `30_reports/110_M182.json` | PASS | `m182_generate_user_x_report_archives_r_event_with_user_block`, `m182_user_x`, `m182_user_x_of_the_scenario_day`, `m182_user_x_response_parses` | Couvert (scénario Android) |
| M183 | Rapport Z des utilisateurs | `30_reports/120_M183.json` | PASS | `m183_user_z_has_the_same_users`, `m183_user_z_response_parses` | Couvert (scénario Android) |
| M184 | Rapport Z combiné (chiffre d'affaires + utilisateurs) | `30_reports/130_M184.json` | PASS | `m184_combined_z`, `m184_combined_z_of_the_scenario_day`, `m184_combined_z_response_parses_and_counters_increment`, `m184_generate_z_report_signs_both_reports_in_one_operation` | Couvert (scénario Android) |
| T1 | Rejeu d'entrée T1 (signOrder) | `30_reports/010_T1.json` | PASS | `t1_table_1_order` | Couvert (scénario Android) |
| T2 | Rejeu d'entrée T2 (signSale) | `30_reports/020_T2.json` | PASS | `t2_table_1_paid_cash`, `t3_the_t2_sale_refunded` | Couvert (scénario Android) |
| T3 | Rejeu d'entrée T3 (signSale, remboursement) | `30_reports/030_T3.json` | PASS | `t3_the_t2_sale_refunded` | Couvert (scénario Android) |
| T4 | Rejeu d'entrée T4 (signSale) | `30_reports/040_T4.json` | PASS | `t4_takeaway_paid_by_card` | Couvert (scénario Android) |
| T5 | Rejeu d'entrée T5 (signOrder) | `30_reports/050_T5.json` | PASS | `t5_table_10_first_round` | Couvert (scénario Android) |
| T6 | Rejeu d'entrée T6 (signOrder) | `30_reports/060_T6.json` | PASS | `t6_table_10_second_round_with_a_free_price` | Couvert (scénario Android) |
| T7 | Rejeu d'entrée T7 (signPreBill) | `30_reports/070_T7.json` | PASS | `t7_table_10_pre_bill` | Couvert (scénario Android) |
| T8 | Rejeu d'entrée T8 (signSale) | `30_reports/080_T8.json` | PASS | `t8_table_10_paid_cash` | Couvert (scénario Android) |

## Advanced Set

| Id | Libellé | Scénario Android | Résultat Android | Tests moteur | Statut |
|---|---|---|---|---|---|
| UC200 | Vente directe d'un produit composite | - | - | - | Hors périmètre |
| UC201 | Vente avec arrondi du paiement | `40_advanced/010_UC201.json` | PASS | `uc201_cash_rounding_is_its_own_payment_line`, `vi_n_uc201_the_ticket_shows_the_sale_total_and_the_rounding_apart` | Couvert (scénario Android) |
| UC220 | Modification de prix sur une ligne de vente | - | - | - | Hors périmètre |
| UC221A | Modification de prix sur une ligne (cas A) | - | - | - | Hors périmètre |
| UC221B | Modification de prix sur une ligne (cas B) | - | - | - | Hors périmètre |
| UC222 | Modification de prix sur un produit composite | - | - | - | Hors périmètre |
| UC223 | Modification de prix au niveau de l'event de vente | `40_advanced/020_UC223.json` | PASS | `uc223_event_discount_at_the_till`, `uc223_event_discount_matches_the_published_transaction` | Couvert (scénario Android) |
| UC224 | Modifications de prix multiples sur une vente | - | - | - | Hors périmètre |
| UC225A | Modification de prix en vente indirecte (cas A) | - | - | `uc225a_discount_after_registration` | Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire) |
| UC225B | Modification de prix en vente indirecte (cas B) | - | - | `uc225a_discount_after_registration` | Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire) |
| UC226 | Modification de prix | - | - | - | Hors périmètre |
| UC229B | Article gratuit en vente directe | - | - | `uc229b_free_line_at_the_till` | Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire) |
| UC230 | Correction d'une ligne de vente avant paiement | `40_advanced/030_UC230.json` | PASS | `uc230_a_line_corrected_before_payment` | Couvert (scénario Android) |
| UC240 | Scission de table vers une nouvelle table | - | - | - | Hors périmètre |
| UC241 | Scission de table vers une table existante | - | - | - | Hors périmètre |
| UC242 | Scission de table vers deux nouvelles tables | - | - | - | Hors périmètre |
| UC243 | Scission de table avec remise | - | - | - | Hors périmètre |
| UC250 | Fusion de tables | - | - | - | Hors périmètre |
| UC251 | Fusion de tables (variante) | - | - | - | Hors périmètre |
| UC252 | Fusion de tables avec remise | - | - | - | Hors périmètre |
| UC260 | Vente d'un bon à usage multiple (MPV) | - | - | - | Hors périmètre |
| UC261 | Utilisation d'un bon à usage multiple (MPV) | - | - | - | Hors périmètre |
| UC262 | Utilisation d'un bon à usage multiple (MPV, variante) | - | - | - | Hors périmètre |
| UC270 | Vente d'un bon à usage unique (SPV) | - | - | - | Hors périmètre |
| UC271 | Utilisation d'un bon à usage unique (SPV) | - | - | - | Hors périmètre |
| UC280 | Produit composite | - | - | - | Hors périmètre |
| UC290 | Paiement mixte en devise étrangère | - | - | - | Hors périmètre |
| UC291 | Paiement en devise étrangère | - | - | - | Hors périmètre |
| UC292 | Pourboire | - | - | - | Hors périmètre |
| UC294 | Paiement en devise étrangère (variante) | - | - | - | Hors périmètre |
| UC295 | Paiement en devise étrangère (variante) | - | - | - | Hors périmètre |
| UC296 | Paiement en devise étrangère avec pourboire | - | - | - | Hors périmètre |
| UC4100 | Compte client | - | - | - | Hors périmètre |
| UC4101 | Transfert de table vers une chambre (changement de centre de coût) | - | - | - | Hors périmètre |
| UC4102 | Facturation combinée à une modification de prix | - | - | - | Hors périmètre |
| UC4103 | Scission d'addition avec changement de centre de coût | - | - | - | Hors périmètre |
| UC4103bis | Scission d'addition (variante bis) | - | - | - | Hors périmètre |
| UC4104 | Modification de prix sur un menu tout compris | - | - | - | Hors périmètre |
| UC4105 | Conversion d'un bon MPV en espèces | - | - | - | Hors périmètre |
| UC4106 | Modification de prix sur un menu tout compris | - | - | - | Hors périmètre |
| UC4107 | Libellés de TVA A/B/C | - | - | - | Hors périmètre |
| UC4108 | Libellé de TVA X (acompte) | - | - | - | Hors périmètre |
| UC4109 | Vente sans paiement | `40_advanced/040_UC4109.json` | PASS | `a_manager_closes_a_table_the_customer_left_unpaid`, `uc4109_unpaid_table` | Couvert (scénario Android) |
| UC4110 | Paiement anticipé (acompte) | - | - | - | Hors périmètre |
| UC4111 | Paiement anticipé (pourcentage du prix) | - | - | - | Hors périmètre |
| UC4112 | Produit vendu au poids | - | - | - | Hors périmètre |
| UC4113 | Acompte sur réservation | - | - | - | Hors périmètre |
| UC4114 | Caution pour événement (festival) | - | - | - | Hors périmètre |
| UC4116 | Correction combinée à une modification de prix | - | - | - | Hors périmètre |
| UC4117 | Code GTIN sur une ligne de produit | - | - | - | Hors périmètre |
| UC4118 | Correction de paiement (avancé) | - | - | - | Hors périmètre |
| UC4119 | Paiement scindé (split payment) | `40_advanced/050_UC4119.json` | PASS | `uc4119_split_payment_on_one_ticket`, `uc4119_three_thirds_cash_last` | Couvert (scénario Android) |
| UC4119bis | Scission d'addition (bis) | - | - | - | Hors périmètre |
| UC4120 | Supplément avec modification de prix | - | - | - | Hors périmètre |
| UC4120bis | Supplément sur une ligne de transaction (bis) | - | - | - | Hors périmètre |
| UC4121 | Facture client | - | - | `uc4121_an_invoice_of_three_sales` | Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire) |
| UC4122 | Note de crédit | - | - | `uc4122_an_invoice_of_a_refund_and_two_sales` | Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire) |
| UC4123 | Changement de centre de coût par chaise | - | - | - | Hors périmètre |

## Manuel

| Id | Libellé | Scénario Android | Résultat Android | Tests moteur | Statut |
|---|---|---|---|---|---|
| ROUND1 | Arrondi cash, vente directe simple 11,52 EUR (situation 1, arrondi -0,02 EUR) | `50_rounding/010_ROUND1.json` | PASS | `round1_11_52_is_paid_11_50`, `round1_dame_blanche_cash` | Couvert (scénario Android) |
| ROUND2 | Arrondi cash, vente directe simple 3,34 EUR (situation 2, arrondi +0,01 EUR) | `50_rounding/020_ROUND2.json` | PASS | `round2_3_34_is_paid_3_35`, `round2_tapas_variation_cash` | Couvert (scénario Android) |
| ROUND3 | Arrondi cash, vente combinée 14,86 EUR (situation 3, arrondi -0,01 EUR) | `50_rounding/030_ROUND3.json` | PASS | `round3_14_86_is_paid_14_85`, `round3_both_desserts_cash` | Couvert (scénario Android) |
| ROUND4 | Arrondi avec chèques-repas non arrondis + solde cash (situation 4, arrondi +0,01 EUR) | `50_rounding/040_ROUND4.json` | PASS | `round4_meal_vouchers_then_cash`, `round4_meal_vouchers_then_cash_rounds_the_cash_rest_only` | Couvert (scénario Android) |
| ROUND5 | Arrondi multi-paiements: chèques-repas + carte de débit + cash (situation 5, arrondi -0,01 EUR) | `50_rounding/050_ROUND5.json` | PASS | `round5_meal_vouchers_card_and_cash` | Couvert (scénario Android) |
| VI_BAR | Contenu des autres tickets sans désignation spécifique (cuisine, bar) | - | - | `vi_bar_a_bar_ticket_never_shows_a_price`, `vi_bar_a_correction_prints_as_a_negative_line`, `vi_bar_a_correction_prints_its_negative_quantity`, `vi_bar_a_direct_sale_has_no_table` +16 ; sur l'hôte seulement : `vi_bar_a_long_note_wraps_under_the_first_word`, `vi_bar_a_note_prints_in_cp858_not_as_question_marks`, `vi_bar_a_removed_note_says_so`, `vi_bar_an_item_note_prints_indented_under_its_item` +4 | Couvert (tests moteur exécutés sur l'appareil) |
| VI_COPY | Contenu du ticket copie (réimpression) | - | - | `vi_copy_m160_a_copy_reproduces_the_original_sale`, `vi_copy_m161_a_copy_of_a_pre_bill_is_not_a_vat_ticket`, `vi_copy_the_copy_mark_and_the_original_block_follow_the_language`, `vi_copy_the_official_copy_example` +1 | Couvert (tests moteur exécutés sur l'appareil) |
| VI_N | Contenu du ticket de vente (event N, ticket de caisse TVA) | - | - | `vi_n_a_sale_ticket_has_no_original_block`, `vi_n_the_issuer_legal_name_is_printed`, `vi_n_the_official_vat_ticket_example`, `vi_n_uc201_the_ticket_shows_the_sale_total_and_the_rounding_apart` | Couvert (tests moteur exécutés sur l'appareil) |
| VI_PROFORMA | Contenu du ticket pro forma (aperçu d'addition) | - | - | `vi_proforma_m123_the_pre_bill_says_addition_provisoire_without_vat`, `vi_proforma_the_official_pre_bill_example` | Couvert (tests moteur exécutés sur l'appareil) |
| VI_REFUND | Contenu du ticket de remboursement (event N REFUND) | - | - | `vi_refund_m111_the_refund_ticket_shows_the_sale_it_reverses`, `vi_refund_says_remboursement_once`, `vi_refund_the_official_refund_example` | Couvert (tests moteur exécutés sur l'appareil) |
| VI_TRAINING | Contenu du ticket en mode formation (training) | - | - | `vi_training_a_training_sale_prints_its_training_marks` | Couvert (tests moteur exécutés sur l'appareil) |
| VI_Z | Contenu du ticket de rapport Z | - | - | `vi_z_the_official_z_report_example` | Couvert (tests moteur exécutés sur l'appareil) |
| V_BUFFER_FULL | Buffer du FDM plein | `60_fdm_errors/020_V_BUFFER_FULL.json` | PASS | - | Couvert (scénario Android) |
| V_BUFFER_NEAR_FULL | Utilisation du buffer du FDM au-delà de 70% | `60_fdm_errors/030_V_BUFFER_NEAR_FULL.json` | PASS | - | Couvert (scénario Android) |
| V_CLIENT_CERT_EXPIRED | Certificat client expiré | - | - | - | Exclu sur Android |
| V_CLIENT_CERT_NEAR_EXPIRATION | Certificat client proche de l'expiration | - | - | - | Exclu sur Android |
| V_CORRUPT_RECORD_ENCOUNTERED | Transaction corrompue rencontrée en mémoire du FDM | - | - | - | Exclu sur Android |
| V_DUPLICATE_REQUEST | Requête en doublon reçue par le FDM | - | - | - | Exclu sur Android |
| V_FDM_LOCKED | FDM verrouillé | `60_fdm_errors/010_V_FDM_LOCKED.json` | PASS | - | Couvert (scénario Android) |
| V_FDM_NOT_OPERATIONAL | FDM non opérationnel | - | - | - | Exclu sur Android |
| V_INITIALIZATION_FAILED | Échec d'initialisation du FDM | - | - | - | Exclu sur Android |
| V_INTERNAL_ERROR | Problème technique interne du FDM | - | - | - | Exclu sur Android |
| V_INVALID_REQUEST | Requête invalide, non traitable par le FDM | - | - | - | Exclu sur Android |
| V_NOP_FAILED | Échec de l'envoi du message NOP | - | - | - | Exclu sur Android |
| V_RTC_NOT_INITIALIZED | Horloge temps réel du FDM non synchronisée | - | - | - | Exclu sur Android |
| V_RTC_SYNC_FAILED | Échec de synchronisation de l'horloge temps réel | - | - | - | Exclu sur Android |
| V_SERVER_CERT_RESOLVE_FAILED | Échec de résolution du certificat serveur | - | - | - | Exclu sur Android |
| V_TASK_FEEDBACK_FAILED | Échec de signalement du résultat de la tâche | - | - | - | Exclu sur Android |
| V_TOO_MANY_MEMORY_ERRORS | Trop de transactions corrompues en mémoire | - | - | - | Exclu sur Android |
| V_TRANSACTION_UPLOAD_FAILED | Échec répété (plus de 10 fois) d'envoi d'une transaction | - | - | - | Exclu sur Android |
| V_UNAUTHORIZED | Requête non autorisée reçue par le FDM | `60_fdm_errors/050_V_UNAUTHORIZED.json` | PASS | - | Couvert (scénario Android) |
| V_UNDEFINED_ERROR | Erreur indéfinie rencontrée par le FDM | - | - | - | Exclu sur Android |
| V_UNDEFINED_OTHER | Autre erreur, avertissement ou information non listée | - | - | - | Exclu sur Android |
| V_UNKNOWN_POS | POS non repris dans la liste d'autorisation du FDM | `60_fdm_errors/040_V_UNKNOWN_POS.json` | PASS | - | Couvert (scénario Android) |
| V_UPDATE_CLIENT_CERT_FAILED | Échec de mise à jour du certificat client | - | - | - | Exclu sur Android |
| V_UPDATE_PARAMS_FAILED | Échec de mise à jour des paramètres | - | - | - | Exclu sur Android |
| V_UPDATE_POS_ALLOWLIST_FAILED | Échec de mise à jour de la liste d'autorisation | - | - | - | Exclu sur Android |
| V_UPDATE_TASK_LIST_FAILED | Échec de téléchargement de la liste de tâches | - | - | - | Exclu sur Android |
| V_UPDATE_TRUST_CERT_FAILED | Échec de mise à jour des certificats de confiance | - | - | - | Exclu sur Android |
| V_UPDATE_URLS_FAILED | Échec de mise à jour des URLs | - | - | - | Exclu sur Android |
| V_UPDATE_VAT_RATES_FAILED | Échec de mise à jour des taux de TVA | - | - | - | Exclu sur Android |

## FAQ

| Id | Libellé | Scénario Android | Résultat Android | Tests moteur | Statut |
|---|---|---|---|---|---|
| FAQ_Q202 | signPreBill: usage et lien avec un signOrder préalable | - | - | `m123_sign_pre_bill_archives_p_event_and_keeps_order_open`, `pre_bill_fdm_failure_on_the_second_step_does_not_burn_its_ticket_or_resend_the_order` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q205 | Une transaction en cours doit-elle donner lieu à un event P ou N | `70_faq/010_FAQ_Q205.json` | PASS | `faq_q205_a_takeaway_closed_unpaid_is_a_direct_sale`, `faq_q205_a_takeaway_is_never_registered_as_an_order`, `faq_q205_open_takeaways_are_listed_until_paid` | Couvert (scénario Android) |
| FAQ_Q206 | Vente soldée à 0,00 EUR (remise de 100%): enregistrement | - | - | `faq_q206_a_zero_sale_is_signed_with_a_vat_ticket` ; sur l'hôte seulement : `a_sale_settled_at_zero_sends_empty_financials_faq_q206` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q207 | Règles d'usage de signPreBill et signOrder en gestion de table | - | - | `faq_q207_a_second_order_carries_only_the_new_lines` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q209 | Ticket de caisse TVA obligatoire même pour une vente à 0,00 EUR | - | - | `faq_q206_a_zero_sale_is_signed_with_a_vat_ticket` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q211 | Unicité de la référence de costCenter | - | - | `faq_q211_each_table_session_has_its_own_reference` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q212 | Évolution de la référence de costCenter lors d'un transfert de table | - | - | `m122_cost_center_change` | Couvert (tests moteur sur l'hôte, passage sur l'appareil à faire) |
| FAQ_Q301 | Distinction entre motifs REFUND et CORRECTION (negQuantityReason) | - | - | `m111_full_refund`, `negative_quantity_reason_depends_on_the_sale`, `uc230_a_line_corrected_before_payment` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q303 | Base de comptage des negQuantityReason dans le rapport X/Z | - | - | `a_table_correction_counts_on_its_order_registration`, `negative_lines_are_counted_per_reason` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q401 | Money In/Out obligatoire dès que la caisse encaisse du cash, même sans tiroir | - | - | `m130_money_in`, `m130_money_in_out_archives_f_event` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q402 | Enregistrement du rendu de monnaie et du cashback dans les objets payment | - | - | `uc201_cash_rounding_is_its_own_payment_line` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q403 | Encodage des pourboires (TIP) et des arrondis (ROUNDING) vers le FDM | - | - | `faq_q403_a_card_payment_has_no_rounding_line`, `faq_q403_rounding_is_reported_apart_and_left_out_of_the_turnover` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q405 | signDrawerOpen: usage et absence de lien obligatoire avec signMoneyInOut | - | - | `a_cash_sale_prints_and_opens_the_drawer`, `the_drawer_opens_with_its_signed_event` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q501 | Périodicité du rapport Z de chiffre d'affaires (au moins une fois par jour) | - | - | `faq_q501_a_day_left_open_is_closed_by_an_automatic_z`, `faq_q501_a_new_period_takes_the_business_day`, `faq_q501_a_service_past_midnight_stays_on_its_day`, `faq_q501_an_fdm_failure_leaves_the_day_open` +4 | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q502 | Les events R de rapport sont exclus des timestamps de la période couverte | - | - | `m180_turnover_x_of_the_scenario_day` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q503 | signCopy obligatoire pour toute réimpression, y compris une facture A4 | - | - | `a_printed_original_can_not_be_printed_again`, `m160_copy_of_sale_archives_c_event_with_ref_to_original` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q504 | Le rapport de chiffre d'affaires n'inclut pas les events de type Copy | - | - | `faq_q504_a_copy_is_not_turnover` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q505 | Identification du deviceId (adresse MAC ou identifiant matériel unique) | - | - | `m110_sale` ; sur l'hôte seulement : `m150_invoice` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q506 | La query Status du FDM est facultative | - | - | `faq_q506_a_sale_is_signed_without_a_status_query` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q507 | La mention TICKET DE CAISSE T.V.A. est réservée au ticket de caisse fiscal | - | - | `every_ticket_carries_its_type_label_and_identification`, `vi_training_a_training_sale_prints_its_training_marks` | Couvert (tests moteur exécutés sur l'appareil) |
| FAQ_Q509 | Champ totalAmount du rapport Z (objet users): somme des transactionTotal | - | - | `faq_q509_the_user_total_leaves_the_rounding_out` | Couvert (tests moteur exécutés sur l'appareil) |

## Exclusions propres à Android

| Id | Raison |
|---|---|
| V_CLIENT_CERT_EXPIRED | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_CLIENT_CERT_NEAR_EXPIRATION | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_CORRUPT_RECORD_ENCOUNTERED | Ce code ne doit jamais être affiché (showPos NEVER) : sur Linux il n'est prouvé que par le test d'interface qui vérifie l'absence de bandeau (ui/e2e/fdm-messages.spec.ts, messages jamais affichés) ; sur Android la classification du moteur écarte ces codes, ce que prouvent les tests errors_that_are_never_shown_are_ignored et a_never_error_does_not_hide_a_warning de fdm_monitor.rs exécutés sur l'appareil, et le test Robolectric FdmBannerTest.nothingIsDrawnWithoutAMessage prouve qu'aucun bandeau n'est dessiné sans message. |
| V_DUPLICATE_REQUEST | Ce code ne doit jamais être affiché (showPos NEVER) : sur Linux il n'est prouvé que par le test d'interface qui vérifie l'absence de bandeau (ui/e2e/fdm-messages.spec.ts, messages jamais affichés) ; sur Android la classification du moteur écarte ces codes, ce que prouvent les tests errors_that_are_never_shown_are_ignored et a_never_error_does_not_hide_a_warning de fdm_monitor.rs exécutés sur l'appareil, et le test Robolectric FdmBannerTest.nothingIsDrawnWithoutAMessage prouve qu'aucun bandeau n'est dessiné sans message. |
| V_FDM_NOT_OPERATIONAL | Sur Linux ce code bloquant n'est prouvé que par un test d'interface (voile qui recouvre les écrans de vente) ; sur Android le voile est indépendant du code et prouvé par les tests Robolectric FdmBlockingVeilTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_INITIALIZATION_FAILED | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_INTERNAL_ERROR | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_INVALID_REQUEST | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_NOP_FAILED | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_RTC_NOT_INITIALIZED | Ce code ne doit jamais être affiché (showPos NEVER) : sur Linux il n'est prouvé que par le test d'interface qui vérifie l'absence de bandeau (ui/e2e/fdm-messages.spec.ts, messages jamais affichés) ; sur Android la classification du moteur écarte ces codes, ce que prouvent les tests errors_that_are_never_shown_are_ignored et a_never_error_does_not_hide_a_warning de fdm_monitor.rs exécutés sur l'appareil, et le test Robolectric FdmBannerTest.nothingIsDrawnWithoutAMessage prouve qu'aucun bandeau n'est dessiné sans message. |
| V_RTC_SYNC_FAILED | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_SERVER_CERT_RESOLVE_FAILED | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_TASK_FEEDBACK_FAILED | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_TOO_MANY_MEMORY_ERRORS | Ce code ne doit jamais être affiché (showPos NEVER) : sur Linux il n'est prouvé que par le test d'interface qui vérifie l'absence de bandeau (ui/e2e/fdm-messages.spec.ts, messages jamais affichés) ; sur Android la classification du moteur écarte ces codes, ce que prouvent les tests errors_that_are_never_shown_are_ignored et a_never_error_does_not_hide_a_warning de fdm_monitor.rs exécutés sur l'appareil, et le test Robolectric FdmBannerTest.nothingIsDrawnWithoutAMessage prouve qu'aucun bandeau n'est dessiné sans message. |
| V_TRANSACTION_UPLOAD_FAILED | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_UNDEFINED_ERROR | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_UNDEFINED_OTHER | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_UPDATE_CLIENT_CERT_FAILED | Ce code ne doit jamais être affiché (showPos NEVER) : sur Linux il n'est prouvé que par le test d'interface qui vérifie l'absence de bandeau (ui/e2e/fdm-messages.spec.ts, messages jamais affichés) ; sur Android la classification du moteur écarte ces codes, ce que prouvent les tests errors_that_are_never_shown_are_ignored et a_never_error_does_not_hide_a_warning de fdm_monitor.rs exécutés sur l'appareil, et le test Robolectric FdmBannerTest.nothingIsDrawnWithoutAMessage prouve qu'aucun bandeau n'est dessiné sans message. |
| V_UPDATE_PARAMS_FAILED | Ce code ne doit jamais être affiché (showPos NEVER) : sur Linux il n'est prouvé que par le test d'interface qui vérifie l'absence de bandeau (ui/e2e/fdm-messages.spec.ts, messages jamais affichés) ; sur Android la classification du moteur écarte ces codes, ce que prouvent les tests errors_that_are_never_shown_are_ignored et a_never_error_does_not_hide_a_warning de fdm_monitor.rs exécutés sur l'appareil, et le test Robolectric FdmBannerTest.nothingIsDrawnWithoutAMessage prouve qu'aucun bandeau n'est dessiné sans message. |
| V_UPDATE_POS_ALLOWLIST_FAILED | Ce code ne doit jamais être affiché (showPos NEVER) : sur Linux il n'est prouvé que par le test d'interface qui vérifie l'absence de bandeau (ui/e2e/fdm-messages.spec.ts, messages jamais affichés) ; sur Android la classification du moteur écarte ces codes, ce que prouvent les tests errors_that_are_never_shown_are_ignored et a_never_error_does_not_hide_a_warning de fdm_monitor.rs exécutés sur l'appareil, et le test Robolectric FdmBannerTest.nothingIsDrawnWithoutAMessage prouve qu'aucun bandeau n'est dessiné sans message. |
| V_UPDATE_TASK_LIST_FAILED | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_UPDATE_TRUST_CERT_FAILED | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_UPDATE_URLS_FAILED | Sur Linux ce code n'est prouvé que par un test d'interface (bandeau sous l'en-tête) ; sur Android le bandeau est indépendant du code et prouvé par les tests Robolectric FdmBannerTest, avec le rendu vérifié par le test Roborazzi FdmScreenshotTest ; le texte français du code est généré depuis la table de l'interface Linux (ui/src/utils/fdmMessages.ts) et sa synchronisation est contrôlée par `node scripts/android/vue_to_android.mjs --check` ; la classification du code est prouvée par les tests du moteur (fdm_monitor.rs) exécutés sur l'appareil. |
| V_UPDATE_VAT_RATES_FAILED | Ce code ne doit jamais être affiché (showPos NEVER) : sur Linux il n'est prouvé que par le test d'interface qui vérifie l'absence de bandeau (ui/e2e/fdm-messages.spec.ts, messages jamais affichés) ; sur Android la classification du moteur écarte ces codes, ce que prouvent les tests errors_that_are_never_shown_are_ignored et a_never_error_does_not_hide_a_warning de fdm_monitor.rs exécutés sur l'appareil, et le test Robolectric FdmBannerTest.nothingIsDrawnWithoutAMessage prouve qu'aucun bandeau n'est dessiné sans message. |
