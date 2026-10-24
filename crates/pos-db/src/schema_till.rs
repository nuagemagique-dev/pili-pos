//! Schema of the till additions: the discounts taken at the till
//! (manager PIN, migration 30) and the kitchen notes of the order lines
//! (migration 31). Part of the certified binary on every installation;
//! rows are written only through the discount commands (feature
//! `hubrise`, whose price change store they reuse).
//!
//! 28 and 29 are left free on purpose for the other lanes.

/// v30 - discounts at the till.
///
/// - `order_entries.till_op`: marks the two journal entries of a
///   PRICE_CHANGE pair (the correction `-q` and the re-entry `+q` of a
///   line that already went to the FDM). The builders send the negative
///   one with `negQuantityReason` PRICE_CHANGE. Set when the entry is
///   inserted, never rewritten (the `order_entries_no_rewrite` trigger
///   of migration 2 lists its columns, so a new column needs its own).
/// - `till_discounts`: the audit trail, one row per application or
///   removal: who, why, how much. The reason is local, never part of a
///   signed payload. Append-only.
pub(crate) const MIGRATION_030_TILL_DISCOUNTS: &str = r#"
ALTER TABLE order_entries ADD COLUMN till_op TEXT
    CHECK (till_op IS NULL OR till_op = 'price_change');

CREATE TRIGGER IF NOT EXISTS order_entries_till_op_fixed
BEFORE UPDATE OF till_op ON order_entries
BEGIN
    SELECT RAISE(ABORT, 'order entries are append-only');
END;

CREATE TABLE IF NOT EXISTS till_discounts (
    id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL REFERENCES orders(id),
    -- NULL for an order discount.
    line_id INTEGER,
    action TEXT NOT NULL CHECK (action IN ('apply','remove')),
    -- NULL on a removal.
    kind TEXT CHECK (kind IS NULL OR kind IN ('percent','amount','free')),
    -- The percent or the amount in cents; NULL on a removal and for
    -- a free order or line.
    value INTEGER,
    -- The change in cents, negative (the sum removed again on a removal
    -- is positive).
    amount_cents INTEGER NOT NULL,
    change_id TEXT NOT NULL,
    grouping_id INTEGER NOT NULL CHECK (grouping_id > 0),
    reason_code TEXT NOT NULL,
    reason_text TEXT,
    manager_insz TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (action = 'apply' OR kind IS NULL)
);
CREATE INDEX IF NOT EXISTS idx_till_discounts_order ON till_discounts(order_id, id);

CREATE TRIGGER IF NOT EXISTS till_discounts_no_delete
BEFORE DELETE ON till_discounts
BEGIN
    SELECT RAISE(ABORT, 'till discounts are append-only');
END;
CREATE TRIGGER IF NOT EXISTS till_discounts_no_update
BEFORE UPDATE ON till_discounts
BEGIN
    SELECT RAISE(ABORT, 'till discounts are append-only');
END;
"#;

/// v31 - kitchen notes per order line.
///
/// - `order_lines.kitchen_note`: the free text the operator wrote for the
///   kitchen ("sans wasabi"), at most 60 characters, NULL when none.
///   Local data only: it never reaches an FDM message, a product name, a
///   ticket or an export (producers FAQ Q210 b).
/// - `order_lines.kitchen_note_printed`: the text last printed on a bar /
///   kitchen slip, NULL if never (or if a removal was printed). A note
///   that differs from it has not reached the kitchen yet.
///
/// `order_lines` is the consolidated, editable view of an order; the
/// append-only journal (`order_entries`) does not carry the note.
pub(crate) const MIGRATION_031_KITCHEN_NOTES: &str = r#"
ALTER TABLE order_lines ADD COLUMN kitchen_note TEXT;
ALTER TABLE order_lines ADD COLUMN kitchen_note_printed TEXT;
"#;
