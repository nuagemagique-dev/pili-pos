//! HubRise mock HTTP server.
//!
//! Minimal in-memory implementation of the HubRise REST API used by
//! `tsb-service` (webshop client) and `moyun-pos` (POS client) during
//! development — no real HubRise account required.
//!
//! Docs revision baseline: see `HUBRISE_DOCS_REVISION`. Update this
//! constant every time the extracted docs under `docs/hubrise/` are
//! refreshed to track mock/reality drift.
//!
//! Run with: `cargo run -p pos-hubrise-mock`
//! Default port: 3001 (override with `HUBRISE_MOCK_PORT`).

#![allow(clippy::result_large_err)]

mod state;

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use chrono::Utc;
use pos_hubrise_types::{
    CallbackEvent, CallbackRegistration, Catalog, CatalogData, ErrorResponse, ErrorType,
    OAuthTokenResponse, Order, OrderStatus,
};
use serde::Deserialize;
use serde_json::{Value, json};
use state::{ClientState, MockState};
use uuid::Uuid;

/// Version of the HubRise docs this mock targets. Bump whenever
/// `docs/hubrise/*.md` is refreshed to surface drift during review.
pub const HUBRISE_DOCS_REVISION: &str = "2026-04-10";
/// URL of the official docs consulted to build this mock.
pub const HUBRISE_DOCS_URL: &str = "https://www.hubrise.com/developers/api/";

type AppState = Arc<MockState>;

#[tokio::main]
async fn main() {
    let port = std::env::var("HUBRISE_MOCK_PORT").unwrap_or_else(|_| "3001".to_string());
    let addr = format!("0.0.0.0:{port}");

    let state: AppState = Arc::new(MockState::new_seeded());

    let app = Router::new()
        // OAuth 2.0
        .route("/oauth2/v1/authorize", get(oauth_authorize))
        .route("/oauth2/v1/token", post(oauth_token))
        .route("/oauth2/v1/revoke", post(oauth_revoke))
        // Orders
        .route(
            "/v1/locations/:location_id/orders",
            post(create_order).get(list_orders),
        )
        .route(
            "/v1/location/orders",
            post(create_order_short).get(list_orders_short),
        )
        .route(
            "/v1/locations/:location_id/orders/:order_id",
            get(retrieve_order).patch(update_order),
        )
        .route(
            "/v1/location/orders/:order_id",
            get(retrieve_order_short).patch(update_order_short),
        )
        // Catalogs
        .route(
            "/v1/location/catalogs",
            get(list_catalogs).post(create_catalog),
        )
        .route(
            "/v1/locations/:location_id/catalogs",
            get(list_catalogs_by_loc).post(create_catalog_by_loc),
        )
        .route(
            "/v1/catalogs/:catalog_id",
            get(retrieve_catalog)
                .put(replace_catalog)
                .delete(delete_catalog),
        )
        // Callbacks
        .route(
            "/v1/callback",
            get(get_callback)
                .post(register_callback)
                .delete(delete_callback),
        )
        .route("/v1/callback/events", get(list_callback_events))
        .route(
            "/v1/callback/events/:event_id",
            delete(delete_callback_event),
        )
        // Location
        .route("/v1/location", get(get_location))
        // Health
        .route("/", get(health))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind");
    println!("HubRise mock listening on http://localhost:{port}");
    println!("Docs revision baseline: {HUBRISE_DOCS_REVISION}");
    println!("Pre-seeded clients:");
    println!("  - tsb-webshop  token=dev-token-webshop");
    println!("  - moyun-pos    token=dev-token-pos");
    println!("Press Ctrl+C to stop");

    axum::serve(listener, app).await.expect("axum serve");
}

async fn health() -> &'static str {
    "ok"
}

// ── Helpers ──────────────────────────────────────────────────────────

fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("X-Access-Token")
        .or_else(|| headers.get("x-access-token"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn unauthorized() -> Response {
    let body = ErrorResponse::new(ErrorType::Unauthorized, "The connection has been blocked");
    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}

fn not_found(msg: &str) -> Response {
    let body = ErrorResponse::new(ErrorType::NotFound, msg);
    (StatusCode::NOT_FOUND, Json(body)).into_response()
}

fn unprocessable(msg: &str) -> Response {
    let body = ErrorResponse::new(ErrorType::UnprocessableEntity, msg);
    (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
}

fn require_client(headers: &HeaderMap, state: &AppState) -> Result<String, Response> {
    let token = extract_token(headers).ok_or_else(unauthorized)?;
    let inner = state.inner.lock().expect("poisoned");
    if !inner.clients.contains_key(&token) {
        return Err(unauthorized());
    }
    Ok(token)
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

fn make_event(resource_type: &str, event_type: &str, extra: Value) -> CallbackEvent {
    let mut map = serde_json::Map::new();
    if let Value::Object(m) = extra {
        map = m;
    }
    CallbackEvent {
        id: Uuid::new_v4().to_string(),
        resource_type: resource_type.to_string(),
        event_type: event_type.to_string(),
        created_at: now_rfc3339(),
        previous_state: None,
        new_state: None,
        extra: map,
    }
}

// ── OAuth ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AuthorizeQuery {
    #[serde(default)]
    redirect_uri: Option<String>,
    #[serde(default)]
    client_id: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    state: Option<String>,
}

/// `GET /oauth2/v1/authorize` — returns a JSON response with an
/// authorization code instead of a real 302 redirect. The dev client
/// reads the code and POSTs it to `/oauth2/v1/token`.
async fn oauth_authorize(
    State(state): State<AppState>,
    Query(q): Query<AuthorizeQuery>,
) -> Response {
    let code = Uuid::new_v4().to_string();

    let token = match q.client_id.as_deref() {
        Some("moyun-pos") | Some("moyun-pos-dev") => "dev-token-pos",
        _ => "dev-token-webshop",
    }
    .to_string();

    {
        let mut inner = state.inner.lock().expect("poisoned");
        inner.pending_auth_codes.insert(code.clone(), token);
    }

    Json(json!({
        "code": code,
        "state": q.state,
        "redirect_uri": q.redirect_uri,
        "scope": q.scope,
        "note": "Mock HubRise: no real redirect. POST the code to /oauth2/v1/token."
    }))
    .into_response()
}

#[derive(Deserialize)]
struct TokenRequest {
    code: String,
}

async fn oauth_token(State(state): State<AppState>, Json(body): Json<TokenRequest>) -> Response {
    let inner = state.inner.lock().expect("poisoned");
    let token = match inner.pending_auth_codes.get(&body.code) {
        Some(t) => t.clone(),
        None => return unprocessable("invalid or expired code"),
    };
    let client = match inner.clients.get(&token) {
        Some(c) => c,
        None => return unprocessable("no client for code"),
    };

    let resp = OAuthTokenResponse {
        access_token: token.clone(),
        token_type: "Bearer".to_string(),
        account_id: inner.default_account_id.clone(),
        location_id: inner.default_location_id.clone(),
        catalog_id: Some(inner.default_catalog_id.clone()),
        customer_list_id: Some(inner.default_customer_list_id.clone()),
        account_name: client.account_name.clone(),
        location_name: client.location_name.clone(),
        catalog_name: Some(client.catalog_name.clone()),
        customer_list_name: Some(client.customer_list_name.clone()),
    };
    Json(resp).into_response()
}

async fn oauth_revoke() -> Response {
    // No-op: we never actually revoke in the mock.
    (StatusCode::OK, Json(json!({"status": "revoked"}))).into_response()
}

// ── Orders ───────────────────────────────────────────────────────────

async fn create_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(location_id): Path<String>,
    Json(mut body): Json<Value>,
) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };

    let order_id = Uuid::new_v4().to_string();
    body["id"] = json!(order_id.clone());
    body["location_id"] = json!(location_id);
    body["created_at"] = json!(now_rfc3339());

    // Store a structured copy.
    let order: Order = match serde_json::from_value(body.clone()) {
        Ok(o) => o,
        Err(e) => return unprocessable(&format!("invalid order body: {e}")),
    };

    {
        let mut inner = state.inner.lock().expect("poisoned");
        inner.orders.insert(order_id.clone(), order.clone());
    }

    // Broadcast order.create event to other clients.
    let event = make_event(
        "order",
        "create",
        json!({ "order_id": order_id, "location_id": order.location_id }),
    );
    state.broadcast_event(&token, event);

    Json(body).into_response()
}

