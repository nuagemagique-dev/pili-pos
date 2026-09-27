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
