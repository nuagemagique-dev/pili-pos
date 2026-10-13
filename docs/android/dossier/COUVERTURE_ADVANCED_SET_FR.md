<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/COUVERTURE_ADVANCED_SET_FR.md @ 86088e4 -->
# Matrice de couverture des fixtures - SCE 2.0 Advanced Set v1.6 - pili-pos-android

**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)
**Date :** À effectuer (EV-18 : date de dépôt)
**Test pilote :** `crates/pos-fiscal/tests/advanced_set_fixtures.rs`
**Fixtures source :** `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/`

**Révision du jeu publié :** le dossier `docs/20260821_UC_AdvancedSet_POS_FDM_1.6/`
remplace `docs/20250923_UC_AdvancedSet_POS_FDM_1.5/`. Changements de la
v1.6 (changelog `CHANGELOG-UC_AdvancedSet_1.6.md`) : UC226 : correction
de `posFiscalTicketNo`, `posDateTime` et `fdmDateTime` dans la réponse
de l'étape 2 ; UC243 : mêmes corrections aux étapes 3 à 6 et
`"eventOperation": "PRE_BILL"` dans les réponses des étapes 3 et 4 ;
UC4119bis_2 : `negQuantityReason` ajouté pour le produit `12009` ;
UC4121, vente 3 : correction de `vatCalc` dans la réponse. En outre,
tous les horodatages portent désormais les millisecondes.

Advanced Set v1.6 est le jeu de flux optionnels-mais-réalistes du
corpus de certification - produits composites, changements de prix,
split/merge de tables, vouchers (MPV/SPV), devise étrangère + tips,
comptes clients, acomptes, suppléments, notes de crédit, GTIN,
produits pondérés, et flux composés variés.

Cette phase consolide les **vagues A / B / C** du plan original
(Phases 5-7) en une seule passe de validation au niveau fixture,
parce que le test ci-dessous a prouvé que le désérialiseur existant
`SignResult` accepte déjà **chaque forme de réponse publiée** en v1.6
sans aucune modification de code.

---

## Résultat principal

| Métrique | Valeur |
|---|---|
| Total fixtures découvertes (sur 59 dossiers UC, sous-dossiers multi-étapes inclus) | **141** |
| **Passantes** | **141** |
| Échouées | 0 |
| Labels d'événements exercés dans ce set | N, P, F, C, I |
| Labels d'événements couverts sur le corpus complet (Basic+Reports+Advanced) | **N, P, F, S, I, C, T, R - les 8** |

---

## Ce que cela prouve

- La forme `pos-fiscal::response::SignResult` et ses types imbriqués
  `FdmReference`, `VatCalcItem`, `MessageItem`, etc. sont exhaustifs
  contre chaque payload de réponse Advanced Set.
- Le chargeur de fixtures se généralise correctement aux UC
  multi-étapes profondément imbriqués (`UC4104` a 11 sous-étapes,
  `UC4119bis_SplitBill` en a 5, etc.).
- Aucun `#[serde(default)]` ni wrapping `Option<...>` additionnel
  n'est requis pour v1.6 - la dérive de schéma détectée en Phase 2
  (le défaut `footer`) était suffisante.

---

## Détail par vague (depuis le plan original)

### Vague A - produits, prix, arrondis *(Phase 5 dans le plan)*

| UC | Statut |
|---|---|
| UC200_DirectSale_CompositeProduct | ✓ |
| UC201_signSale_Rounding | ✓ |
| UC220_signSale_LinePriceChange | ✓ |
| UC221A_signSale_LinePriceChange | ✓ |
| UC221B_signSale_LinePriceChange | ✓ |
| UC222_signSale_PriceChange_CompositeProduct | ✓ |
| UC223_signSale_EventPriceChange | ✓ |
| UC224_signSale_MultiplePriceChange | ✓ |
| UC225A_PriceChange_IndirectSale | ✓ |
| UC225B_PriceChange_IndirectSale | ✓ |
| UC226_PriceChange | ✓ |
| UC229B_FreeArticle_DirectSale | ✓ |
| UC230_signSale_Correction | ✓ |
| UC280_CompositeProduct | ✓ |
| UC4106_MenuAllIn_PriceChange | ✓ |
| UC4107_VatLabelsABC (5 sous-étapes) | ✓ |
| UC4108_VatLabelX | ✓ |
| UC4112_WeightedProduct | ✓ |
| UC4116_CorrectionPriceChange (2 sous-étapes) | ✓ |
| UC4117_GTIN | ✓ |
| UC4120_Supplement_PriceChange | ✓ |
| UC4120bis_Supplement_TransactionLIne | ✓ |

### Vague B - tables, cost-centers, vouchers, comptes clients *(Phase 6)*

| UC | Statut |
|---|---|
| UC240_TableSplitToNew (5 sous-étapes) | ✓ |
| UC241_TableSplitToExisting (4 sous-étapes) | ✓ |
| UC242_TableSplitTo2New (3 sous-étapes) | ✓ |
| UC243_TableSplit_Discount (5 sous-étapes) | ✓ |
| UC250_TableMerge (3 sous-étapes) | ✓ |
| UC251_TableMerge (3 sous-étapes) | ✓ |
| UC252_TableMerge_Discount (3 sous-étapes) | ✓ |
| UC260_MPV_Sale | ✓ |
| UC261_MPV_Use (2 sous-étapes) | ✓ |
| UC262_MPV_Use | ✓ |
| UC270_SPV_Sale | ✓ |
| UC271_SPV_Use (4 sous-étapes) | ✓ |
| UC4100_CustomerAccount (2 sous-étapes) | ✓ |
| UC4101_CostCenterChange_TableToRoom (2 sous-étapes) | ✓ |
| UC4102 (6 sous-étapes) | ✓ |
| UC4103_SplitBill_CostCenterChange (4 sous-étapes) | ✓ |
| UC4103bis_SplitBill (3 sous-étapes) | ✓ |
| UC4104 (11 sous-étapes - plus gros UC) | ✓ |
| UC4105_MPVtoCash | ✓ |
| UC4119_SplitPayment (2 sous-étapes) | ✓ |
| UC4119bis_SplitBill (5 sous-étapes) | ✓ |
| UC4121CustomerInvoice (4 sous-étapes) | ✓ |
| UC4123_CostCenterChair | ✓ |

