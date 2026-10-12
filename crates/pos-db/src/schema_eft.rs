//! Schema of the linked card payment terminal (EFT), from migration 27.
//!
//! Part of the certified binary on every installation; the table is
//! written only when `pos.json` has a `payment_terminal` section.

/// v27 - the card payments taken on the linked terminal.
///
/// One row per payment asked of the terminal, written BEFORE the
/// terminal is called (status `sent`): a crash between the call and the
/// answer leaves a row that says "a card may have been charged", never
/// a charge nobody knows about. A row is never deleted. Once a payment
/// is `approved`, `declined`, `cancelled` or `not_sent` it never
/// changes again, except that an approved payment gets its
/// `used_event_id` once: the fiscal event (the sale N) that settled it.
/// `unknown` (no final answer) is the only state that moves on, to
/// whatever the terminal says when asked again.
pub(crate) const MIGRATION_027_EFT_PAYMENTS: &str = r#"
CREATE TABLE IF NOT EXISTS eft_payments (
    id INTEGER PRIMARY KEY,
    -- UUID v4 sent to the terminal (idempotency key, status key).
    reference TEXT NOT NULL UNIQUE,
    order_id INTEGER NOT NULL REFERENCES orders(id),
    -- pos.json payment_terminal.kind
    terminal_kind TEXT NOT NULL,
    amount_cents INTEGER NOT NULL CHECK(amount_cents > 0),
    status TEXT NOT NULL DEFAULT 'sent'
        CHECK(status IN ('sent','approved','declined','cancelled','unknown','not_sent')),
    card_type TEXT CHECK(card_type IN ('debit','credit','unknown')),
    brand TEXT,
    terminal_reference TEXT,
    authorisation_code TEXT,
    masked_pan TEXT,
    customer_receipt TEXT,
    -- Why it was declined, what is missing for 'unknown', the error
    -- for 'not_sent'.
    detail TEXT,
    user_insz TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at TEXT,
    -- The sale (fiscal event N) this payment was settled in.
    used_event_id INTEGER REFERENCES fiscal_events(id),
    -- An approved payment always says which card type to sign.
    CHECK(status <> 'approved' OR card_type IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_eft_payments_order ON eft_payments(order_id);
CREATE INDEX IF NOT EXISTS idx_eft_payments_open
    ON eft_payments(status) WHERE status IN ('sent','unknown');

-- Never deleted.
CREATE TRIGGER IF NOT EXISTS eft_payments_no_delete
BEFORE DELETE ON eft_payments
BEGIN
    SELECT RAISE(ABORT, 'eft_payments rows are never deleted');
END;

-- A payment starts as 'sent' and unused.
CREATE TRIGGER IF NOT EXISTS eft_payments_starts_sent
BEFORE INSERT ON eft_payments
WHEN NEW.status <> 'sent' OR NEW.used_event_id IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'eft_payments rows start as sent and unused');
END;

-- What was asked never changes.
CREATE TRIGGER IF NOT EXISTS eft_payments_request_fixed
BEFORE UPDATE ON eft_payments
WHEN NEW.id <> OLD.id
  OR NEW.reference <> OLD.reference
  OR NEW.order_id <> OLD.order_id
  OR NEW.terminal_kind <> OLD.terminal_kind
  OR NEW.amount_cents <> OLD.amount_cents
  OR NEW.user_insz <> OLD.user_insz
  OR NEW.created_at <> OLD.created_at
BEGIN
    SELECT RAISE(ABORT, 'eft_payments: the request of a payment never changes');
END;

-- Only 'sent' and 'unknown' move on, and never back to 'sent'.
CREATE TRIGGER IF NOT EXISTS eft_payments_status_moves
BEFORE UPDATE OF status ON eft_payments
WHEN NEW.status <> OLD.status
  AND (OLD.status NOT IN ('sent','unknown') OR NEW.status = 'sent')
BEGIN
    SELECT RAISE(ABORT, 'eft_payments: this status change is not allowed');
END;

-- An answer is final: approved, declined, cancelled and not_sent rows
-- change in nothing but used_event_id (below).
CREATE TRIGGER IF NOT EXISTS eft_payments_answer_final
BEFORE UPDATE ON eft_payments
WHEN OLD.status IN ('approved','declined','cancelled','not_sent')
  AND (NEW.status <> OLD.status
    OR NEW.card_type IS NOT OLD.card_type
    OR NEW.brand IS NOT OLD.brand
    OR NEW.terminal_reference IS NOT OLD.terminal_reference
    OR NEW.authorisation_code IS NOT OLD.authorisation_code
    OR NEW.masked_pan IS NOT OLD.masked_pan
    OR NEW.customer_receipt IS NOT OLD.customer_receipt
    OR NEW.detail IS NOT OLD.detail
    OR NEW.resolved_at IS NOT OLD.resolved_at)
BEGIN
    SELECT RAISE(ABORT, 'eft_payments: a final answer never changes');
END;

-- Used once, by one sale, and only when approved.
CREATE TRIGGER IF NOT EXISTS eft_payments_used_once
BEFORE UPDATE OF used_event_id ON eft_payments
WHEN NEW.used_event_id IS NOT OLD.used_event_id
  AND (OLD.used_event_id IS NOT NULL OR OLD.status <> 'approved')
BEGIN
    SELECT RAISE(ABORT, 'eft_payments: only an approved payment is used, once');
END;
"#;
