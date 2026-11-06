//! Schema of the table zones and the covers (migration 91, UX audit T1 and
//! T5). Part of the certified binary on every installation.
//!
//! Local data, never fiscal: nothing here reaches an FDM message, a signed
//! ticket, an archived event or an export. The table identifier of every
//! signed request stays the table number.

/// v91 - the names of the table zones and the covers of an order.
///
/// - `table_plan.zone_names` and `table_plan_log.zone_names`: the optional
///   names of the zone letters of the plan ("A · Salle, B · Terrasse"), kept
///   with the plan they belong to and logged with it. The log stays
///   append-only (its triggers refuse an update and a delete; adding a column
///   is not one).
/// - `order_covers`: the number of guests of an order (1 to 30), set from the
///   cart header. One row per order that has one; the row is replaced when
///   the number changes and deleted when the operator clears it. It shows on
///   the table card and is only a hint for the number of equal parts at
///   checkout (never applied by itself).
pub(crate) const MIGRATION_091_TABLE_ZONES_COVERS: &str = r"
ALTER TABLE table_plan ADD COLUMN zone_names TEXT NOT NULL DEFAULT '';
ALTER TABLE table_plan_log ADD COLUMN zone_names TEXT NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS order_covers (
    order_id INTEGER PRIMARY KEY REFERENCES orders(id),
    covers INTEGER NOT NULL CHECK(covers BETWEEN 1 AND 30),
    set_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
";
