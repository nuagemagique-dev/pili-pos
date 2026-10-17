# Note de mise à jour v1.1 : facturation B2B et Peppol

Document de travail du dossier de la mise à jour v1.1 de la caisse enregistreuse. Il complète
les annexes du dossier v1.0.0, qui ne sont pas modifiées : elles décrivent la version 1.0.0.
Cette section traite de la facturation (événement I, cas M150, UC4121, UC4122) et de l'envoi
des factures sur le réseau Peppol. Les autres sections de la note sont ajoutées par les autres
volets de la mise à jour.

## 1. Ce qui change pour l'utilisateur

- La caisse sait établir une **facture à un client professionnel** à partir de ventes déjà
  encaissées. Le caissier choisit une ou plusieurs ventes (tickets signés, événement N), choisit
  ou crée le client, contrôle l'aperçu, puis signe. La facture est signée par le module de
  contrôle (événement I, avec les références fdmRefs des ventes), imprimée sur papier, et, si
  l'installation est paramétrée pour Peppol et que le client est belge, mise en file d'envoi.
- Les factures ne demandent **aucun interrupteur** : un événement I n'est produit que lorsqu'un
  caissier le demande. L'envoi Peppol, lui, ne fonctionne que si l'installation contient une
  section `peppol` dans sa configuration.
- La fonction est présente dans **tous** les binaires (Linux et Android). Elle n'est plus
  réservée à une compilation particulière.

## 2. Règles appliquées par le logiciel

| Règle | Comportement |
|---|---|
| Une vente, une facture | Une vente ne figure que sur une seule facture. Une seconde tentative est refusée et le numéro de la facture existante est indiqué. |
| Ventes admises | Seuls les tickets de vente normaux (événement N) sont facturables. Un remboursement (N négatif) peut figurer sur une facture (cas UC4122). Sont refusés : événements de formation, copies, commandes de table, ventes clôturées impayées. |
| Mode formation | Aucune facture n'est établie en mode formation. |
| Lignes et montants | Les lignes, la TVA et les totaux viennent des archives signées des ventes et des réponses du module de contrôle. L'écran n'envoie que des identifiants de ventes et de client. |
| Numérotation | Numéro de facture = année d'émission suivie d'une séquence continue d'au moins quatre chiffres (par exemple 20260001), jamais remise à zéro. Le numéro n'est consommé que lorsque l'événement est archivé : un échec du module de contrôle ne crée aucun trou. Si la réponse du module se perd, la demande est renvoyée telle quelle et la facture garde son numéro. |
| Centre de coût | La demande signée porte un centre de coût de type CUSTOMER, dont la référence est le nom du client (600 caractères au plus). |
| Numéro de TVA | Contrôlé hors ligne : chiffres de contrôle pour un numéro belge, préfixe pays et 2 à 13 caractères pour un numéro étranger. |
| Droits | Établir une facture, rechercher et enregistrer un client : caissier ou gérant. Panneau Peppol et relance d'un envoi : gérant. Nul ne signe au nom d'un autre opérateur. |

## 3. Impression

