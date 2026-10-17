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

(Les migrations 61 à 63 s'ajouteront à ce tableau avec leurs fonctions.)

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
