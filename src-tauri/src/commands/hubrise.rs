//! Tauri command handlers for HubRise integration.
//!
//! These are the typed wrappers invoked by the Vue frontend via
//! `@tauri-apps/api/core`. All methods return `Result<_, String>`
//! for consistent JSON-serialized error display.

use pos_hubrise::{
    client::Client as HubriseClient,
    oauth::{build_authorize_url, exchange_code, OAuthConfig},
    token_store::{self, StoredConnection},
};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

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

/// Configuration defaults for the HubRise OAuth flow on the POS.
/// In production these come from an env file or a Settings screen —
/// for now we ship the dev mock defaults so the flow can be exercised
/// out of the box.
fn default_oauth_config() -> OAuthConfig {
    OAuthConfig {
        oauth_base_url: std::env::var("HUBRISE_OAUTH_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3001/oauth2/v1".to_string()),
        base_url: std::env::var("HUBRISE_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3001/v1".to_string()),
        client_id: std::env::var("HUBRISE_POS_CLIENT_ID")
            .unwrap_or_else(|_| "tsb-pos".to_string()),
        client_secret: std::env::var("HUBRISE_POS_CLIENT_SECRET")
            .unwrap_or_else(|_| String::new()),
        redirect_uri: std::env::var("HUBRISE_POS_REDIRECT_URI")
            .unwrap_or_else(|_| "urn:ietf:wg:oauth:2.0:oob".to_string()),
        scope: std::env::var("HUBRISE_POS_SCOPE").unwrap_or_else(|_| {
            "location[orders.write,customer_list.read,catalog.read]".to_string()
        }),
    }
}

#[tauri::command]
pub fn hubrise_get_connection_status(
    state: State<'_, AppState>,
) -> Result<HubriseConnectionInfo, String> {
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

#[tauri::command]
pub fn hubrise_start_oauth_flow() -> Result<String, String> {
    let cfg = default_oauth_config();
    build_authorize_url(&cfg, None).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn hubrise_complete_oauth(
    state: State<'_, AppState>,
    code: String,
) -> Result<HubriseConnectionInfo, String> {
    let cfg = default_oauth_config();
    let tok = exchange_code(&state.shared_http, &cfg, &code)
        .await
        .map_err(|e| e.to_string())?;

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

    Ok(HubriseConnectionInfo::from_stored(&stored))
}

#[tauri::command]
pub fn hubrise_disconnect(state: State<'_, AppState>) -> Result<(), String> {
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        token_store::clear(&db).map_err(|e| e.to_string())?;
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
    /// wasabi", …) — empty when the item has no modifiers.
    pub options: Vec<String>,
    pub customer_notes: Option<String>,
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
    /// Delivery address fields — used by the "Tokyo webshop"
    /// delivery cards (where TSB's own driver decides whether to
    /// accept or refuse based on distance). Aggregator channels
    /// (Deliveroo/Uber Eats/Takeaway) handle delivery themselves,
    /// so the UI only renders this block for TSB-origin orders.
    pub customer_address_1: Option<String>,
    pub customer_address_2: Option<String>,
    pub customer_postal_code: Option<String>,
    pub customer_city: Option<String>,
}

/// Parse a HubRise money string such as "12.50 EUR" into cents.
/// Mirrors the parser in `pos_hubrise::poller`.
fn money_to_cents(raw: &str) -> i64 {
    let num = raw.split_whitespace().next().unwrap_or("0");
    let parsed: f64 = num.parse().unwrap_or(0.0);
    (parsed * 100.0).round() as i64
}

/// Address fields teased out of a HubRise `customer` object — only
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
                .and_then(|s| s.parse::<i64>().ok())
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
        if joined.is_empty() { None } else { Some(joined) }
    });
    let confirmed_time = value
        .get("confirmed_time")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let address = value
        .get("customer")
        .map(|c| CustomerAddress {
            address_1: c.get("address_1").and_then(|v| v.as_str()).map(String::from),
            address_2: c.get("address_2").and_then(|v| v.as_str()).map(String::from),
            postal_code: c.get("postal_code").and_then(|v| v.as_str()).map(String::from),
            city: c.get("city").and_then(|v| v.as_str()).map(String::from),
        })
        .unwrap_or_default();

    (items, customer_notes, customer_name, confirmed_time, address)
}

#[tauri::command]
pub fn hubrise_list_webshop_orders(
    state: State<'_, AppState>,
) -> Result<Vec<WebshopOrderSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, hubrise_order_id, status, hubrise_status, source, service_type, total_cents, created_at, raw_hubrise_json
             FROM orders
             WHERE is_fiscal = 0
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
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[derive(Debug, Deserialize)]
pub struct PatchStatusRequest {
    pub hubrise_order_id: String,
    pub new_status: String,
}

#[tauri::command]
pub async fn hubrise_patch_order_status(
    state: State<'_, AppState>,
    req: PatchStatusRequest,
) -> Result<(), String> {
    let stored = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        token_store::load(&db).map_err(|e| e.to_string())?
    };
    let Some(stored) = stored else {
        return Err("not connected to HubRise".to_string());
    };

    let client = HubriseClient::new(
        state.shared_http.clone(),
        stored.base_url.clone(),
        stored.access_token.clone(),
    );
    pos_hubrise::push::patch_order_status(
        &client,
        &stored.location_id,
        &req.hubrise_order_id,
        &req.new_status,
    )
    .await
    .map_err(|e| e.to_string())?;

    // Mirror the new status on the local row.
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE orders SET hubrise_status = ?1 WHERE hubrise_order_id = ?2",
        rusqlite::params![req.new_status, req.hubrise_order_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct SetConfirmedTimeRequest {
    pub hubrise_order_id: String,
    /// RFC3339 UTC timestamp ("2026-04-11T11:30:00Z").
    pub confirmed_time: String,
}

/// Push a new `confirmed_time` to HubRise and patch the local raw
/// JSON blob so the UI sees the change without waiting for the next
/// poller tick.
#[tauri::command]
pub async fn hubrise_set_confirmed_time(
    state: State<'_, AppState>,
    req: SetConfirmedTimeRequest,
) -> Result<(), String> {
    let stored = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        token_store::load(&db).map_err(|e| e.to_string())?
    };
    let Some(stored) = stored else {
        return Err("not connected to HubRise".to_string());
    };

    let client = HubriseClient::new(
        state.shared_http.clone(),
        stored.base_url.clone(),
        stored.access_token.clone(),
    );
    pos_hubrise::push::patch_order_confirmed_time(
        &client,
        &stored.location_id,
        &req.hubrise_order_id,
        &req.confirmed_time,
    )
    .await
    .map_err(|e| e.to_string())?;

    // Mirror confirmed_time into the local raw_hubrise_json blob so
    // the next list query returns the new value. We patch only that
    // single field rather than re-fetching the order.
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let raw: Option<String> = db
        .query_row(
            "SELECT raw_hubrise_json FROM orders WHERE hubrise_order_id = ?1",
            rusqlite::params![req.hubrise_order_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if let Some(raw) = raw
        && let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&raw)
    {
        if let Some(obj) = value.as_object_mut() {
            obj.insert(
                "confirmed_time".to_string(),
                serde_json::Value::String(req.confirmed_time.clone()),
            );
        }
        if let Ok(updated) = serde_json::to_string(&value) {
            db.execute(
                "UPDATE orders SET raw_hubrise_json = ?1 WHERE hubrise_order_id = ?2",
                rusqlite::params![updated, req.hubrise_order_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
