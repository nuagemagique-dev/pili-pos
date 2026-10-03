//! A `TestApp` connected to the real HubRise API, gently.
//!
//! The stored connection's `base_url` points at a local pacing proxy,
//! which forwards each call to `PILI_LIVE_HUBRISE_API` (default
//! `https://api.hubrise.com`):
//!
//! - one call at a time, at least `PILI_LIVE_PAUSE_MS` (default 1000)
//!   after the previous one;
//! - at most `PILI_LIVE_HUBRISE_MAX_REQUESTS` (default 40) calls per
//!   test binary run; past the cap the proxy answers 503 itself and
//!   forwards nothing;
//! - headers and bodies pass through untouched and are never logged.
//!
//! The token comes from `target/live-hubrise/token.json`, written by
//! `scripts/live_hubrise_login.sh`: a location-level token of a test
//! location, with `catalog.write` so the suite can load its test menu.
//!
//! With `PILI_LIVE_RECORD_DIR` set, each call is also written there as
//! `<n>_<method>_<path>.json` (method, path, status, request and
//! response bodies), with customer details and tokens removed: the
//! fixtures of `crates/pos-hubrise-mock/fixtures/hubrise`.

#![cfg(feature = "hubrise")]

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use pos_hubrise::token_store::{self, StoredConnection};
use serde_json::Value;

use super::fixtures::workspace_root;
use super::harness::TestApp;

/// Calls forwarded to HubRise by this test binary.
static FORWARDED: AtomicUsize = AtomicUsize::new(0);

pub fn live_hubrise_requests() -> usize {
    FORWARDED.load(Ordering::SeqCst)
}

/// What `scripts/live_hubrise_login.sh` stored.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LiveToken {
    pub access_token: String,
    pub account_id: String,
    pub account_name: String,
    pub location_id: String,
    pub location_name: String,
    pub catalog_id: Option<String>,
    pub catalog_name: Option<String>,
}

/// The stored token, or `None` to skip.
pub fn live_token() -> Option<LiveToken> {
    let path = workspace_root().join("target/live-hubrise/token.json");
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn env_number(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

struct Proxy {
    target: String,
    client: reqwest::Client,
    pause: Duration,
    cap: usize,
    last: tokio::sync::Mutex<Option<Instant>>,
}

/// Fields that name or reach a person, or grant access.
const PRIVATE: &[&str] = &[
    "access_token",
    "first_name",
    "last_name",
    "email",
    "phone",
    "address_1",
    "address_2",
    "postal_code",
    "city",
    "latitude",
    "longitude",
    "delivery_notes",
    "customer_notes",
    "company_name",
    // The client app's name, and the account's.
    "created_by",
    "account_name",
    "location_name",
];

fn scrub(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for key in PRIVATE {
                if let Some(v) = map.get_mut(*key)
                    && !v.is_null()
                {
                    *v = Value::String("<scrubbed>".into());
                }
            }
            map.values_mut().for_each(scrub);
        }
        Value::Array(items) => items.iter_mut().for_each(scrub),
        _ => {}
    }
}

fn record(n: usize, method: &str, path: &str, status: u16, request: &[u8], response: &[u8]) {
    let Ok(dir) = std::env::var("PILI_LIVE_RECORD_DIR") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let as_json = |raw: &[u8]| -> Value {
        if raw.is_empty() {
            return Value::Null;
        }
        let mut v = serde_json::from_slice(raw)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(raw).into()));
        scrub(&mut v);
        v
    };
    let slug: String = path
        .split('?')
        .next()
        .unwrap_or_default()
        .trim_start_matches("/v1/")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let entry = serde_json::json!({
        "method": method,
        "path": path,
        "status": status,
        "request": as_json(request),
        "response": as_json(response),
    });
    std::fs::write(
        dir.join(format!("{n:03}_{method}_{slug}.json")),
        serde_json::to_string_pretty(&entry).unwrap() + "\n",
    )
    .unwrap();
}

