//! The cached ranking of the best sellers (migration 92, UX audit C12).
//!
//! Local data, never fiscal. The engine computes the ranking from the
//! archived sales (`commands/favourites.rs`); this module only stores it and
//! tells whether it is due again: an hour old, or a Z report made since.

use rusqlite::{Connection, OptionalExtension, params};

/// How old the ranking may get before it is computed again, in minutes.
pub const MAX_AGE_MINUTES: f64 = 60.0;

/// The id of the last Z report, 0 when none was made.
pub fn latest_z_id(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row("SELECT COALESCE(MAX(id), 0) FROM z_reports", [], |r| {
        r.get(0)
    })
}

/// Whether the cached ranking must be computed again: none stored yet, one
/// hour old (SQLite's clock, like every `datetime('now')`), or a Z report was
/// made after it was computed.
pub fn is_due(conn: &Connection) -> Result<bool, rusqlite::Error> {
    let meta: Option<(i64, f64)> = conn
        .query_row(
            "SELECT last_z_id, (julianday('now') - julianday(computed_at)) * 1440.0
             FROM favourites_cache_meta WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((last_z, age_minutes)) = meta else {
        return Ok(true);
    };
    Ok(age_minutes >= MAX_AGE_MINUTES || latest_z_id(conn)? != last_z)
}

/// Replace the cached ranking by `ranking` (product id and units, best first)
/// and stamp it with now and the last Z report. Atomic.
pub fn replace(conn: &Connection, ranking: &[(i64, i64)]) -> Result<(), rusqlite::Error> {
    super::with_savepoint(conn, |tx| {
        let last_z = latest_z_id(tx)?;
        tx.execute("DELETE FROM favourites_top_sold", [])?;
        for (index, (item, units)) in ranking.iter().enumerate() {
            let rank = i64::try_from(index + 1).unwrap_or(i64::MAX);
            tx.execute(
                "INSERT INTO favourites_top_sold (rank, menu_item_id, units) VALUES (?1, ?2, ?3)",
                params![rank, item, units],
            )?;
        }
        tx.execute(
            "INSERT INTO favourites_cache_meta (id, computed_at, last_z_id)
             VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?1)
             ON CONFLICT(id) DO UPDATE SET computed_at = excluded.computed_at,
                 last_z_id = excluded.last_z_id",
            params![last_z],
        )?;
        Ok(())
    })
}

/// The cached ranking: product id and units, best first.
pub fn ranking(conn: &Connection) -> Result<Vec<(i64, i64)>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT menu_item_id, units FROM favourites_top_sold ORDER BY rank")?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::migrate::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn nothing_cached_is_due_then_a_fresh_ranking_is_not() {
        let conn = db();
        assert!(is_due(&conn).unwrap());
        replace(&conn, &[(7, 12), (3, 5)]).unwrap();
        assert!(!is_due(&conn).unwrap());
        assert_eq!(ranking(&conn).unwrap(), vec![(7, 12), (3, 5)]);
    }

    #[test]
    fn an_hour_old_ranking_is_due() {
        let conn = db();
        replace(&conn, &[(1, 1)]).unwrap();
        conn.execute(
            "UPDATE favourites_cache_meta SET computed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-61 minutes')",
            [],
        )
        .unwrap();
        assert!(is_due(&conn).unwrap());
        conn.execute(
            "UPDATE favourites_cache_meta SET computed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-59 minutes')",
            [],
        )
        .unwrap();
        assert!(!is_due(&conn).unwrap());
    }

    #[test]
    fn a_z_report_made_since_makes_it_due() {
        let conn = db();
        replace(&conn, &[(1, 1)]).unwrap();
        assert!(!is_due(&conn).unwrap());
        conn.execute(
            "INSERT INTO z_reports (report_date, sequence_number, data_json) VALUES ('2026-11-06', 1, '{}')",
            [],
        )
        .unwrap();
        assert!(is_due(&conn).unwrap());
        replace(&conn, &[(1, 1)]).unwrap();
        assert!(!is_due(&conn).unwrap());
    }

    #[test]
    fn replacing_drops_the_old_rows() {
        let conn = db();
        replace(&conn, &[(1, 9), (2, 8), (3, 7)]).unwrap();
        replace(&conn, &[(4, 2)]).unwrap();
        assert_eq!(ranking(&conn).unwrap(), vec![(4, 2)]);
    }
}