async fn create_order_short(
    state: State<AppState>,
    headers: HeaderMap,
    body: Json<Value>,
) -> Response {
    let loc = state
        .inner
        .lock()
        .expect("poisoned")
        .default_location_id
        .clone();
    create_order(state, headers, Path(loc), body).await
}

async fn retrieve_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((_location_id, order_id)): Path<(String, String)>,
) -> Response {
    if let Err(e) = require_client(&headers, &state) {
        return e;
    }
    let inner = state.inner.lock().expect("poisoned");
    match inner.orders.get(&order_id) {
        Some(o) => Json(o.clone()).into_response(),
        None => not_found("order not found"),
    }
}

async fn retrieve_order_short(
    state: State<AppState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> Response {
    let loc = state
        .inner
        .lock()
        .expect("poisoned")
        .default_location_id
        .clone();
    retrieve_order(state, headers, Path((loc, order_id))).await
}

async fn list_orders(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(_location_id): Path<String>,
) -> Response {
    if let Err(e) = require_client(&headers, &state) {
        return e;
    }
    let inner = state.inner.lock().expect("poisoned");
    let all: Vec<Order> = inner.orders.values().cloned().collect();
    Json(all).into_response()
}

async fn list_orders_short(state: State<AppState>, headers: HeaderMap) -> Response {
    let loc = state
        .inner
        .lock()
        .expect("poisoned")
        .default_location_id
        .clone();
    list_orders(state, headers, Path(loc)).await
}

async fn update_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((_location_id, order_id)): Path<(String, String)>,
    Json(patch): Json<Value>,
) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };

    let updated_order = {
        let mut inner = state.inner.lock().expect("poisoned");
        let order = match inner.orders.get_mut(&order_id) {
            Some(o) => o,
            None => return not_found("order not found"),
        };
        if let Some(s) = patch.get("status").and_then(|v| v.as_str())
            && let Ok(status) = serde_json::from_str::<OrderStatus>(&format!("\"{s}\""))
        {
            order.status = status;
        }
        if let Some(t) = patch.get("confirmed_time").and_then(|v| v.as_str()) {
            order.confirmed_time = Some(t.to_string());
        }
        if let Some(p) = patch.get("private_ref").and_then(|v| v.as_str()) {
            order.private_ref = Some(p.to_string());
        }
        order.clone()
    };

    // Broadcast order.update event.
    let event = make_event(
        "order",
        "update",
        json!({
            "order_id": order_id,
            "location_id": updated_order.location_id,
        }),
    );
    state.broadcast_event(&token, event);

    Json(updated_order).into_response()
}

async fn update_order_short(
    state: State<AppState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    body: Json<Value>,
) -> Response {
    let loc = state
        .inner
        .lock()
        .expect("poisoned")
        .default_location_id
        .clone();
    update_order(state, headers, Path((loc, order_id)), body).await
}

// ── Catalogs ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateCatalogBody {
    name: String,
    #[serde(default)]
    data: Option<CatalogData>,
}

async fn create_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateCatalogBody>,
) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let (catalog_id, location_id) = {
        let inner = state.inner.lock().expect("poisoned");
        (
            inner.default_catalog_id.clone(),
            inner.default_location_id.clone(),
        )
    };
    let catalog = Catalog {
        id: catalog_id.clone(),
        location_id: Some(location_id),
        name: body.name,
        created_at: Some(now_rfc3339()),
        data: body.data.or_else(|| Some(CatalogData::default())),
    };
    {
        let mut inner = state.inner.lock().expect("poisoned");
        inner.catalogs.insert(catalog_id.clone(), catalog.clone());
    }
    let event = make_event("catalog", "create", json!({ "catalog_id": catalog_id }));
    state.broadcast_event(&token, event);
    Json(catalog).into_response()
}

async fn create_catalog_by_loc(
    state: State<AppState>,
    headers: HeaderMap,
    Path(_location_id): Path<String>,
    body: Json<CreateCatalogBody>,
) -> Response {
    create_catalog(state, headers, body).await
}

async fn list_catalogs(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(e) = require_client(&headers, &state) {
        return e;
    }
    let inner = state.inner.lock().expect("poisoned");
    let list: Vec<_> = inner
        .catalogs
        .values()
        .map(|c| {
            json!({
                "id": c.id,
                "name": c.name,
                "created_at": c.created_at,
            })
        })
        .collect();
    Json(list).into_response()
}

