//! Durable outbox for POS → HubRise writes.
//!
//! Two kinds of rows, written in the same SQLite transaction as the
//! change they report (pure SQLite, zero network), drained by a
//! background worker whenever HubRise is reachable:
//!
//! - `create_order`: a POS sale pushed to HubRise for central
//!   reporting. Each carries a `private_ref` (`pili-order-<id>`): a
//!   retry after a lost answer finds the order already created
//!   (`GET orders?private_ref=`) instead of creating it twice. Rows
//!   older than 7 days expire.
//! - `patch_order`: a status or time change of an online order
//!   (`received`, `accepted`, `completed`, ...). They never expire,
//!   and the PATCHes of one order are sent in the order they were
//!   queued: a later one waits for the earlier to go through.
//!
//! Errors: HubRise refusing a change (400, 404, 422) parks the row
//! with its reason; a timeout, 429 or 5xx is retried with backoff
//! (30 s doubling, capped at 30 min); a 401 stops the pass and marks
//! the connection's authentication error, without counting attempts.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use crate::client::Client;
use crate::errors::{HubriseError, Result};
use crate::token_store;

/// Attempts before an order creation is parked. PATCHes retry forever.
const MAX_CREATE_ATTEMPTS: i64 = 10;
/// Base delay between retries (doubles each attempt).
const BASE_BACKOFF_SECS: i64 = 30;
/// Longest delay between two retries.
const MAX_BACKOFF_SECS: i64 = 30 * 60;
/// How often the worker wakes up to scan the outbox.
const TICK_INTERVAL: Duration = Duration::from_secs(30);
/// Max rows processed per pass.
const BATCH_SIZE: i64 = 50;

/// The `private_ref` of the HubRise order a POS sale creates.
pub fn sale_private_ref(order_id: i64) -> String {
    format!("pili-order-{order_id}")
}

/// Enqueue a completed POS order for async push to HubRise.
/// Called from the checkout path - pure SQLite write, no network.
/// Queuing the same order twice keeps one row.
pub fn enqueue(conn: &Connection, order_id: i64, body_json: &str) -> Result<()> {
    let private_ref = sale_private_ref(order_id);
    let body = match serde_json::from_str::<Value>(body_json) {
        Ok(mut v) if v.is_object() => {
            v["private_ref"] = Value::String(private_ref.clone());
            v.to_string()
        }
        _ => body_json.to_string(),
    };
    conn.execute(
        "INSERT OR IGNORE INTO hubrise_outbox (order_id, body_json, kind, private_ref)
         VALUES (?1, ?2, 'create_order', ?3)",
        params![order_id, body, private_ref],
    )?;
    Ok(())
}

/// Queue a PATCH of HubRise order `hubrise_order_id` (local order
/// `order_id`), sent after any PATCH already queued for it.
pub fn enqueue_patch(
    conn: &Connection,
    order_id: i64,
    hubrise_order_id: &str,
    body: &Value,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO hubrise_outbox (order_id, body_json, kind, hubrise_order_id)
         VALUES (?1, ?2, 'patch_order', ?3)",
        params![order_id, body.to_string(), hubrise_order_id],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Rows waiting to be sent, and rows parked on an error.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct Backlog {
    pub pending: i64,
    pub parked: i64,
}

pub fn backlog(conn: &Connection) -> Result<Backlog> {
    Ok(conn.query_row(
        "SELECT
            COALESCE(SUM(sent_at IS NULL AND parked_at IS NULL), 0),
            COALESCE(SUM(sent_at IS NULL AND parked_at IS NOT NULL), 0)
         FROM hubrise_outbox",
        [],
        |r| {
            Ok(Backlog {
                pending: r.get(0)?,
                parked: r.get(1)?,
            })
        },
    )?)
}

/// Context needed by the outbox worker loop.
pub struct OutboxCtx {
    pub http: reqwest::Client,
    /// Dedicated SQLite connection (same WAL file, separate from UI).
    pub db: Arc<Mutex<Connection>>,
    /// Wakes the worker before its next tick (a status change to send).
    pub wake: Arc<tokio::sync::Notify>,
    /// Off: the queue waits, nothing is sent.
    pub enabled: crate::switch::Switch,
}

/// Run the outbox drain loop until `cancel` is triggered.
pub async fn run_loop(ctx: OutboxCtx, cancel: CancellationToken) {
    let mut ticker = interval(TICK_INTERVAL);
    // Skip the first immediate tick (let the app boot fully first).
    ticker.tick().await;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = ticker.tick() => {}
            _ = ctx.wake.notified() => {}
        }
        if !ctx.enabled.is_on() {
            continue;
        }
        if let Err(e) = process_pending(&ctx.http, &ctx.db).await {
            tracing::warn!(?e, "hubrise outbox pass error");
        }
    }
}

/// What one pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PassReport {
    pub sent: usize,
    pub retried: usize,
    pub parked: usize,
    /// HubRise refused the token: the pass stopped.
    pub unauthorized: bool,
}

