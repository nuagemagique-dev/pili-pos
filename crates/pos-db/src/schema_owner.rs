//! Schema of the owner features, migrations 60 to 63, in every
//! build (no cargo feature). Each task of the owner-features lane adds its
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

/// v62 - courses and "Envoyer la suite" (kitchen printing only).
///
/// Kitchen data, never fiscal: nothing here reaches `OrderLine`, a
/// signed request, a product name or a department. A line has no row
/// in `order_line_courses` until a course is set for it; no row means
/// course 0 ("Direct", printed at once). `line_id` carries no foreign
/// key on purpose: a corrected line is deleted from `order_lines`, but
/// the journal entries it left (`order_entries.line_id`) still need
/// their course to be held and netted at release.
///
/// `order_course_releases` lists the courses released for an order; the
/// released course is the highest one, 1 when none (courses 0 and 1 are
/// printed with the first slip).
///
/// `kitchen_slip_entries` records the journal entries a kitchen slip
/// really printed (only when the printer reported `Printed`), with the
/// course they had then. Entries signed before this migration are
/// recorded as printed (their slip was printed or abandoned long ago).
pub(crate) const MIGRATION_062_COURSES: &str = r#"
CREATE TABLE IF NOT EXISTS order_line_courses (
    line_id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES orders(id),
    course INTEGER NOT NULL CHECK(course BETWEEN 0 AND 3),
    -- INSZ of the operator who set it; NULL when the product's category gave it.
    set_by TEXT,
    set_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_order_line_courses_order ON order_line_courses(order_id);

CREATE TABLE IF NOT EXISTS order_course_releases (
    order_id INTEGER NOT NULL REFERENCES orders(id),
    course INTEGER NOT NULL CHECK(course BETWEEN 1 AND 3),
    released_by TEXT,
    released_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (order_id, course)
);

CREATE TABLE IF NOT EXISTS kitchen_slip_entries (
    entry_id INTEGER PRIMARY KEY REFERENCES order_entries(id),
    course INTEGER NOT NULL CHECK(course BETWEEN 0 AND 3),
    printed_at TEXT NOT NULL DEFAULT (datetime('now')),
    slip TEXT NOT NULL CHECK(slip IN ('immediate','release','sale'))
);

INSERT OR IGNORE INTO kitchen_slip_entries (entry_id, course, slip)
    SELECT id, 0, 'immediate' FROM order_entries WHERE sent_event_id IS NOT NULL;
"#;

/// v63 - multi-purpose gift vouchers (MPV, spec v2.2 §5 VAT X and
/// `VOUCHER_STORE`).
///
/// A voucher is created in the archive transaction of the sale (event N
/// or T) that sold it, and every use is a movement written in the
/// archive transaction of the sale that redeemed it. Nothing here is
/// updated or deleted: the balance, the expiry status and the
/// cancellation are computed from the movements (`SUM(amount_cents)`,
/// `expires_on`, a `cancel` movement). A voucher sold in training mode
/// is marked `training` and is usable in training mode only.
///
/// The system product that sells a voucher is `menu_items.external_id =
/// 'pili:gift-voucher'`, VAT X, out of scope, no kitchen printer, free
/// price, and inactive so the grid never shows it and catalog syncs
/// (which deactivate rows they do not list) have nothing to change. Its
/// category `pili:system` is inactive for the same reason. Only the
/// columns of the base schema are written (the catalog columns have
/// defaults). Their ids (990000, 990001) are fixed and far above any
/// catalog id so existing rows and fixtures never collide with them.
pub(crate) const MIGRATION_063_GIFT_VOUCHERS: &str = r#"
INSERT OR IGNORE INTO categories (id, external_id, sort_order, active)
    VALUES (990000, 'pili:system', 9999, 0);

INSERT OR IGNORE INTO menu_items
    (id, external_id, category_id, price_cents, vat_code, vat_code_takeaway, vat_category,
     kitchen_printer, sort_order, active, open_price)
    SELECT 990001, 'pili:gift-voucher', c.id, 0, 'X', 'X', 'out_of_scope', 0, 0, 0, 1
    FROM categories c WHERE c.external_id = 'pili:system';

INSERT OR IGNORE INTO translations (entity_type, entity_id, field, lang, value)
    SELECT 'category', c.id, 'name', t.lang, t.value
    FROM categories c,
         (SELECT 'fr' AS lang, 'Systeme' AS value UNION ALL SELECT 'en', 'System'
          UNION ALL SELECT 'nl', 'Systeem' UNION ALL SELECT 'zh', '系统') t
    WHERE c.external_id = 'pili:system';

INSERT OR IGNORE INTO translations (entity_type, entity_id, field, lang, value)
    SELECT 'menu_item', m.id, 'name', t.lang, t.value
    FROM menu_items m,
         (SELECT 'fr' AS lang, 'Bon cadeau' AS value UNION ALL SELECT 'en', 'Gift voucher'
          UNION ALL SELECT 'nl', 'Cadeaubon' UNION ALL SELECT 'zh', '礼品券') t
    WHERE m.external_id = 'pili:gift-voucher';

CREATE TABLE IF NOT EXISTS gift_vouchers (
    id INTEGER PRIMARY KEY,
    -- 12 digits: 10 random + 2 check digits (97 - n mod 97).
    code TEXT NOT NULL UNIQUE,
    face_value_cents INTEGER NOT NULL CHECK(face_value_cents > 0),
    issued_on TEXT NOT NULL,
    expires_on TEXT NOT NULL,
    training INTEGER NOT NULL CHECK(training IN (0,1)),
    issue_event_id INTEGER NOT NULL REFERENCES fiscal_events(id),
    issue_order_id INTEGER NOT NULL,
    issue_line_id INTEGER NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TRIGGER IF NOT EXISTS gift_vouchers_no_update
BEFORE UPDATE ON gift_vouchers
BEGIN
    SELECT RAISE(ABORT, 'gift_vouchers is append-only');
END;

CREATE TRIGGER IF NOT EXISTS gift_vouchers_no_delete
BEFORE DELETE ON gift_vouchers
BEGIN
    SELECT RAISE(ABORT, 'gift_vouchers is append-only');
END;

CREATE TABLE IF NOT EXISTS gift_voucher_movements (
    id INTEGER PRIMARY KEY,
    voucher_id INTEGER NOT NULL REFERENCES gift_vouchers(id),
    -- issue +face, redeem -x, cancel -balance.
    kind TEXT NOT NULL CHECK(kind IN ('issue','redeem','cancel')),
    amount_cents INTEGER NOT NULL,
    fiscal_event_id INTEGER NOT NULL REFERENCES fiscal_events(id),
    order_id INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_gift_voucher_movements_voucher
    ON gift_voucher_movements(voucher_id, id);

CREATE TRIGGER IF NOT EXISTS gift_voucher_movements_no_update
BEFORE UPDATE ON gift_voucher_movements
BEGIN
    SELECT RAISE(ABORT, 'gift_voucher_movements is append-only');
END;

CREATE TRIGGER IF NOT EXISTS gift_voucher_movements_no_delete
BEFORE DELETE ON gift_voucher_movements
BEGIN
    SELECT RAISE(ABORT, 'gift_voucher_movements is append-only');
END;

CREATE TABLE IF NOT EXISTS gift_voucher_prints (
    id INTEGER PRIMARY KEY,
    voucher_id INTEGER NOT NULL REFERENCES gift_vouchers(id),
    kind TEXT NOT NULL CHECK(kind IN ('original','duplicate','balance')),
    printed_by TEXT,
    printed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TRIGGER IF NOT EXISTS gift_voucher_prints_no_update
BEFORE UPDATE ON gift_voucher_prints
BEGIN
    SELECT RAISE(ABORT, 'gift_voucher_prints is append-only');
END;

CREATE TRIGGER IF NOT EXISTS gift_voucher_prints_no_delete
BEFORE DELETE ON gift_voucher_prints
BEGIN
    SELECT RAISE(ABORT, 'gift_voucher_prints is append-only');
END;
"#;
