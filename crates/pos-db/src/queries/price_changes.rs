//! Price changes on order lines (feature `hubrise`, migration 23): the
//! discounts of an online order, on the journal entries the FDM
//! receives. Amounts are in cents, negative for a discount; scope and
//! type are the spec's words (`LINE` / `EVENT`, `PUBLIC` / `INTERNAL`).

use std::collections::BTreeMap;

use rusqlite::{Connection, params};

/// A change to record on a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPriceChange<'a> {
    pub grouping_id: i32,
    pub change_id: &'a str,
    pub name: &'a str,
    pub scope: &'a str,
    pub change_type: &'a str,
    pub amount_cents: i64,
}

/// A recorded change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceChangeRow {
    pub entry_id: i64,
    pub line_id: i64,
    pub grouping_id: i32,
    pub change_id: String,
    pub name: String,
    pub scope: String,
    pub change_type: String,
    pub amount_cents: i64,
}

/// Record `changes` on the line's latest journal entry (the one that
/// just added it), and bring the order total to the net amount.
pub fn add_to_line(
    conn: &Connection,
    line_id: i64,
    changes: &[NewPriceChange<'_>],
) -> Result<(), rusqlite::Error> {
    if changes.is_empty() {
        return Ok(());
    }
    let (order_id, entry_id): (i64, i64) = conn.query_row(
        "SELECT order_id, MAX(id) FROM order_entries WHERE line_id = ?1",
        params![line_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    for c in changes {
        conn.execute(
            "INSERT INTO order_price_changes
                (order_id, entry_id, line_id, grouping_id, change_id, name, scope,
                 change_type, amount_cents)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                order_id,
                entry_id,
                line_id,
                c.grouping_id,
                c.change_id,
                c.name,
                c.scope,
                c.change_type,
                c.amount_cents
            ],
        )?;
    }
    super::orders::recalculate_totals(conn, order_id)
}

/// Record `changes` on journal entry `entry_id` of `line_id`, and bring
/// the order total to the net amount (a till discount on the entries of
/// a `PRICE_CHANGE` pair).
pub fn add_to_entry(
    conn: &Connection,
    entry_id: i64,
    changes: &[NewPriceChange<'_>],
) -> Result<(), rusqlite::Error> {
    let (order_id, line_id): (i64, i64) = conn.query_row(
        "SELECT order_id, line_id FROM order_entries WHERE id = ?1",
        params![entry_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    for c in changes {
        conn.execute(
            "INSERT INTO order_price_changes
                (order_id, entry_id, line_id, grouping_id, change_id, name, scope,
                 change_type, amount_cents)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                order_id,
                entry_id,
                line_id,
                c.grouping_id,
                c.change_id,
                c.name,
                c.scope,
                c.change_type,
                c.amount_cents
            ],
        )?;
    }
    super::orders::recalculate_totals(conn, order_id)
}

/// A correction removing the whole line: its changes, negated, go on
/// the correction entry just journalled, so the line nets to zero.
pub(crate) fn negate_on_correction(conn: &Connection, line_id: i64) -> Result<(), rusqlite::Error> {
    let entry_id: i64 = conn.query_row(
        "SELECT MAX(id) FROM order_entries WHERE line_id = ?1",
        params![line_id],
        |r| r.get(0),
    )?;
    negate_line_on_entry(conn, line_id, entry_id)
}

/// The line's changes so far, negated, on journal entry `entry_id` (the
/// correction of a `PRICE_CHANGE` pair or of the whole line).
pub fn negate_line_on_entry(
    conn: &Connection,
    line_id: i64,
    entry_id: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO order_price_changes
            (order_id, entry_id, line_id, grouping_id, change_id, name, scope,
             change_type, amount_cents)
         SELECT order_id, ?2, line_id, grouping_id, change_id, name, scope, change_type,
                -SUM(amount_cents)
         FROM order_price_changes WHERE line_id = ?1
         GROUP BY order_id, line_id, grouping_id, change_id, name, scope, change_type
         HAVING SUM(amount_cents) <> 0",
        params![line_id, entry_id],
    )?;
    Ok(())
}

/// Whether the line carries a change (its quantity then cannot change
/// in part: the change was spread over its whole amount).
pub fn line_has_changes(conn: &Connection, line_id: i64) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM order_price_changes WHERE line_id = ?1)",
        params![line_id],
        |r| r.get(0),
    )
}

/// Σ of the order's changes, in cents.
pub fn order_total_cents(conn: &Connection, order_id: i64) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM order_price_changes WHERE order_id = ?1",
        params![order_id],
        |r| r.get(0),
    )
}

fn rows(conn: &Connection, order_id: i64) -> Result<Vec<PriceChangeRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT entry_id, line_id, grouping_id, change_id, name, scope, change_type, amount_cents
         FROM order_price_changes WHERE order_id = ?1 ORDER BY entry_id, grouping_id, id",
    )?;
    stmt.query_map(params![order_id], |r| {
        Ok(PriceChangeRow {
            entry_id: r.get(0)?,
            line_id: r.get(1)?,
            grouping_id: r.get(2)?,
            change_id: r.get(3)?,
            name: r.get(4)?,
            scope: r.get(5)?,
            change_type: r.get(6)?,
            amount_cents: r.get(7)?,
        })
    })?
    .collect()
}

/// The changes of each journal entry (the lines an FDM message
/// carries), by entry id.
pub fn by_entry(
    conn: &Connection,
    order_id: i64,
) -> Result<BTreeMap<i64, Vec<PriceChangeRow>>, rusqlite::Error> {
    let mut out: BTreeMap<i64, Vec<PriceChangeRow>> = BTreeMap::new();
    for row in rows(conn, order_id)? {
        out.entry(row.entry_id).or_default().push(row);
    }
    Ok(out)
}

/// The changes of each consolidated line (what the ticket prints), by
/// line id: Σ over the line's entries, per change.
pub fn by_line(
    conn: &Connection,
    order_id: i64,
) -> Result<BTreeMap<i64, Vec<PriceChangeRow>>, rusqlite::Error> {
    let mut out: BTreeMap<i64, Vec<PriceChangeRow>> = BTreeMap::new();
    for row in rows(conn, order_id)? {
        let changes = out.entry(row.line_id).or_default();
        match changes
            .iter_mut()
            .find(|c| c.grouping_id == row.grouping_id && c.change_id == row.change_id)
        {
            Some(c) => c.amount_cents += row.amount_cents,
            None => changes.push(row),
        }
    }
    for changes in out.values_mut() {
        changes.retain(|c| c.amount_cents != 0);
    }
    out.retain(|_, c| !c.is_empty());
    Ok(out)
}
