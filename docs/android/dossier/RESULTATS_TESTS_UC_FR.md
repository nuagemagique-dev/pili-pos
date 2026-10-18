<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/RESULTATS_TESTS_UC_FR.md @ 86088e4 -->
# Résultats des tests UC - pili-pos-android 1.0.0

**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)
**Date :** À effectuer (EV-18 : date de dépôt)
**Référence :** §IV du manuel de certification (Procédure de communication GraphQL).

Squelette des résultats des UC obligatoires du Basic Set v1.8 (22 UC spec), des 5 UC Reports v1.3 et des cinq situations d'arrondi (ROUND1 à ROUND5), plus onze UC de l'Advanced Set v1.6 rejoués par le même harnais, dont les deux UC de facture (UC4121 et UC4122). Aucun UC n'est hors périmètre : M122 (transfert de table), M132 (correction de paiement) et M150 (facture, événement I) sont dans le périmètre ; M112A n'est pas un parcours proposé (parcours M112B). Aucun résultat n'est repris du dossier Linux : chaque bloc ci-dessous attend les résultats des UC joués sur le terminal émulé par la suite instrumentée (`À effectuer (EV-06 : résultats des UC sur le terminal émulé)`).

Pour chaque UC rejoué, le bloc final fournira : le scénario, les champs à vérifier, la requête construite par le moteur et archivée, la réponse du FDM, la comparaison champ par champ avec la fixture officielle et le test automatisé. Les fichiers JSON bruts iront dans `Report_Testing` (`À effectuer (EV-06 : requête et réponse JSON de chaque UC)`). Les tickets sont dans TICKETS_EXEMPLES_FR.md. Le FDM utilisé pour ces essais est nommé dans chaque bloc ; la journée sur le simulateur en ligne module2 est dans JOURNAL_TESTS_PRE_SOUMISSION_FR.md. La comparaison avec les requêtes du dossier Linux est `À effectuer (EV-07 : comparaison Android et Linux)`.

**Tableau « Champs à vérifier » de chaque UC :**

| Champ | Valeur fixture | Statut |
|---|---|:-:|
| À effectuer (EV-06 : champs de la fixture) | À effectuer (EV-06) | À effectuer (EV-06) |

# Basic Set v1.8 (22 UC)
## M110_signSale
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M111_signSale_Refund
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M112A_signSale_RefundPartial
Non proposé comme parcours (le remboursement partiel est toujours émis en ticket séparé, parcours M112B). À effectuer (EV-06 : lecture de la fixture par le moteur sur le terminal émulé).
## M112B_signSale_RefundPartial
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M121_signOrder
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M122_signCostCenterChange
Transfert de table (attribution). Rejeu applicatif côté moteur : `m122_cost_center_change`, écart nul. À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture sur le terminal émulé).
## M123_signPrebill
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M130_signMoneyInOut
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M131_signDrawerOpen
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M132_signPaymentCorrection
Correction de paiement. Rejeu applicatif côté moteur : `m132_payment_correction`, un écart déclaré (type de carte `CARD_DEBIT` au lieu de `CARD_CREDIT`). Deux différences ne sont pas comparées (la comparaison porte sur le type, le montant et le type de montant de chaque ligne) : la nouvelle ligne de carte part en saisie manuelle (`inputMethod` MANUAL, la fixture a AUTOMATIC) et la ligne d'espèces n'indique pas de tiroir (champ optionnel, la fixture nomme le tiroir 1). À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture sur le terminal émulé).
## M133_signMoneyInOut_DrawerDeclaration
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M140_signWorkIn
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M141_signWorkOut
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M150_signInvoice
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M160_signCopy_Sale
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M161_signCopy_Order
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M170_signWorkIn_Training
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M171B_signSale_Training
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M171_signSale_Training
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M172_signCopy_Training
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## Q100_Status
À effectuer (EV-06 : réponse du FDM au statut et validation automatisée).
## Q110_Status_multi_lang_messages
À effectuer (EV-06 : réponse du FDM au statut et validation automatisée).
# Reports v1.3 (5 UC)
## M180_signReportTurnoverX
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M181_signReportTurnoverZ
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M182_signReportUserX
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M183_signReportUserZ
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M184_signReportTurnoverZUserZ
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
# Arrondissement des méthodes de paiement (manuel §IV)
## ROUND1_Arrondi_cash_Dame_Blanche
À effectuer (EV-06 : situation rejouée, requête construite, lignes financials et ticket imprimé).
## ROUND2_Arrondi_cash_Tapas_Variation
À effectuer (EV-06 : situation rejouée, requête construite, lignes financials et ticket imprimé).
## ROUND3_Arrondi_cash_vente_combinee
À effectuer (EV-06 : situation rejouée, requête construite, lignes financials et ticket imprimé).
## ROUND4_Cheques_repas_puis_cash
À effectuer (EV-06 : situation rejouée, requête construite, lignes financials et ticket imprimé).
## ROUND5_Multi_paiements
À effectuer (EV-06 : situation rejouée, requête construite, lignes financials et ticket imprimé).
# Advanced Set v1.6 (5 UC rejoués)
## UC230_signSale_Correction
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## UC4109_NoPayment
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## UC4119_SplitPayment
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## UC4121_Invoice
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## UC4122_Invoice_refund
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).

# Bons à usage multiple de l'Advanced Set v1.6 (3 UC rejoués)

Les trois UC de bon cadeau à usage multiple sont rejoués côté moteur par `crates/pos-engine/tests/spf_e2e_gift_vouchers.rs` (écarts expliqués dans *BONS_CADEAUX_FR.md* §9). Les bons à usage unique (UC270, UC271) et la conversion d'un bon en espèces (UC4105) ne sont pas offerts en v1.0.0.

## UC260_MPV_Sale
Vente d'un bon de 50,00 EUR payée par carte. Rejeu applicatif côté moteur : `uc260_mpv_sale`, deux écarts déclarés (produit et département du bon). À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture sur le terminal émulé).
## UC261_MPV_Use
Utilisation du bon pour un menu, le reste en espèces. Rejeu applicatif côté moteur : `uc261_mpv_use`, deux écarts déclarés (produit composé géré comme un produit simple). À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture sur le terminal émulé).
## UC262_MPV_Use
Utilisation du bon pour une commande mixte à emporter. Rejeu applicatif côté moteur : `uc262_mpv_use`, un écart déclaré (taux de la commande entière). À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture sur le terminal émulé).

# Récapitulatif

| Famille | UC (spec) | UC dans le périmètre v1.0.0 | Lecture des fixtures | Rejeu applicatif (build `cert`) |
|---|---|---|:-:|:-:|
| Basic Set v1.8 | 22 | 21 (hors M112A) | À effectuer (EV-06) | À effectuer (EV-06) |
| Reports v1.3 | 5 | 5 | À effectuer (EV-06) | À effectuer (EV-06) |
| Advanced Set v1.6 (factures) | 2 | 2 | À effectuer (EV-06) | À effectuer (EV-06) |
| Advanced Set v1.6 (bons à usage multiple) | 3 | 3 | À effectuer (EV-06) | À effectuer (EV-06) |
| **Total** | **32** | **31** | À effectuer (EV-06) | À effectuer (EV-06) |

Pour le détail des couvertures, voir COUVERTURE_FIXTURES_FR.md, COUVERTURE_RAPPORTS_FR.md et COUVERTURE_ADVANCED_SET_FR.md.
