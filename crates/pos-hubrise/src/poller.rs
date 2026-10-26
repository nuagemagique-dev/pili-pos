//! The HubRise event poller: reads the passive callback's events
//! (`docs/hubrise/06-callbacks.md`), handles each, acknowledges it.
//!
//! Every pass reads all pages of `GET /callback/events`, oldest first.
//! An event is stored before it is handled (idempotence: the same
//! event twice is handled once), then deleted on HubRise. One that
//! keeps failing is parked after [`MAX_EVENT_ATTEMPTS`] tries so it
//! never blocks the others.
//!
//! Orders: the event names the order, the poller fetches it (events
//! are listed without their state), stores it, and queues the
//! `received` acknowledgement for a new one. Catalogs: the menu is
//! synchronised from the connected catalog.
//!
//! At start-up (and on connect) [`start`] registers the passive
//! callback and fetches the orders HubRise holds as `new` or
//! `received`: an order placed while the POS was off is not lost.
//!
//! Critical constraint: the `std::sync::MutexGuard` protecting the
//! SQLite connection is not `Send`, so it is NEVER held across
//! `.await`: each step locks briefly, HTTP calls run unlocked.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pos_hubrise_types::{CallbackEvent, Catalog, Order, OrderStatus};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use crate::client::Client;
use crate::dispatcher::{DispatchOutcome, mark_processed, persist_event};
use crate::errors::{HubriseError, Result};
use crate::token_store::{self, StoredConnection};
use crate::util::path_segment;
use crate::{callbacks, outbox};
use pos_catalog::ApplyOutcome;

/// Default polling interval (HubRise asks for at least 30 s).
pub const POLL_INTERVAL: Duration = Duration::from_secs(30);
/// Shortest gap between two passes, even when woken on purpose.
pub const MIN_POLL_GAP: Duration = Duration::from_secs(5);
/// How often the catalog is compared with the applied one.
pub const CATALOG_DRIFT_INTERVAL: Duration = Duration::from_secs(15 * 60);
/// Tries before an event is parked.
pub const MAX_EVENT_ATTEMPTS: i64 = 5;

/// Hook callback invoked by the loop whenever a dispatch produces a
/// meaningful outcome. The Tauri layer uses this to `app.emit(...)`.
pub type OutcomeHook = Arc<dyn Fn(DispatchOutcome, CallbackEvent) + Send + Sync>;

/// What the poller does with a fetched order and a changed catalog.
/// The Tauri layer plugs the webshop and menu logic in here; the
/// defaults only mirror the order and apply the catalog as is.
pub trait Handlers: Send + Sync {
    /// Store or refresh an order fetched from HubRise. Returns true
    /// when the POS should acknowledge it (`received`).
    fn on_order(&self, conn: &Connection, order: &Order) -> Result<bool>;
    /// Apply the connected catalog.
    fn on_catalog(&self, conn: &mut Connection, catalog: &Catalog) -> Result<ApplyOutcome>;
}

/// The default handlers: mirror the order row, apply the catalog.
pub struct DefaultHandlers;

impl Handlers for DefaultHandlers {
    fn on_order(&self, conn: &Connection, order: &Order) -> Result<bool> {
        let ingested = crate::webshop::ingest(conn, order)?;
        Ok(ingested.pending && order.status == OrderStatus::New)
    }

    fn on_catalog(&self, conn: &mut Connection, catalog: &Catalog) -> Result<ApplyOutcome> {
        crate::catalog_map::apply(conn, catalog)
    }
}

/// Context needed by the poller loop.
pub struct PollerCtx {
    pub http: reqwest::Client,
    /// Dedicated SQLite connection used by the background loop.
    /// Must be separate from the UI connection to avoid lock contention.
    pub db: Arc<Mutex<Connection>>,
    pub on_outcome: Option<OutcomeHook>,
    pub handlers: Arc<dyn Handlers>,
    /// Wakes the loop for an early pass (after a staff action).
    pub poll_now: Arc<tokio::sync::Notify>,
    /// Wakes the outbox after queuing an acknowledgement.
    pub outbox_wake: Option<Arc<tokio::sync::Notify>>,
    /// Off: no connection is loaded, so nothing reaches HubRise.
    pub enabled: crate::switch::Switch,
}

