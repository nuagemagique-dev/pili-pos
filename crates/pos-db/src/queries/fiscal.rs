use rusqlite::{Connection, params};

/// Minimal fiscal event info needed for copy operations.
#[derive(Debug, Clone)]
pub struct FiscalEventRef {
    pub id: i64,
    pub event_type: String,
    pub sequence_number: i64,
    pub order_id: Option<i64>,
    pub fdm_id: Option<String>,
    pub fdm_datetime: Option<String>,
    pub event_label: Option<String>,
    pub event_counter: Option<i64>,
    pub total_counter: Option<i64>,
    pub short_signature: Option<String>,
    pub qr_data: Option<String>,
    pub fdm_response_json: Option<String>,
}

/// Get a fiscal event by ID.
pub fn get_fiscal_event(
    conn: &Connection,
    event_id: i64,
) -> Result<Option<FiscalEventRef>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, event_type, sequence_number, order_id,
                fdm_id, fdm_datetime, event_label,
                short_signature, qr_data, fdm_response_json
         FROM fiscal_events WHERE id = ?1",
    )?;

    let mut rows = stmt.query_map(params![event_id], |row| {
        Ok(FiscalEventRef {
            id: row.get(0)?,
            event_type: row.get(1)?,
            sequence_number: row.get(2)?,
            order_id: row.get(3)?,
            fdm_id: row.get(4)?,
            fdm_datetime: row.get(5)?,
            event_label: row.get(6)?,
            event_counter: None,
            total_counter: None,
            short_signature: row.get(7)?,
            qr_data: row.get(8)?,
            fdm_response_json: row.get(9)?,
        })
    })?;

    match rows.next() {
        Some(Ok(mut event)) => {
            // Extract counters from fdm_response_json if available
            if let Some(fdm_ref) = event
                .fdm_response_json
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .and_then(|val| val.get("fdmRef").cloned())
            {
                event.event_counter = fdm_ref.get("eventCounter").and_then(|v| v.as_i64());
                event.total_counter = fdm_ref.get("totalCounter").and_then(|v| v.as_i64());
            }
            Ok(Some(event))
        }
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

/// List recent fiscal events (for copy selection).
pub fn list_recent_events(
    conn: &Connection,
    limit: i64,
) -> Result<Vec<FiscalEventRef>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, event_type, sequence_number, order_id,
                fdm_id, fdm_datetime, event_label,
                short_signature, qr_data, fdm_response_json
         FROM fiscal_events
         ORDER BY id DESC LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit], |row| {
        Ok(FiscalEventRef {
            id: row.get(0)?,
            event_type: row.get(1)?,
            sequence_number: row.get(2)?,
            order_id: row.get(3)?,
            fdm_id: row.get(4)?,
            fdm_datetime: row.get(5)?,
            event_label: row.get(6)?,
            event_counter: None,
            total_counter: None,
            short_signature: row.get(7)?,
            qr_data: row.get(8)?,
            fdm_response_json: row.get(9)?,
        })
    })?;

    rows.collect()
}

/// Recent closed N-events with order info for the refund picker.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundableEvent {
    pub fiscal_event_id: i64,
    pub ticket_number: i64,
    pub order_id: i64,
    pub table_number: Option<i32>,
    pub total_cents: i64,
    pub created_at: String,
}

