# Service en plusieurs temps : entrées, plats, desserts

Fonction de cuisine, non fiscale, comprise dans le binaire certifié et éteinte par défaut.

## 1. Ce que fait la fonction

La caisse peut envoyer à la cuisine d'abord les entrées, puis les plats, puis les desserts,
à la demande du personnel (bouton « Envoyer la suite »), au lieu de tout imprimer en une
fois. C'est une aide à l'organisation du service : le « temps » d'une ligne est une donnée de
cuisine qui n'existe que dans la caisse. La fonction ne signe rien, n'écrit aucun événement
fiscal et ne change ni un nom de produit, ni un département, ni un montant.

La fonction est éteinte par défaut. Éteinte, les tickets de cuisine et de bar sont exactement
ceux d'avant (test octet pour octet).

## 2. Les temps

| Temps | Nom à l'écran et sur le ticket | Usage |
| --- | --- | --- |
| 0 | Direct | boissons et tout ce qui part tout de suite (c'est aussi le temps d'une catégorie non réglée) |
| 1 | Entrées | premier envoi |
| 2 | Plats | envoyés par « Envoyer la suite » |
| 3 | Desserts | envoyés par « Envoyer la suite » |

Noms en français, néerlandais et allemand sur le ticket (la langue du ticket), en français,
chinois et anglais à l'écran. Un gérant règle, dans Réglages, le panneau « Service en
plusieurs temps » : un interrupteur et, pour chaque catégorie active, le temps de ses produits
(quatre boutons). Réglage conservé dans `owner_settings` (clé `courses`), journal des
changements inclus.

## 3. Règles

