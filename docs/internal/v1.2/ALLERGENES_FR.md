# Allergènes : les 14 allergènes à déclaration obligatoire (mise à jour v1.2)

Statut : brouillon de la mise à jour v1.2 (hors dossier de certification v1.0.0).
Décisions : V12-D17 à V12-D21 et entrée « T3 » dans `docs/android/DECISIONS.md`.

## 1. Ce que fait la fonction

La caisse peut afficher, pour chaque produit et chaque option, les allergènes qu'il contient
et ceux dont il peut contenir des traces, parmi les 14 substances de l'annexe II du
règlement (UE) n° 1169/2011. Elle peut aussi imprimer une fiche d'allergènes pour un produit
ou pour toute la carte. C'est une information pour le client et le personnel : la fonction ne
signe rien et n'écrit aucun événement fiscal.

Les 14 substances, dans l'ordre de l'annexe : 1 céréales contenant du gluten, 2 crustacés,
3 œufs, 4 poissons, 5 arachides, 6 soja, 7 lait, 8 fruits à coque, 9 céleri, 10 moutarde,
11 graines de sésame, 12 anhydride sulfureux et sulfites, 13 lupin, 14 mollusques.

## 2. Ce que veut dire chaque état

| État | Ce que la caisse affiche |
| --- | --- |
| Liste déclarée avec des allergènes | « Contient : ... » et, s'il y en a, « Traces : ... » |
| Liste déclarée et vide | « Aucun allergène » |
| Pas de déclaration | « Information non disponible, demandez au responsable » |

Une absence de déclaration n'est jamais présentée comme « aucun allergène ». Deux niveaux
existent : « contient » et « peut contenir » (traces).

## 3. D'où viennent les listes

- **Catalogue HubRise.** Étiquettes `allergen:<code>` (contient), `traces:<code>` (traces)
  et `allergen:none` (liste déclarée vide) sur le produit, la déclinaison ou l'option. Un
  code inconnu est un défaut du catalogue : le catalogue entier est refusé et l'écran de
  rejet le dit.
- **Saisie locale d'un gérant.** La commande existe dans le moteur (réservée aux
  administrateurs, refusée tant qu'un catalogue gère la carte) ; aucun écran ne l'appelle
  encore dans cette version.
- Un catalogue qui ne dit rien d'un produit n'efface jamais une liste saisie localement. Un
  catalogue qui retire ses étiquettes efface la liste qu'il avait lui-même déclarée.

## 4. Journal des changements

Chaque changement réel d'une liste écrit une ligne dans `allergen_change_log` : avant, après,
source (catalogue ou local), auteur. Le journal ne s'efface ni ne se modifie (déclencheurs).
Une synchronisation qui ne change rien n'écrit rien.

## 5. Affichage

Écran de commande, sur Linux (Vue) et sur Android (Compose), mêmes textes :

- des pastilles numérotées (numéro de l'annexe) sur les produits et sur les options, six au
  plus sur un produit puis « +n » ; aucun dessin d'icône ;
- un bouton « Allergènes » : tant que le mode est actif, toucher un produit ouvre sa fiche
  au lieu de l'ajouter à la commande, avec une légende des 14 numéros ;
- la fiche du produit : « Contient », « Traces », les options avec leur propre mention, deux
  boutons d'impression (ce produit, toute la carte). Hors du français, les noms français
  suivent entre parenthèses.

## 6. Impression

Ticket non fiscal, langues français, néerlandais, anglais et allemand (une interface chinoise
imprime en français). Titre « ALLERGÈNES », nom de l'établissement, date, un bloc par produit
(« Contient », « Traces », les options). Aucun prix. Pied de page : la mention du règlement
(« Selon le règlement (UE) n° 1169/2011, annexe II. Informations fournies par l'exploitant. »)
et la mention d'un document qui n'est pas un ticket valable pour la T.V.A.

## 7. Ce qui ne change pas

- Aucun allergène n'entre dans une demande signée au module fiscal (test).
- Aucune donnée ne quitte la caisse : les listes sont lues du catalogue, jamais envoyées.
- Aucune donnée personnelle n'est concernée.

## 8. Tests

Modèle et noms, migration (contraintes et journal inaltérable), import HubRise (étiquettes,
code inconnu, absence de déclaration, retrait), ticket (modèle exact en français, pas de prix,
pieds de page), commandes du moteur (15 scénarios et un test d'accès), façade Android,
interface Vue (Playwright) et Compose.