/// Run the polling loop until `cancel` is triggered.
pub async fn run_loop(ctx: PollerCtx, cancel: CancellationToken) {
    let mut ticker = interval(POLL_INTERVAL);
    // The first tick is immediate: start-up registers the callback and
    // fetches what arrived while the POS was off.
    let mut started = false;
    let mut last_pass: Option<Instant> = None;
    // The start-up already syncs the catalog.
    let mut last_catalog_check = Instant::now();

    loop {
        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = ticker.tick() => {}
            _ = ctx.poll_now.notified() => {
                if last_pass.is_some_and(|t| t.elapsed() < MIN_POLL_GAP) {
                    continue;
                }
            }
        }
        // Off: nothing reaches HubRise; switched on again, the start-up
        // runs again (callback, catalog, what arrived meanwhile).
        if !ctx.enabled.is_on() {
            started = false;
            continue;
        }
        if !started {
            match start(&ctx).await {
                Ok(()) => started = true,
                Err(e) => tracing::warn!(?e, "hubrise start-up"),
            }
        }
        if let Err(e) = poll_once(&ctx).await {
            tracing::warn!(?e, "hubrise poll error");
        }
        // Drift check: a catalog change HubRise sent no event for (one
        // made by this client, or an event lost) still reaches the
        // menu. Unchanged catalogs are a hash compare, no write.
        if started && last_catalog_check.elapsed() >= CATALOG_DRIFT_INTERVAL {
            last_catalog_check = Instant::now();
            if let Err(e) = check_catalog(&ctx).await {
                tracing::warn!(?e, "hubrise catalog drift check");
            }
        }
        last_pass = Some(Instant::now());
    }
}

fn load_client(ctx: &PollerCtx) -> Result<Option<(StoredConnection, Client)>> {
    if !ctx.enabled.is_on() {
        return Ok(None);
    }
    let db = crate::util::lock_db(&ctx.db);
    Ok(token_store::load(&db)?.map(|stored| {
        let client = Client::new(ctx.http.clone(), &stored.base_url, &stored.access_token);
        (stored, client)
    }))
}

