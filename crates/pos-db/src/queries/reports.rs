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

/// Count drawer open events for a booking date. The `event_label`
/// column stores the FDM event letter (`F`); DRAWER_OPEN vs
/// MONEY_IN_OUT is only distinguishable via `eventOperation` in the
/// archived FDM response.
pub fn count_drawer_opens(conn: &Connection, booking_date: &str) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT COUNT(*) FROM fiscal_events
         WHERE event_type = 'F'
           AND json_extract(fdm_response_json, '$.eventOperation') = 'DRAWER_OPEN'
           AND DATE(created_at) = ?1",
        params![booking_date],
        |row| row.get(0),
    )
}

// ── Per-user aggregates (user reports M182/M183/M184) ─────────

/// Per-user activity range + sales total for a booking date.
#[derive(Debug, Clone)]
pub struct UserActivity {
    pub user_insz: String,
    /// RFC 3339 `posDateTime` of the user's first fiscal event
    /// (falls back to `created_at` for pre-signature rows).
    pub first_datetime: String,
    pub last_datetime: String,
    /// Net sales total (N events) in eurocents.
    pub total_cents: i64,
}

/// Aggregate fiscal-event activity per operator for a booking date.
/// Every operator with at least one archived event that day appears,
/// even with zero sales (the M184 fixture shows a zero-total user).
pub fn get_user_activity(
    conn: &Connection,
    booking_date: &str,
) -> Result<Vec<UserActivity>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT user_insz,
                MIN(COALESCE(json_extract(event_data_json, '$.posDateTime'), created_at)),
                MAX(COALESCE(json_extract(event_data_json, '$.posDateTime'), created_at)),
                COALESCE(SUM(CASE WHEN event_type = 'N' THEN
                    CAST(json_extract(event_data_json, '$.transaction.transactionTotal') AS REAL) * 100
                ELSE 0 END), 0)
         FROM fiscal_events
         WHERE DATE(created_at) = ?1
         GROUP BY user_insz
         ORDER BY user_insz",
    )?;
    let rows = stmt.query_map(params![booking_date], |row| {
        Ok(UserActivity {
            user_insz: row.get(0)?,
            first_datetime: row.get(1)?,
            last_datetime: row.get(2)?,
            total_cents: row.get::<_, f64>(3)?.round() as i64,
        })
    })?;
    rows.collect()
}

/// Per-user payment totals for a booking date (attributed via the
/// order's operator).
#[derive(Debug, Clone)]
pub struct UserPaymentTotal {
    pub user_insz: String,
    pub method: String,
    pub amount_cents: i64,
    pub count: i64,
}

pub fn get_user_payment_totals(
    conn: &Connection,
    booking_date: &str,
) -> Result<Vec<UserPaymentTotal>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT o.user_insz, p.method, SUM(p.amount_cents), COUNT(*)
         FROM payments p
         JOIN orders o ON o.id = p.order_id
         WHERE DATE(p.created_at) = ?1
         GROUP BY o.user_insz, p.method
         ORDER BY o.user_insz, p.method",
    )?;
    let rows = stmt.query_map(params![booking_date], |row| {
        Ok(UserPaymentTotal {
            user_insz: row.get(0)?,
            method: row.get(1)?,
            amount_cents: row.get(2)?,
            count: row.get(3)?,
        })
    })?;
    rows.collect()
}

/// One work-in/work-out registration for the user reports.
#[derive(Debug, Clone)]
pub struct UserSocialEvent {
    pub user_insz: String,
    pub pos_datetime: String,
    /// `WORK_IN` or `WORK_OUT` (from the archived FDM response).
    pub operation: String,
}

pub fn get_user_social_events(
    conn: &Connection,
    booking_date: &str,
) -> Result<Vec<UserSocialEvent>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT user_insz,
                COALESCE(json_extract(event_data_json, '$.posDateTime'), created_at),
                json_extract(fdm_response_json, '$.eventOperation')
         FROM fiscal_events
         WHERE event_type = 'S'
           AND DATE(created_at) = ?1
           AND json_extract(fdm_response_json, '$.eventOperation')
               IN ('WORK_IN', 'WORK_OUT')
         ORDER BY id",
    )?;
    let rows = stmt.query_map(params![booking_date], |row| {
        Ok(UserSocialEvent {
            user_insz: row.get(0)?,
            pos_datetime: row.get(1)?,
            operation: row.get(2)?,
        })
    })?;
    rows.collect()
}

// ── FDM device range (fdmDevices block of X/Z reports) ────────

/// First/last FDM reference of a booking date, read from the
/// archived responses. `None` when no signed event exists that day.
#[derive(Debug, Clone)]
pub struct FdmRefRange {
    pub fdm_id: String,
    pub first_datetime: String,
    pub first_total_counter: i64,
    pub last_datetime: String,
    pub last_total_counter: i64,
}

