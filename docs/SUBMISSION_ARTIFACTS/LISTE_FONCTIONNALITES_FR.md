# Liste des fonctionnalités - pili-pos v1.0.0

Inventaire exhaustif des fonctionnalités du système de caisse
`pili-pos` dans sa version soumise à certification (v1.0.0,
build `--no-default-features --features cert`).

## 1. Fonctionnalités fiscales (SCE 2.0)

### 1.1 Événements gérés

| Code | Type | Mutation FDM | Implémentation |
|---|---|---|---|
| N | Vente normale | `signSale` | `crates/pos-engine/src/commands/checkout.rs::do_checkout` (ou `do_checkout_split` pour plusieurs moyens de paiement), qui passent par `sell` puis `signing::sign` |
| N (rembours.) | Remboursement complet ou partiel (M111, M112B) | `signSale` avec lignes négatives + `fdmRef` | `crates/pos-engine/src/commands/refund.rs::do_refund_checkout` |
| P | Commande en cuisine (pro-forma) | `signOrder` | `crates/pos-engine/src/commands/tables.rs::do_sign_table_order` |
| P | Addition (pré-ticket) | `signPreBill` | `crates/pos-engine/src/commands/tables.rs::do_sign_pre_bill` |
| T | Vente en formation | `signSale` (isTraining=true) | `do_checkout` / `do_checkout_split` avec `training_mode=true` |
| T | Présence en formation (aucune prestation n'est ouverte dans `staff_shifts` ; le Gérant ne peut pas pointer un autre employé en formation) | `signWorkIn` (isTraining=true) | `social::do_work_in` avec `training_mode=true` |
| C | Copie de ticket | `signCopy` | `crates/pos-engine/src/commands/copy.rs::do_copy_ticket` |
| I | Facture B2B (hors binaire certifié v1.0.0, voir §3 bis) | `signInvoice` | `crates/pos-engine/src/commands/invoice.rs::do_sign_invoice_fiscal` (émission Peppol/UBL séparée : `do_issue_b2b_invoice`), sous `#[cfg(feature = "peppol")]` |
| F | Mouvement de fonds | `signMoneyInOut` | `crates/pos-engine/src/commands/financial.rs::do_money_in_out` |
| F-WAR | Ouverture tiroir hors-vente | `signDrawerOpen` | `crates/pos-engine/src/commands/financial.rs::do_open_drawer_fiscal` |
| S | Pointage entrée | `signWorkIn` | `crates/pos-engine/src/commands/social.rs::do_work_in` |
| S | Pointage sortie | `signWorkOut` | `crates/pos-engine/src/commands/social.rs::do_work_out` |
| S | Pointage d'un autre employé par le Gérant (arrivée, départ ; profil « Présence seule ») | `signWorkIn` / `signWorkOut` au nom de l'employé | `crates/pos-engine/src/commands/social.rs::{do_clock_in_employee,do_clock_out_employee}` |
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
| X | - | Hors champ TVA |

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
  juste après son archivage, via l'API Online Backup.
- Rétention : indéfinie (au moins 7 ans, durée légale belge).

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

## 2. Fonctionnalités métier

### 2.1 Prise de commande

- Plan de tables (`TablesView.vue`) avec statut "libre" / "occupée" ;
  nombre de tables réglé par `table_count` (20 par défaut, 25 sur
  l'image d'admission, comme le jeu de test SPF), ou numéros choisis
  par la clé `tables` de `pos.json` (nombres et plages, de 1 à 999,
  200 tables au plus).
- Plan de salle modifiable par un Gérant dans "Config", panneau "Plan
  de salle" ; chaque changement est journalisé en ajout seul
  (`table_plan_log`). Une table hors plan qui a encore une commande
  ouverte reste affichée : aucune commande non réglée n'est masquée.
- Grille catégories + items (`OrderView.vue`) optimisée tactile (boutons
  ≥ 48 × 48 px).
- Modificateurs par item (sauces, cuisson...) - table `modifier_groups`.
- Réutilisation d'une commande ouverte vide pour la même table
  (anti-doublon, `crates/pos-db/src/queries/orders.rs::create_order`).

### 2.2 Encaissement

- Modes de paiement : espèces, carte et chèques-repas électroniques.
  Par défaut la carte est saisie à la main (terminal autonome, méthode
  `MANUAL`, `CARD_DEBIT`).
- **Terminal de paiement relié (facultatif, par installation)** : réglage
  `payment_terminal` de `pos.json`. La caisse envoie le montant au
  terminal (Worldline Terminal API, CCV Payment API ou OPI) et enregistre
  sa réponse dans `eft_payments` avant et après l'appel. Rien n'est
  débité si le FDM n'est pas opérationnel. La ligne de paiement signée
  est `AUTOMATIC` : le type de carte vient du terminal (`CARD_DEBIT`
  id "2", `CARD_CREDIT` id "6", `CARD_UNKNOWN` id "7"), la marque est
  dans `provider`, l'identifiant de transaction dans `reference` ; le
  ticket imprime la marque. Une réponse inconnue bloque la vente de la
  commande jusqu'à vérification ; un paiement accepté doit être repris
  par la vente. Pas d'arrondi des paiements par carte EFT, pour toute
  installation. Mode formation : aucune vraie carte n'est débitée.
  Remboursement toujours en espèces. Terminal Worldline C-TEP (Yomani,
  Yoximo) non pris en charge (protocole fermé). Voir
  `MANUEL_UTILISATEUR_FR.md` §7.2 et `MANUEL_INSTALLATION_FR.md` §4.2.
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

### 2.4 Mode formation

- Drapeau global `training_mode` dans `PosConfig`.
- Aucun événement T ne touche aux compteurs N / S / R productifs.
- Filigrane visuel rouge plein écran (`TrainingBanner.vue`).
- Mention sur tous les tickets imprimés en mode formation.

### 2.5 Multi-langue

- Schéma `translations` polymorphe : entité × champ × langue × valeur
  (`crates/pos-db/src/schema.rs`).
- Langues supportées : 中文 (zh, principale pour le personnel),
  Français (fr, principale pour les clients et obligation légale),
  Nederlands (nl), Deutsch (de, migration `007`), English (en).
- Langue d'affichage et langue des tickets fixées par installation (`pos.json::language`) ; pas de choix par utilisateur.

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
- Menu de la caisse reflet du catalogue HubRise, verrouillé pendant la
  connexion.

## 3 bis. Fonctionnalités exclues (hors scope cert)

- **Facturation B2B (événement I, Peppol)** : absente du binaire
  certifié (feature Cargo `peppol` hors de `cert`), livrée dans une
  mise à jour certifiée ultérieure. Voir §4.

## 4. Intégration Peppol (B2B)

Hors binaire certifié v1.0.0 (voir §3 bis). Voir `DECISION_PEPPOL_AP_FR.md` pour le détail.

Implémentation présente :
- Sérialiseur UBL 2.1 BIS Billing 3.0 (`crates/pos-peppol/src/ubl.rs`).
- Trait `AccessPoint` (`crates/pos-peppol/src/access_point.rs`) avec
  implémentation `LoggingAccessPoint` (no-op pour développement) et
  stub `StorecoveAccessPoint` (intégration commerciale à finaliser).
- Commande Tauri `issue_b2b_invoice`
  (`crates/pos-engine/src/commands/invoice.rs`).
- Table `peppol_deliveries` pour la traçabilité.

Le déploiement avec un Access Point en production est une étape
contractuelle indépendante de la certification SCE 2.0.

## 5. Versions de spécifications implémentées

| Spécification | Version | Date |
|---|---|---|
| Description détaillée POS | v2.2 | 2026-04-17 |
| Description détaillée FDM | v1.4 | 2026-04-17 |
| Addendum 1 (POS/FDM) | v2.1 ES | 2026-02-09 |
| Use cases Basic Set | v1.8 | 2026-08-21 |
| Use cases Reports | v1.3 | 2026-08-21 |
| Use cases Advanced Set | v1.6 | 2026-08-21 |
| Peppol BIS Billing | 3.0 | - |
| FPSFIN API Protocol FDM-FinCloud | 1.10 | 2026-04-14 |

## 6. Périphériques pris en charge

- **Terminal POS** : Penetek A3216 M82 ou Advantech USC-360J (Intel
  J6412, x86_64, 8 Go RAM, 128 Go SSD, écran 15,6" tactile PCAP
  1920×1080, IP65). Cf. `CHECKLIST_DEPLOIEMENT_KIOSK_FR.md`.
- **Imprimante thermique** : Epson TM-m30III, raccordée en LAN
  (TCP 9100). Protocole ESC/POS.
- **Tiroir-caisse** : standard RJ11 connecté à l'imprimante.
- **FDM** : module2 (PRACSYS, certifié SCE 2.0). Connexion LAN
  (GraphQL en HTTP, ou HTTPS avec mTLS), jeton Bearer propre au FDM.
  Machine de test : simulateur en ligne module2.
