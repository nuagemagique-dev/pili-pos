<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/LIGNES_PRODUITS_FR.md @ 86088e4 -->
# Lignes de produits prédéfinies - machine de test

**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)

Réponse au §III "Lignes de produit (Product Lines)" du manuel de
certification (p. 29-30). La machine de test livrée à l'Intake est
initialisée avec **exactement** la liste de produits du manuel, afin
que les UC du Basic Set, des Reports et de l'Advanced Set puissent être
rejoués tels que définis (les fixtures référencent ces `productId`).

- Source : `src-tauri/seed_spf_test.sql`, le même fichier que sous
  Linux. Sur Android il n'est plus appliqué à la construction d'une
  image : il voyage sous le nom `seed.sql` dans le lot de mise en service
  signé (graine facultative du jeu de test du SPF, `docs/android/APP.md`
  §3) et le terminal l'applique une seule fois, à une base qui n'existe
  pas encore, à l'import du lot (`MANUEL_INSTALLATION_FR.md` §2.1 et §3.4).
- Tables : `menu_items` (`id` = `productId`, `category_id` =
  `departmentId`, `price_cents`, `vat_code` sur place, `active`,
  `open_price`) + `categories` + `translations`. Les noms des produits
  et des départements sont ceux du jeu de test SPF, à l'identique, dans
  les quatre langues de l'écran (FR/EN/NL/DE) : ce sont les
  `productName` et `departmentName` envoyés au FDM.
- En exploitation, chaque établissement charge son propre catalogue
  (graine de son lot, ou HubRise, `MANUEL_PROGRAMMATION_FR.md` §4) ;
  l'APK `pili-pos-android` est identique (le catalogue est de la
  donnée, pas du code).

## Liste chargée

9 des 25 produits du manuel exercent des fonctionnalités **déclarées
non proposées à la vente en v1.0.0** (prix au poids, produits composés,
vouchers mono-usage ; les quatre « Gift Voucher General » sont remplacés par
le produit système « Bon cadeau » à montant libre - cf.
REPONSES_MANUEL_CERT_FR.md §III.F "Fonctionnalités avancées" et
*BONS_CADEAUX_FR.md*). Ils restent en base (`active = 0`) pour la
traçabilité du `productId` mais ne sont **pas proposés à la vente** ;
les départements 30 (Menus) et 400 (Vouchers) n'ont donc plus aucun
produit disponible et sont masqués dans la grille (`categories.active
= 0` pour ces deux départements). Voir `src-tauri/seed_spf_test.sql`.

Le produit `22010` ("Plat du jour") est en **prix libre**
(`open_price = 1`) : le prix n'est pas fixé au catalogue, il est saisi
par l'opérateur à l'ajout au panier. Il reste **disponible** en
v1.0.0 - le prix libre est implémenté (cf. REPONSES_MANUEL_CERT_FR.md
§III.F).

