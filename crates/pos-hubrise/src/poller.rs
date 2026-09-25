//! Tokio-based polling loop that fetches HubRise callback events
//! every 30 seconds and dispatches them.
//!
//! Critical constraint: the `std::sync::MutexGuard` used to protect
//! the SQLite connection is not `Send`, so we must NEVER hold it
//! across `.await`. Each loop iteration takes the lock briefly to
//! read stored state, releases it, performs HTTP calls, then
//! re-acquires it to write results.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use pos_hubrise_types::{CallbackEvent, Catalog, Order, ServiceType};
use rusqlite::{Connection, params};
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use crate::catalog_apply;
use crate::client::Client;
use crate::dispatcher::{DispatchOutcome, mark_failed, mark_processed, persist_event};
use crate::errors::{HubriseError, Result};
use crate::token_store;

/// Default polling interval (HubRise recommends at least 30s).
pub const POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Hook callback invoked by the loop whenever a dispatch produces a
/// meaningful outcome. The Tauri layer uses this to `app.emit(...)`.
pub type OutcomeHook = Arc<dyn Fn(DispatchOutcome, CallbackEvent) + Send + Sync>;

/// Context needed by the poller loop.
pub struct PollerCtx {
    pub http: reqwest::Client,
    /// Dedicated SQLite connection used by the background loop.
    /// Must be separate from the UI connection to avoid lock contention.
    pub db: Arc<Mutex<Connection>>,
    pub on_outcome: Option<OutcomeHook>,
}

/// Run the polling loop until `cancel` is triggered.
pub async fn run_loop(ctx: PollerCtx, cancel: CancellationToken) {
    let mut ticker = interval(POLL_INTERVAL);
    // Skip the first immediate tick so startup doesn't hammer the API
    // before the connection has a chance to be configured.
    ticker.tick().await;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                return;
            }
            _ = ticker.tick() => {
                if let Err(e) = poll_once(&ctx).await {
                    tracing::warn!(?e, "hubrise poll error");
                }
            }
        }
    }
}

