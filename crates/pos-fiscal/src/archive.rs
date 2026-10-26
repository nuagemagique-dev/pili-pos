use crate::response::SignResult;
use crate::types::EventLabel;

/// Archived fiscal event stored in SQLite.
/// This record is APPEND-ONLY and must NEVER be deleted or modified.
#[derive(Debug, Clone)]
pub struct ArchivedEvent {
    pub id: i64,
    pub event_type: EventLabel,
    pub sequence_number: i64,
    pub order_id: Option<i64>,
    pub pos_id: String,
    pub terminal_id: String,
    pub user_insz: String,
    pub event_data_json: String,
    pub fdm_response_json: Option<String>,
    pub fdm_id: Option<String>,
    pub fdm_datetime: Option<String>,
    pub event_label: Option<String>,
    pub short_signature: Option<String>,
    pub qr_data: Option<String>,
}

/// Parameters for archiving a fiscal event.
#[derive(Debug, Clone, Copy)]
pub struct ArchiveParams<'a> {
    pub event_type: EventLabel,
    pub sequence_number: i64,
    pub order_id: Option<i64>,
    pub pos_id: &'a str,
    pub terminal_id: &'a str,
    pub user_insz: &'a str,
    pub request_json: &'a str,
    pub sign_result: Option<&'a SignResult>,
}

/// Archive a fiscal event and FDM response into the `fiscal_events` table.
/// MUST be called within a database transaction for crash safety.
///
/// `fdm_response_json` holds the FDM's answer as the parsed
/// [`SignResult`] serialized again, not the bytes the FDM sent. This is
/// deliberate (decision of the Rust review, item 3): the bytes are not
/// available here (the HTTP body is parsed into a JSON value, which
/// already normalises number formatting and key order), every field the
/// POS spec v2.2 defines is a field of `SignResult` and is checked
/// against the request by `response_check` before archiving, and the
/// stored form is deterministic, which the SPF dossier, the goldens, the
/// accounting export and the SQL readers (`json_extract` on `$.fdmRef`)
/// depend on. Keeping a raw copy would add a second form to every reader
/// and change archived bytes for no field the till or the report uses;
/// an FDM field outside the specification is dropped and logged by
/// nothing, which is acceptable because the FDM itself keeps its journal.
pub fn archive_event(
    conn: &rusqlite::Connection,
    params: ArchiveParams<'_>,
) -> Result<i64, rusqlite::Error> {
    // A failed serialisation must stop the archive: storing "" would
    // lose the signed response (the transaction rolls back and the
    // caller keeps the pending signature for a retry).
    let response_json = params
        .sign_result
        .map(|r| serde_json::to_string(r).map_err(to_sql_error))
        .transpose()?;

    let fdm_id = params.sign_result.map(|r| r.fdm_ref.fdm_id.clone());
    let fdm_datetime = params.sign_result.map(|r| r.fdm_ref.fdm_date_time.clone());
    let event_label_str = params
        .sign_result
        .map(|r| enum_text(&r.fdm_ref.event_label))
        .transpose()?;
    let short_sig = params.sign_result.and_then(|r| r.short_signature.clone());
    let qr_data = params.sign_result.and_then(|r| r.verification_url.clone());

    let event_type_str = enum_text(&params.event_type)?;

    if let Some(result) = params.sign_result {
        warn_on_counter_regression(conn, result);
    }

    conn.execute(
        "INSERT INTO fiscal_events (
            event_type, sequence_number, order_id, pos_id, terminal_id,
            user_insz, event_data_json, fdm_response_json, fdm_id,
            fdm_datetime, event_label, short_signature, qr_data
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            event_type_str,
            params.sequence_number,
            params.order_id,
            params.pos_id,
            params.terminal_id,
            params.user_insz,
            params.request_json,
            response_json,
            fdm_id,
            fdm_datetime,
            event_label_str,
            short_sig,
            qr_data,
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

fn to_sql_error(e: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(e))
}

/// The wire text of a unit enum (`"N"` becomes `N`).
fn enum_text<T: serde::Serialize>(value: &T) -> Result<String, rusqlite::Error> {
    let text = serde_json::to_string(value).map_err(to_sql_error)?;
    Ok(text.trim_matches('"').to_string())
}

/// The FDM's counters only go up. A signed event whose `totalCounter`
/// (or per-label `eventCounter`) is not above the last one archived
/// from the same FDM points at a replayed or foreign response. The
/// event is already signed at this point, so it is still archived (a
/// refusal would lose a record the FDM keeps anyway), but the
/// technician finds it in the log.
fn warn_on_counter_regression(conn: &rusqlite::Connection, result: &SignResult) {
    for regression in counter_regressions(conn, result) {
        tracing::warn!(
            fdm_id = %result.fdm_ref.fdm_id,
            counter = regression.counter,
            received = regression.received,
            last_archived = regression.last_archived,
            "FDM counter did not increase"
        );
    }
}

/// A counter of a new FDM response that is not above the last one
/// archived from the same FDM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterRegression {
    pub counter: &'static str,
    pub received: i64,
    pub last_archived: i64,
}

