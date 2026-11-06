# Xudox vers Pili : correspondance des gestes de caisse

Source : manuel utilisateur Xudox v2.01 (février 2015, 46 pages). Xudox est la caisse de la plupart des restaurants
chinois de Belgique. Le but de cette note : qu'un serveur ou un patron habitué à Xudox retrouve ses gestes sur Pili,
et savoir ce qui reste à adapter. Pili garde les habitudes (taper le code du menu, multiplier), pas l'écran de Xudox :
l'interface reste celle de Pili, propre et moderne.

Légende : **OK** existe dans Pili, **Partiel** existe sous une autre forme, **Manque** pas d'équivalent aujourd'hui,
**Non** volontairement absent (loi SCE 2.0 ou hors périmètre). Mise à jour après les trois vagues de l'audit UX
(saisie par code, vague 1 à 3) : tout ce qui était « Nouveau » est maintenant simplement **OK**.

## 1. Prise de commande

| Xudox | Ce que ça fait | Pili | Statut |
| --- | --- | --- | --- |
| KeyB (clavier "Please input", touches 0-9, A-Z, OK) | taper le numéro du plat au lieu de chercher dans les catégories | pavé "Saisie code" ancré sous la grille (300 px, la grille garde ses 5 colonnes) ; la grille se filtre pendant la frappe, `A12` puis OK ou toucher le plat | OK |
| Touche `(*)乘` du clavier, boutons 1 à 8 en bas | multiplier le plat | touche "× Qté" : `3 × A12` ou `A12 × 3` ; `3 ×` puis toucher un plat dans la grille | OK |
| Touche `(-)减`, bouton `-1` | retirer une portion d'un plat déjà sur la note | bouton − sur la ligne de la note (pas de retrait par code : une seule façon de retirer) | OK |
| Taper le plat puis un chiffre (le chiffre est la quantité) | | juste après l'ajout, pendant 3 s, un chiffre fixe la quantité de cet ajout (2 chiffres jusqu'à 99) ; ne s'applique pas aux produits pesés, à options ou à prix libre (audit C7) | OK |
| Réglage "Keyboard" : le clavier s'ouvre seul à chaque nouvelle table | | le pavé reste ouvert ou fermé selon le dernier choix, par terminal | OK |
| Clavier physique du PC | | sur le pavé ouvert : chiffres, lettres, `*`, Entrée = OK, Retour arrière, Échap = effacer | OK |
| Boutons catégories (Menu, Soep, Voor, Hoofd...) + "Touch panel" | | onglets de catégories + grille de plats | OK |
| Affichage par numéro | | Réglages, Affichage des produits : Nom / Code | OK |
| "Combain" : un même plat tapé deux fois reste sur une ligne | | fusion automatique des lignes identiques | OK |
| `+`, `−`, Verwijderen | | boutons + / − / supprimer sur la ligne | OK |
| "Show Chinese" | | langue de l'interface 中文, noms des plats en chinois | OK |
| "Met 伴随", "Separate 分割线" (lignes pointillées sur le bon cuisine) | séparer les services | "Service en plusieurs temps" (Entrées, Plats, Desserts) et "Envoyer la suite" | Partiel |
| "Beep sound" à chaque ajout | | bip court et clignotement de 200 ms de la ligne à chaque ajout ; réglage "Bip à chaque ajout" par terminal dans Config (audit C9) | OK |
| "Favorite 顶置" (plats favoris en première page) | | premier onglet "Favoris" (étoile) : les 18 plats les plus vendus sur 30 jours, ou une sélection du gérant (étoile dans l'éditeur de menu) ; caché s'il est vide (audit C12) | OK |

## 2. Cuisine et bar

| Xudox | Pili | Statut |
| --- | --- | --- |
| BAR (boissons au bar), Chief / Keuken (cuisine) | "Envoyer en cuisine" imprime bar et cuisine selon le plat | OK |
| Keuken : Soep / Voor / Hoofd séparément | services (cours) + "Envoyer la suite" | Partiel |
| "WOK" (rappel du menu à volonté non saisi) | | Manque |
| "Aperitif" (la cuisine voit que la table est à l'apéritif) | | Manque |
| "Print one by one" (un bon par plat) | | Manque |
| Notes sur un plat ("Met" + texte) | note cuisine par ligne (60 caractères) et pastilles de notes fréquentes à toucher, réglées dans Config, "Notes rapides" (audit C11) | OK |

## 3. Tables

| Xudox | Pili | Statut |
| --- | --- | --- |
| "Tafels" + plan de salle | écran Tables, plan de salle configurable (1 à 999, avec zones) ; pastille de l'initiale du serveur sur la table occupée ; couverts facultatifs ("4 couv.") ; appui long de 500 ms sur une table occupée : Ouvrir, Addition, Déplacer, Payer (audit T5, T7) | OK |
| Numéros de table avec lettre de zone (A-19, B-30) | plan "A 1-20, B 21-40" avec noms de zones facultatifs ("A · Salle") ; tables groupées par zone, écrites "A-19" à l'écran et sur les bons ; le numéro (19) reste celui du ticket et des requêtes signées (audit T1) | OK |
| "Wisselen T" / "Change T" (changer de table) | Touche « Table 4 » de l'en-tête, ou « Déplacer » (Plus, ou plan de salle) : une table libre déplace la commande aussitôt, message « Annuler » 5 s (événement P M122) | OK |
| "Splitting table" (déplacer des plats vers une table T-1_B) | partage au paiement : parts égales (2 à 4, ou "Autre" 2 à 20) ou "Par plats" (on coche les plats de chaque part), toujours une seule vente ; pas de seconde note ni de scission de table | Partiel |
| "Delete 去除本台" (supprimer la note) | "Client parti sans payer" avec PIN gérant ; une vente n'est jamais effacée | Non (loi : aucune suppression) |

## 4. Paiement et remises

| Xudox | Pili | Statut |
| --- | --- | --- |
| Rekening : cash, carte, AmEx | Payer : espèces, carte (terminal lié ou manuel), chèques-repas, bon cadeau | OK |
| "Pro forma / Addition 结账印单" | "Addition" (événement P), depuis le panier ou par appui long sur la table | OK |
| "Bill type" : passer de cash à carte après le ticket | correction de paiement (M132, PIN gérant) | OK |
| "Kort" -5 / -10 / -20 %, % libre | remise 5 / 10 / 15 / 20 / 50 %, % ou montant libre (PIN gérant, demandé une seule fois par commande pendant 2 minutes pour les remises et offres suivantes, audit P8) | OK |
| "Gratis / Offert" | "Offrir" la ligne ou l'article (PIN gérant, même règle des 2 minutes) | OK |
| "Calculator" (rendu monnaie) | rendu calculé à l'écran de paiement, montant exact déjà saisi : espèces exactes en deux touches (Payer, Encaisser) | OK |
| Plusieurs amis paient en espèces la même note | chaque part en espèces garde son montant donné et son rendu ; la vente signée porte une seule ligne espèces arrondie (audit P3) | OK |
| "2H / 3H / Self defined end" (fin du buffet sur la note) | | Manque |
| "Takeaway %" (prix emporter moins cher), "Drink no discount" | TVA emporter (C) ; pas de remise emporter automatique | Manque |

## 5. Emporter, téléphone, réservation

| Xudox | Pili | Statut |
| --- | --- | --- |
| "Meeneem / Take away 外卖" | "À emporter" | OK |
| Tel-S / Tel-E / WM-E (plages de numéros salle, livraison, emporter) | emporter : numéro du jour "E-61" qui repart chaque jour au premier numéro réglé dans Config, en gros sur les bons cuisine et bar ; pas de plage de numéros pour la livraison (audit T2) | Partiel |
| "Phone" (fiche client : nom, adresse, téléphone) | emporter : nom (30 caractères) et téléphone facultatifs, effacés 72 h après la vente ; clients B2B pour factures ; pas de fiche client avec adresse | Partiel |
| "Order" (réservations) | | Non (hors périmètre caisse) |

## 6. Personnel, rapports, base de données

| Xudox | Pili | Statut |
| --- | --- | --- |
| Mot de passe 4 caractères, comptes serveurs | PIN, badge optionnel | OK |
| Staff management Start / End | pointage (événements S) à la connexion et en fin de service | OK |
| Rapport X / Z financier, X / Z utilisateur | Rapports X / Z (obligatoires SCE 2.0) ; retrouver un ticket par son numéro imprimé ("N° de ticket") pour la copie et le remboursement (audit R2) | OK |
| "Account" (listing mensuel) | rapport des ventes, export comptable | OK |
| Tiroir-caisse (mot de passe) | bouton "Ouvrir le tiroir" (événement F M131) ; une entrée ou sortie de caisse ouvre aussi le tiroir dans le même geste (audit R3) | OK |
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
- Un chiffre tapé juste après l'ajout d'un plat simple (3 s) fixe sa quantité, comme le « plat puis chiffre » de Xudox :
  le champ du code doit être vide, un second chiffre prolonge jusqu'à 99, toute autre touche ou un autre ajout rend les
  chiffres au champ du code. Un plat pesé, à options ou à prix libre n'ouvre jamais cette fenêtre.
- Rien de fiscal ne change : le pavé passe par la même commande que la grille (`add_order_line`) et lit le menu par
  `get_menu_items` sans catégorie. Retirer un plat reste le rôle de la note (boutons − et supprimer).
- Code : `ui/src/utils/codeEntry.ts`, `ui/src/components/CodePad.vue`, `OrderView.vue` ;
  `android/.../feature/order/CodeEntry.kt`, `CodeSearch.kt`, `CodePadFlow.kt`, `CodePad.kt`, `CodePadField.kt`.

## 8. Suites proposées, par ordre d'impact pour un habitué Xudox

Ce qui reste après les trois vagues (bip, zones, favoris, partage par plats, tiroir, ticket par numéro sont faits) :

1. Scission de note proprement dite (déplacer des plats vers une seconde note ouverte) : UC Advanced Set "table split",
   événements fiscaux à vérifier avant tout. Le partage "Par plats" au paiement couvre l'usage courant sans seconde note.
2. Heure de fin de buffet (2H / 3H) imprimée sur l'addition, rappel "WOK" pour l'à volonté, "Aperitif" visible en cuisine
   et "Print one by one" (un bon par plat).
3. Remise automatique à emporter ("Takeaway %", "Drink no discount") : aujourd'hui seule la TVA emporter (C) change.
4. Livraison : plages de numéros dédiées (Tel-E / WM-E) et vraie fiche client avec adresse, au-delà du nom et du
   téléphone facultatifs de l'emporter.
5. À vérifier sur le matériel avant de promettre : l'appui long de 500 ms au doigt sur le D3 Pro, la lecture d'un
   code-barres de bon cadeau par un lecteur USB réel et son impression CODE128 sur l'Epson TM-m30III, le bip sur le
   haut-parleur du terminal (ces points sont couverts par des tests simulés, pas encore essayés sur l'appareil).
6. Le nom chinois d'un produit reste le seul champ qui ne se tape pas au doigt (aucun clavier à l'écran ne l'écrit,
   `docs/android/DECISIONS.md`, UX-C13-DEV).