async fn poll_once(ctx: &PollerCtx) -> Result<()> {
    // 1. Snapshot the stored connection under a short, non-await lock.
    let stored_opt = {
        let db = ctx.db.lock().expect("poller db poisoned");
        token_store::load(&db)?
    };
    let Some(stored) = stored_opt else {
        return Ok(());
    };

    let client = Client::new(
        ctx.http.clone(),
        stored.base_url.clone(),
        stored.access_token.clone(),
    );

    // 2. Fetch events over HTTP (no lock held).
    let events: Vec<CallbackEvent> = match client.get_json("/callback/events?count=100").await {
        Ok(v) => v,
        Err(HubriseError::Unauthorized) => {
            tracing::warn!("hubrise poll: unauthorized - token may be expired");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    // 3. Persist + dispatch each event.
    for event in events {
        let raw = serde_json::to_string(&event)?;

        // 3a. Short lock: idempotent insert.
        {
            let db = ctx.db.lock().expect("poller db poisoned");
            let _ = persist_event(&db, &event, &raw)?;
        }

        // 3b. Route through a helper that returns the outcome after
        //     any async work, keeping the DB lock scope minimal.
        let outcome_res = process_event(&client, &ctx.db, &event).await;

        match outcome_res {
            Ok(outcome) => {
                {
                    let db = ctx.db.lock().expect("poller db poisoned");
                    mark_processed(&db, &event.id)?;
                }
                if let Some(hook) = &ctx.on_outcome {
                    hook(outcome, event.clone());
                }
                // Ack on the remote side so the event isn't replayed.
                let _ = client
                    .delete(&format!("/callback/events/{}", event.id))
                    .await;
            }
            Err(e) => {
                let db = ctx.db.lock().expect("poller db poisoned");
                let _ = mark_failed(&db, &event.id, &format!("{e:?}"));
            }
        }
    }

    Ok(())
}

/// Handle a single event, splitting async HTTP work from SQLite
/// writes so we never hold a `MutexGuard` across `.await`.
async fn process_event(
    client: &Client,
    db: &Arc<Mutex<Connection>>,
    event: &CallbackEvent,
) -> Result<DispatchOutcome> {
    if event.is_catalog_update() {
        let catalog_id = event
            .extra
            .get("catalog_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let Some(catalog_id) = catalog_id else {
            return Ok(DispatchOutcome::Ignored);
        };

        // Async HTTP fetch - no lock held.
        let path = format!("/catalogs/{}", catalog_id);
        let catalog: Catalog = client.get_json(&path).await?;

        // Short lock to apply the snapshot.
        {
            let mut conn = db.lock().expect("poller db poisoned");
            catalog_apply::apply(&mut conn, &catalog)?;
        }
        return Ok(DispatchOutcome::CatalogUpdated);
    }

    if event.is_order_create() {
        let Some(order_id) = event
            .extra
            .get("order_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        else {
            return Ok(DispatchOutcome::Ignored);
        };
        let location_id = event
            .extra
            .get("location_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Async HTTP fetch - no lock held.
        let path = format!("/locations/{}/orders/{}", location_id, order_id);
        let order: Order = client.get_json(&path).await?;

        // Short lock to insert the local webshop row. Idempotent via
        // the UNIQUE index on orders(hubrise_order_id).
        {
            let conn = db.lock().expect("poller db poisoned");
            ingest_webshop_order(&conn, &order)?;
        }
        return Ok(DispatchOutcome::OrderCreated);
    }

    if event.is_order_update() {
        return Ok(DispatchOutcome::OrderUpdated);
    }

    Ok(DispatchOutcome::Ignored)
}

/// Insert a HubRise order into the local `orders` table as a
/// non-fiscal webshop row (is_fiscal = 0). Idempotent: relies on a
/// UNIQUE index on `hubrise_order_id` so repeated events are a no-op.
fn ingest_webshop_order(conn: &Connection, order: &Order) -> Result<()> {
    // Map HubRise service_type -> POS order_type. The orders table
    // CHECK constraint only allows 'dine_in' or 'takeaway', so both
    // delivery and collection get bucketed as 'takeaway' for now.
    let order_type = match order.service_type {
        Some(ServiceType::EatIn) => "dine_in",
        _ => "takeaway",
    };
    let service_type_str: Option<&'static str> = order.service_type.map(|s| match s {
        ServiceType::Delivery => "delivery",
        ServiceType::Collection => "collection",
        ServiceType::EatIn => "eat_in",
    });
    let total_cents = parse_money_cents(order.total.as_deref().unwrap_or("0.00 EUR"));
    // Prefer the HubRise `channel` (e.g. "Deliveroo", "Uber Eats") as
    // the local `source` so the UI can differentiate platforms. Fall
    // back to "webshop" when the channel is absent.
    let source = order
        .channel
        .clone()
        .unwrap_or_else(|| "webshop".to_string());
    let hubrise_status = order.status.as_wire();
    // Placeholder INSZ for system-inserted orders - the schema
    // requires a non-null user_insz and webshop orders have no
    // operator yet.
    let system_insz = "00000000029";

    // Serialize the full HubRise Order so the UI can surface items,
    // customer notes, etc. without going through order_lines (which
    // has a hard FK on menu_items(id) that HubRise rows never match).
    let raw_json = serde_json::to_string(order)?;

    conn.execute(
        "INSERT OR IGNORE INTO orders
            (order_type, status, user_insz, subtotal_cents, total_cents,
             hubrise_order_id, is_fiscal, source, hubrise_status, service_type,
             raw_hubrise_json)
         VALUES (?1, 'open', ?2, ?3, ?4, ?5, 0, ?6, ?7, ?8, ?9)",
        params![
            order_type,
            system_insz,
            total_cents,
            total_cents,
            order.id,
            source,
            hubrise_status,
            service_type_str,
            raw_json,
        ],
    )?;
    Ok(())
}

/// Parse a HubRise money string such as `"28.50 EUR"` into an
/// integer number of cents. Invalid strings fall back to 0.
fn parse_money_cents(money: &str) -> i64 {
    let num_part = money.split_whitespace().next().unwrap_or("0");
    let parsed: f64 = num_part.parse().unwrap_or(0.0);
    (parsed * 100.0).round() as i64
}