/// Register the passive callback, then fetch the orders HubRise holds
/// as `new` or `received` and refresh the local open ones. Called at
/// start-up and right after connecting. Does nothing when not
/// connected.
pub async fn start(ctx: &PollerCtx) -> Result<()> {
    let Some((stored, client)) = load_client(ctx)? else {
        return Ok(());
    };
    match callbacks::ensure_passive(&client).await {
        Ok(_) => {
            let db = crate::util::lock_db(&ctx.db);
            token_store::mark_callback_registered(&db)?;
        }
        Err(e) => return Err(note_error(ctx, e)),
    }
    // The menu first, so backfilled orders resolve against it. A
    // failed read (the catalog endpoint allows 10 calls a minute) does
    // not stop the start: the drift check tries again.
    if let Err(e) = sync_catalog(ctx, &client, &stored).await {
        tracing::warn!(error = %e, "hubrise catalog sync at start failed");
        if matches!(e, HubriseError::Unauthorized) {
            return Err(note_error(ctx, e));
        }
    }
    for status in ["new", "received"] {
        let path = format!(
            "/locations/{}/orders?status={status}",
            path_segment(&stored.location_id)
        );
        let orders: Vec<Order> = match fetch_all(&client, &path).await {
            Ok(o) => o,
            Err(e) => return Err(note_error(ctx, e)),
        };
        for order in orders {
            handle_order(ctx, &order)?;
        }
    }
    // Orders already known may have changed while the POS was off (a
    // platform cancellation): the open ones, and the ones signed in the
    // last two days, which a cancellation turns into a refund to make.
    let open: Vec<String> = {
        let db = crate::util::lock_db(&ctx.db);
        let mut stmt = db.prepare(
            "SELECT hubrise_order_id FROM orders
             WHERE hubrise_order_id IS NOT NULL
               AND (status = 'open' OR id IN (
                    SELECT order_id FROM webshop_orders
                    WHERE fiscal_state IN ('signed_n', 'signed_p')
                      AND julianday(accepted_at) >= julianday('now', '-2 days')))",
        )?;
        stmt.query_map([], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?
    };
    for id in open {
        let path = format!(
            "/locations/{}/orders/{}",
            path_segment(&stored.location_id),
            path_segment(&id)
        );
        match client.get_json::<Order>(&path).await {
            Ok(order) => {
                handle_order(ctx, &order)?;
            }
            Err(HubriseError::Api { status: 404, .. }) => {}
            Err(e) => return Err(note_error(ctx, e)),
        }
    }
    Ok(())
}

/// Every page of a list.
async fn fetch_all<T: serde::de::DeserializeOwned>(client: &Client, path: &str) -> Result<Vec<T>> {
    let mut all = Vec::new();
    let mut cursor: Option<String> = None;
    let mut seen = std::collections::HashSet::new();
    loop {
        let page = client.get_page::<T>(path, cursor.as_deref()).await?;
        all.extend(page.items);
        match next_cursor(&mut seen, page.next_cursor)? {
            Some(next) => cursor = Some(next),
            None => return Ok(all),
        }
    }
}

/// Most pages one list is followed for.
const MAX_PAGES: usize = 1000;

/// The cursor to fetch next, or `None` at the end of the list. A cursor
/// seen before means HubRise is going round in a cycle (A, B, A): the end
/// for the pages read so far. Too many pages is an error.
fn next_cursor(
    seen: &mut std::collections::HashSet<String>,
    next: Option<String>,
) -> Result<Option<String>> {
    let Some(next) = next else { return Ok(None) };
    if !seen.insert(next.clone()) {
        return Ok(None);
    }
    if seen.len() >= MAX_PAGES {
        return Err(HubriseError::Other(format!(
            "pagination exceeds {MAX_PAGES} pages"
        )));
    }
    Ok(Some(next))
}

/// Record `e` for the status screen (401: the authentication error).
fn note_error(ctx: &PollerCtx, e: HubriseError) -> HubriseError {
    let db = crate::util::lock_db(&ctx.db);
    let _ = match &e {
        HubriseError::Unauthorized => {
            token_store::set_auth_error(&db, "HubRise refused the access token")
        }
        other => token_store::set_last_error(&db, Some(&other.to_string())),
    };
    e
}

/// What one pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PollReport {
    pub events: usize,
    pub handled: usize,
    pub failed: usize,
    pub parked: usize,
    pub unauthorized: bool,
}

