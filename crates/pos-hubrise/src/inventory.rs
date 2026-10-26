//! Push of the sold-out products to the HubRise inventory (V11-D15).
//!
//! The engine queues a row of `hubrise_inventory_sync` (one per product, marked dirty again at
//! every change) when a cashier sets or clears "Épuisé", only while HubRise is on and the
//! product has a SKU ref. This pass, called by the outbox loop, sends the CURRENT state of each
//! dirty product at send time, so several changes made while offline end as one request:
//!
//! - sold out for the business day: `{"sku_ref", "stock": "0", "expires_at": <end of day>}`;
//! - sold out until re-enabled: `{"sku_ref", "stock": "0"}`;
//! - available (cleared, or the end of day is past): `{"sku_ref", "stock": null}`.
//!
//! The request is `PATCH /catalogs/{catalog_id}/location/inventory` with the location token.
//! Same error policy as the order outbox: a timeout, 429 or 5xx is retried forever with backoff
//! (30 s doubling, capped at 30 min), a refusal (400, 403, 404, 422) parks the row with its
//! reason, a 401 stops the pass and marks the connection's authentication error.

use std::sync::{Arc, Mutex};

use pos_hubrise_types::inventory::InventoryEntry;
use rusqlite::{Connection, OptionalExtension, params};

use crate::client::Client;
use crate::errors::{HubriseError, Result};
use crate::outbox::PassReport;
use crate::token_store;

/// Base delay between retries (doubles each attempt).
const BASE_BACKOFF_SECS: i64 = 30;
/// Longest delay between two retries.
const MAX_BACKOFF_SECS: i64 = 30 * 60;
/// Max rows processed per pass.
const BATCH_SIZE: i64 = 50;

/// What the inventory says about a product.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stock {
    /// Sold out; `expires_at` is the end of the business day, `None` until re-enabled.
    SoldOut { expires_at: Option<String> },
    /// Available: the entry is removed from the inventory.
    Unlimited,
}

/// The inventory entry for `sku_ref` in `stock`.
pub fn entry_for(sku_ref: &str, stock: &Stock) -> InventoryEntry {
    match stock {
        Stock::SoldOut { expires_at } => InventoryEntry {
            sku_ref: Some(sku_ref.to_string()),
            option_ref: None,
            stock: Some("0".to_string()),
            expires_at: expires_at.clone(),
        },
        Stock::Unlimited => InventoryEntry {
            sku_ref: Some(sku_ref.to_string()),
            option_ref: None,
            stock: None,
            expires_at: None,
        },
    }
}

/// The product's current state, read when the request is built.
fn current_stock(conn: &Connection, menu_item_id: i64) -> Result<Stock> {
    let row: Option<Option<String>> = conn
        .query_row(
            "SELECT until FROM menu_item_sold_out
             WHERE menu_item_id = ?1
               AND (until IS NULL OR julianday(until) > julianday('now'))",
            params![menu_item_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(match row {
        Some(expires_at) => Stock::SoldOut { expires_at },
        None => Stock::Unlimited,
    })
}

struct Row {
    menu_item_id: i64,
    sku_ref: String,
    version: i64,
    attempts: i64,
}

enum Outcome {
    Sent,
    Retry(String),
    Park(String),
    Unauthorized,
}

fn classify(e: HubriseError) -> Outcome {
    match e {
        HubriseError::Unauthorized => Outcome::Unauthorized,
        HubriseError::Api { status, body } if matches!(status, 400 | 403 | 404 | 422) => {
            Outcome::Park(format!("HubRise {status}: {body}"))
        }
        other => Outcome::Retry(other.to_string()),
    }
}

/// Rows waiting for the push and rows parked on an error.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Backlog {
    pub pending: i64,
    pub parked: i64,
}

pub fn backlog(conn: &Connection) -> Result<Backlog> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(dirty = 1 AND parked_at IS NULL), 0),
                COALESCE(SUM(dirty = 1 AND parked_at IS NOT NULL), 0)
         FROM hubrise_inventory_sync",
        [],
        |r| {
            Ok(Backlog {
                pending: r.get(0)?,
                parked: r.get(1)?,
            })
        },
    )?)
}

/// Put parked rows back in the queue (an operator's "retry").
pub fn retry_parked(conn: &Connection) -> Result<usize> {
    Ok(conn.execute(
        "UPDATE hubrise_inventory_sync
         SET parked_at = NULL, attempts = 0, next_attempt_at = NULL
         WHERE dirty = 1 AND parked_at IS NOT NULL",
        [],
    )?)
}