async fn forward(State(proxy): State<Arc<Proxy>>, request: Request) -> impl IntoResponse {
    // One call at a time: the lock is held for the whole round trip.
    let mut last = proxy.last.lock().await;
    if FORWARDED.load(Ordering::SeqCst) >= proxy.cap {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            format!(
                "live request cap reached ({}), raise PILI_LIVE_HUBRISE_MAX_REQUESTS",
                proxy.cap
            ),
        )
            .into_response();
    }
    if let Some(at) = *last {
        let since = at.elapsed();
        if since < proxy.pause {
            tokio::time::sleep(proxy.pause - since).await;
        }
    }
    let n = FORWARDED.fetch_add(1, Ordering::SeqCst) + 1;
    let method = request.method().clone();
    let path = request
        .uri()
        .path_and_query()
        .map(|p| p.as_str().to_string())
        .unwrap_or_default();
    let headers = request.headers().clone();
    let body: Bytes = axum::body::to_bytes(request.into_body(), 4 << 20)
        .await
        .unwrap_or_default();
    let mut out = proxy
        .client
        .request(method.clone(), format!("{}{}", proxy.target, path))
        .body(body.clone());
    for (name, value) in &headers {
        if name != "host" && name != "content-length" {
            out = out.header(name, value);
        }
    }
    let answer = out.send().await;
    *last = Some(Instant::now());
    match answer {
        Ok(resp) => {
            let status = resp.status();
            let content_type = resp.headers().get("content-type").cloned();
            let link = resp.headers().get("link").cloned();
            let bytes = resp.bytes().await.unwrap_or_default();
            eprintln!(
                "live HubRise call {n}: {method} {} -> {}",
                path.split('?').next().unwrap_or_default(),
                status.as_u16()
            );
            record(n, method.as_str(), &path, status.as_u16(), &body, &bytes);
            let mut response = (
                StatusCode::from_u16(status.as_u16()).unwrap(),
                bytes.to_vec(),
            )
                .into_response();
            if let Some(ct) = content_type {
                response.headers_mut().insert("content-type", ct);
            }
            if let Some(link) = link {
                response.headers_mut().insert("link", link);
            }
            response
        }
        Err(e) => {
            eprintln!("live HubRise call {n}: {method} failed: {e}");
            (StatusCode::BAD_GATEWAY, e.to_string()).into_response()
        }
    }
}

/// Start the pacing proxy; its base URL (`http://127.0.0.1:<port>`).
pub async fn start_proxy() -> String {
    let proxy = Arc::new(Proxy {
        target: std::env::var("PILI_LIVE_HUBRISE_API")
            .unwrap_or_else(|_| "https://api.hubrise.com".into()),
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap(),
        pause: Duration::from_millis(env_number("PILI_LIVE_PAUSE_MS", 1000)),
        cap: env_number("PILI_LIVE_HUBRISE_MAX_REQUESTS", 40) as usize,
        last: tokio::sync::Mutex::new(None),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = axum::Router::new().fallback(forward).with_state(proxy);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    url
}

/// A TestApp on one database file, connected to the live location
/// through the proxy; `None` without a stored token.
pub async fn live_hubrise_app() -> Option<(TestApp, tempfile::TempDir, LiveToken, String)> {
    let Some(token) = live_token() else {
        eprintln!("no target/live-hubrise/token.json: run scripts/live_hubrise_login.sh");
        return None;
    };
    let (app, dir) = TestApp::with_shared_db().await;
    let proxy = start_proxy().await;
    let stored = StoredConnection {
        access_token: token.access_token.clone(),
        account_id: token.account_id.clone(),
        account_name: token.account_name.clone(),
        location_id: token.location_id.clone(),
        location_name: token.location_name.clone(),
        catalog_id: token.catalog_id.clone().expect("the token names a catalog"),
        catalog_name: token.catalog_name.clone(),
        customer_list_id: None,
        customer_list_name: None,
        scope: "location[orders.write,catalog.write]".into(),
        base_url: format!("{proxy}/v1"),
        last_event_cursor: None,
    };
    {
        let db = app.state.poller_db.lock().unwrap();
        token_store::save(&db, &stored).unwrap();
    }
    Some((app, dir, token, proxy))
}

/// A HubRise client through the proxy, acting as the ordering side.
pub fn live_client(app: &TestApp, token: &LiveToken, proxy: &str) -> pos_hubrise::client::Client {
    pos_hubrise::client::Client::new(
        app.state.shared_http.clone(),
        format!("{proxy}/v1"),
        token.access_token.clone(),
    )
}