/// One pass over the waiting events.
pub async fn poll_once(ctx: &PollerCtx) -> Result<PollReport> {
    let mut report = PollReport::default();
    let Some((stored, client)) = load_client(ctx)? else {
        return Ok(report);
    };

    let events: Vec<CallbackEvent> = match fetch_all(&client, "/callback/events").await {
        Ok(v) => v,
        Err(HubriseError::Unauthorized) => {
            note_error(ctx, HubriseError::Unauthorized);
            report.unauthorized = true;
            return Ok(report);
        }
        Err(e) => return Err(note_error(ctx, e)),
    };
    {
        let db = crate::util::lock_db(&ctx.db);
        token_store::clear_auth_error(&db)?;
        token_store::update_cursor(&db, None)?;
    }
    report.events = events.len();

    for event in events {
        let raw = serde_json::to_string(&event)?;
        let (done, parked) = {
            let db = crate::util::lock_db(&ctx.db);
            persist_event(&db, &event, &raw)?;
            event_state(&db, &event.id)?
        };
        if done || parked {
            // Handled (or given up) on an earlier pass whose delete was
            // lost: acknowledge it again.
            let _ = client
                .delete(&format!("/callback/events/{}", path_segment(&event.id)))
                .await;
            continue;
        }

        match process_event(ctx, &client, &stored, &event).await {
            Ok(outcome) => {
                {
                    let db = crate::util::lock_db(&ctx.db);
                    mark_processed(&db, &event.id)?;
                }
                report.handled += 1;
                if let Some(hook) = &ctx.on_outcome {
                    hook(outcome, event.clone());
                }
                let _ = client
                    .delete(&format!("/callback/events/{}", path_segment(&event.id)))
                    .await;
            }
            Err(HubriseError::Unauthorized) => {
                note_error(ctx, HubriseError::Unauthorized);
                report.unauthorized = true;
                return Ok(report);
            }
            Err(e) => {
                let parked = {
                    let db = crate::util::lock_db(&ctx.db);
                    record_failure(&db, &event.id, &e.to_string())?
                };
                if parked {
                    report.parked += 1;
                    tracing::error!(event = %event.id, %e, "hubrise event parked");
                    let _ = client
                        .delete(&format!("/callback/events/{}", path_segment(&event.id)))
                        .await;
                } else {
                    report.failed += 1;
                    tracing::warn!(event = %event.id, %e, "hubrise event failed, will retry");
                }
            }
        }
    }
    if let Err(e) = refresh_on_catalog_miss(ctx, &client, &stored).await {
        tracing::warn!(%e, "hubrise catalog refresh for unresolved orders");
    }
    Ok(report)
}

