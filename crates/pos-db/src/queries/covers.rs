//! The covers of an order (migration 91, UX audit T5): how many guests sit
//! at the table. Local data, never fiscal: it shows on the table card and
//! is the default number of equal parts at checkout, nothing else.

use rusqlite::{Connection, OptionalExtension, params};

/// Most guests an order can hold.
pub const MAX_COVERS: i64 = 30;

/// The covers of `order_id`; `None` when none were set.
pub fn get(conn: &Connection, order_id: i64) -> Result<Option<i64>, rusqlite::Error> {
    conn.query_row(
        "SELECT covers FROM order_covers WHERE order_id = ?1",
        params![order_id],
        |r| r.get(0),
    )
    .optional()
}

/// Set (`Some`, 1 to [`MAX_COVERS`]) or clear (`None`) the covers of an
/// order. A value out of range is refused by the table's own check.
pub fn set(conn: &Connection, order_id: i64, covers: Option<i64>) -> Result<(), rusqlite::Error> {
    match covers {
        Some(n) => {
            conn.execute(
                "INSERT INTO order_covers (order_id, covers) VALUES (?1, ?2)
                 ON CONFLICT(order_id) DO UPDATE SET covers = excluded.covers,
                     set_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
                params![order_id, n],
            )?;
        }
        None => {
            conn.execute(
                "DELETE FROM order_covers WHERE order_id = ?1",
                params![order_id],
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::migrate::run_migrations(&conn).unwrap();
        conn
    }

    fn order(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO orders (table_number, order_type, status, user_insz, subtotal_cents, total_cents)
             VALUES (4, 'dine_in', 'open', '94060242576', 0, 0)",
            [],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn covers_are_set_replaced_and_cleared() {
        let conn = db();
        let id = order(&conn);
        assert_eq!(get(&conn, id).unwrap(), None);
        set(&conn, id, Some(4)).unwrap();
        assert_eq!(get(&conn, id).unwrap(), Some(4));
        set(&conn, id, Some(2)).unwrap();
        assert_eq!(get(&conn, id).unwrap(), Some(2));
        set(&conn, id, None).unwrap();
        assert_eq!(get(&conn, id).unwrap(), None);
    }

    #[test]
    fn covers_stay_between_1_and_30() {
        let conn = db();
        let id = order(&conn);
        for bad in [0, -1, 31, 500] {
            assert!(set(&conn, id, Some(bad)).is_err(), "{bad}");
        }
        set(&conn, id, Some(MAX_COVERS)).unwrap();
        set(&conn, id, Some(1)).unwrap();
    }
}
