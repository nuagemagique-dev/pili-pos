# Diagramme entité-relation de la base de données

**Date :** 2026-10-03
**Référence :** §II.C "Relations entre les tables" du manuel de certification.
**Source de vérité :** `crates/pos-db/src/schema.rs`, `schema_hubrise.rs`, `schema_catalog.rs`, `schema_eft.rs`, `schema_till.rs`, `schema_invoicing.rs` et `schema_badges.rs`, assemblés par `crates/pos-db/src/migrate.rs` (le SQL définit le schéma exact ; ce document reflète la révision actuelle : migrations 1 à 11, 20 à 28, 30 et 31, 40 à 42, puis 70).

Ce document décrit visuellement les relations entre les tables de la
base SQLite `pos.db`. Pour l'inventaire détaillé champ-par-champ,
*voir : SCHEMA_BD_FR.md*. Pour la sécurité et les contraintes,
*voir : SECURITE_BD_FR.md*.

Une note sur les clés : les colonnes `user_insz` (dans `orders`,
`fiscal_events`, `pending_signs`, `staff_shifts`) contiennent l'INSZ de
l'opérateur mais ne portent **aucune clé étrangère** vers `users` : un
événement fiscal garde son INSZ même si l'utilisateur est désactivé ou
retiré. Le lien est applicatif.

---

## 1. Vue d'ensemble par domaines

La base est organisée en 19 domaines logiques :

```
+-----------------+   +---------------+   +----------------+
|     CATALOG     |   |    ORDERS     |   |    PAYMENTS    |
| categories,     |   | orders,       |   | payments       |
| menu_items,     |   | order_lines   |   | (cash, card,   |
| modifier_groups,|   | (commandes)   |   |  meal_voucher, |
| modifiers,      |   |               |   |  other)        |
| catalog_sync_   |   |               |   |                |
| state           |   |               |   |                |
+--------+--------+   +-------+-------+   +-------+--------+
         |                    |                   |
         +---------+----------+-------------------+
                   |
+------------------v-------------------+
|        FISCAL (append-only)          |
| fiscal_events, fiscal_counters,      |
| order_entries (journal),             |
| booking_periods, pending_signs       |
+-------------+------------------------+
              |
+-------------v-----------+   +---------------------+
|  USERS & PRESTATIONS    |   |   REPORTS (Z)       |
| users (opérateurs INSZ),|   | z_reports           |
| staff_shifts            |   | (archive rapports Z)|
+-------------------------+   +---------------------+

+-----------------------+   +-----------------------------+
|   I18N                |   |  TICKET NUMÉRIQUE (§III.H)  |
|  languages,           |   |  ticket_deliveries (file    |
|  translations         |   |  d'envoi e-mail, partiel)   |
|  (polymorphe vers     |   |  original_prints            |
|   catalog)            |   |  (append-only strict)       |
+-----------------------+   +-----------------------------+

+-------------------------------+   +--------------------------+
|  COMMANDES EN LIGNE (HubRise, |   |  FACTURATION (migration 40)|
|  migrations 20 à 25)          |   |  b2b_customers, invoices,|
|  webshop_orders,              |   |  invoice_sales,          |
|  hubrise_channels,            |   |  peppol_deliveries,      |
|  hubrise_cash_payment_refs,   |   |  peppol_delivery_log     |
|  hubrise_connection/events/   |   +--------------------------+
|  outbox, order_price_changes, |
|  integration_switches/_log    |
+-------------------------------+

+-------------------------------+   +--------------------------------+
|  PRODUIT ÉPUISÉ (migration 41)|   |  ÉDITEUR DE CARTE (migration 42)|
|  menu_item_sold_out,          |   |  menu_change_log (append-only) |
|  sold_out_log (append-only),  |   +--------------------------------+
|  hubrise_inventory_sync       |
+-------------------------------+

+-------------------------------+   +--------------------------------+
|  PLAN DE SALLE (migration 11) |   |  TERMINAL DE PAIEMENT RELIÉ    |
|  table_plan (plan en vigueur),|   |  (migration 27)                |
|  table_plan_log (append-only) |   |  eft_payments (append-only)    |
+-------------------------------+   +--------------------------------+

+----------------------------------------------------+
|  BADGES DU PERSONNEL (migration 70, option)        |
|  badge_settings (réglage unique), staff_badges     |
|  (liens, retrait seul), badge_log et badge_scans   |
|  (ajout seul)                                      |
+----------------------------------------------------+

+-------------------------------+   +--------------------------------+
|  RÉGLAGES DU PROPRIÉTAIRE     |   |  ALLERGÈNES (migration 61)     |
|  (migration 60)               |   |  menu_item_allergens,          |
|  owner_settings,              |   |  modifier_allergens,           |
|  owner_settings_log et        |   |  allergen_declarations,        |
|  owner_alert_log (ajout seul) |   |  allergen_change_log (ajout    |
+-------------------------------+   |  seul)                         |
                                    +--------------------------------+

+-------------------------------+   +--------------------------------+
|  SERVICE EN PLUSIEURS TEMPS   |   |  BONS CADEAUX (migration 63)   |
|  (migration 62, cuisine)      |   |  gift_vouchers,                |
|  order_line_courses,          |   |  gift_voucher_movements,       |
|  order_course_releases,       |   |  gift_voucher_prints (ajout    |
|  kitchen_slip_entries         |   |  seul)                         |
+-------------------------------+   +--------------------------------+
```

`_migrations_applied` (suivi des migrations) est hors domaine.

---

## 2. Diagrammes ER détaillés

### 2.1 Catalogue et traductions

