//! Sold-out products (migration 41, V11-D14): the current state per product, the append-only
//! log of every set, clear and expiry, and the queue row of the HubRise inventory push.
//!
//! `until` is the end of the business day as RFC 3339 text with the Brussels offset, or `None`
//! for "until re-enabled". Expiry is by time: a row whose `until` is past no longer counts as
//! sold out (SQLite's `julianday` reads the offset), and the next read removes it and writes
//! its `expired` log row.

use rusqlite::{Connection, OptionalExtension, params};

/// A product that is sold out now.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SoldOutItem {
    pub menu_item_id: i64,
    /// Name in the asked language (then French, then `#id`).
    pub name: String,
    /// End of the business day (RFC 3339), `None`: until re-enabled.
    pub until: Option<String>,
    pub set_by: String,
    pub set_at: String,
}

/// One line of the log.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SoldOutLogRow {
    pub menu_item_id: i64,
    /// `set`, `clear` or `expired`.
    pub action: String,
    pub until: Option<String>,
    pub by: Option<String>,
    pub at: String,
}

/// Whether the product is sold out right now.
pub fn is_sold_out(conn: &Connection, menu_item_id: i64) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM menu_item_sold_out
                       WHERE menu_item_id = ?1
                         AND (until IS NULL OR julianday(until) > julianday('now')))",
        params![menu_item_id],
        |r| r.get::<_, i32>(0),
    )
    .map(|v| v != 0)
}

/// Write the `expired` log row of every sold-out row whose end of day is past, and remove the
/// row. Returns the ids that expired.
pub fn expire_due(conn: &Connection) -> Result<Vec<i64>, rusqlite::Error> {
    let tx = conn.unchecked_transaction()?;
    let due: Vec<(i64, String)> = {
        let mut stmt = tx.prepare(
            "SELECT menu_item_id, until FROM menu_item_sold_out
             WHERE until IS NOT NULL AND julianday(until) <= julianday('now')",
        )?;
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?
    };
    for (id, until) in &due {
        tx.execute(
            "INSERT INTO sold_out_log (menu_item_id, action, until, by) VALUES (?1, 'expired', ?2, NULL)",
            params![id, until],
        )?;
        tx.execute(
            "DELETE FROM menu_item_sold_out WHERE menu_item_id = ?1",
            params![id],
        )?;
    }
    tx.commit()?;
    Ok(due.into_iter().map(|(id, _)| id).collect())
}

/// Mark the product sold out (replacing an earlier state) and log it. Two statements: the
/// caller wraps them (with the inventory queue row) in one transaction.
pub fn set(
    conn: &Connection,
    menu_item_id: i64,
    until: Option<&str>,
    by: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO menu_item_sold_out (menu_item_id, until, set_by, set_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(menu_item_id) DO UPDATE SET
             until = excluded.until, set_by = excluded.set_by, set_at = excluded.set_at",
        params![menu_item_id, until, by],
    )?;
    conn.execute(
        "INSERT INTO sold_out_log (menu_item_id, action, until, by) VALUES (?1, 'set', ?2, ?3)",
        params![menu_item_id, until, by],
    )?;
    Ok(())
}

/// Make the product available again and log it. False (and no log row) when it was not sold out.
pub fn clear(conn: &Connection, menu_item_id: i64, by: &str) -> Result<bool, rusqlite::Error> {
    let until: Option<Option<String>> = conn
        .query_row(
            "SELECT until FROM menu_item_sold_out WHERE menu_item_id = ?1",
            params![menu_item_id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(until) = until else {
        return Ok(false);
    };
    conn.execute(
        "DELETE FROM menu_item_sold_out WHERE menu_item_id = ?1",
        params![menu_item_id],
    )?;
    conn.execute(
        "INSERT INTO sold_out_log (menu_item_id, action, until, by) VALUES (?1, 'clear', ?2, ?3)",
        params![menu_item_id, until, by],
    )?;
    Ok(true)
}

/// The products sold out now, in menu order.
pub fn list(conn: &Connection, lang: &str) -> Result<Vec<SoldOutItem>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT s.menu_item_id,
                COALESCE((SELECT value FROM translations
                          WHERE entity_type = 'menu_item' AND entity_id = m.id
                            AND field = 'name' AND lang = ?1),
                         (SELECT value FROM translations
                          WHERE entity_type = 'menu_item' AND entity_id = m.id
                            AND field = 'name' AND lang = 'fr'),
                         '#' || m.id),
                s.until, s.set_by, s.set_at
         FROM menu_item_sold_out s
         JOIN menu_items m ON m.id = s.menu_item_id
         WHERE s.until IS NULL OR julianday(s.until) > julianday('now')
         ORDER BY m.sort_order, m.id",
    )?;
    stmt.query_map(params![lang], |r| {
        Ok(SoldOutItem {
            menu_item_id: r.get(0)?,
            name: r.get(1)?,
            until: r.get(2)?,
            set_by: r.get(3)?,
            set_at: r.get(4)?,
        })
    })?
    .collect()
}

