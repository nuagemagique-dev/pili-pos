//! Engine functions behind the HubRise integration commands.
//!
//! The IPC wrappers in `crate::ipc::hubrise` call these on behalf of
//! the Vue frontend. All methods return `Result<_, String>` for
//! consistent JSON-serialized error display.

use std::sync::Arc;

use pos_hubrise::{
    oauth::{OAuthConfig, build_authorize_url, exchange_code},
    outbox,
    poller::{self, PollerCtx},
    token_store::{self, StoredConnection},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

/// The poller's context over the app state: the loop in `main.rs`
/// and the commands that start or wake it share it.
pub fn poller_ctx(state: &AppState) -> PollerCtx {
    PollerCtx {
        http: state.shared_http.clone(),
        db: state.poller_db.clone(),
        on_outcome: None,
        handlers: Arc::new(poller::DefaultHandlers),
        poll_now: state.hubrise_poll_now.clone(),
        outbox_wake: Some(state.hubrise_outbox_wake.clone()),
        enabled: state.hubrise_switch.clone(),
    }
}

/// What the frontend sees about the current HubRise connection.
#[derive(Debug, Clone, Serialize)]
pub struct HubriseConnectionInfo {
    pub connected: bool,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub location_id: Option<String>,
    pub location_name: Option<String>,
    pub catalog_id: Option<String>,
    pub catalog_name: Option<String>,
    pub customer_list_id: Option<String>,
    pub customer_list_name: Option<String>,
}

impl HubriseConnectionInfo {
    fn from_stored(stored: &StoredConnection) -> Self {
        Self {
            connected: true,
            account_id: Some(stored.account_id.clone()),
            account_name: Some(stored.account_name.clone()),
            location_id: Some(stored.location_id.clone()),
            location_name: Some(stored.location_name.clone()),
            catalog_id: Some(stored.catalog_id.clone()),
            catalog_name: stored.catalog_name.clone(),
            customer_list_id: stored.customer_list_id.clone(),
            customer_list_name: stored.customer_list_name.clone(),
        }
    }
}

/// The installation's HubRise OAuth client: the `hubrise` section of
/// `pos.json`, its secret read from `client_secret_file`. Without the
/// section, a development build (`dev_fallback`) takes the
/// `HUBRISE_*` variables (from `.env`), else the local mock; a release
/// build refuses (`hubrise_not_configured`).
pub fn oauth_config(
    config: &pos_core::config::PosConfig,
    dev_fallback: bool,
) -> Result<OAuthConfig, String> {
    if let Some(h) = &config.hubrise {
        let client_secret = h.client_secret().map_err(|problem| {
            tracing::error!(%problem, "pos.json: HubRise client secret");
            errors::hubrise::SECRET_UNREADABLE.to_string()
        })?;
        return Ok(OAuthConfig {
            oauth_base_url: h.oauth_base_url.clone(),
            base_url: h.api_base_url.clone(),
            client_id: h.client_id.clone(),
            client_secret,
            redirect_uri: h.redirect_uri.clone(),
            scope: h.scope.clone(),
        });
    }
    if !dev_fallback {
        return Err(errors::hubrise::NOT_CONFIGURED.to_string());
    }
    let env = |key: &str, default: &str| std::env::var(key).unwrap_or_else(|_| default.to_string());
    Ok(OAuthConfig {
        oauth_base_url: env("HUBRISE_OAUTH_BASE_URL", "http://localhost:3001/oauth2/v1"),
        base_url: env("HUBRISE_BASE_URL", "http://localhost:3001/v1"),
        client_id: env("HUBRISE_POS_CLIENT_ID", "pili-pos"),
        client_secret: env("HUBRISE_POS_CLIENT_SECRET", ""),
        redirect_uri: env("HUBRISE_POS_REDIRECT_URI", "urn:ietf:wg:oauth:2.0:oob"),
        scope: env(
            "HUBRISE_POS_SCOPE",
            "location[orders.write,customer_list.read,catalog.read]",
        ),
    })
}

/// [`oauth_config`] for this build: the development fallback only in
/// debug builds.
fn installation_oauth_config(state: &AppState) -> Result<OAuthConfig, String> {
    let config = state.config.lock().map_err(errors::internal)?;
    oauth_config(&config, cfg!(debug_assertions))
}

/// HubRise refused the connection code or did not answer: said in the
/// screen's language, its own words kept as the detail.
fn oauth_failed(e: impl std::fmt::Display) -> String {
    tracing::warn!(error = %e, "hubrise: connection refused");
    errors::coded(errors::hubrise::OAUTH_FAILED, &[("detail", &e)])
}

/// No operator check on purpose: both login screens read it before
/// anyone signs in (start-up checks). It holds no customer data.
pub fn do_hubrise_get_connection_status(state: &AppState) -> Result<HubriseConnectionInfo, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    match token_store::load(&db).map_err(|e| e.to_string())? {
        Some(stored) => Ok(HubriseConnectionInfo::from_stored(&stored)),
        None => Ok(HubriseConnectionInfo {
            connected: false,
            account_id: None,
            account_name: None,
            location_id: None,
            location_name: None,
            catalog_id: None,
            catalog_name: None,
            customer_list_id: None,
            customer_list_name: None,
        }),
    }
}

