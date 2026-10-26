//! A `TestApp` that talks to a live FDM (the module2 cloud simulator),
//! gently.
//!
//! The app's `fdm_url` points at a local pacing proxy, which forwards
//! each call to `PILI_LIVE_FDM_URL`:
//!
//! - one call at a time, at least `PILI_LIVE_PAUSE_MS` (default 1000)
//!   after the previous one;
//! - at most `PILI_LIVE_MAX_REQUESTS` (default 60) calls per test
//!   binary run; past the cap the proxy answers a GraphQL error itself
//!   and forwards nothing;
//! - the request body and the `Authorization` header pass through
//!   untouched and are never logged.
//!
//! `PILI_LIVE_SIMULATOR_VERSION`, when set, is sent as the
//! `Simulator-Version` header module2's simulator reads (without it,
//! the simulator answers as its latest version).
//!
//! The bearer token reaches the app the way it does in production:
//! `fdm_token_file` names a file holding `PILI_LIVE_FDM_TOKEN`.
//!
//! A live FDM keeps its counters, so the POS's own ticket numbers
//! (`fiscal_counters`) are carried from one run to the next in
//! `target/live-fdm/state.json`.
//!
//! With `PILI_LIVE_RECORD_DIR` set, the proxy also writes each call
//! there as it went over the wire: `<label>_request.json` (the GraphQL
//! body) and `<label>_response.json` (the answer, with the simulator's
//! `visitor_ip` removed), plus a line in `calls.jsonl` (label, HTTP
//! status, whether a token was sent). The label is the one set with
//! [`record_as`] when it starts with the operation's name, else the
//! operation's name.

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

/// The label of the next recorded call (see [`record_as`]).
static NEXT_LABEL: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Record the next call under `label` (with `PILI_LIVE_RECORD_DIR`).
pub fn record_as(label: &str) {
    *NEXT_LABEL.lock().unwrap() = Some(label.to_string());
}

/// The operations a GraphQL body asks for: its sign mutations, joined
/// with `+`, else `status` / `Status`, else `other`.
fn operation_name(body: &[u8]) -> String {
    let query = serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v["query"].as_str().map(str::to_string))
        .unwrap_or_default();
    let mut signs = Vec::new();
    let mut rest = query.as_str();
    while let Some(at) = rest.find("sign") {
        let name: String = rest[at..]
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        rest = &rest[at + name.len()..];
        if rest.trim_start().starts_with('(') {
            signs.push(name);
        }
    }
    if !signs.is_empty() {
        return signs.join("+");
    }
    for name in ["Status", "status"] {
        if query.contains(&format!("{name}(")) || query.contains(&format!("{name} {{")) {
            return name.to_string();
        }
    }
    "other".to_string()
}

/// Drop every `visitor_ip` (the module2 simulator echoes the caller's
/// public address in its `info` object).
fn scrub(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.remove("visitor_ip");
            map.values_mut().for_each(scrub);
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(scrub),
        _ => {}
    }
}

/// Write one call to `PILI_LIVE_RECORD_DIR`, when set.
/// Whether a recorded call carried the FDM token.
#[derive(Clone, Copy)]
enum Token {
    Sent,
    Absent,
}