```
+----------------------+         +----------------------+
|     languages        |         |     translations     |
|----------------------|         |----------------------|
| code         TEXT PK |<--------| lang         TEXT FK |
| name         TEXT    |    1..N | id           INT PK  |
| active       INT     |         | entity_type  TEXT    |
+----------------------+         | entity_id    INT     |
 fr, nl, en, zh (mig. 1)         | field        TEXT    |
 de (migration 7)                | value        TEXT    |
                                 | UNIQUE(entity_type,   |
                                 |  entity_id, field,    |
                                 |  lang)                |
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
| external_id  TEXT UK |
| sort_order   INT     |
| color        TEXT    |
| active       INT     |
| hidden       INT (21)|
+----------+-----------+
           |
           | 1..N
           |
+----------v-----------+
|     menu_items       |
|----------------------|
| id           INT PK  |
| external_id  TEXT UK |
| category_id  INT FK  |
| price_cents  INT     |
| vat_code     TEXT    |
| vat_category TEXT    |
| vat_code_takeaway    |
|    TEXT (mig. 2)     |
| open_price   INT (2) |
| kind         TEXT(21)|
| on_pos       INT (21)|
| kitchen_printer INT  |
| sort_order   INT     |
| active       INT     |
+----------+-----------+
           |
           | 1..N
           |
+----------v-----------+
|   modifier_groups    |
|----------------------|
| id           INT PK  |
| external_id  TEXT UK |
| menu_item_id INT FK  |
| min_select   INT     |
| max_select   INT     |
| active       INT (21)|
| sort_order   INT (21)|
+----------+-----------+
           |
           | 1..N
           |
+----------v-----------+      +--------------------------+
|      modifiers       |      |   catalog_sync_state     |
|----------------------|      |--------------------------|
| id           INT PK  |      | source        TEXT PK    |
| external_id  TEXT UK |      | manages_menu  INT        |
| group_id     INT FK  |      | hash          TEXT       |
| price_cents  INT     |      | applied_at    TEXT       |
| sort_order   INT     |      | rejected_at   TEXT       |
| active       INT (21)|      | issues_json   TEXT       |
+----------------------+      | catalog_json  TEXT       |
                              +--------------------------+
                              (table autonome, migration 21)
```

### 2.2 Commandes, lignes, paiements, journal

```
+----------------------+
|       orders         |
|----------------------|
| id           INT PK  |<---------------------------+
| table_number INT     |                            |
| order_type   TEXT    |   (dine_in | takeaway)     |
| status       TEXT    |   (open | closed | voided)|
| user_insz    TEXT    |   (pas de FK)              |
| created_at   TEXT    |                            |
| closed_at    TEXT    |                            |
| subtotal_cents INT   |                            |
| total_cents  INT     |                            |
| hubrise_order_id TEXT|  (UNIQUE partiel)          |
| is_fiscal    INT     |                            |
| source       TEXT    |                            |
| hubrise_status TEXT  |                            |
| service_type TEXT    |                            |
| raw_hubrise_json TEXT|                            |
| pre_bill_at  TEXT (9)|                            |
+---+---------+--------+                            |
    | 1..N    | 1..N                                | N..1
    |         |                                     |
+---v-------+ +--v-------------------+     +--------+--------+
|order_lines| |    order_entries     |     |    payments     |
|-----------| |----------------------|     |-----------------|
| id   PK   | | id           INT PK  |     | id        INT PK|
| order_id  | | order_id     INT FK  |     | order_id  INT FK|
|   FK      | | line_id      INT     |     | method    TEXT  |
| menu_item_| | menu_item_id INT     |     |  cash | card |  |
|  id   FK  | | quantity     INT <>0 |     |  meal_voucher | |
| quantity  | | unit_price_  INT     |     |  other (mig. 8) |
| unit_price| |    cents             |     | amount_cents INT|
|  _cents   | | vat_code     TEXT    |     | created_at TEXT |
| vat_code  | | name         TEXT    |     +-----------------+
| name      | | department_id TEXT   |
| modifiers_| | department_name TEXT |
|  json     | | kind         TEXT    |
| department| |  entry|correction|   |
|  _id,_name| |  refund              |
| created_at| | sent_event_id INT FK |---> fiscal_events (nullable)
+-----------+ | created_at   TEXT    |
 vue           +----------------------+
 consolidée    journal append-only
 (dénormalisée) (triggers, voir §4)
```

### 2.3 Fiscal, périodes, signatures en attente, rapports Z

```
+----------------------+
|    booking_periods   |
|----------------------|
| id           TEXT PK |  (UUID)
| booking_date TEXT    |
| opened_at    TEXT    |
| closed_at    TEXT    |
| closing_event_id INT |---+
| FK (nullable)        |   | CHECK((closed_at IS NULL) =
+----------------------+   |       (closing_event_id IS NULL))
                           v
+----------------------+  +----------------------+
|    fiscal_events     |<-+                      |
|----------------------|                         |
| id           INT PK  |                         |
| event_type   TEXT    |  CHECK N,P,T,C,I,F,S,R  |
| sequence_number INT  |  UNIQUE(event_type,     |
|                      |         sequence_number)|
| order_id     INT FK  |---> orders (nullable)   |
| pos_id       TEXT    |                         |
| terminal_id  TEXT    |                         |
| user_insz    TEXT    |  (pas de FK)            |
| event_data_json TEXT |                         |
| fdm_response_json    |                         |
| fdm_id       TEXT    |                         |
| fdm_datetime TEXT    |                         |
| event_label  TEXT    |                         |
| short_signature TEXT |                         |
| qr_data      TEXT    |                         |
| created_at   TEXT    |                         |
| archived     INT     |                         |
+----+-----------+-----+                         |
     ^           ^                               |
     | 0..1      | 0..1                          |
+----+--------+ +-+----------------------+       |
|  z_reports  | |     pending_signs      |       |
|-------------| |------------------------|       |
| id     PK   | | id            INT PK   |       |
| report_date | | event_label   TEXT     |       |
| sequence_   | | first_ticket_no INT    |       |
|  number INT | | ticket_count  INT 1|2  |       |
|  (non       | | mutation      TEXT     |       |
|  unique)    | | is_training   INT      |       |
| data_json   | | request_body  TEXT     |       |
| fiscal_     | | archive_json  TEXT     |       |
|  event_id FK| | kind, subject TEXT     |       |
| created_at  | | order_id      INT      |       |
+-------------+ | context_json  TEXT     |       |
                | user_insz     TEXT     |       |
                | status  open|signed|   |       |
                |   refused|not_sent     |       |
                | attempts, last_error   |       |
                | fiscal_event_id INT FK |-------+
                | created_at, resolved_at|
                +------------------------+

+--------------------+
|  fiscal_counters   |
|--------------------|
| event_label TEXT PK|
| current_value INT  |
+--------------------+
(table autonome, mise à jour atomique dans la même transaction
 SQLite qui ajoute à fiscal_events, sous le verrou de signature)
```