pub fn counter_regressions(
    conn: &rusqlite::Connection,
    result: &SignResult,
) -> Vec<CounterRegression> {
    let fdm = &result.fdm_ref;
    let label = serde_json::to_string(&fdm.event_label).unwrap_or_default();
    let label = label.trim_matches('"');
    let last = conn.query_row(
        "SELECT
            MAX(json_extract(fdm_response_json, '$.fdmRef.totalCounter')),
            MAX(CASE WHEN event_label = ?2
                THEN json_extract(fdm_response_json, '$.fdmRef.eventCounter') END)
         FROM fiscal_events WHERE fdm_id = ?1",
        rusqlite::params![fdm.fdm_id, label],
        |row| Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, Option<i64>>(1)?)),
    );
    let Ok((last_total, last_event)) = last else {
        return vec![];
    };
    let mut regressions = vec![];
    if let Some(last) = last_total.filter(|&t| fdm.total_counter <= t) {
        regressions.push(CounterRegression {
            counter: "totalCounter",
            received: fdm.total_counter,
            last_archived: last,
        });
    }
    if let Some(last) = last_event.filter(|&c| fdm.event_counter <= c) {
        regressions.push(CounterRegression {
            counter: "eventCounter",
            received: fdm.event_counter,
            last_archived: last,
        });
    }
    regressions
}

/// Every event archived in the booking period `booking_period_id`
/// (the `bookingPeriodId` of its request), in archive order. The input
/// of the turnover and user reports.
pub fn events_in_booking_period(
    conn: &rusqlite::Connection,
    booking_period_id: &str,
) -> Result<Vec<ArchivedEvent>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, event_type, sequence_number, order_id, pos_id, terminal_id,
                user_insz, event_data_json, fdm_response_json, fdm_id,
                fdm_datetime, event_label, short_signature, qr_data
         FROM fiscal_events
         WHERE json_extract(event_data_json, '$.bookingPeriodId') = ?1
         ORDER BY id",
    )?;
    let rows = stmt.query_map(rusqlite::params![booking_period_id], |row| {
        let label_str: String = row.get(1)?;
        let event_type =
            serde_json::from_str::<EventLabel>(&format!("\"{label_str}\"")).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, e.into())
            })?;
        Ok(ArchivedEvent {
            id: row.get(0)?,
            event_type,
            sequence_number: row.get(2)?,
            order_id: row.get(3)?,
            pos_id: row.get(4)?,
            terminal_id: row.get(5)?,
            user_insz: row.get(6)?,
            event_data_json: row.get(7)?,
            fdm_response_json: row.get(8)?,
            fdm_id: row.get(9)?,
            fdm_datetime: row.get(10)?,
            event_label: row.get(11)?,
            short_signature: row.get(12)?,
            qr_data: row.get(13)?,
        })
    })?;
    rows.collect()
}

