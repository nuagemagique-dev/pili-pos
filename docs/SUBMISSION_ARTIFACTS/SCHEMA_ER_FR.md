# Diagramme entité-relation de la base de données

**Date :** 2026-05-14
**Référence :** §II.C "Relations entre les tables" du manuel de certification.
**Source de vérité :** `crates/pos-db/src/schema.rs` (le SQL définit le schéma exact ; ce document reflète la révision actuelle).

Ce document décrit visuellement les relations entre les tables de la
base SQLite `pos.db`. Pour l'inventaire détaillé champ-par-champ,
*voir : SCHEMA_BD_FR.md*. Pour la sécurité et les contraintes,
*voir : SECURITE_BD_FR.md*.

---

## 1. Vue d'ensemble par domaines

La base est organisée en 6 domaines logiques :

```
+-----------------+   +---------------+   +----------------+
|     CATALOG     |   |    ORDERS     |   |    PAYMENTS    |
| (menu, prix)    |   | (commandes en |   | (encaissements,|
|                 |   |  cours)       |   |  modes paiement)
+--------+--------+   +-------+-------+   +-------+--------+
         |                    |                   |
         |                    |                   |
         +---------+----------+-------------------+
                   |
                   |
+------------------v-------------------+
|        FISCAL (append-only)          |
|  (events FDM, compteurs, archive)    |
+-------------+------------------------+
              |
              |
+-------------v-----------+   +---------------------+
|        USERS            |   |   REPORTS (Z)       |
| (opérateurs INSZ)       |   | (archive rapports Z)|
+-------------------------+   +---------------------+


+-----------------------+
|   I18N (translations) |
|  (polymorphe vers     |
|   catalog)            |
+-----------------------+


+-----------------------+
|  PEPPOL (livraisons   |
|   B2B append-only)    |
+-----------------------+


+-----------------------------+
|  TICKET NUMÉRIQUE (§III.H)  |
|  ticket_deliveries (file    |
|  d'envoi e-mail, partiel)   |
|  original_prints            |
|  (append-only strict)       |
+-----------------------------+
```

---

## 2. Diagramme ER détaillé

