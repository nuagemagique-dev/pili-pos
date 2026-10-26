//! A card payment terminal for development, demos and the tests of
//! `pos-eft`: one HTTP port serves the Worldline Terminal API v2 and the
//! CCV Payment API (method `terminal`) plus a control API and a page of
//! recent payments; one TCP port is a CCV terminal speaking OPI.
//!
//! A test tool outside the certified build: never a normal dependency of
//! a shipped crate (dev-dependency only), see `README.md`.
//!
//! The server runs on its own thread with its own tokio runtime, so a
//! test can start a [`Mock`] from synchronous or asynchronous code.
//!
//! ```no_run
//! let mock = pos_eft_mock::Mock::start();
//! let url = mock.url(); // http://127.0.0.1:<port>
//! let opi = mock.opi_addr(); // 127.0.0.1:<port>
//! mock.next(pos_eft_mock::Next { delay_ms: 0, ..Default::default() });
//! assert!(mock.payments().is_empty());
//! ```

mod ccv;
mod opi;
mod state;
mod worldline;

use std::fmt::Write as _;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, Uri, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use serde_json::{Value, json};

pub use state::{Behaviour, Next, PaymentView, Protocol, RequestLog};
use state::{Shared, lock, new_shared};

/// Where the mock listens.
#[derive(Debug, Clone)]
pub struct Config {
    /// The HTTP port (both cloud APIs, the control API, the page).
    pub http: SocketAddr,
    /// The OPI terminal (channel 0).
    pub opi: SocketAddr,
    /// The till's channel 1 port, which the OPI terminal connects back
    /// to for the receipt (0 turns the receipt off).
    pub opi_device_port: u16,
    /// When set, both cloud APIs demand this API key.
    pub api_key: Option<String>,
}

impl Config {
    /// `127.0.0.1` on free ports, the usual channel 1 port.
    pub fn local() -> Self {
        Self {
            http: SocketAddr::from(([127, 0, 0, 1], 0)),
            opi: SocketAddr::from(([127, 0, 0, 1], 0)),
            opi_device_port: 4102,
            api_key: None,
        }
    }

    /// The binary's configuration: `PILI_EFT_MOCK_PORT` (19400),
    /// `PILI_EFT_MOCK_OPI_PORT` (19410), `PILI_EFT_MOCK_OPI_DEVICE_PORT`
    /// (4102), `PILI_EFT_MOCK_API_KEY` (none), always on `127.0.0.1`.
    pub fn from_env() -> Result<Self, String> {
        fn port(name: &str, default: u16) -> Result<u16, String> {
            match std::env::var(name) {
                Ok(v) => v
                    .trim()
                    .parse()
                    .map_err(|_| format!("{name}: a port expected, got {v:?}")),
                Err(_) => Ok(default),
            }
        }
        Ok(Self {
            http: SocketAddr::from(([127, 0, 0, 1], port("PILI_EFT_MOCK_PORT", 19400)?)),
            opi: SocketAddr::from(([127, 0, 0, 1], port("PILI_EFT_MOCK_OPI_PORT", 19410)?)),
            opi_device_port: port("PILI_EFT_MOCK_OPI_DEVICE_PORT", 4102)?,
            api_key: std::env::var("PILI_EFT_MOCK_API_KEY")
                .ok()
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty()),
        })
    }
}

/// A running mock. Stops when dropped or on [`Mock::stop`].
pub struct Mock {
    http: SocketAddr,
    opi: SocketAddr,
    state: Shared,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl std::fmt::Debug for Mock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mock")
            .field("http", &self.http)
            .field("opi", &self.opi)
            .finish_non_exhaustive()
    }
}

impl Mock {
    /// Serve on free local ports.
    ///
    /// # Panics
    /// When the local ports cannot be bound or the thread cannot start.
    pub fn start() -> Mock {
        Mock::start_on(Config::local()).expect("pos-eft-mock: cannot start")
    }

