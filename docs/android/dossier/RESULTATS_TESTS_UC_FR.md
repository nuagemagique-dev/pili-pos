<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/RESULTATS_TESTS_UC_FR.md @ 86088e4 -->
# Résultats des tests UC - pili-pos-android 1.0.0

**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)
**Date :** À effectuer (EV-18 : date de dépôt)
**Référence :** §IV du manuel de certification (Procédure de communication GraphQL).

Squelette des résultats des UC obligatoires du Basic Set v1.8 (22 UC spec), des 5 UC Reports v1.3 et des cinq situations d'arrondi (ROUND1 à ROUND5), plus trois UC de l'Advanced Set v1.6 rejoués par le même harnais. Hors périmètre v1.0.0, chacun dans sa section : M150 (event I, voir DECISION_PEPPOL_AP_FR.md) et M132 ; M122 (transfert de table) est dans le périmètre ; M112A n'est pas un parcours proposé (parcours M112B). Aucun résultat n'est repris du dossier Linux : chaque bloc ci-dessous attend les résultats des UC joués sur le terminal émulé par la suite instrumentée (`À effectuer (EV-06 : résultats des UC sur le terminal émulé)`).

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
Hors périmètre v1.0.0. À effectuer (EV-06 : lecture de la fixture par le moteur sur le terminal émulé).
## M133_signMoneyInOut_DrawerDeclaration
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M140_signWorkIn
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M141_signWorkOut
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## M150_signInvoice
Hors périmètre v1.0.0 (voir DECISION_PEPPOL_AP_FR.md). À effectuer (EV-06 : lecture de la fixture par le moteur sur le terminal émulé).
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
# Advanced Set v1.6 (3 UC rejoués)
## UC230_signSale_Correction
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## UC4109_NoPayment
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).
## UC4119_SplitPayment
À effectuer (EV-06 : champs vérifiés, requête construite par le moteur, réponse du FDM et comparaison avec la fixture).

# Récapitulatif

| Famille | UC (spec) | UC dans le périmètre v1.0.0 | Lecture des fixtures | Rejeu applicatif (build `cert`) |
|---|---|---|:-:|:-:|
| Basic Set v1.8 | 22 | 19 (hors M112A, M132 et M150) | À effectuer (EV-06) | À effectuer (EV-06) |
| Reports v1.3 | 5 | 5 | À effectuer (EV-06) | À effectuer (EV-06) |
| **Total** | **27** | **24** | À effectuer (EV-06) | À effectuer (EV-06) |

Pour le détail des couvertures, voir COUVERTURE_FIXTURES_FR.md, COUVERTURE_RAPPORTS_FR.md et COUVERTURE_ADVANCED_SET_FR.md.