/// Send what is due. One request per product: a refusal of one SKU parks that row only.
pub async fn process_pending(
    http: &reqwest::Client,
    db: &Arc<Mutex<Connection>>,
) -> Result<PassReport> {
    let mut report = PassReport::default();
    let (stored, rows) = {
        let db = crate::util::lock_db(db);
        let Some(stored) = token_store::load(&db)? else {
            return Ok(report);
        };
        let mut stmt = db.prepare(
            "SELECT menu_item_id, sku_ref, version, attempts FROM hubrise_inventory_sync
             WHERE dirty = 1 AND parked_at IS NULL
               AND (next_attempt_at IS NULL OR next_attempt_at <= datetime('now'))
             ORDER BY menu_item_id
             LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![BATCH_SIZE], |r| {
                Ok(Row {
                    menu_item_id: r.get(0)?,
                    sku_ref: r.get(1)?,
                    version: r.get(2)?,
                    attempts: r.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        (stored, rows)
    };
    let client = Client::new(http.clone(), &stored.base_url, &stored.access_token);
    let path = format!("/catalogs/{}/location/inventory", stored.catalog_id);

    for row in rows {
        // The current state, at send time.
        let stock = {
            let db = crate::util::lock_db(db);
            current_stock(&db, row.menu_item_id)?
        };
        let body = vec![entry_for(&row.sku_ref, &stock)];
        let outcome = match client
            .patch_json::<_, serde_json::Value>(&path, &body)
            .await
        {
            Ok(_) => Outcome::Sent,
            Err(e) => classify(e),
        };
        let db = crate::util::lock_db(db);
        match outcome {
            Outcome::Sent => {
                // A change made during the send bumped `version`: the row stays dirty.
                db.execute(
                    "UPDATE hubrise_inventory_sync
                     SET dirty = CASE WHEN version = ?1 THEN 0 ELSE 1 END,
                         attempts = 0, next_attempt_at = NULL, error_msg = NULL,
                         synced_at = datetime('now')
                     WHERE menu_item_id = ?2",
                    params![row.version, row.menu_item_id],
                )?;
                token_store::clear_auth_error(&db)?;
                report.sent += 1;
            }
            Outcome::Retry(err) => {
                let delay = (BASE_BACKOFF_SECS << row.attempts.min(10)).min(MAX_BACKOFF_SECS);
                db.execute(
                    "UPDATE hubrise_inventory_sync
                     SET attempts = attempts + 1, last_attempt_at = datetime('now'),
                         next_attempt_at = datetime('now', ?1), error_msg = ?2
                     WHERE menu_item_id = ?3",
                    params![format!("+{delay} seconds"), err, row.menu_item_id],
                )?;
                tracing::warn!(sku = %row.sku_ref, attempt = row.attempts + 1, %err, "inventory: will retry");
                report.retried += 1;
            }
            Outcome::Park(err) => {
                db.execute(
                    "UPDATE hubrise_inventory_sync
                     SET parked_at = datetime('now'), last_attempt_at = datetime('now'),
                         attempts = attempts + 1, error_msg = ?1
                     WHERE menu_item_id = ?2",
                    params![err, row.menu_item_id],
                )?;
                tracing::warn!(sku = %row.sku_ref, %err, "inventory: parked, HubRise refused it");
                report.parked += 1;
            }
            Outcome::Unauthorized => {
                token_store::set_auth_error(&db, "HubRise refused the access token")?;
                report.unauthorized = true;
                return Ok(report);
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_three_states_have_the_documented_bodies() {
        let day = entry_for(
            "sku-1",
            &Stock::SoldOut {
                expires_at: Some("2026-10-18T04:00:00+02:00".into()),
            },
        );
        assert_eq!(
            serde_json::to_value(day).unwrap(),
            json!({"sku_ref": "sku-1", "stock": "0", "expires_at": "2026-10-18T04:00:00+02:00"})
        );
        let until_reenabled = entry_for("sku-1", &Stock::SoldOut { expires_at: None });
        assert_eq!(
            serde_json::to_value(until_reenabled).unwrap(),
            json!({"sku_ref": "sku-1", "stock": "0"})
        );
        let cleared = entry_for("sku-1", &Stock::Unlimited);
        assert_eq!(
            serde_json::to_value(cleared).unwrap(),
            json!({"sku_ref": "sku-1", "stock": null})
        );
    }
}
