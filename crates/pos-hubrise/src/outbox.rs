//! Durable outbox for POS → HubRise order push.
//!
//! The checkout path writes into `hubrise_outbox` (pure SQLite, zero
//! network), and this module provides a background worker that drains
//! the queue whenever HubRise is reachable. If HubRise or the
//! internet is down, orders accumulate safely until connectivity
//! returns - the register is never blocked.
//!
//! Retry policy: exponential backoff capped at ~30 min. After 10
//! consecutive failures the row stays in the table but is skipped
//! until manual intervention (or HubRise comes back, at which point
//! the next successful tick resets the streak).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::{Connection, params};
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use crate::client::Client;
use crate::errors::Result;
use crate::token_store;

/// Maximum number of push attempts before the row is parked.
const MAX_ATTEMPTS: i64 = 10;
/// Base delay between retries (doubles each attempt).
const BASE_BACKOFF_SECS: u64 = 30;
/// How often the worker wakes up to scan the outbox.
const TICK_INTERVAL: Duration = Duration::from_secs(30);
/// Max rows processed per tick to avoid long lock holds.
const BATCH_SIZE: i64 = 20;

/// Enqueue a completed POS order for async push to HubRise.
/// Called from the checkout path - pure SQLite write, no network.
pub fn enqueue(conn: &Connection, order_id: i64, body_json: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO hubrise_outbox (order_id, body_json)
         VALUES (?1, ?2)",
        params![order_id, body_json],
    )?;
    Ok(())
}

/// Context needed by the outbox worker loop.
pub struct OutboxCtx {
    pub http: reqwest::Client,
    /// Dedicated SQLite connection (same WAL file, separate from UI).
    pub db: Arc<Mutex<Connection>>,
}

/// Run the outbox drain loop until `cancel` is triggered.
pub async fn run_loop(ctx: OutboxCtx, cancel: CancellationToken) {
    let mut ticker = interval(TICK_INTERVAL);
    // Skip the first immediate tick (let the app boot fully first).
    ticker.tick().await;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = ticker.tick() => {
                if let Err(e) = process_pending(&ctx).await {
                    tracing::warn!(?e, "hubrise outbox tick error");
                }
            }
        }
    }
}

