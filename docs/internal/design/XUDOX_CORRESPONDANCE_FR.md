# Xudox vers Pili : correspondance des gestes de caisse

Source : manuel utilisateur Xudox v2.01 (février 2015, 46 pages). Xudox est la caisse de la plupart des restaurants
chinois de Belgique. Le but de cette note : qu'un serveur ou un patron habitué à Xudox retrouve ses gestes sur Pili,
et savoir ce qui reste à adapter. Pili garde les habitudes (taper le code du menu, multiplier), pas l'écran de Xudox :
l'interface reste celle de Pili, propre et moderne.

Légende : **OK** existe dans Pili, **Nouveau** ajouté par la branche `feat/code-entry-pad`, **Partiel** existe sous une
autre forme, **Manque** pas d'équivalent aujourd'hui, **Non** volontairement absent (loi SCE 2.0 ou hors périmètre).

## 1. Prise de commande

| Xudox | Ce que ça fait | Pili | Statut |
| --- | --- | --- | --- |
| KeyB (clavier "Please input", touches 0-9, A-Z, OK) | taper le numéro du plat au lieu de chercher dans les catégories | pavé "Saisie code" ancré sous la grille (300 px, la grille garde ses 5 colonnes) ; la grille se filtre pendant la frappe, `A12` puis OK ou toucher le plat | Nouveau |
| Touche `(*)乘` du clavier, boutons 1 à 8 en bas | multiplier le plat | touche "× Qté" : `3 × A12` ou `A12 × 3` ; `3 ×` puis toucher un plat dans la grille | Nouveau |
| Touche `(-)减`, bouton `-1` | retirer une portion d'un plat déjà sur la note | bouton − sur la ligne de la note (pas de retrait par code : une seule façon de retirer) | OK |
| Réglage "Keyboard" : le clavier s'ouvre seul à chaque nouvelle table | | le pavé reste ouvert ou fermé selon le dernier choix, par terminal | Nouveau |
| Clavier physique du PC | | sur le pavé ouvert : chiffres, lettres, `*`, Entrée = OK, Retour arrière, Échap = effacer | Nouveau |
| Boutons catégories (Menu, Soep, Voor, Hoofd...) + "Touch panel" | | onglets de catégories + grille de plats | OK |
| Affichage par numéro | | Réglages, Affichage des produits : Nom / Code | OK |
| "Combain" : un même plat tapé deux fois reste sur une ligne | | fusion automatique des lignes identiques | OK |
| `+`, `−`, Verwijderen | | boutons + / − / supprimer sur la ligne | OK |
| "Show Chinese" | | langue de l'interface 中文, noms des plats en chinois | OK |
| "Met 伴随", "Separate 分割线" (lignes pointillées sur le bon cuisine) | séparer les services | "Service en plusieurs temps" (Entrées, Plats, Desserts) et "Envoyer la suite" | Partiel |
| "Beep sound" à chaque ajout | | message vert sous le pavé ; pas de son | Manque |
| "Favorite 顶置" (plats favoris en première page) | | pas de page de favoris | Manque |

## 2. Cuisine et bar

