# Lignes de produits prédéfinies - machine de test

Réponse au §III "Lignes de produit (Product Lines)" du manuel de
certification (p. 29-30). La machine de test livrée à l'Intake est
initialisée avec **exactement** la liste de produits du manuel, afin
que les UC du Basic Set, des Reports et de l'Advanced Set puissent être
rejoués tels que définis (les fixtures référencent ces `productId`).

- Source : `src-tauri/seed_spf_test.sql`, appliqué à la construction de
  l'image d'admission (cf. `CHECKLIST_DEPLOIEMENT_KIOSK_FR.md`).
- Tables : `menu_items` (`id` = `productId`, `category_id` =
  `departmentId`, `price_cents`, `vat_code` sur place) + `categories` +
  `translations` (libellés FR/EN/NL/DE).
- En exploitation, chaque établissement charge son propre catalogue ;
  le binaire `moyun-pos` est identique (le catalogue est de la
  donnée, pas du code).

## Liste chargée

| Disponible | productId | Nom du produit | Département | Prix (€) | Unité | TVA sur place | TVA à emporter |
|---|---|---|---|---|---|---|---|
| ☑ | 10006 | Dry Martini | 10 Aperitifs | 12,00 | PIECE | A | A |
| ☑ | 12007 | Perrier | 12 Soft Drinks | 2,50 | PIECE | A | C |
| ☑ | 12009 | Coca Cola | 12 Soft Drinks | 3,04 | PIECE | A | C |
| ☑ | 15001 | Bourgogne Red | 15 Wines | 35,00 | PIECE | A | A |
| ☑ | 15002 | Chardonnay White | 15 Wines | 40,00 | PIECE | A | A |
| ☑ | 15003 | Wine (glass) | 15 Wines | 8,00 | PIECE | A | A |
| ☑ | 15023 | Matching Wines | 15 Wines | 24,00 | PIECE | A | A |
| ☑ | 20001 | Carpaccio Beef | 20 Starters | 20,00 | PIECE | B | C |
| ☑ | 22001 | Burger of the Chef | 22 Main Dishes | 28,00 | PIECE | B | C |
| ☑ | 22005 | Catch of the day | 22 Main Dishes | 45,00 | KG | B | C |
| ☑ | 22010 | Dish of the day | 22 Main Dishes | prix libre | PIECE | B | C |
| ☑ | 25003 | Dame Blanche | 25 Desserts | 11,52 | PIECE | B | C |
| ☑ | 25008 | Crème brûlée | 25 Desserts | 10,00 | PIECE | B | C |
| ☑ | 28007 | Tapas variation | 28 Tapas | 3,34 | PIECE | B | C |
| ☑ | 30001 | Business menu (*) | 30 Menus | 38,00 | PIECE | B | C |
| ☑ | 30010 | Business menu all-in (**) | 30 Menus | 58,00 | PIECE | A&B | A&C |
| ☑ | 30020 | Menu drink (***) | 30 Menus | 12,00 | PIECE | A | A&C |
| ☑ | 40001 | Gift Voucher General | 400 Vouchers | 50,00 | PIECE | X | X |
| ☑ | 40002 | Gift Voucher General | 400 Vouchers | 100,00 | PIECE | X | X |
| ☑ | 40003 | Gift Voucher General | 400 Vouchers | 150,00 | PIECE | X | X |
| ☑ | 40004 | Gift Voucher General | 400 Vouchers | 250,00 | PIECE | X | X |
| ☑ | 40020 | Voucher Special Diner Offer | 400 Vouchers | 75,00 | PIECE | B | C |
| ☑ | 90001 | Bottle S | 90 Bottle Deposits | 0,01 | PIECE | X | X |
| ☑ | 90100 | Beer bin / keg beer | 90 Bottle Deposits | 4,49 | PIECE | X | X |
| ☑ | 500001 | Cigarettes (20 pcs) | 500 Cigarettes | 8,40 | PIECE | D | D |

(*) = 1 Carpaccio (EUR 15) + 1 Burger (EUR 23)
(**) = 1 Carpaccio (EUR 16) + 1 Burger (EUR 24) + 1 Matching Wines (EUR 18)
(***) = 1 Aperitif + 1 Soft Drink

## Codes TVA

- **A = 21%** - alcools, boissons rafraîchissantes
- **B = 12%** - services de restauration (consommation sur place)
- **C = 6%** - vente à emporter (nourriture, boissons non alcoolisées)
- **D = 0%** - taux zéro (cigarettes)
- **X = N/A** - hors champ (bons d'achat multi-usage, consignes)

`menu_items.vat_code` porte le code **sur place** ; le passage en
vente à emporter recalcule le code par ligne (`order_lines.vat_code`)
selon la colonne « à emporter » ci-dessus. Les menus composites
(30010, 30020) sont ventilés par composant, chaque composant portant
son propre code TVA.

## Vérification sur la machine de test

```bash
sqlite3 -header -column /var/lib/moyun-pos/pos.db \
  "SELECT m.id AS productId, t.value AS nom, m.category_id AS dept,
          m.price_cents, m.vat_code
   FROM menu_items m
   JOIN translations t ON t.entity_type = 'menu_item'
                      AND t.entity_id = m.id
                      AND t.field = 'name' AND t.lang = 'en'
   ORDER BY m.id;"
```

## Devise

Le POS fonctionne **uniquement en EUR**. Le champ
`FinancialInput.currencyCode` est fixé à `"EUR"` ; les UC de devise
étrangère (UC290-UC293) sont couverts par les tests mais désactivés
dans l'interface.

## Exemple chiffré de calcul de TVA (M110)

Commande sur place : 2× Dry Martini (10006) + 1× Burger of the Chef
(22001), payée en espèces.

| TVA | Brut | Net | Taxe |
|---|---|---|---|
| A (21%) | 24,00 | 19,83 | 4,17 |
| B (12%) | 28,00 | 25,00 | 3,00 |
| **Total** | **52,00** | **44,83** | **7,17** |

Imprimé sur le ticket en tant que bloc récapitulatif TVA (cf.
`TICKETS_EXEMPLES_FR.md`).