### 2.4 Utilisateurs et prestations

```
+---------------------+       +---------------------+
|       users         |       |    staff_shifts     |
|---------------------|       |---------------------|
| insz   TEXT PK (11d)|       | id         INT PK   |
| name   TEXT         |  - - -| user_insz  TEXT     |
| role   TEXT         | lien  | started_at TEXT     |
| pin_hash TEXT       | appli-| ended_at   TEXT     |
| active   INT        | catif | started_by TEXT     |
| presence_only INT   |       | ended_by   TEXT     |
+---------------------+       +---------------------+
 role : admin|cashier|waiter   (migration 9 ; index local des postes
 presence_only : 0|1           ouverts par un S WorkIn, clos par un
 (migration 28)                S WorkOut ; sans FK ; started_by et
                               ended_by : migration 28, INSZ de qui a
                               enregistré, NULL avant)
```

### 2.5 Ticket numérique et impressions d'original

```
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

### 2.6 Commandes en ligne (HubRise, migrations 20 à 25)

```
+---------------------------+      +---------------------------+
|     hubrise_channels      |      | hubrise_cash_payment_refs |
|---------------------------|      |---------------------------|
| channel        TEXT PK    |      | ref            TEXT PK    |
| kind           TEXT       |      +---------------------------+
| payment_type   TEXT       |
| charge_policy  TEXT       |
+---------------------------+
 (tables de paramétrage, sans FK ; liées à webshop_orders par la valeur
  de `channel` / de la référence de paiement)

+---------------------------+  1..1  +-----------------------+
|      webshop_orders       |------->|        orders         |
|---------------------------| order_id (PK = FK)             |
| order_id        INT PK,FK |        +-----------------------+
| hubrise_order_id TEXT UK  |
| channel, channel_kind     |
| service_type              |
| payment_mode, payment_type|
| fiscal_state              |  p_event_id, n_event_id,
| resolution                |  refund_event_id  FK (nullable)
| attention_json,           |-----------------------> fiscal_events
|  warnings_json, lines_json|
| p_event_id, n_event_id,   |
|  refund_event_id          |
| accepted_by, accepted_at  |
| resolved_at               |
+---------------------------+

+---------------------------+  N..1  +-----------------------+
|   order_price_changes     |------->|        orders         |
|---------------------------| order_id FK                    |
| id              INT PK    |        +-----------------------+
| order_id        INT FK    |  N..1  +-----------------------+
| entry_id        INT FK    |------->|     order_entries     |
| line_id         INT       |        +-----------------------+
| grouping_id     INT >0    |
| change_id, name  TEXT     |
| scope  LINE|EVENT         |
| change_type PUBLIC|INTERNAL
| amount_cents    INT       |
| created_at      TEXT      |
+---------------------------+

+---------------------------+  +---------------------------+
|    hubrise_connection     |  |      hubrise_events       |
|---------------------------|  |---------------------------|
| id INT PK (= 1, singleton)|  | id            TEXT PK     |
| access_token, account_*,  |  | resource_type, event_type |
| location_*, catalog_*,    |  | payload_json              |
| customer_list_*, scope,   |  | received_at, processed_at |
| base_url, connected_at,   |  | error_msg, attempts,      |
| last_poll_at,             |  | parked_at                 |
| last_event_cursor,        |  +---------------------------+
| last_catalog_sync_at,     |
| auth_error_at, last_error,|  +---------------------------+
| callback_registered_at    |  |      hubrise_outbox       |
+---------------------------+  |---------------------------|
                               | id INT PK, order_id INT   |
                               | kind create_order|        |
                               |   patch_order             |
                               | body_json, hubrise_order_id|
                               | private_ref (UNIQUE part.)|
                               | created_at, attempts,     |
                               | last_attempt_at, sent_at, |
                               | next_attempt_at, parked_at|
                               | error_msg                 |
                               +---------------------------+

+---------------------------+  +---------------------------+
|   integration_switches    |  |   integration_switch_log  |
|---------------------------|  |---------------------------|
| name TEXT PK ('hubrise')  |  | id INT PK                 |
| enabled, config_value INT |  | name TEXT, enabled INT    |
| changed_at, changed_by    |  | source config|admin       |
+---------------------------+  | changed_by, changed_at    |
                               +---------------------------+
                               (append-only par triggers)
