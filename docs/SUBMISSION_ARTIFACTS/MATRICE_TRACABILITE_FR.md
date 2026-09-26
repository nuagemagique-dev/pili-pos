# Matrice de traçabilité des tests SPF

Ce document liste chaque identifiant de test décrit par le SPF Finances (Basic Set, Reports, Advanced Set, checklist du manuel de certification, FAQ producteurs) et indique s'il est couvert par un test automatisé (Rust ou interface utilisateur), déclaré hors périmètre, non applicable, ou couvert par les tests d'un autre identifiant.

Généré automatiquement par `scripts/uc_matrix.py` à partir des dossiers sous `docs/`, des sources de test (`crates/*/tests`, `crates/*/src`, `src-tauri/tests`, `src-tauri/src`, `ui/e2e/`) et des exclusions déclarées dans `scripts/uc_scope.toml`. Une réexécution sans changement de code ou de documentation produit un fichier identique.

Date de génération (date du dernier commit git) : 2026-09-26

Légende du statut : Couvert (au moins un test automatisé trouvé), Hors périmètre (fonctionnalité absente de moyun-pos v1.0.0, exclusion documentée), Non applicable (ce cas précis ne peut pas se produire avec le comportement actuel), Couvert par X (les tests de l'identifiant X couvrent aussi ce cas), MANQUANT (aucun test automatisé trouvé et aucune exclusion déclarée).

## Basic Set

| ID | Intitulé | Tests Rust | Tests UI | Statut |
|---|---|---|---|---|
| M110 | Vente directe (ticket de caisse TVA) | `m110_sale`, `m110_sign_sale_archives_n_event_and_closes_order`, `m110_sign_sale_response_shape` | - | Couvert |
| M111 | Remboursement total d'une vente | `m111_full_refund`, `m111_full_refund_archives_negative_n_with_fdm_ref` | - | Couvert |
| M112A | Remboursement partiel combiné à une nouvelle vente | - | - | Non applicable |
| M112B | Remboursement partiel en ticket séparé | `m112b_partial_refund_as_a_separate_ticket`, `m112b_partial_refund_is_separate_event_chained_to_original` | - | Couvert |
| M121 | Commande en gestion de table | `m121_sign_order_archives_p_event_and_keeps_order_open`, `m121_sign_order_response_omits_n_only_fields`, `m121_table_order` | - | Couvert |
| M122 | Changement de centre de coût (transfert de table) | - | - | Hors périmètre |
| M123 | Addition provisoire (pre-bill) | `m123_pre_bill`, `m123_sign_pre_bill_archives_p_event_and_keeps_order_open` | - | Couvert |
| M130 | Mouvement de caisse (argent entrant/sortant) | `m130_money_in`, `m130_money_in_out_archives_f_event` | - | Couvert |
| M131 | Ouverture du tiroir-caisse | `m131_drawer_open`, `m131_drawer_open_archives_f_event` | - | Couvert |
| M132 | Correction de paiement | - | - | Hors périmètre |
| M133 | Déclaration de fond de caisse (comptage) | `m133_drawer_declaration` | `[M133] an empty drawer is declared as 0`, `[M133] the counted cash is declared with the amount typed` | Couvert |
| M140 | Entrée en service de l'utilisateur | `m140_work_in`, `m140_work_in_archives_s_event` | - | Couvert |
| M141 | Sortie de service de l'utilisateur | `m141_work_out`, `m141_work_out_archives_s_event` | - | Couvert |
| M150 | Facturation | `m150_invoice_fdm_failure_does_not_advance_i_counter`, `m150_sign_invoice_archives_i_event_referencing_the_sale` | - | Hors périmètre |
| M160 | Copie d'un ticket de vente | `m160_copy_of_a_sale`, `m160_copy_of_sale_archives_c_event_with_ref_to_original` | - | Couvert |
| M161 | Copie d'une commande | `m161_copy_of_an_order`, `m161_copy_of_order_archives_c_event_with_ref_to_p_event` | - | Couvert |
| M170 | Entrée en service en mode formation | `m170_work_in_in_training`, `m170_work_in_training_does_not_advance_normal_s_counter` | - | Couvert |
| M171 | Vente en mode formation | `m171_sale_in_training`, `m171_sale_in_training_uses_t_counter_and_keeps_n_at_zero`, `m171_training_sale_has_no_vat_calc` | - | Couvert |
| M171B | Vente en mode formation (style de requête @skip) | - | - | MANQUANT |
| M172 | Copie en mode formation | `m172_copy_in_training`, `m172_copy_in_training_mode_archives_c_event` | - | Couvert |
| Q100 | Requête de statut du FDM | `q100_status_response_shape` | - | Couvert |
| Q110 | Requête de statut, messages multilingues | `q110_multi_lang_status_response_shape`, `status_multi_lang_parses_q110_envelope` | - | Couvert |

## Reports

| ID | Intitulé | Tests Rust | Tests UI | Statut |
|---|---|---|---|---|
| M180 | Rapport X du chiffre d'affaires | `m180_generate_x_report_archives_r_event_without_resetting_counter`, `m180_turnover_x`, `m180_turnover_x_of_the_scenario_day`, `m180_turnover_x_response_parses` | - | Couvert |
| M181 | Rapport Z du chiffre d'affaires | `m181_turnover_z_has_the_same_totals`, `m181_turnover_z_response_parses` | - | Couvert |
| M182 | Rapport X des utilisateurs | `m182_generate_user_x_report_archives_r_event_with_user_block`, `m182_user_x`, `m182_user_x_of_the_scenario_day`, `m182_user_x_response_parses` | - | Couvert |
| M183 | Rapport Z des utilisateurs | `m183_user_z_has_the_same_users`, `m183_user_z_response_parses` | - | Couvert |
| M184 | Rapport Z combiné (chiffre d'affaires + utilisateurs) | `m184_combined_z`, `m184_combined_z_of_the_scenario_day`, `m184_combined_z_response_parses_and_counters_increment`, `m184_generate_z_report_signs_both_reports_in_one_operation` | - | Couvert |
| T1 | Rejeu d'entrée T1 (signOrder) | - | - | MANQUANT |
| T2 | Rejeu d'entrée T2 (signSale) | - | - | MANQUANT |
| T3 | Rejeu d'entrée T3 (signSale, remboursement) | - | - | MANQUANT |
| T4 | Rejeu d'entrée T4 (signSale) | - | - | MANQUANT |
| T5 | Rejeu d'entrée T5 (signOrder) | - | - | MANQUANT |
| T6 | Rejeu d'entrée T6 (signOrder) | - | - | MANQUANT |
| T7 | Rejeu d'entrée T7 (signPreBill) | - | - | MANQUANT |
| T8 | Rejeu d'entrée T8 (signSale) | - | - | MANQUANT |

## Advanced Set

| ID | Intitulé | Tests Rust | Tests UI | Statut |
|---|---|---|---|---|
| UC200 | Vente directe d'un produit composite | - | - | Hors périmètre |
| UC201 | Vente avec arrondi du paiement | `uc201_cash_rounding_is_its_own_payment_line` | - | Couvert |
| UC220 | Modification de prix sur une ligne de vente | - | - | Hors périmètre |
| UC221A | Modification de prix sur une ligne (cas A) | - | - | Hors périmètre |
| UC221B | Modification de prix sur une ligne (cas B) | - | - | Hors périmètre |
| UC222 | Modification de prix sur un produit composite | - | - | Hors périmètre |
| UC223 | Modification de prix au niveau de l'event de vente | - | - | Hors périmètre |
| UC224 | Modifications de prix multiples sur une vente | - | - | Hors périmètre |
| UC225A | Modification de prix en vente indirecte (cas A) | - | - | Hors périmètre |
| UC225B | Modification de prix en vente indirecte (cas B) | - | - | Hors périmètre |
| UC226 | Modification de prix | - | - | Hors périmètre |
| UC229B | Article gratuit en vente directe | - | - | Hors périmètre |
| UC230 | Correction d'une ligne de vente avant paiement | - | - | MANQUANT |
| UC240 | Scission de table vers une nouvelle table | - | - | Hors périmètre |
| UC241 | Scission de table vers une table existante | - | - | Hors périmètre |
| UC242 | Scission de table vers deux nouvelles tables | - | - | Hors périmètre |
| UC243 | Scission de table avec remise | - | - | Hors périmètre |
| UC250 | Fusion de tables | - | - | Hors périmètre |
| UC251 | Fusion de tables (variante) | - | - | Hors périmètre |
| UC252 | Fusion de tables avec remise | - | - | Hors périmètre |
| UC260 | Vente d'un bon à usage multiple (MPV) | - | - | Hors périmètre |
| UC261 | Utilisation d'un bon à usage multiple (MPV) | - | - | Hors périmètre |
| UC262 | Utilisation d'un bon à usage multiple (MPV, variante) | - | - | Hors périmètre |
| UC270 | Vente d'un bon à usage unique (SPV) | - | - | Hors périmètre |
| UC271 | Utilisation d'un bon à usage unique (SPV) | - | - | Hors périmètre |
| UC280 | Produit composite | - | - | Hors périmètre |
| UC290 | Paiement mixte en devise étrangère | - | - | Hors périmètre |
| UC291 | Paiement en devise étrangère | - | - | Hors périmètre |
| UC292 | Pourboire | - | - | Hors périmètre |
| UC294 | Paiement en devise étrangère (variante) | - | - | Hors périmètre |
| UC295 | Paiement en devise étrangère (variante) | - | - | Hors périmètre |
| UC296 | Paiement en devise étrangère avec pourboire | - | - | Hors périmètre |
| UC4100 | Compte client | - | - | Hors périmètre |
| UC4101 | Transfert de table vers une chambre (changement de centre de coût) | - | - | Hors périmètre |
| UC4102 | Facturation combinée à une modification de prix | - | - | Hors périmètre |
| UC4103 | Scission d'addition avec changement de centre de coût | - | - | Hors périmètre |
| UC4103bis | Scission d'addition (variante bis) | - | - | Hors périmètre |
| UC4104 | Modification de prix sur un menu tout compris | - | - | Hors périmètre |
| UC4105 | Conversion d'un bon MPV en espèces | - | - | Hors périmètre |
| UC4106 | Modification de prix sur un menu tout compris | - | - | Hors périmètre |
| UC4107 | Libellés de TVA A/B/C | - | - | Hors périmètre |
| UC4108 | Libellé de TVA X (acompte) | - | - | Hors périmètre |
| UC4109 | Vente sans paiement | - | - | MANQUANT |
| UC4110 | Paiement anticipé (acompte) | - | - | Hors périmètre |
| UC4111 | Paiement anticipé (pourcentage du prix) | - | - | Hors périmètre |
| UC4112 | Produit vendu au poids | - | - | Hors périmètre |
| UC4113 | Acompte sur réservation | - | - | Hors périmètre |
| UC4114 | Caution pour événement (festival) | - | - | Hors périmètre |
| UC4116 | Correction combinée à une modification de prix | - | - | Hors périmètre |
| UC4117 | Code GTIN sur une ligne de produit | - | - | Hors périmètre |
| UC4118 | Correction de paiement (avancé) | - | - | Hors périmètre |
| UC4119 | Paiement scindé (split payment) | - | - | Hors périmètre |
| UC4119bis | Scission d'addition (bis) | - | - | Hors périmètre |
| UC4120 | Supplément avec modification de prix | - | - | Hors périmètre |
| UC4120bis | Supplément sur une ligne de transaction (bis) | - | - | Hors périmètre |
| UC4121 | Facture client | - | - | Hors périmètre |
| UC4122 | Note de crédit | - | - | Hors périmètre |
| UC4123 | Changement de centre de coût par chaise | - | - | Hors périmètre |

## Manuel

| ID | Intitulé | Tests Rust | Tests UI | Statut |
|---|---|---|---|---|
| ROUND1 | Arrondi cash, vente directe simple 11,52 EUR (situation 1, arrondi -0,02 EUR) | - | - | MANQUANT |
| ROUND2 | Arrondi cash, vente directe simple 3,34 EUR (situation 2, arrondi +0,01 EUR) | - | - | MANQUANT |
| ROUND3 | Arrondi cash, vente combinée 14,86 EUR (situation 3, arrondi -0,01 EUR) | - | - | MANQUANT |
| ROUND4 | Arrondi avec chèques-repas non arrondis + solde cash (situation 4, arrondi +0,01 EUR) | - | - | MANQUANT |
| ROUND5 | Arrondi multi-paiements: chèques-repas + carte de débit + cash (situation 5, arrondi -0,01 EUR) | - | - | MANQUANT |
| VI_BAR | Contenu des autres tickets sans désignation spécifique (cuisine, bar) | `vi_bar_a_bar_ticket_never_shows_a_price`, `vi_bar_a_correction_prints_as_a_negative_line`, `vi_bar_a_correction_prints_its_negative_quantity`, `vi_bar_a_direct_sale_has_no_table` +14 | `[VI_BAR] a pre-bill whose slip failed says so next to its confirmation`, `[VI_BAR] a printed slip needs no message`, `[VI_BAR] a sale whose slip failed is still paid, with the warning`, `[VI_BAR] a slip that did not print is told, the order stays registered` | Couvert |
| VI_COPY | Contenu du ticket copie (réimpression) | - | - | MANQUANT |
| VI_N | Contenu du ticket de vente (event N, ticket de caisse TVA) | - | - | MANQUANT |
| VI_PROFORMA | Contenu du ticket pro forma (aperçu d'addition) | - | - | MANQUANT |
| VI_REFUND | Contenu du ticket de remboursement (event N REFUND) | - | - | MANQUANT |
| VI_TRAINING | Contenu du ticket en mode formation (training) | - | - | MANQUANT |
| VI_Z | Contenu du ticket de rapport Z | - | - | MANQUANT |
| V_BUFFER_FULL | Buffer du FDM plein | - | - | MANQUANT |
| V_BUFFER_NEAR_FULL | Utilisation du buffer du FDM au-delà de 70% | - | - | MANQUANT |
| V_CLIENT_CERT_EXPIRED | Certificat client expiré | - | - | MANQUANT |
| V_CLIENT_CERT_NEAR_EXPIRATION | Certificat client proche de l'expiration | - | - | MANQUANT |
| V_CORRUPT_RECORD_ENCOUNTERED | Transaction corrompue rencontrée en mémoire du FDM | - | - | MANQUANT |
| V_DUPLICATE_REQUEST | Requête en doublon reçue par le FDM | - | - | MANQUANT |
| V_FDM_LOCKED | FDM verrouillé | - | - | MANQUANT |
| V_FDM_NOT_OPERATIONAL | FDM non opérationnel | - | - | MANQUANT |
| V_INITIALIZATION_FAILED | Échec d'initialisation du FDM | - | - | MANQUANT |
| V_INTERNAL_ERROR | Problème technique interne du FDM | - | - | MANQUANT |
| V_INVALID_REQUEST | Requête invalide, non traitable par le FDM | - | - | MANQUANT |
| V_NOP_FAILED | Échec de l'envoi du message NOP | - | - | MANQUANT |
| V_RTC_NOT_INITIALIZED | Horloge temps réel du FDM non synchronisée | - | - | MANQUANT |
| V_RTC_SYNC_FAILED | Échec de synchronisation de l'horloge temps réel | - | - | MANQUANT |
| V_SERVER_CERT_RESOLVE_FAILED | Échec de résolution du certificat serveur | - | - | MANQUANT |
| V_TASK_FEEDBACK_FAILED | Échec de signalement du résultat de la tâche | - | - | MANQUANT |
| V_TOO_MANY_MEMORY_ERRORS | Trop de transactions corrompues en mémoire | - | - | MANQUANT |
| V_TRANSACTION_UPLOAD_FAILED | Échec répété (plus de 10 fois) d'envoi d'une transaction | - | - | MANQUANT |
| V_UNAUTHORIZED | Requête non autorisée reçue par le FDM | - | - | MANQUANT |
| V_UNDEFINED_ERROR | Erreur indéfinie rencontrée par le FDM | - | - | MANQUANT |
| V_UNDEFINED_OTHER | Autre erreur, avertissement ou information non listée | - | - | MANQUANT |
| V_UNKNOWN_POS | POS non repris dans la liste d'autorisation du FDM | - | - | MANQUANT |
| V_UPDATE_CLIENT_CERT_FAILED | Échec de mise à jour du certificat client | - | - | MANQUANT |
| V_UPDATE_PARAMS_FAILED | Échec de mise à jour des paramètres | - | - | MANQUANT |
| V_UPDATE_POS_ALLOWLIST_FAILED | Échec de mise à jour de la liste d'autorisation | - | - | MANQUANT |
| V_UPDATE_TASK_LIST_FAILED | Échec de téléchargement de la liste de tâches | - | - | MANQUANT |
| V_UPDATE_TRUST_CERT_FAILED | Échec de mise à jour des certificats de confiance | - | - | MANQUANT |
| V_UPDATE_URLS_FAILED | Échec de mise à jour des URLs | - | - | MANQUANT |
| V_UPDATE_VAT_RATES_FAILED | Échec de mise à jour des taux de TVA | - | - | MANQUANT |

## FAQ

| ID | Intitulé | Tests Rust | Tests UI | Statut |
|---|---|---|---|---|
| FAQ_Q202 | signPreBill: usage et lien avec un signOrder préalable | - | - | MANQUANT |
| FAQ_Q205 | Une transaction en cours doit-elle donner lieu à un event P ou N | - | - | MANQUANT |
| FAQ_Q206 | Vente soldée à 0,00 EUR (remise de 100%): enregistrement | `faq_q206_a_zero_sale_is_signed_with_a_vat_ticket` | - | Couvert |
| FAQ_Q207 | Règles d'usage de signPreBill et signOrder en gestion de table | - | - | MANQUANT |
| FAQ_Q209 | Ticket de caisse TVA obligatoire même pour une vente à 0,00 EUR | - | - | MANQUANT |
| FAQ_Q211 | Unicité de la référence de costCenter | - | - | MANQUANT |
| FAQ_Q212 | Évolution de la référence de costCenter lors d'un transfert de table | - | - | MANQUANT |
| FAQ_Q301 | Distinction entre motifs REFUND et CORRECTION (negQuantityReason) | - | - | MANQUANT |
| FAQ_Q303 | Base de comptage des negQuantityReason dans le rapport X/Z | - | - | MANQUANT |
| FAQ_Q401 | Money In/Out obligatoire dès que la caisse encaisse du cash, même sans tiroir | - | - | MANQUANT |
| FAQ_Q402 | Enregistrement du rendu de monnaie et du cashback dans les objets payment | - | - | MANQUANT |
| FAQ_Q403 | Encodage des pourboires (TIP) et des arrondis (ROUNDING) vers le FDM | `faq_q403_a_card_payment_has_no_rounding_line`, `faq_q403_rounding_is_reported_apart_and_left_out_of_the_turnover` | - | Couvert |
| FAQ_Q405 | signDrawerOpen: usage et absence de lien obligatoire avec signMoneyInOut | - | - | MANQUANT |
| FAQ_Q501 | Périodicité du rapport Z de chiffre d'affaires (au moins une fois par jour) | `faq_q501_a_day_left_open_is_closed_by_an_automatic_z`, `faq_q501_a_new_period_takes_the_business_day`, `faq_q501_a_service_past_midnight_stays_on_its_day`, `faq_q501_an_fdm_failure_leaves_the_day_open` +4 | - | Couvert |
| FAQ_Q502 | Les events R de rapport sont exclus des timestamps de la période couverte | - | - | MANQUANT |
| FAQ_Q503 | signCopy obligatoire pour toute réimpression, y compris une facture A4 | - | - | MANQUANT |
| FAQ_Q504 | Le rapport de chiffre d'affaires n'inclut pas les events de type Copy | - | - | MANQUANT |
| FAQ_Q505 | Identification du deviceId (adresse MAC ou identifiant matériel unique) | - | - | MANQUANT |
| FAQ_Q506 | La query Status du FDM est facultative | - | - | MANQUANT |
| FAQ_Q507 | La mention TICKET DE CAISSE TVA est réservée au ticket de caisse fiscal | - | - | MANQUANT |
| FAQ_Q509 | Champ totalAmount du rapport Z (objet users): somme des transactionTotal | - | - | MANQUANT |
