//! HubRise mock HTTP server.
//!
//! In-memory implementation of the HubRise REST API used by the POS
//! (and the webshop side of tests) - no real HubRise account needed.
//! It follows the rules the POS depends on: events only for a
//! registered callback and never for the client's own changes,
//! paginated event lists without state fields, order status rules,
//! the catalog read limit, HubRise error bodies with per-field
//! `errors[]`, the connection-wide rate limits.
//!
//! Besides the API (`/v1`, `/oauth2/v1`) it serves a control API
//! (`/mock/*`) for tests: inject template orders, cancel them as a
//! platform would, replace the catalog, set stock, inject faults,
//! flip behaviour toggles, read the log of every request received.
//! See `README.md`.
//!
//! Docs revision baseline: see `HUBRISE_DOCS_REVISION`. Update this
//! constant every time the extracted docs under `docs/` are
//! refreshed to track mock/reality drift.
//!
//! Run with: `cargo run -p pos-hubrise-mock`
//! Default port: 3001 (override with `HUBRISE_MOCK_PORT`). Tests mount
//! [`app`] directly.

#![allow(clippy::result_large_err)]

pub mod builders;
pub mod state;

use std::sync::Arc;

use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use pos_hubrise_types::{
    CallbackEvent, CallbackRegistration, CatalogData, ErrorResponse, ErrorType, InventoryEntry,
    OAuthTokenResponse, ValidationError,
};
use serde::Deserialize;
use serde_json::{Value, json};
use state::{ApiError, Fault, LoggedRequest, MockState, WEBSHOP_TOKEN, validate_catalog};
use uuid::Uuid;

/// Version of the HubRise docs this mock targets. Bump whenever
/// `docs/*.md` is refreshed to surface drift during review.
pub const HUBRISE_DOCS_REVISION: &str = "2026-09-30";
/// URL of the official docs consulted to build this mock.
pub const HUBRISE_DOCS_URL: &str = "https://www.hubrise.com/developers/api/";

/// Page size of paginated lists (HubRise's default and maximum).
const PAGE_SIZE: usize = 100;

/// Shared mock state behind every handler.
pub type AppState = Arc<MockState>;

/// The mock's router over `state`: what `main.rs` serves, and what
/// tests mount on an ephemeral port.
pub fn app(state: AppState) -> Router {
    let api = Router::new()
        // OAuth 2.0
        .route("/oauth2/v1/authorize", get(oauth_authorize))
        .route("/oauth2/v1/token", post(oauth_token))
        .route("/oauth2/v1/revoke", post(oauth_revoke))
        // Orders
        .route(
            "/v1/locations/{location_id}/orders",
            post(create_order).get(list_orders),
        )
        .route(
            "/v1/location/orders",
            post(create_order_short).get(list_orders_short),
        )
        .route(
            "/v1/locations/{location_id}/orders/{order_id}",
            get(retrieve_order).patch(update_order),
        )
        .route(
            "/v1/location/orders/{order_id}",
            get(retrieve_order_short).patch(update_order_short),
        )
        // Catalogs
        .route(
            "/v1/location/catalogs",
            get(list_catalogs).post(create_catalog),
        )
        .route(
            "/v1/locations/{location_id}/catalogs",
            get(list_catalogs_by_loc).post(create_catalog_by_loc),
        )
        .route(
            "/v1/catalogs/{catalog_id}",
            get(retrieve_catalog)
                .put(replace_catalog)
                .delete(delete_catalog),
        )
        // Inventory - scoped and unscoped, by catalog and by location.
        .route(
            "/v1/location/inventory",
            get(get_inventory_default)
                .put(replace_inventory_default)
                .patch(patch_inventory_default),
        )
        .route(
            "/v1/locations/{location_id}/inventory",
            get(get_inventory_default_loc)
                .put(replace_inventory_default_loc)
                .patch(patch_inventory_default_loc),
        )
        .route(
            "/v1/catalogs/{catalog_id}/location/inventory",
            get(get_inventory_cat)
                .put(replace_inventory_cat)
                .patch(patch_inventory_cat),
        )
        .route(
            "/v1/catalogs/{catalog_id}/locations/{location_id}/inventory",
            get(get_inventory_cat_loc)
                .put(replace_inventory_cat_loc)
                .patch(patch_inventory_cat_loc),
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
            "/v1/callback/events/{event_id}",
            get(retrieve_callback_event).delete(delete_callback_event),
        )
        // Location
        .route("/v1/location", get(get_location))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            log_and_faults,
        ));

    let control = Router::new()
        .route("/mock/orders", post(control_inject_order))
        .route(
            "/mock/orders/{order_id}",
            axum::routing::patch(control_update_order),
        )
        .route("/mock/orders/{order_id}/cancel", post(control_cancel_order))
        .route("/mock/catalog", axum::routing::put(control_replace_catalog))
        .route(
            "/mock/inventory",
            axum::routing::put(control_replace_inventory),
        )
        .route("/mock/inventory/expire", post(control_expire_inventory))
        .route(
            "/mock/faults",
            post(control_add_fault).delete(control_clear_faults),
        )
        .route(
            "/mock/requests",
            get(control_requests).delete(control_clear_requests),
        )
        .route("/mock/events", get(control_events))
        .route("/mock/limits", post(control_limits))
        .route("/mock/settings", axum::routing::put(control_settings))
        .route("/mock/reset", post(control_reset));

    Router::new()
        .merge(api)
        .merge(control)
        .route("/", get(health))
        .fallback(routing_error)
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

