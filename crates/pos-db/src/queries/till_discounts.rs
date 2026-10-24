//! Discounts taken at the till (feature `hubrise`, migration 30): the
//! audit trail of every application and removal, and the reads the
//! discount command and the event builders need. The amounts
//! themselves live in `order_price_changes`.

use std::collections::BTreeSet;

use rusqlite::{Connection, params};

/// One audit row to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTillDiscount<'a> {
    pub order_id: i64,
    /// `None` for an order discount.
    pub line_id: Option<i64>,
    /// `apply` or `remove`.
    pub action: &'a str,
    /// `percent`, `amount` or `free`; `None` on a removal.
    pub kind: Option<&'a str>,
    /// The percent or the amount in cents (`None` for free and on a removal).
    pub value: Option<i64>,
    /// The change in cents; positive on a removal (what is given back).
    pub amount_cents: i64,
    pub change_id: &'a str,
    pub grouping_id: i32,
    pub reason_code: &'a str,
    pub reason_text: Option<&'a str>,
    pub manager_insz: &'a str,
}

/// A recorded audit row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TillDiscountRow {
    pub id: i64,
    pub order_id: i64,
    pub line_id: Option<i64>,
    pub action: String,
    pub kind: Option<String>,
    pub value: Option<i64>,
    pub amount_cents: i64,
    pub change_id: String,
    pub grouping_id: i32,
    pub reason_code: String,
    pub reason_text: Option<String>,
    pub manager_insz: String,
}

pub fn insert(conn: &Connection, d: &NewTillDiscount<'_>) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO till_discounts
            (order_id, line_id, action, kind, value, amount_cents, change_id, grouping_id,
             reason_code, reason_text, manager_insz)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            d.order_id,
            d.line_id,
            d.action,
            d.kind,
            d.value,
            d.amount_cents,
            d.change_id,
            d.grouping_id,
            d.reason_code,
            d.reason_text,
            d.manager_insz
        ],
    )?;
    Ok(())
}

/// The order's audit rows, oldest first.
pub fn list(conn: &Connection, order_id: i64) -> Result<Vec<TillDiscountRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, order_id, line_id, action, kind, value, amount_cents, change_id,
                grouping_id, reason_code, reason_text, manager_insz
         FROM till_discounts WHERE order_id = ?1 ORDER BY id",
    )?;
    stmt.query_map(params![order_id], |r| {
        Ok(TillDiscountRow {
            id: r.get(0)?,
            order_id: r.get(1)?,
            line_id: r.get(2)?,
            action: r.get(3)?,
            kind: r.get(4)?,
            value: r.get(5)?,
            amount_cents: r.get(6)?,
            change_id: r.get(7)?,
            grouping_id: r.get(8)?,
            reason_code: r.get(9)?,
            reason_text: r.get(10)?,
            manager_insz: r.get(11)?,
        })
    })?
    .collect()
}

/// The ids of the order's journal entries that belong to a PRICE_CHANGE
/// pair: the builders send the negative ones with `negQuantityReason`
/// PRICE_CHANGE, the bar slip skips them all.
pub fn price_change_entry_ids(
    conn: &Connection,
    order_id: i64,
) -> Result<BTreeSet<i64>, rusqlite::Error> {
    let mut stmt = conn
        .prepare("SELECT id FROM order_entries WHERE order_id = ?1 AND till_op = 'price_change'")?;
    stmt.query_map(params![order_id], |r| r.get(0))?.collect()
}

/// How many journal entries a line has, and how many of them are not
/// transmitted yet.
pub fn line_entry_counts(conn: &Connection, line_id: i64) -> Result<(i64, i64), rusqlite::Error> {
    conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(sent_event_id IS NULL), 0)
         FROM order_entries WHERE line_id = ?1",
        params![line_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
}

/// Whether any entry of the order was transmitted in a signed event.
pub fn any_entry_sent(conn: &Connection, order_id: i64) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM order_entries
                       WHERE order_id = ?1 AND sent_event_id IS NOT NULL)",
        params![order_id],
        |r| r.get(0),
    )
}

/// The highest `groupingId` of the order's price changes (0 for none).
pub fn max_grouping_id(conn: &Connection, order_id: i64) -> Result<i32, rusqlite::Error> {
    conn.query_row(
        "SELECT COALESCE(MAX(grouping_id), 0) FROM order_price_changes WHERE order_id = ?1",
        params![order_id],
        |r| r.get(0),
    )
}
