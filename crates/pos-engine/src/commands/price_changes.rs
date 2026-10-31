//! Price changes of an order's lines (feature `hubrise`): read from
//! the database as the event builders and the ticket want them. A
//! build without `HubRise` records none.

use pos_db::queries::price_changes::{self, PriceChangeRow};
use pos_fiscal::price_change::{LineChange, LineChanges};
use pos_fiscal::types::{PriceChangeScope, PriceChangeType};
use rusqlite::Connection;

fn change(row: &PriceChangeRow) -> LineChange {
    LineChange {
        grouping_id: row.grouping_id,
        id: row.change_id.clone(),
        name: row.name.clone(),
        scope: if row.scope == "LINE" {
            PriceChangeScope::Line
        } else {
            PriceChangeScope::Event
        },
        change_type: if row.change_type == "INTERNAL" {
            PriceChangeType::Internal
        } else {
            PriceChangeType::Public
        },
        amount_cents: row.amount_cents,
    }
}

fn map(rows: std::collections::BTreeMap<i64, Vec<PriceChangeRow>>) -> LineChanges {
    rows.into_iter()
        .map(|(id, rows)| (id, rows.iter().map(change).collect()))
        .collect()
}

/// The order's changes: by journal entry (what the FDM receives) and
/// by consolidated line (what the ticket prints).
pub(crate) fn load(conn: &Connection, order_id: i64) -> Result<(LineChanges, LineChanges), String> {
    let entries = price_changes::by_entry(conn, order_id).map_err(|e| e.to_string())?;
    let lines = price_changes::by_line(conn, order_id).map_err(|e| e.to_string())?;
    Ok((map(entries), map(lines)))
}

/// Record `changes` on the line just added (its latest entry).
pub(crate) fn record(
    conn: &Connection,
    line_id: i64,
    changes: &[LineChange],
) -> Result<(), String> {
    let rows: Vec<_> = changes
        .iter()
        .map(|c| price_changes::NewPriceChange {
            grouping_id: c.grouping_id,
            change_id: &c.id,
            name: &c.name,
            scope: match c.scope {
                PriceChangeScope::Line => "LINE",
                PriceChangeScope::Event => "EVENT",
            },
            change_type: match c.change_type {
                PriceChangeType::Public => "PUBLIC",
                PriceChangeType::Internal => "INTERNAL",
            },
            amount_cents: c.amount_cents,
        })
        .collect();
    price_changes::add_to_line(conn, line_id, &rows).map_err(|e| e.to_string())
}
