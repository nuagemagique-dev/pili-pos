<!-- Dérivé de docs/SUBMISSION_ARTIFACTS/LISTE_FONCTIONNALITES_FR.md @ 86088e4 -->
# Liste des fonctionnalités - pili-pos-android v1.0.0

**Produit :** pili-pos-android 1.0.0 (posSwVersion pili-pos-android/1.0.0)
**Plateforme :** Android 13 (API 33), terminal Sunmi D3 Pro (variante avec services Google)

Inventaire exhaustif des fonctionnalités du système de caisse
`pili-pos-android` dans sa version soumise à certification (v1.0.0,
moteur construit avec `default-features = false, features = ["cert"]`,
`android/rust/pos-ffi/Cargo.toml`). Les fonctionnalités fiscales (§1) et
métier (§2) sont celles du moteur commun à `pili-pos` ; ce que l'interface
Android en offre est décrit au §7.

## 1. Fonctionnalités fiscales (SCE 2.0)

### 1.1 Événements gérés

| Code | Type | Mutation FDM | Implémentation |
|---|---|---|---|
| N | Vente normale | `signSale` | `crates/pos-engine/src/commands/checkout.rs::do_checkout` (ou `do_checkout_split` pour plusieurs moyens de paiement), qui passent par `sell` puis `signing::sign` |
| N (rembours.) | Remboursement complet ou partiel (M111, M112B) | `signSale` avec lignes négatives + `fdmRef` | `crates/pos-engine/src/commands/refund.rs::do_refund_checkout` |
| P | Commande en cuisine (pro-forma) | `signOrder` | `crates/pos-engine/src/commands/tables.rs::do_sign_table_order` |
| P | Addition (pré-ticket) | `signPreBill` | `crates/pos-engine/src/commands/tables.rs::do_sign_pre_bill` |
| P | Changement de table (transfert d'une commande ouverte) | `signCostCenterChange` | `crates/pos-engine/src/commands/table_transfer.rs::do_transfer_table` |
| T | Vente en formation | `signSale` (isTraining=true) | `do_checkout` / `do_checkout_split` avec `training_mode=true` |
| T | Présence en formation (aucune prestation n'est ouverte dans `staff_shifts` ; le Gérant ne peut pas pointer un autre employé en formation, ni un badge pointer en formation) | `signWorkIn` (isTraining=true) | `social::do_work_in` avec `training_mode=true` ; le pointage par badge est refusé en formation (`presence_training`), la connexion par badge reste possible |
| F | Correction du moyen de paiement d'une vente (Gérant + PIN) | `signPaymentCorrection` | `crates/pos-engine/src/commands/payment_correction.rs::do_correct_payment` |
| C | Copie de ticket, ou d'une facture (réservée au gérant) | `signCopy` | `crates/pos-engine/src/commands/copy.rs::do_copy_ticket` |
| I | Facture B2B (client professionnel, à partir de ventes déjà encaissées) | `signInvoice` | `crates/pos-engine/src/commands/invoice.rs::do_sign_invoice_fiscal` ; impression `crates/pos-print/src/invoice.rs` ; envoi Peppol `crates/pos-engine/src/commands/peppol_queue.rs` (*FACTURATION_PEPPOL_FR.md*) |
| F | Mouvement de fonds | `signMoneyInOut` | `crates/pos-engine/src/commands/financial.rs::do_money_in_out` |
| F-WAR | Ouverture tiroir hors-vente | `signDrawerOpen` | `crates/pos-engine/src/commands/financial.rs::do_open_drawer_fiscal` |
| S | Pointage entrée | `signWorkIn` | `crates/pos-engine/src/commands/social.rs::do_work_in` |
| S | Pointage sortie | `signWorkOut` | `crates/pos-engine/src/commands/social.rs::do_work_out` |
| S | Pointage d'un autre employé par le Gérant (arrivée, départ ; profil « Présence seule ») | `signWorkIn` / `signWorkOut` au nom de l'employé | `crates/pos-engine/src/commands/social.rs::{do_clock_in_employee,do_clock_out_employee}` |
| S | Pointage par badge (arrivée, départ), depuis n'importe quel écran, sans déconnecter l'opérateur ; option désactivée par défaut, badge lié par un Gérant avec son PIN, empreinte HMAC seule conservée | `signWorkIn` / `signWorkOut` au nom du titulaire du badge | `crates/pos-engine/src/commands/badges.rs::do_badge_tap` (lien : `do_link_badge`, retrait : `do_revoke_badge`), signature `crates/pos-engine/src/commands/social.rs::sign_work_event` |
| S | Pointage de fin de service par badge sur l'écran de connexion (départ seul, sans session) | `signWorkOut` au nom du titulaire du badge | `crates/pos-engine/src/commands/badges.rs::do_badge_tap` avec `end_shift` |
| R | Rapport X (intermédiaire) | `signReportTurnoverX` | `crates/pos-engine/src/commands/report.rs::do_generate_x_report` |
| R | Rapport utilisateurs X | `signReportUserX` | `crates/pos-engine/src/commands/report.rs::do_generate_user_x_report` |
| R | Rapport Z (clôture, M184) | `signReportTurnoverZ` + `signReportUserZ` (une seule opération GraphQL) | `crates/pos-engine/src/commands/report.rs::do_generate_z_report` |

### 1.2 Codes TVA gérés

| Code | Taux | Usage |
|---|---|---|
| A | 21 % | Boissons (alcool, soft drinks) |
| B | 12 % | Service en salle (cuisine consommée sur place) |
| C | 6 % | À emporter (cuisine non consommée sur place) |
| D | 0 % | Produits exonérés |
| X | - | Hors champ TVA (vente d'un bon cadeau, *BONS_CADEAUX_FR.md*) |

Implémentation : `crates/pos-core/src/vat.rs::VatCode`.

### 1.3 Mécanismes de signature et compteurs

- **Compteurs séquentiels par type d'événement** - voir
  `STRATEGIE_COMPTEURS_FR.md`.
- **Allocation en transaction atomique** (peek-then-commit) qui
  garantit qu'un échec FDM ne crée jamais de trou dans la séquence
  - voir `STRATEGIE_COMPTEURS_FR.md` §5 et le test de régression
  `pili-pos::spf_e2e_no_gap`.
- **Contrôle de chaque réponse FDM avant archivage** (écho de la
  requête, opération et label, shortSignature dérivée de la signature
  pour N) - `crates/pos-fiscal/src/response_check.rs`. Une réponse
  incohérente n'est jamais archivée.
- **JSON canonique** (RFC 8785) pour la reproductibilité des
  signatures - `crates/pos-fiscal/src/canonical.rs`.
- **Clé de doublon FDM**
  `{posId, posDateTime, terminalId, eventLabel, posFiscalTicketNo}`
  - `crates/pos-fiscal/src/inputs.rs::IdempotencyKey`.
- **Verrou de signature unique** : toute opération qui signe un
  événement FDM s'exécute sous un seul verrou, le numéro de ticket est
  lu à l'intérieur - `crates/pos-engine/src/commands/signing.rs`.
- **Renvoi à l'identique d'une réponse perdue** : la requête exacte est
  stockée avant envoi (`pending_signs`, migration 10) et renvoyée octet
  pour octet tant que le FDM n'a pas répondu ; l'événement est archivé
  une fois, sous son premier numéro, avec ses effets.

### 1.4 Archivage local

- Table `fiscal_events` (append-only, jamais de DELETE ni UPDATE ;
  des triggers SQLite le font respecter).
- Sauvegarde automatique de la base SQLite après chaque rapport Z,
  juste après son archivage, via l'API Online Backup (copie chiffrée
  hors site facultative : §1.7).
- Rétention du journal fiscal : indéfinie (au moins 7 ans, durée légale
  belge). Seules les 7 copies de sauvegarde les plus récentes sont
  conservées, dans `files/engine/backups/` (`SECURITE_BD_FR.md` §5).

### 1.5 Ticket fiscal imprimé

Chaque ticket contient (cf. `crates/pos-print/src/receipt.rs`) :
- En-tête restaurant : nom, adresse, numéro de TVA, numéro
  d'établissement.
- Lignes : quantité, désignation, prix unitaire, code TVA, total.
- Tableau récapitulatif TVA par taux.
- Bloc "Control data" du FDM : `fdmId`, `fdmDateTime`,
  `eventLabel`, `eventCounter`, `totalCounter`, `shortSignature`.
- QR Code Model 2, ECC M, version 3 (29×29 modules), contenu = URL
  38 caractères fournie par le FDM (`crates/pos-fiscal/src/qr.rs`).
- Mention "TRAINING - NE TIENT PAS LIEU DE TICKET TVA VALABLE" en
  filigrane si `isTraining=true`.

### 1.6 Ticket numérique par e-mail (§III.H)

- Support choisi par l'opérateur avant signature : Papier (défaut) /
  E-mail / Les deux (`commands::delivery::{resolve, resolve_with}`),
  encodé dans le corps signé (`ticketMedium`).
- Mise en file dans la même transaction que l'archive fiscale
  (`delivery::enqueue_in_tx`, table `ticket_deliveries`, migration
  `005`) ; envoi immédiat borné à 8 s, repli papier automatique de
  l'original sinon (`delivery::deliver`).
- Réessais en tâche de fond avec recul exponentiel (1/5/15/60 min,
  jusqu'à 5 tentatives) via `pos-mail::outbox`, toutes les 30 s
  (`commands::delivery::run_outbox_once`).
- Original imprimé une seule fois (POS spec v2.2 §8.1) - table
  `original_prints` (migration `006`), `delivery::print_original_once`
  refuse toute seconde impression de l'original ; toute reproduction
  passe par l'événement C.
- Adresse en clair effacée dès l'envoi/repli réussi et purgée après
  72 h maximum (RGPD) ; seul un indice masqué est conservé.
- Disponible pour la vente/remboursement (N), la formation (T, même
  commande) et la copie (C). Non câblé pour les événements P, F, I, R,
  qui restent papier uniquement en v1.0.0.
- Détail complet : `TICKET_NUMERIQUE_FR.md`.

### 1.7 Export nocturne (facultatif)

Absent tant que la section `export` de `pos.json` n'est pas renseignée.
Détail complet : `EXPORT_NOCTURNE_FR.md`.

- Après chaque rapport Z réel (manuel ou automatique, jamais en mode
  formation), production de trois fichiers : une sauvegarde chiffrée
  de `pos.db` (age, X25519, pour le propriétaire de l'établissement et,
  en option, le producteur), une copie statistique nettoyée de la
  période close (opérateurs sous pseudonyme, aucune donnée personnelle
  directement identifiante) et un manifeste avec les empreintes SHA-256.
- Envoi HTTPS (`PUT`) vers le point de réception propre à l'installation,
  jeton dans un fichier séparé, manifeste envoyé en dernier ; nouvelles
  tentatives de 1 à 60 minutes, sans limite du nombre d'essais pendant
  `keep_days` jours.
- Tâche de fond sur une connexion SQLite en lecture seule, hors du
  verrou de signature : elle ne retarde jamais une vente ni un Z.
- Copies locales conservées `keep_days` jours (14 par défaut), dans la
  mémoire interne, jamais sur une carte SD.
- État et nouvel essai réservés au Gérant (panneau « Export nocturne »
  de « Config » sur Linux, écran dédié sur Android).
- Le journal légal `fiscal_events` reste local ; l'export est une copie
  et ne fait pas du POS un POS dans le cloud.

## 2. Fonctionnalités métier

### 2.1 Prise de commande

- Plan de tables (écran « Tables », `TablesScreen.kt`) avec statut "libre" / "occupée" ;
  nombre de tables réglé par `table_count` (20 par défaut, 25 sur
  l'image d'admission, comme le jeu de test SPF).
- Grille catégories + items (écran de commande, `MenuGrid.kt`)
  optimisée tactile (boutons ≥ 48 × 48 px).
- Modificateurs par item (sauces, cuisson...) - table `modifier_groups`.
- Réutilisation d'une commande ouverte vide pour la même table
  (anti-doublon, `crates/pos-db/src/queries/orders.rs::create_order`).
- Changement de table : « Changer de table » dans la commande, ou « Déplacer une table » sur le plan ;
  la commande garde son contenu et sa référence et passe à une table libre. Une table occupée est
  refusée (pas de fusion) ; un bon de transfert sans prix est imprimé au bar.
- Note de cuisine par ligne (« Note » dans le panier, tout le personnel) :
  60 caractères au plus, imprimables par l'imprimante du bar ou de la
  cuisine ; la note s'imprime sous l'article sur le bon de commande, et une
  note modifiée après l'envoi imprime un bon « MODIFICATION » sans quantité
  ni prix (`crates/pos-engine/src/commands/order.rs::do_set_line_note`). La
  note reste dans la base locale (`order_lines`, migration 31) : elle n'est
  jamais envoyée au FDM ni imprimée sur un ticket TVA, une addition, une
  copie ou un export.
- **Produit épuisé** : un caissier ou un gérant déclare un produit
  « Épuisé » jusqu'à la fin de la journée commerciale ou jusqu'à
  réactivation ; la caisse refuse de l'ajouter à une commande ou
  d'augmenter sa quantité (`product_sold_out`). Journal en ajout seul
  (`sold_out_log`), transmission à l'inventaire HubRise si l'intégration
  est activée. Aucun effet fiscal (*PRODUIT_EPUISE_FR.md*).

### 2.2 Encaissement

- Modes de paiement : espèces, carte, chèques-repas électroniques et bons cadeaux
  (ligne `VOUCHER_STORE`, §2.10)
  (carte : saisie manuelle - pas de terminal intégré dans cette version).
- Paiement en plusieurs moyens sur un seul ticket : carte et
  chèques-repas pour un montant exact, espèces pour le reste
  (`crates/pos-fiscal/src/payment.rs::settle`).
- **Arrondi 5 cents légal belge** sur le reste payé en espèces
  seulement (`crates/pos-fiscal/src/ticket.rs::round_to_5_cents`).
- Ouverture automatique du tiroir-caisse via commande ESC/POS
  `0x1B 0x70 0x00 0x19 0xFA` quand une partie est payée en espèces.
- Vente sans paiement (client parti sans payer) : Gérant avec PIN,
  N signé sans ligne de paiement, ticket "Non paye", jamais
  remboursable (`crates/pos-engine/src/commands/unpaid.rs`).
- Ventes à emporter en cours listées sur l'écran des tables, jamais
  enregistrées en P (FAQ Q205).
- Impression du ticket fiscal après acceptation FDM (automatique pour
  Papier/Les deux ; en secours seulement, si l'e-mail n'aboutit pas
  dans le budget de 8 s, pour un ticket "E-mail" seul - §1.6).

### 2.3 Rapports

- **Rapport X** : tout moment de la journée, non remettant à zéro.
- **Rapport Z** : clôture de journée fiscale, démarre une nouvelle
  période de réservation (UUID régénéré).
- Bannière d'avertissement "Z non encore généré" affichée à partir
  de 22 h si aucun Z du jour
  (`commands::report::do_needs_z_report`).
- **Z automatique d'une journée en retard** : si la période d'ouverture
  appartient à une journée terminée (heure de fin de journée
  `day_end_hour`, 5 h par défaut) et n'a pas de Z, le POS le génère
  lui-même au nom de l'identité système
  (`commands::report::close_overdue_day`, au démarrage puis toutes les
  60 s ; FAQ producteurs Q501c). Une journée sans activité n'a pas de Z.
- Sauvegarde automatique de la base après chaque Z, juste après son
  archivage.
- **Rapport de ventes (non fiscal)** : quatrième onglet « Ventes » des
  Rapports, par produit, par heure et avec la concordance avec les
  rapports Z ; imprimable sous le titre « RAPPORT DE VENTES - DOCUMENT
  NON FISCAL ». Il ne signe rien et ne touche à aucun compteur
  (*RAPPORTS_VENTES_FR.md*).

### 2.4 Mode formation

- Drapeau global `training_mode` dans `PosConfig`.
- Aucun événement T ne touche aux compteurs N / S / R productifs.
- Filigrane visuel plein écran et bandeau de formation (`TrainingBanner.kt`).
- Mention sur tous les tickets imprimés en mode formation.

### 2.5 Multi-langue

- Schéma `translations` polymorphe : entité × champ × langue × valeur
  (`crates/pos-db/src/schema.rs`).
- Langues supportées : 中文 (zh, principale pour le personnel),
  Français (fr, principale pour les clients et obligation légale),
  Nederlands (nl), Deutsch (de, migration `007`), English (en).
- Langue des tickets fixée par installation (`pos.json::language`, FR, NL ou DE ; `GARANTIE_MAJ_LANGUES_FR.md` §3) ; pas de choix par utilisateur. L'interface Android est en français, en anglais ou en chinois simplifié.

### 2.6 Édition de la carte

Un gérant crée et modifie les catégories, les produits (prix, TVA sur
place et à emporter, noms en français, néerlandais, anglais et chinois),
les groupes d'options et les options directement à la caisse, dans
"Config", panneau "Carte". Rien n'est supprimé (un élément est masqué),
chaque modification est journalisée en ajout seul (`menu_change_log`),
l'éditeur est verrouillé tant qu'un catalogue externe (HubRise) gère la
carte. Aucun effet fiscal : une commande ouverte et un événement signé
gardent le nom, le prix, la TVA et le département d'origine (arrêté
ministériel, art. 44). Voir *EDITEUR_CARTE_FR.md*.

### 2.7 Facturation B2B

Une facture à un client professionnel est établie à partir d'une ou de
plusieurs ventes déjà encaissées (Rapports, onglet Tickets, « Facture ») :
choix ou création du client (numéro de TVA contrôlé hors ligne), aperçu,
signature (événement I), impression papier, copie réservée au gérant,
liste des factures. Une vente figure sur une seule facture ; numérotation
continue ; le remboursement (N négatif) peut figurer sur une facture. Le
journal des factures (art. 33) est la vue `v_invoice_journal`. Voir
*FACTURATION_PEPPOL_FR.md*.

### 2.8 Allergènes (information, non fiscale)

Les 14 allergènes à déclaration obligatoire (annexe II du règlement (UE) n° 1169/2011) s'affichent sur les
produits et les options (pastilles numérotées, mode « Allergènes », fiche du produit) et s'impriment sur une fiche
non fiscale, sans prix, pour un produit ou toute la carte. Listes importées du catalogue HubRise ou saisies par un
gérant dans l'éditeur de carte ; une absence de déclaration s'affiche « information non disponible », jamais
« aucun allergène ». Aucun allergène n'entre dans une demande signée. Voir *ALLERGENES_FR.md*.

### 2.9 Service en plusieurs temps (cuisine, non fiscal)

Fonction éteinte par défaut, réglée par un gérant : chaque catégorie a un temps (Direct, Entrées, Plats, Desserts),
le premier ticket de cuisine imprime le Direct et les entrées, « Envoyer la suite » imprime ensuite les plats puis
les desserts. Le temps d'une ligne n'entre dans aucune demande signée, aucun nom de produit ni aucun ticket de
T.V.A. ; éteinte, les tickets sont identiques octet pour octet. Voir *SERVICES_CUISINE_FR.md*.

### 2.10 Bons cadeaux à usage multiple (fiscal)

Fonction éteinte par défaut, réglée par un gérant. Vente d'un bon : ligne de T.V.A. X (hors champ), département
`GV`, code à 12 chiffres dont 2 de contrôle, bon imprimé. Utilisation : méthode de paiement « Bon cadeau »,
ligne `VOUCHER_STORE` (identité « 8 », saisie `MANUAL`), montant exact jamais arrondi, solde conservé sur le bon,
sans rendu de monnaie. Cas officiels UC260, UC261 et UC262 rejoués (*RESULTATS_TESTS_UC_FR.md*). Un bon vendu et
non utilisé se rembourse (le bon est annulé) ; une vente payée avec un bon se rembourse en espèces (choix de
l'établissement). Voir *BONS_CADEAUX_FR.md*. Non offerts en v1.0.0 : bons à usage unique (UC270, UC271),
conversion d'un bon en espèces (UC4105), remboursement vers un bon.

### 2.11 Alertes du propriétaire sur téléphone (non fiscales)

Fonction éteinte par défaut : un court message Telegram quand le module fiscal ne répond plus, qu'une imprimante
ou le terminal de carte est en défaut, que le Z du jour n'est pas fait ou qu'une commande en ligne attend. Aucune
donnée personnelle ni montant dans les messages ; la caisse n'attend jamais l'envoi. Voir
*ALERTES_PROPRIETAIRE_FR.md*.

### 2.12 Outil d'export pour le comptable (hors du binaire certifié)

Un outil en ligne de commande distinct, `pili-accounting` (`crates/pos-accounting`), produit pour le comptable des
fichiers CSV et un journal équilibré à partir d'une copie de l'export de la caisse ou d'une sauvegarde. Il n'est pas
dans le binaire certifié ni dans le paquet livré avec la caisse : aucune crate livrée ne dépend de lui (test
d'isolation), il ne lit jamais la base en service (une copie en service est refusée), n'écrit jamais dans la source,
ne signe rien et n'envoie rien au module fiscal. Il ne touche donc aucun événement fiscal ; il ne figure ici que
pour expliquer qu'il est hors du périmètre du binaire soumis.

## 3. Commandes en ligne (plateformes et webshop)

Présentes dans le binaire certifié, activées par installation
(`hubrise_enabled` dans `pos.json`, puis un Gérant dans Réglages, chaque
changement journalisé). Détail : `COMMANDES_EN_LIGNE_FR.md`.

- Réception des commandes Deliveroo, Uber Eats et du webshop par
  HubRise ; contrôle contre le menu de la caisse.
- Commande payée : N à l'acceptation, paiement `APP` ou `ONLINE`,
  ticket de caisse TVA papier.
- Commande à payer : P à l'acceptation, N à la remise avec le même
  `costCenter` (FAQ Q203 b).
- Commande refusée : jamais signée. Annulation après le N :
  remboursement confirmé par un Gérant, avec `fdmRef`. Annulation après
  le P : corrections et N à 0,00 EUR.
- Remises en `priceChanges` ; frais des plateformes exclus par défaut.
- Remise à la caisse : pourcentage, montant ou « Offert » sur la commande
  ou sur une ligne, confirmée par le PIN d'un Gérant, signée en
  `priceChanges` (`TILL-DISCOUNT`, `TILL-FREE`, et `TILL-FREE-LINE` pour une ligne offerte, portée LINE) ; le motif reste dans la
  table locale `till_discounts`.
- Menu de la caisse reflet du catalogue HubRise, verrouillé pendant la
  connexion.

## 3 bis. Fonctionnalités exclues (hors scope cert)

- **Crédit client et compte client différé** : absents ; les ventes des
  cas UC4121 et UC4122 payées « crédit client » sont rejouées au comptant.
- **Facture en mode formation** : refusée. **Envoi Peppol vers un client
  étranger** : la facture est imprimée seulement.
- **Note de crédit autonome** : absente (un remboursement se reprend sur
  une facture, *FLUX_REMBOURSEMENT_FR.md* §3).
- **Bons cadeaux : non offerts en v1.0.0** : bons à usage unique (UC270, UC271 : leur utilisation reprend le
  découpage de T.V.A. d'un produit composé), conversion d'un bon en espèces (UC4105), remboursement vers un bon
  (une vente payée avec un bon se rembourse en espèces).
- **Division et fusion de tables** : absentes ; seul le transfert d'une
  commande vers une table libre (changement de centre de coût, M122) est
  proposé. **Correction d'un paiement par terminal** : une carte prise sur
  le terminal de paiement relié ne se corrige pas ; la correction de
  paiement (M132) porte sur une ligne saisie à la main.
- **Assistance à distance** : aucun outil d'assistance sur Android ; le support se fait sur place (§7.3).

## 4. Intégration Peppol (B2B)

Dans tous les binaires, sans fonction Cargo. Voir `FACTURATION_PEPPOL_FR.md` (règles, file d'envoi) et `DECISION_PEPPOL_AP_FR.md` (point d'accès).

- Sérialiseur UBL 2.1 BIS Billing 3.0 (`crates/pos-peppol/src/ubl.rs`).
- Trait `AccessPoint` (`crates/pos-peppol/src/access_point.rs`) avec
  `StorecoveAccessPoint` (API Storecove v2, recherche du destinataire et
  envoi) et `LoggingAccessPoint` (développement, refusé en production).
- File d'envoi avec nouvelles tentatives et journal en ajout seul
  (`crates/pos-engine/src/commands/peppol_queue.rs`, tables
  `peppol_deliveries` et `peppol_delivery_log`) ; panneau Peppol du
  gérant (envois en attente, remis, bloqués, relance).
- Réglages : section `peppol` de `pos.json` (*MANUEL_INSTALLATION_FR.md*).

Le déploiement avec un Access Point en production (compte, clé,
essai sur le bac à sable) est une étape de l'exploitant indépendante de
la certification SCE 2.0.

## 5. Versions de spécifications implémentées

| Spécification | Version | Date |
|---|---|---|
| Description détaillée POS | v2.2 | 2026-04-17 |
| Description détaillée FDM | v1.4 | 2026-04-17 |
| Addendum 1 (POS/FDM) | v2.1 ES | 2026-02-09 |
| Use cases Basic Set | v1.8 | 2026-08-21 |
| Use cases Reports | v1.3 | 2026-08-21 |
| Use cases Advanced Set | v1.6 | 2026-08-21 |
| Peppol BIS Billing | 3.0 | Présent dans la build Android (`pos-peppol`, dépendance du moteur) |
| FPSFIN API Protocol FDM-FinCloud | 1.10 | 2026-04-14 |

## 6. Périphériques pris en charge

- **Terminal POS** : Sunmi D3 Pro, variante avec services Google (Android
  13, API 33, arm64-v8a, écran 1920 × 1080 en paysage). Cf.
  `PLATEFORME_ANDROID_FR.md` §1. Faits matériels du vrai terminal : À
  effectuer (EV-15 : fiche mesurée d'un vrai D3 Pro, ou déclaration que la
  machine de test est le profil émulé).
- **Imprimante thermique** : Epson TM-m30III, raccordée en LAN
  (TCP 9100). Protocole ESC/POS.
- **Tiroir-caisse** : standard RJ11 connecté à l'imprimante.
- **FDM** : module2 (PRACSYS, certifié SCE 2.0). Connexion LAN
  (GraphQL en HTTP, ou HTTPS avec mTLS), jeton Bearer propre au FDM.
  Machine de test : simulateur en ligne module2.

## 7. Interface Android et parité avec l'interface Linux

L'interface Kotlin et Compose reprend écran par écran l'interface Vue de `pili-pos` et n'ajoute aucune règle fiscale : l'interface affiche et transmet, le moteur décide (`DOCUMENT_PROCESSUS_TECHNIQUE_FR.md` §1). L'index `docs/android/PARITY.md`, contrôlé par `scripts/android/check_ui_parity.py`, rattache chaque composant Vue à son équivalent Compose et chaque commande à son appelant. Sur les 146 commandes que l'interface Linux utilise (la build de certification en compte 148), **136 sont appelées** par l'interface Android et **10 sont exclues**, avec leurs raisons (§7.2) ; aucune n'est manquante ni en attente. Les commandes de facture, de produit épuisé, d'édition de la carte et de rapport de ventes sont parmi les 136.

### 7.1 Écrans offerts

- Connexion par PIN, verrouillage, fin de service, version affichée (`posSwVersion`) : §1.1 (S), §2.1.
- Plan des tables et prise de commande avec modificateurs : §2.1.
- Encaissement et ticket papier ou e-mail : §1.5, §1.6, §2.2 ; vente sans paiement : §2.2.
- Rapports X et Z, copie de ticket, remboursement, mouvements de caisse : §1.1, §2.3.
- Réglages (employés, mode formation, envois de tickets, interrupteur HubRise) : §2.4, §3.
- Commandes en ligne : §3. Export nocturne : §1.7.
- Facturation B2B (choix des ventes, client, aperçu, liste des factures, panneau Peppol) : §2.7. Produit épuisé : §2.1. Édition de la carte : §2.6. Rapport de ventes : §2.3.
- Allergènes : §2.8. Service en plusieurs temps : §2.9. Bons cadeaux (bouton de commande, méthode d'encaissement, onglet de rapports) : §2.10. Alertes du propriétaire (réglages) : §2.11.

### 7.2 Commandes exclues (10)

| Commandes | Raison |
|---|---|
| `start_remote_support`, `stop_remote_support`, `remote_support_status`, `remote_support_running` | Assistance à distance absente de la version 1.0 (§7.3) |
| `check_fdm_status` | Sans appelant côté Vue ; `fdm_health` et `fdm_check_now` le remplacent |
| `list_fiscal_events` | Sans appelant côté Vue |
| `sign_table_order` | Sans appelant côté Vue ; `leave_order` enregistre l'événement P |
| `reprint_course_slip` | Aucun écran dans la version 1.0 : la réimpression en DUPLICATA d'un bon de service en plusieurs temps reste une commande du moteur seul (`SERVICES_CUISINE_FR.md`) |
| `work_in`, `work_out` | Sans appelant côté Vue ; la connexion fait entrer et `end_shift` fait sortir ; le Gérant pointe un autre employé avec `clock_in_employee` et `clock_out_employee` |

### 7.3 Assistance à distance absente

La version 1.0 ne comporte aucun outil d'assistance à distance : le moteur est ouvert sans programme de support, l'interface ne propose ni écran ni bandeau d'assistance, et aucun port n'écoute (`PLATEFORME_ANDROID_FR.md` §6 et §7).

### 7.4 Fonctions propres à Android

Elles n'interviennent pas dans le calcul fiscal : provisionnement par lot signé sur clé USB, propriétaire de l'appareil et mode kiosque (`PLATEFORME_ANDROID_FR.md` §3), écran « À propos » (§3.9), export pour le SPF sur clé USB ou carte SD (`SECURITE_BD_FR.md` §8) et mise à jour lancée par le Gérant (`GARANTIE_MAJ_LANGUES_FR.md` §2.1).