fn record(request: &[u8], token: Token, status: u16, response: &str) {
    use std::io::Write;
    let Ok(dir) = std::env::var("PILI_LIVE_RECORD_DIR") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    // A label names the call of its own operation only: an extra call
    // in between (a status check) keeps its operation's name.
    let operation = operation_name(request);
    let mut next = NEXT_LABEL.lock().unwrap();
    let mut label = match next.as_deref() {
        Some(l) if l.starts_with(&operation) => next.take().unwrap(),
        _ => operation.clone(),
    };
    drop(next);
    let base = label.clone();
    for n in 2.. {
        if !dir.join(format!("{label}_request.json")).exists() {
            break;
        }
        label = format!("{base}_{n}");
    }
    let as_json = |raw: &[u8]| {
        serde_json::from_slice::<serde_json::Value>(raw)
            .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(raw).into()))
    };
    let mut answer = as_json(response.as_bytes());
    scrub(&mut answer);
    let pretty = |v: &serde_json::Value| format!("{}\n", serde_json::to_string_pretty(v).unwrap());
    std::fs::write(
        dir.join(format!("{label}_request.json")),
        pretty(&as_json(request)),
    )
    .unwrap();
    std::fs::write(dir.join(format!("{label}_response.json")), pretty(&answer)).unwrap();
    let line = serde_json::json!({
        "label": label,
        "operation": operation,
        "httpStatus": status,
        "token": matches!(token, Token::Sent),
    });
    let mut calls = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("calls.jsonl"))
        .unwrap();
    writeln!(calls, "{line}").unwrap();
}

/// The test setup identity module2 allows (`UC_TestSetup_20240806`).
pub const LIVE_POS_ID: &str = "CPOS0031234567";

/// The live FDM from the environment, or `None` to skip.
pub fn live_url() -> Option<String> {
    std::env::var("PILI_LIVE_FDM_URL")
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
    simulator_version: Option<String>,
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
                "live request cap reached ({}), raise PILI_LIVE_MAX_REQUESTS",
                proxy.cap
            )}]
        });
        return (StatusCode::OK, body.to_string()).into_response();
    }
    if let Some(at) = *last {
        let since = at.elapsed();
        if since < proxy.pause {
            tokio::time::sleep(proxy.pause.checked_sub(since).unwrap()).await;
        }
    }
    let n = FORWARDED.fetch_add(1, Ordering::SeqCst) + 1;
    let with_token = headers.contains_key(header::AUTHORIZATION);
    let mut request = proxy
        .client
        .post(&proxy.target)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body.clone());
    if let Some(auth) = headers.get(header::AUTHORIZATION) {
        request = request.header(header::AUTHORIZATION, auth);
    }
    if let Some(version) = &proxy.simulator_version {
        request = request.header("Simulator-Version", version);
    }
    let answer = request.send().await;
    *last = Some(Instant::now());
    match answer {
        Ok(response) => {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            eprintln!("live FDM call {n}: HTTP {status}");
            record(
                &body,
                if with_token {
                    Token::Sent
                } else {
                    Token::Absent
                },
                status.as_u16(),
                &text,
            );
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
        pause: Duration::from_millis(env_number("PILI_LIVE_PAUSE_MS", 1000)),
        cap: usize::try_from(env_number("PILI_LIVE_MAX_REQUESTS", 60)).unwrap_or(usize::MAX),
        simulator_version: std::env::var("PILI_LIVE_SIMULATOR_VERSION")
            .ok()
            .filter(|v| !v.trim().is_empty()),
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
    /// `PILI_LIVE_FDM_URL`, through the pacing proxy, with the test
    /// setup identity module2 allows. Ticket numbers go on from the
    /// previous live run.
    pub async fn live(target: String) -> Self {
        let mut app = TestApp::new().await;
        app.sign_on_live_fdm(target).await;
        app
    }

    /// Sign this app's events on the live FDM `target` from now on,
    /// through the pacing proxy, with the identity module2 allows and
    /// the ticket numbers of the previous live runs.
    pub async fn sign_on_live_fdm(&mut self, target: String) {
        self.live = true;
        let app = &*self;
        let url = start_proxy(target).await;
        {
            let mut config = app.state.config.lock().unwrap();
            config.fdm_url = url;
            config.pos_id = std::env::var("PILI_LIVE_POS_ID").unwrap_or(LIVE_POS_ID.into());
            config.terminal_id = "POS-1-BAR".into();
            config.device_id = "F0:1D:3D:CC:13:C8".into();
            if let Ok(token) = std::env::var("PILI_LIVE_FDM_TOKEN")
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
