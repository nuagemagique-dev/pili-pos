use rusqlite::{Connection, params};

/// Turnover totals by event type for report generation.
#[derive(Debug, Clone)]
pub struct EventTotal {
    pub event_label: String,
    pub ticket_count: i64,
    pub amount_cents: i64,
}

/// VAT totals for report generation.
#[derive(Debug, Clone)]
pub struct VatTotal {
    pub vat_code: String,
    pub total_cents: i64,
}

/// Payment totals for report generation.
#[derive(Debug, Clone)]
pub struct PaymentTotal {
    pub method: String,
    pub amount_cents: i64,
    pub count: i64,
}

/// Get event totals for a given booking date (for turnover reports).
pub fn get_event_totals(
    conn: &Connection,
    booking_date: &str,
) -> Result<Vec<EventTotal>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT event_type, COUNT(*), COALESCE(SUM(
            CAST(json_extract(event_data_json, '$.transaction.transactionTotal') AS REAL) * 100
        ), 0)
         FROM fiscal_events
         WHERE DATE(created_at) = ?1
         GROUP BY event_type",
    )?;

    let rows = stmt.query_map(params![booking_date], |row| {
        Ok(EventTotal {
            event_label: row.get(0)?,
            ticket_count: row.get(1)?,
            amount_cents: row.get::<_, f64>(2)? as i64,
        })
    })?;

    rows.collect()
}

/// Get VAT totals from closed orders for a given date.
pub fn get_vat_totals(
    conn: &Connection,
    booking_date: &str,
) -> Result<Vec<VatTotal>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT ol.vat_code, SUM(ol.unit_price_cents * ol.quantity)
         FROM order_lines ol
         JOIN orders o ON o.id = ol.order_id
         WHERE o.status = 'closed' AND DATE(o.closed_at) = ?1
         GROUP BY ol.vat_code",
    )?;

    let rows = stmt.query_map(params![booking_date], |row| {
        Ok(VatTotal {
            vat_code: row.get(0)?,
            total_cents: row.get(1)?,
        })
    })?;

    rows.collect()
}

/// Get payment totals for a given date.
pub fn get_payment_totals(
    conn: &Connection,
    booking_date: &str,
) -> Result<Vec<PaymentTotal>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT method, SUM(amount_cents), COUNT(*)
         FROM payments
         WHERE DATE(created_at) = ?1
         GROUP BY method",
    )?;

    let rows = stmt.query_map(params![booking_date], |row| {
        Ok(PaymentTotal {
            method: row.get(0)?,
            amount_cents: row.get(1)?,
            count: row.get(2)?,
        })
    })?;

    rows.collect()
}

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

/// Check if at least one Z-report was generated today.
pub fn has_z_report_today(conn: &Connection) -> Result<bool, rusqlite::Error> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM z_reports WHERE report_date = date('now')",
        [],
        |row| row.get(0),
    )?;
    Ok(count > 0)
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

/// Get the first and last fiscal event datetimes for a booking date.
pub fn get_datetime_range(
    conn: &Connection,
    booking_date: &str,
) -> Result<(String, String), rusqlite::Error> {
    conn.query_row(
        "SELECT
            COALESCE(MIN(created_at), datetime('now')),
            COALESCE(MAX(created_at), datetime('now'))
         FROM fiscal_events
         WHERE DATE(created_at) = ?1",
        params![booking_date],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
}

/// Count drawer open events for a booking date.
pub fn count_drawer_opens(conn: &Connection, booking_date: &str) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT COUNT(*) FROM fiscal_events
         WHERE event_type = 'F' AND event_label = 'DRAWER_OPEN'
         AND DATE(created_at) = ?1",
        params![booking_date],
        |row| row.get(0),
    )
    .or(Ok(0))
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

    #[test]
    fn has_z_report_today_false_on_empty_db() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        assert!(!has_z_report_today(&conn).unwrap());
    }

    #[test]
    fn has_z_report_today_true_after_save() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        save_z_report(&conn, &today, 1, "{}", None).unwrap();
        assert!(has_z_report_today(&conn).unwrap());
    }

    #[test]
    fn has_z_report_today_false_for_yesterday() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        save_z_report(&conn, "2025-01-01", 1, "{}", None).unwrap();
        assert!(!has_z_report_today(&conn).unwrap());
    }
}