- Une ligne nouvelle prend le temps de sa catégorie (0 si la catégorie n'est pas réglée).
- Le personnel peut changer le temps d'une ligne tant qu'aucun ticket de cuisine ne l'a portée
  (sinon : `course_already_sent`).
- Chaque commande de table a un temps libéré, 1 au départ. Quand un événement P est signé
  (ou le N d'une vente directe), le ticket imprime les lignes de temps 0 et celles dont le
  temps n'est pas supérieur au temps libéré, groupées par temps sous un en-tête
  (`-- Plats --`). Les autres sont retenues.
- « Envoyer la suite » : si la commande a des lignes non enregistrées, la caisse les signe
  d'abord par le chemin habituel de la commande de table (module fiscal absent : refus, rien
  n'est signé, libéré ni imprimé, comme aujourd'hui). Elle libère ensuite le plus petit temps
  retenu et imprime ses lignes. Un temps dont les lignes sont déjà signées s'imprime sans
  le module fiscal.
- Une vente directe (comptoir, sans table) imprime tous les temps d'un coup, avec les en-têtes.
- Une vente qui clôt une table qui retient encore un temps n'imprime pas ce temps : les deux
  interfaces avertissent avant l'encaissement (« Suite non envoyée : Plats ») avec un bouton
  « Envoyer maintenant ». Le personnel décide, la caisse n'imprime jamais seule.
- Le registre `kitchen_slip_entries` n'enregistre une ligne qu'une fois le ticket réellement
  imprimé : une imprimante en panne laisse les lignes à imprimer, un nouvel essai les reprend.

### Corrections

| Cas | Effet |
| --- | --- |
| Correction d'une ligne déjà imprimée | imprimée tout de suite (règle d'aujourd'hui) |
| Correction d'une ligne retenue | reste retenue, compensée à la libération |
| Quantité nette nulle à la libération | rien n'est imprimé pour cette ligne |

## 4. Ticket de libération et duplicata

Le ticket de libération a le même titre et les mêmes mentions que le ticket de bar d'aujourd'hui
(aucun montant ; la mention « ceci n'est pas un ticket de caisse T.V.A. valable »), plus la
ligne « SUITE : Plats » sous la table et les en-têtes de temps. La réimpression d'un ticket
(`reprint_course_slip`) est une fonction du moteur seul : aucun écran ne la propose. Un ticket ainsi réimprimé porte « DUPLICATA » sous le titre et n'enregistre rien. Mode
formation : la marque de formation reste sur ces tickets.

## 5. Pourquoi rien n'atteint le module fiscal

- Spécification du module fiscal, section 8.1 « Impressions » : les tickets de cuisine et de
  bar ne mentionnent jamais de montants et suivent la mention d'un document non valable pour la
  T.V.A.
- Section 8.2 : l'imprimé associé peut être consolidé et regroupé ; la consolidation ne
  concerne jamais les messages du module fiscal. Le regroupement par temps est une
  consolidation d'imprimé.
- FAQ des producteurs, Q210 b : ce que le point de vente fait pour l'affichage du ticket de
  cuisine ou de bar reste au niveau du point de vente. Le temps suit la même lecture que
  l'option sans supplément.

En conséquence, le temps vit dans `CartLine` et dans `TableInfo` (temps retenu), jamais dans
`OrderLine` (celui des constructeurs de demandes signées), ni dans un nom de produit, un
département, les demandes P, N, T et pré-addition, ni le ticket de T.V.A. Un test parcourt
tous les textes archivés, les demandes signées et les tickets d'une vente à plusieurs temps et
n'y trouve aucune donnée de temps.

## 6. Données

Migration 62 (*SCHEMA_BD_FR.md*) : `order_line_courses`,
`order_course_releases`, `kitchen_slip_entries`. Aucune table fiscale n'est modifiée.
Les lignes déjà signées avant la migration sont reprises comme déjà imprimées (temps 0).
Aucune donnée ne quitte la caisse.

## 7. Tests

Moteur (`crates/pos-engine/tests/courses.rs`, 20 scénarios) : réglage et validation, temps d'une
catégorie, tickets identiques éteint, premier envoi, libération en séquence, signature préalable,
module fiscal absent, temps déjà signé, corrections, changement de temps, vente directe, vente
de table, imprimante en panne puis reprise, duplicata, formation, langue, absence de donnée de
temps dans les textes archivés, demandes signées identiques allumé ou éteint, plus un test
d'accès. Ticket (`pos-print`) : octet pour octet identique sans temps, en-têtes, mentions,
noms par langue, largeur du papier. Migration (contraintes, reprise de l'existant). Interface (Playwright, `ui/e2e/courses.spec.ts`).

## 7 bis. Droits

Régler la fonction (interrupteur et temps de chaque catégorie) est réservé au Gérant. Tout profil connecté peut
lire le réglage, changer le temps d'une ligne qui n'a pas encore été envoyée et utiliser « Envoyer la suite ».

## 8. Interactions avec les autres fonctions de la caisse

- **Temps d'une catégorie et éditeur de carte.** Le sélecteur de temps (Direct, Entrées, Plats, Desserts) est dans le
  panneau d'une catégorie de l'éditeur de carte quand la fonction est allumée ; le même bouton « Enregistrer » écrit
  la catégorie puis son temps (*EDITEUR_CARTE_FR.md*).
- **Modification de prix.** Une ligne de modification de prix du journal de commande n'est jamais retenue ni imprimée
  par les temps : elle est consommée au premier envoi et ne revient pas à « Envoyer la suite », qui imprime le
  produit une seule fois.
- **Notes de cuisine.** Une note imprime sur le premier ticket de sa ligne, sur le ticket de libération d'une
  ligne retenue et sur un DUPLICATA. Une note posée sur une ligne retenue qui n'a jamais été imprimée ne produit
  aucun ticket de modification : la note part avec la libération.
- **Transfert de table.** La demande de transfert ne porte aucune donnée de temps ; les temps retenus suivent la
  commande et le ticket de libération nomme la table de destination.
- **Mode « Allergènes » et mode « Épuisé ».** Les deux modes prennent le toucher d'un produit : en allumer un éteint
  l'autre.

Ces interactions sont vérifiées par `crates/pos-engine/tests/lane_interactions.rs`.

## Captures d'écran

Les écrans de cette fonction sont les captures 84 (réglage), 85 (panier par temps et « Envoyer la suite »), 86 (choix du temps), 87 (plan de salle, « Suite : Plats »), 88 (avertissement au paiement) et 78 (temps d'une catégorie dans l'éditeur de carte) de *CAPTURES_ECRAN_FR.pdf*.