    /// Serve where `config` says.
    pub fn start_on(config: Config) -> std::io::Result<Mock> {
        let http_listener = std::net::TcpListener::bind(config.http)?;
        http_listener.set_nonblocking(true)?;
        let opi_listener = std::net::TcpListener::bind(config.opi)?;
        opi_listener.set_nonblocking(true)?;
        let http = http_listener.local_addr()?;
        let opi = opi_listener.local_addr()?;
        let state = new_shared(config.api_key, config.opi_device_port);
        let app = router(Arc::clone(&state));
        let served = Arc::clone(&state);
        let (shutdown, stop) = tokio::sync::oneshot::channel::<()>();
        let thread = std::thread::Builder::new()
            .name("pos-eft-mock".to_string())
            .spawn(move || {
                let Ok(rt) = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                else {
                    return;
                };
                rt.block_on(async move {
                    let (Ok(http_listener), Ok(opi_listener)) = (
                        tokio::net::TcpListener::from_std(http_listener),
                        tokio::net::TcpListener::from_std(opi_listener),
                    ) else {
                        return;
                    };
                    tokio::select! {
                        _ = axum::serve(http_listener, app) => {}
                        () = opi::serve(opi_listener, served) => {}
                        _ = stop => {}
                    }
                });
                // Drop the connection tasks (a delayed answer included) at once.
                rt.shutdown_background();
            })?;
        Ok(Mock {
            http,
            opi,
            state,
            shutdown: Some(shutdown),
            thread: Some(thread),
        })
    }

    /// The HTTP base URL, `http://127.0.0.1:<port>` (a `base_url` for
    /// both cloud kinds).
    pub fn url(&self) -> String {
        format!("http://{}", self.http)
    }

    /// The OPI terminal (an `address` for `ccv_opi`).
    pub fn opi_addr(&self) -> SocketAddr {
        self.opi
    }

    /// The behaviour of the next payment (queued; after the queue, the
    /// default).
    pub fn next(&self, next: Next) {
        lock(&self.state).next.push_back(next);
    }

    /// The behaviour of every payment without a queued one.
    pub fn set_default(&self, next: Next) {
        lock(&self.state).default = next;
    }

    /// Demand this API key on both cloud APIs (`None`: no check).
    pub fn set_api_key(&self, key: Option<&str>) {
        lock(&self.state).api_key = key.map(str::to_string);
    }

    /// The till's channel 1 port the OPI terminal prints to.
    pub fn set_device_port(&self, port: u16) {
        lock(&self.state).device_port = port;
    }

    /// Make the Worldline terminals appear in the merchant's connected
    /// terminals (the default).
    pub fn bring_terminals_online(&self) {
        lock(&self.state).terminals_online = true;
    }

    /// Make the Worldline terminals look offline: the merchant's connected
    /// terminals list is empty.
    pub fn take_terminals_offline(&self) {
        lock(&self.state).terminals_online = false;
    }

    /// Serve the merchant's terminal list only on v1, v2 answering 404.
    pub fn serve_merchant_v1_only(&self) {
        lock(&self.state).merchant_v1_only = true;
    }

    /// Every request received, oldest first.
    pub fn requests(&self) -> Vec<RequestLog> {
        lock(&self.state).requests.clone()
    }

    /// Every payment started, oldest first, with where it stands.
    pub fn payments(&self) -> Vec<PaymentView> {
        lock(&self.state).views()
    }

    /// Forget payments, requests and queued behaviours.
    pub fn reset(&self) {
        reset(&self.state);
    }

    /// Stop serving (idempotent).
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.stop();
    }
}

fn reset(shared: &Shared) {
    let mut inner = lock(shared);
    inner.next.clear();
    inner.default = Next::default();
    inner.payments.clear();
    inner.requests.clear();
    inner.terminals_online = true;
    inner.merchant_v1_only = false;
    inner.opi_last = None;
}

