# Bons cadeaux à usage multiple

Fonction fiscale de la caisse (vente en T.V.A. X, paiement `VOUCHER_STORE`), comprise dans le binaire certifié et éteinte par défaut.

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
- La ligne d'un bon ne reçoit ni service (cuisine) ni note de cuisine : le moteur refuse ces deux demandes
  (`gift_voucher_line_no_kitchen`), puisqu'un bon ne part jamais en cuisine.
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
- **Remboursement d'une vente payée par bon.** Le bon lui-même n'est jamais échangé contre de
  l'argent : il a servi à payer des marchandises. Rembourser ces marchandises est le remboursement
  d'une vente, pas l'échange d'un bon contre des espèces. Ce remboursement
  se fait en espèces (choix de l'établissement), avec la confirmation du gérant, comme pour toute autre
  vente, car la caisse ne rembourse qu'en espèces ; le bon n'est pas recrédité (recréditer un bon n'est pas
  offert en v1.0.0, section 8).
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
| Rapports, onglet « Bons cadeaux » | recherche par code (Serveur et Gérant ; l'Apprenti n'y a pas accès) ; pour un gérant, liste des bons (valides d'abord, filtre par état), mouvements, bouton « Réimprimer » |
| Réglages, panneau « Bons cadeaux » | interrupteur, validité, montant maximal |

Textes en français, anglais et chinois.

## 8. Ce qui n'est pas offert en v1.0.0, et pourquoi

| Sujet | Raison |
| --- | --- |
| Bon à usage unique (SPV, UC270 et UC271) | La T.V.A. est due à la vente, sur les marchandises. L'utilisation est une ligne négative (`negQuantityReason VOUCHER`) dont le découpage de T.V.A. doit refléter la vente. Le cas officiel UC271 utilise le bon « SPV Menu all-in » (quantité -1, `negQuantityReason VOUCHER`, A -18,00 et B -40,00, soit -58,00), dont le découpage de T.V.A. reprend celui du produit composé « menu all-in » pour lequel le bon est valable ; UC270 est un autre bon (« Voucher Special Diner Offer », 2 menus d'affaires à B 75,00). Utiliser un bon à usage unique demande donc le découpage de T.V.A. d'un produit composé, et les produits composés ne sont pas pris en charge (même écart que pour UC261 et UC4109) : nous ne devinons pas. Non offert en v1.0.0 ; la décision serait à revoir seulement si les produits composés étaient pris en charge ou si le SPF publiait une précision |
| Bon converti en espèces (UC4105) | Un bon cadeau n'est jamais échangé contre de l'argent ; la fonction n'est pas offerte |
| Remboursement vers un bon | Une vente payée par bon, une fois remboursée, se fait en espèces (choix de l'établissement), avec la confirmation du gérant ; recréditer le bon n'est pas offert |

UC270, UC271 et UC4105 sont déclarés « hors périmètre » dans *MATRICE_TRACABILITE_FR.md* : les fonctions correspondantes ne sont pas offertes.

## 7 bis. Droits

| Action | Profil |
|---|---|
| Régler la fonction (interrupteur, validité, montant maximal), lister les bons, réimprimer un bon (DUPLICATA) | Gérant |
| Vendre un bon (ligne de commande), chercher un bon par son code, payer avec un bon | Serveur et Gérant |
| Lire le réglage | tout profil connecté |

Un profil sans le droit ne voit pas le bouton, et le moteur refuse de toute façon la commande (`forbidden`).

## 8 bis. Interactions avec les autres fonctions de la caisse

- **Remises.** Aucune remise ni offre ne s'applique à la ligne d'un bon (`discount_invalid`) ; une remise sur la
  commande donne à la ligne du bon un poids nul : le bon garde sa valeur faciale et la ligne X ne porte aucune
  modification de prix. L'écran ne propose ni « Offert » ni « Note » sur cette ligne. Un service de cuisine
  ou une note de cuisine est refusé de même (`gift_voucher_line_no_kitchen`).
- **Facture à un client professionnel (événement I).** Une vente qui vend un bon ne peut pas être facturée
  (`invoice_gift_voucher_sale`) : la ligne est hors champ de T.V.A. (catégorie O) et la norme EN 16931 (règles
  BR-O-02 et BR-O-11 à BR-O-14) interdit une facture de catégorie O qui porte un identifiant de T.V.A. ou qui mêle
  d'autres catégories, alors que nos factures portent toujours l'identifiant de T.V.A. du vendeur. Lecture
  prudente : la vente entière est refusée plutôt que découpée. Une vente payée avec un bon (paiement
  `VOUCHER_STORE`, lignes à leur taux normal) se facture normalement.
- **Correction d'un moyen de paiement (M132).** La correction ne touche jamais une ligne `VOUCHER_STORE` (le mouvement
  du bon est écrit dans l'opération d'archive de la vente et ne peut pas suivre un autre moyen), ne corrige jamais
  vers le moyen « Bon cadeau » et ne déplace jamais une ligne d'une vente qui vend un bon vers les chèques-repas
  (un bon ne s'achète pas avec des chèques-repas, comme à l'encaissement). Codes `payment_correction_gift_voucher`,
  `payment_correction_method` et `gift_voucher_payment_invalid`. L'écran montre la ligne du bon verrouillée, avec
  une note.
- **Rapports.** Le rapport de ventes non fiscal compte la ligne de bon (rayon « Bons cadeaux », T.V.A. X) et son
  total est égal à celui du Z ; le paiement `VOUCHER_STORE` y est une ligne de paiement.
- **Éditeur de carte.** Le produit et la catégorie système du bon (identifiants externes `pili:`) n'apparaissent pas
  dans l'éditeur, qui ne peut ni les renommer, ni les modifier, ni les activer.

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

Migration 63 (*SCHEMA_BD_FR.md*) : catégorie et produit système,
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
Migration (contraintes, déclencheurs). Interface de l'application (tests `OrderGiftVoucherTest`, `CheckoutGiftVoucherTest`, `GiftVouchersTest` et `GiftVoucherSectionTest`, façade `android/rust/pos-ffi/tests/ffi/gift_vouchers.rs`).
Interactions avec les autres fonctions : `crates/pos-engine/tests/lane_interactions.rs`.

## Captures d'écran

Les écrans de cette fonction sont les captures A132 (réglage), A193 (montant du bon), A171 (ligne de bon), A172 (fin de vente d'un bon), A173 (code du bon), A174 (bon trouvé), A175 (bon et espèces), A176 (vente payée), A177 (liste des bons) et A178 (vérification d'un bon) de *CAPTURES_ECRAN_FR.pdf*.
