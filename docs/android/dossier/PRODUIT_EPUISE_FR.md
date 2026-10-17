# Produit épuisé

Annexe du dossier de certification de la version 1.0.0. Elle décrit la fonction « Épuisé » : un
caissier ou un gérant déclare qu'un produit n'est plus disponible, la caisse refuse alors de
l'ajouter à une commande, et, si la caisse est reliée à HubRise, l'état du produit est transmis
au stock en ligne. La fonction n'a **aucun effet fiscal** : elle ne produit aucun événement,
ne touche à aucun compteur et ne modifie ni un ticket ni un rapport.

## 1. Parcours de l'opérateur

1. Sur l'écran de commande, le caissier ou le gérant touche le bouton **« Épuisé »**. Le mode
   est actif (le bouton change de couleur et une aide s'affiche).
2. Il touche un produit. Une fenêtre propose trois états : **« Épuisé jusqu'à la fin de la
   journée »**, **« Épuisé jusqu'à réactivation »** et **« Disponible »**. L'état actuel est
   entouré, avec, pour un produit épuisé, la date et l'heure de fin.
3. Hors du mode, un produit épuisé est grisé et porte la pastille **« Épuisé »**. Un appui sur
   ce produit affiche le message « Ce produit est épuisé. » et rien n'est ajouté.
4. Le bouton **« Produits épuisés (n) »** ouvre la liste de tous les produits épuisés, avec leur
   état, et le bouton « Remettre en vente » rend un produit disponible.

L'apprenti (profil sans droit d'encaissement) voit les produits grisés et le message
de refus, mais n'a ni le bouton « Épuisé » ni la liste : le moteur refuse ces commandes à un
profil qui ne peut pas encaisser. Les deux écrans (caisse Linux et caisse portable Android)
offrent les mêmes boutons, les mêmes textes (français, anglais, chinois) et le même
comportement.

## 2. Règle appliquée par le moteur

| Règle | Comportement |
|---|---|
| Refus | L'ajout d'un produit épuisé à une commande (avec ou sans options) et l'augmentation de la quantité d'une ligne de ce produit sont refusés avec le code `product_sold_out`. |
| Ce qui reste permis | Diminuer une quantité, supprimer une ligne et conserver les lignes déjà présentes sur une commande. Les commandes en ligne ne sont pas concernées. |
| Fin de journée | « Jusqu'à la fin de la journée » se termine à l'heure de fin de journée de l'installation (`day_end_hour`), heure de Bruxelles, au lendemain de la journée commerciale en cours. L'expiration est liée à l'heure, pas à la clôture Z. |
| Droits | Déclarer, lever ou lister : caissier ou gérant. |
| Ordre des contrôles | Le contrôle d'épuisement précède la lecture du produit : un produit épuisé est refusé avant tout calcul de ligne. |

## 3. Journal

Chaque déclaration, chaque levée et chaque expiration est ajoutée à la table `sold_out_log`
(action `set`, `clear` ou `expired`, fin de journée, auteur, date). La table est en ajout
seulement : des déclencheurs refusent toute modification et toute suppression. L'état courant
est dans `menu_item_sold_out` (une ligne par produit épuisé). Une expiration est inscrite au
journal à la lecture suivante du menu ou de la liste.

Requête de contrôle du journal :

```sql
SELECT at, menu_item_id, action, until, by FROM sold_out_log ORDER BY id;
```

## 4. Transmission à HubRise

La transmission n'a lieu que si l'intégration HubRise est activée **et** si le produit possède
une référence de SKU. Sinon rien n'est mis en file et le produit n'est épuisé que sur la caisse.
La table `hubrise_inventory_sync` contient une ligne par produit à transmettre. Le processus
d'envoi existant de la caisse envoie l'état **courant** au moment de l'envoi, par
`PATCH /catalogs/{catalog_id}/location/inventory` avec le jeton du point de vente :

| État du produit | Contenu envoyé |
|---|---|
| Épuisé jusqu'à la fin de la journée | stock `"0"` avec `expires_at` (fin de journée, décalage de Bruxelles) |
| Épuisé jusqu'à réactivation | stock `"0"` sans `expires_at` |
| Disponible (levé ou expiré) | stock `null` (l'entrée est retirée) |

Plusieurs changements faits hors ligne se réduisent à une seule requête portant l'état courant.
Politique d'erreur identique à celle des commandes : délai ou erreur serveur, nouvel essai avec
attente croissante (30 s doublées, plafonnées à 30 min) sans limite ; refus (400, 403, 404, 422),
la ligne est mise de côté avec son motif ; jeton refusé (401), le passage s'arrête et la
connexion est marquée en erreur d'authentification. Un nouveau changement du même produit remet
la ligne en file. Si HubRise est désactivé, rien n'est mis en file et sa réactivation ne rejoue
pas les anciens changements.

Remarque d'installation : l'envoi du stock demande que le jeton HubRise ait le droit d'écriture
sur l'inventaire (champ facultatif `scope` de la configuration). Sans ce droit, HubRise refuse
(403), la ligne est mise de côté avec son motif et la caisse continue de refuser le produit
localement.

## 5. Tests associés

Refus à l'ajout, avec options et à l'augmentation de quantité ; diminution et suppression
permises ; expiration par l'heure et ligne `expired` écrite une seule fois ; journal immuable ;
droits ; absence d'effet fiscal (aucun événement ajouté) ; corps et chemin de la requête
HubRise dans les trois états ; regroupement de plusieurs changements ; essai après erreur 503 ;
refus 403 et 422 mis de côté ; jeton refusé ; HubRise désactivé ; produit sans SKU.