/// The events signed for `order_id` before event `before_event_id`,
/// whatever their booking period: the order registrations a sale
/// follows (a table may close after a Z).
pub fn order_events_before(
    conn: &rusqlite::Connection,
    order_id: i64,
    before_event_id: i64,
) -> Result<Vec<ArchivedEvent>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, event_type, sequence_number, order_id, pos_id, terminal_id,
                user_insz, event_data_json, fdm_response_json, fdm_id,
                fdm_datetime, event_label, short_signature, qr_data
         FROM fiscal_events
         WHERE order_id = ?1 AND id < ?2
         ORDER BY id",
    )?;
    let rows = stmt.query_map(rusqlite::params![order_id, before_event_id], |row| {
        let label_str: String = row.get(1)?;
        let event_type =
            serde_json::from_str::<EventLabel>(&format!("\"{label_str}\"")).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, e.into())
            })?;
        Ok(ArchivedEvent {
            id: row.get(0)?,
            event_type,
            sequence_number: row.get(2)?,
            order_id: row.get(3)?,
            pos_id: row.get(4)?,
            terminal_id: row.get(5)?,
            user_insz: row.get(6)?,
            event_data_json: row.get(7)?,
            fdm_response_json: row.get(8)?,
            fdm_id: row.get(9)?,
            fdm_datetime: row.get(10)?,
            event_label: row.get(11)?,
            short_signature: row.get(12)?,
            qr_data: row.get(13)?,
        })
    })?;
    rows.collect()
}

/// `transactionTotal` of the archived event an FDM reference names.
pub fn transaction_total_by_fdm_ref(
    conn: &rusqlite::Connection,
    fdm_ref: &serde_json::Value,
) -> Result<Option<f64>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT json_extract(event_data_json, '$.transaction.transactionTotal')
         FROM fiscal_events
         WHERE json_extract(fdm_response_json, '$.fdmRef.fdmId') = ?1
           AND json_extract(fdm_response_json, '$.fdmRef.fdmDateTime') = ?2
           AND json_extract(fdm_response_json, '$.fdmRef.eventLabel') = ?3
           AND json_extract(fdm_response_json, '$.fdmRef.eventCounter') = ?4
           AND json_extract(fdm_response_json, '$.fdmRef.totalCounter') = ?5",
    )?;
    let mut rows = stmt.query(rusqlite::params![
        fdm_ref["fdmId"].as_str(),
        fdm_ref["fdmDateTime"].as_str(),
        fdm_ref["eventLabel"].as_str(),
        fdm_ref["eventCounter"].as_i64(),
        fdm_ref["totalCounter"].as_i64(),
    ])?;
    match rows.next()? {
        Some(row) => row.get(0),
        None => Ok(None),
    }
}