/// The HubRise page where the installer logs in and grants access,
/// and the same link as a QR code: the kiosk has no browser, so the
/// installer opens it on a phone and types back the code HubRise shows.
#[derive(Debug, Clone, Serialize)]
pub struct AuthorizeLink {
    pub url: String,
    /// SVG markup, rendered here from `url` only.
    pub qr_svg: String,
}

/// Admins only, like every other connection setting.
pub fn do_hubrise_start_oauth_flow(state: &AppState) -> Result<AuthorizeLink, String> {
    crate::commands::integrations::require_hubrise(state)?;
    access::operator(state, Access::Admin)?;
    let cfg = installation_oauth_config(state)?;
    let url = build_authorize_url(&cfg, None).map_err(oauth_failed)?;
    let qr_svg = qr_svg(&url)?;
    Ok(AuthorizeLink { url, qr_svg })
}

/// `text` as an SVG QR code (ECC M), black on white with its quiet
/// zone, without the XML declaration so it can sit inline in the page.
fn qr_svg(text: &str) -> Result<String, String> {
    use qrcode::render::svg;
    let code = qrcode::QrCode::with_error_correction_level(text, qrcode::EcLevel::M)
        .map_err(errors::internal)?;
    let svg = code
        .render::<svg::Color>()
        .min_dimensions(240, 240)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();
    Ok(match svg.find("<svg") {
        Some(start) => svg[start..].to_string(),
        None => svg,
    })
}

/// Exchange the code HubRise showed for this location's token, store
/// the connection, and start the integration. Admins only.
pub async fn do_hubrise_complete_oauth(
    state: &AppState,
    code: &str,
) -> Result<HubriseConnectionInfo, String> {
    crate::commands::integrations::require_hubrise(state)?;
    access::operator(state, Access::Admin)?;
    let cfg = installation_oauth_config(state)?;
    let tok = exchange_code(&state.shared_http, &cfg, code)
        .await
        .map_err(oauth_failed)?;

    let stored = StoredConnection {
        access_token: tok.access_token.clone(),
        account_id: tok.account_id.clone(),
        account_name: tok.account_name.clone(),
        location_id: tok.location_id.clone(),
        location_name: tok.location_name.clone(),
        catalog_id: tok.catalog_id.clone().unwrap_or_default(),
        catalog_name: tok.catalog_name.clone(),
        customer_list_id: tok.customer_list_id.clone(),
        customer_list_name: tok.customer_list_name.clone(),
        scope: cfg.scope.clone(),
        base_url: cfg.base_url.clone(),
        last_event_cursor: None,
    };

    // Persist via the poller_db connection so the background loop
    // picks up the change on its next tick without needing to acquire
    // the UI db mutex.
    {
        let db = state.poller_db.lock().map_err(|e| e.to_string())?;
        token_store::save(&db, &stored).map_err(|e| e.to_string())?;
    }
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        token_store::save(&db, &stored).map_err(|e| e.to_string())?;
    }

    // Register the passive callback and fetch what HubRise holds now,
    // without waiting for the poller's next tick. A failure here is
    // retried by the poller; the connection itself is kept.
    if let Err(e) = poller::start(&poller_ctx(state)).await {
        tracing::warn!(%e, "hubrise: first start after connecting failed");
    }
    state.hubrise_poll_now.notify_one();

    Ok(HubriseConnectionInfo::from_stored(&stored))
}