```
+----------------------+         +----------------------+
|     languages        |         |     translations     |
|----------------------|         |----------------------|
| code         TEXT PK |<--------| lang         TEXT FK |
| name         TEXT    |    1..N | entity_type  TEXT    |
| active       INT     |         | entity_id    INT     |
+----------------------+         | field        TEXT    |
                                 | value        TEXT    |
                                 | (PK composite)        |
                                 +----------+-----------+
                                            |
                  polymorphe entity_type ∈  |
                  {category, menu_item,      |
                   modifier_group, modifier} |
                                            |
+----------------------+      1   N         |
|     categories       |<-------------------+
|----------------------|
| id           INT PK  |
| sort_order   INT     |
| color        TEXT    |
| active       INT     |
+----------+-----------+
           |
           | 1..N
           |
+----------v-----------+      1   N         +----------------------+
|     menu_items       |<-------------------|     translations     |
|----------------------|                    +----------------------+
| id           INT PK  |
| category_id  INT FK  |--+
| price_cents  INT     |  |
| vat_code     TEXT    |  |
| kitchen_     INT     |  |
|    printer           |  |  1..N
| sort_order   INT     |  |
| active       INT     |  |
+----------------------+  |
           ^              |
           |              v
           | 1..N    +----+------------------+
           |         |   modifier_groups     |
           |         |-----------------------|
           |         | id          INT PK    |
           |         | menu_item_id INT FK   |
           |         | min_select  INT       |
           |         | max_select  INT       |
           |         +-----------+-----------+
           |                     |
           |                     | 1..N
           |                     |
           |                +----v-----------+
           |                |   modifiers    |
           |                |----------------|
           |                | id    INT PK   |
           |                | group_id INT FK|
           |                | price_cents INT|
           |                | sort_order INT |
           |                +----------------+
           |
           |
+----------+-----------+
|     order_lines      |
|----------------------|
| id           INT PK  |
| order_id     INT FK  |--+
| menu_item_id INT FK  |  |
| quantity     INT     |  |
| unit_price_  INT     |  |
|     cents            |  |
| vat_code     TEXT    |  |
| name         TEXT    |  | dénormalisé pour archive
| modifiers_   TEXT    |  |
|     json             |  |
| created_at   TEXT    |  |
+----------------------+  |
                          |
                          | N..1
                          v
+----------------------+----+
|       orders             |
|--------------------------|
| id           INT PK      |<--------------+
| table_number INT         |               |
| order_type   TEXT        |               |  N..1
| status       TEXT        |               |
| user_insz    TEXT FK     |---+           |
| created_at   TEXT        |   |           |
| closed_at    TEXT        |   |           |
| subtotal_    INT         |   |       +---+--+
|     cents                |   |       | payments |
| total_cents  INT         |   |       |----------|
+--------------------------+   |       | id   INT PK
            ^                  |       | order_id INT FK
            |                  |       | method   TEXT
            | 0..1             |       | amount_cents INT
            |                  |       | created_at TEXT
            |                  |       +----------+
+-----------+----------+       |
|    fiscal_events     |       |
|----------------------|       |
| id           INT PK  |       |
| event_type   TEXT    |  N..1 |
| sequence_    INT UK  +-------+
|     number           |       |
| order_id     INT FK  +-------+
| pos_id       TEXT    |
| terminal_id  TEXT    |
| user_insz    TEXT FK +---+
| event_data_  TEXT    |   |
|     json             |   |
| fdm_response_TEXT    |   |
|     json             |   |
| fdm_id       TEXT    |   |
| fdm_         TEXT    |   |
|     datetime         |   |
| event_label  TEXT    |   |
| short_       TEXT    |   |
|     signature        |   |
| qr_data      TEXT    |   |
| created_at   TEXT    |   |
| archived     INT     |   |
+----------+-----------+   |
           ^               |
           |               |
           | 0..1          |
           |               |
+----------+----------+    |       +---------------------+
|      z_reports     |    |       |       users         |
|--------------------|    |       |---------------------|
| id        INT PK   |    +------>| insz   TEXT PK (11d)|
| report_date TEXT   |            | name   TEXT         |
| sequence_  INT UK  |            | role   TEXT         |
|    number          |            | pin_hash TEXT       |
| data_json  TEXT    |            | active   INT        |
| fiscal_evt INT FK  |            +---------------------+
|     _id            |
| created_at TEXT    |
+--------------------+


+--------------------+
|  fiscal_counters   |
|--------------------|
| event_label TEXT PK|
| current_value INT  |
+--------------------+
(table autonome, mise à jour atomique par BEGIN IMMEDIATE
 dans la même transaction qui ajoute à fiscal_events)


+--------------------------+
|   peppol_deliveries      |
|--------------------------|
| id              INT PK   |
| fiscal_event_id INT FK   |---+
| invoice_no      TEXT     |   |
| customer_       TEXT     |   |
|     endpoint_id          |   |
| ap_vendor       TEXT     |   | 1..1
| ap_message_id   TEXT     |   v
| ubl_xml         TEXT     |  fiscal_events
| ap_response_    TEXT     |  (event_type='I')
|     json                 |
| status          TEXT     |
| error_msg       TEXT     |
| created_at      TEXT     |
| accepted_at     TEXT     |
+--------------------------+


+--------------------------+
|   ticket_deliveries      |
|--------------------------|
| id              INT PK   |
| fiscal_event_id INT FK   |---+
| medium          TEXT     |   |
| recipient       TEXT     |   | 0..N (au plus 1 en pratique :
|     (nullable)           |   | une seule demande d'e-mail
| recipient_hint  TEXT     |   |  par ticket signé)
| ticket_text     TEXT     |   v
| verification_   TEXT     |  fiscal_events
|     url (null.)          |
| status          TEXT     |
| attempts        INT      |
| next_attempt_at TEXT     |
|     (nullable)           |
| provider_       TEXT     |
|     message_id           |
| last_error      TEXT     |
| fallback_       INT      |
|     printed              |
| created_at      TEXT     |
| updated_at      TEXT     |
+--------------------------+
(fiscal_event_id / ticket_text / medium immuables par trigger ;
 les autres colonnes suivent le cycle de vie pending → sending →
 sent | failed | fallback_printed | purged)


+--------------------------+
|   original_prints        |
|--------------------------|
| fiscal_event_id INT PK,FK|---+
| printed_at      TEXT     |   | 0..1
+--------------------------+   v
                              fiscal_events
(append-only strict : ni DELETE ni UPDATE, même sur printed_at)
```

---

## 3. Relations principales (verbalisées)

