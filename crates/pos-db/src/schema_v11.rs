//! Schema of the v1.1 update (first certified update after v1.0.0), from migration 40.
//!
//! These migrations live in their own block (40 and up, V11-D2) so that the v1.0.x lanes can
//! keep claiming 28 and up without a clash. Every build has them; no cargo feature.

/// v40 - B2B invoices (event I) and the Peppol delivery queue.
///
/// - `b2b_customers`: the customers an invoice is made out to. Not fiscal, editable; the
///   invoice keeps its own snapshot (`invoices.customer_json`).
/// - `invoices` and `invoice_sales`: one row per signed I event and the N events it covers.
///   Append-only (AM art. 32 and 33): never updated, never deleted. One N event is covered by
///   at most one invoice (`sale_fiscal_event_id UNIQUE`).
/// - `peppol_deliveries` (migration 1, never shipped with rows) becomes the delivery queue and
///   audit row: new columns for attempts, next attempt, park time and lookup status.
/// - `peppol_delivery_log`: every attempt, append-only.
/// - `v_invoice_journal`: the outgoing invoices sub-journal of AM art. 33, as a view over the
///   append-only tables (the database is the export).
pub(crate) const MIGRATION_040_INVOICING: &str = r#"
CREATE TABLE IF NOT EXISTS b2b_customers (
    id INTEGER PRIMARY KEY,
    vat_number TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    street TEXT NOT NULL,
    postal_code TEXT NOT NULL,
    city TEXT NOT NULL,
    country_code TEXT NOT NULL DEFAULT 'BE',
    peppol_status TEXT NOT NULL DEFAULT 'unknown'
        CHECK(peppol_status IN ('unknown','registered','not_registered')),
    peppol_checked_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_by TEXT
);

