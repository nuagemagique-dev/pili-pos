//! Schema of the sale by weight (migration 80): which products a scale
//! weighs, the weight and price on the order lines, and the local audit of
//! what the scale said. Part of the certified binary on every installation.
//!
//! `fiscal_events` does not change: the archive already holds everything
//! the FDM signs.

/// v80 - sale by weight.
///
/// - `menu_items.sold_by_weight`: `price_cents` is then the price per kilo.
/// - `menu_items.scale_tare_grams`: the container's fixed tare sent to the
///   scale, optional.
/// - `order_lines` and `order_entries`: `weight_grams` and
///   `weighed_amount_cents`, both empty on a line sold by the piece. Copied
///   to the journal when the entry is inserted, never rewritten (the
///   `order_entries_no_rewrite` trigger of migration 2 lists its columns,
///   so these need their own).
/// - `scale_weighings`: what the scale returned for each line, with its raw
///   values. The proof between the scale and the signed line. Local data,
///   never in an FDM message. Append-only.
/// - `scale_check_log`: the steps of the scale's software check.
///   Append-only.
/// - `owner_alert_log`: rebuilt, because its `kind` CHECK now also accepts
///   `scale`. Its rows, ids and append-only triggers are kept.
pub(crate) const MIGRATION_080_SALE_BY_WEIGHT: &str = r#"
ALTER TABLE menu_items ADD COLUMN sold_by_weight INTEGER NOT NULL DEFAULT 0;
ALTER TABLE menu_items ADD COLUMN scale_tare_grams INTEGER
    CHECK (scale_tare_grams IS NULL OR scale_tare_grams BETWEEN 0 AND 9999);

ALTER TABLE order_lines ADD COLUMN weight_grams INTEGER
    CHECK (weight_grams IS NULL OR weight_grams > 0);
ALTER TABLE order_lines ADD COLUMN weighed_amount_cents INTEGER
    CHECK (weighed_amount_cents IS NULL OR weighed_amount_cents >= 0);

ALTER TABLE order_entries ADD COLUMN weight_grams INTEGER
    CHECK (weight_grams IS NULL OR weight_grams > 0);
ALTER TABLE order_entries ADD COLUMN weighed_amount_cents INTEGER
    CHECK (weighed_amount_cents IS NULL OR weighed_amount_cents >= 0);

CREATE TRIGGER IF NOT EXISTS order_entries_weight_fixed
BEFORE UPDATE OF weight_grams, weighed_amount_cents ON order_entries
BEGIN
    SELECT RAISE(ABORT, 'order entries are append-only');
END;

CREATE TABLE IF NOT EXISTS scale_weighings (
    id INTEGER PRIMARY KEY,
    -- No foreign key: a removed line goes, its weighing stays.
    line_id INTEGER NOT NULL,
    order_id INTEGER NOT NULL REFERENCES orders(id),
    weight_grams INTEGER NOT NULL CHECK (weight_grams > 0),
    unit_price_cents INTEGER NOT NULL,
    amount_cents INTEGER NOT NULL,
    tare_grams INTEGER,
    operator_insz TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_scale_weighings_line ON scale_weighings(line_id);

CREATE TRIGGER IF NOT EXISTS scale_weighings_no_delete
BEFORE DELETE ON scale_weighings
BEGIN
    SELECT RAISE(ABORT, 'scale weighings are append-only');
END;
CREATE TRIGGER IF NOT EXISTS scale_weighings_no_update
BEFORE UPDATE ON scale_weighings
BEGIN
    SELECT RAISE(ABORT, 'scale weighings are append-only');
END;

CREATE TABLE IF NOT EXISTS scale_check_log (
    id INTEGER PRIMARY KEY,
    event TEXT NOT NULL CHECK (event IN ('requested','answered','passed','refused','no_values')),
    detail TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TRIGGER IF NOT EXISTS scale_check_log_no_delete
BEFORE DELETE ON scale_check_log
BEGIN
    SELECT RAISE(ABORT, 'scale check log is append-only');
END;
CREATE TRIGGER IF NOT EXISTS scale_check_log_no_update
BEFORE UPDATE ON scale_check_log
BEGIN
    SELECT RAISE(ABORT, 'scale check log is append-only');
END;

DROP TRIGGER owner_alert_log_no_update;
DROP TRIGGER owner_alert_log_no_delete;
DROP INDEX idx_owner_alert_log_kind;
ALTER TABLE owner_alert_log RENAME TO owner_alert_log_before_scale;
CREATE TABLE owner_alert_log (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN ('fdm','printer','terminal','scale','z_missing','online_waiting','test')),
    transition TEXT NOT NULL CHECK(transition IN ('raised','reminder','resolved','test')),
    -- Counts, a booking date or a channel name; never personal data.
    detail TEXT,
    outcome TEXT NOT NULL CHECK(outcome IN ('sent','failed','suppressed')),
    http_status INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
INSERT INTO owner_alert_log (id, kind, transition, detail, outcome, http_status, created_at)
    SELECT id, kind, transition, detail, outcome, http_status, created_at
    FROM owner_alert_log_before_scale;
DROP TABLE owner_alert_log_before_scale;
CREATE INDEX idx_owner_alert_log_kind ON owner_alert_log(kind, id);
CREATE TRIGGER owner_alert_log_no_update
BEFORE UPDATE ON owner_alert_log
BEGIN
    SELECT RAISE(ABORT, 'owner_alert_log is append-only');
END;
CREATE TRIGGER owner_alert_log_no_delete
BEFORE DELETE ON owner_alert_log
BEGIN
    SELECT RAISE(ABORT, 'owner_alert_log is append-only');
END;
"#;
