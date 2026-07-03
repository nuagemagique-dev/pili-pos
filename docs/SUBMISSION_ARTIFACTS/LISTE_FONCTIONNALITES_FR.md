# Liste des fonctionnalités - moyun-pos v1.0.0

Inventaire exhaustif des fonctionnalités du système de caisse
`moyun-pos` dans sa version soumise à certification (v1.0.0,
build `--no-default-features --features cert`).

## 1. Fonctionnalités fiscales (SCE 2.0)

### 1.1 Événements gérés

| Code | Type | Mutation FDM | Implémentation |
|---|---|---|---|
| N | Vente normale | `signSale` | `src-tauri/src/commands/checkout.rs::do_checkout` |
| N (rembours.) | Remboursement complet ou partiel (M111, M112B) | `signSale` avec lignes négatives + `fdmRef` | `src-tauri/src/commands/refund.rs::do_refund_checkout` |
| P | Commande en cuisine (pro-forma) | `signOrder` | `src-tauri/src/commands/tables.rs::do_sign_table_order` |
| P | Pré-ticket | `signPreBill` | `src-tauri/src/commands/tables.rs::do_sign_pre_bill` |
| T | Vente en formation | `signSale` (isTraining=true) | `do_checkout` avec `training_mode=true` |
| T | Présence en formation | `signWorkIn` (isTraining=true) | `do_work_in` avec `training_mode=true` |
| C | Copie de ticket | `signCopy` | `src-tauri/src/commands/copy.rs::do_copy_ticket` |
| I | Facture B2B | `signInvoice` | `src-tauri/src/commands/invoice.rs::do_sign_invoice_fiscal` (émission Peppol/UBL séparée : `do_issue_b2b_invoice`) |
| F | Mouvement de fonds | `signMoneyInOut` | `src-tauri/src/commands/financial.rs::do_money_in_out` |
| F-WAR | Ouverture tiroir hors-vente | `signDrawerOpen` | `src-tauri/src/commands/financial.rs::do_open_drawer_fiscal` |
| S | Pointage entrée | `signWorkIn` | `src-tauri/src/commands/social.rs::do_work_in` |
| S | Pointage sortie | `signWorkOut` | `src-tauri/src/commands/social.rs::do_work_out` |
| R | Rapport X (intermédiaire) | `signReportTurnoverX` | `src-tauri/src/commands/report.rs::do_generate_x_report` |
| R | Rapport utilisateurs X | `signReportUserX` | `src-tauri/src/commands/report.rs::do_generate_user_x_report` |
| R | Rapport Z (clôture, M184) | `signReportTurnoverZ` + `signReportUserZ` (une seule opération GraphQL) | `src-tauri/src/commands/report.rs::do_generate_z_report` |

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
  `COUNTER_STRATEGY.md`.
- **Allocation en transaction atomique** (peek-then-commit) qui
  garantit qu'un échec FDM ne crée jamais de trou dans la séquence
  - voir `COUNTER_STRATEGY.md` §3 et le test de régression
  `moyun-pos::spf_e2e_no_gap`.
- **Vérification ECDSA secp256r1** de la signature FDM -
  `crates/pos-fiscal/src/signature.rs::verify_short_signature`.
- **JSON canonique** (RFC 8785) pour la reproductibilité des
  signatures - `crates/pos-fiscal/src/canonical.rs`.
- **Clé d'idempotence**
  `{posId, posDateTime, terminalId, eventLabel, posFiscalTicketNo}`
  - `crates/pos-fiscal/src/inputs.rs::IdempotencyKey`.

### 1.4 Archivage local

- Table `fiscal_events` (append-only, jamais de DELETE ni UPDATE).
- Sauvegarde automatique de la base SQLite avant chaque rapport Z
  via l'API Online Backup.
- Rétention : indéfinie (au moins 7 ans, durée légale belge).

### 1.5 Ticket fiscal imprimé

Chaque ticket contient (cf. `crates/pos-print/src/receipt.rs`) :
- En-tête restaurant : nom, adresse, numéro de TVA, numéro
  d'établissement.
- Lignes : quantité, désignation, prix unitaire, code TVA, total.
- Tableau récapitulatif TVA par taux.
- Bloc "Control data" du FDM : `fdmId`, `fdmDateTime`,
  `eventLabel`, `eventCounter`, `totalCounter`, `shortSignature`.
- QR Code Type 2, 25×25 modules, contenu = URL 38 caractères fournie
  par le FDM (`crates/pos-fiscal/src/qr.rs`).
