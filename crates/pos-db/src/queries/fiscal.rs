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
pub fn get_fiscal_event(conn: &Connection, event_id: i64) -> Result<Option<FiscalEventRef>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, event_type, sequence_number, order_id,
                fdm_id, fdm_datetime, event_label,
                short_signature, qr_data, fdm_response_json
         FROM fiscal_events WHERE id = ?1"
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
            if let Some(fdm_ref) = event.fdm_response_json.as_deref()
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
pub fn list_recent_events(conn: &Connection, limit: i64) -> Result<Vec<FiscalEventRef>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, event_type, sequence_number, order_id,
                fdm_id, fdm_datetime, event_label,
                short_signature, qr_data, fdm_response_json
         FROM fiscal_events
         ORDER BY id DESC LIMIT ?1"
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
pub fn list_refundable_events(conn: &Connection, limit: i64) -> Result<Vec<RefundableEvent>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT fe.id, fe.sequence_number, fe.order_id,
                o.table_number, o.total_cents, fe.created_at
         FROM fiscal_events fe
         JOIN orders o ON o.id = fe.order_id
         WHERE fe.event_type = 'N'
           AND o.status = 'closed'
         ORDER BY fe.created_at DESC
         LIMIT ?1"
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
