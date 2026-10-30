# Vente au poids et balance de caisse

Fonction fiscale de la caisse, comprise dans le binaire certifié et absente tant que l'installation ne contient pas de
section `scale`. Cas d'usage officiel : UC4112 de l'Advanced Set v1.6.

## 1. Objet et statut

Un restaurant vend un produit au kilo, par exemple un bol composé par le client. La caisse envoie le prix au kilo à une
balance de caisse Dibal DPOS-400, attend une pesée stable et écrit la ligne à partir de ce que la balance renvoie. Aucun
écran ne permet de saisir un poids.

État à la date du dossier, sans réserve cachée :

| Point | État |
| --- | --- |
| Volet fiscal (ligne, signature, ticket, rapports, remboursement) | Implémenté et testé ; UC4112 rejoué par le moteur (§8) |
| Protocole Bizerba « Checkout-Dialog 06 » | Implémenté d'après sa description, testé contre le simulateur du dépôt `pos-scale-mock` seulement |
| Essai sur une vraie DPOS-400 | Aucun. À effectuer (EV-25 : transcription de 200 pesées sur une vraie balance) |
| Raccordement | Réseau (TCP) seulement. USB et série : À effectuer (EV-22 : essai à vide, classe USB de la balance, port série du D3 Pro) |
| Contrôle du logiciel demandé par la balance (§4) | La caisse répond avec des valeurs fixes réglées dans `scale.check_values` ; l'algorithme du fabricant n'est pas connu. À effectuer (EV-26 : valeurs attribuées et contrôle accepté par une vraie balance) |
| Métrologie légale | Aucune réponse du SPF Économie à ce jour. À effectuer (EV-21 : réponses écrites du SPF Économie et de Dibal ou de son revendeur) |

**Aucune installation chez un client n'active la liaison avant une réponse écrite du service Métrologie du SPF Économie.**
Sans section `scale`, un produit vendu au poids reste hors vente et les ventes à la pièce ne changent pas.

## 2. La ligne pesée

| Donnée | Valeur |
| --- | --- |
| Produit | `menu_items.sold_by_weight` = 1 ; `price_cents` est alors le prix au kilo ; `scale_tare_grams` est une tare fixe facultative envoyée à la balance |
| Ligne de commande | Quantité 1, avec `weight_grams` et `weighed_amount_cents`. Deux pesées d'un même produit font deux lignes |
| Demande signée | `quantity` en kilogrammes, à trois décimales au plus, `quantityType` = `KILOGRAM`, `unitPrice` = prix au kilo ; le montant de la ligne est celui que la balance a calculé |
| Ticket | Le poids, le prix au kilo et le montant (« 0,612 kg », « 19,90 €/kg »), à 80 et à 58 mm |
| Facture | Unité `KGM` |
| Bon de cuisine | « 1 x » et le nom : la cuisine reçoit un bol, sans poids ni prix |
| Temps de service, transfert de table, formation | Comme une autre ligne ; les kilos signés ne changent pas |

La caisse refuse une pesée dont le poids n'est pas positif, dont l'unité n'est pas le kilogramme, dont le prix unitaire
renvoyé diffère de celui qu'elle a envoyé ou dont le montant s'écarte de plus d'un centime de `poids x prix`. Rien n'est
alors ajouté (`scale_price_mismatch`, `scale_unit_unsupported`). L'écart d'un centime est admis parce que la balance est
l'instrument légal et que sa règle d'arrondi n'est pas encore relevée ; l'acceptation par le FDM d'un tel écart reste à
vérifier (À effectuer (EV-14 : passage réel contre le simulateur module2, avec une vente pesée)).

Une ligne pesée ne change pas : pas de « + » ni de « - » (`line_weighed_fixed`). Elle se retire entière avant le paiement,
et la remise à la caisse s'applique au montant renvoyé par la balance. La pesée elle-même n'est pas un événement fiscal :
la ligne suit ensuite le chemin habituel (P, N ou T).

**Remboursement :** une ligne pesée se rembourse entière et une seule fois (`refund_weighed_line_whole`,
`refund_exceeds_sold`). Le remboursement signe le même poids en négatif, avec `negQuantityReason` = `REFUND`. Voir
*FLUX_REMBOURSEMENT_FR.md*.

**Commandes en ligne :** personne ne pèse à distance. Une commande en ligne qui contient un produit vendu au poids est
retenue et signalée à l'écran des commandes en ligne (*COMMANDES_EN_LIGNE_FR.md*). Une vente pesée à la caisse et renvoyée
à HubRise garde ses kilos et son prix au kilo.

## 3. Le protocole

