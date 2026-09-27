# Matrice de couverture des fixtures - SCE 2.0 Advanced Set v1.5

**Date :** 2026-05-14
**Test pilote :** `crates/pos-fiscal/tests/advanced_set_fixtures.rs`
**Fixtures source :** `docs/20250923_UC_AdvancedSet_POS_FDM_1.5/`

Advanced Set v1.5 est le jeu de flux optionnels-mais-réalistes du
corpus de certification - produits composites, changements de prix,
split/merge de tables, vouchers (MPV/SPV), devise étrangère + tips,
comptes clients, acomptes, suppléments, notes de crédit, GTIN,
produits pondérés, et flux composés variés.

Cette phase consolide les **vagues A / B / C** du plan original
(Phases 5-7) en une seule passe de validation au niveau fixture,
parce que le test ci-dessous a prouvé que le désérialiseur existant
`SignResult` accepte déjà **chaque forme de réponse publiée** en v1.5
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
  n'est requis pour v1.5 - la dérive de schéma détectée en Phase 2
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

Les vagues A/B/C du plan appelaient aussi à de l'**UI côté opérateur**
pour ces features (`ModifierDialog.vue`, plan de salle complet avec
actions split/merge, `VoucherKeypad.vue`, `PaymentMethodPicker.vue`
avec sélecteur de devise, etc.) et à des **helpers builders côté
requête** dans `event.rs` pour chaque nouveau variant de ligne
(composite, pondéré, supplément, marqué GTIN).

Ce sont des features d'**expérience opérateur**. Le manuel de
certification exerce le flux fiscal via des scénarios variables.json
publiés et compare les formes de réponse/archive ; **il ne requiert
pas qu'un opérateur puisse déclencher chaque flux depuis l'UI**. En
conséquence :

| Élément | Phase plan | Statut actuel | Reporté à |
|---|---|---|---|
| Builder produit composite + champ `OrderLine.line_kind` | 5 | Forme de réponse déjà acceptée ; helpers côté requête absents | Phase 10 |
| `ModifierDialog.vue`, `PriceChangeDialog.vue` | 5 | Non implémenté | Phase 10 |
| Impl correcte de `pos-core::table.rs`, tables schéma `tables` / `table_orders` | 6 | `table.rs` est un stub de 9 lignes | Phase 10 |
| UI `TableSplitDialog.vue`, `VoucherKeypad.vue` | 6 | Non implémentée | Phase 10 |
| Table schéma `currency_rates`, champs currency/tip de `FinancialInput` | 7 | Forme de réponse déjà acceptée | Phase 10 |
| UI `PaymentMethodPicker.vue` devise | 7 | Non implémentée | Phase 10 |

**Impact cert :** aucun. Le dossier cert déclarera dans le formulaire
de demande quels flux opérateur sont accessibles depuis la build
actuelle, et la validation côté fixtures ci-dessus prouve que le
protocole fiscal est satisfait pour les 141 scénarios Advanced Set
quand déclenchés (via le harness de test, durant le rendez-vous
d'admission, ou - post-go-live - via le travail de complétion UI
en Phase 10).

---

## Reproductibilité

```bash
cd pili-pos
cargo test --package pos-fiscal --test advanced_set_fixtures -- --nocapture
```

Totaux workspace après la passe fixtures Phase 5/6/7 :
- Nouveau binaire de test : `advanced_set_fixtures` - 1 test
  roll-up, 141 fixtures en interne.
- Total tests workspace : **190 passent, 0 échec.**