/// Any request matching no route: HubRise's `routing_error` envelope
/// (distinct from `not_found`, which is a known resource missing).
async fn routing_error(method: Method, uri: Uri) -> Response {
    let message = format!("no route matches {method} {}", uri.path());
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new(ErrorType::RoutingError, message)),
    )
        .into_response()
}

// ── Helpers ──────────────────────────────────────────────────────────

fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("X-Access-Token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn error_response(status: u16, message: &str) -> Response {
    let error_type = error_type_of(status);
    let status_code = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status_code, Json(ErrorResponse::new(error_type, message))).into_response()
}

/// A 422 (or other status) carrying per-field `errors[]`.
fn error_response_fields(status: u16, message: &str, fields: &[(String, String)]) -> Response {
    let error_type = error_type_of(status);
    let status_code = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut body = ErrorResponse::new(error_type, message);
    body.errors = fields
        .iter()
        .map(|(field, message)| ValidationError {
            field: field.clone(),
            message: message.clone(),
        })
        .collect();
    (status_code, Json(body)).into_response()
}

fn error_type_of(status: u16) -> ErrorType {
    match status {
        401 => ErrorType::Unauthorized,
        403 => ErrorType::Forbidden,
        404 => ErrorType::NotFound,
        415 => ErrorType::UnsupportedMediaType,
        422 => ErrorType::UnprocessableEntity,
        429 => ErrorType::TooManyRequests,
        _ => ErrorType::InternalError,
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if self.fields.is_empty() {
            error_response(self.status, &self.message)
        } else {
            error_response_fields(self.status, &self.message, &self.fields)
        }
    }
}

fn unauthorized() -> Response {
    error_response(401, "The connection has been blocked")
}

fn not_found(msg: &str) -> Response {
    error_response(404, msg)
}

fn require_client(headers: &HeaderMap, state: &AppState) -> Result<String, Response> {
    let token = extract_token(headers).ok_or_else(unauthorized)?;
    let inner = state.inner.lock().expect("poisoned");
    if !inner.clients.contains_key(&token) || inner.revoked_tokens.contains(&token) {
        return Err(unauthorized());
    }
    Ok(token)
}

#[derive(Deserialize, Default)]
struct Page {
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default)]
    count: Option<usize>,
}

/// One page of `items` and the `X-Cursor-Next` header when more follow.
fn paginate<T: serde::Serialize + Clone>(items: &[T], page: &Page) -> Response {
    let start: usize = page
        .cursor
        .as_deref()
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let count = page.count.unwrap_or(PAGE_SIZE).clamp(1, PAGE_SIZE);
    let end = (start + count).min(items.len());
    let slice = items
        .get(start.min(items.len())..end)
        .unwrap_or(&[])
        .to_vec();
    let mut resp = Json(slice).into_response();
    if end < items.len()
        && let Ok(v) = HeaderValue::from_str(&end.to_string())
    {
        resp.headers_mut().insert("X-Cursor-Next", v);
    }
    resp
}