struct Row {
    id: i64,
    kind: String,
    body: String,
    attempts: i64,
    hubrise_order_id: Option<String>,
    private_ref: Option<String>,
}

enum Outcome {
    Sent,
    Retry(String),
    Park(String),
    Unauthorized,
}

/// Send what is due, in queue order. A PATCH waiting behind one sent
/// in this pass goes in the same pass.
pub async fn process_pending(
    http: &reqwest::Client,
    db: &Arc<Mutex<Connection>>,
) -> Result<PassReport> {
    let mut total = PassReport::default();
    // Bounded: each round sends at least one row or stops.
    for _ in 0..20 {
        let round = process_round(http, db).await?;
        total.sent += round.sent;
        total.retried += round.retried;
        total.parked += round.parked;
        total.unauthorized |= round.unauthorized;
        if round.sent == 0 || round.unauthorized {
            break;
        }
    }
    Ok(total)
}

/// One round over the rows due now.
async fn process_round(http: &reqwest::Client, db: &Arc<Mutex<Connection>>) -> Result<PassReport> {
    let mut report = PassReport::default();
    let (stored, rows) = {
        let db = db.lock().expect("outbox db poisoned");
        // Order creations older than 7 days expire: a push for central
        // reporting is not worth a spike after weeks offline.
        let expired = db.execute(
            "UPDATE hubrise_outbox
             SET parked_at = datetime('now'), error_msg = 'TTL exceeded (7 days)'
             WHERE kind = 'create_order' AND sent_at IS NULL AND parked_at IS NULL
               AND created_at < datetime('now', '-7 days')",
            [],
        )?;
        if expired > 0 {
            tracing::warn!(expired, "outbox: expired old order pushes (TTL 7 days)");
        }
        let Some(stored) = token_store::load(&db)? else {
            return Ok(report);
        };
        let mut stmt = db.prepare(
            "SELECT id, kind, body_json, attempts, hubrise_order_id, private_ref
             FROM hubrise_outbox o
             WHERE sent_at IS NULL AND parked_at IS NULL
               AND (next_attempt_at IS NULL OR next_attempt_at <= datetime('now'))
               AND NOT EXISTS (
                   SELECT 1 FROM hubrise_outbox e
                   WHERE e.hubrise_order_id = o.hubrise_order_id
                     AND e.sent_at IS NULL AND e.parked_at IS NULL AND e.id < o.id)
             ORDER BY id
             LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![BATCH_SIZE], |r| {
                Ok(Row {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    body: r.get(2)?,
                    attempts: r.get(3)?,
                    hubrise_order_id: r.get(4)?,
                    private_ref: r.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        (stored, rows)
    };
    let client = Client::new(http.clone(), &stored.base_url, &stored.access_token);
    let location = stored.location_id.clone();

    for row in rows {
        let outcome = match serde_json::from_str::<Value>(&row.body) {
            Err(e) => Outcome::Park(format!("bad json: {e}")),
            Ok(body) => send(&client, &location, &row, &body).await,
        };
        let db = db.lock().expect("outbox db poisoned");
        match outcome {
            Outcome::Sent => {
                db.execute(
                    "UPDATE hubrise_outbox SET sent_at = datetime('now'), error_msg = NULL
                     WHERE id = ?1",
                    params![row.id],
                )?;
                token_store::clear_auth_error(&db)?;
                report.sent += 1;
            }
            Outcome::Retry(err) => {
                let delay = (BASE_BACKOFF_SECS << row.attempts.min(10)).min(MAX_BACKOFF_SECS);
                let park = row.kind == "create_order" && row.attempts + 1 >= MAX_CREATE_ATTEMPTS;
                db.execute(
                    "UPDATE hubrise_outbox
                     SET attempts = attempts + 1, last_attempt_at = datetime('now'),
                         next_attempt_at = datetime('now', ?1), error_msg = ?2,
                         parked_at = CASE WHEN ?3 THEN datetime('now') END
                     WHERE id = ?4",
                    params![format!("+{delay} seconds"), err, park, row.id],
                )?;
                tracing::warn!(id = row.id, attempt = row.attempts + 1, %err, "outbox: will retry");
                if park {
                    report.parked += 1;
                } else {
                    report.retried += 1;
                }
            }
            Outcome::Park(err) => {
                db.execute(
                    "UPDATE hubrise_outbox
                     SET parked_at = datetime('now'), last_attempt_at = datetime('now'),
                         attempts = attempts + 1, error_msg = ?1
                     WHERE id = ?2",
                    params![err, row.id],
                )?;
                tracing::warn!(id = row.id, %err, "outbox: parked, HubRise refused it");
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

/// Send one row.
async fn send(client: &Client, location: &str, row: &Row, body: &Value) -> Outcome {
    let result = match (row.kind.as_str(), &row.hubrise_order_id) {
        ("patch_order", Some(order_id)) => client
            .patch_json::<_, Value>(&format!("/locations/{location}/orders/{order_id}"), body)
            .await
            .map(|_| ()),
        ("patch_order", None) => return Outcome::Park("patch without order id".into()),
        _ => {
            // A retry: the first try may have been created already.
            if row.attempts > 0
                && let Some(private_ref) = &row.private_ref
            {
                match already_created(client, location, private_ref).await {
                    Ok(true) => return Outcome::Sent,
                    Ok(false) => {}
                    Err(e) => return classify(e),
                }
            }
            client
                .post_json::<_, Value>(&format!("/locations/{location}/orders"), body)
                .await
                .map(|_| ())
        }
    };
    match result {
        Ok(()) => Outcome::Sent,
        // Created by an earlier try whose answer was lost.
        Err(HubriseError::Api { status: 422, body }) if body.contains("private_ref") => {
            Outcome::Sent
        }
        Err(e) => classify(e),
    }
}

async fn already_created(client: &Client, location: &str, private_ref: &str) -> Result<bool> {
    let path = format!(
        "/locations/{location}/orders?private_ref={}",
        url::form_urlencoded::byte_serialize(private_ref.as_bytes()).collect::<String>()
    );
    let found: Vec<Value> = client.get_json(&path).await?;
    Ok(!found.is_empty())
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

/// The error a parked or retried row last got, for the status screen.
pub fn last_error(conn: &Connection) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT error_msg FROM hubrise_outbox
             WHERE sent_at IS NULL AND error_msg IS NOT NULL
             ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?)
}

/// Put parked rows back in the queue (an operator's "retry").
pub fn retry_parked(conn: &Connection) -> Result<usize> {
    Ok(conn.execute(
        "UPDATE hubrise_outbox
         SET parked_at = NULL, attempts = 0, next_attempt_at = NULL
         WHERE sent_at IS NULL AND parked_at IS NOT NULL",
        [],
    )?)
}

/// The ids of orders with a PATCH still waiting, for tests.
pub fn orders_with_pending_patches(conn: &Connection) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT hubrise_order_id FROM hubrise_outbox
         WHERE kind = 'patch_order' AND sent_at IS NULL AND parked_at IS NULL",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pos_db::schema::run_migrations;
    use serde_json::json;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn enqueue_inserts_one_row_per_order() {
        let conn = test_conn();
        enqueue(&conn, 42, r#"{"status":"completed"}"#).unwrap();
        enqueue(&conn, 42, r#"{"status":"completed"}"#).unwrap();
        let (count, body): (i64, String) = conn
            .query_row(
                "SELECT COUNT(*), MAX(body_json) FROM hubrise_outbox WHERE order_id = 42",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(count, 1, "deduplicated by private_ref");
        let body: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body["private_ref"], "pili-order-42");
    }

    #[test]
    fn patches_queue_per_order() {
        let conn = test_conn();
        enqueue_patch(&conn, 1, "o1", &json!({"status": "received"})).unwrap();
        enqueue_patch(&conn, 1, "o1", &json!({"status": "accepted"})).unwrap();
        enqueue_patch(&conn, 2, "o2", &json!({"status": "received"})).unwrap();
        assert_eq!(
            backlog(&conn).unwrap(),
            Backlog {
                pending: 3,
                parked: 0
            }
        );
        let pending = orders_with_pending_patches(&conn).unwrap();
        assert!(pending.contains("o1") && pending.contains("o2"));
    }

    #[test]
    fn old_order_pushes_expire_but_patches_do_not() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO hubrise_outbox (order_id, body_json, kind, private_ref, created_at)
             VALUES (99, '{}', 'create_order', 'pili-order-99', datetime('now', '-8 days'))",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO hubrise_outbox (order_id, body_json, kind, hubrise_order_id, created_at)
             VALUES (98, '{}', 'patch_order', 'o98', datetime('now', '-8 days'))",
            [],
        )
        .unwrap();
        let db = Arc::new(Mutex::new(conn));
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        // Not connected: the pass only expires rows.
        rt.block_on(process_pending(&reqwest::Client::new(), &db))
            .unwrap();
        let conn = db.lock().unwrap();
        assert_eq!(
            backlog(&conn).unwrap(),
            Backlog {
                pending: 1,
                parked: 1
            }
        );
    }

    #[test]
    fn parked_rows_can_be_retried() {
        let conn = test_conn();
        enqueue_patch(&conn, 1, "o1", &json!({"status": "accepted"})).unwrap();
        conn.execute("UPDATE hubrise_outbox SET parked_at = datetime('now')", [])
            .unwrap();
        assert_eq!(backlog(&conn).unwrap().parked, 1);
        assert_eq!(retry_parked(&conn).unwrap(), 1);
        assert_eq!(
            backlog(&conn).unwrap(),
            Backlog {
                pending: 1,
                parked: 0
            }
        );
    }
}
