//! Schema of the HubRise integration (feature `hubrise`), from
//! migration 20. The certification build never creates these columns
//! and tables.

/// v20 - HubRise transport: event retries, error reporting, a general
/// outbox (order creations and order PATCHes, sent in order per
/// order, creations deduplicated by `private_ref`).
pub(crate) const MIGRATION_020_HUBRISE_TRANSPORT: &str = r#"
ALTER TABLE hubrise_connection ADD COLUMN auth_error_at TEXT;
ALTER TABLE hubrise_connection ADD COLUMN last_error TEXT;
ALTER TABLE hubrise_connection ADD COLUMN callback_registered_at TEXT;

ALTER TABLE hubrise_events ADD COLUMN attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE hubrise_events ADD COLUMN parked_at TEXT;

ALTER TABLE hubrise_outbox ADD COLUMN kind TEXT NOT NULL DEFAULT 'create_order'
    CHECK(kind IN ('create_order', 'patch_order'));
ALTER TABLE hubrise_outbox ADD COLUMN hubrise_order_id TEXT;
ALTER TABLE hubrise_outbox ADD COLUMN private_ref TEXT;
ALTER TABLE hubrise_outbox ADD COLUMN next_attempt_at TEXT;
ALTER TABLE hubrise_outbox ADD COLUMN parked_at TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_hubrise_outbox_private_ref
    ON hubrise_outbox(private_ref) WHERE private_ref IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_hubrise_outbox_order
    ON hubrise_outbox(hubrise_order_id, id) WHERE sent_at IS NULL;
"#;

/// v22 - online orders on their way to the FDM. One row per HubRise
/// order next to its `orders` row: how it resolved against the POS
/// menu, how it is paid, and where it stands fiscally. Channels map a
/// HubRise channel to a platform or the own webshop; cash payments
/// are told apart by their `ref`.
pub(crate) const MIGRATION_022_WEBSHOP_ORDERS: &str = r#"
CREATE TABLE IF NOT EXISTS hubrise_channels (
    -- Lower-cased HubRise `channel`; 'webshop' stands for orders
    -- without one.
    channel TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN ('platform','webshop')),
    -- FDM payment type of a prepaid order.
    payment_type TEXT NOT NULL CHECK(payment_type IN ('APP','ONLINE')),
    -- 'include': the order's charges are sold lines; 'exclude': the
    -- platform keeps them, they are not the restaurant's turnover.
    charge_policy TEXT NOT NULL DEFAULT 'include'
        CHECK(charge_policy IN ('include','exclude'))
);

INSERT OR IGNORE INTO hubrise_channels (channel, kind, payment_type) VALUES
    ('deliveroo', 'platform', 'APP'),
    ('uber eats', 'platform', 'APP'),
    ('just eat takeaway', 'platform', 'APP'),
    ('takeaway.com', 'platform', 'APP'),
    ('website', 'webshop', 'ONLINE'),
    ('webshop', 'webshop', 'ONLINE');

CREATE TABLE IF NOT EXISTS hubrise_cash_payment_refs (
    ref TEXT PRIMARY KEY
);

INSERT OR IGNORE INTO hubrise_cash_payment_refs (ref) VALUES ('CASH'), ('CSH');

CREATE TABLE IF NOT EXISTS webshop_orders (
    order_id INTEGER PRIMARY KEY REFERENCES orders(id),
    hubrise_order_id TEXT NOT NULL UNIQUE,
    channel TEXT NOT NULL,
    channel_kind TEXT CHECK(channel_kind IN ('platform','webshop')),
    service_type TEXT CHECK(service_type IN ('delivery','collection','eat_in')),
    -- NULL while the payment is unclear.
    payment_mode TEXT CHECK(payment_mode IN ('prepaid','unpaid')),
    payment_type TEXT CHECK(payment_type IN ('APP','ONLINE')),
    fiscal_state TEXT NOT NULL DEFAULT 'pending' CHECK(fiscal_state IN (
        'pending','signing','signed_p','signed_n','voided','rejected',
        'refund_pending','cancel_pending','refunded','closed')),
    resolution TEXT NOT NULL DEFAULT 'needs_review'
        CHECK(resolution IN ('ok','needs_review')),
    -- JSON: why the order needs review, and softer warnings.
    attention_json TEXT NOT NULL DEFAULT '[]',
    warnings_json TEXT NOT NULL DEFAULT '[]',
    -- JSON: the lines to sign, resolved against the menu.
    lines_json TEXT NOT NULL DEFAULT '[]',
    p_event_id INTEGER REFERENCES fiscal_events(id),
    n_event_id INTEGER REFERENCES fiscal_events(id),
    refund_event_id INTEGER REFERENCES fiscal_events(id),
    accepted_by TEXT,
    accepted_at TEXT,
    resolved_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_webshop_orders_state
    ON webshop_orders(fiscal_state, resolution);
"#;