/// Record every API request, apply injected faults, refuse a
/// non-JSON `Content-Type` on a write with a body (415), and enforce
/// the connection-wide rate limit.
async fn log_and_faults(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(str::to_string);
    let token = extract_token(req.headers());
    let content_type = req
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let (parts, body) = req.into_parts();
    let bytes: Bytes = axum::body::to_bytes(body, 1 << 20)
        .await
        .unwrap_or_default();
    let body_json = serde_json::from_slice::<Value>(&bytes).ok().or_else(|| {
        // Form bodies (OAuth token exchange) are logged as text.
        (!bytes.is_empty()).then(|| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
    });

    // OAuth's token endpoint legitimately takes a form body; every
    // other write route documents a JSON body.
    let is_write = matches!(method.as_str(), "POST" | "PUT" | "PATCH");
    let wrong_media_type = is_write
        && !bytes.is_empty()
        && path != "/oauth2/v1/token"
        && !content_type
            .as_deref()
            .is_some_and(|ct| ct.starts_with("application/json"));

    let rate_limited = {
        let mut inner = state.inner.lock().expect("poisoned");
        match token.as_deref() {
            Some(t) if inner.clients.contains_key(t) => inner.request_allowed(t).err(),
            _ => None,
        }
    };

    let fault = {
        let mut inner = state.inner.lock().expect("poisoned");
        let found = inner.faults.iter().position(|f| {
            (f.method == "*" || f.method.eq_ignore_ascii_case(&method)) && path.starts_with(&f.path)
        });
        found.map(|i| {
            let f = inner.faults[i].clone();
            if let Some(times) = inner.faults[i].times.as_mut() {
                *times -= 1;
                if *times == 0 {
                    inner.faults.remove(i);
                }
            }
            f
        })
    };

    if let Some(Fault { delay_ms, .. }) = &fault
        && *delay_ms > 0
    {
        tokio::time::sleep(std::time::Duration::from_millis(*delay_ms)).await;
    }
    let resp = if let Some(Fault { status, .. }) = fault
        && status != 0
    {
        error_response(status, "injected fault")
    } else if wrong_media_type {
        error_response(
            415,
            "Content-Type must be application/json for this request",
        )
    } else if let Some(err) = rate_limited {
        err.into_response()
    } else {
        let req = Request::from_parts(parts, Body::from(bytes));
        next.run(req).await
    };

    let mut inner = state.inner.lock().expect("poisoned");
    let client = token.as_deref().and_then(|t| inner.client_label(t));
    inner.requests.push(LoggedRequest {
        method,
        path,
        query,
        client,
        body: body_json,
        status: resp.status().as_u16(),
    });
    resp
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

/// `GET /oauth2/v1/authorize` - returns a JSON response with an
/// authorization code instead of a real 302 redirect. The dev client
/// reads the code and POSTs it to `/oauth2/v1/token`.
async fn oauth_authorize(
    State(state): State<AppState>,
    Query(q): Query<AuthorizeQuery>,
) -> Response {
    let code = Uuid::new_v4().to_string();
    {
        let mut inner = state.inner.lock().expect("poisoned");
        let token = q
            .client_id
            .as_deref()
            .and_then(|id| inner.oauth_clients.get(id).cloned())
            .unwrap_or_else(|| WEBSHOP_TOKEN.to_string());
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

/// `POST /oauth2/v1/token`: HubRise takes a form body `code=...` with
/// the client in HTTP Basic auth; a JSON body `{client_id,
/// client_secret, code}` is accepted too.
async fn oauth_token(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let code = serde_json::from_slice::<Value>(&body)
        .ok()
        .and_then(|v| v.get("code").and_then(Value::as_str).map(str::to_string))
        .or_else(|| url_form_value(&String::from_utf8_lossy(&body), "code"));
    let Some(code) = code else {
        return error_response(400, "missing code");
    };
    let mut inner = state.inner.lock().expect("poisoned");
    // A code works once (docs/02-oauth.md).
    let Some(token) = inner.pending_auth_codes.remove(&code) else {
        return error_response(400, "invalid or expired code");
    };
    // With Basic auth, the client must be the one the code was made for.
    if let Some(client_id) = basic_auth_user(&headers)
        && inner
            .oauth_clients
            .get(&client_id)
            .is_some_and(|t| *t != token)
    {
        return error_response(401, "invalid client id or secret");
    }
    let Some(client) = inner.clients.get(&token) else {
        return error_response(400, "no client for code");
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

/// The value of `key` in an `application/x-www-form-urlencoded` body.
fn url_form_value(body: &str, key: &str) -> Option<String> {
    url::form_urlencoded::parse(body.as_bytes())
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
}

/// The user part of an `Authorization: Basic` header.
fn basic_auth_user(headers: &HeaderMap) -> Option<String> {
    use base64::Engine;
    let value = headers.get("Authorization")?.to_str().ok()?;
    let encoded = value.strip_prefix("Basic ")?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let text = String::from_utf8(decoded).ok()?;
    text.split_once(':').map(|(user, _)| user.to_string())
}

/// `POST /oauth2/v1/revoke` makes the calling connection's token
/// unusable: every later request with it 401s, until `/mock/reset`.
async fn oauth_revoke(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(token) = extract_token(&headers) else {
        return error_response(400, "missing X-Access-Token");
    };
    let mut inner = state.inner.lock().expect("poisoned");
    if !inner.clients.contains_key(&token) {
        return error_response(400, "unknown token");
    }
    inner.revoked_tokens.insert(token);
    (StatusCode::OK, Json(json!({"status": "revoked"}))).into_response()
}

// ── Orders ───────────────────────────────────────────────────────────

fn default_location(state: &AppState) -> String {
    state
        .inner
        .lock()
        .expect("poisoned")
        .default_location_id
        .clone()
}

async fn create_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(location_id): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let mut inner = state.inner.lock().expect("poisoned");
    match inner.create_order(&token, &location_id, body) {
        Ok(order) => Json(order).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn create_order_short(
    state: State<AppState>,
    headers: HeaderMap,
    body: Json<Value>,
) -> Response {
    let loc = default_location(&state);
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
    let loc = default_location(&state);
    retrieve_order(state, headers, Path((loc, order_id))).await
}

#[derive(Deserialize, Default)]
struct OrdersQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    private_ref: Option<String>,
    #[serde(default)]
    created_by: Option<String>,
    #[serde(default)]
    after: Option<String>,
    #[serde(default)]
    before: Option<String>,
    #[serde(flatten)]
    page: PageFields,
}

#[derive(Deserialize, Default)]
struct PageFields {
    #[serde(default)]
    cursor: Option<String>,
    #[serde(default)]
    count: Option<usize>,
}

async fn list_orders(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(location_id): Path<String>,
    Query(q): Query<OrdersQuery>,
) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let mut inner = state.inner.lock().expect("poisoned");
    if let Err(e) = inner.read_order_list_allowed(&token) {
        return e.into_response();
    }
    // `private_ref` is a per-client lookup: it never sees another
    // client's orders, even at the same location.
    if let Some(private_ref) = &q.private_ref {
        let orders: Vec<Value> = inner
            .orders_by_private_ref(&token, &location_id, private_ref)
            .cloned()
            .collect();
        return paginate(
            &orders,
            &Page {
                cursor: q.page.cursor,
                count: q.page.count,
            },
        );
    }
    let time = |o: &Value| {
        o["created_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
    };
    let bound = |s: &Option<String>| {
        s.as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
    };
    let (after, before) = (bound(&q.after), bound(&q.before));
    // Newest first, as HubRise lists orders.
    let orders: Vec<Value> = inner
        .order_ids
        .iter()
        .rev()
        .filter_map(|id| inner.orders.get(id))
        .filter(|o| o["location_id"] == location_id.as_str())
        .filter(|o| q.status.as_deref().is_none_or(|s| o["status"] == s))
        .filter(|o| q.created_by.as_deref().is_none_or(|c| o["created_by"] == c))
        .filter(|o| after.is_none_or(|a| time(o).is_some_and(|t| t >= a)))
        .filter(|o| before.is_none_or(|b| time(o).is_some_and(|t| t < b)))
        .cloned()
        .collect();
    paginate(
        &orders,
        &Page {
            cursor: q.page.cursor,
            count: q.page.count,
        },
    )
}

async fn list_orders_short(
    state: State<AppState>,
    headers: HeaderMap,
    q: Query<OrdersQuery>,
) -> Response {
    let loc = default_location(&state);
    list_orders(state, headers, Path(loc), q).await
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
    let mut inner = state.inner.lock().expect("poisoned");
    match inner.update_order(&token, &order_id, &patch) {
        Ok(order) => Json(order).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn update_order_short(
    state: State<AppState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    body: Json<Value>,
) -> Response {
    let loc = default_location(&state);
    update_order(state, headers, Path((loc, order_id)), body).await
}

// ── Catalogs ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CatalogBody {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    data: Option<CatalogData>,
}

async fn create_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CatalogBody>,
) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let data = body.data.unwrap_or_default();
    if let Err(e) = validate_catalog(&data) {
        return e.into_response();
    }
    let mut inner = state.inner.lock().expect("poisoned");
    let id = inner.new_id("cat");
    Json(inner.replace_catalog(&token, &id, body.name, data)).into_response()
}

async fn create_catalog_by_loc(
    state: State<AppState>,
    headers: HeaderMap,
    Path(_location_id): Path<String>,
    body: Json<CatalogBody>,
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
        .map(|c| json!({ "id": c.id, "name": c.name, "created_at": c.created_at }))
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

#[derive(Deserialize, Default)]
struct CatalogQuery {
    #[serde(default)]
    hide_data: Option<bool>,
}

async fn retrieve_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(catalog_id): Path<String>,
    Query(q): Query<CatalogQuery>,
) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let mut inner = state.inner.lock().expect("poisoned");
    if let Err(e) = inner.read_catalog_allowed(&token) {
        return e.into_response();
    }
    match inner.catalogs.get(&catalog_id) {
        Some(c) => {
            let mut c = c.clone();
            if q.hide_data == Some(true) {
                c.data = None;
            }
            Json(c).into_response()
        }
        None => not_found("catalog not found"),
    }
}

async fn replace_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(catalog_id): Path<String>,
    Json(body): Json<CatalogBody>,
) -> Response {
    // `PUT /catalogs/:id` is documented Account-access only; the mock
    // has no access-level model at all (see README "Differences from
    // real HubRise"), so any valid client token may replace a catalog.
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let data = body.data.unwrap_or_default();
    if let Err(e) = validate_catalog(&data) {
        return e.into_response();
    }
    let mut inner = state.inner.lock().expect("poisoned");
    Json(inner.replace_catalog(&token, &catalog_id, body.name, data)).into_response()
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
    let mut inner = state.inner.lock().expect("poisoned");
    if inner.catalogs.remove(&catalog_id).is_none() {
        return not_found("catalog not found");
    }
    let mut extra = serde_json::Map::new();
    extra.insert("catalog_id".to_string(), json!(catalog_id));
    inner.emit(
        &token,
        CallbackEvent {
            id: String::new(),
            resource_type: "catalog".to_string(),
            event_type: "delete".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            previous_state: None,
            new_state: None,
            extra,
        },
    );
    (StatusCode::OK, Json(json!({"status":"deleted"}))).into_response()
}

// ── Inventory ────────────────────────────────────────────────────────

async fn get_inventory(
    state: &AppState,
    headers: &HeaderMap,
    catalog_id: Option<&str>,
) -> Response {
    if let Err(e) = require_client(headers, state) {
        return e;
    }
    let inner = state.inner.lock().expect("poisoned");
    Json(inner.inventory_of(catalog_id)).into_response()
}

async fn replace_inventory(
    state: &AppState,
    headers: &HeaderMap,
    catalog_id: &str,
    entries: Vec<InventoryEntry>,
) -> Response {
    let token = match require_client(headers, state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let mut inner = state.inner.lock().expect("poisoned");
    match inner.replace_inventory(&token, catalog_id, entries) {
        Ok(entries) => Json(entries).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn patch_inventory(
    state: &AppState,
    headers: &HeaderMap,
    catalog_id: &str,
    entries: Vec<InventoryEntry>,
) -> Response {
    let token = match require_client(headers, state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let mut inner = state.inner.lock().expect("poisoned");
    match inner.patch_inventory(&token, catalog_id, entries) {
        // The real PATCH response carries only the modified entries.
        Ok(entries) => Json(entries).into_response(),
        Err(e) => e.into_response(),
    }
}

fn default_catalog(state: &AppState) -> String {
    state
        .inner
        .lock()
        .expect("poisoned")
        .default_catalog_id
        .clone()
}

async fn get_inventory_default(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let cat = default_catalog(&state);
    get_inventory(&state, &headers, Some(&cat)).await
}

async fn replace_inventory_default(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(entries): Json<Vec<InventoryEntry>>,
) -> Response {
    let cat = default_catalog(&state);
    replace_inventory(&state, &headers, &cat, entries).await
}

async fn patch_inventory_default(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(entries): Json<Vec<InventoryEntry>>,
) -> Response {
    let cat = default_catalog(&state);
    patch_inventory(&state, &headers, &cat, entries).await
}

async fn get_inventory_default_loc(
    state: State<AppState>,
    headers: HeaderMap,
    Path(_location_id): Path<String>,
) -> Response {
    get_inventory_default(state, headers).await
}

async fn replace_inventory_default_loc(
    state: State<AppState>,
    headers: HeaderMap,
    Path(_location_id): Path<String>,
    body: Json<Vec<InventoryEntry>>,
) -> Response {
    replace_inventory_default(state, headers, body).await
}

async fn patch_inventory_default_loc(
    state: State<AppState>,
    headers: HeaderMap,
    Path(_location_id): Path<String>,
    body: Json<Vec<InventoryEntry>>,
) -> Response {
    patch_inventory_default(state, headers, body).await
}

async fn get_inventory_cat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(catalog_id): Path<String>,
) -> Response {
    get_inventory(&state, &headers, Some(&catalog_id)).await
}

async fn replace_inventory_cat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(catalog_id): Path<String>,
    Json(entries): Json<Vec<InventoryEntry>>,
) -> Response {
    replace_inventory(&state, &headers, &catalog_id, entries).await
}

async fn patch_inventory_cat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(catalog_id): Path<String>,
    Json(entries): Json<Vec<InventoryEntry>>,
) -> Response {
    patch_inventory(&state, &headers, &catalog_id, entries).await
}

async fn get_inventory_cat_loc(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((catalog_id, _location_id)): Path<(String, String)>,
) -> Response {
    get_inventory(&state, &headers, Some(&catalog_id)).await
}

async fn replace_inventory_cat_loc(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((catalog_id, _location_id)): Path<(String, String)>,
    Json(entries): Json<Vec<InventoryEntry>>,
) -> Response {
    replace_inventory(&state, &headers, &catalog_id, entries).await
}

async fn patch_inventory_cat_loc(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((catalog_id, _location_id)): Path<(String, String)>,
    Json(entries): Json<Vec<InventoryEntry>>,
) -> Response {
    patch_inventory(&state, &headers, &catalog_id, entries).await
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
    if !body.events.is_object() {
        return error_response(422, "events must be an object");
    }
    // Active callbacks (a non-null url, delivered by HubRise with
    // retries and an HMAC signature) are not implemented by this
    // mock: refuse instead of silently treating it as passive.
    if body.url.is_some() {
        return error_response_fields(
            422,
            "active callbacks (a non-null url) are not supported by this mock",
            &[(
                "url".to_string(),
                "only passive polling (url: null) is supported by pos-hubrise-mock".to_string(),
            )],
        );
    }
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
        client.events.clear();
    }
    (StatusCode::OK, Json(json!({"status":"deleted"}))).into_response()
}

/// An event as listed: without its state fields
/// (`docs/06-callbacks.md` "Event Retrieval").
fn without_states(event: &CallbackEvent) -> CallbackEvent {
    CallbackEvent {
        previous_state: None,
        new_state: None,
        ..event.clone()
    }
}

async fn list_callback_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(page): Query<Page>,
) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let inner = state.inner.lock().expect("poisoned");
    let events: Vec<CallbackEvent> = inner.clients[&token]
        .events
        .iter()
        .map(without_states)
        .collect();
    paginate(&events, &page)
}

async fn retrieve_callback_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(event_id): Path<String>,
) -> Response {
    let token = match require_client(&headers, &state) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let inner = state.inner.lock().expect("poisoned");
    match inner.clients[&token]
        .events
        .iter()
        .find(|e| e.id == event_id)
    {
        Some(e) => Json(e.clone()).into_response(),
        None => not_found("event not found"),
    }
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
    let client = inner.clients.get_mut(&token).expect("client");
    let before = client.events.len();
    client.events.retain(|e| e.id != event_id);
    if client.events.len() == before {
        return not_found("event not found");
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
        "timezone": { "name": "Europe/Brussels" },
    }))
    .into_response()
}