/// Forget the connection. Admins only, and like switching the
/// integration off, refused while online orders are still to settle.
pub fn do_hubrise_disconnect(state: &AppState) -> Result<(), String> {
    access::operator(state, Access::Admin)?;
    let open = crate::commands::integrations::unsettled(state)?;
    if open > 0 {
        return Err(errors::coded(
            errors::hubrise::ONLINE_ORDERS_OPEN,
            &[("count", &open)],
        ));
    }
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        token_store::clear(&db).map_err(|e| e.to_string())?;
        // The menu stays as last synced, and can be edited again.
        pos_catalog::release(&db, pos_hubrise::CATALOG_SOURCE).map_err(errors::internal)?;
    }
    {
        let db = state.poller_db.lock().map_err(|e| e.to_string())?;
        token_store::clear(&db).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// A single line inside a webshop order, flattened for the UI.
#[derive(Debug, Clone, Serialize)]
pub struct WebshopOrderItem {
    pub product_name: String,
    pub quantity: i64,
    pub unit_price_cents: i64,
    pub line_total_cents: i64,
    /// Human-readable list of applied options ("Sauce soja", "Extra
    /// wasabi", …) - empty when the item has no modifiers.
    pub options: Vec<String>,
    pub customer_notes: Option<String>,
    /// Chinese name of the menu product whose French name matches
    /// `product_name`, when there is one: the kitchen staff read
    /// Chinese first (UX guide §10).
    pub product_name_zh: Option<String>,
}

/// Summary of a webshop order kept locally (is_fiscal = 0).
///
/// Items + customer metadata are extracted from `raw_hubrise_json`
/// at query time so the Vue view can render the full order without
/// a second IPC round-trip.
#[derive(Debug, Clone, Serialize)]
pub struct WebshopOrderSummary {
    pub id: i64,
    pub hubrise_order_id: Option<String>,
    pub status: String,
    pub hubrise_status: Option<String>,
    pub source: String,
    pub service_type: Option<String>,
    pub total_cents: i64,
    pub created_at: String,
    pub items: Vec<WebshopOrderItem>,
    pub customer_notes: Option<String>,
    pub customer_name: Option<String>,
    /// HubRise `confirmed_time` (RFC3339). Populated by the
    /// "+15/+30/+45/+60 min" delay chips so the staff sees the
    /// agreed-upon ready-at time on the card.
    pub confirmed_time: Option<String>,
    /// Delivery address fields - used by the "Tokyo webshop"
    /// delivery cards (where TSB's own driver decides whether to
    /// accept or refuse based on distance). Aggregator channels
    /// (Deliveroo/Uber Eats/Takeaway) handle delivery themselves,
    /// so the UI only renders this block for TSB-origin orders.
    pub customer_address_1: Option<String>,
    pub customer_address_2: Option<String>,
    pub customer_postal_code: Option<String>,
    pub customer_city: Option<String>,
    /// How the order resolved against the menu and where it stands
    /// fiscally; `None` for rows stored before migration 22.
    pub webshop: Option<pos_hubrise::webshop::WebshopState>,
}

/// Parse a HubRise money string such as "12.50 EUR" into cents.
fn money_to_cents(raw: &str) -> i64 {
    pos_hubrise_types::money::parse_cents(raw).unwrap_or(0)
}

/// Address fields teased out of a HubRise `customer` object - only
/// populated for `delivery` service_type orders where the Vue card
/// needs to render a per-order delivery block.
#[derive(Debug, Default, Clone)]
struct CustomerAddress {
    address_1: Option<String>,
    address_2: Option<String>,
    postal_code: Option<String>,
    city: Option<String>,
}

/// Best-effort extraction of items + customer fields + ready-at
/// time from a stored HubRise Order JSON blob. Any parse error
/// degrades to empty/None defaults so the card still renders.
fn summary_from_raw(
    raw: Option<String>,
) -> (
    Vec<WebshopOrderItem>,
    Option<String>,
    Option<String>,
    Option<String>,
    CustomerAddress,
) {
    let Some(raw) = raw else {
        return (Vec::new(), None, None, None, CustomerAddress::default());
    };
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return (Vec::new(), None, None, None, CustomerAddress::default()),
    };

    let mut items: Vec<WebshopOrderItem> = Vec::new();
    if let Some(arr) = value.get("items").and_then(|v| v.as_array()) {
        for raw_item in arr {
            let product_name = raw_item
                .get("product_name")
                .and_then(|v| v.as_str())
                .unwrap_or("(item)")
                .to_string();
            // HubRise encodes quantity as a string, fall back to 1.
            let quantity = raw_item
                .get("quantity")
                .and_then(|v| v.as_str())
                .and_then(pos_hubrise_types::money::parse_quantity)
                .unwrap_or(1);
            let unit_price_cents = raw_item
                .get("price")
                .and_then(|v| v.as_str())
                .map(money_to_cents)
                .unwrap_or(0);
            let line_total_cents = unit_price_cents * quantity;
            let mut options: Vec<String> = Vec::new();
            if let Some(opts) = raw_item.get("options").and_then(|v| v.as_array()) {
                for opt in opts {
                    if let Some(name) = opt.get("name").and_then(|v| v.as_str()) {
                        options.push(name.to_string());
                    }
                }
            }
            let customer_notes = raw_item
                .get("customer_notes")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            items.push(WebshopOrderItem {
                product_name,
                quantity,
                unit_price_cents,
                line_total_cents,
                options,
                customer_notes,
                product_name_zh: None,
            });
        }
    }

    let customer_notes = value
        .get("customer_notes")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let customer_name = value.get("customer").and_then(|c| {
        let first = c.get("first_name").and_then(|v| v.as_str()).unwrap_or("");
        let last = c.get("last_name").and_then(|v| v.as_str()).unwrap_or("");
        let joined = format!("{first} {last}").trim().to_string();
        if joined.is_empty() {
            None
        } else {
            Some(joined)
        }
    });
    let confirmed_time = value
        .get("confirmed_time")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let address = value
        .get("customer")
        .map(|c| CustomerAddress {
            address_1: c
                .get("address_1")
                .and_then(|v| v.as_str())
                .map(String::from),
            address_2: c
                .get("address_2")
                .and_then(|v| v.as_str())
                .map(String::from),
            postal_code: c
                .get("postal_code")
                .and_then(|v| v.as_str())
                .map(String::from),
            city: c.get("city").and_then(|v| v.as_str()).map(String::from),
        })
        .unwrap_or_default();

    (
        items,
        customer_notes,
        customer_name,
        confirmed_time,
        address,
    )
}