- Mention "TRAINING - NE TIENT PAS LIEU DE TICKET TVA VALABLE" en
  filigrane si `isTraining=true`.

## 2. Fonctionnalités métier

### 2.1 Prise de commande

- Plan de table jusqu'à 50 tables (`TablesView.vue`) avec statut
  "libre" / "occupée".
- Grille catégories + items (`OrderView.vue`) optimisée tactile (boutons
  ≥ 48 × 48 px).
- Modificateurs par item (sauces, cuisson...) - table `modifier_groups`.
- Réutilisation d'une commande ouverte vide pour la même table
  (anti-doublon, `crates/pos-db/src/queries/orders.rs::create_order`).

### 2.2 Encaissement

- Modes de paiement : espèces, carte (saisie manuelle -
  pas de terminal intégré dans cette version).
- **Arrondi 5 cents légal belge** sur les paiements espèces
  (`crates/pos-fiscal/src/ticket.rs::round_to_5_cents`).
- Ouverture automatique du tiroir-caisse via commande ESC/POS
  `0x1B 0x70 0x00 0x19 0xFA`.
- Impression automatique du ticket fiscal après acceptation FDM.

### 2.3 Rapports

- **Rapport X** : tout moment de la journée, non remettant à zéro.
- **Rapport Z** : clôture de journée fiscale, démarre une nouvelle
  période de réservation (UUID régénéré).
- Bannière d'avertissement "Z non encore généré" affichée à partir
  de 22 h si aucun Z du jour
  (`commands::report::do_needs_z_report`).
- Sauvegarde automatique de la base avant chaque Z.

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
  Nederlands (nl), English (en).
- Sélection de la langue d'affichage par utilisateur.

## 3. Fonctionnalités exclues (hors scope cert)

Pour la build de certification (`--features cert`), les
fonctionnalités suivantes sont **physiquement absentes du binaire** :

- **Intégration webshop / agrégateurs de livraison** (Uber Eats,
  Deliveroo, etc.) - exclue par le système de features Cargo.
  Vérification : `nm moyun-pos | grep -c pos_hubrise` → **0**.
- **Vue de gestion webshop** (`WebshopOrdersView.vue`) - exclue du
  bundle via la variable d'environnement build Vite.

Voir `BASELINE_TEST_REPORT.md` pour les preuves binaires.

## 4. Intégration Peppol (B2B)

Voir `PEPPOL_AP_DECISION.md` pour le détail.

Implémentation présente :
- Sérialiseur UBL 2.1 BIS Billing 3.0 (`crates/pos-peppol/src/ubl.rs`).
- Trait `AccessPoint` (`crates/pos-peppol/src/access_point.rs`) avec
  implémentation `LoggingAccessPoint` (no-op pour développement) et
  stub `StorecoveAccessPoint` (intégration commerciale à finaliser).
- Commande Tauri `issue_b2b_invoice`
  (`src-tauri/src/commands/invoice.rs`).
- Table `peppol_deliveries` pour la traçabilité.

Le déploiement avec un Access Point en production est une étape
contractuelle indépendante de la certification SCE 2.0.

## 5. Versions de spécifications implémentées

| Spécification | Version | Date |
|---|---|---|
| Description détaillée POS | v2.2 | 2026-04-17 |
| Description détaillée FDM | v1.4 | 2026-04-17 |
| Addendum 1 (POS/FDM) | v2.1 ES | 2026-02-09 |
| Use cases Basic Set | v1.7 | 2025-09-16 |
| Use cases Reports | v1.3 | 2025-09-16 |
| Use cases Advanced Set | v1.5 | 2025-09-23 |
| Peppol BIS Billing | 3.0 | - |
| FPSFIN API Protocol FDM-FinCloud | 1.09 | 2025-10-17 |

## 6. Périphériques pris en charge

- **Terminal POS** : Penetek A3216 M82 ou Advantech USC-360J (Intel
  J6412, x86_64, 8 Go RAM, 128 Go SSD, écran 15,6" tactile PCAP
  1920×1080, IP65). Cf. `KIOSK_DEPLOYMENT_CHECKLIST.md`.
- **Imprimante thermique** : Epson TM-m30III (USB / LAN / WiFi /
  Bluetooth). Protocole ESC/POS.
- **Tiroir-caisse** : standard RJ11 connecté à l'imprimante.
- **FDM** : Checkbox CBX01 (TAIGER BV, ~€195 HTVA, achat unique
  via Eutronix réf. TAFIP0002.00). Connexion LAN (HTTP/GraphQL).