/// One pass: fetch pending rows, push each, mark sent or bump retry.
async fn process_pending(ctx: &OutboxCtx) -> Result<()> {
    // 0. Expire rows older than 7 days - prevents a reconnection
    //    spike if HubRise was down for weeks.
    {
        let db = ctx.db.lock().expect("outbox db poisoned");
        let expired = db
            .execute(
                "UPDATE hubrise_outbox
             SET sent_at = 'expired', error_msg = 'TTL exceeded (7 days)'
             WHERE sent_at IS NULL AND created_at < datetime('now', '-7 days')",
                [],
            )
            .unwrap_or(0);
        if expired > 0 {
            tracing::warn!(expired, "outbox: expired old rows (TTL 7 days)");
        }
    }

    // 1. Read the stored HubRise connection (short lock).
    let stored = {
        let db = ctx.db.lock().expect("outbox db poisoned");
        token_store::load(&db)?
    };
    let Some(stored) = stored else {
        return Ok(()); // Not connected - nothing to push.
    };

    let client = Client::new(
        ctx.http.clone(),
        stored.base_url.clone(),
        stored.access_token.clone(),
    );

    // 2. Snapshot pending rows (short lock).
    let rows: Vec<(i64, String, i64)> = {
        let db = ctx.db.lock().expect("outbox db poisoned");
        let mut stmt = db.prepare(
            "SELECT id, body_json, attempts
             FROM hubrise_outbox
             WHERE sent_at IS NULL
               AND attempts < ?1
               AND created_at > datetime('now', '-7 days')
             ORDER BY created_at ASC
             LIMIT ?2",
        )?;
        let mapped = stmt.query_map(params![MAX_ATTEMPTS, BATCH_SIZE], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;
        mapped.collect::<std::result::Result<Vec<_>, _>>()?
    };

    if rows.is_empty() {
        return Ok(());
    }

    let location_id = stored.location_id.clone();

    for (id, body_json, attempts) in rows {
        // Exponential backoff: skip if too soon since last attempt.
        // We check this in Rust rather than SQL because SQLite's
        // datetime arithmetic is cumbersome and the row count is small.
        if attempts > 0 {
            let should_skip = {
                let db = ctx.db.lock().expect("outbox db poisoned");
                let last: Option<String> = db
                    .query_row(
                        "SELECT last_attempt_at FROM hubrise_outbox WHERE id = ?1",
                        params![id],
                        |row| row.get(0),
                    )
                    .ok()
                    .flatten();
                if let Some(last_str) = last {
                    let backoff_secs = BASE_BACKOFF_SECS * (1u64 << attempts.min(8) as u32);
                    // Parse SQLite datetime "YYYY-MM-DD HH:MM:SS" as UTC.
                    if let Ok(last_dt) =
                        chrono::NaiveDateTime::parse_from_str(&last_str, "%Y-%m-%dT%H:%M:%SZ")
                            .or_else(|_| {
                                chrono::NaiveDateTime::parse_from_str(
                                    &last_str,
                                    "%Y-%m-%d %H:%M:%S",
                                )
                            })
                    {
                        let cutoff = last_dt + chrono::Duration::seconds(backoff_secs as i64);
                        chrono::Utc::now().naive_utc() < cutoff
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            if should_skip {
                continue;
            }
        }

        // 3. Async HTTP push - no lock held.
        let body: serde_json::Value = match serde_json::from_str(&body_json) {
            Ok(v) => v,
            Err(e) => {
                // Malformed JSON - park the row permanently.
                let db = ctx.db.lock().expect("outbox db poisoned");
                db.execute(
                    "UPDATE hubrise_outbox SET attempts = ?1, error_msg = ?2 WHERE id = ?3",
                    params![MAX_ATTEMPTS, format!("bad json: {e}"), id],
                )?;
                continue;
            }
        };

        let result = crate::push::create_order(&client, &location_id, &body).await;

        // 4. Short lock to update the row.
        let db = ctx.db.lock().expect("outbox db poisoned");
        match result {
            Ok(resp) => {
                let hubrise_id = resp.get("id").and_then(|v| v.as_str()).unwrap_or("");
                db.execute(
                    "UPDATE hubrise_outbox SET sent_at = datetime('now'), error_msg = NULL WHERE id = ?1",
                    params![id],
                )?;
                tracing::info!(id, hubrise_id, "outbox: pushed order to HubRise");
            }
            Err(e) => {
                db.execute(
                    "UPDATE hubrise_outbox
                     SET attempts = attempts + 1,
                         last_attempt_at = datetime('now'),
                         error_msg = ?1
                     WHERE id = ?2",
                    params![format!("{e}"), id],
                )?;
                tracing::warn!(id, attempt = attempts + 1, %e, "outbox: push failed");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pos_db::schema::run_migrations;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn enqueue_inserts_row() {
        let conn = test_conn();
        enqueue(&conn, 42, r#"{"status":"completed"}"#).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM hubrise_outbox WHERE order_id = 42",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn enqueue_multiple_orders() {
        let conn = test_conn();
        enqueue(&conn, 1, "{}").unwrap();
        enqueue(&conn, 2, "{}").unwrap();
        enqueue(&conn, 3, "{}").unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM hubrise_outbox WHERE sent_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn ttl_expires_old_rows() {
        let conn = test_conn();
        // Insert a row with created_at 8 days ago.
        conn.execute(
            "INSERT INTO hubrise_outbox (order_id, body_json, created_at)
             VALUES (99, '{}', datetime('now', '-8 days'))",
            [],
        )
        .unwrap();
        // Run the expire logic.
        let expired = conn
            .execute(
                "UPDATE hubrise_outbox
             SET sent_at = 'expired', error_msg = 'TTL exceeded (7 days)'
             WHERE sent_at IS NULL AND created_at < datetime('now', '-7 days')",
                [],
            )
            .unwrap();
        assert_eq!(expired, 1);
        // Verify it's no longer pending.
        let pending: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM hubrise_outbox WHERE sent_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending, 0);
    }

    #[test]
    fn recent_rows_not_expired() {
        let conn = test_conn();
        enqueue(&conn, 1, "{}").unwrap();
        let expired = conn
            .execute(
                "UPDATE hubrise_outbox
             SET sent_at = 'expired', error_msg = 'TTL exceeded'
             WHERE sent_at IS NULL AND created_at < datetime('now', '-7 days')",
                [],
            )
            .unwrap();
        assert_eq!(expired, 0);
    }
}