// ── Control API (tests only) ─────────────────────────────────────────

#[derive(Deserialize)]
struct InjectOrder {
    /// A name from [`builders::TEMPLATES`], or none for `order` alone.
    #[serde(default)]
    template: Option<String>,
    /// Fields merged over the template (or the whole order).
    #[serde(default)]
    order: Option<Value>,
}

/// `POST /mock/orders`: create an order as the webshop client, as a
/// platform or the webshop would; the POS is told if it registered.
async fn control_inject_order(
    State(state): State<AppState>,
    Json(body): Json<InjectOrder>,
) -> Response {
    let mut order = match body.template.as_deref() {
        Some(name) => match builders::template(name) {
            Some(b) => b.build(),
            None => return error_response(422, &format!("unknown template {name}")),
        },
        None => json!({ "status": "new", "items": [] }),
    };
    if let Some(Value::Object(fields)) = body.order {
        for (k, v) in fields {
            order[k] = v;
        }
    }
    let mut inner = state.inner.lock().expect("poisoned");
    let loc = inner.default_location_id.clone();
    match inner.create_order(WEBSHOP_TOKEN, &loc, order) {
        Ok(order) => Json(order).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `PATCH /mock/orders/:id`: change an order as the webshop client.
async fn control_update_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
    Json(patch): Json<Value>,
) -> Response {
    let mut inner = state.inner.lock().expect("poisoned");
    match inner.update_order(WEBSHOP_TOKEN, &order_id, &patch) {
        Ok(order) => Json(order).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `POST /mock/orders/:id/cancel`: the platform cancels the order.
async fn control_cancel_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> Response {
    let mut inner = state.inner.lock().expect("poisoned");
    match inner.update_order(WEBSHOP_TOKEN, &order_id, &json!({ "status": "cancelled" })) {
        Ok(order) => Json(order).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `PUT /mock/catalog`: replace the default catalog as the webshop
/// client (`{"data": ...}`, or `{}` for the seed catalog).
async fn control_replace_catalog(
    State(state): State<AppState>,
    Json(body): Json<CatalogBody>,
) -> Response {
    let mut inner = state.inner.lock().expect("poisoned");
    let id = inner.default_catalog_id.clone();
    let data = body.data.unwrap_or_else(builders::seed_catalog);
    Json(inner.replace_catalog(WEBSHOP_TOKEN, &id, body.name, data)).into_response()
}

#[derive(Deserialize)]
struct ControlInventory {
    #[serde(default)]
    catalog_id: Option<String>,
    entries: Vec<InventoryEntry>,
}

/// `PUT /mock/inventory`: set stock as "the other side" (a platform
/// or the HubRise back office would) - replaces the whole inventory
/// of `catalog_id` (or the default catalog) and announces it.
async fn control_replace_inventory(
    State(state): State<AppState>,
    Json(body): Json<ControlInventory>,
) -> Response {
    let mut inner = state.inner.lock().expect("poisoned");
    let catalog_id = body
        .catalog_id
        .unwrap_or_else(|| inner.default_catalog_id.clone());
    match inner.replace_inventory(WEBSHOP_TOKEN, &catalog_id, body.entries) {
        Ok(entries) => Json(entries).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `POST /mock/inventory/expire`: announce every entry with an
/// `expires_at` in the default catalog as HubRise's documented
/// `inventory.patch` expiry notice.
async fn control_expire_inventory(State(state): State<AppState>) -> Response {
    state.inner.lock().expect("poisoned").expire_inventory();
    StatusCode::NO_CONTENT.into_response()
}

async fn control_add_fault(State(state): State<AppState>, Json(fault): Json<Fault>) -> Response {
    state.inner.lock().expect("poisoned").faults.push(fault);
    StatusCode::NO_CONTENT.into_response()
}

async fn control_clear_faults(State(state): State<AppState>) -> Response {
    state.inner.lock().expect("poisoned").faults.clear();
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Deserialize)]
struct RequestsQuery {
    /// Only requests whose path starts with this.
    #[serde(default)]
    path: Option<String>,
}

async fn control_requests(
    State(state): State<AppState>,
    Query(q): Query<RequestsQuery>,
) -> Response {
    let inner = state.inner.lock().expect("poisoned");
    let list: Vec<LoggedRequest> = inner
        .requests
        .iter()
        .filter(|r| q.path.as_deref().is_none_or(|p| r.path.starts_with(p)))
        .cloned()
        .collect();
    Json(list).into_response()
}

async fn control_clear_requests(State(state): State<AppState>) -> Response {
    state.inner.lock().expect("poisoned").requests.clear();
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Deserialize)]
struct EventsQuery {
    token: String,
}

/// `GET /mock/events?token=...`: the events waiting for a client,
/// with their state fields.
async fn control_events(State(state): State<AppState>, Query(q): Query<EventsQuery>) -> Response {
    let inner = state.inner.lock().expect("poisoned");
    match inner.clients.get(&q.token) {
        Some(c) => Json(c.events.iter().cloned().collect::<Vec<_>>()).into_response(),
        None => not_found("no such client"),
    }
}

#[derive(Deserialize, Default)]
struct Limits {
    #[serde(default)]
    catalog_reads_per_minute: Option<usize>,
    #[serde(default)]
    order_list_reads_per_minute: Option<usize>,
}

/// `POST /mock/limits`: change the per-minute read limits.
async fn control_limits(State(state): State<AppState>, Json(l): Json<Limits>) -> Response {
    let mut inner = state.inner.lock().expect("poisoned");
    if let Some(v) = l.catalog_reads_per_minute {
        inner.catalog_reads_per_minute = v;
    }
    if let Some(v) = l.order_list_reads_per_minute {
        inner.settings.order_list_reads_per_minute = v;
    }
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Deserialize, Default)]
struct SettingsBody {
    #[serde(default)]
    strict_transitions: Option<bool>,
    #[serde(default)]
    strict_private_ref: Option<bool>,
    #[serde(default)]
    rate_limit_per_minute: Option<usize>,
    #[serde(default)]
    rate_limit_per_hour: Option<usize>,
    #[serde(default)]
    rate_limit_per_day: Option<usize>,
}

/// `PUT /mock/settings`: flip the behaviour toggles (see `README.md`
/// "Toggles"). Every field is optional; only the fields given change.
async fn control_settings(State(state): State<AppState>, Json(s): Json<SettingsBody>) -> Response {
    let mut inner = state.inner.lock().expect("poisoned");
    if let Some(v) = s.strict_transitions {
        inner.settings.strict_transitions = v;
    }
    if let Some(v) = s.strict_private_ref {
        inner.settings.strict_private_ref = v;
    }
    if let Some(v) = s.rate_limit_per_minute {
        inner.settings.rate_limit_per_minute = v;
    }
    if let Some(v) = s.rate_limit_per_hour {
        inner.settings.rate_limit_per_hour = v;
    }
    if let Some(v) = s.rate_limit_per_day {
        inner.settings.rate_limit_per_day = v;
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn control_reset(State(state): State<AppState>) -> Response {
    state.reset();
    StatusCode::NO_CONTENT.into_response()
}