/// No operator check on purpose: the Android order board loads the
/// orders as soon as the engine is ready, before anyone signs in, so a
/// new online order is seen (and its chime heard) on a locked till.
/// Read only.
pub fn do_list_webshop_orders(state: &AppState) -> Result<Vec<WebshopOrderSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, hubrise_order_id, status, hubrise_status, source, service_type, total_cents, created_at, raw_hubrise_json
             FROM orders
             WHERE hubrise_order_id IS NOT NULL
             ORDER BY created_at DESC
             LIMIT 200",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let raw_json: Option<String> = row.get(8)?;
            let (items, customer_notes, customer_name, confirmed_time, address) =
                summary_from_raw(raw_json);
            Ok(WebshopOrderSummary {
                id: row.get(0)?,
                hubrise_order_id: row.get(1)?,
                status: row.get(2)?,
                hubrise_status: row.get(3)?,
                source: row.get(4)?,
                service_type: row.get(5)?,
                total_cents: row.get(6)?,
                created_at: row.get(7)?,
                items,
                customer_notes,
                customer_name,
                confirmed_time,
                customer_address_1: address.address_1,
                customer_address_2: address.address_2,
                customer_postal_code: address.postal_code,
                customer_city: address.city,
                webshop: None,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    for order in out.iter_mut() {
        order.webshop = pos_hubrise::webshop::load(&db, order.id).map_err(errors::internal)?;
    }
    let chinese = chinese_names_by_french_name(&db).map_err(|e| e.to_string())?;
    for item in out.iter_mut().flat_map(|o| o.items.iter_mut()) {
        item.product_name_zh = chinese.get(&item.product_name.to_lowercase()).cloned();
    }
    Ok(out)
}

/// Menu products' Chinese names, keyed by their lower-cased French name.
fn chinese_names_by_french_name(
    db: &rusqlite::Connection,
) -> Result<std::collections::HashMap<String, String>, rusqlite::Error> {
    let mut stmt = db.prepare(
        "SELECT fr.value, zh.value
         FROM translations fr
         JOIN translations zh
           ON zh.entity_type = 'menu_item' AND zh.entity_id = fr.entity_id
              AND zh.field = 'name' AND zh.lang = 'zh'
         WHERE fr.entity_type = 'menu_item' AND fr.field = 'name' AND fr.lang = 'fr'",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?.to_lowercase(),
            row.get::<_, String>(1)?,
        ))
    })?;
    rows.collect()
}

