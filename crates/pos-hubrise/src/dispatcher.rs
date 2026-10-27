//! Stored `HubRise` events: persisted in SQLite before they are handled
//! (idempotence), marked once handled. The routing is in `poller`.

use pos_hubrise_types::CallbackEvent;
use rusqlite::{Connection, params};

use crate::errors::Result;

/// Minimal hint type returned by the dispatcher so the caller (e.g.
/// the Tauri layer) can emit frontend events with typed payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchOutcome {
    OrderCreated,
    OrderUpdated,
    CatalogUpdated,
    /// The catalog has issues; the menu stayed as it was.
    CatalogRejected,
    /// First sync over a hand-entered menu: waiting for a manager.
    CatalogAwaitingAdoption,
    Ignored,
}

/// Store the event in SQLite (idempotent) and return true if it was
/// a newly inserted row.
pub fn persist_event(conn: &Connection, event: &CallbackEvent, raw: &str) -> Result<bool> {
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO hubrise_events
            (id, resource_type, event_type, payload_json, received_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        params![event.id, event.resource_type, event.event_type, raw],
    )?;
    Ok(inserted > 0)
}

/// Mark an event as successfully processed.
pub fn mark_processed(conn: &Connection, event_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE hubrise_events SET processed_at = datetime('now') WHERE id = ?1",
        params![event_id],
    )?;
    Ok(())
}

/// Mark an event as failed with an error message.
pub fn mark_failed(conn: &Connection, event_id: &str, err: &str) -> Result<()> {
    conn.execute(
        "UPDATE hubrise_events SET error_msg = ?1 WHERE id = ?2",
        params![err, event_id],
    )?;
    Ok(())
}