/// `(processed, parked)` of a stored event.
fn event_state(conn: &Connection, id: &str) -> Result<(bool, bool)> {
    Ok(conn.query_row(
        "SELECT processed_at IS NOT NULL, parked_at IS NOT NULL FROM hubrise_events WHERE id = ?1",
        params![id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?)
}

/// Count a failed try; park the event at the limit. Returns true when
/// parked.
fn record_failure(conn: &Connection, id: &str, err: &str) -> Result<bool> {
    conn.execute(
        "UPDATE hubrise_events
         SET attempts = attempts + 1, error_msg = ?1,
             parked_at = CASE WHEN attempts + 1 >= ?2 THEN datetime('now') END
         WHERE id = ?3",
        params![err, MAX_EVENT_ATTEMPTS, id],
    )?;
    let parked: Option<String> = conn
        .query_row(
            "SELECT parked_at FROM hubrise_events WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    Ok(parked.is_some())
}

/// Handle one event (HTTP unlocked, SQLite in short locks).
async fn process_event(
    ctx: &PollerCtx,
    client: &Client,
    stored: &StoredConnection,
    event: &CallbackEvent,
) -> Result<DispatchOutcome> {
    let field = |k: &str| {
        event
            .extra
            .get(k)
            .and_then(|v| v.as_str())
            .map(str::to_string)
    };
    match (event.resource_type.as_str(), event.event_type.as_str()) {
        ("catalog", "create" | "update") => {
            // Only the connected catalog makes the menu.
            if field("catalog_id").as_deref() != Some(stored.catalog_id.as_str()) {
                return Ok(DispatchOutcome::Ignored);
            }
            Ok(match sync_catalog(ctx, client, stored).await? {
                ApplyOutcome::Applied { .. } => DispatchOutcome::CatalogUpdated,
                ApplyOutcome::Rejected { .. } => DispatchOutcome::CatalogRejected,
                ApplyOutcome::AwaitingAdoption => DispatchOutcome::CatalogAwaitingAdoption,
                ApplyOutcome::Unchanged => DispatchOutcome::Ignored,
            })
        }
        ("order", kind @ ("create" | "update")) => {
            let Some(order_id) = field("order_id") else {
                return Ok(DispatchOutcome::Ignored);
            };
            let location = field("location_id").unwrap_or_else(|| stored.location_id.clone());
            if location != stored.location_id {
                return Ok(DispatchOutcome::Ignored);
            }
            let order: Order = client
                .get_json(&format!(
                    "/locations/{}/orders/{}",
                    path_segment(&location),
                    path_segment(&order_id)
                ))
                .await?;
            handle_order(ctx, &order)?;
            Ok(if kind == "create" {
                DispatchOutcome::OrderCreated
            } else {
                DispatchOutcome::OrderUpdated
            })
        }
        _ => Ok(DispatchOutcome::Ignored),
    }
}

/// Store an order through the handlers and queue its acknowledgement.
fn handle_order(ctx: &PollerCtx, order: &Order) -> Result<()> {
    let acknowledge = {
        let mut db = crate::util::lock_db(&ctx.db);
        let conn = db.transaction()?;
        let ack = ctx.handlers.on_order(&conn, order)?;
        if ack {
            let local_id: i64 = conn.query_row(
                "SELECT id FROM orders WHERE hubrise_order_id = ?1",
                params![order.id],
                |r| r.get(0),
            )?;
            // Once: a second event for the same new order queues nothing.
            let queued: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM hubrise_outbox
                 WHERE hubrise_order_id = ?1 AND body_json LIKE '%\"received\"%')",
                params![order.id],
                |r| r.get(0),
            )?;
            if !queued {
                outbox::enqueue_patch(
                    &conn,
                    local_id,
                    &order.id,
                    &json!({ "status": "received" }),
                )?;
            }
        }
        conn.commit()?;
        ack
    };
    if acknowledge && let Some(wake) = &ctx.outbox_wake {
        wake.notify_one();
    }
    Ok(())
}

/// Fetch the connected catalog and hand it to the handlers.
/// Fetch and apply the connected catalog, if connected.
pub async fn check_catalog(ctx: &PollerCtx) -> Result<Option<ApplyOutcome>> {
    let Some((stored, client)) = load_client(ctx)? else {
        return Ok(None);
    };
    match sync_catalog(ctx, &client, &stored).await {
        Ok(outcome) => {
            if let Some(hook) = &ctx.on_outcome
                && let Some(kind) = catalog_outcome(&outcome)
            {
                hook(kind, synthetic_catalog_event(&stored));
            }
            Ok(Some(outcome))
        }
        Err(e) => Err(note_error(ctx, e)),
    }
}

/// The drift check has no HubRise event; the hook gets one that names
/// the catalog.
fn synthetic_catalog_event(stored: &StoredConnection) -> CallbackEvent {
    let mut extra = serde_json::Map::new();
    extra.insert("catalog_id".into(), stored.catalog_id.clone().into());
    CallbackEvent {
        id: format!("drift-{}", chrono::Utc::now().timestamp()),
        resource_type: "catalog".into(),
        event_type: "update".into(),
        created_at: chrono::Utc::now().to_rfc3339(),
        previous_state: None,
        new_state: None,
        extra,
    }
}

fn catalog_outcome(outcome: &ApplyOutcome) -> Option<DispatchOutcome> {
    match outcome {
        ApplyOutcome::Applied { .. } => Some(DispatchOutcome::CatalogUpdated),
        ApplyOutcome::Rejected { .. } => Some(DispatchOutcome::CatalogRejected),
        ApplyOutcome::AwaitingAdoption => Some(DispatchOutcome::CatalogAwaitingAdoption),
        ApplyOutcome::Unchanged => None,
    }
}

pub async fn sync_catalog(
    ctx: &PollerCtx,
    client: &Client,
    stored: &StoredConnection,
) -> Result<ApplyOutcome> {
    let catalog: Catalog = client
        .get_json(&format!("/catalogs/{}", path_segment(&stored.catalog_id)))
        .await?;
    let mut db = crate::util::lock_db(&ctx.db);
    let outcome = ctx.handlers.on_catalog(&mut db, &catalog)?;
    if matches!(outcome, ApplyOutcome::Applied { .. }) {
        let fixed = crate::webshop::re_resolve_pending(&db)?;
        if fixed > 0 {
            tracing::info!(fixed, "hubrise orders resolved after a catalog change");
        }
    }
    Ok(outcome)
}

/// Pending orders that name products, options or charges the menu
/// does not have may be ahead of the catalog: fetch it again, at most
/// once a minute (the catalog endpoint allows 10 calls a minute).
pub async fn refresh_on_catalog_miss(
    ctx: &PollerCtx,
    client: &Client,
    stored: &StoredConnection,
) -> Result<Option<ApplyOutcome>> {
    let due: bool = {
        let db = crate::util::lock_db(&ctx.db);
        db.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM webshop_orders
                WHERE fiscal_state = 'pending' AND resolution = 'needs_review'
                  AND (attention_json LIKE '%\"unknown_product\"%'
                       OR attention_json LIKE '%\"unknown_option\"%'
                       OR attention_json LIKE '%\"unknown_charge\"%'))
             AND COALESCE(
                (SELECT last_catalog_sync_at <= datetime('now', '-60 seconds')
                 FROM hubrise_connection WHERE id = 1), 1)",
            [],
            |r| r.get(0),
        )?
    };
    if !due {
        return Ok(None);
    }
    sync_catalog(ctx, client, stored).await.map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pos_db::schema::run_migrations;

    #[test]
    fn a_cursor_cycle_ends_the_list() {
        let mut seen = std::collections::HashSet::new();
        // A -> B -> A: the third answer repeats a cursor.
        assert_eq!(
            next_cursor(&mut seen, Some("A".into())).unwrap(),
            Some("A".into())
        );
        assert_eq!(
            next_cursor(&mut seen, Some("B".into())).unwrap(),
            Some("B".into())
        );
        assert_eq!(next_cursor(&mut seen, Some("A".into())).unwrap(), None);
        assert_eq!(next_cursor(&mut seen, None).unwrap(), None);
    }

    #[test]
    fn endless_fresh_cursors_stop_at_the_page_cap() {
        let mut seen = std::collections::HashSet::new();
        let mut pages = 0;
        let outcome = loop {
            match next_cursor(&mut seen, Some(format!("c{pages}"))) {
                Ok(Some(_)) => pages += 1,
                other => break other,
            }
        };
        assert!(outcome.is_err());
        assert_eq!(pages, MAX_PAGES - 1);
    }

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        run_migrations(&c).unwrap();
        c
    }

    fn order(status: &str) -> Order {
        serde_json::from_value(json!({
            "id": "o1", "location_id": "l1", "status": status,
            "created_at": "2026-09-27T12:00:00+02:00", "channel": "Deliveroo",
            "service_type": "delivery", "total": "12.50 EUR"
        }))
        .unwrap()
    }

    #[test]
    fn an_order_is_inserted_once_then_refreshed() {
        let c = conn();
        crate::webshop::ingest(&c, &order("new")).unwrap();
        crate::webshop::ingest(&c, &order("cancelled")).unwrap();
        let (n, status, total, source): (i64, String, i64, String) = c
            .query_row(
                "SELECT COUNT(*), MAX(hubrise_status), MAX(total_cents), MAX(source) FROM orders",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            (n, status.as_str(), total, source.as_str()),
            (1, "cancelled", 1250, "Deliveroo")
        );
    }

    #[test]
    fn an_event_is_parked_after_the_limit() {
        let c = conn();
        let ev: CallbackEvent = serde_json::from_value(json!({
            "id": "e1", "resource_type": "order", "event_type": "create",
            "created_at": "2026-09-27T12:00:00+02:00"
        }))
        .unwrap();
        persist_event(&c, &ev, "{}").unwrap();
        for i in 1..MAX_EVENT_ATTEMPTS {
            assert!(!record_failure(&c, "e1", "boom").unwrap(), "try {i}");
        }
        assert!(record_failure(&c, "e1", "boom").unwrap());
        assert_eq!(event_state(&c, "e1").unwrap(), (false, true));
    }
}