fn router(shared: Shared) -> Router {
    Router::new()
        .route("/api/v1/merchants/{umid}", get(worldline::merchant_v1))
        .route("/api/v2/merchants/{umid}", get(worldline::merchant))
        .route(
            "/api/v2/merchants/{umid}/terminals/{utid}/payments/sync",
            post(worldline::payments_sync),
        )
        .route(
            "/api/v2/merchants/{umid}/terminals/{utid}/status/sync",
            post(worldline::status_sync),
        )
        .route(
            "/api/v2/merchants/{umid}/terminals/{utid}/payments/abort",
            post(worldline::abort),
        )
        .route("/api/v1/payment", post(ccv::create_payment))
        .route("/api/v1/transaction", get(ccv::get_transaction))
        .route("/api/v1/cancel", post(ccv::cancel))
        .route("/mock/next", post(set_next))
        .route("/mock/requests", get(list_requests))
        .route("/mock/payments", get(list_payments))
        .route("/mock/reset", post(reset_all))
        .route("/", get(page))
        .fallback(not_found)
        .with_state(shared)
}

async fn not_found(State(shared): State<Shared>, uri: Uri) -> Response {
    lock(&shared).log(Protocol::Ccv, "?", uri.path(), Value::Null);
    (StatusCode::NOT_FOUND, "no such endpoint in pos-eft-mock").into_response()
}

async fn set_next(State(shared): State<Shared>, body: axum::body::Bytes) -> Response {
    let next: Next = if body.iter().all(u8::is_ascii_whitespace) {
        Next::default()
    } else {
        match serde_json::from_slice(&body) {
            Ok(n) => n,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, "application/json")],
                    json!({"error": e.to_string()}).to_string(),
                )
                    .into_response();
            }
        }
    };
    lock(&shared).next.push_back(next.clone());
    axum::Json(next).into_response()
}

async fn list_requests(State(shared): State<Shared>) -> axum::Json<Vec<RequestLog>> {
    axum::Json(lock(&shared).requests.clone())
}

async fn list_payments(State(shared): State<Shared>) -> axum::Json<Vec<PaymentView>> {
    axum::Json(lock(&shared).views())
}

async fn reset_all(State(shared): State<Shared>) -> StatusCode {
    reset(&shared);
    StatusCode::NO_CONTENT
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The payments, newest first, refreshed every 3 s.
async fn page(State(shared): State<Shared>) -> Html<String> {
    let views = lock(&shared).views();
    let mut html = String::from(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta http-equiv=\"refresh\" content=\"3\"><title>Payment terminal mock</title>\
         <style>body{font-family:sans-serif;background:#eee;margin:16px}\
         table{border-collapse:collapse;background:#fff}td,th{padding:6px 12px;border-bottom:1px solid #ddd;text-align:left}\
         .approved{color:#0a7a2f}.declined,.cancelled{color:#b3261e}.in_progress{color:#946200}</style></head><body>\
         <h1>Payment terminal mock</h1>\
         <p>Control: <code>POST /mock/next</code>, <code>GET /mock/requests</code>, <code>POST /mock/reset</code></p>\
         <table><tr><th>#</th><th>Time</th><th>Protocol</th><th>Reference</th><th>Amount</th><th>Brand</th><th>Outcome</th></tr>",
    );
    for v in views.iter().rev().take(50) {
        let _ = write!(
            html,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>EUR {}</td><td>{}</td><td class=\"{}\">{}</td></tr>",
            v.id,
            escape(&v.at),
            serde_json::to_value(v.protocol)
                .ok()
                .and_then(|p| p.as_str().map(str::to_string))
                .unwrap_or_default(),
            escape(&v.reference),
            state::amount_text(v.amount_cents),
            escape(&v.brand),
            v.outcome,
            v.outcome.replace('_', " "),
        );
    }
    if views.is_empty() {
        html.push_str("<tr><td colspan=\"7\">No payment yet.</td></tr>");
    }
    html.push_str("</table></body></html>");
    Html(html)
}
