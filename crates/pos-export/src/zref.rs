//! Reference data of a Z report needed by the export.
//!
//! A real Z report is a row of `z_reports` whose turnover Z event closed a
//! booking period. A training Z takes no number, closes no period and has no
//! such row, so it can never appear here; the lookup also requires the closing
//! event to be a real `R` event, so a row pointing at a training event would
//! still be ignored. Everything in this module only reads.

use rusqlite::{Connection, Row, params};

/// The Z report that closes a booking period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZRef {
    /// Z report number (`z_reports.sequence_number`).
    pub z_report_no: i64,
    /// Booking date of the closed period, `YYYY-MM-DD`.
    pub booking_date: String,
    /// Id of the closed booking period (the `bookingPeriodId` of its events).
    pub period_id: String,
    /// Local id of the turnover Z event that closed the period.
    pub closing_event_id: i64,
    /// When the period was closed (UTC, RFC 3339).
    pub closed_at: String,
}

const SELECT_REAL_Z: &str =
    "SELECT z.sequence_number, p.booking_date, p.id, p.closing_event_id, p.closed_at
     FROM z_reports z
     JOIN fiscal_events e ON e.id = z.fiscal_event_id
     JOIN booking_periods p ON p.closing_event_id = e.id
     WHERE e.event_type = 'R'
       AND p.closed_at IS NOT NULL
       AND (?1 IS NULL OR z.sequence_number > ?1)";

fn from_row(row: &Row<'_>) -> rusqlite::Result<ZRef> {
    Ok(ZRef {
        z_report_no: row.get(0)?,
        booking_date: row.get(1)?,
        period_id: row.get(2)?,
        closing_event_id: row.get(3)?,
        closed_at: row.get(4)?,
    })
}

/// The real Z reports numbered above `after` (all of them when `None`), by
/// ascending number. Training Z reports are never listed.
pub fn real_zs_after(conn: &Connection, after: Option<i64>) -> Result<Vec<ZRef>, String> {
    let sql = format!("{SELECT_REAL_Z} ORDER BY z.sequence_number ASC");
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("zref: prepare: {e}"))?;
    let rows = stmt
        .query_map(params![after], from_row)
        .map_err(|e| format!("zref: query: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("zref: read: {e}"))
}

/// The real Z report with the highest number, if any.
pub fn latest_real_z(conn: &Connection) -> Result<Option<ZRef>, String> {
    let sql = format!("{SELECT_REAL_Z} ORDER BY z.sequence_number DESC LIMIT 1");
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("zref: prepare: {e}"))?;
    let mut rows = stmt
        .query_map(params![Option::<i64>::None], from_row)
        .map_err(|e| format!("zref: query: {e}"))?;
    match rows.next() {
        None => Ok(None),
        Some(row) => row.map(Some).map_err(|e| format!("zref: read: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        pos_db::schema::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn an_empty_database_has_no_z() {
        let conn = db();
        assert_eq!(real_zs_after(&conn, None).unwrap().len(), 0);
        assert_eq!(latest_real_z(&conn).unwrap(), None);
    }

    #[test]
    fn a_z_without_a_closed_period_is_not_listed() {
        let conn = db();
        conn.execute_batch(
            "INSERT INTO fiscal_events (id, event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json)
                 VALUES (1, 'R', 1, 'P', 'T', '00000000029', '{}');
             INSERT INTO z_reports (report_date, sequence_number, data_json, fiscal_event_id)
                 VALUES ('2026-10-04', 1, '{}', 1);",
        )
        .unwrap();
        assert_eq!(real_zs_after(&conn, None).unwrap().len(), 0);
    }
}
