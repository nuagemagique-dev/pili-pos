# E-facturation B2B Peppol - Décision d'Access Point et déploiement

**État à la date de soumission (2026-05-14) :** Sérialiseur UBL +
abstraction AccessPoint implémentés et couverts par tests unitaires.
Aucun AP live n'est câblé dans l'image kiosk - l'étape d'achat côté
opérateur est suivie ici.

## Contexte réglementaire

À partir du **2026-01-01**, tout assujetti TVA belge doit délivrer les
factures B2B sous forme de **documents électroniques structurés** via
le réseau **Peppol**, en utilisant le profil UBL **BIS Billing 3.0**.
L'obligation est indépendante (et fonctionne en parallèle) de la
certification SCE-2.0 - SCE-2.0 régit la piste d'audit FDM/POS, Peppol
régit la délivrance des documents au niveau B2B.

Source : Arrêté Royal de 2024, transposant la Directive UE 2010/45/UE.

## Périmètre à la date de soumission

Tokyo Sushi Bar est massivement B2C (clients sur place + emporter).
Les factures B2B sont exceptionnelles - typiquement quand un
restaurant voisin ou un client entreprise commande un plateau à
emporter.

Le dossier de certification déclare donc Peppol comme :

> **Dans le périmètre, prêt au câblage.** La génération UBL 2.1
> BIS Billing 3.0 est implémentée et testée contre un jeu d'échantillons
> interne. Le transport AS4 est délégué à un Peppol Access Point ; le
> choix de l'AP et l'enregistrement du Peppol endpoint-ID sont des
> formalités côté opérateur indépendantes de la revue SCE-2.0.

## Candidats Access Point

| Fournisseur | Style API | Tarification | Participant Peppol belge | Notes |
|---|---|---|---|---|
| **Storecove** | REST + enveloppe JSON | ~0,05-0,20 € / facture | Oui | API la plus simple. Sandbox disponible. |
| **Unifiedpost** | REST | Sur contrat | Oui | Acteur plus important, onboarding plus enterprise. |
| **Doccle** | REST | Sur contrat | Oui | Belge, orienté B2C-friendly. |

**Recommandation :** **Storecove** pour le premier déploiement - API
REST la plus simple, onboarding le plus rapide, prix par facture
compatible avec le volume attendu de Tokyo Sushi Bar (< 5 factures B2B
par mois).

## État de l'implémentation

Crate : `crates/pos-peppol`

- `ubl.rs` - Sérialiseur UBL 2.1 BIS Billing 3.0. Mappe les labels TVA
  SCE-2.0 (A/B/C/D/X) sur les catégories fiscales BIS (S/Z/E + %).
  5 tests unitaires couvrent les namespaces, la propagation du n° de
  facture, les codes de schéma d'endpoint-id de partie, les helpers
  de date et la couverture complète des labels TVA.
- `access_point.rs` - Trait async `AccessPoint`. Deux implémentations :
  - `LoggingAccessPoint` - dev/no-op (défaut dans la build cert tant
    qu'aucune clé d'AP live n'est configurée). Retourne un
    `DeliveryReceipt` synthétique pour que l'opérateur puisse quand
    même répéter le flux B2B de bout en bout.
  - `StorecoveAccessPoint` - squelette de production. `submit()`
    retourne une erreur explicite "not wired up" qui pointe vers ce
    document.
- `errors.rs` - Enum `PeppolError` (`Xml`, `MissingField`,
  `AccessPoint`, `Http`).

IPC : `src-tauri/src/commands/invoice.rs::issue_b2b_invoice`. Construit
l'UBL à partir des lignes d'articles + détails fournisseur/acheteur,
soumet via l'`AccessPoint` configuré, persiste dans `peppol_deliveries`.

Schéma : table `peppol_deliveries` (append-only, même règle de
rétention que `fiscal_events`). Colonnes : `fiscal_event_id`,
`invoice_no`, `customer_endpoint_id`, `ap_vendor`, `ap_message_id`,
`ubl_xml`, `ap_response_json`, `status` (queued / accepted / failed),
`error_msg`, `created_at`, `accepted_at`.

Feature flag : `peppol` (default-on, inclus dans la build `cert`).

## Plan de déploiement (côté opérateur, post-cert)

1. **Enregistrer le Peppol endpoint-ID de Tokyo Sushi Bar.** Le TVA
   belge utilise le schéma `0208` (numéro d'entreprise BCE, 10 chiffres
   sans préfixe `BE`) ; l'endpoint-ID est `0208:0772499585`. L'enregistrement est effectué par l'AP choisi
   à l'onboarding - prend typiquement 1 à 2 jours ouvrables.
2. **Ouvrir un compte Storecove**, demander une clé API sandbox,
   exécuter un test de bout en bout contre leur récepteur de
   référence (Storecove fournit un endpoint de test qui renvoie l'UBL
   en écho).
3. **Câbler les credentials.** Définir
   `PEPPOL_AP_BASE_URL=https://api.storecove.com` et
   `PEPPOL_AP_API_KEY=...` dans l'environnement kiosk. Basculer la
   factory `AccessPoint` dans `commands/invoice.rs` de
   `LoggingAccessPoint` vers `StorecoveAccessPoint`.
4. **Compléter le corps de `StorecoveAccessPoint::submit()`** -
   envelopper l'UBL dans le JSON `LegalDocumentInvoice` de Storecove,
   POST sur `/v2/legal_documents`, parser le `document_id` dans notre
   `message_id`. Déjà ébauché dans les commentaires inline.
5. **Smoke test production** - émettre une facture de 1 € vers un
   récepteur de test connu, vérifier la délivrance dans le dashboard
   Storecove, vérifier que la ligne `peppol_deliveries` transitionne
   queued → accepted.

## Pourquoi cela est acceptable pour la soumission cert

- L'obligation Peppol est **séparée** de SCE-2.0 - la revue
  certification SPF Finances porte sur le flux FDM/POS, pas sur le
  transport AS4.
- L'UBL est **entièrement généré** aujourd'hui ; seul le câblage HTTP
  vers l'AP reste, gardé derrière un feature flag et des credentials
  explicites.
- La table d'audit `peppol_deliveries` est dans le schéma, prête à
  recevoir des lignes le jour où l'AP est câblé.
- Le périmètre déclaré dans le formulaire de demande cert est
  "E-facturation B2B Peppol : dans le périmètre, achat d'AP en
  cours" - pleinement véridique.

## Risques

- **Panne AP** - si Storecove est down, l'opérateur reçoit une erreur
  UI et peut redéclencher depuis l'écran de détail commande. Nous ne
  perdons PAS silencieusement le document ; la ligne `peppol_deliveries`
  reste en `status='failed'` avec `error_msg` rempli, et apparaît
  donc dans le rapport de fin de journée comme action à traiter.
- **Coût par facture** - à moins de 5 factures B2B par mois, Tokyo
  Sushi Bar est confortablement sous le seuil où le coût AP devient
  sensible.
- **Verrouillage AP** - changer d'AP plus tard est un changement de
  config + un ajustement d'enveloppe UBL, sans migration DB nécessaire
  (la colonne `ap_vendor` stocke intentionnellement la chaîne du
  fournisseur pour l'audit mais n'est pas une référence dure).