| De → Vers | Cardinalité | Sémantique |
|---|---|---|
| `categories` → `menu_items` | 1 → N | Une catégorie contient N articles de menu |
| `menu_items` → `modifier_groups` | 1 → N | Un article peut avoir N groupes de modificateurs (sauce, cuisson, ...) |
| `modifier_groups` → `modifiers` | 1 → N | Chaque groupe contient N options (Soja, Wasabi, ...) |
| `orders` → `order_lines` | 1 → N | Une commande contient N lignes |
| `order_lines` → `menu_items` | N → 1 | Référence dénormalisée vers l'article (nom + prix copiés) |
| `orders` → `payments` | 1 → N | Une commande peut être réglée par N paiements (split) |
| `orders` → `fiscal_events` | 1 → 0..N | Une commande peut générer 0 (open) à plusieurs events fiscaux (P → N, ou N + C copie, etc.) |
| `fiscal_events` → `orders` | N → 0..1 | Un event fiscal référence 0 (events système comme S, F-WAR) ou 1 commande |
| `users` → `orders` | 1 → N | Un opérateur ouvre N commandes |
| `users` → `fiscal_events` | 1 → N | Un opérateur émet N events (via `user_insz`) |
| `z_reports` → `fiscal_events` | 1 → 1 | Chaque Z archive un event R correspondant |
| `peppol_deliveries` → `fiscal_events` | 1 → 1 | Chaque livraison Peppol est liée à un event I |
| `ticket_deliveries` → `fiscal_events` | 0..N → 1 | Une ligne par ticket dont le support demandé inclut l'e-mail (`DIGITAL`/`PAPER_DIGITAL`) ; ajoutée dans la même transaction que l'event N/T/C correspondant |
| `original_prints` → `fiscal_events` | 0..1 → 1 | Au plus une ligne par événement dont l'original a été imprimé (contrainte de clé primaire = garde applicative contre une seconde impression) |
| `translations` → (polymorphe) | N → 1 | Traduction d'un champ d'une entité (category / menu_item / modifier_group / modifier) dans une langue (FR / NL / EN / ZH) |
| `translations` → `languages` | N → 1 | Référence le code langue |
| `fiscal_counters` (autonome) | - | Pas de FK ; clé `event_label` ∈ {N, P, T, C, I, F, S, R} |

---

## 4. Append-only et intégrité

Ces tables sont **append-only** au niveau applicatif (jamais de DELETE, et pas d'UPDATE hors exceptions ciblées) :

- `fiscal_events` - archive des events fiscaux signés par le FDM
- `z_reports` - archive des rapports de clôture
- `peppol_deliveries` - archive des livraisons B2B Peppol
- `original_prints` - un original n'est jamais réimprimé silencieusement (§8.1) ; ni DELETE ni UPDATE, aucune exception
- `ticket_deliveries` - append-only **partiel** : `fiscal_event_id`, `ticket_text` et `medium` sont immuables par trigger, mais les autres colonnes (`status`, `attempts`, `recipient`...) évoluent volontairement pour suivre le cycle de vie de l'envoi (file d'attente, pas une archive figée)

Pour les autres tables (catalogue, commandes en cours, paiements, utilisateurs) les opérations UPDATE/DELETE sont autorisées via la couche commandes (avec audit trail dans les logs systemd-journald). Les `orders` sont marquées `status='closed'` ou `status='voided'` plutôt que supprimées.

La contrainte `fiscal_events.sequence_number UNIQUE` empêche tout doublon de compteur fiscal au niveau base. La table `fiscal_counters` est mise à jour atomiquement dans la même transaction SQLite `BEGIN IMMEDIATE` que l'insert dans `fiscal_events` (cf. *STRATEGIE_COMPTEURS_FR.md* §5 - fix B3).

---

## 5. Indexes

| Table | Index | Colonnes | Raison |
|---|---|---|---|
| `translations` | `idx_translations_lookup` | `entity_type, entity_id, lang` | Lookup rapide pour le rendu UI multilingue |
| `fiscal_events` | (implicite via PK + UNIQUE) | `id`, `sequence_number` | Lookup par ID et garantie d'unicité du compteur |
| `orders` | (implicite via PK) | `id` | Lookup par ID |
| `users` | (implicite via PK) | `insz` | Auth login |
| `ticket_deliveries` | `idx_ticket_deliveries_due` | `status, next_attempt_at` | Sélection rapide des envois dus par le worker de réessai (§III.H) |

Des indexes additionnels peuvent être ajoutés en post-cert si des problèmes de performance apparaissent à plus grande échelle (multi-restaurant). Pour le périmètre mono-terminal (~150 tickets/jour pour un restaurant type), aucun index supplémentaire n'est nécessaire.

---

## 6. Tables hors périmètre cert

Certaines tables existent dans le schéma mais sont **inutilisées dans la build cert** (intégrations tierces exclues par feature Cargo) :

- `hubrise_outbox` - queue d'événements webshop. Le schéma est préservé pour rétro-compatibilité de lecture, mais aucun chemin de code vivant ne l'écrit dans la build cert.

Ces tables apparaissent dans le SQL de seed mais ne reçoivent aucune écriture en production cert. *Voir : RAPPORT_TESTS_BASELINE_FR.md* §"Vérification de build" pour la preuve par `nm moyun-pos | grep -c pos_hubrise == 0`.

---

## 7. Versionnement du schéma

Le schéma est versionné via `pos-db::migrate::run_migrations`. La table `_migrations_applied` enregistre chaque migration appliquée (`version`, `name`, `applied_at`). Aucune modification de schéma post-cert n'est possible sans une nouvelle demande de certification (Article 109 du manuel).

Version actuelle du schéma : `v6` (`initial-schema`,
`catalogue-fiscal-fields`, `booking-periods`,
`fiscal-archive-append-only`, `ticket-deliveries`,
`original-prints`) - liste exacte dans `SCHEMA_BD_FR.md` §6 et
`crates/pos-db/src/migrate.rs::MIGRATIONS`.
