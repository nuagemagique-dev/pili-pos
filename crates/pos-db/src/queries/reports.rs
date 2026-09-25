//! Z-report bookkeeping: numbering, storage and history.
//!
//! The report totals themselves are computed from the signed fiscal
//! events of the booking period (`pos_fiscal::report_totals`), never
//! from the order or payment tables.

use rusqlite::{Connection, params};

/// Get the next Z-report sequence number.
pub fn next_z_report_number(conn: &Connection) -> Result<i64, rusqlite::Error> {
    let current: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sequence_number), 0) FROM z_reports",
        [],
        |row| row.get(0),
    )?;
    Ok(current + 1)
}

/// Lightweight Z-report row for the history list (no data_json blob).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZReportSummary {
    pub id: i64,
    pub report_date: String,
    pub sequence_number: i64,
    pub created_at: String,
    pub fiscal_event_id: Option<i64>,
}

/// List all Z-reports (newest first, capped at 365) for the history view.
pub fn list_z_reports(conn: &Connection) -> Result<Vec<ZReportSummary>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, report_date, sequence_number, created_at, fiscal_event_id
         FROM z_reports
         ORDER BY sequence_number DESC
         LIMIT 365",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ZReportSummary {
            id: row.get(0)?,
            report_date: row.get(1)?,
            sequence_number: row.get(2)?,
            created_at: row.get(3)?,
            fiscal_event_id: row.get(4)?,
        })
    })?;
    rows.collect()
}

/// Store a Z-report record.
pub fn save_z_report(
    conn: &Connection,
    report_date: &str,
    sequence_number: i64,
    data_json: &str,
    fiscal_event_id: Option<i64>,
) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO z_reports (report_date, sequence_number, data_json, fiscal_event_id)
         VALUES (?1, ?2, ?3, ?4)",
        params![report_date, sequence_number, data_json, fiscal_event_id],
    )?;
    Ok(conn.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::run_migrations;

    #[test]
    fn next_z_report_starts_at_one() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        assert_eq!(next_z_report_number(&conn).unwrap(), 1);
    }

    #[test]
    fn save_and_count_z_reports() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        save_z_report(&conn, "2026-04-09", 1, "{}", None).unwrap();
        save_z_report(&conn, "2026-04-09", 2, "{}", None).unwrap();

        assert_eq!(next_z_report_number(&conn).unwrap(), 3);
    }

    #[test]
    fn list_z_reports_returns_newest_first() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        save_z_report(&conn, "2026-04-09", 1, "{}", None).unwrap();
        save_z_report(&conn, "2026-04-10", 2, "{}", None).unwrap();
        save_z_report(&conn, "2026-04-11", 3, "{}", None).unwrap();

        let reports = list_z_reports(&conn).unwrap();
        assert_eq!(reports.len(), 3);
        assert_eq!(reports[0].sequence_number, 3); // newest first
        assert_eq!(reports[2].sequence_number, 1);
    }

    #[test]
    fn list_z_reports_empty_db() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        assert!(list_z_reports(&conn).unwrap().is_empty());
    }
}
