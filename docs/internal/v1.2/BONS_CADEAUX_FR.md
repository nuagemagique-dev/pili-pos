# Bons cadeaux à usage multiple (mise à jour v1.2)

Statut : brouillon de la mise à jour v1.2 (hors dossier de certification v1.0.0).
Décisions : V12-D24 à V12-D31 et entrée « T5 » dans `docs/android/DECISIONS.md`.

## 1. Ce que fait la fonction

La caisse vend des bons cadeaux et les accepte comme moyen de paiement. Un bon est un bon à
usage multiple (MPV) : il n'est lié à aucun produit ni taux de T.V.A. au moment de la vente, la
T.V.A. est due quand le client achète des marchandises avec le bon. Le bon est utilisable en
une ou plusieurs fois, sans rendu de monnaie, jusqu'à sa date de fin de validité.

La fonction est éteinte par défaut. Éteinte, rien ne change à l'écran ni dans les tickets.

## 2. Vendre un bon

- Un gérant active la fonction dans Réglages (panneau « Bons cadeaux ») : un interrupteur, la
  validité en mois (12 à 36, 12 par défaut) et le montant maximal d'un bon (de 1,00 à 10 000,00 ;
  500,00 par défaut). Réglage conservé dans `owner_settings` (clé `gift_vouchers`), journal des
  changements inclus. Elle ne se désactive pas tant qu'un bon réel (hors formation) peut encore être utilisé : le moteur refuse (`gift_vouchers_outstanding`), car les clients gardent des bons valables au moins douze mois.
- Sur l'écran de commande, le bouton « Bon cadeau » demande le montant, de 1,00 au maximum réglé,
  et ajoute une ligne au panier : un produit système « Bon cadeau » (migration 63), taux de
  T.V.A. X (hors champ), département `GV`, prix libre, jamais affiché sur la grille. La ligne
  porte une pastille « Bon » à la place des boutons de quantité : un bon par ligne, la quantité
  ne change pas (`gift_voucher_line_fixed`). La ligne peut être supprimée avant la vente.
- Aucune remise ni changement de prix ne s'applique à la ligne d'un bon.
- Une commande qui vend un bon se règle en espèces ou par carte (manuelle ou terminal). Elle ne
  se règle ni avec des chèques-repas ni avec un bon cadeau (`gift_voucher_payment_invalid`), et
  ne peut pas être clôturée « sans paiement ».
- Au moment de la vente, la caisse tire un code par bon (section 3) et l'enregistre dans la
  même opération que l'événement N signé. Une réponse tardive du module fiscal, ou une reprise
  après une panne, crée le même code (le code est conservé avec la vente en attente).
- Après l'événement N, la caisse imprime le bon (section 6).

## 3. Le code d'un bon

12 chiffres : 10 chiffres tirés au hasard par le générateur du système, puis 2 chiffres de
contrôle (97 moins le reste de la division par 97), affichés par groupes de quatre
(`1234 5678 9095`) et saisis sur le pavé numérique. Les chiffres de contrôle détectent une
faute de frappe avant toute recherche. Les recherches qui échouent (forme, contrôle ou code
inconnu) sont limitées à 5 par minute et par caisse (`gift_voucher_lookup_throttled`).

Le code complet n'est imprimé que sur le bon. Les écrans, le ticket « solde du bon » et les
journaux n'en montrent que la forme masquée (`**** **** 9095`). Le code n'entre dans aucune
demande signée ni aucun ticket de T.V.A. (test sur tous les textes archivés).

## 4. Payer avec un bon

Au paiement, le bouton « Bon cadeau » ouvre la saisie du code. La caisse montre le solde, la
date de fin de validité et l'état (valide, épuisé, expiré, annulé). Le montant retenu est le
plus petit du solde et du reste à payer ; si le bon vaut plus que la note, le solde reste sur le
bon (aucune monnaie rendue). Plusieurs bons peuvent payer une même vente ; un bon n'est utilisé
qu'une fois par vente.

Dans la demande signée, le paiement est une ligne de type `VOUCHER_STORE`, identité « 8 »,
nom « Gift voucher », mode de saisie `MANUAL`, montant exact (jamais arrondi, comme les
chèques-repas). Il est enregistré avec la méthode `other` dans la table des paiements.

