# Schéma de base de données : ajouts de la mise à jour v1.2

Statut : brouillon de la mise à jour v1.2 (hors dossier de certification v1.0.0).
Décision : V12-D1 (migrations 60 à 63) dans `docs/android/DECISIONS.md`.

Les migrations de la v1.2 sont dans `crates/pos-db/src/schema_v12.rs`, enregistrées dans
la liste `V12_MIGRATIONS` de `crates/pos-db/src/migrate.rs`, dans toutes les versions du
binaire (sans option de compilation). Elles ne modifient ni ne suppriment aucune table
fiscale existante : elles ajoutent des tables ou des colonnes facultatives.

| Migration | Nom | Fonction | Contenu |
| --- | --- | --- | --- |
| 60 | `owner-settings-alerts` | B. Alertes du propriétaire | `owner_settings`, `owner_settings_log`, `owner_alert_log` |
| 61 | `allergens` | C. Allergènes | `menu_item_allergens`, `modifier_allergens`, `allergen_declarations`, `allergen_change_log` |
| 62 | `courses` | D. Service en plusieurs temps | `order_line_courses`, `order_course_releases`, `kitchen_slip_entries` |

(La migration 63 s'ajoutera à ce tableau avec leurs fonctions.)

## Migration 60 : réglages du propriétaire et journal d'alertes

### `owner_settings`

| Colonne | Type | Rôle |
| --- | --- | --- |
| `key` | texte, clé primaire | nom du réglage (`alerts`, puis `courses` et `gift_vouchers`) |
| `value_json` | texte | valeur au format JSON, validée par le moteur (champs inconnus refusés) |
| `changed_at` | texte | date de la dernière modification |
| `changed_by` | texte, facultatif | gérant à l'origine du changement |

Pas de contrainte sur la clé : une clé absente veut dire « réglages par défaut » (fonction éteinte).

### `owner_settings_log`

Une ligne par enregistrement : `id`, `key`, `value_json`, `changed_by`, `changed_at`.
La table ne s'efface ni ne se modifie (déclencheurs `no_update` et `no_delete`). Chaque
enregistrement écrit la ligne de `owner_settings` et celle du journal dans une seule
transaction ; un enregistrement identique à la valeur courante n'écrit rien.

### `owner_alert_log`

Une ligne par message d'alerte, envoyé ou non (`docs/internal/v1.2/ALERTES_PROPRIETAIRE_FR.md`).

| Colonne | Type | Valeurs |
| --- | --- | --- |
| `id` | entier | suite croissante |
| `kind` | texte | `fdm`, `printer`, `terminal`, `z_missing`, `online_waiting`, `test` |
| `transition` | texte | `raised`, `reminder`, `resolved`, `test` |
| `detail` | texte, facultatif | compteurs, date comptable, noms de canaux ; jamais de donnée personnelle |
| `outcome` | texte | `sent`, `failed`, `suppressed` |
| `http_status` | entier, facultatif | code de réponse du service, s'il a répondu |
| `created_at` | texte | date et heure (UTC) |

Index sur (`kind`, `id`). La table ne s'efface ni ne se modifie (déclencheurs). Les valeurs
de `kind`, `transition` et `outcome` sont contrôlées par la base (contraintes `CHECK`).
Aucune ligne n'est purgée : la table reste petite (une ligne par message, au plus 20 messages envoyés par heure).

## Migration 61 : allergènes

### `menu_item_allergens` et `modifier_allergens`

Une ligne par allergène d'un produit (`menu_item_id`) ou d'une option (`modifier_id`).

| Colonne | Type | Valeurs |
| --- | --- | --- |
| `menu_item_id` ou `modifier_id` | entier | produit ou option (clé étrangère) |
| `allergen` | texte | `gluten`, `crustaceans`, `eggs`, `fish`, `peanuts`, `soybeans`, `milk`, `nuts`, `celery`, `mustard`, `sesame`, `sulphites`, `lupin`, `molluscs` (contrôlé par `CHECK`) |
| `level` | texte | `contains` ou `may_contain` (traces) |

Clé primaire (produit ou option, allergène). Les tables ne contiennent que les allergènes
présents : une liste vide n'a pas de ligne ici.

### `allergen_declarations`

Une ligne par produit ou option dont la liste est déclarée, même vide. Sans ligne : « information
non disponible ».

| Colonne | Type | Rôle |
| --- | --- | --- |
| `entity_type` | texte | `menu_item` ou `modifier` |
| `entity_id` | entier | identifiant du produit ou de l'option |
| `source` | texte | `catalog` (synchronisation) ou `local` (saisie d'un gérant) |
| `declared_at` | texte | date de la déclaration |
| `declared_by` | texte, facultatif | gérant à l'origine d'une saisie locale ; vide pour le catalogue |

### `allergen_change_log`

Une ligne par changement réel d'une liste : `id`, `entity_type`, `entity_id`, `before_json`,
`after_json`, `source`, `changed_by`, `changed_at`. La table ne s'efface ni ne se modifie
(déclencheurs `no_update` et `no_delete`). Index sur (`entity_type`, `entity_id`, `id`).

Aucune table fiscale n'est modifiée.

## Migration 62 : service en plusieurs temps

Données de cuisine uniquement, jamais fiscales (`docs/internal/v1.2/SERVICES_CUISINE_FR.md`).

### `order_line_courses`

Le temps d'une ligne de commande. Une ligne sans ligne ici est au temps 0 (Direct).

| Colonne | Type | Rôle |
| --- | --- | --- |
| `line_id` | entier, clé primaire | ligne de commande (sans clé étrangère : l'identifiant d'une ligne supprimée peut être réutilisé) |
| `order_id` | entier | commande (clé étrangère) |
| `course` | entier | 0 à 3 (contrôlé par `CHECK`) |
| `set_by` | texte, facultatif | opérateur qui a changé le temps ; vide quand la catégorie l'a donné |
| `set_at` | texte | date |

Index sur `order_id`.

### `order_course_releases`

Un temps libéré par « Envoyer la suite ». Sans ligne, le temps libéré est 1.

| Colonne | Type | Rôle |
| --- | --- | --- |
| `order_id`, `course` | clé primaire | commande et temps libéré (1 à 3, contrôlé par `CHECK`) |
| `released_by` | texte, facultatif | opérateur |
| `released_at` | texte | date |

### `kitchen_slip_entries`

Une ligne du journal de commande, une fois imprimée sur un ticket de cuisine ou de bar.
Écrite seulement quand le ticket est sorti.

| Colonne | Type | Rôle |
| --- | --- | --- |
| `entry_id` | entier, clé primaire | ligne du journal `order_entries` (clé étrangère) |
| `course` | entier | 0 à 3 |
| `printed_at` | texte | date |
| `slip` | texte | `immediate` (envoi habituel), `release` (suite) ou `sale` (vente directe), contrôlé par `CHECK` |

La migration reprend comme déjà imprimées (temps 0, `immediate`) les lignes du journal déjà
enregistrées par un événement signé. Aucune table fiscale n'est modifiée.
