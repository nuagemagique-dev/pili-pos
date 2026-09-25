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
-- cert/legal audit trail. Never delete — same retention rule as
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

/// Migration 2 — fiscal catalogue fields (manual §III product lines).
///
/// - `menu_items.vat_code_takeaway`: VAT label for takeaway sales
///   ("À emporter" column); NULL means the dine-in label applies.
/// - `order_lines.department_id` / `department_name`: the product's
///   category at the time the line was added, sent to the FDM as
///   `departmentId` / `departmentName` and totalled in X/Z reports.
pub(crate) const MIGRATION_002_CATALOGUE_FISCAL_FIELDS: &str = r#"
ALTER TABLE menu_items ADD COLUMN vat_code_takeaway TEXT
    CHECK(vat_code_takeaway IS NULL OR vat_code_takeaway IN ('A','B','C','D','X'));
ALTER TABLE order_lines ADD COLUMN department_id TEXT;
ALTER TABLE order_lines ADD COLUMN department_name TEXT;
"#;

/// Apply all schema migrations. Delegates to the versioned runner in
/// `crate::migrate` so every entry point goes through the same
/// `_migrations_applied` tracking. Kept as a re-export for the many
/// existing call sites (`pos_db::schema::run_migrations`) so we don't
/// have to chase down imports every time a migration changes.
pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    crate::migrate::run_migrations(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();
    }
}
