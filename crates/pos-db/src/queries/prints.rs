//! Original ticket prints (migration v6, `original_prints`).
//!
//! POS spec v2.2 §8.1: an original ticket is printed once; any later
//! print is a copy (event C). The print commands record the print once
//! the printer accepted it and refuse to print the same original again.

use rusqlite::{Connection, OptionalExtension, params};

/// Record that the original of `fiscal_event_id` was printed. Returns
/// `false` when it was already recorded (the table keeps the first
/// print time).
pub fn record_original_print(
    conn: &Connection,
    fiscal_event_id: i64,
) -> Result<bool, rusqlite::Error> {
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO original_prints (fiscal_event_id) VALUES (?1)",
        params![fiscal_event_id],
    )?;
    Ok(inserted == 1)
}

/// Whether the original of `fiscal_event_id` has been printed.
pub fn original_printed(conn: &Connection, fiscal_event_id: i64) -> Result<bool, rusqlite::Error> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM original_prints WHERE fiscal_event_id = ?1",
            params![fiscal_event_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::run_migrations;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn fiscal_event(conn: &Connection, seq: i64) -> i64 {
        conn.execute(
            "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json)
             VALUES ('N', ?1, 'P', 'T1', '00000000029', '{}')",
            params![seq],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn nothing_is_printed_at_first() {
        let conn = db();
        let id = fiscal_event(&conn, 1);
        assert!(!original_printed(&conn, id).unwrap());
    }

    #[test]
    fn a_print_is_recorded_once() {
        let conn = db();
        let id = fiscal_event(&conn, 1);
        assert!(record_original_print(&conn, id).unwrap());
        assert!(original_printed(&conn, id).unwrap());
        assert!(
            !record_original_print(&conn, id).unwrap(),
            "second record is a no-op"
        );
    }

    #[test]
    fn prints_are_tracked_per_event() {
        let conn = db();
        let a = fiscal_event(&conn, 1);
        let b = fiscal_event(&conn, 2);
        record_original_print(&conn, a).unwrap();
        assert!(original_printed(&conn, a).unwrap());
        assert!(!original_printed(&conn, b).unwrap());
    }

    #[test]
    fn a_print_needs_an_existing_fiscal_event() {
        let conn = db();
        assert!(record_original_print(&conn, 999).is_err());
    }

    #[test]
    fn prints_can_never_be_deleted_or_rewritten() {
        let conn = db();
        let id = fiscal_event(&conn, 1);
        record_original_print(&conn, id).unwrap();
        assert!(conn.execute("DELETE FROM original_prints", []).is_err());
        assert!(
            conn.execute("UPDATE original_prints SET printed_at = 'x'", [])
                .is_err()
        );
        assert!(original_printed(&conn, id).unwrap());
    }
}