#[derive(Debug, Deserialize)]
pub struct PatchStatusRequest {
    pub hubrise_order_id: String,
    pub new_status: String,
}

/// The statuses staff set from the board. `accepted`, `rejected` and
/// `completed` belong to the commands that sign (or refuse to sign)
/// the order, never to a bare status change.
pub const STAFF_STATUSES: &[&str] = &[
    "in_preparation",
    "awaiting_shipment",
    "awaiting_collection",
    "in_delivery",
];

/// Queue a status change for HubRise and show it at once on the POS.
pub fn do_patch_order_status(
    state: &AppState,
    hubrise_order_id: &str,
    new_status: &str,
) -> Result<(), String> {
    crate::commands::integrations::require_hubrise(state)?;
    access::operator(state, Access::Staff)?;
    if !STAFF_STATUSES.contains(&new_status) {
        return Err(errors::hubrise::STATUS_NOT_ALLOWED.to_string());
    }
    queue_patch(
        state,
        hubrise_order_id,
        &json!({ "status": new_status }),
        |raw| raw["status"] = json!(new_status),
    )?;
    let db = state.db.lock().map_err(errors::internal)?;
    db.execute(
        "UPDATE orders SET hubrise_status = ?1 WHERE hubrise_order_id = ?2",
        rusqlite::params![new_status, hubrise_order_id],
    )
    .map_err(errors::internal)?;
    Ok(())
}