| Disponible | productId | Nom du produit | Département | Prix (€) | Unité | TVA sur place | TVA à emporter | Motif si non disponible |
|---|---|---|---|---|---|---|---|---|
| ☑ | 10006 | Dry Martini | 10 Aperitifs | 12,00 | PIECE | A | A | |
| ☑ | 12007 | Perrier | 12 Soft Drinks | 2,50 | PIECE | A | C | |
| ☑ | 12009 | Coca Cola | 12 Soft Drinks | 3,04 | PIECE | A | C | |
| ☑ | 15001 | Bourgogne Red | 15 Wines | 35,00 | PIECE | A | A | |
| ☑ | 15002 | Chardonnay White | 15 Wines | 40,00 | PIECE | A | A | |
| ☑ | 15003 | Wine (glass) | 15 Wines | 8,00 | PIECE | A | A | |
| ☑ | 15023 | Matching Wines | 15 Wines | 24,00 | PIECE | A | A | |
| ☑ | 20001 | Carpaccio Beef | 20 Starters | 20,00 | PIECE | B | C | |
| ☑ | 22001 | Burger of the CHEF | 22 Main Dishes | 28,00 | PIECE | B | C | |
| ☐ | 22005 | Catch of the day | 22 Main Dishes | 45,00 | KG | B | C | Prix au poids : non supporté en v1.0.0 (pas de balance intégrée, cf. §III.F "Prix au poids") |
| ☑ | 22010 | Dish of the day | 22 Main Dishes | prix libre (`open_price`) | PIECE | B | C | |
| ☑ | 25003 | Dame Blanche | 25 Desserts | 11,52 | PIECE | B | C | |
| ☑ | 25008 | crème brûlée | 25 Desserts | 10,00 | PIECE | B | C | |
| ☑ | 28007 | Tapas variation | 28 Tapas | 3,34 | PIECE | B | C | |
| ☐ | 30001 | Business menu | 30 Menus | 38,00 | PIECE | B | C | Produit composé : non supporté en v1.0.0 (fixtures parsent, UI non implémentée, cf. §III.F "Produits composés" / Advanced Set UC280) |
| ☐ | 30010 | Business menu all-in | 30 Menus | 58,00 | PIECE | A&B | A&B | Idem (produit composé ; un seul label TVA en base, `B`) |
| ☐ | 30020 | Menu drink | 30 Menus | 12,00 | PIECE | A | A | Idem (produit composé) |
| ☐ | 40001 | Gift Voucher General | 400 Vouchers | 50,00 | PIECE | X | X | Remplacé par le produit système « Bon cadeau » (montant saisi à la vente, rayon `GV`, T.V.A. X) : UC260 à UC262 rejoués (cf. §III.F "Vouchers à but multiple", *BONS_CADEAUX_FR.md*) |
| ☐ | 40002 | Gift Voucher General | 400 Vouchers | 100,00 | PIECE | X | X | Idem |
| ☐ | 40003 | Gift Voucher General | 400 Vouchers | 150,00 | PIECE | X | X | Idem |
| ☐ | 40004 | Gift Voucher General | 400 Vouchers | 250,00 | PIECE | X | X | Idem |
| ☐ | 40020 | Voucher Special Diner Offer | 400 Vouchers | 75,00 | PIECE | B | B | Voucher à but unique : non offert en v1.0.0 (cf. §III.F "Vouchers à but unique", UC270/271) |
| ☑ | 90001 | Bottle S | 90 Bottle Deposits | 0,01 | PIECE | X | X | |
| ☑ | 90100 | Beer bin/ keg beer | 90 Bottle Deposits | 4,49 | PIECE | X | X | |
| ☑ | 500001 | Cigarettes (20 pcs) | 500 Cigarettes | 8,40 | PIECE | D | D | |

**Consignes (90001/90100) :** restent **disponibles** en v1.0.0 -
`active = 1` dans le seed, non listées dans les fonctionnalités
déclarées non supportées de REPONSES_MANUEL_CERT_FR.md §III.F. Les
deux tableaux sont donc cohérents ; aucune correction croisée requise.

