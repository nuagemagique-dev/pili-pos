//! Booking periods: the span of events a Z report closes (POS spec v2.2
//! §2.2.2 `bookingPeriodId`, `bookingDate`). See migration v3.

use rusqlite::{Connection, OptionalExtension, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookingPeriod {
    pub id: String,
    /// `YYYY-MM-DD`, the local date the period opened.
    pub booking_date: String,
}

/// The open booking period, if any.
pub fn open_period(conn: &Connection) -> Result<Option<BookingPeriod>, rusqlite::Error> {
    conn.query_row(
        "SELECT id, booking_date FROM booking_periods WHERE closed_at IS NULL",
        [],
        |row| {
            Ok(BookingPeriod {
                id: row.get(0)?,
                booking_date: row.get(1)?,
            })
        },
    )
    .optional()
}

/// The open booking period, opened with `new_id` and `today` (local
/// date) when there is none: the first event after a Z opens the next
/// period.
pub fn current_period(
    conn: &Connection,
    new_id: &str,
    today: &str,
) -> Result<BookingPeriod, rusqlite::Error> {
    if let Some(open) = open_period(conn)? {
        return Ok(open);
    }
    conn.execute(
        "INSERT INTO booking_periods (id, booking_date) VALUES (?1, ?2)",
        params![new_id, today],
    )?;
    Ok(BookingPeriod {
        id: new_id.to_string(),
        booking_date: today.to_string(),
    })
}

/// Close the open period `id` with the Z report that closed it.
/// Fails if `id` is not the open period.
pub fn close_period(
    conn: &Connection,
    id: &str,
    closing_event_id: i64,
) -> Result<(), rusqlite::Error> {
    let changed = conn.execute(
        "UPDATE booking_periods
         SET closed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), closing_event_id = ?2
         WHERE id = ?1 AND closed_at IS NULL",
        params![id, closing_event_id],
    )?;
    if changed == 1 {
        Ok(())
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
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

    /// A fiscal event to close periods with (`closing_event_id` is a FK).
    fn z_event(conn: &Connection, seq: i64) -> i64 {
        conn.execute(
            "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json)
             VALUES ('R', ?1, 'P', 'T', '00000000029', '{}')",
            [seq],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn no_period_is_open_on_a_new_database() {
        assert_eq!(open_period(&db()).unwrap(), None);
    }

    #[test]
    fn the_first_event_opens_a_period_and_later_ones_reuse_it() {
        let conn = db();
        let first = current_period(&conn, "bp-1", "2026-07-03").unwrap();
        assert_eq!(first.id, "bp-1");
        assert_eq!(first.booking_date, "2026-07-03");
        // A restart, or the next day before the Z, keeps the same period.
        let again = current_period(&conn, "bp-2", "2026-07-04").unwrap();
        assert_eq!(again, first);
    }

    #[test]
    fn closing_a_period_opens_the_next_one_on_the_next_event() {
        let conn = db();
        current_period(&conn, "bp-1", "2026-07-03").unwrap();
        close_period(&conn, "bp-1", z_event(&conn, 1)).unwrap();
        assert_eq!(open_period(&conn).unwrap(), None);
        let next = current_period(&conn, "bp-2", "2026-07-04").unwrap();
        assert_eq!(next.id, "bp-2");
        assert_eq!(next.booking_date, "2026-07-04");
    }

    #[test]
    fn only_the_open_period_can_be_closed() {
        let conn = db();
        current_period(&conn, "bp-1", "2026-07-03").unwrap();
        let z = z_event(&conn, 1);
        assert!(close_period(&conn, "bp-unknown", z).is_err());
        close_period(&conn, "bp-1", z).unwrap();
        assert!(close_period(&conn, "bp-1", z_event(&conn, 2)).is_err());
    }

    #[test]
    fn two_periods_can_never_be_open() {
        let conn = db();
        current_period(&conn, "bp-1", "2026-07-03").unwrap();
        let err = conn.execute(
            "INSERT INTO booking_periods (id, booking_date) VALUES ('bp-2', '2026-07-03')",
            [],
        );
        assert!(err.is_err());
    }

    #[test]
    fn closed_periods_cannot_be_reopened_or_deleted() {
        let conn = db();
        current_period(&conn, "bp-1", "2026-07-03").unwrap();
        close_period(&conn, "bp-1", z_event(&conn, 1)).unwrap();
        assert!(
            conn.execute(
                "UPDATE booking_periods SET closed_at = NULL, closing_event_id = NULL WHERE id = 'bp-1'",
                []
            )
            .is_err()
        );
        assert!(
            conn.execute("DELETE FROM booking_periods WHERE id = 'bp-1'", [])
                .is_err()
        );
    }

    #[test]
    fn a_closed_period_always_names_its_z_report() {
        let conn = db();
        current_period(&conn, "bp-1", "2026-07-03").unwrap();
        let err = conn.execute(
            "UPDATE booking_periods SET closed_at = 'now' WHERE id = 'bp-1'",
            [],
        );
        assert!(err.is_err());
    }
}
