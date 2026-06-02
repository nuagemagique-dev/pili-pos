# Lignes de produits - Catalogue prédéfini Tokyo Sushi Bar

Requis par le Manuel de certification SPF Finances §IV "Lignes de
Produits Prédéfinies". Il s'agit d'un instantané des tables
`menu_items` + `categories` de la base SQLite de seed de l'image
kiosk. Le format reprend le gabarit du manuel de certification : une
ligne par article, mappée à sa catégorie TVA.

## Catégories (dans l'ordre d'affichage)

| # | Catégorie (FR) | Catégorie (ZH) | Notes |
|---|---|---|---|
| 1 | Entrées | 前菜 | Entrées froides + chaudes |
| 2 | Sushi | 寿司 | Nigiri, maki, sashimi |
| 3 | Plats chauds | 热菜 | Plats cuisinés |
| 4 | Menus | 套餐 | Menus composites (UC280) |
| 5 | Desserts | 甜品 | |
| 6 | Boissons | 饮品 | TVA 21% |
| 7 | Alcools | 酒类 | TVA 21% |

## Mapping TVA (selon SCE-2.0)

- **A = 21%** - alcools, boissons rafraîchissantes
- **B = 12%** - services de restauration (consommation sur place)
- **C = 6%** - vente à emporter (nourriture)
- **D = 0%** - exempté
- **X = N/A** - hors champ

## Extrait - Catégorie Sushi (illustratif)

| Article | TVA (sur place / emporter) | Prix (€) | Commentaire |
|---|---|---|---|
| Saumon roll x8 | B / C | 8,50 | Roll froid |
| California roll x8 | B / C | 9,00 | Avocat, concombre |
| Sashimi mix 12 pcs | B / C | 14,50 | |
| Menu chef (sashimi+riz+miso) | B / C | 24,50 | Composite - UC280 |

(Table complète extraite de la DB de seed au moment du build ; l'image
kiosk embarque la liste canonique dans `pos.db`. Le relecteur peut la
dumper via :
`sqlite3 /var/lib/moyun-pos/pos.db ".mode csv" "SELECT * FROM menu_items_view"`.)

## Groupes de modificateurs (UC4106 changements de prix menu-all-in)

- **Sauce** (min 0, max 2) - Soja, Wasabi, Gingembre.
- **Cuisson** (min 1, max 1) - Bleu, Saignant, À point, Bien cuit.

Les prix des modificateurs s'intègrent au prix de la ligne, la TVA
hérite du parent.

## Déclaration de devise étrangère

Tokyo Sushi Bar accepte **uniquement l'EUR**. Le champ
`FinancialInput.currencyCode` est fixé à `"EUR"` et les UC de devise
étrangère (UC290-UC293) sont couverts par les tests mais désactivés
dans l'UI de production pour éviter toute erreur opérateur.

## Exemple chiffré de calcul de TVA

Commande :
- 1× Saumon roll x8 (sur place) = 8,50 € TVA B
- 1× Coca 33cl = 3,50 € TVA A

| TVA | Brut | Net | Taxe |
|---|---|---|---|
| A (21%) | 3,50 | 2,89 | 0,61 |
| B (12%) | 8,50 | 7,59 | 0,91 |
| **Total** | **12,00** | **10,48** | **1,52** |

Imprimé sur le ticket en tant que bloc récapitulatif TVA selon
POS v2.2 §III.3.4.
