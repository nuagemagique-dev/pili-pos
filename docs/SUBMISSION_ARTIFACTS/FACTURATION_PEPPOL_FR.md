# Facturation B2B et envoi Peppol

Annexe du dossier de certification de la version 1.0.0. Elle décrit l'événement I (facture à un
client professionnel, cas M150, UC4121 et UC4122), les règles appliquées, la numérotation,
l'impression, la requête du journal des factures (article 33 de l'arrêté ministériel du
29 avril 2024) et le comportement de l'envoi Peppol, y compris sans réseau. Elle complète
*DECISION_PEPPOL_AP_FR.md* (choix du point d'accès). La fonction est présente dans tous les
binaires de la version 1.0.0 : elle n'est pas réservée à une compilation particulière, et un
événement I n'est produit que lorsqu'un caissier le demande. La saisie de texte se fait au clavier à l'écran de la caisse.

## 1. Parcours de l'opérateur

1. **Choix des ventes.** Dans « Rapports », onglet « Tickets », l'opérateur coche une ou
   plusieurs ventes (tickets signés, événement N) puis touche « Facture ». Une vente déjà
   facturée porte la mention du numéro de facture et ne peut plus être cochée.
2. **Client.** L'opérateur saisit le numéro de TVA et touche « Vérifier », ou cherche un client
   connu par son nom ou son numéro. Un numéro inconnu ouvre un formulaire (nom, rue et numéro,
   code postal, ville, pays). Le numéro est contrôlé **par le moteur, hors ligne** (chiffres de
   contrôle pour un numéro belge). Une pastille indique si le client est inscrit sur Peppol,
   non inscrit, ou inconnu (réseau absent ou sans réponse).
3. **Aperçu.** L'écran montre le client, les tickets repris, les lignes, la ventilation de la
   TVA par taux (base, TVA, total), le total et la mention « déjà payé : montant (ventes
   encaissées au comptoir), reste à payer : 0,00 € ». Les lignes et les montants viennent des
   archives signées des ventes : l'écran n'envoie que des identifiants de ventes et de client.
   Tout problème détecté par le moteur (vente déjà facturée, formation, ticket non facturable)
   est affiché et bloque la signature.
4. **Signature.** « Établir et imprimer la facture » signe l'événement I auprès du module de
   contrôle, imprime la facture et met l'envoi Peppol en file si les conditions de la section 6
   sont réunies. L'écran de résultat donne le numéro, l'état de l'impression et l'état de
   l'envoi.
5. **Liste des factures.** Sous la liste des tickets, un tableau reprend les factures établies :
   numéro, client, date, total, état de l'envoi Peppol (en attente, remise, bloquée, aucun
   envoi) et un bouton « Copie » réservé au gérant (événement C sur l'événement I).
6. **Panneau Peppol (gérant).** Dans « Config », un panneau n'apparaît que si l'installation
   contient une section `peppol` : fournisseur, identifiant d'envoi, nombre d'envois en attente,
   remis et bloqués, et la liste des envois bloqués avec leur motif et le bouton « Réessayer ».
   Le panneau se rafraîchit toutes les 30 secondes.

## 2. Règles appliquées par le logiciel

| Règle | Comportement |
|---|---|
| Une vente, une facture | Une vente ne figure que sur une seule facture. Une seconde tentative est refusée et le numéro de la facture existante est indiqué. |
| Ventes admises | Seuls les tickets de vente normaux (événement N) sont facturables. Un remboursement (N négatif) peut figurer sur une facture (cas UC4122). Sont refusés : événements de formation, copies, commandes de table, ventes clôturées impayées, ventes qui vendent un bon cadeau (`invoice_gift_voucher_sale` : ligne hors champ de T.V.A., catégorie O, incompatible avec nos factures ; voir `BONS_CADEAUX_FR.md` §8 bis). |
| Mode formation | Aucune facture n'est établie en mode formation. |
| Client belge sans Peppol | Sans section `peppol`, aucune facture n'est signée pour un numéro de TVA belge (`invoice_peppol_required`). |
| Lignes et montants | Les lignes, la TVA et les totaux viennent des archives signées des ventes et des réponses du module de contrôle. L'écran n'envoie que des identifiants de ventes et de client. |
| Centre de coût | La demande signée porte un centre de coût de type CUSTOMER, dont la référence est le nom du client (600 caractères au plus). |
| Numéro de TVA | Contrôlé hors ligne : chiffres de contrôle pour un numéro belge, préfixe pays et 2 à 13 caractères pour un numéro étranger. |
| Droits | Le moteur accepte, pour établir une facture et pour rechercher et enregistrer un client, un caissier ou un gérant (un apprenti est refusé) ; le panneau Peppol et la relance d'un envoi sont réservés au gérant. L'écran n'y mène que par « Rapports » et « Config », réservés au gérant. Nul ne signe au nom d'un autre opérateur. |

## 3. Numérotation

Le numéro de facture est l'année d'émission suivie d'une séquence continue d'au moins quatre
chiffres (par exemple 20260001). La séquence n'est jamais remise à zéro. Le numéro n'est
consommé que lorsque l'événement I est archivé avec la réponse du module de contrôle : un
échec du module ne crée donc aucun trou. Si la réponse se perd, la demande est renvoyée telle
quelle (même identifiant) et la facture garde son numéro. Les lignes `invoices`,
`invoice_sales` et `peppol_deliveries` sont écrites dans la même transaction que l'archive de
l'événement et l'avance du compteur.

## 4. Impression

La facture est imprimée en papier uniquement (français, néerlandais ou allemand selon la langue
de l'installation). Elle porte : le titre FACTURE, le numéro de facture, la date d'émission, le
vendeur (nom, adresse, TVA, numéro d'établissement), le client (nom, adresse, TVA), les
références des tickets d'origine (identifiant du module de contrôle, compteurs, numéro et date
du ticket), les lignes de ces tickets, la ventilation de la TVA par taux (base, TVA, total), le
total, la mention « déjà payé » et les données de contrôle de l'événement I **sans** signature
numérique et sans code QR (art. 43 de l'arrêté ministériel). Le pied de page rappelle que le
document n'est pas un ticket de caisse TVA valable. La **copie** d'une facture (événement C qui
référence l'événement I) reprend la même mise en page avec la mention COPIE (comme les copies de tickets de vente, et non « COPY TICKET »), et ses propres données de contrôle sous l'intitulé « Informations FDM de la copie ». Elle n'est jamais envoyée en version numérique.

## 5. Journal des factures (article 33)

Les factures sont conservées dans des tables en ajout seul. La vue `v_invoice_journal` en donne
une ligne par facture, dans l'ordre des numéros : numéro, date, TVA et nom du client, base et
TVA par taux (A, B, C), base D et X, total, identifiant du module de contrôle, libellé de
l'événement, date du module, numéro d'ordre de l'événement et état du dernier envoi Peppol.

```sql
SELECT invoice_no, issue_date, customer_vat_number, customer_name,
       base_a_cents, vat_a_cents, base_b_cents, vat_b_cents,
       base_c_cents, vat_c_cents, total_cents, fdm_id, event_label, peppol_status
FROM v_invoice_journal
WHERE issue_date BETWEEN '2026-01-01' AND '2026-12-31';
```

Les montants sont en centimes. Le rapport Z de la période liste chaque facture avec le montant
des ventes qu'elle référence.

## 6. Envoi Peppol

- **Quand.** Seulement si l'installation contient une section `peppol` et si le client est
  belge. Une facture à un client étranger est imprimée seulement, sans ligne d'envoi.
- **Sans section `peppol`.** Le moteur refuse de signer une facture pour un client dont le numéro
  de TVA commence par `BE` (code `invoice_peppol_required`) : depuis le 01.01.2026 une facture B2B
  à un assujetti belge doit passer par Peppol, et le manuel de certification demande de désactiver
  l'émission non conforme. Le refus a lieu avant la signature (aucun événement I, aucun numéro
  consommé, la vente reste facturable) et l'écran affiche le message traduit. Un client dont le
  numéro de TVA n'est pas belge reçoit la facture imprimée seulement.
- **Identifiants.** Le destinataire est `0208:` suivi du numéro d'entreprise à dix chiffres.
- **File d'envoi.** Chaque envoi est « en file », « remis » ou « bloqué » (valeurs `queued`, `accepted` et `failed` de la table `peppol_deliveries`). Une panne passagère
  (délai dépassé, code 429 ou 5xx, réseau absent) est retentée sans fin, avec une attente de 30
  secondes qui double jusqu'à 30 minutes. Un refus du point d'accès, un destinataire non inscrit
  ou une clé refusée **bloque** l'envoi avec son motif. Chaque tentative est ajoutée à un
  journal en ajout seul. Le gérant relance un envoi bloqué depuis le panneau Peppol.
- **Sans réseau.** La facture ne dépend que du module de contrôle. Si la vérification du
  destinataire échoue ou dépasse 3 secondes, la pastille indique « inconnu », la facture est
  établie et imprimée, et l'envoi est retenté plus tard par la file. Rien n'est jamais perdu :
  les documents attendent dans la base.
- **Point d'accès.** L'implémentation suit l'API Storecove v2 (voir *DECISION_PEPPOL_AP_FR.md*) et n'a été
  exercée que contre un simulateur local fourni avec le logiciel. Le fournisseur « logging »
  (développement) journalise le document sans rien envoyer : il est refusé dans la version de
  production, où la section `peppol` exige `storecove`. Aucun appel réel n'a été fait à un
  fournisseur ; le contrôle du document par le schématron Peppol se fait au point d'accès réel,
  que le simulateur ne reproduit pas. Un essai sur le bac à sable du fournisseur reste à faire
  par le propriétaire de l'installation avant la mise en service (voir la section 9).

Risque connu : dans le document Peppol (UBL), la TVA est recalculée à partir des lignes
arrondies (arrondi de chaque ligne, puis arrondi de la base par taux multipliée par le taux),
alors que la facture imprimée reprend la TVA par taux calculée par le module de contrôle pour
les tickets. Les deux peuvent différer de quelques centimes ; l'écart est absorbé par le montant
d'arrondi à payer du document. Le total à payer reste celui des tickets.

## 7. Paramétrage de l'installation

Section facultative `peppol` du fichier de configuration (`pos.json`) :

| Réglage | Rôle |
|---|---|
| `provider` | `storecove` (obligatoire en production) ou `logging` (développement uniquement, refusé en production). |
| `base_url`, `api_key_file`, `legal_entity_id` | Requis pour `storecove` ; la clé reste dans un fichier à part. |
| `sender_id` | Identifiant Peppol du vendeur : `0208:` et dix chiffres. |
| `supplier_street`, `supplier_postal_code`, `supplier_city`, `supplier_country` | Adresse du vendeur dans le document (pays `BE` par défaut). |

La commande de contrôle de configuration signale chaque réglage invalide sous le code
`config_peppol`, avec le nom du réglage.

## 8. Résultats des cas de test

| Cas | Contenu | Résultat |
|---|---|---|
| M150 | Facture référençant deux ventes | Deux ventes, puis une facture. Les champs métier de la demande sont identiques à ceux du scénario du jeu de base 1.8 (type et référence du centre de coût CUSTOMER, numéro de TVA du client, libellés des références). Différences voulues : identifiant du centre de coût (`C` suivi du numéro du client, contre `100` dans le scénario), numéro de facture et compteurs du module de contrôle des références, qui sont ceux de la session de test. Chaque référence est comparée à la réponse du module de contrôle pour la vente correspondante : réussi |
| UC4121 | Facture de trois ventes | Trois ventes payées en espèces (le scénario les paie en crédit client, qui n'existe pas dans le produit). Champs métier identiques au scénario avancé 1.6 ; numéro de facture, identifiant du centre de coût et compteurs différents par construction. Références comparées à la réponse du module de contrôle : réussi |
| UC4122 | Facture d'un remboursement et de deux ventes | La vente d'origine est d'abord facturée (précondition du scénario : le ticket remboursé « avait déjà été facturé »), puis remboursée (le remboursement est un événement N négatif), puis deux ventes sont faites, payées en espèces ; la seconde facture reprend le remboursement et les deux ventes. Mêmes réserves que UC4121 sur le numéro, le centre de coût et les compteurs. Références comparées à la réponse du module de contrôle : réussi |

Les règles sont couvertes par des tests : une vente une facture, mode formation refusé,
client belge refusé sans section `peppol` (et accepté avec, ou avec un numéro étranger),
numérotation continue malgré un échec du module, réponse perdue récupérée, rapport Z, copie de
facture, file d'envoi (accepté, 503 puis accepté, refus 422, clé refusée, non inscrit, relance,
réseau absent, aucun envoi pour un client étranger sans section `peppol`, client étranger, journal immuable). Les
écrans sont couverts par des tests de parcours (Vue, Playwright).

Les pièces (requête construite par pili-pos, réponse du module de contrôle) sont dans *RESULTATS_TESTS_UC_FR.md* (sections M150, UC4121 et UC4122) et dans `evidence/uc/`.

## 9. Limites connues

- Pas de crédit client ni de compte client différé : les ventes des cas UC4121 et UC4122 payées
  « crédit client » sont rejouées en espèces dans les tests, ce qui ne change rien à la demande
  de facture.
- Pas d'envoi Peppol vers un client étranger.
- Pas de facture à un client belge sans section `peppol` (refus `invoice_peppol_required`).
- Pas de facture en mode formation.
- Le contrôle du document par le schématron Peppol se fait au point d'accès réel ; le
  simulateur ne le reproduit pas. Un essai sur le bac à sable du fournisseur reste à faire par le
  propriétaire de l'installation avant la mise en service.

## Captures d'écran

Les écrans de cette fonction sont les captures 66 (sélection des ventes), 67 (client), 68 (aperçu), 69 (facture établie), 70 (refus faute de Peppol), 71 (liste des factures) et 72 (panneau Peppol, envoi bloqué) de *CAPTURES_ECRAN_FR.pdf*.
