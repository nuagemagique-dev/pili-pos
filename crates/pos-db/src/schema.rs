use rusqlite::Connection;

pub(crate) const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS languages (
    code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1
);

INSERT OR IGNORE INTO languages (code, name) VALUES
    ('fr', 'Français'),
    ('nl', 'Nederlands'),
    ('en', 'English'),
    ('zh', '中文');

CREATE TABLE IF NOT EXISTS translations (
    id INTEGER PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id INTEGER NOT NULL,
    field TEXT NOT NULL,
    lang TEXT NOT NULL REFERENCES languages(code),
    value TEXT NOT NULL,
    UNIQUE(entity_type, entity_id, field, lang)
);

CREATE INDEX IF NOT EXISTS idx_translations_lookup
    ON translations(entity_type, entity_id, lang);

-- Categories. `external_id` is the HubRise category ref used to
-- correlate local rows with remote catalog data on sync.
CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY,
    external_id TEXT UNIQUE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    color TEXT,
    active INTEGER NOT NULL DEFAULT 1
);

-- Menu items. `vat_category` mirrors the value pushed to HubRise
-- SKU custom_fields; `vat_code` is the legacy SCE code kept around
-- for existing order/reporting queries and populated by the sync
-- layer (derived from vat_category assuming takeaway default).
CREATE TABLE IF NOT EXISTS menu_items (
    id INTEGER PRIMARY KEY,
    external_id TEXT UNIQUE,
    category_id INTEGER NOT NULL REFERENCES categories(id),
    price_cents INTEGER NOT NULL,
    vat_code TEXT NOT NULL CHECK(vat_code IN ('A','B','C','D','X')),
    vat_category TEXT NOT NULL DEFAULT 'food'
        CHECK(vat_category IN ('food','beverage','zero_rated','out_of_scope')),
    kitchen_printer INTEGER NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS modifier_groups (
    id INTEGER PRIMARY KEY,
    external_id TEXT UNIQUE,
    min_select INTEGER NOT NULL DEFAULT 0,
    max_select INTEGER NOT NULL DEFAULT 1,
    menu_item_id INTEGER NOT NULL REFERENCES menu_items(id)
);

CREATE TABLE IF NOT EXISTS modifiers (
    id INTEGER PRIMARY KEY,
    external_id TEXT UNIQUE,
    group_id INTEGER NOT NULL REFERENCES modifier_groups(id),
    price_cents INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0
);

-- Orders. `is_fiscal` separates dine-in (1 = FDM signed) from webshop
-- orders received via HubRise (0 = display-only, no fiscal counter).
CREATE TABLE IF NOT EXISTS orders (
    id INTEGER PRIMARY KEY,
    table_number INTEGER,
    order_type TEXT NOT NULL CHECK(order_type IN ('dine_in','takeaway')),
    status TEXT NOT NULL CHECK(status IN ('open','closed','voided')) DEFAULT 'open',
    user_insz TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    closed_at TEXT,
    subtotal_cents INTEGER NOT NULL DEFAULT 0,
    total_cents INTEGER NOT NULL DEFAULT 0,
    hubrise_order_id TEXT,
    is_fiscal INTEGER NOT NULL DEFAULT 1,
    source TEXT NOT NULL DEFAULT 'pos_local',
    hubrise_status TEXT,
    service_type TEXT,
    -- Full HubRise Order payload kept as raw JSON so the webshop
    -- orders view can render items without touching the FK-heavy
    -- fiscal order_lines table. NULL for POS-local (dine-in) rows.
    raw_hubrise_json TEXT
);

CREATE TABLE IF NOT EXISTS order_lines (
    id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES orders(id),
    menu_item_id INTEGER NOT NULL REFERENCES menu_items(id),
    quantity INTEGER NOT NULL DEFAULT 1,
    unit_price_cents INTEGER NOT NULL,
    vat_code TEXT NOT NULL,
    name TEXT NOT NULL,
    modifiers_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS fiscal_events (
    id INTEGER PRIMARY KEY,
    event_type TEXT NOT NULL CHECK(event_type IN ('N','P','T','C','I','F','S','R')),
    sequence_number INTEGER NOT NULL,
    order_id INTEGER REFERENCES orders(id),
    pos_id TEXT NOT NULL,
    terminal_id TEXT NOT NULL,
    user_insz TEXT NOT NULL,
    event_data_json TEXT NOT NULL,
    fdm_response_json TEXT,
    fdm_id TEXT,
    fdm_datetime TEXT,
    event_label TEXT,
    short_signature TEXT,
    qr_data TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    archived INTEGER NOT NULL DEFAULT 0,
    UNIQUE(event_type, sequence_number)
);

CREATE TABLE IF NOT EXISTS payments (
    id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES orders(id),
    method TEXT NOT NULL CHECK(method IN ('cash','card','other')),
    amount_cents INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS users (
    insz TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('admin','cashier','waiter')),
    pin_hash TEXT,
    active INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS fiscal_counters (
    event_label TEXT PRIMARY KEY,
    current_value INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS z_reports (
    id INTEGER PRIMARY KEY,
    report_date TEXT NOT NULL,
    sequence_number INTEGER NOT NULL,
    data_json TEXT NOT NULL,
    fiscal_event_id INTEGER REFERENCES fiscal_events(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- HubRise connection singleton (one row per POS install).
CREATE TABLE IF NOT EXISTS hubrise_connection (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    access_token TEXT NOT NULL,
    account_id TEXT NOT NULL,
    account_name TEXT NOT NULL,
    location_id TEXT NOT NULL,
    location_name TEXT NOT NULL,
    catalog_id TEXT NOT NULL,
    catalog_name TEXT,
    customer_list_id TEXT,
    customer_list_name TEXT,
    scope TEXT NOT NULL,
    base_url TEXT NOT NULL,
    connected_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_poll_at TEXT,
    last_event_cursor TEXT,
    last_catalog_sync_at TEXT
);

-- Events received from HubRise (idempotence via HubRise event id).
CREATE TABLE IF NOT EXISTS hubrise_events (
    id TEXT PRIMARY KEY,
    resource_type TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    received_at TEXT NOT NULL DEFAULT (datetime('now')),
    processed_at TEXT,
    error_msg TEXT
);
CREATE INDEX IF NOT EXISTS idx_hubrise_events_pending
    ON hubrise_events(received_at)
    WHERE processed_at IS NULL;

-- Idempotent ingestion of HubRise webshop orders: the poller uses
-- INSERT OR IGNORE on orders(hubrise_order_id), which requires this
-- unique index. NULLs are allowed (and treated as distinct) so local
-- POS-origin rows are unaffected.
CREATE UNIQUE INDEX IF NOT EXISTS idx_orders_hubrise_order_id
    ON orders(hubrise_order_id)
    WHERE hubrise_order_id IS NOT NULL;

-- Outbox for POS → HubRise order push. Checkout writes here (pure
-- SQLite, no network) and a background worker drains the queue when
-- HubRise is reachable. If HubRise or the internet is down, orders
-- accumulate safely in this table until connectivity returns.
CREATE TABLE IF NOT EXISTS hubrise_outbox (
    id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL,
    body_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    attempts INTEGER NOT NULL DEFAULT 0,
    last_attempt_at TEXT,
    sent_at TEXT,
    error_msg TEXT
);
CREATE INDEX IF NOT EXISTS idx_hubrise_outbox_pending
    ON hubrise_outbox(created_at) WHERE sent_at IS NULL;

-- Peppol B2B e-invoice delivery audit trail. From 2026-01-01 Belgian
-- VAT-registered B2B invoices must be delivered via the Peppol network
-- (Royal Decree, BIS Billing 3.0 profile). Every UBL document we hand
-- to our Access Point produces one row here, kept indefinitely for the
-- cert/legal audit trail. Never delete - same retention rule as
-- fiscal_events.
CREATE TABLE IF NOT EXISTS peppol_deliveries (
    id INTEGER PRIMARY KEY,
    fiscal_event_id INTEGER REFERENCES fiscal_events(id),
    invoice_no TEXT NOT NULL,
    customer_endpoint_id TEXT NOT NULL,
    ap_vendor TEXT NOT NULL,
    ap_message_id TEXT,
    ubl_xml TEXT NOT NULL,
    ap_response_json TEXT,
    status TEXT NOT NULL CHECK(status IN ('queued','accepted','failed')) DEFAULT 'queued',
    error_msg TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    accepted_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_peppol_deliveries_invoice
    ON peppol_deliveries(invoice_no);
"#;

/// Migration 2 - fiscal catalogue fields (manual §III product lines).
///
/// - `menu_items.vat_code_takeaway`: VAT label for takeaway sales
///   ("À emporter" column); NULL means the dine-in label applies.
/// - `menu_items.open_price`: free-price product ("Prix libre"), the
///   operator enters the unit price when adding it.
/// - `order_lines.department_id` / `department_name`: the product's
///   category at the time the line was added, sent to the FDM as
///   `departmentId` / `departmentName` and totalled in X/Z reports.
/// - `order_entries`: append-only journal of every change to an order
///   (additions, corrections, refund lines) in chronological order.
///   `order_lines` is the consolidated view shown and printed; the FDM
///   messages carry the journal (POS description v2.2 §2, "toutes les
///   données d'entrée, par ordre chronologique, y compris les
///   corrections"). `sent_event_id` is set once an entry has been
///   transmitted in a P (order) or the final N event.
pub(crate) const MIGRATION_002_CATALOGUE_FISCAL_FIELDS: &str = r#"
ALTER TABLE menu_items ADD COLUMN vat_code_takeaway TEXT
    CHECK(vat_code_takeaway IS NULL OR vat_code_takeaway IN ('A','B','C','D','X'));
ALTER TABLE menu_items ADD COLUMN open_price INTEGER NOT NULL DEFAULT 0;
ALTER TABLE order_lines ADD COLUMN department_id TEXT;
ALTER TABLE order_lines ADD COLUMN department_name TEXT;

CREATE TABLE IF NOT EXISTS order_entries (
    id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES orders(id),
    line_id INTEGER NOT NULL,
    menu_item_id INTEGER NOT NULL,
    quantity INTEGER NOT NULL CHECK(quantity <> 0),
    unit_price_cents INTEGER NOT NULL,
    vat_code TEXT NOT NULL CHECK(vat_code IN ('A','B','C','D','X')),
    name TEXT NOT NULL,
    department_id TEXT NOT NULL,
    department_name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('entry','correction','refund')),
    sent_event_id INTEGER REFERENCES fiscal_events(id),
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_order_entries_order ON order_entries(order_id, id);
CREATE TRIGGER IF NOT EXISTS order_entries_no_delete
BEFORE DELETE ON order_entries
BEGIN
    SELECT RAISE(ABORT, 'order entries are append-only');
END;
CREATE TRIGGER IF NOT EXISTS order_entries_no_rewrite
BEFORE UPDATE OF order_id, line_id, menu_item_id, quantity, unit_price_cents,
                 vat_code, name, department_id, department_name, kind, created_at
ON order_entries
BEGIN
    SELECT RAISE(ABORT, 'order entries are append-only');
END;
"#;

/// Migration 3 - append-only enforcement for the fiscal archive.
///
/// CLAUDE.md's "Critical Behavioral Rules" state plainly: "No
/// deletion - fiscal records can NEVER be deleted or modified." The
/// `order_entries` journal already got this guarantee at the DB
/// level in migration 2 (`order_entries_no_delete` /
/// `order_entries_no_rewrite`), but `fiscal_events` - the actual
/// signed FDM archive, i.e. the table the rule is really about - and
/// `z_reports` had no such protection: any bug (or a stray manual
/// `UPDATE`/`DELETE`) could silently rewrite or erase a signed
/// fiscal record. No code path legitimately updates or deletes rows
/// in either table (archival is INSERT-only; nothing ever revises a
/// `fiscal_events` or `z_reports` row afterwards), so blocking every
/// column, unlike `order_entries`' `sent_event_id` exception, is safe.
pub(crate) const MIGRATION_004_FISCAL_ARCHIVE_APPEND_ONLY: &str = r#"
CREATE TRIGGER IF NOT EXISTS fiscal_events_no_delete
BEFORE DELETE ON fiscal_events
BEGIN
    SELECT RAISE(ABORT, 'fiscal events are append-only');
END;
CREATE TRIGGER IF NOT EXISTS fiscal_events_no_rewrite
BEFORE UPDATE ON fiscal_events
BEGIN
    SELECT RAISE(ABORT, 'fiscal events are append-only');
END;
CREATE TRIGGER IF NOT EXISTS z_reports_no_delete
BEFORE DELETE ON z_reports
BEGIN
    SELECT RAISE(ABORT, 'z-reports are append-only');
END;
CREATE TRIGGER IF NOT EXISTS z_reports_no_rewrite
BEFORE UPDATE ON z_reports
BEGIN
    SELECT RAISE(ABORT, 'z-reports are append-only');
END;
"#;

/// Apply all schema migrations. Delegates to the versioned runner in
/// `crate::migrate` so every entry point goes through the same
/// `_migrations_applied` tracking. Kept as a re-export for the many
/// existing call sites (`pos_db::schema::run_migrations`) so we don't
/// have to chase down imports every time a migration changes.
pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    crate::migrate::run_migrations(conn)
}

/// v3 - Booking periods (POS spec v2.2 §2.2.2 `bookingPeriodId`).
///
/// A booking period runs from the first event after a Z report to the
/// next Z report, and carries the booking date of its opening. It is
/// persisted so that restarting the POS never splits a day, and at most
/// one period is open at a time.
pub(crate) const MIGRATION_003_BOOKING_PERIODS: &str = r#"
CREATE TABLE IF NOT EXISTS booking_periods (
    id TEXT PRIMARY KEY,
    booking_date TEXT NOT NULL,
    opened_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    closed_at TEXT,
    closing_event_id INTEGER REFERENCES fiscal_events(id),
    CHECK ((closed_at IS NULL) = (closing_event_id IS NULL))
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_booking_periods_one_open
    ON booking_periods ((closed_at IS NULL)) WHERE closed_at IS NULL;
-- A closed period stays closed.
CREATE TRIGGER IF NOT EXISTS booking_periods_no_reopen
BEFORE UPDATE ON booking_periods
WHEN OLD.closed_at IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'booking period is closed');
END;
CREATE TRIGGER IF NOT EXISTS booking_periods_no_delete
BEFORE DELETE ON booking_periods
BEGIN
    SELECT RAISE(ABORT, 'booking periods cannot be deleted');
END;
"#;

/// v5 - Digital ticket deliveries (cert manual §III.H, "ticket
/// digital" option; POS spec v2.2 §6.5 verification URL requirement).
///
/// The digital ticket is sent by email via the Scaleway Transactional
/// Email HTTP API, with a paper fallback if it can't be delivered.
/// This table is the send outbox *and* the GDPR audit trail:
///
/// - `medium`: `DIGITAL` (email only, paper only as a failure
///   fallback) or `PAPER_DIGITAL` (paper always printed, email sent
///   as a bonus).
/// - `recipient` holds the plaintext email only as long as it's
///   needed to (re)send: nulled by `mark_sent` on success, and by the
///   72 h GDPR sweep (`purge_recipients_older_than`) regardless of
///   outcome. `recipient_hint` (masked, e.g. `j***@e***.be`) is kept
///   forever for support/dossier readability.
/// - `ticket_text` / `medium` / `fiscal_event_id` are immutable once
///   written (enforced by trigger) - they're the fiscal record of
///   *what* was sent, never deleted (like `fiscal_events` itself).
/// - `status` lifecycle: `pending` → `sending` (claimed by a worker,
///   see `sending` in the CHECK - this is what makes concurrent
///   `run_once` calls claim-atomic instead of double-sending) →
///   `sent` | `failed` (retries exhausted) | `fallback_printed`
///   (paper printed instead). A `failed` row's recipient is nulled by
///   the 72 h sweep and its status flips to `purged`.
/// - `attempts` / `next_attempt_at` / `last_error` drive the
///   exponential-backoff retry loop in `pos-mail::outbox`.
pub(crate) const MIGRATION_005_TICKET_DELIVERIES: &str = r#"
CREATE TABLE IF NOT EXISTS ticket_deliveries (
    id INTEGER PRIMARY KEY,
    fiscal_event_id INTEGER NOT NULL REFERENCES fiscal_events(id),
    medium TEXT NOT NULL CHECK(medium IN ('DIGITAL','PAPER_DIGITAL')),
    recipient TEXT,
    recipient_hint TEXT NOT NULL,
    ticket_text TEXT NOT NULL,
    verification_url TEXT,
    status TEXT NOT NULL
        CHECK(status IN ('pending','sending','sent','failed','fallback_printed','purged'))
        DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TEXT,
    provider_message_id TEXT,
    last_error TEXT,
    fallback_printed INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_ticket_deliveries_due
    ON ticket_deliveries(status, next_attempt_at);
CREATE TRIGGER IF NOT EXISTS ticket_deliveries_no_delete
BEFORE DELETE ON ticket_deliveries
BEGIN
    SELECT RAISE(ABORT, 'ticket deliveries are append-only, never delete');
END;
CREATE TRIGGER IF NOT EXISTS ticket_deliveries_no_rewrite
BEFORE UPDATE OF fiscal_event_id, ticket_text, medium
ON ticket_deliveries
BEGIN
    SELECT RAISE(ABORT, 'ticket deliveries: fiscal_event_id/ticket_text/medium are immutable');
END;
"#;

/// v6 - Original ticket prints (POS spec v2.2 §8.1: an original ticket
/// is printed once; any later print is a copy, event C).
///
/// One row per fiscal event whose original was printed. The checkout,
/// refund and copy commands record the print after the printer accepted
/// it, and refuse to print that original again.
pub(crate) const MIGRATION_006_ORIGINAL_PRINTS: &str = r#"
CREATE TABLE IF NOT EXISTS original_prints (
    fiscal_event_id INTEGER PRIMARY KEY REFERENCES fiscal_events(id),
    printed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE TRIGGER IF NOT EXISTS original_prints_no_delete
BEFORE DELETE ON original_prints
BEGIN
    SELECT RAISE(ABORT, 'original prints are append-only, never delete');
END;
CREATE TRIGGER IF NOT EXISTS original_prints_no_update
BEFORE UPDATE ON original_prints
BEGIN
    SELECT RAISE(ABORT, 'original prints are append-only, never update');
END;
"#;

/// v7 - German, the third national language (tickets are printed in
/// FR, NL or DE): a German-region catalogue can carry German names.
pub(crate) const MIGRATION_007_GERMAN_LANGUAGE: &str = r#"
INSERT OR IGNORE INTO languages (code, name) VALUES ('de', 'Deutsch');
"#;

/// v8 - Meal vouchers (split payment): the order's payment history
/// accepts a `meal_voucher` row. SQLite cannot alter a CHECK, so the
/// table is rebuilt with its rows.
pub(crate) const MIGRATION_008_MEAL_VOUCHER_PAYMENTS: &str = r#"
CREATE TABLE payments_v8 (
    id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES orders(id),
    method TEXT NOT NULL CHECK(method IN ('cash','card','meal_voucher','other')),
    amount_cents INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
INSERT INTO payments_v8 (id, order_id, method, amount_cents, created_at)
    SELECT id, order_id, method, amount_cents, created_at FROM payments;
DROP TABLE payments;
ALTER TABLE payments_v8 RENAME TO payments;
"#;

/// v9 - Screen state shown by the UI.
/// - `staff_shifts`: one row per shift, opened by a signed WorkIn and
///   closed by a signed WorkOut. Locking the screen keeps the shift
///   open, so the login screen can show who is on shift and since when.
///   The fiscal record stays the S event; this table is a local index.
/// - `orders.pre_bill_at`: when the last provisional bill was printed,
///   so the Tables screen can flag a table whose bill is on the table.
pub(crate) const MIGRATION_009_SHIFTS_AND_PRE_BILL: &str = r#"
CREATE TABLE IF NOT EXISTS staff_shifts (
    id INTEGER PRIMARY KEY,
    user_insz TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_staff_shifts_open ON staff_shifts(user_insz, ended_at);
ALTER TABLE orders ADD COLUMN pre_bill_at TEXT;
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ticket_language_can_hold_translations() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        for lang in ["fr", "nl", "de", "en", "zh"] {
            let known: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM languages WHERE code = ?1 AND active = 1",
                    [lang],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(known, 1, "{lang}");
        }
    }

    #[test]
    fn a_meal_voucher_payment_can_be_recorded_and_old_rows_are_kept() {
        let conn = Connection::open_in_memory().unwrap();
        crate::migrate::run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO orders (id, order_type, status, user_insz) VALUES (1, 'takeaway', 'closed', '00000000029');
             INSERT INTO payments (order_id, method, amount_cents) VALUES (1, 'meal_voucher', 313), (1, 'cash', 775);",
        )
        .unwrap();
        assert!(
            conn.execute(
                "INSERT INTO payments (order_id, method, amount_cents) VALUES (1, 'bitcoin', 1)",
                []
            )
            .is_err()
        );
        // Rows written before v8 survive the rebuild.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_SQL).unwrap();
        conn.execute_batch(
            "INSERT INTO orders (id, order_type, status, user_insz) VALUES (1, 'takeaway', 'closed', '00000000029');
             INSERT INTO payments (order_id, method, amount_cents) VALUES (1, 'card', 1200);",
        )
        .unwrap();
        conn.execute_batch(MIGRATION_008_MEAL_VOUCHER_PAYMENTS)
            .unwrap();
        let kept: (String, i64) = conn
            .query_row("SELECT method, amount_cents FROM payments", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(kept, ("card".to_string(), 1200));
    }

    #[test]
    fn schema_creates_all_tables() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert!(tables.contains(&"categories".to_string()));
        assert!(tables.contains(&"menu_items".to_string()));
        assert!(tables.contains(&"orders".to_string()));
        assert!(tables.contains(&"order_lines".to_string()));
        assert!(tables.contains(&"fiscal_events".to_string()));
        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"hubrise_connection".to_string()));
        assert!(tables.contains(&"hubrise_events".to_string()));
        assert!(tables.contains(&"ticket_deliveries".to_string()));
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();
    }
}