```

### 2.7 Facturation et Peppol (migration 40)

```
+------------------+   1..n   +---------------+   1..n   +-----------------+
|  b2b_customers   |<---------|   invoices    |---------->|  invoice_sales  |
|------------------|          |---------------|           |-----------------|
| id INT PK        |          | id INT PK     |           | invoice_id FK   |
| vat_number UNIQ  |          | invoice_seq   |           | sale_fiscal_    |
| name, street,    |          | invoice_no    |           |   event_id UNIQ |
| postal_code,     |          | customer_id FK|           | position        |
| city, country    |          | customer_json |           +--------+--------+
| peppol_status    |          | issue_date    |                    |
| peppol_checked_at|          | total_cents   |                    | n..1
+------------------+          | vat_json      |                    v
                              | fiscal_event_ |              fiscal_events
                              |   id FK UNIQ  |--------------> (event_type='I'
                              +-------+-------+                 et ventes 'N')
                                      | 1..n
                                      v
+--------------------------+   +--------------------------+
|   peppol_deliveries      |   |   peppol_delivery_log    |
|--------------------------|   |--------------------------|
| id              INT PK   |<--| delivery_id FK           |
| fiscal_event_id INT FK   |   | attempt, outcome         |
| invoice_no      TEXT     |   | http_status, detail, at  |
| customer_endpoint_id     |   +--------------------------+
| ap_vendor, ap_message_id |
| ubl_xml, ap_response_json|   Vue v_invoice_journal :
| status (queued|accepted| |   invoices + fiscal_events
|         failed)          |   + dernier envoi Peppol
| attempts, next_attempt_at|
| last_attempt_at,         |
| parked_at, lookup_status |
| error_msg, created_at,   |
| accepted_at              |
+--------------------------+
(invoices, invoice_sales et peppol_delivery_log sont en ajout seul,
 par déclencheurs)
```

### 2.7 bis Produit épuisé et éditeur de carte (migrations 41 et 42)

```
+---------------------------+   +---------------------------+
|     menu_item_sold_out    |   |       sold_out_log        |
|---------------------------|   |---------------------------|
| menu_item_id INT PK, FK   |   | id INT PK                 |
| until TEXT (NULL = jusqu'à|   | menu_item_id FK           |
|   réactivation)           |   | action set|clear|expired  |
| set_by, set_at            |   | until, by, at             |
+---------------------------+   +---------------------------+
                                (ajout seul, par déclencheurs)

+---------------------------+   +---------------------------+
|  hubrise_inventory_sync   |   |    menu_change_log        |
|---------------------------|   |---------------------------|
| menu_item_id INT PK, FK   |   | id INT PK                 |
| sku_ref, dirty, version   |   | entity_type, entity_id    |
| attempts, next_attempt_at |   | action create|update      |
| last_attempt_at, parked_at|   | before_json, after_json   |
| error_msg, synced_at      |   | changed_by, changed_at    |
+---------------------------+   +---------------------------+
                                (ajout seul, par déclencheurs)
```

### 2.8 Plan de salle (migration 11)

```
+---------------------------+  +---------------------------+
|        table_plan         |  |      table_plan_log       |
|---------------------------|  |---------------------------|
| id INT PK (toujours 1)    |  | id INT PK                 |
| plan TEXT ("1-12, 20-25") |  | plan TEXT                 |
| config_plan TEXT          |  | source config|admin       |
| changed_at TEXT           |  | changed_by TEXT           |
| changed_by TEXT (INSZ ou  |  | changed_at TEXT           |
|   NULL pour pos.json)     |  +---------------------------+
+---------------------------+  (append-only par triggers)
```

Aucune clé étrangère : le plan ne porte que des numéros de table, et
`changed_by` est l'INSZ du Gérant (lien applicatif). Les commandes
gardent leur numéro de table (`orders.table_number`) ; le plan ne
détermine que les tables que l'écran des tables affiche, plus toute table
hors plan qui a une commande ouverte.

### 2.9 Terminal de paiement relié (migration 27)

```
+----------------------------------+
|           eft_payments           |
|----------------------------------|
| id INT PK                        |
| reference TEXT UNIQUE (UUID)     |
| order_id INT FK ---------------------> orders
| terminal_kind TEXT               |
| amount_cents INT > 0             |
| status sent|approved|declined|   |
|   cancelled|unknown|not_sent     |
| card_type debit|credit|unknown   |
| brand, terminal_reference,       |
| authorisation_code, masked_pan,  |
| customer_receipt, detail TEXT    |
| user_insz TEXT (pas de FK)       |
| created_at, resolved_at TEXT     |
| used_event_id INT FK ----------------> fiscal_events (nullable)
+----------------------------------+
 ajout seul par triggers (voir §4)
```

Une ligne par paiement demandé au terminal, écrite avant l'appel. Elle
référence sa commande et, une fois la vente signée, l'événement N qui l'a
réglée (`used_event_id`, une seule fois). Un paiement `approved` impose
un `card_type` (`CHECK`).

### 2.10 Badges du personnel (migration 70)

```
+----------------------------------+
|          badge_settings          |   une seule ligne (id = 1)
|----------------------------------|
| reader off|nfc|usb               |
| login_enabled 0|1                |
| wedge_max_gap_ms, wedge_min_len, |
| wedge_max_len                    |
| key_check, changed_at/by         |
+----------------------------------+

+----------------------------------+        +-------------------------------+
|           staff_badges           |        |           badge_log           |
|----------------------------------|        |-------------------------------|
| id INT PK                        |<-------| badge_id INT FK (nullable)    |
| badge_hash TEXT(64) (HMAC)       |        | action link|revoke|settings   |
| user_insz TEXT FK ----------------------> users.insz                      |
| linked_at, linked_by TEXT        |        | user_insz, detail, changed_by |
| revoked_at, revoked_by TEXT      |        | changed_at                    |
| revoke_reason manager|           |        +-------------------------------+
|   deactivated|key_changed        |
+----------------------------------+        +-------------------------------+
 index unique partiel : une empreinte        |          badge_scans          |
 active ; un badge actif par employé         |-------------------------------|
                                             | outcome clock_in|clock_out|   |
                                             |  presence_failed|login|...   |
                                             | user_insz, badge_ref(12), ... |
                                             +-------------------------------+