pub fn get_fdm_ref_range(
    conn: &Connection,
    booking_date: &str,
) -> Result<Option<FdmRefRange>, rusqlite::Error> {
    let one = |order: &str| -> Result<Option<(String, String, i64)>, rusqlite::Error> {
        let mut stmt = conn.prepare(&format!(
            "SELECT json_extract(fdm_response_json, '$.fdmRef.fdmId'),
                    json_extract(fdm_response_json, '$.fdmRef.fdmDateTime'),
                    json_extract(fdm_response_json, '$.fdmRef.totalCounter')
             FROM fiscal_events
             WHERE fdm_response_json IS NOT NULL AND DATE(created_at) = ?1
             ORDER BY id {order} LIMIT 1"
        ))?;
        let mut rows = stmt.query_map(params![booking_date], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        rows.next().transpose()
    };
    let (Some(first), Some(last)) = (one("ASC")?, one("DESC")?) else {
        return Ok(None);
    };
    Ok(Some(FdmRefRange {
        fdm_id: first.0,
        first_datetime: first.1,
        first_total_counter: first.2,
        last_datetime: last.1,
        last_total_counter: last.2,
    }))
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

    /// Insert a minimal archived fiscal event for the aggregate tests.
    fn insert_event(
        conn: &Connection,
        seq: i64,
        event_type: &str,
        insz: &str,
        request_json: &str,
        response_json: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO fiscal_events (
                event_type, sequence_number, pos_id, terminal_id,
                user_insz, event_data_json, fdm_response_json
             ) VALUES (?1, ?2, 'CFOD0061234567', 'POS1', ?3, ?4, ?5)",
            params![event_type, seq, insz, request_json, response_json],
        )
        .unwrap();
    }

    fn today() -> String {
        chrono::Utc::now().format("%Y-%m-%d").to_string()
    }

    #[test]
    fn drawer_opens_counted_via_event_operation() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        insert_event(
            &conn,
            1,
            "F",
            "111",
            "{}",
            Some(r#"{"eventOperation":"DRAWER_OPEN"}"#),
        );
        insert_event(
            &conn,
            2,
            "F",
            "111",
            "{}",
            Some(r#"{"eventOperation":"MONEY_IN_OUT"}"#),
        );
        assert_eq!(count_drawer_opens(&conn, &today()).unwrap(), 1);
    }

    #[test]
    fn user_activity_aggregates_sales_and_range() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        insert_event(
            &conn,
            1,
            "S",
            "11111111111",
            r#"{"posDateTime":"2026-07-03T09:00:00+02:00"}"#,
            Some(r#"{"eventOperation":"WORK_IN"}"#),
        );
        insert_event(
            &conn,
            2,
            "N",
            "11111111111",
            r#"{"posDateTime":"2026-07-03T12:00:00+02:00","transaction":{"transactionTotal":25.5}}"#,
            None,
        );
        insert_event(
            &conn,
            3,
            "N",
            "22222222222",
            r#"{"posDateTime":"2026-07-03T13:00:00+02:00","transaction":{"transactionTotal":10.0}}"#,
            None,
        );

        let users = get_user_activity(&conn, &today()).unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].user_insz, "11111111111");
        assert_eq!(users[0].total_cents, 2550);
        assert_eq!(users[0].first_datetime, "2026-07-03T09:00:00+02:00");
        assert_eq!(users[0].last_datetime, "2026-07-03T12:00:00+02:00");
        assert_eq!(users[1].total_cents, 1000);
    }

    #[test]
    fn user_social_events_filtered_and_ordered() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        insert_event(
            &conn,
            1,
            "S",
            "11111111111",
            r#"{"posDateTime":"2026-07-03T09:00:00+02:00"}"#,
            Some(r#"{"eventOperation":"WORK_IN"}"#),
        );
        insert_event(
            &conn,
            2,
            "N",
            "11111111111",
            "{}",
            Some(r#"{"eventOperation":"SALE"}"#),
        );
        insert_event(
            &conn,
            3,
            "S",
            "11111111111",
            r#"{"posDateTime":"2026-07-03T17:00:00+02:00"}"#,
            Some(r#"{"eventOperation":"WORK_OUT"}"#),
        );

        let events = get_user_social_events(&conn, &today()).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].operation, "WORK_IN");
        assert_eq!(events[1].operation, "WORK_OUT");
        assert_eq!(events[1].pos_datetime, "2026-07-03T17:00:00+02:00");
    }

    #[test]
    fn user_payment_totals_grouped_by_operator() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO orders (id, order_type, status, user_insz) VALUES
             (1, 'dine_in', 'closed', '11111111111'),
             (2, 'dine_in', 'closed', '22222222222')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO payments (order_id, method, amount_cents) VALUES
             (1, 'cash', 1200), (1, 'card', 800), (2, 'cash', 500)",
            [],
        )
        .unwrap();

        let totals = get_user_payment_totals(&conn, &today()).unwrap();
        assert_eq!(totals.len(), 3);
        assert_eq!(totals[0].user_insz, "11111111111");
        assert_eq!(totals[0].method, "card");
        assert_eq!(totals[0].amount_cents, 800);
        assert_eq!(totals[2].user_insz, "22222222222");
        assert_eq!(totals[2].amount_cents, 500);
    }

    #[test]
    fn fdm_ref_range_reads_archived_responses() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        assert!(get_fdm_ref_range(&conn, &today()).unwrap().is_none());

        insert_event(
            &conn,
            1,
            "N",
            "111",
            "{}",
            Some(
                r#"{"fdmRef":{"fdmId":"CBX01000042","fdmDateTime":"2026-07-03T10:00:00Z","totalCounter":40}}"#,
            ),
        );
        insert_event(&conn, 2, "N", "111", "{}", None); // unsigned row ignored
        insert_event(
            &conn,
            3,
            "N",
            "111",
            "{}",
            Some(
                r#"{"fdmRef":{"fdmId":"CBX01000042","fdmDateTime":"2026-07-03T18:00:00Z","totalCounter":57}}"#,
            ),
        );

        let range = get_fdm_ref_range(&conn, &today()).unwrap().unwrap();
        assert_eq!(range.fdm_id, "CBX01000042");
        assert_eq!(range.first_total_counter, 40);
        assert_eq!(range.last_total_counter, 57);
        assert_eq!(range.first_datetime, "2026-07-03T10:00:00Z");
        assert_eq!(range.last_datetime, "2026-07-03T18:00:00Z");
    }
}