Contrôles faits sous le verrou de signature, avant l'envoi au module fiscal :

| Contrôle | Erreur |
| --- | --- |
| le bon existe | `gift_voucher_not_found` |
| même mode (formation ou réel), pas annulé | `gift_voucher_not_usable` |
| date de fin non dépassée (date locale) | `gift_voucher_expired` |
| solde suffisant pour la part | `gift_voucher_insufficient` |
| part positive, un seul paiement par bon, pas dans une vente de bon | `gift_voucher_payment_invalid` |

Le débit (mouvement `redeem`) est écrit dans l'opération d'archive qui suit la signature. La
signature est sérialisée et rien n'est signé pendant qu'une réponse est attendue : le solde ne
peut pas bouger entre le contrôle et l'écriture.

Mode formation : les bons créés en formation sont marqués et ne servent qu'en formation ; les
bons réels ne servent pas en formation.

## 5. Expiration, remboursement, rapports

- **Expiration.** La date de fin = date locale de la vente + la validité réglée. L'expiration
  n'est pas un événement : un bon expiré est refusé au paiement, un gérant peut toujours le
  retrouver. Aucun effet sur la T.V.A.
- **Remboursement de la vente d'un bon.** Rembourser la ligne d'un bon annule ce bon dans
  l'opération d'archive du remboursement, s'il n'a jamais servi et n'est pas déjà annulé ; sinon
  le remboursement est refusé (`gift_voucher_used`). La caisse choisit, parmi les bons non
  utilisés de la vente, ceux de la ligne remboursée ; s'il n'y en a pas assez, elle refuse.
- **Remboursement d'une vente payée par bon.** Le remboursement reste un remboursement en
  espèces, avec la confirmation du gérant, comme aujourd'hui : la valeur du bon devient de
  l'argent (question Q6, à confirmer par le propriétaire) et le bon n'est pas recrédité (différé,
  section 8).
- **Rapports.** Les rapports X et Z n'ont pas besoin de changer : le taux X est dans le calcul
  de T.V.A. du module fiscal, les paiements sont groupés par identité, nom et type. Le Z imprimé
  nomme le paiement « Bon cadeau » (fr), « Cadeaubon » (nl), « Geschenkgutschein » (de) (l'anglais
  et le chinois impriment en français).
- **HubRise.** L'écho d'une vente de comptoir laisse de côté la vente qui vend un bon (aucune
  ligne de bon n'est envoyée) ; un paiement par bon y est nommé « Other ».

## 6. Tickets imprimés

Impression non fiscale sur l'imprimante de tickets, avec la mention « ceci n'est pas un ticket de
caisse T.V.A. valable ». Langues : français, néerlandais, allemand (la langue de l'établissement ;
l'anglais et le chinois impriment en français).

| Ticket | Contenu |
| --- | --- |
| Bon | « BON CADEAU », établissement et adresse, valeur, code en grand et code QR du code, date d'émission, « Valable jusqu'au jj.mm.aaaa », « Utilisable en une ou plusieurs fois, sans rendu de monnaie », référence de la vente |
| Duplicata | le même, avec « DUPLICATA » ; commande de gérant, journalisée (`gift_voucher_prints`) |
| Solde du bon | imprimé après le ticket de T.V.A. quand un paiement laisse un solde : code masqué (4 derniers chiffres), solde restant, date de fin |
| Formation | la marque de formation reste sur ces tickets |

Une impression n'est enregistrée que si le ticket est réellement sorti. Un échec ne bloque pas la
vente : l'écran de fin de vente le dit (« Bon non imprimé : le réimprimer depuis Rapports, Bons
cadeaux »).

## 7. Écrans

| Écran | Contenu |
| --- | --- |
| Commande | bouton « Bon cadeau » (si la fonction est active), saisie du montant, ligne avec pastille « Bon » |
| Encaissement | méthode « Bon cadeau » (saisie du code, solde, validité, état, montant retenu), notes d'impression à la fin de la vente |
| Rapports, onglet « Bons cadeaux » | recherche par code (caissiers et gérants ; un serveur n'y a pas accès) ; pour un gérant, liste des bons (valides d'abord, filtre par état), mouvements, bouton « Réimprimer » |
| Réglages, panneau « Bons cadeaux » | interrupteur, validité, montant maximal |

Mêmes écrans et mêmes textes (fr, en, zh) sur Linux (Vue) et sur Android (Compose).

## 8. Ce qui est différé, et pourquoi

| Sujet | Raison |
| --- | --- |
| Bon à usage unique (SPV, UC270 et UC271) | La T.V.A. est due à la vente, sur les marchandises. L'utilisation est une ligne négative (`negQuantityReason VOUCHER`) dont le découpage de T.V.A. doit refléter la vente. Le cas officiel UC271 utilise le bon « SPV Menu all-in » (quantité -1, `negQuantityReason VOUCHER`, A -18,00 et B -40,00, soit -58,00), dont le découpage de T.V.A. reprend celui du produit composé « menu all-in » pour lequel le bon est valable ; UC270 est un autre bon (« Voucher Special Diner Offer », 2 menus d'affaires à B 75,00). Utiliser un bon à usage unique demande donc le découpage de T.V.A. d'un produit composé, et les produits composés ne sont pas pris en charge (même écart que pour UC261 et UC4109) : nous ne devinons pas. À reprendre avec les produits composés, ou si le SPF publie une précision (question Q4) |
| Bon converti en espèces (UC4105) | Un bon cadeau n'est pas échangeable contre de l'argent ; aucun besoin |
| Remboursement vers un bon | Le remboursement d'une vente payée par bon reste en espèces avec confirmation du gérant ; recréditer le bon est reporté (question Q6) |

