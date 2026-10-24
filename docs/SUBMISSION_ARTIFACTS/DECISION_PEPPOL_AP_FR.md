# E-facturation B2B Peppol - Décision d'Access Point et déploiement

**État à la date de soumission :** la facture B2B (événement fiscal **I**)
et son envoi sur le réseau Peppol sont **dans la version 1.0.0** et dans
tous ses binaires : il n'y a pas de fonction Cargo `peppol`, et le choix
du point d'accès est celui décrit ci-dessous. Le comportement complet
(parcours de l'opérateur, règles, numérotation, impression, journal,
file d'envoi, cas M150, UC4121 et UC4122) est dans
*FACTURATION_PEPPOL_FR.md*. Le point d'accès n'a été exercé que contre un
simulateur local fourni avec le logiciel : aucun appel réel n'a été fait
à un fournisseur (voir « Ce qui reste à faire par l'exploitant »).

## Contexte réglementaire

À partir du **2026-01-01**, tout assujetti TVA belge doit délivrer les
factures B2B sous forme de **documents électroniques structurés** via
le réseau **Peppol**, en utilisant le profil UBL **BIS Billing 3.0**.
L'obligation est indépendante (et fonctionne en parallèle) de la
certification SCE-2.0 - SCE-2.0 régit la piste d'audit FDM/POS, Peppol
régit la délivrance des documents au niveau B2B.

Source : Arrêté Royal de 2024, transposant la Directive UE 2010/45/UE.

## Comment la caisse s'y conforme

La caisse appelle l'API d'un point d'accès Peppol agréé (réponse à
l'encadré « facturation » du manuel de certification, §III.F : API du POS
vers un point d'accès du réseau Peppol).

- La facture est signée par le module de contrôle (événement I) et
  imprimée sur papier ; **en plus**, pour un client belge et dans une
  installation qui a une section `peppol`, un document UBL 2.1 au
  profil BIS Billing 3.0 est mis en file puis envoyé au point d'accès.
- Un client étranger reçoit la facture imprimée seulement.
- Une installation **sans** section `peppol` ne signe aucune facture pour un
  client qui a un numéro de TVA belge (code `invoice_peppol_required`) :
  depuis le 01.01.2026 une telle facture doit passer par Peppol, et le manuel
  de certification demande de désactiver l'émission quand elle ne serait pas
  conforme. Rien n'est signé et aucun numéro n'est consommé. Un client sans
  numéro de TVA belge reçoit la facture imprimée comme avant.

## Candidats Access Point et choix

| Fournisseur | Style API | Tarification | Participant Peppol belge | Notes |
|---|---|---|---|---|
| **Storecove** | REST + enveloppe JSON | ~0,05-0,20 € / facture | Oui | API la plus simple. Sandbox disponible. |
| **Unifiedpost** | REST | Sur contrat | Oui | Acteur plus important, onboarding plus enterprise. |
| **Doccle** | REST | Sur contrat | Oui | Belge, orienté B2C-friendly. |

**Choix :** **Storecove** - API REST la plus simple, onboarding le plus
rapide, prix par facture compatible avec le volume attendu d'un
restaurant (< 5 factures B2B par mois). Le fournisseur est un réglage
d'installation (`peppol.provider`) ; en changer plus tard est un
changement de configuration et un ajustement de l'enveloppe, sans
migration de base (la colonne `ap_vendor` conserve le nom du fournisseur
pour l'audit sans en dépendre).

## Implémentation

Crate : `crates/pos-peppol`.

- `ubl.rs` - Sérialiseur UBL 2.1 BIS Billing 3.0, en centimes entiers
  (aucun nombre flottant pour l'argent). Les codes de TVA A, B et C sont
  la catégorie `S` à 21, 12 et 6 %, D la catégorie `Z`, X la catégorie `E`
  avec le motif d'exonération `vatex-eu-o`. Une ligne de TVA par
  catégorie, les remises et majorations de ticket en `AllowanceCharge`, le
  montant déjà payé en `PrepaidAmount` et un arrondi de document qui ramène
  le solde à 0,00.
- `access_point.rs` - Trait asynchrone `AccessPoint` (envoi et recherche
  du destinataire) et deux implémentations : `StorecoveAccessPoint`
  (API Storecove v2 : recherche `POST /api/v2/discovery/receives`, envoi
  `POST /api/v2/document_submissions`, jeton porteur) et
  `LoggingAccessPoint` (développement : journalise le document, n'envoie
  rien ; **refusé dans une version de production**, où `peppol.provider`
  doit être `storecove`).
- `errors.rs` - `PeppolError` : panne passagère (retentée), refus
  (envoi mis de côté), clé refusée (envoi mis de côté et passe arrêtée).

Commandes du moteur : `crates/pos-engine/src/commands/invoice.rs`
(facture, clients, journal) et `peppol_queue.rs` (file d'envoi). Tables :
`peppol_deliveries` et `peppol_delivery_log` (ajout seul), voir
*SCHEMA_BD_FR.md* §4. Réglages : section `peppol` de `pos.json` (clé API
dans un fichier à part ; *MANUEL_INSTALLATION_FR.md*).

## Ce qui reste à faire par l'exploitant

1. **Enregistrer le Peppol endpoint-ID de l'établissement.** Le TVA
   belge utilise le schéma `0208` (numéro d'entreprise BCE, 10 chiffres
   sans préfixe `BE`) ; l'endpoint-ID est `0208:` suivi de ce numéro BCE (par exemple
   `0208:0123456789`). L'enregistrement est effectué par le point d'accès
   choisi à l'onboarding - prend typiquement 1 à 2 jours ouvrables.
2. **Ouvrir un compte Storecove**, demander une clé API sandbox et
   l'identifiant d'entité légale, renseigner la section `peppol` (adresse
   de l'API, fichier de la clé, `legal_entity_id`, `sender_id`, adresse du
   vendeur) et exécuter un essai de bout en bout sur leur bac à sable : le
   contrôle du document par le schématron Peppol et la forme exacte de
   l'API réelle n'ont pas pu être vérifiés avec le simulateur.
3. **Smoke test production** - établir une facture de 1 € vers un
   récepteur de test connu, vérifier la délivrance dans le tableau de bord
   du fournisseur et que l'envoi passe de « en file » à « remis » dans le
   panneau Peppol.
4. **Confirmer le traitement du code de TVA X** (exonéré, motif
   `vatex-eu-o`) sur le bac à sable.

## Pourquoi la facture ne dépend pas du point d'accès

- L'obligation Peppol est **séparée** de SCE-2.0 : la facture (événement I)
  ne dépend que du module de contrôle. Une panne du point d'accès ou du
  réseau ne bloque jamais l'établissement ni l'impression de la facture.
- Les documents attendent dans la base (`peppol_deliveries`), retentés
  sans fin avec une attente de 30 secondes qui double jusqu'à 30 minutes ;
  un refus, un destinataire non inscrit ou une clé refusée met l'envoi de
  côté avec sa raison, visible du gérant qui peut le relancer.

## Risques

- **Panne du point d'accès** - le document n'est jamais perdu : la ligne
  reste en file (`queued`) ou est mise de côté (`failed`) avec
  `error_msg` rempli, et chaque tentative est ajoutée à
  `peppol_delivery_log`. Le gérant la voit dans le panneau Peppol.
- **Écart de quelques centimes sur la TVA du document Peppol** - la TVA
  du document est recalculée à partir des lignes arrondies, alors que la
  facture imprimée reprend la TVA par taux du module de contrôle ; l'écart
  éventuel est absorbé par le montant d'arrondi à payer du document, le
  total à payer restant celui des tickets (*FACTURATION_PEPPOL_FR.md* §6).
- **Coût par facture** - à moins de 5 factures B2B par mois, un
  restaurant type est confortablement sous le seuil où le coût du point
  d'accès devient sensible.