async fn list_catalogs_by_loc(
    state: State<AppState>,
    headers: HeaderMap,
    Path(_location_id): Path<String>,
) -> Response {
    list_catalogs(state, headers).await
}

async fn retrieve_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(catalog_id): Path<String>,
) -> Response {
    if let Err(e) = require_client(&headers, &state) {
        return e;
    }
    let inner = state.inner.lock().expect("poisoned");
    match inner.catalogs.get(&catalog_id) {
        Some(c) => Json(c.clone()).into_response(),
        None => not_found("catalog not found"),
    }
}

async fn replace_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(catalog_id): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };

    {
        let mut inner = state.inner.lock().expect("poisoned");
        let default_location = inner.default_location_id.clone();
        let existing = inner
            .catalogs
            .entry(catalog_id.clone())
            .or_insert_with(|| Catalog {
                id: catalog_id.clone(),
                location_id: Some(default_location),
                name: "Menu principal".to_string(),
                created_at: Some(now_rfc3339()),
                data: Some(CatalogData::default()),
            });
        if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
            existing.name = name.to_string();
        }
        if let Some(data) = body.get("data")
            && let Ok(parsed) = serde_json::from_value::<CatalogData>(data.clone())
        {
            existing.data = Some(parsed);
        }
    }

    let event = make_event("catalog", "update", json!({ "catalog_id": catalog_id }));
    state.broadcast_event(&token, event);

    let inner = state.inner.lock().expect("poisoned");
    Json(inner.catalogs.get(&catalog_id).cloned()).into_response()
}

async fn delete_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(catalog_id): Path<String>,
) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    {
        let mut inner = state.inner.lock().expect("poisoned");
        inner.catalogs.remove(&catalog_id);
    }
    let event = make_event("catalog", "delete", json!({ "catalog_id": catalog_id }));
    state.broadcast_event(&token, event);
    (StatusCode::OK, Json(json!({"status":"deleted"}))).into_response()
}

// ── Callbacks ────────────────────────────────────────────────────────

async fn get_callback(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let inner = state.inner.lock().expect("poisoned");
    match inner
        .clients
        .get(&token)
        .and_then(|c| c.registered_callback.clone())
    {
        Some(cb) => Json(cb).into_response(),
        None => Json(json!({"url": null, "events": {}})).into_response(),
    }
}

async fn register_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CallbackRegistration>,
) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let mut inner = state.inner.lock().expect("poisoned");
    if let Some(client) = inner.clients.get_mut(&token) {
        client.registered_callback = Some(body.clone());
    }
    Json(body).into_response()
}

async fn delete_callback(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let mut inner = state.inner.lock().expect("poisoned");
    if let Some(client) = inner.clients.get_mut(&token) {
        client.registered_callback = None;
    }
    (StatusCode::OK, Json(json!({"status":"deleted"}))).into_response()
}

#[derive(Deserialize)]
struct EventsQuery {
    #[serde(default)]
    _cursor: Option<String>,
    #[serde(default)]
    count: Option<usize>,
}

async fn list_callback_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<EventsQuery>,
) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let count = q.count.unwrap_or(100).min(100);
    let inner = state.inner.lock().expect("poisoned");
    let client: &ClientState = inner.clients.get(&token).expect("client");
    let events: Vec<CallbackEvent> = client.events.iter().take(count).cloned().collect();
    Json(events).into_response()
}

async fn delete_callback_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(event_id): Path<String>,
) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let mut inner = state.inner.lock().expect("poisoned");
    if let Some(client) = inner.clients.get_mut(&token) {
        client.events.retain(|e| e.id != event_id);
    }
    (StatusCode::OK, Json(json!({"status":"deleted"}))).into_response()
}

// ── Location ─────────────────────────────────────────────────────────

async fn get_location(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let inner = state.inner.lock().expect("poisoned");
    let client = inner.clients.get(&token).expect("client");
    Json(json!({
        "id": inner.default_location_id,
        "account_id": inner.default_account_id,
        "name": client.location_name,
        "catalog_id": inner.default_catalog_id,
        "customer_list_id": inner.default_customer_list_id,
    }))
    .into_response()
}