**Note sur la colonne TVA "à emporter" :** `menu_items` ne porte
qu'un seul `vat_code` (contrainte `CHECK(vat_code IN ('A','B','C','D','X'))`,
cf. `crates/pos-db/src/schema.rs`) - les produits composés (30001,
30010, 30020) ne sont donc pas ventilés par composant TVA en base ;
c'est précisément pourquoi ils sont déclarés non supportés en v1.0.0
(§III.F "Produits composés"). Les codes "à emporter" ci-dessus
reflètent la table de recalcul `menu_items.vat_code_takeaway` du seed
(NULL = même code qu'en salle).

## Reste du jeu de test SPF

Le lot de la machine de test reprend aussi le reste de
`docs/20260821_UC_Reports_1.3/UC_TestSetup_20240806.md` :

| Élément du jeu de test | Dans le lot de la machine de test |
|---|---|
| Établissement | "The FPS Finances Experience", Boulevard Albert II 33, 1030 Brussels, TVA `BE0000000097`, unité d'établissement `2000000042` (`pos.json` du lot, `MANUEL_INSTALLATION_FR.md` §4.2) |
| POS de la machine de test | `pos_id` `CPOS0031234567`, `terminal_id` `POS-2-ANDROID` ; `device_id` = numéro de série matériel du terminal (FAQ producteurs Q505 b), lu à chaque ouverture du moteur (`PLATEFORME_ANDROID_FR.md` §4). Valeur de la machine de test : `EMULATOR37X2X12X0` (profil émulé, capture A82) |
| Personnel | John (barman, `75061189702`), Elisa (chef de salle, `84022899837`), Sammy (sommelier, `80113078968`), rôle Serveur, avec les PIN de REPONSES_MANUEL_CERT_FR.md ; s'y ajoutent les comptes Technicien, Gérant et Apprenti |
| Tables | 25 tables (`table_count`), BAR 1 à 4, DINING ROOM 5 à 15, TERRACE 16 à 25 |
| Tiroir | un tiroir, sur le POS de la machine de test |
| Moyens de paiement | espèces (EUR), carte (un moyen "Card", type `CARD_DEBIT`), chèques-repas électroniques, bons cadeaux (ligne `VOUCHER_STORE`, *BONS_CADEAUX_FR.md*) |

Non reproduit, car hors périmètre v1.0.0 (REPONSES_MANUEL_CERT_FR.md
§III.F) : le second POS `CPOS0031234568` de la réception et son PMS
hôtel, les terminaux et handhelds (TER-1 à TER-5), la table haute du
bar (8 chaises, sans numéro), les chaises comme centres de coût, les
devises GBP/USD, les bons d'achat à usage unique, la distinction des réseaux de cartes (Visa, Mastercard, Diners, American Express) et Payconiq / Apple Pay.

## Lignes d'un transfert de table (M122)

Un changement de table (`signCostCenterChange`) reprend les lignes de
la commande : le côté `transfer.from` les porte en quantité négative
(motif `COST_CENTER_CHANGE`, table d'origine), le côté `transfer.to`
les porte en quantité positive (table de destination). Le prix, le
code TVA et la remise portée par une ligne sont conservés tels quels
(même `groupingId`) ; rien n'est recalculé.

## Lignes d'une vente remisée à la caisse

Une remise saisie à la caisse par un Gérant (pourcentage, montant ou
« Offert ») est signée comme un `priceChanges`, jamais comme une ligne
supplémentaire du ticket. Une remise de commande est de portée `EVENT`
(identifiant `TILL-DISCOUNT`), répartie sur les lignes au centime près ;
un article offert est de portée `LINE` (`TILL-FREE-LINE`, ou `TILL-FREE`
pour toute la commande) et sa ligne est signée à 0,00. Chaque changement
est une paire dans le journal (`PRICE_CHANGE`) : la pose et, le cas
échéant, son retrait, qui rétablit le prix d'origine. Le motif n'est
pas dans la transaction signée.

## Notes de cuisine (hors données produit)

La note qu'un membre du personnel écrit sur une ligne (« sans wasabi »)
n'est pas une donnée de produit : elle ne fait partie ni du nom du
produit, ni des modificateurs, ni d'aucune ligne de transaction signée
(réponse de l'administration aux producteurs, FAQ Q210 b). Elle reste dans
la colonne locale `order_lines.kitchen_note` (migration 31), n'entre pas
dans le journal `order_entries` et ne part qu'à l'imprimante du bar ou de
la cuisine.

## Codes TVA

- **A = 21%** - alcools, boissons rafraîchissantes
- **B = 12%** - services de restauration (consommation sur place)
- **C = 6%** - vente à emporter (nourriture, boissons non alcoolisées)
- **D = 0%** - taux zéro (cigarettes)
- **X = N/A** - hors champ (vente d'un bon d'achat multi-usage, consignes)

`menu_items.vat_code` porte le code **sur place** ; le passage en
vente à emporter recalcule le code par ligne (`order_lines.vat_code`)
selon la colonne « à emporter » ci-dessus (`menu_items.vat_code_takeaway`).

## Vérification sur la machine de test

Le terminal n'offre ni console ni `sqlite3`. La liste se contrôle sur
la copie de la base écrite par l'export du Gérant (« Export pour le
SPF », `SECURITE_BD_FR.md` §8.2), ouverte sur l'ordinateur du
contrôleur :

```bash
sqlite3 -readonly -header -column pos.db \
  "SELECT m.id AS productId, t.value AS nom, m.category_id AS dept,
          m.price_cents, m.vat_code, m.open_price, m.active
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