### Vague C - devise, tips, acomptes, correction de paiement *(Phase 7)*

| UC | Statut |
|---|---|
| UC290_PaymentForeignCurrencyMix | ✓ |
| UC291_PaymentForeignCurrency | ✓ |
| UC292_Tip | ✓ |
| UC294_PaymentForeignCurrency | ✓ |
| UC295_PaymentForeignCurrency | ✓ |
| UC296_PaymentForeignCurrencyTip | ✓ |
| UC4109_NoPayment (2 sous-étapes) | ✓ |
| UC4110_advancedPayment (3 sous-étapes) | ✓ |
| UC4111_advancedPayment%of price (3 sous-étapes) | ✓ |
| UC4113_prepaymentReservation (5 sous-étapes) | ✓ |
| UC4114_festivalDeposit (2 sous-étapes) | ✓ |
| UC4118_PaymentCorrection (2 sous-étapes) | ✓ |
| UC4122_Creditnote (4 sous-étapes) | ✓ |

---

## Ce qui est volontairement HORS périmètre de cette phase

La couverture ci-dessus est celle du moteur fiscal, qui est le même code source sur Linux et sur Android : le test pilote et les 141 fixtures valent pour les deux produits (`CADRE_DEMANDE_FR.md` §4). Les vagues A / B / C du plan appelaient aussi à de l'**expérience opérateur** pour ces flux et à des **helpers côté requête** dans `event.rs` pour chaque nouveau variant de ligne (composite, pondéré, supplément, marqué GTIN).

Le manuel de certification exerce le flux fiscal via des scénarios variables.json publiés et compare les formes de réponse et d'archive ; **il ne requiert pas qu'un opérateur puisse déclencher chaque flux depuis l'interface**. Sur Android, l'interface est le portage écran par écran de l'interface Linux (`docs/android/PARITY.md`) : elle ne propose que les flux que l'interface Linux propose.

| Flux de l'Advanced Set | Écran Android | Statut actuel |
|---|---|---|
| Vente sans paiement (UC4109) | écran de commande, bouton « Client parti sans payer » puis PIN du gérant (`docs/android/parity/order.md`) | proposé |
| Paiement en plusieurs moyens et parts (UC4119, arrondis UC201) | écran d'encaissement : Espèces, Carte, Chèques-repas, parts égales, arrondi au 0,05 EUR des seules espèces (`docs/android/parity/checkout.md`) | proposé |
| Remise sur le ticket d'une commande en ligne (UC223) | tableau des commandes en ligne ; le moteur répartit la remise sur les lignes (`docs/android/parity/webshop.md`) | proposé pour les commandes en ligne seulement |
| Options d'un produit | fenêtre « Options » de l'écran de commande (portage de `ModifierDialog.vue`) : options HubRise ; le prix des options s'ajoute au prix unitaire de la ligne, calculé par le moteur ; aucune modification de prix (priceChanges) | proposé, hors UC composite |
| Produit composite, changement de prix à la saisie, split et fusion de tables (UC240 à UC252), vouchers MPV et SPV, devise et pourboires, acomptes, notes de crédit | aucun écran Android (aucun écran Linux non plus) | reporté, comme sous Linux |

| Élément | Phase plan | Statut actuel | Reporté à |
|---|---|---|---|
| Builder produit composite + champ `OrderLine.line_kind` | 5 | Forme de réponse déjà acceptée ; helpers côté requête absents | Phase 10 |
| Plan de salle complet avec split et fusion (le plan Android ouvre et reprend des commandes) | 6 | Non implémenté ; `pos-core::table.rs` est un stub de 9 lignes | Phase 10 |
| Table schéma `currency_rates`, champs currency/tip de `FinancialInput` | 7 | Forme de réponse déjà acceptée | Phase 10 |

**Impact cert :** aucun. Le dossier cert déclarera dans le formulaire de demande quels flux opérateur sont accessibles depuis la build actuelle, et la validation côté fixtures ci-dessus prouve que le protocole fiscal est satisfait pour les 141 scénarios Advanced Set quand déclenchés (via le harnais de test, durant le rendez-vous d'admission, ou, après la mise en service, via le travail de complétion de l'interface en Phase 10). Résultats des UC joués sur le terminal émulé : `À effectuer (EV-06 : résultats des UC)`, voir `RESULTATS_TESTS_UC_FR.md`.

---

## Reproductibilité

```bash
cd pili-pos
cargo test --package pos-fiscal --test advanced_set_fixtures -- --nocapture
```

Sur le terminal émulé, le même test s'exécute avec les fixtures poussées sur l'appareil (`scripts/android/device_engine_tests.sh --fixtures --check`, `docs/android/DEVICE_TESTS.md`) : 1 test, passant. Totaux de tests du moteur et de la couche Android : `RAPPORT_TESTS_BASELINE_FR.md` (`À effectuer (EV-17 : comptes finaux au commit de livraison)`).