La facture est imprimée en papier uniquement (français, néerlandais ou allemand selon la langue
de l'installation). Elle porte : le titre FACTURE, le numéro de facture, la date d'émission, le
vendeur (nom, adresse, TVA, numéro d'établissement), le client (nom, adresse, TVA), les
références des tickets d'origine (identifiant du module de contrôle, compteurs, numéro et date
du ticket), les lignes de ces tickets, la ventilation de la TVA par taux (base, TVA, total), le
total, la mention « déjà payé » et les données de contrôle de l'événement I **sans** signature
numérique et sans code QR (art. 43 de l'arrêté ministériel). Le pied de page rappelle que le
document n'est pas un ticket de caisse TVA valable. La **copie** d'une facture (événement C qui
référence l'événement I) reprend la même mise en page avec la mention COPIE.

## 4. Journal des factures (art. 33)

La base de données conserve les factures dans des tables en ajout seul (aucune modification ni
suppression possible) et expose la vue `v_invoice_journal` : numéro, date, client (TVA et nom),
base et TVA par taux, total, référence du module de contrôle de l'événement I et état de
l'envoi Peppol. Le rapport Z liste, pour la période, chaque facture avec le montant des ventes
qu'elle référence.

## 5. Envoi Peppol

- **Destinataires** : seuls les clients belges (numéro de TVA belge) sont envoyés sur Peppol.
  L'identifiant du participant est `0208` suivi du numéro d'entreprise à dix chiffres. Une
  facture à un client étranger est imprimée seulement et ne crée aucune ligne d'envoi.
- **Document** : facture UBL 2.1 au profil Peppol BIS Billing 3.0, calculée en centimes entiers
  (aucun nombre flottant pour l'argent), avec une ligne de TVA par catégorie, les remises et
  majorations de ticket en AllowanceCharge, le montant déjà payé en PrepaidAmount et un arrondi
  de document qui ramène le solde à 0,00.
- **Vérification du destinataire** : l'appel de découverte du point d'accès dure 3 secondes au
  plus. Son résultat est mis en cache sur le client (inscrit, non inscrit, inconnu). Un échec ou
  une absence de réseau donne « inconnu » et **ne bloque jamais** la facture, qui ne dépend que
  du module de contrôle.
- **File d'envoi** : un envoi passe de « en file » à « accepté » ou « en échec ». Une panne
  passagère (délai dépassé, code 429 ou 5xx) est retentée sans fin, avec une attente de 30
  secondes qui double jusqu'à 30 minutes. Un refus du point d'accès, un destinataire non inscrit
  ou un défaut d'authentification **met l'envoi de côté** avec sa raison ; une clé refusée
  arrête aussi la passe en cours. Chaque tentative est ajoutée à un journal en ajout seul.
- **Gérant** : le panneau Peppol montre les compteurs et les envois, et permet de relancer un
  envoi mis de côté.
- **Point d'accès** : l'implémentation suit l'API Storecove v2 et n'a été exercée que contre un
  simulateur local fourni avec le logiciel. Le fournisseur « logging » (développement) journalise
  le document sans rien envoyer. Aucun appel réel n'a été fait à un fournisseur.

## 6. Paramétrage de l'installation

Section facultative `peppol` du fichier de configuration (`pos.json` ou lot Android) :

| Réglage | Rôle |
|---|---|
| `provider` | `storecove` ou `logging` (par défaut). |
| `base_url`, `api_key_file`, `legal_entity_id` | Requis pour `storecove` ; la clé reste dans un fichier à part. |
| `sender_id` | Identifiant Peppol du vendeur : `0208:` et dix chiffres. |
| `supplier_street`, `supplier_postal_code`, `supplier_city`, `supplier_country` | Adresse du vendeur dans le document (pays `BE` par défaut). |

La commande de contrôle de configuration signale chaque réglage invalide sous le code
`config_peppol`, avec le nom du réglage.

## 7. Limites connues de la v1.1

- Pas de crédit client ni de compte client différé : les ventes des cas UC4121 et UC4122 payées
  « crédit client » sont rejouées en espèces dans les tests, ce qui ne change rien à la demande
  de facture.
- Pas d'envoi Peppol vers un client étranger.
- Pas de facture en mode formation.
- Le contrôle du document par le schématron Peppol se fait au point d'accès réel ; le
  simulateur ne le reproduit pas. Un essai sur le bac à sable du fournisseur reste à faire par le
  propriétaire de l'installation avant la mise en service.

## 8. Tests associés

Cas M150 rejoué champ par champ ; cas UC4121 (facture de trois ventes) et UC4122 (facture d'un
remboursement et de deux ventes) ; règles (une vente une facture, formation refusée, numérotation
continue, perte de réponse récupérée, rapport Z, copie de facture) ; file d'envoi contre le
simulateur (accepté, 503 puis accepté, refus 422, clé refusée, non inscrit, relance, réseau
absent, rien sans section `peppol`, client étranger, journal immuable) ; tests de contrat des
enregistrements de l'interface Android.

## 9. Écrans de facturation et de suivi Peppol

Les deux caisses (Linux et portable) offrent les mêmes écrans, décrits dans le document
`FACTURATION_PEPPOL_FR.md` : choix des ventes et action « Facture » dans l'onglet « Tickets »,
fenêtre de facturation en trois étapes (client, aperçu, résultat), liste des factures avec l'état
de l'envoi Peppol et copie réservée au gérant, panneau Peppol de la configuration (fournisseur,
compteurs, envois bloqués, « Réessayer »). Le contrôle du numéro de TVA est fait par le moteur,
hors ligne ; l'absence de réseau ne bloque jamais la facture. La saisie du texte se fait au
clavier à l'écran sur Linux et au clavier du système sur Android, avec les mêmes règles
(majuscules, longueurs maximales). Aucune règle fiscale n'est portée par l'écran : il envoie des
identifiants de ventes et de client, le moteur calcule et signe.