Protocole 24 de la balance, enregistrements 01, 03, 04 et 05 (prix, tare, texte) de la caisse vers la balance, `ENQ` pour
demander le poids, 02 (résultat), 09 (état, après 08), 11 (demande et résultat du contrôle) et 10 (réponse au contrôle).
Le texte envoyé à la balance est le nom du produit en ASCII, coupé à 13 caractères ; un nom sans transcription n'est pas envoyé.

| Code d'état | Comportement de la caisse |
| --- | --- |
| 20 encore en mouvement, 22 prix pas encore disponible | Attend et redemande, l'écran indique la stabilisation |
| 21 aucun mouvement depuis la dernière pesée | Aucune ligne (`scale_no_motion`) : un même bol ne se compte pas deux fois |
| 30, 31, 32 sous le minimum, sous-charge, surcharge | Aucune ligne (`scale_below_min`, `scale_underload`, `scale_overload`) |
| 11, 12, 13 prix, tare ou texte invalide | Abandon (`scale_price_refused`) |
| 01 erreur générale | Abandon (`scale_error`) |
| 02 erreur de parité | Une répétition, puis abandon (`scale_error`) |

Une liaison coupée, une trame abîmée ou incomplète ne créent jamais de ligne (`scale_unreachable`, `scale_timed_out`, `scale_error`).
L'opérateur peut annuler à tout moment (`scale_cancelled`) : aucune ligne, aucun événement.

## 4. Le contrôle du logiciel

La balance demande un contrôle à la mise sous tension, après une erreur et toutes les 50 pesées, y compris pendant une
pesée. La caisse répond avec les valeurs de `scale.check_values`. Tant que la balance ne répond pas « contrôle OK », la
caisse n'envoie aucun prix (`scale_check_failed`). **Il n'existe aucun réglage pour contourner ce contrôle.** Chaque étape
est écrite dans `scale_check_log`. Les valeurs fixes ne sont pas la solution finale : la réponse attendue dépend de
l'algorithme du fabricant et de ce que le SPF Économie exigera (§1).

## 5. En cas de panne

Les ventes à la pièce continuent : la balance ne bloque rien d'autre. Les produits au poids ne se vendent plus tant
qu'elle ne répond pas. Le voyant « Balance » de l'en-tête passe au rouge et, si les alertes sur téléphone sont activées,
un message part au deuxième contrôle manqué (*ALERTES_PROPRIETAIRE_FR.md*). Les règles du FDM ne changent pas : sans FDM,
aucune vente, pesée ou non.

## 6. Configuration

Section facultative de `pos.json`, absente par défaut (*MANUEL_INSTALLATION_FR.md* §4.4 ter) ; l'indicateur de vente au
poids et la tare se règlent dans l'éditeur de carte (*EDITEUR_CARTE_FR.md*).

## 7. Traces conservées

| Table | Contenu |
| --- | --- |
| `scale_weighings` (ajout seul) | Ligne, commande, poids, prix au kilo, montant, tare, opérateur et heure de chaque pesée : le lien entre la balance et la ligne signée. Reste si la ligne est retirée |
| `scale_check_log` (ajout seul) | Étapes du contrôle du logiciel (`requested`, `answered`, `passed`, `refused`, `no_values`) |

Ces tables sont locales et n'entrent dans aucun message au FDM. Détail : *SCHEMA_BD_FR.md* §5 ter.

## 8. Tests

- UC4112 rejoué champ par champ : `crates/pos-engine/tests/spf_e2e_fixtures.rs`.
- Moteur contre le simulateur (ligne, refus, double pesée, contrôle, correction, remboursement, Z, X, rapport de ventes,
  facture, addition, copie à 80 et 58 mm, formation, cuisine, temps de service, transfert de table, ticket mixte, écho
  HubRise, installation sans balance) : `crates/pos-engine/tests/scale.rs`.
- Pilote contre le simulateur, par une vraie connexion (chaque code d'état, 50e pesée, liaison coupée en pleine trame) :
  `crates/pos-scale-mock/tests/dialog.rs`.
- Propriété : quelle que soit la suite d'octets reçue, pas de pesée sans un enregistrement 02 complet et contrôlé :
  `crates/pos-scale/tests/no_line_without_a_weight.rs`.
- Migration 80 : `crates/pos-db/src/migrate.rs`. Écrans : `android/app/src/test/kotlin/dev/nuagemagique/pilipos/feature/order/WeighDialogTest.kt`.

## 9. Machine de test

La machine de test est le profil émulé du D3 Pro, sans port USB ni série. La balance y est le simulateur `pos-scale-mock`,
par le réseau, sur le même modèle que le FDM et l'imprimante simulés. Elle peut être refusée à l'admission : la question
est posée au SPF Finances avec le dossier.

## Captures d'écran

À effectuer (EV-09 : captures de la fenêtre de pesée, du panier, du remboursement, des réglages et du voyant).
