//! A `TestApp` that talks to a live FDM (the module2 cloud simulator),
//! gently.
//!
//! The app's `fdm_url` points at a local pacing proxy, which forwards
//! each call to `MOYUN_LIVE_FDM_URL`:
//!
//! - one call at a time, at least `MOYUN_LIVE_PAUSE_MS` (default 1000)
//!   after the previous one;
//! - at most `MOYUN_LIVE_MAX_REQUESTS` (default 60) calls per test
//!   binary run; past the cap the proxy answers a GraphQL error itself
//!   and forwards nothing;
//! - the request body and the `Authorization` header pass through
//!   untouched and are never logged.
//!
//! The bearer token reaches the app the way it does in production:
//! `fdm_token_file` names a file holding `MOYUN_LIVE_FDM_TOKEN`.
//!
//! A live FDM keeps its counters, so the POS's own ticket numbers
//! (`fiscal_counters`) are carried from one run to the next in
//! `target/live-fdm/state.json`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::post;

use super::fixtures::workspace_root;
use super::harness::TestApp;

/// Calls forwarded to the live FDM by this test binary.
static FORWARDED: AtomicUsize = AtomicUsize::new(0);

/// The test setup identity module2 allows (UC_TestSetup_20240806).
pub const LIVE_POS_ID: &str = "CPOS0031234567";

/// The live FDM from the environment, or `None` to skip.
pub fn live_url() -> Option<String> {
    std::env::var("MOYUN_LIVE_FDM_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
}

fn env_number(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Calls forwarded so far by this test binary.
pub fn live_requests() -> usize {
    FORWARDED.load(Ordering::SeqCst)
}

struct Proxy {
    target: String,
    client: reqwest::Client,
    pause: Duration,
    cap: usize,
    last: tokio::sync::Mutex<Option<Instant>>,
}

async fn forward(
    State(proxy): State<std::sync::Arc<Proxy>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // One call at a time: the lock is held for the whole round trip.
    let mut last = proxy.last.lock().await;
    if FORWARDED.load(Ordering::SeqCst) >= proxy.cap {
        let body = serde_json::json!({
            "data": null,
            "errors": [{"message": format!(
                "live request cap reached ({}), raise MOYUN_LIVE_MAX_REQUESTS",
                proxy.cap
            )}]
        });
        return (StatusCode::OK, body.to_string()).into_response();
    }
    if let Some(at) = *last {
        let since = at.elapsed();
        if since < proxy.pause {
            tokio::time::sleep(proxy.pause - since).await;
        }
    }
    let n = FORWARDED.fetch_add(1, Ordering::SeqCst) + 1;
    let mut request = proxy
        .client
        .post(&proxy.target)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body);
    if let Some(auth) = headers.get(header::AUTHORIZATION) {
        request = request.header(header::AUTHORIZATION, auth);
    }
    let answer = request.send().await;
    *last = Some(Instant::now());
    match answer {
        Ok(response) => {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            eprintln!("live FDM call {n}: HTTP {status}");
            let status = StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            (status, text).into_response()
        }
        Err(e) => {
            eprintln!("live FDM call {n}: transport error {e}");
            (StatusCode::BAD_GATEWAY, e.to_string()).into_response()
        }
    }
}

/// Start the pacing proxy in front of `target`; returns its GraphQL URL.
async fn start_proxy(target: String) -> String {
    let proxy = std::sync::Arc::new(Proxy {
        target,
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap(),
        pause: Duration::from_millis(env_number("MOYUN_LIVE_PAUSE_MS", 1000)),
        cap: env_number("MOYUN_LIVE_MAX_REQUESTS", 60) as usize,
        last: tokio::sync::Mutex::new(None),
    });
    let router = axum::Router::new()
        .route("/graphql", post(forward))
        .with_state(proxy);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{addr}/graphql")
}

/// Where the POS ticket numbers of the previous live runs are kept.
fn state_file() -> PathBuf {
    workspace_root().join("target/live-fdm/state.json")
}

impl TestApp {
    /// An app signing against the live FDM named by
    /// `MOYUN_LIVE_FDM_URL`, through the pacing proxy, with the test
    /// setup identity module2 allows. Ticket numbers go on from the
    /// previous live run.
    pub async fn live(target: String) -> Self {
        let mut app = TestApp::new().await;
        app.live = true;
        let url = start_proxy(target).await;
        {
            let mut config = app.state.config.lock().unwrap();
            config.fdm_url = url;
            config.pos_id = std::env::var("MOYUN_LIVE_POS_ID").unwrap_or(LIVE_POS_ID.into());
            config.terminal_id = "POS-1-BAR".into();
            config.device_id = "F0:1D:3D:CC:13:C8".into();
            if let Ok(token) = std::env::var("MOYUN_LIVE_FDM_TOKEN")
                && !token.trim().is_empty()
            {
                let dir = workspace_root().join("target/live-fdm");
                std::fs::create_dir_all(&dir).unwrap();
                let file = dir.join("fdm.token");
                std::fs::write(&file, token.trim()).unwrap();
                config.fdm_token_file = Some(file.to_string_lossy().into_owned());
            }
        }
        app.restore_ticket_numbers();
        app
    }

    /// Seed `fiscal_counters` from the previous live runs.
    fn restore_ticket_numbers(&self) {
        let Ok(raw) = std::fs::read_to_string(state_file()) else {
            return;
        };
        let saved: BTreeMap<String, i64> = serde_json::from_str(&raw).unwrap_or_default();
        let conn = self.state.db.lock().unwrap();
        for (label, value) in saved {
            conn.execute(
                "INSERT INTO fiscal_counters (event_label, current_value) VALUES (?1, ?2)
                 ON CONFLICT(event_label) DO UPDATE SET current_value = excluded.current_value",
                rusqlite::params![label, value],
            )
            .unwrap();
        }
    }

    /// Keep this run's ticket numbers for the next live run.
    pub fn save_ticket_numbers(&self) {
        let conn = self.state.db.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT event_label, current_value FROM fiscal_counters")
            .unwrap();
        let counters: BTreeMap<String, i64> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let file = state_file();
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, serde_json::to_string_pretty(&counters).unwrap()).unwrap();
    }
}