```

`staff_badges.user_insz` est, contrairement aux colonnes `user_insz` des
tables fiscales, une clé étrangère vers `users` : un badge ne désigne
qu'un employé connu. Les journaux `badge_log` et `badge_scans` gardent
l'INSZ sans clé étrangère. Le numéro du badge n'apparaît dans aucune
colonne (`SECURITE_BD_FR.md` §2.5).

### 2.11 Fonctions du propriétaire (migrations 60 à 63)

```
+---------------------------+   +---------------------------+
|      owner_settings       |   |    owner_settings_log     |
|---------------------------|   |---------------------------|
| key TEXT PK               |   | id INT PK                 |
| value_json                |   | key, value_json           |
| changed_at, changed_by    |   | changed_by, changed_at    |
+---------------------------+   +---------------------------+
                                (ajout seul, par déclencheurs)
+---------------------------+
|      owner_alert_log      |   (ajout seul, par déclencheurs)
|---------------------------|
| id INT PK                 |
| kind, transition, detail  |
| outcome, http_status      |
| created_at                |
+---------------------------+

+---------------------------+   +---------------------------+
|   menu_item_allergens     |   |    modifier_allergens     |
|---------------------------|   |---------------------------|
| menu_item_id FK  } PK     |   | modifier_id FK   } PK     |
| allergen (14)    }        |   | allergen (14)    }        |
| level contains|may_contain|   | level contains|may_contain|
+---------------------------+   +---------------------------+
+---------------------------+   +---------------------------+
|  allergen_declarations    |   |   allergen_change_log     |
|---------------------------|   |---------------------------|
| entity_type } PK          |   | id INT PK                 |
| entity_id   }             |   | entity_type, entity_id    |
| source catalog|local      |   | before_json, after_json   |
| declared_at, declared_by  |   | source, changed_by/at     |
+---------------------------+   +---------------------------+
                                (ajout seul, par déclencheurs)

+---------------------------+   +---------------------------+
|    order_line_courses     |   |   order_course_releases   |
|---------------------------|   |---------------------------|
| line_id INT PK (sans FK)  |   | order_id FK } PK          |
| order_id FK -> orders     |   | course 1-3  }             |
| course 0-3, set_by/at     |   | released_by, released_at  |
+---------------------------+   +---------------------------+
+---------------------------+
|   kitchen_slip_entries    |
|---------------------------|
| entry_id PK, FK ----------------> order_entries.id
| course 0-3, printed_at    |
| slip immediate|release|   |
|      sale                 |
+---------------------------+

+---------------------------+        +---------------------------+
|       gift_vouchers       |        |  gift_voucher_movements   |
|---------------------------|        |---------------------------|
| id INT PK                 |<-------| voucher_id FK             |
| code TEXT UNIQUE (12)     |        | kind issue|redeem|cancel  |
| face_value_cents > 0      |        | amount_cents (signé)      |
| issued_on, expires_on     |        | fiscal_event_id FK -------+--> fiscal_events.id
| training 0|1              |        | order_id, created_at      |
| issue_event_id FK --------+--> fiscal_events.id               |
| issue_order_id,           |        +---------------------------+
| issue_line_id UNIQUE      |        +---------------------------+
+---------------------------+<-------| gift_voucher_prints       |
                                     | voucher_id FK             |
                                     | kind original|duplicate|  |
                                     |      balance              |
                                     | printed_by, printed_at    |
                                     +---------------------------+
 (les trois tables des bons : ajout seul, par déclencheurs)
