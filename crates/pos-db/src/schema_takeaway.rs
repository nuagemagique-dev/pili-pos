//! Schema of the numbered takeaway (migration 90, UX audit T2): the number
//! of the day each takeaway sale carries ("E-61") and the optional name and
//! phone of the customer. Part of the certified binary on every
//! installation.
//!
//! Local data, never fiscal: nothing here reaches an FDM message, a signed
//! ticket, an archived event or an export.

/// v90 - the takeaway tags.
///
/// One row per takeaway order of the till (an online order keeps its
/// platform reference and gets no row):
///
/// - `day`: the business day the number belongs to (the booking date the
///   engine already uses, `YYYY-MM-DD`). The numbers restart each day, so
///   `(day, number)` is unique.
/// - `number`: the number of the day, starting at the installation's first
///   number (default 61).
/// - `customer_name` and `customer_phone`: optional, typed by the operator.
///   Personal data: the engine clears them 72 hours after the order opened,
///   like the e-mail address of a digital ticket
///   (`contact_cleared_at` says when). The number stays.
///
/// A takeaway that never held anything is voided on leaving
/// (`orders::void_if_empty`), and its row goes with it so its number is
/// handed out again when it was the last one.
pub(crate) const MIGRATION_090_TAKEAWAY_TAGS: &str = r"
CREATE TABLE IF NOT EXISTS takeaway_tags (
    order_id INTEGER PRIMARY KEY REFERENCES orders(id),
    day TEXT NOT NULL,
    number INTEGER NOT NULL CHECK(number BETWEEN 1 AND 9999),
    customer_name TEXT CHECK(customer_name IS NULL OR length(customer_name) <= 60),
    customer_phone TEXT CHECK(customer_phone IS NULL OR length(customer_phone) <= 30),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    contact_cleared_at TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_takeaway_tags_day_number
    ON takeaway_tags(day, number);
";