/// The log, latest first.
pub fn history(conn: &Connection, limit: i64) -> Result<Vec<SoldOutLogRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT menu_item_id, action, until, by, at FROM sold_out_log ORDER BY id DESC LIMIT ?1",
    )?;
    stmt.query_map(params![limit], |r| {
        Ok(SoldOutLogRow {
            menu_item_id: r.get(0)?,
            action: r.get(1)?,
            until: r.get(2)?,
            by: r.get(3)?,
            at: r.get(4)?,
        })
    })?
    .collect()
}

/// The SKU ref of the product (`menu_items.external_id`), if it has one.
pub fn sku_ref(conn: &Connection, menu_item_id: i64) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row(
        "SELECT external_id FROM menu_items WHERE id = ?1",
        params![menu_item_id],
        |r| r.get(0),
    )
    .optional()
    .map(Option::flatten)
}

/// Queue the product for the HubRise inventory push: one row per product, marked dirty again
/// (a new version, retries cleared) at every change.
pub fn queue_inventory_sync(
    conn: &Connection,
    menu_item_id: i64,
    sku_ref: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO hubrise_inventory_sync (menu_item_id, sku_ref) VALUES (?1, ?2)
         ON CONFLICT(menu_item_id) DO UPDATE SET
             sku_ref = excluded.sku_ref, dirty = 1, version = version + 1, attempts = 0,
             next_attempt_at = NULL, parked_at = NULL, error_msg = NULL",
        params![menu_item_id, sku_ref],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::run_migrations;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO categories (id, sort_order) VALUES (1, 0);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order)
                 VALUES (1, 1, 500, 'B', 0), (2, 1, 600, 'B', 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value)
                 VALUES ('menu_item', 1, 'name', 'fr', 'Maki');",
        )
        .unwrap();
        conn
    }

    #[test]
    fn set_clear_and_log() {
        let c = conn();
        assert!(!is_sold_out(&c, 1).unwrap());
        set(&c, 1, None, "u1").unwrap();
        assert!(is_sold_out(&c, 1).unwrap());
        assert_eq!(list(&c, "fr").unwrap()[0].name, "Maki");
        assert_eq!(list(&c, "zh").unwrap()[0].name, "Maki");
        assert!(clear(&c, 1, "u2").unwrap());
        assert!(!clear(&c, 1, "u2").unwrap(), "nothing to clear: no log row");
        let log = history(&c, 10).unwrap();
        assert_eq!(
            log.iter().map(|r| r.action.as_str()).collect::<Vec<_>>(),
            vec!["clear", "set"]
        );
    }

    #[test]
    fn a_past_end_of_day_expires_by_time() {
        let c = conn();
        set(&c, 1, Some("2020-01-01T03:00:00+01:00"), "u1").unwrap();
        set(&c, 2, Some("2999-01-01T03:00:00+01:00"), "u1").unwrap();
        assert!(!is_sold_out(&c, 1).unwrap(), "past: no longer sold out");
        assert!(is_sold_out(&c, 2).unwrap());
        assert_eq!(list(&c, "fr").unwrap().len(), 1);
        assert_eq!(expire_due(&c).unwrap(), vec![1]);
        assert!(expire_due(&c).unwrap().is_empty(), "written once");
        let log = history(&c, 10).unwrap();
        assert_eq!(log[0].action, "expired");
        assert_eq!(log[0].menu_item_id, 1);
    }

    #[test]
    fn the_inventory_queue_keeps_one_row_per_product_and_counts_versions() {
        let c = conn();
        queue_inventory_sync(&c, 1, "sku-1").unwrap();
        c.execute(
            "UPDATE hubrise_inventory_sync SET dirty = 0, attempts = 3",
            [],
        )
        .unwrap();
        queue_inventory_sync(&c, 1, "sku-1").unwrap();
        let (n, dirty, version, attempts): (i64, i64, i64, i64) = c
            .query_row(
                "SELECT COUNT(*), MAX(dirty), MAX(version), MAX(attempts) FROM hubrise_inventory_sync",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!((n, dirty, version, attempts), (1, 1, 2, 0));
    }
}