```

Les tables du service en plusieurs temps sont des données de cuisine :
aucune n'est lue pour construire une requête de signature. Le produit
système qui vend un bon (`menu_items` 990001, `external_id`
`pili:gift-voucher`, taux X) et sa catégorie (990000, `pili:system`) sont
inactifs et cachés de l'éditeur de carte (`SCHEMA_BD_FR.md` §2.9.5).

---

## 3. Relations principales (verbalisées)

| De → Vers | Cardinalité | Sémantique |
|---|---|---|
| `categories` → `menu_items` | 1 → N | Une catégorie contient N articles de menu |
| `menu_items` → `modifier_groups` | 1 → N | Un article peut avoir N groupes de modificateurs (sauce, cuisson, ...) |
| `modifier_groups` → `modifiers` | 1 → N | Chaque groupe contient N options (Soja, Wasabi, ...) |
| `orders` → `order_lines` | 1 → N | Une commande contient N lignes |
| `order_lines` → `menu_items` | N → 1 | Référence vers l'article (nom + prix copiés, figés) |
| `order_lines` (note de cuisine) | - | Colonnes locales `kitchen_note` et `kitchen_note_printed` (migration 31), sans clé : jamais copiées dans `order_entries`, jamais envoyées au FDM |
| `orders` → `order_entries` | 1 → N | Le journal chronologique de la commande (ajouts, corrections, remboursements) |
| `order_entries` → `fiscal_events` | N → 0..1 | `sent_event_id` : l'événement P ou N qui a transmis l'entrée |
| `orders` → `payments` | 1 → N | Une commande peut être réglée par N paiements (split, titres-repas) |
| `orders` → `eft_payments` | 1 → 0..N | Les paiements demandés au terminal relié pour cette commande (aucun sans terminal relié) |
| `eft_payments` → `fiscal_events` | N → 0..1 | `used_event_id` : l'événement N qui a réglé le paiement accepté |
| `orders` → `fiscal_events` | 1 → 0..N | Une commande peut générer 0 (open) à plusieurs events fiscaux (P → N, ou N + C copie, etc.) |
| `fiscal_events` → `orders` | N → 0..1 | Un event fiscal référence 0 (events système comme S, F-WAR) ou 1 commande |
| `users` → `orders` / `fiscal_events` / `pending_signs` / `staff_shifts` | 1 → N | Par la valeur de `user_insz` ; **aucune clé étrangère** (lien applicatif) |
| `booking_periods` → `fiscal_events` | 0..1 → 1 | `closing_event_id` : l'événement R qui clôt la période ; un événement ne peut être archivé dans une période close |
| `pending_signs` → `fiscal_events` | 0..1 → 1 | `fiscal_event_id` : l'événement archivé (premier événement si `ticket_count = 2`) |
| `z_reports` → `fiscal_events` | 0..1 → 1 | Chaque Z archive un event R correspondant. `sequence_number` n'est pas unique (la contrainte d'unicité est portée par `fiscal_events`) |
| `peppol_deliveries` → `fiscal_events` | 0..1 → 1 | Liée à un event I ; une ligne par facture envoyée sur Peppol |
| `invoices` → `fiscal_events` | 1 → 1 | L'événement I de la facture (unique) |
| `invoice_sales` → `invoices`, `fiscal_events` | N → 1 et 1 → 1 | Les ventes N couvertes par la facture ; une vente figure sur une seule facture (unique) |
| `invoices` → `b2b_customers` | N → 0..1 | Le client ; la facture garde son instantané |
| `peppol_delivery_log` → `peppol_deliveries` | N → 1 | Une ligne par tentative d'envoi |
| `menu_item_sold_out`, `sold_out_log`, `hubrise_inventory_sync` → `menu_items` | 0..1 → 1, N → 1, 0..1 → 1 | Produit épuisé et sa transmission au stock HubRise |
| `menu_item_allergens` → `menu_items`, `modifier_allergens` → `modifiers` | N → 1 | Allergènes déclarés d'un produit ou d'une option (migration 61) ; `allergen_declarations` et `allergen_change_log` désignent l'élément par (`entity_type`, `entity_id`), sans clé étrangère |
| `order_line_courses`, `order_course_releases` → `orders` | N → 1 | Temps de cuisine des lignes et temps libérés (migration 62) ; `line_id` sans clé étrangère |
| `kitchen_slip_entries` → `order_entries` | 0..1 → 1 | Ligne du journal imprimée sur un ticket de cuisine ou de bar |
| `gift_vouchers` → `fiscal_events` | N → 1 | `issue_event_id` : l'événement N ou T de la vente du bon (migration 63) |
| `gift_voucher_movements` → `gift_vouchers`, `fiscal_events` | N → 1 | Émission, utilisation ou annulation d'un bon, avec l'événement signé qui l'a causée |
| `gift_voucher_prints` → `gift_vouchers` | N → 1 | Original, duplicata ou solde imprimé |
| `ticket_deliveries` → `fiscal_events` | 0..N → 1 | Une ligne par ticket dont le support demandé inclut l'e-mail (`DIGITAL`/`PAPER_DIGITAL`) ; ajoutée dans la même transaction que l'event N/T/C correspondant |
| `original_prints` → `fiscal_events` | 0..1 → 1 | Au plus une ligne par événement dont l'original a été imprimé (contrainte de clé primaire = garde applicative contre une seconde impression) |
| `webshop_orders` → `orders` | 1 → 1 | Une ligne par commande en ligne, à côté de sa ligne `orders` |
| `webshop_orders` → `fiscal_events` | N → 0..1 | `p_event_id`, `n_event_id`, `refund_event_id` |
| `order_price_changes` → `orders`, `order_entries` | N → 1 | Remises d'une commande en ligne, par ligne du journal ; `line_id` désigne la ligne d'`order_lines` |
| `till_discounts` → `orders`, `order_entries` | N → 1 | Pose ou retrait d'une remise de caisse (motif et Gérant, jamais envoyés au FDM) ; `line_id` NULL pour une remise de commande |
| `translations` → (polymorphe) | N → 1 | Traduction d'un champ d'une entité (category / menu_item / modifier_group / modifier) dans une langue (FR / NL / DE / EN / ZH) |
| `translations` → `languages` | N → 1 | Référence le code langue |
| `fiscal_counters` (autonome) | - | Pas de FK ; clé `event_label` ∈ {N, P, T, C, I, F, S, R} |

---

## 4. Append-only et intégrité

Les tables ci-dessous sont protégées **au niveau de la base** par des triggers SQLite (`RAISE(ABORT, ...)`), en plus de la discipline applicative :

| Table | Triggers | Effet |
|---|---|---|
| `fiscal_events` | `fiscal_events_no_delete`, `fiscal_events_no_rewrite` | Ni DELETE, ni UPDATE |
| `fiscal_events` | `fiscal_events_period_open` (BEFORE INSERT) | Refuse un événement dont le `bookingPeriodId` désigne une période close |
| `z_reports` | `z_reports_no_delete`, `z_reports_no_rewrite` | Ni DELETE, ni UPDATE |
| `order_entries` | `order_entries_no_delete`, `order_entries_no_rewrite`, `order_entries_till_op_fixed` | Pas de DELETE ; UPDATE interdit sauf `sent_event_id` ; la marque `till_op` (migration 30) ne change jamais |
| `booking_periods` | `booking_periods_no_reopen`, `booking_periods_no_delete` | Une période close ne change plus ; pas de DELETE |
| `ticket_deliveries` | `ticket_deliveries_no_delete`, `ticket_deliveries_no_rewrite` | Pas de DELETE ; `fiscal_event_id`, `ticket_text`, `medium` immuables (les autres colonnes suivent le cycle de vie de l'envoi : append-only **partiel**) |
| `original_prints` | `original_prints_no_delete`, `original_prints_no_update` | Ni DELETE, ni UPDATE, aucune exception |
| `pending_signs` | `pending_signs_request_immutable`, `pending_signs_context_on_resolution`, `pending_signs_no_reopen`, `pending_signs_no_delete` | Ce qui a été envoyé est immuable ; une ligne résolue ne change plus ; pas de DELETE |
| `order_price_changes` | `order_price_changes_no_delete`, `order_price_changes_no_update` | Ni DELETE, ni UPDATE |
| `till_discounts` | `till_discounts_no_delete`, `till_discounts_no_update` | Ni DELETE, ni UPDATE |
| `integration_switch_log` | `integration_switch_log_no_update`, `integration_switch_log_no_delete` | Ni DELETE, ni UPDATE |
| `table_plan_log` | `table_plan_log_no_update`, `table_plan_log_no_delete` | Ni DELETE, ni UPDATE |
| `invoices` | `invoices_no_update`, `invoices_no_delete` | Ni DELETE, ni UPDATE |
| `invoice_sales` | `invoice_sales_no_update`, `invoice_sales_no_delete` | Ni DELETE, ni UPDATE |
| `peppol_delivery_log` | `peppol_delivery_log_no_update`, `peppol_delivery_log_no_delete` | Ni DELETE, ni UPDATE |
| `sold_out_log` | `sold_out_log_no_update`, `sold_out_log_no_delete` | Ni DELETE, ni UPDATE |
| `menu_change_log` | `menu_change_log_no_update`, `menu_change_log_no_delete` | Ni DELETE, ni UPDATE |
| `eft_payments` | `eft_payments_no_delete`, `eft_payments_starts_sent`, `eft_payments_request_fixed`, `eft_payments_status_moves`, `eft_payments_answer_final`, `eft_payments_used_once` | Pas de DELETE ; une ligne naît `sent` et non utilisée ; la demande (commande, montant, référence, opérateur) est immuable ; seuls `sent` et `unknown` changent de statut ; une réponse définitive ne change plus (hors `used_event_id`, posé une fois pour un paiement `approved`) |
| `staff_badges` | `staff_badges_no_delete`, `staff_badges_revoke_only` | Pas de DELETE ; la seule modification possible est un retrait, une fois (l'empreinte, l'INSZ et les données de lien sont immuables) |
| `badge_log` | `badge_log_no_update`, `badge_log_no_delete` | Ni DELETE, ni UPDATE |
| `badge_scans` | `badge_scans_no_update`, `badge_scans_no_delete` | Ni DELETE, ni UPDATE |
| `owner_settings_log` | `owner_settings_log_no_update`, `owner_settings_log_no_delete` | Ni DELETE, ni UPDATE |
| `owner_alert_log` | `owner_alert_log_no_update`, `owner_alert_log_no_delete` | Ni DELETE, ni UPDATE |
| `allergen_change_log` | `allergen_change_log_no_update`, `allergen_change_log_no_delete` | Ni DELETE, ni UPDATE |
| `gift_vouchers` | `gift_vouchers_no_update`, `gift_vouchers_no_delete` | Ni DELETE, ni UPDATE |
| `gift_voucher_movements` | `gift_voucher_movements_no_update`, `gift_voucher_movements_no_delete` | Ni DELETE, ni UPDATE |
| `gift_voucher_prints` | `gift_voucher_prints_no_update`, `gift_voucher_prints_no_delete` | Ni DELETE, ni UPDATE |

Soit 60 triggers.

Pour les autres tables (catalogue, commandes en cours, paiements, utilisateurs, `staff_shifts`, tables HubRise hors journal) les opérations UPDATE/DELETE sont autorisées via la couche commandes. Les `orders` sont marquées `status='closed'` ou `status='voided'` plutôt que supprimées. Le catalogue synchronisé met à jour ses lignes en place ; l'historique fiscal ne dépend pas de ces lignes (nom et prix sont figés dans `order_lines` / `order_entries`).

La contrainte `UNIQUE(event_type, sequence_number)` de `fiscal_events` empêche tout doublon de compteur fiscal au niveau base, par type d'événement. La table `fiscal_counters` est mise à jour atomiquement dans la même transaction SQLite que l'insert dans `fiscal_events`, sous le verrou de signature unique (cf. *STRATEGIE_COMPTEURS_FR.md* §5). Les requêtes de signature en attente de réponse sont conservées dans `pending_signs` (cf. *SCHEMA_BD_FR.md* §3.9).

---

## 5. Indexes

Index explicites (hors index implicites des clés primaires et des contraintes UNIQUE) :

| Table | Index | Colonnes | Raison |
|---|---|---|---|
| `translations` | `idx_translations_lookup` | `entity_type, entity_id, lang` | Lookup rapide pour le rendu UI multilingue |
| `orders` | `idx_orders_hubrise_order_id` (unique partiel, `WHERE hubrise_order_id IS NOT NULL`) | `hubrise_order_id` | Ingestion idempotente des commandes en ligne |
| `order_entries` | `idx_order_entries_order` | `order_id, id` | Relire le journal d'une commande dans l'ordre |
| `booking_periods` | `idx_booking_periods_one_open` (unique partiel, `WHERE closed_at IS NULL`) | `(closed_at IS NULL)` | Au plus une période ouverte |
| `ticket_deliveries` | `idx_ticket_deliveries_due` | `status, next_attempt_at` | Sélection rapide des envois dus par le worker de réessai (§III.H) |
| `staff_shifts` | `idx_staff_shifts_open` | `user_insz, ended_at` | Retrouver le poste ouvert d'un employé |
| `pending_signs` | `idx_pending_signs_one_open` (unique partiel, `WHERE status = 'open'`) | `(status)` | Au plus une requête de signature ouverte |
| `pending_signs` | `idx_pending_signs_order` | `order_id, status` | Retrouver la requête en attente d'une commande |
| `peppol_deliveries` | `idx_peppol_deliveries_invoice` | `invoice_no` | Recherche par numéro de facture (table inutilisée en v1.0.0) |
| `hubrise_events` | `idx_hubrise_events_pending` (partiel, `WHERE processed_at IS NULL`) | `received_at` | Événements HubRise restant à traiter |
| `hubrise_outbox` | `idx_hubrise_outbox_pending` (partiel, `WHERE sent_at IS NULL`) | `created_at` | Statuts restant à envoyer |
| `hubrise_outbox` | `idx_hubrise_outbox_private_ref` (unique partiel, `WHERE private_ref IS NOT NULL`) | `private_ref` | Une création de commande n'est mise en file qu'une fois |
| `hubrise_outbox` | `idx_hubrise_outbox_order` (partiel, `WHERE sent_at IS NULL`) | `hubrise_order_id, id` | Envoi dans l'ordre, par commande |
| `webshop_orders` | `idx_webshop_orders_state` | `fiscal_state, resolution` | Liste des commandes en ligne par état |
| `order_price_changes` | `idx_order_price_changes_order` | `order_id, entry_id` | Remises d'une commande |
| `till_discounts` | `idx_till_discounts_order` | `order_id, id` | Remises de caisse d'une commande |
| `eft_payments` | `idx_eft_payments_order` | `order_id` | Paiements carte d'une commande |
| `eft_payments` | `idx_eft_payments_open` (partiel, `WHERE status IN ('sent','unknown')`) | `status` | Paiements qui attendent encore leur réponse |
| `owner_alert_log` | `idx_owner_alert_log_kind` | `kind, id` | Dernier message d'une alerte |
| `allergen_change_log` | `idx_allergen_change_log_entity` | `entity_type, entity_id, id` | Historique d'un produit ou d'une option |
| `order_line_courses` | `idx_order_line_courses_order` | `order_id` | Temps des lignes d'une commande |
| `gift_voucher_movements` | `idx_gift_voucher_movements_voucher` | `voucher_id, id` | Solde et historique d'un bon |

Index implicites utiles : `fiscal_events` (`id`, `UNIQUE(event_type, sequence_number)`), `users` (`insz`), `translations` (`UNIQUE(entity_type, entity_id, field, lang)`), `webshop_orders` (`order_id`, `hubrise_order_id`).

Des indexes additionnels peuvent être ajoutés en post-cert si des problèmes de performance apparaissent à plus grande échelle (multi-restaurant). Pour le périmètre mono-terminal (~150 tickets/jour pour un restaurant type), aucun index supplémentaire n'est nécessaire.

---

## 6. Tables des commandes en ligne

Les tables des commandes en ligne (`webshop_orders`, `hubrise_channels`,
`hubrise_cash_payment_refs`, `hubrise_outbox`, `hubrise_events`,
`hubrise_connection`, `order_price_changes`, `integration_switches`,
`integration_switch_log`, `catalog_sync_state`, migrations 20 à 25 et
colonnes de la migration 21) sont décrites dans `SCHEMA_BD_FR.md` §5.
Elles ne sont écrites que si l'intégration est activée pour
l'installation (`COMMANDES_EN_LIGNE_FR.md` §2), sauf le journal de
l'interrupteur, qui reçoit au moins la valeur de départ.
`webshop_orders.order_id` référence `orders.id` ; ses colonnes
`p_event_id`, `n_event_id` et `refund_event_id` référencent
`fiscal_events.id`.

---

## 7. Versionnement du schéma

Le schéma est versionné via `pos-db::migrate::run_migrations`. La table `_migrations_applied` enregistre chaque migration appliquée (`version`, `name`, `applied_at`). Aucune modification de schéma post-cert n'est possible sans une nouvelle demande de certification (Article 109 du manuel).

Version actuelle du schéma : migrations `1` à `11`, `20` à `28`, `30` et `31`, `40` à `42`, `60` à `63`, puis `70`
(trente migrations ; 12 à 19 sont réservées aux correctifs du binaire
certifié) : `initial-schema`, `catalogue-fiscal-fields`,
`booking-periods`, `fiscal-archive-append-only`, `ticket-deliveries`,
`original-prints`, `german-language`, `meal-voucher-payments`,
`shifts-and-pre-bill`, `pending-signs`, `table-plan`, `hubrise-transport`,
`catalog-sync`, `webshop-orders`, `price-changes`,
`platform-charges`, `integration-switch`, `product-codes`,
`eft-payments`, `presence-registration`, `till-discounts`, `kitchen-notes`,
`invoicing`, `sold-out`, `menu-editor`, `owner-settings-alerts`,
`allergens`, `courses`, `gift-vouchers`, `staff-badges` - liste exacte dans
`SCHEMA_BD_FR.md` §6 et `crates/pos-db/src/migrate.rs`
(`MIGRATIONS`, `HUBRISE_MIGRATIONS`, `CATALOG_MIGRATIONS`,
`OWNER_MIGRATIONS`). Les migrations 20 à 28, 30, 31, 40 à 42, 60 à 63 et
70 font partie du binaire certifié.