CREATE TABLE IF NOT EXISTS invoices (
    id INTEGER PRIMARY KEY,
    -- One continuous counter over all invoices, never reset (AM art. 32).
    invoice_seq INTEGER NOT NULL UNIQUE,
    invoice_no TEXT NOT NULL UNIQUE,
    fiscal_event_id INTEGER NOT NULL UNIQUE REFERENCES fiscal_events(id),
    customer_id INTEGER REFERENCES b2b_customers(id),
    -- The customer as printed (name, address, VAT number), frozen at signing.
    customer_json TEXT NOT NULL,
    issue_date TEXT NOT NULL,
    total_cents INTEGER NOT NULL,
    -- {"A": {"base": n, "vat": n, "total": n}, ...} in cents, per VAT label.
    vat_json TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TRIGGER IF NOT EXISTS invoices_no_update
BEFORE UPDATE ON invoices
BEGIN
    SELECT RAISE(ABORT, 'invoices is append-only');
END;

CREATE TRIGGER IF NOT EXISTS invoices_no_delete
BEFORE DELETE ON invoices
BEGIN
    SELECT RAISE(ABORT, 'invoices is append-only');
END;

CREATE TABLE IF NOT EXISTS invoice_sales (
    invoice_id INTEGER NOT NULL REFERENCES invoices(id),
    -- One sale is invoiced at most once (V11-D4).
    sale_fiscal_event_id INTEGER NOT NULL UNIQUE REFERENCES fiscal_events(id),
    position INTEGER NOT NULL,
    PRIMARY KEY (invoice_id, position)
);

CREATE TRIGGER IF NOT EXISTS invoice_sales_no_update
BEFORE UPDATE ON invoice_sales
BEGIN
    SELECT RAISE(ABORT, 'invoice_sales is append-only');
END;

CREATE TRIGGER IF NOT EXISTS invoice_sales_no_delete
BEFORE DELETE ON invoice_sales
BEGIN
    SELECT RAISE(ABORT, 'invoice_sales is append-only');
END;

-- The Peppol queue: statuses stay queued (waiting or retrying), accepted, failed (parked).
ALTER TABLE peppol_deliveries ADD COLUMN attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE peppol_deliveries ADD COLUMN next_attempt_at TEXT;
ALTER TABLE peppol_deliveries ADD COLUMN last_attempt_at TEXT;
ALTER TABLE peppol_deliveries ADD COLUMN parked_at TEXT;
ALTER TABLE peppol_deliveries ADD COLUMN lookup_status TEXT NOT NULL DEFAULT 'unknown'
    CHECK(lookup_status IN ('unknown','registered','not_registered'));
CREATE INDEX IF NOT EXISTS idx_peppol_deliveries_queue
    ON peppol_deliveries(next_attempt_at) WHERE status = 'queued';

CREATE TABLE IF NOT EXISTS peppol_delivery_log (
    id INTEGER PRIMARY KEY,
    delivery_id INTEGER NOT NULL REFERENCES peppol_deliveries(id),
    attempt INTEGER NOT NULL,
    outcome TEXT NOT NULL,
    http_status INTEGER,
    detail TEXT,
    at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_peppol_delivery_log_delivery
    ON peppol_delivery_log(delivery_id);

CREATE TRIGGER IF NOT EXISTS peppol_delivery_log_no_update
BEFORE UPDATE ON peppol_delivery_log
BEGIN
    SELECT RAISE(ABORT, 'peppol_delivery_log is append-only');
END;

CREATE TRIGGER IF NOT EXISTS peppol_delivery_log_no_delete
BEFORE DELETE ON peppol_delivery_log
BEGIN
    SELECT RAISE(ABORT, 'peppol_delivery_log is append-only');
END;

-- Outgoing invoices sub-journal (AM art. 33): one row per invoice, base and VAT per rate.
CREATE VIEW IF NOT EXISTS v_invoice_journal AS
SELECT
    i.invoice_no AS invoice_no,
    i.issue_date AS issue_date,
    json_extract(i.customer_json, '$.vat_number') AS customer_vat_number,
    json_extract(i.customer_json, '$.name') AS customer_name,
    json_extract(i.vat_json, '$.A.base') AS base_a_cents,
    json_extract(i.vat_json, '$.A.vat') AS vat_a_cents,
    json_extract(i.vat_json, '$.B.base') AS base_b_cents,
    json_extract(i.vat_json, '$.B.vat') AS vat_b_cents,
    json_extract(i.vat_json, '$.C.base') AS base_c_cents,
    json_extract(i.vat_json, '$.C.vat') AS vat_c_cents,
    json_extract(i.vat_json, '$.D.base') AS base_d_cents,
    json_extract(i.vat_json, '$.X.base') AS base_x_cents,
    i.total_cents AS total_cents,
    f.fdm_id AS fdm_id,
    f.event_label AS event_label,
    f.fdm_datetime AS fdm_datetime,
    f.sequence_number AS event_sequence,
    (SELECT p.status FROM peppol_deliveries p
      WHERE p.fiscal_event_id = i.fiscal_event_id
      ORDER BY p.id DESC LIMIT 1) AS peppol_status
FROM invoices i
JOIN fiscal_events f ON f.id = i.fiscal_event_id
ORDER BY i.invoice_seq;
"#;

/// v41 - sold-out products ("Épuisé", V11-D14, V11-D15).
///
/// - `menu_item_sold_out`: the current state, one row per product. `until` is the end of the
///   business day as RFC 3339 text with the Brussels offset, or NULL for "until re-enabled".
///   A row whose `until` is past is expired (by time, not by Z); it is kept until the next
///   read writes the `expired` log row and removes it.
/// - `sold_out_log`: every set, clear and expiry, append-only.
/// - `hubrise_inventory_sync`: one row per product pushed to the HubRise inventory (only
///   written when HubRise is on and the product has an `external_id`). `dirty` rows are drained
///   by the HubRise outbox loop, which sends the CURRENT state at send time. `version` counts
///   the changes, so a change made during a send is not lost.
///
/// No fiscal effect: sold-out is a till rule, not an event.
pub(crate) const MIGRATION_041_SOLD_OUT: &str = r#"
CREATE TABLE IF NOT EXISTS menu_item_sold_out (
    menu_item_id INTEGER PRIMARY KEY REFERENCES menu_items(id),
    until TEXT,
    set_by TEXT NOT NULL,
    set_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS sold_out_log (
    id INTEGER PRIMARY KEY,
    menu_item_id INTEGER NOT NULL REFERENCES menu_items(id),
    action TEXT NOT NULL CHECK(action IN ('set','clear','expired')),
    until TEXT,
    by TEXT,
    at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_sold_out_log_item ON sold_out_log(menu_item_id);

CREATE TRIGGER IF NOT EXISTS sold_out_log_no_update
BEFORE UPDATE ON sold_out_log
BEGIN
    SELECT RAISE(ABORT, 'sold_out_log is append-only');
END;

CREATE TRIGGER IF NOT EXISTS sold_out_log_no_delete
BEFORE DELETE ON sold_out_log
BEGIN
    SELECT RAISE(ABORT, 'sold_out_log is append-only');
END;

CREATE TABLE IF NOT EXISTS hubrise_inventory_sync (
    menu_item_id INTEGER PRIMARY KEY REFERENCES menu_items(id),
    sku_ref TEXT NOT NULL,
    dirty INTEGER NOT NULL DEFAULT 1,
    version INTEGER NOT NULL DEFAULT 1,
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TEXT,
    last_attempt_at TEXT,
    parked_at TEXT,
    error_msg TEXT,
    synced_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_hubrise_inventory_sync_dirty
    ON hubrise_inventory_sync(dirty) WHERE dirty = 1;
"#;

/// v42 - the on-screen menu editor (V11-D17).
///
/// - `menu_change_log`: every create and update of a category, product, option group or option,
///   with the row before and after as JSON (names per language included), who and when.
///   Append-only: nothing is ever updated or deleted, like `table_plan_log`.
///
/// The editor itself writes the existing menu tables; nothing is deleted from them either (an
/// `active` flag hides an entry). Order lines keep their own snapshot of name, price and VAT
/// label, so a change here never alters an open order or a signed event (AM art. 44).
pub(crate) const MIGRATION_042_MENU_EDITOR: &str = r#"
CREATE TABLE IF NOT EXISTS menu_change_log (
    id INTEGER PRIMARY KEY,
    entity_type TEXT NOT NULL
        CHECK(entity_type IN ('category','product','modifier_group','modifier')),
    entity_id INTEGER NOT NULL,
    action TEXT NOT NULL CHECK(action IN ('create','update')),
    before_json TEXT,
    after_json TEXT NOT NULL,
    changed_by TEXT NOT NULL,
    changed_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_menu_change_log_entity
    ON menu_change_log(entity_type, entity_id);

CREATE TRIGGER IF NOT EXISTS menu_change_log_no_update
BEFORE UPDATE ON menu_change_log
BEGIN
    SELECT RAISE(ABORT, 'menu_change_log is append-only');
END;

CREATE TRIGGER IF NOT EXISTS menu_change_log_no_delete
BEFORE DELETE ON menu_change_log
BEGIN
    SELECT RAISE(ABORT, 'menu_change_log is append-only');
END;
"#;