`scripts/uc_scope.toml` garde UC270, UC271 et UC4105 « hors périmètre » avec la raison « différé en
v1.2 ».

## 9. Cas d'usage officiels (jeu avancé v1.6)

Rejoués par `crates/pos-engine/tests/spf_e2e_gift_vouchers.rs` : la demande construite par la
caisse est comparée champ par champ avec la fixture ; les lignes de paiement (`VOUCHER_STORE`,
`MANUAL`, montants) doivent être identiques ; toute autre différence est listée avec sa raison,
et une différence non listée fait échouer le test.

| Cas | Résultat | Écarts expliqués |
| --- | --- | --- |
| UC260, vente d'un bon de 50,00 payée par carte | conforme | `productId` : le bon est le produit système « Bon cadeau » ; `departmentId` : département `GV`. La carte de la fixture est en saisie automatique (terminal), celle de la caisse est en saisie manuelle |
| UC261, utilisation du bon pour un menu | conforme | `lineType` et `vats` de la ligne du menu : un produit composé n'est pas géré, le menu est un seul produit qui porte sa T.V.A. (B 116,00), comme UC4109 |
| UC262, utilisation du bon pour une commande mixte, à emporter | conforme | `transactionLines[1].vats[0].label` : une commande est sur place ou à emporter dans son ensemble ; vendue à emporter, le hamburger prend le taux C (la fixture mélange un hamburger en salle B et une boisson à emporter C) |

## 10. Données

Migration 63 (`docs/internal/v1.2/SCHEMA_BD_V1_2_FR.md`) : catégorie et produit système,
`gift_vouchers`, `gift_voucher_movements`, `gift_voucher_prints`. Les trois tables refusent toute
modification et toute suppression (déclencheurs). Le solde, l'état et l'annulation sont calculés
depuis les mouvements. Aucune table fiscale n'est modifiée. Aucune donnée ne quitte la caisse.

## 11. Tests

Moteur (`crates/pos-engine/tests/gift_vouchers.rs`, 38 scénarios) : réglage et validation, ligne et
montant, émission, réponse perdue puis reprise (un seul bon), utilisation partielle, deux bons,
expiration, bons de formation séparés, remboursements (bon non utilisé, bon utilisé, vente payée
par bon), rapports X et Z, limite de recherche, impressions, tables en ajout seulement, absence du
code dans les archives ; cas officiels UC260, UC261 et UC262 ; test d'accès par rôle. Fiscal
(`pos-fiscal`) : règles de paiement. Ticket (`pos-print`) : bon, solde, duplicata, formation.
Migration (contraintes, déclencheurs). Façade Android, interface Vue (Playwright,
`ui/e2e/gift-vouchers.spec.ts`) et Compose.
