//! Schema of the v1.2 owner features, migrations 60 to 63, in every
//! build (no cargo feature). Each task of the v1.2 lane adds its
//! migration here: 60 owner settings and alerts, 61 allergens, 62
//! courses, 63 gift vouchers.

/// v60 - the owner settings store and the owner alert log.
///
/// `owner_settings` keeps one JSON document per key (`alerts`,
/// `courses`, `gift_vouchers`, ...). The key has no CHECK, so a later
/// feature adds a key without rebuilding the table; values are
/// validated in Rust. An absent key means the feature's defaults (off).
/// Every change also writes a row of `owner_settings_log`, in the same
/// transaction (`queries::owner_settings`). Both logs are append-only,
/// like the table plan log.
///
/// `owner_alert_log` records every message the alert worker tried to
/// send (or dropped): one row per transition, never updated or deleted.
/// It is also how a restart knows an alert is already open.
pub(crate) const MIGRATION_060_OWNER_SETTINGS_ALERTS: &str = r#"
CREATE TABLE IF NOT EXISTS owner_settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    -- INSZ of the manager who set it.
    changed_by TEXT
);

CREATE TABLE IF NOT EXISTS owner_settings_log (
    id INTEGER PRIMARY KEY,
    key TEXT NOT NULL,
    value_json TEXT NOT NULL,
    changed_by TEXT,
    changed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TRIGGER IF NOT EXISTS owner_settings_log_no_update
BEFORE UPDATE ON owner_settings_log
BEGIN
    SELECT RAISE(ABORT, 'owner_settings_log is append-only');
END;

CREATE TRIGGER IF NOT EXISTS owner_settings_log_no_delete
BEFORE DELETE ON owner_settings_log
BEGIN
    SELECT RAISE(ABORT, 'owner_settings_log is append-only');
END;

CREATE TABLE IF NOT EXISTS owner_alert_log (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN ('fdm','printer','terminal','z_missing','online_waiting','test')),
    transition TEXT NOT NULL CHECK(transition IN ('raised','reminder','resolved','test')),
    -- Counts, a booking date or a channel name; never personal data.
    detail TEXT,
    outcome TEXT NOT NULL CHECK(outcome IN ('sent','failed','suppressed')),
    http_status INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_owner_alert_log_kind ON owner_alert_log(kind, id);

CREATE TRIGGER IF NOT EXISTS owner_alert_log_no_update
BEFORE UPDATE ON owner_alert_log
BEGIN
    SELECT RAISE(ABORT, 'owner_alert_log is append-only');
END;

CREATE TRIGGER IF NOT EXISTS owner_alert_log_no_delete
BEFORE DELETE ON owner_alert_log
BEGIN
    SELECT RAISE(ABORT, 'owner_alert_log is append-only');
END;
"#;

/// v61 - the 14 EU allergens (Regulation 1169/2011, Annex II).
///
/// Non-fiscal display and print data per product and per option. A
/// product (or option) is *declared* when it has a row in
/// `allergen_declarations`: its list (`menu_item_allergens` /
/// `modifier_allergens`, possibly no row at all = "no allergen") is then
/// complete. Without a declaration there is no information.
///
/// `allergen_change_log` is append-only (AFSCA traceability): one row per
/// real change, with the list before and after as JSON.
pub(crate) const MIGRATION_061_ALLERGENS: &str = r#"
CREATE TABLE IF NOT EXISTS menu_item_allergens (
    menu_item_id INTEGER NOT NULL REFERENCES menu_items(id),
    allergen TEXT NOT NULL CHECK(allergen IN (
        'gluten','crustaceans','eggs','fish','peanuts','soybeans','milk',
        'nuts','celery','mustard','sesame','sulphites','lupin','molluscs')),
    level TEXT NOT NULL CHECK(level IN ('contains','may_contain')),
    PRIMARY KEY (menu_item_id, allergen)
);

CREATE TABLE IF NOT EXISTS modifier_allergens (
    modifier_id INTEGER NOT NULL REFERENCES modifiers(id),
    allergen TEXT NOT NULL CHECK(allergen IN (
        'gluten','crustaceans','eggs','fish','peanuts','soybeans','milk',
        'nuts','celery','mustard','sesame','sulphites','lupin','molluscs')),
    level TEXT NOT NULL CHECK(level IN ('contains','may_contain')),
    PRIMARY KEY (modifier_id, allergen)
);

CREATE TABLE IF NOT EXISTS allergen_declarations (
    entity_type TEXT NOT NULL CHECK(entity_type IN ('menu_item','modifier')),
    entity_id INTEGER NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('catalog','local')),
    declared_at TEXT NOT NULL DEFAULT (datetime('now')),
    -- INSZ of the manager for a local edit; NULL for a catalog sync.
    declared_by TEXT,
    PRIMARY KEY (entity_type, entity_id)
);

CREATE TABLE IF NOT EXISTS allergen_change_log (
    id INTEGER PRIMARY KEY,
    entity_type TEXT NOT NULL CHECK(entity_type IN ('menu_item','modifier')),
    entity_id INTEGER NOT NULL,
    before_json TEXT NOT NULL,
    after_json TEXT NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('catalog','local')),
    changed_by TEXT,
    changed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_allergen_change_log_entity
    ON allergen_change_log(entity_type, entity_id, id);

CREATE TRIGGER IF NOT EXISTS allergen_change_log_no_update
BEFORE UPDATE ON allergen_change_log
BEGIN
    SELECT RAISE(ABORT, 'allergen_change_log is append-only');
END;

CREATE TRIGGER IF NOT EXISTS allergen_change_log_no_delete
BEFORE DELETE ON allergen_change_log
BEGIN
    SELECT RAISE(ABORT, 'allergen_change_log is append-only');
END;
"#;