/// Queue `patch` for HubRise order `hubrise_order_id` and apply
/// `mirror` to the stored order, in one transaction; wake the outbox.
pub fn queue_patch(
    state: &AppState,
    hubrise_order_id: &str,
    patch: &serde_json::Value,
    mirror: impl FnOnce(&mut serde_json::Value),
) -> Result<(), String> {
    let db = state.db.lock().map_err(errors::internal)?;
    if token_store::load(&db).map_err(errors::internal)?.is_none() {
        return Err(errors::hubrise::NOT_CONNECTED.to_string());
    }
    let tx = db.unchecked_transaction().map_err(errors::internal)?;
    let row: Option<(i64, Option<String>)> = tx
        .query_row(
            "SELECT id, raw_hubrise_json FROM orders WHERE hubrise_order_id = ?1",
            rusqlite::params![hubrise_order_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    let Some((order_id, raw)) = row else {
        return Err(errors::hubrise::ORDER_UNKNOWN.to_string());
    };
    outbox::enqueue_patch(&tx, order_id, hubrise_order_id, patch).map_err(errors::internal)?;
    if let Some(mut value) = raw.and_then(|r| serde_json::from_str::<serde_json::Value>(&r).ok()) {
        mirror(&mut value);
        tx.execute(
            "UPDATE orders SET raw_hubrise_json = ?1 WHERE id = ?2",
            rusqlite::params![value.to_string(), order_id],
        )
        .map_err(errors::internal)?;
    }
    tx.commit().map_err(errors::internal)?;
    state.hubrise_outbox_wake.notify_one();
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct SetConfirmedTimeRequest {
    pub hubrise_order_id: String,
    /// RFC3339 UTC timestamp ("2026-04-11T11:30:00Z").
    pub confirmed_time: String,
}

/// Queue a new `confirmed_time` for HubRise and show it at once.
pub fn do_set_confirmed_time(
    state: &AppState,
    hubrise_order_id: &str,
    confirmed_time: &str,
) -> Result<(), String> {
    crate::commands::integrations::require_hubrise(state)?;
    access::operator(state, Access::Staff)?;
    if chrono::DateTime::parse_from_rfc3339(confirmed_time).is_err() {
        return Err(errors::hubrise::TIME_INVALID.to_string());
    }
    queue_patch(
        state,
        hubrise_order_id,
        &json!({ "confirmed_time": confirmed_time }),
        |raw| raw["confirmed_time"] = json!(confirmed_time),
    )
}

/// What the status lights and the settings panel show.
#[derive(Debug, Clone, Serialize)]
pub struct HubriseStatus {
    #[serde(flatten)]
    pub health: token_store::ConnectionHealth,
    pub outbox: outbox::Backlog,
    pub outbox_error: Option<String>,
    /// Events HubRise sent that could not be handled.
    pub parked_events: i64,
    /// Online orders still waiting for staff after 5 minutes.
    pub stale_orders: i64,
    /// The last catalog sync: when, and the issues of a refused one.
    pub catalog: Option<pos_catalog::SyncState>,
    /// A first catalog waits for the manager to adopt local products.
    pub catalog_awaiting_adoption: bool,
}

/// No operator check on purpose: the status lights read it on every
/// screen, the login screen included. Counts only, no customer data.
pub fn do_hubrise_status(state: &AppState) -> Result<HubriseStatus, String> {
    let db = state.db.lock().map_err(errors::internal)?;
    let health = token_store::health(&db).map_err(errors::internal)?;
    let outbox = outbox::backlog(&db).map_err(errors::internal)?;
    let outbox_error = outbox::last_error(&db).map_err(errors::internal)?;
    let parked_events: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM hubrise_events WHERE parked_at IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .map_err(errors::internal)?;
    let stale_orders: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM orders
             WHERE hubrise_order_id IS NOT NULL AND status = 'open'
               AND hubrise_status IN ('new', 'received')
               AND created_at <= datetime('now', '-5 minutes')",
            [],
            |r| r.get(0),
        )
        .map_err(errors::internal)?;
    let catalog = pos_catalog::state(&db, pos_hubrise::CATALOG_SOURCE).map_err(errors::internal)?;
    let catalog_awaiting_adoption = pos_catalog::pending_catalog(&db, pos_hubrise::CATALOG_SOURCE)
        .map_err(errors::internal)?
        .is_some();
    Ok(HubriseStatus {
        health,
        outbox,
        outbox_error,
        parked_events,
        stale_orders,
        catalog,
        catalog_awaiting_adoption,
    })
}

/// Fetch and apply the HubRise catalog now (Settings, "Resynchroniser").
pub async fn do_sync_catalog(
    state: &AppState,
) -> Result<Option<pos_catalog::ApplyOutcome>, String> {
    crate::commands::integrations::require_hubrise(state)?;
    access::operator(state, Access::Staff)?;
    let ctx = poller_ctx(state);
    let outcome = poller::check_catalog(&ctx)
        .await
        .map_err(errors::internal)?;
    if outcome.is_none() {
        return Err(errors::hubrise::NOT_CONNECTED.to_string());
    }
    Ok(outcome)
}

/// The local products that can keep their id when HubRise takes over
/// the menu (same French name and price as a catalog product).
pub fn do_adoption_preview(state: &AppState) -> Result<Vec<pos_catalog::Adoption>, String> {
    access::operator(state, Access::Admin)?;
    let db = state.db.lock().map_err(errors::internal)?;
    let Some(pending) =
        pos_catalog::pending_catalog(&db, pos_hubrise::CATALOG_SOURCE).map_err(errors::internal)?
    else {
        return Err(errors::hubrise::NOTHING_PENDING.to_string());
    };
    pos_catalog::adoption_preview(&db, &pending).map_err(errors::internal)
}

/// The manager confirms the take-over: the chosen products keep their
/// id, every other hand-entered product is deactivated, and the menu
/// mirrors HubRise. Refused while a POS order is open, since its
/// products could vanish from the menu.
pub fn do_adopt_catalog(
    state: &AppState,
    adoptions: &[pos_catalog::Adoption],
) -> Result<pos_catalog::ApplyOutcome, String> {
    crate::commands::integrations::require_hubrise(state)?;
    access::operator(state, Access::Admin)?;
    let mut db = state.db.lock().map_err(errors::internal)?;
    let open: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM orders WHERE status = 'open' AND hubrise_order_id IS NULL",
            [],
            |r| r.get(0),
        )
        .map_err(errors::internal)?;
    if open > 0 {
        return Err(errors::hubrise::OPEN_ORDERS.to_string());
    }
    pos_catalog::adopt_and_apply(&mut db, pos_hubrise::CATALOG_SOURCE, adoptions)
        .map_err(errors::internal)?
        .ok_or_else(|| errors::hubrise::NOTHING_PENDING.to_string())
}

/// How each HubRise channel is signed (Settings).
pub fn do_list_channels(state: &AppState) -> Result<Vec<pos_hubrise::webshop::Channel>, String> {
    access::operator(state, Access::Admin)?;
    let db = state.db.lock().map_err(errors::internal)?;
    pos_hubrise::webshop::list_channels(&db).map_err(errors::internal)
}

/// Add or change a channel; pending orders resolve again. Returns how
/// many of them now resolve cleanly.
pub fn do_set_channel(
    state: &AppState,
    channel: &pos_hubrise::webshop::Channel,
) -> Result<usize, String> {
    crate::commands::integrations::require_hubrise(state)?;
    access::operator(state, Access::Admin)?;
    let valid = !channel.channel.trim().is_empty()
        && ["platform", "webshop"].contains(&channel.kind.as_str())
        && ["APP", "ONLINE"].contains(&channel.payment_type.as_str())
        && ["include", "exclude"].contains(&channel.charge_policy.as_str());
    if !valid {
        return Err(errors::hubrise::CHANNEL_INVALID.to_string());
    }
    let db = state.db.lock().map_err(errors::internal)?;
    pos_hubrise::webshop::set_channel(&db, channel).map_err(errors::internal)
}
