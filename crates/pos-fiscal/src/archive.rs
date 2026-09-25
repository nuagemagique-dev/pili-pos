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

/// Archive a fiscal event and FDM response into the fiscal_events table.
/// MUST be called within a database transaction for crash safety.
pub fn archive_event(
    conn: &rusqlite::Connection,
    params: ArchiveParams<'_>,
) -> Result<i64, rusqlite::Error> {
    let response_json = params
        .sign_result
        .map(|r| serde_json::to_string(r).unwrap_or_default());

    let fdm_id = params.sign_result.map(|r| r.fdm_ref.fdm_id.clone());
    let fdm_datetime = params.sign_result.map(|r| r.fdm_ref.fdm_date_time.clone());
    let event_label_str = params.sign_result.map(|r| {
        serde_json::to_string(&r.fdm_ref.event_label)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string()
    });
    let short_sig = params.sign_result.and_then(|r| r.short_signature.clone());
    let qr_data = params.sign_result.and_then(|r| r.verification_url.clone());

    let event_type_str = serde_json::to_string(&params.event_type)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

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
        // `fiscal_events` has UNIQUE(event_type, sequence_number) —
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
    // CLAUDE.md: "No deletion — fiscal records can NEVER be deleted
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
        // The row is still there — the trigger really blocked it.
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
}