| Xudox | Pili | Statut |
| --- | --- | --- |
| BAR (boissons au bar), Chief / Keuken (cuisine) | "Envoyer en cuisine" imprime bar et cuisine selon le plat | OK |
| Keuken : Soep / Voor / Hoofd séparément | services (cours) + "Envoyer la suite" | Partiel |
| "WOK" (rappel du menu à volonté non saisi) | | Manque |
| "Aperitif" (la cuisine voit que la table est à l'apéritif) | | Manque |
| "Print one by one" (un bon par plat) | | Manque |
| Notes sur un plat ("Met" + texte) | note cuisine par ligne (60 caractères) | OK |

## 3. Tables

| Xudox | Pili | Statut |
| --- | --- | --- |
| "Tafels" + plan de salle | écran Tables, plan de salle configurable (1 à 999) | OK |
| Numéros de table avec lettre de zone (A-19, B-30) | numéros seulement | Manque |
| "Wisselen T" / "Change T" (changer de table) | Touche « Table 4 » de l'en-tête, ou « Déplacer » (Plus, ou plan de salle) : une table libre déplace la commande aussitôt, message « Annuler » 5 s (événement P M122) | OK |
| "Splitting table" (déplacer des plats vers une table T-1_B) | paiement en plusieurs parts au paiement, pas de scission de note | Manque |
| "Delete 去除本台" (supprimer la note) | "Client parti sans payer" avec PIN gérant ; une vente n'est jamais effacée | Non (loi : aucune suppression) |

## 4. Paiement et remises

| Xudox | Pili | Statut |
| --- | --- | --- |
| Rekening : cash, carte, AmEx | Payer : espèces, carte (terminal lié ou manuel), chèques-repas, bon cadeau | OK |
| "Pro forma / Addition 结账印单" | "Addition" (événement P) | OK |
| "Bill type" : passer de cash à carte après le ticket | correction de paiement (M132, PIN gérant) | OK |
| "Kort" -5 / -10 / -20 %, % libre | remise 5 / 10 / 15 / 20 / 50 %, % ou montant libre (PIN gérant) | OK |
| "Gratis / Offert" | "Offrir" la ligne (PIN gérant) | OK |
| "Calculator" (rendu monnaie) | rendu calculé à l'écran de paiement | OK |
| "2H / 3H / Self defined end" (fin du buffet sur la note) | | Manque |
| "Takeaway %" (prix emporter moins cher), "Drink no discount" | TVA emporter (C) ; pas de remise emporter automatique | Manque |

## 5. Emporter, téléphone, réservation

| Xudox | Pili | Statut |
| --- | --- | --- |
| "Meeneem / Take away 外卖" | "À emporter" | OK |
| Tel-S / Tel-E / WM-E (plages de numéros salle, livraison, emporter) | | Manque |
| "Phone" (fiche client : nom, adresse, téléphone) | clients B2B pour factures seulement | Manque |
| "Order" (réservations) | | Non (hors périmètre caisse) |

## 6. Personnel, rapports, base de données

| Xudox | Pili | Statut |
| --- | --- | --- |
| Mot de passe 4 caractères, comptes serveurs | PIN, badge optionnel | OK |
| Staff management Start / End | pointage (événements S) à la connexion et en fin de service | OK |
| Rapport X / Z financier, X / Z utilisateur | Rapports X / Z (obligatoires SCE 2.0) | OK |
| "Account" (listing mensuel) | rapport des ventes, export comptable | OK |
| Tiroir-caisse (mot de passe) | ouverture tiroir (événement F M131) | OK |
| Database : catégories, plats, TVA A/B/C/D, Append / Replace / Move | éditeur de menu (Réglages) ou catalogue HubRise | OK |

## 7. La saisie par code dans Pili (détail)

- Le code est celui du menu imprimé (`menu_items.code`, venu du catalogue). La comparaison ignore la casse et les
  espaces.
- Pendant la frappe, la grille montre les plats de toutes les catégories dont le code commence par le texte tapé (codes
  courts d'abord, puis A2 avant A10). Le plat que OK prendra est entouré : le code exact, sinon le seul résultat. Un
  appui sur un résultat l'ajoute directement ; un onglet de catégorie efface le code et revient à la carte.
- Le champ affiche la quantité dans une pastille ("3 ×") et a un bouton ✕ pour tout effacer ; la ligne sous le champ dit
  ce que OK va faire (plat et prix, nombre de résultats, ou l'erreur).
- Le nombre avant `×` est toujours la quantité (`12 × 3` = douze fois le plat 3). `A12 × 3` marche aussi. La quantité
  sur le pavé s'applique aussi au plat touché dans la grille.
- Quantité de 1 à 99. Un plat pesé ou à options ne prend pas de multiplicateur (message "Ce produit s'ajoute un par
  un") ; un plat à prix libre ouvre la saisie du prix puis prend la quantité.
- Deux plats avec le même code : refus "Plusieurs produits ont le code ..." (à corriger dans le catalogue).
- En mode Épuisé ou Allergènes, le code ouvre la fiche du plat comme un appui sur la grille.
- Rien de fiscal ne change : le pavé passe par la même commande que la grille (`add_order_line`) et lit le menu par
  `get_menu_items` sans catégorie. Retirer un plat reste le rôle de la note (boutons − et supprimer).
- Code : `ui/src/utils/codeEntry.ts`, `ui/src/components/CodePad.vue`, `OrderView.vue` ;
  `android/.../feature/order/CodeEntry.kt`, `CodeSearch.kt`, `CodePadFlow.kt`, `CodePad.kt`, `CodePadField.kt`.

## 8. Suites proposées, par ordre d'impact pour un habitué Xudox

1. Bip à l'ajout (réglage par terminal), comme "Beep sound".
2. Numéros de table avec lettre de zone (A-19) : touche le plan de salle et le type des tables, à étudier.
3. Scission de note (déplacer des plats vers une seconde note) : UC Advanced Set "table split", événements à vérifier.
4. Plages de numéros emporter / livraison et fiche client téléphone.
5. Favoris en tête de grille.
6. Heure de fin de buffet (2H / 3H) imprimée sur l'addition, et rappel "WOK" pour l'à volonté.
