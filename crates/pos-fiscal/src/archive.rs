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

    fn setup() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        pos_db::schema::run_migrations(&conn).unwrap();
        conn
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
    }
}
