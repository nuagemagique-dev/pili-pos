//! Event dispatcher: routes HubRise callback events to domain handlers.
//!
//! Events are persisted in SQLite first (idempotence), then dispatched.

use pos_hubrise_types::CallbackEvent;
use rusqlite::{params, Connection};

use crate::catalog_apply::apply;
use crate::client::Client;
use crate::errors::Result;

/// Minimal hint type returned by the dispatcher so the caller (e.g.
/// the Tauri layer) can emit frontend events with typed payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchOutcome {
    OrderCreated,
    OrderUpdated,
    CatalogUpdated,
    Ignored,
}

/// Store the event in SQLite (idempotent) and return true if it was
/// a newly inserted row.
pub fn persist_event(conn: &Connection, event: &CallbackEvent, raw: &str) -> Result<bool> {
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO hubrise_events
            (id, resource_type, event_type, payload_json, received_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        params![
            event.id,
            event.resource_type,
            event.event_type,
            raw
        ],
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

/// Dispatch a single event. This is the hot path called from the
/// background poller: order events populate local orders, catalog
/// events fetch and apply a new snapshot.
pub async fn dispatch(
    client: &Client,
    conn: &mut Connection,
    event: &CallbackEvent,
) -> Result<DispatchOutcome> {
    if event.is_catalog_update() {
        let catalog_id = event
            .extra
            .get("catalog_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if let Some(catalog_id) = catalog_id {
            let path = format!("/catalogs/{}", catalog_id);
            let catalog: pos_hubrise_types::Catalog = client.get_json(&path).await?;
            apply(conn, &catalog)?;
            return Ok(DispatchOutcome::CatalogUpdated);
        }
        return Ok(DispatchOutcome::Ignored);
    }

    if event.is_order_create() {
        // Order ingestion will be refined in task #18 (UI integration).
        // For now we persist the raw event; the Tauri layer can query
        // the mock for the full order body via the client.
        return Ok(DispatchOutcome::OrderCreated);
    }

    if event.is_order_update() {
        return Ok(DispatchOutcome::OrderUpdated);
    }

    Ok(DispatchOutcome::Ignored)
}
