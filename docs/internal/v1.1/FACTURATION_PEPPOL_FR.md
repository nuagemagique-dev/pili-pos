# Facturation B2B et envoi Peppol (version 1.1)

Document de travail du dossier de la mise à jour v1.1. Il décrit l'événement I (facture à un
client professionnel), la numérotation, l'impression, la requête du journal des factures
(article 33 de l'arrêté ministériel du 29 avril 2024), les résultats des cas M150, UC4121 et
UC4122, et le comportement de l'envoi Peppol, y compris sans réseau. Il remplace la note de
décision sur le point d'accès Peppol pour la version 1.1. Les écrans décrits existent à
l'identique sur la caisse Linux et sur la caisse portable Android : seules les saisies de texte
diffèrent (clavier à l'écran sur Linux, clavier du système sur Android).

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
   contrôle, imprime la facture et met l'envoi Peppol en file si les conditions de la section 5
   sont réunies. L'écran de résultat donne le numéro, l'état de l'impression et l'état de
   l'envoi.
5. **Liste des factures.** Sous la liste des tickets, un tableau reprend les factures établies :
   numéro, client, date, total, état de l'envoi Peppol (en attente, remise, bloquée, aucun
   envoi) et un bouton « Copie » réservé au gérant (événement C sur l'événement I).
6. **Panneau Peppol (gérant).** Dans « Config », un panneau n'apparaît que si l'installation
   contient une section `peppol` : fournisseur, identifiant d'envoi, nombre d'envois en attente,
   remis et bloqués, et la liste des envois bloqués avec leur motif et le bouton « Réessayer ».
   Le panneau se rafraîchit toutes les 30 secondes.

## 2. Numérotation

Le numéro de facture est l'année d'émission suivie d'une séquence continue d'au moins quatre
chiffres (par exemple 20260001). La séquence n'est jamais remise à zéro. Le numéro n'est
consommé que lorsque l'événement I est archivé avec la réponse du module de contrôle : un
échec du module ne crée donc aucun trou. Si la réponse se perd, la demande est renvoyée telle
quelle (même identifiant) et la facture garde son numéro. Les lignes `invoices`,
`invoice_sales` et `peppol_deliveries` sont écrites dans la même transaction que l'archive de
l'événement et l'avance du compteur.

## 3. Impression

La facture est imprimée sur papier, dans la langue de l'installation. Elle porte le titre
FACTURE, le numéro, la date d'émission, le vendeur, le client, les références des tickets
d'origine, les lignes, la ventilation de la TVA, le total, la mention « déjà payé » et les
données de contrôle de l'événement I, sans signature numérique ni code QR (article 43 de
l'arrêté ministériel). Une copie (événement C qui référence l'événement I) reprend la mise en
page avec la mention COPIE et ses propres données de contrôle ; elle n'est jamais envoyée en
version numérique. La mention imprimée est COPIE (comme pour les copies de tickets de vente), et non « COPY TICKET » ; les données de contrôle de la copie sont imprimées sous leur propre intitulé (« Informations FDM de la copie »), celles de la facture d'origine restant dans le bloc principal.

## 4. Journal des factures (article 33)

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

## 5. Envoi Peppol

- **Quand.** Seulement si l'installation contient une section `peppol` et si le client est
  belge. Une facture à un client étranger est imprimée seulement, sans ligne d'envoi.
- **Identifiants.** Le destinataire est `0208:` suivi du numéro d'entreprise à dix chiffres.
- **File d'envoi.** Chaque envoi est « en file », « remis » ou « bloqué ». Une panne passagère
  (délai dépassé, code 429 ou 5xx, réseau absent) est retentée sans fin, avec une attente de 30
  secondes qui double jusqu'à 30 minutes. Un refus du point d'accès, un destinataire non inscrit
  ou une clé refusée **bloque** l'envoi avec son motif. Chaque tentative est ajoutée à un
  journal en ajout seul. Le gérant relance un envoi bloqué depuis le panneau Peppol.
- **Sans réseau.** La facture ne dépend que du module de contrôle. Si la vérification du
  destinataire échoue ou dépasse 3 secondes, la pastille indique « inconnu », la facture est
  établie et imprimée, et l'envoi est retenté plus tard par la file. Rien n'est jamais perdu :
  les documents attendent dans la base.
- **Réserve.** Le format suit l'API v2 du fournisseur prévu et n'a été exercé que contre un
  simulateur local. Un essai sur le bac à sable du fournisseur reste à faire par le propriétaire
  de l'installation avant la mise en service.

Risque connu : dans le document Peppol (UBL), la TVA est recalculée à partir des lignes
arrondies (arrondi de chaque ligne, puis arrondi de la base par taux multipliée par le taux),
alors que la facture imprimée reprend la TVA par taux calculée par le module de contrôle pour
les tickets. Les deux peuvent différer de quelques centimes ; l'écart est absorbé par le montant
d'arrondi à payer du document. Le total à payer reste celui des tickets.

## 6. Résultats des cas de test

| Cas | Contenu | Résultat |
|---|---|---|
| M150 | Facture référençant deux ventes | Deux ventes, puis une facture. Les champs métier de la demande sont identiques à ceux du scénario du jeu de base 1.8 (type et référence du centre de coût CUSTOMER, numéro de TVA du client, libellés des références). Différences voulues : identifiant du centre de coût (`C` suivi du numéro du client, contre `100` dans le scénario), numéro de facture et compteurs du module de contrôle des références, qui sont ceux de la session de test. Chaque référence est comparée à la réponse du module de contrôle pour la vente correspondante : réussi |
| UC4121 | Facture de trois ventes | Trois ventes payées en espèces (le scénario les paie en crédit client, hors périmètre). Champs métier identiques au scénario avancé 1.6 ; numéro de facture, identifiant du centre de coût et compteurs différents par construction. Références comparées à la réponse du module de contrôle : réussi |
| UC4122 | Facture d'un remboursement et de deux ventes | Une vente remboursée (le remboursement est un événement N négatif), puis deux ventes, payées en espèces. Mêmes réserves que UC4121 sur le numéro, le centre de coût et les compteurs. Références comparées à la réponse du module de contrôle : réussi |

Les règles sont couvertes par des tests : une vente une facture, mode formation refusé,
numérotation continue malgré un échec du module, réponse perdue récupérée, rapport Z, copie de
facture, file d'envoi (accepté, 503 puis accepté, refus 422, clé refusée, non inscrit, relance,
réseau absent, aucun envoi sans section `peppol`, client étranger, journal immuable). Les
écrans sont couverts par des tests de parcours (Vue) et des tests unitaires et de captures
(Android).