/// List recent N-type events that can be refunded.
pub fn list_refundable_events(
    conn: &Connection,
    limit: i64,
) -> Result<Vec<RefundableEvent>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT fe.id, fe.sequence_number, fe.order_id,
                o.table_number, o.total_cents, fe.created_at
         FROM fiscal_events fe
         JOIN orders o ON o.id = fe.order_id
         WHERE fe.event_type = 'N'
           AND o.status = 'closed'
         ORDER BY fe.created_at DESC
         LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit], |row| {
        Ok(RefundableEvent {
            fiscal_event_id: row.get(0)?,
            ticket_number: row.get(1)?,
            order_id: row.get(2)?,
            table_number: row.get(3)?,
            total_cents: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;

    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::run_migrations;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn insert_event(
        conn: &Connection,
        seq: i64,
        event_type: &str,
        order_id: Option<i64>,
        response_json: Option<&str>,
    ) -> i64 {
        conn.execute(
            "INSERT INTO fiscal_events (
                event_type, sequence_number, order_id, pos_id, terminal_id,
                user_insz, event_data_json, fdm_response_json
             ) VALUES (?1, ?2, ?3, 'CFOD0061234567', 'POS1', '00000000029', '{}', ?4)",
            params![event_type, seq, order_id, response_json],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn get_fiscal_event_missing_returns_none() {
        let conn = setup();
        assert!(get_fiscal_event(&conn, 999).unwrap().is_none());
    }

    #[test]
    fn get_fiscal_event_extracts_counters_from_response() {
        let conn = setup();
        let id = insert_event(
            &conn,
            1,
            "N",
            None,
            Some(r#"{"fdmRef":{"eventCounter":7,"totalCounter":123}}"#),
        );
        let event = get_fiscal_event(&conn, id).unwrap().unwrap();
        assert_eq!(event.event_counter, Some(7));
        assert_eq!(event.total_counter, Some(123));
    }

    #[test]
    fn get_fiscal_event_without_response_has_no_counters() {
        let conn = setup();
        let id = insert_event(&conn, 1, "N", None, None);
        let event = get_fiscal_event(&conn, id).unwrap().unwrap();
        assert_eq!(event.event_counter, None);
        assert_eq!(event.total_counter, None);
    }

    #[test]
    fn get_fiscal_event_with_malformed_response_json_has_no_counters() {
        // A corrupt/partial fdm_response_json must not fail the whole
        // lookup — it just yields no counters.
        let conn = setup();
        let id = insert_event(&conn, 1, "N", None, Some("not json"));
        let event = get_fiscal_event(&conn, id).unwrap().unwrap();
        assert_eq!(event.event_counter, None);
        assert_eq!(event.total_counter, None);
    }

    #[test]
    fn list_recent_events_orders_newest_first_and_respects_limit() {
        let conn = setup();
        insert_event(&conn, 1, "N", None, None);
        insert_event(&conn, 2, "N", None, None);
        insert_event(&conn, 3, "P", None, None);

        let all = list_recent_events(&conn, 10).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].sequence_number, 3, "newest first");
        assert_eq!(all[2].sequence_number, 1);

        let limited = list_recent_events(&conn, 2).unwrap();
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].sequence_number, 3);
        assert_eq!(limited[1].sequence_number, 2);
    }

    fn seed_order(conn: &Connection, id: i64, table: Option<i32>, total_cents: i64) {
        conn.execute(
            "INSERT INTO orders (id, table_number, order_type, status, user_insz, total_cents)
             VALUES (?1, ?2, 'dine_in', 'closed', '00000000029', ?3)",
            params![id, table, total_cents],
        )
        .unwrap();
    }

    #[test]
    fn refundable_events_only_include_closed_n_events() {
        let conn = setup();
        seed_order(&conn, 1, Some(5), 2500);
        insert_event(&conn, 1, "N", Some(1), None);

        seed_order(&conn, 2, Some(6), 4200);
        // Training sale on order 2: NOT a refundable N event.
        insert_event(&conn, 2, "T", Some(2), None);

        seed_order(&conn, 3, Some(7), 1000);
        // Pro forma (P) is not refundable either.
        insert_event(&conn, 3, "P", Some(3), None);

        let refundable = list_refundable_events(&conn, 20).unwrap();
        assert_eq!(refundable.len(), 1);
        assert_eq!(refundable[0].order_id, 1);
        assert_eq!(refundable[0].total_cents, 2500);
        assert_eq!(refundable[0].table_number, Some(5));
    }

    #[test]
    fn refundable_events_exclude_open_orders() {
        // An N event can theoretically exist while the order is still
        // open in weird replay scenarios; the refund picker must only
        // offer genuinely closed sales.
        let conn = setup();
        conn.execute(
            "INSERT INTO orders (id, table_number, order_type, status, user_insz, total_cents)
             VALUES (1, 5, 'dine_in', 'open', '00000000029', 2500)",
            [],
        )
        .unwrap();
        insert_event(&conn, 1, "N", Some(1), None);

        assert!(list_refundable_events(&conn, 20).unwrap().is_empty());
    }

    #[test]
    fn refundable_events_respects_limit() {
        let conn = setup();
        for i in 1..=5 {
            seed_order(&conn, i, Some(i as i32), 1000 * i);
            insert_event(&conn, i, "N", Some(i), None);
        }
        assert_eq!(list_refundable_events(&conn, 3).unwrap().len(), 3);
        assert_eq!(list_refundable_events(&conn, 100).unwrap().len(), 5);
    }
}