/// Retrieve an archived event by its sequence number and event type.
pub fn get_archived_event(
    conn: &rusqlite::Connection,
    event_type: EventLabel,
    sequence_number: i64,
) -> Result<Option<ArchivedEvent>, rusqlite::Error> {
    let event_type_str = serde_json::to_string(&event_type)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    let mut stmt = conn.prepare(
        "SELECT id, event_type, sequence_number, order_id, pos_id, terminal_id,
                user_insz, event_data_json, fdm_response_json, fdm_id,
                fdm_datetime, event_label, short_signature, qr_data
         FROM fiscal_events
         WHERE event_type = ?1 AND sequence_number = ?2",
    )?;

    let mut rows = stmt.query_map(rusqlite::params![event_type_str, sequence_number], |row| {
        let label_str: String = row.get(1)?;
        let event_label = serde_json::from_str::<EventLabel>(&format!("\"{label_str}\""))
            .unwrap_or(EventLabel::N);

        Ok(ArchivedEvent {
            id: row.get(0)?,
            event_type: event_label,
            sequence_number: row.get(2)?,
            order_id: row.get(3)?,
            pos_id: row.get(4)?,
            terminal_id: row.get(5)?,
            user_insz: row.get(6)?,
            event_data_json: row.get(7)?,
            fdm_response_json: row.get(8)?,
            fdm_id: row.get(9)?,
            fdm_datetime: row.get(10)?,
            event_label: row.get(11)?,
            short_signature: row.get(12)?,
            qr_data: row.get(13)?,
        })
    })?;

    match rows.next() {
        Some(Ok(event)) => Ok(Some(event)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::FdmReference;

    #[test]
    fn enum_text_is_the_bare_wire_text() {
        assert_eq!(enum_text(&EventLabel::N).unwrap(), "N");
        assert_eq!(enum_text(&EventLabel::T).unwrap(), "T");
    }

    /// A value `serde_json` cannot write (a map with non-string keys)
    /// is an error, never an empty string in the archive.
    #[test]
    fn an_unserialisable_value_is_an_error_not_an_empty_string() {
        let mut bad = std::collections::BTreeMap::new();
        bad.insert((1, 2), 3);
        assert!(enum_text(&bad).is_err());
    }

    fn setup() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        pos_db::schema::run_migrations(&conn).unwrap();
        conn
    }

    fn sign_result() -> SignResult {
        SignResult {
            pos_id: "CFOD0061234567".to_string(),
            pos_fiscal_ticket_no: 1,
            pos_date_time: "2026-04-12T19:30:00+02:00".to_string(),
            terminal_id: Some("POS1".to_string()),
            device_id: "DEV001".to_string(),
            event_operation: crate::types::EventOperation::Sale,
            fdm_ref: FdmReference {
                fdm_id: "CBX01000042".to_string(),
                fdm_date_time: "2026-04-12T19:30:01+02:00".to_string(),
                event_label: EventLabel::N,
                event_counter: 1,
                total_counter: 1,
            },
            fdm_sw_version: "1.0".to_string(),
            digital_signature: "AAAA".to_string(),
            short_signature: Some("abc123".to_string()),
            verification_url: Some(
                "https://www.gs2.be/12345678901234567890123456789012345678".to_string(),
            ),
            vat_calc: None,
            buffer_capacity_used: 0.0,
            warnings: None,
            informations: None,
            footer: vec![],
        }
    }

    #[test]
    fn archive_and_retrieve() {
        let conn = setup();
        let id = archive_event(
            &conn,
            ArchiveParams {
                event_type: EventLabel::N,
                sequence_number: 1,
                order_id: None,
                pos_id: "CFOD0061234567",
                terminal_id: "POS1",
                user_insz: "00000000029",
                request_json: r#"{"test": true}"#,
                sign_result: None,
            },
        )
        .unwrap();

        assert!(id > 0);

        let event = get_archived_event(&conn, EventLabel::N, 1)
            .unwrap()
            .unwrap();
        assert_eq!(event.pos_id, "CFOD0061234567");
        assert_eq!(event.order_id, None);
        assert_eq!(event.event_data_json, r#"{"test": true}"#);
        // No sign_result yet: FDM control-data columns stay unset.
        assert!(event.fdm_id.is_none());
        assert!(event.fdm_response_json.is_none());
        assert!(event.qr_data.is_none());
    }

    #[test]
    fn a_sale_total_is_found_by_its_fdm_reference() {
        let conn = setup();
        let sr = sign_result();
        archive_event(
            &conn,
            ArchiveParams {
                event_type: EventLabel::N,
                sequence_number: 1,
                order_id: None,
                pos_id: "CFOD0061234567",
                terminal_id: "POS1",
                user_insz: "00000000029",
                request_json: r#"{"transaction": {"transactionTotal": 27.5}}"#,
                sign_result: Some(&sr),
            },
        )
        .unwrap();
        let fdm_ref = serde_json::to_value(&sr.fdm_ref).unwrap();
        assert_eq!(
            transaction_total_by_fdm_ref(&conn, &fdm_ref).unwrap(),
            Some(27.5)
        );
        let mut other = fdm_ref;
        other["eventCounter"] = serde_json::json!(2);
        assert_eq!(transaction_total_by_fdm_ref(&conn, &other).unwrap(), None);
    }

    #[test]
    fn archive_with_sign_result_populates_fdm_columns() {
        let conn = setup();
        let sr = sign_result();
        conn.execute(
            "INSERT INTO orders (id, order_type, status, user_insz) VALUES (42, 'dine_in', 'closed', '00000000029')",
            [],
        )
        .unwrap();
        let id = archive_event(
            &conn,
            ArchiveParams {
                event_type: EventLabel::N,
                sequence_number: 1,
                order_id: Some(42),
                pos_id: "CFOD0061234567",
                terminal_id: "POS1",
                user_insz: "00000000029",
                request_json: r#"{"test": true}"#,
                sign_result: Some(&sr),
            },
        )
        .unwrap();
        assert!(id > 0);

        let event = get_archived_event(&conn, EventLabel::N, 1)
            .unwrap()
            .unwrap();
        assert_eq!(event.order_id, Some(42));
        assert_eq!(event.fdm_id.as_deref(), Some("CBX01000042"));
        assert_eq!(
            event.fdm_datetime.as_deref(),
            Some("2026-04-12T19:30:01+02:00")
        );
        assert_eq!(event.event_label.as_deref(), Some("N"));
        assert_eq!(event.short_signature.as_deref(), Some("abc123"));
        assert_eq!(
            event.qr_data.as_deref(),
            Some("https://www.gs2.be/12345678901234567890123456789012345678")
        );
        assert!(event.fdm_response_json.is_some());
    }

    #[test]
    fn get_archived_event_missing_returns_none() {
        let conn = setup();
        assert!(
            get_archived_event(&conn, EventLabel::N, 999)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn duplicate_sequence_number_for_same_label_is_rejected() {
        // `fiscal_events` has UNIQUE(event_type, sequence_number) -
        // the DB itself is the last line of defense against a gap or
        // a duplicate ticket number ever being archived twice.
        let conn = setup();
        archive_event(
            &conn,
            ArchiveParams {
                event_type: EventLabel::N,
                sequence_number: 1,
                order_id: None,
                pos_id: "CFOD0061234567",
                terminal_id: "POS1",
                user_insz: "00000000029",
                request_json: "{}",
                sign_result: None,
            },
        )
        .unwrap();

        let dup = archive_event(
            &conn,
            ArchiveParams {
                event_type: EventLabel::N,
                sequence_number: 1,
                order_id: None,
                pos_id: "CFOD0061234567",
                terminal_id: "POS1",
                user_insz: "00000000029",
                request_json: r#"{"different": true}"#,
                sign_result: None,
            },
        );
        assert!(dup.is_err());
    }

    #[test]
    fn same_sequence_number_is_fine_across_different_labels() {
        // Per-event-type counters (POS description v2.2): N #1 and P
        // #1 coexisting on the same day is expected, not a collision.
        let conn = setup();
        archive_event(
            &conn,
            ArchiveParams {
                event_type: EventLabel::N,
                sequence_number: 1,
                order_id: None,
                pos_id: "CFOD0061234567",
                terminal_id: "POS1",
                user_insz: "00000000029",
                request_json: "{}",
                sign_result: None,
            },
        )
        .unwrap();
        let ok = archive_event(
            &conn,
            ArchiveParams {
                event_type: EventLabel::P,
                sequence_number: 1,
                order_id: None,
                pos_id: "CFOD0061234567",
                terminal_id: "POS1",
                user_insz: "00000000029",
                request_json: "{}",
                sign_result: None,
            },
        );
        assert!(ok.is_ok());
    }

    // ── Append-only enforcement (migration 3) ───────────────────
    //
    // CLAUDE.md: "No deletion - fiscal records can NEVER be deleted
    // or modified." Once archived, a fiscal_events row must be
    // impossible to touch again, at the DB level, not just by
    // convention.

    #[test]
    fn archived_event_cannot_be_deleted() {
        let conn = setup();
        archive_event(
            &conn,
            ArchiveParams {
                event_type: EventLabel::N,
                sequence_number: 1,
                order_id: None,
                pos_id: "CFOD0061234567",
                terminal_id: "POS1",
                user_insz: "00000000029",
                request_json: "{}",
                sign_result: None,
            },
        )
        .unwrap();

        let err = conn
            .execute("DELETE FROM fiscal_events WHERE sequence_number = 1", [])
            .unwrap_err();
        assert!(format!("{err}").contains("append-only"));
        // The row is still there - the trigger really blocked it.
        assert!(
            get_archived_event(&conn, EventLabel::N, 1)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn archived_event_cannot_be_rewritten() {
        let conn = setup();
        archive_event(
            &conn,
            ArchiveParams {
                event_type: EventLabel::N,
                sequence_number: 1,
                order_id: None,
                pos_id: "CFOD0061234567",
                terminal_id: "POS1",
                user_insz: "00000000029",
                request_json: r#"{"original": true}"#,
                sign_result: None,
            },
        )
        .unwrap();

        assert!(
            conn.execute(
                "UPDATE fiscal_events SET event_data_json = '{}' WHERE sequence_number = 1",
                []
            )
            .is_err()
        );
        assert!(
            conn.execute(
                "UPDATE fiscal_events SET sequence_number = 2 WHERE sequence_number = 1",
                []
            )
            .is_err()
        );

        let event = get_archived_event(&conn, EventLabel::N, 1)
            .unwrap()
            .unwrap();
        assert_eq!(event.event_data_json, r#"{"original": true}"#);
    }

    #[test]
    fn z_reports_are_append_only_too() {
        let conn = setup();
        pos_db::queries::reports::save_z_report(&conn, "2026-04-12", 1, "{}", None).unwrap();

        assert!(conn.execute("DELETE FROM z_reports", []).is_err());
        assert!(
            conn.execute("UPDATE z_reports SET data_json = '{}'", [])
                .is_err()
        );
    }

    fn archive_signed(conn: &rusqlite::Connection, seq: i64, sr: &SignResult) {
        archive_event(
            conn,
            ArchiveParams {
                event_type: sr.fdm_ref.event_label,
                sequence_number: seq,
                order_id: None,
                pos_id: "CFOD0061234567",
                terminal_id: "POS1",
                user_insz: "12345678901",
                request_json: "{}",
                sign_result: Some(sr),
            },
        )
        .unwrap();
    }

    fn with_counters(label: EventLabel, event: i64, total: i64) -> SignResult {
        let mut sr = sign_result();
        sr.fdm_ref.event_label = label;
        sr.fdm_ref.event_counter = event;
        sr.fdm_ref.total_counter = total;
        sr
    }

    #[test]
    fn increasing_counters_are_not_a_regression() {
        let conn = setup();
        archive_signed(&conn, 1, &with_counters(EventLabel::N, 1, 1));
        archive_signed(&conn, 1, &with_counters(EventLabel::P, 1, 2));
        assert_eq!(
            counter_regressions(&conn, &with_counters(EventLabel::N, 2, 3)),
            Vec::new()
        );
    }

    #[test]
    fn a_total_counter_going_back_is_reported() {
        let conn = setup();
        archive_signed(&conn, 1, &with_counters(EventLabel::N, 1, 5));
        archive_signed(&conn, 1, &with_counters(EventLabel::P, 1, 6));
        let found = counter_regressions(&conn, &with_counters(EventLabel::N, 2, 6));
        assert_eq!(
            found,
            vec![CounterRegression {
                counter: "totalCounter",
                received: 6,
                last_archived: 6,
            }]
        );
    }

    #[test]
    fn an_event_counter_going_back_is_reported_per_label() {
        let conn = setup();
        archive_signed(&conn, 1, &with_counters(EventLabel::N, 4, 4));
        // P has its own counter: 1 after N's 4 is fine.
        assert_eq!(
            counter_regressions(&conn, &with_counters(EventLabel::P, 1, 5)),
            Vec::new()
        );
        let found = counter_regressions(&conn, &with_counters(EventLabel::N, 3, 5));
        assert_eq!(found[0].counter, "eventCounter");
        assert_eq!((found[0].received, found[0].last_archived), (3, 4));
    }

    #[test]
    fn another_fdm_starts_its_own_counters() {
        let conn = setup();
        archive_signed(&conn, 1, &with_counters(EventLabel::N, 9, 9));
        let mut replacement = with_counters(EventLabel::N, 1, 1);
        replacement.fdm_ref.fdm_id = "CBX01000099".to_string();
        assert_eq!(counter_regressions(&conn, &replacement), Vec::new());
    }

    #[test]
    fn a_counter_regression_is_logged_but_still_archived() {
        let conn = setup();
        archive_signed(&conn, 1, &with_counters(EventLabel::N, 2, 2));
        archive_signed(&conn, 2, &with_counters(EventLabel::N, 1, 1));
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM fiscal_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2, "the signed event is never lost");
    }
}
