//! Mock of the Storecove API v2 (Peppol access point), for the engine's invoice tests and
//! the admission image. Routes under [`PREFIX`]:
//!
//! - `POST /storecove/api/v2/discovery/receives`: `{"code":"OK","email":false}` when the
//!   participant (`identifier`) is registered, 404 `{"code":"NOT_FOUND"}` when not.
//! - `POST /storecove/api/v2/document_submissions`: validates the envelope (legal entity,
//!   recipient, base64 document) and answers `{"guid": "mock-guid-N"}`.
//! - `GET /storecove/mock/requests`: every call received, as JSON (the in-process handle
//!   [`StorecoveMock::requests`] gives the same list to Rust tests).
//!
//! Every answer is controllable through a [`StorecoveMock`] handle: required API key,
//! participants that are not registered, scripted statuses for the next submissions and
//! discoveries (a 503 then success, a 422, a 401), and a delay to exercise timeouts. The
//! default mock accepts everything. It never validates the UBL against a Schematron; the live
//! Storecove sandbox does that (owner task, plan Q2).

use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use base64::Engine as _;
use serde_json::{Value, json};

pub const PREFIX: &str = "/storecove/api/v2";

/// One call received, as the request log keeps it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct StorecoveCall {
    /// `discovery/receives` or `document_submissions`.
    pub path: String,
    /// The bearer token that came with the call, if any.
    pub bearer: Option<String>,
    pub body: Value,
    /// The status the mock answered.
    pub status: u16,
}

#[derive(Default)]
struct Inner {
    /// API key required as bearer token. `None`: any (or no) token is accepted.
    required_key: Option<String>,
    /// Participants that are not on Peppol.
    not_registered: HashSet<String>,
    /// Statuses for the next submissions, one per call; then `200`.
    submit_script: VecDeque<u16>,
    /// Statuses for the next discoveries, one per call; then the registry answer.
    lookup_script: VecDeque<u16>,
    delay: Option<Duration>,
    calls: Vec<StorecoveCall>,
    guid_counter: u64,
}

/// A handle on one mock Storecove. Clone it freely; the clones share the state.
#[derive(Clone, Default)]
pub struct StorecoveMock {
    inner: Arc<Mutex<Inner>>,
}

impl std::fmt::Debug for StorecoveMock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorecoveMock").finish_non_exhaustive()
    }
}

impl StorecoveMock {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Require `Authorization: Bearer <key>` on every call (401 otherwise).
    pub fn require_key(&self, key: &str) {
        self.lock().required_key = Some(key.to_string());
    }

    /// Say `participant` (`0208:0308357159`) is not registered on Peppol.
    pub fn set_not_registered(&self, participant: &str) {
        self.lock().not_registered.insert(participant.to_string());
    }

    /// Statuses the next submissions answer, in order (e.g. `[503, 200]`); a status other than
    /// 2xx carries an error body.
    pub fn script_submissions(&self, statuses: &[u16]) {
        self.lock().submit_script = statuses.iter().copied().collect();
    }

    /// Statuses the next discoveries answer, in order.
    pub fn script_lookups(&self, statuses: &[u16]) {
        self.lock().lookup_script = statuses.iter().copied().collect();
    }

    /// Wait this long before answering (a slow access point).
    pub fn set_delay(&self, delay: Option<Duration>) {
        self.lock().delay = delay;
    }

    /// Every call received so far.
    pub fn requests(&self) -> Vec<StorecoveCall> {
        self.lock().calls.clone()
    }

    /// The submissions received (`document_submissions`).
    pub fn submissions(&self) -> Vec<StorecoveCall> {
        self.requests()
            .into_iter()
            .filter(|c| c.path == "document_submissions")
            .collect()
    }

    /// The UBL XML of the `n`th submission (0-based), decoded.
    pub fn submitted_xml(&self, n: usize) -> Option<String> {
        let call = self.submissions().into_iter().nth(n)?;
        let b64 = call.body["document"]["rawDocumentData"]["document"].as_str()?;
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
        String::from_utf8(bytes).ok()
    }

    /// Forget the calls received (the scripts and settings stay).
    pub fn clear_requests(&self) {
        self.lock().calls.clear();
    }
}

/// The mock's routes, answering from `mock`.
pub fn router(mock: StorecoveMock) -> Router {
    Router::new()
        .route("/storecove/api/v2/discovery/receives", post(discovery))
        .route("/storecove/api/v2/document_submissions", post(submission))
        .route("/storecove/mock/requests", get(requests))
        .with_state(mock)
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_string)
}

fn error_body(status: u16) -> Value {
    json!({ "code": format!("MOCK_{status}"), "message": format!("mock answer {status}") })
}

fn log(mock: &StorecoveMock, path: &str, headers: &HeaderMap, body: &Value, status: u16) {
    mock.lock().calls.push(StorecoveCall {
        path: path.to_string(),
        bearer: bearer(headers),
        body: body.clone(),
        status,
    });
}

/// The delay, then the 401 check; `Some(answer)` when the call is refused already.
async fn preamble(mock: &StorecoveMock, headers: &HeaderMap) -> Option<u16> {
    let (delay, key) = {
        let g = mock.lock();
        (g.delay, g.required_key.clone())
    };
    if let Some(d) = delay {
        tokio::time::sleep(d).await;
    }
    match key {
        Some(k) if bearer(headers).as_deref() != Some(k.as_str()) => Some(401),
        _ => None,
    }
}

async fn discovery(
    State(mock): State<StorecoveMock>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let identifier = body["identifier"].as_str().unwrap_or_default().to_string();
    let status = if let Some(s) = preamble(&mock, &headers).await {
        s
    } else {
        let mut g = mock.lock();
        if let Some(s) = g.lookup_script.pop_front() {
            s
        } else if identifier.is_empty() {
            422
        } else if g.not_registered.contains(&identifier) {
            404
        } else {
            200
        }
    };
    log(&mock, "discovery/receives", &headers, &body, status);
    let payload = match status {
        200 => json!({ "code": "OK", "email": false }),
        404 => json!({ "code": "NOT_FOUND" }),
        s => error_body(s),
    };
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(payload),
    )
}

async fn submission(
    State(mock): State<StorecoveMock>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let status = if let Some(s) = preamble(&mock, &headers).await {
        s
    } else {
        let scripted = mock.lock().submit_script.pop_front();
        let valid = body["legalEntityId"].is_i64()
            && body["routing"]["eIdentifiers"][0]["id"].is_string()
            && body["document"]["rawDocumentData"]["document"]
                .as_str()
                .is_some_and(|d| base64::engine::general_purpose::STANDARD.decode(d).is_ok());
        match scripted {
            Some(s) => s,
            None if !valid => 422,
            None => 200,
        }
    };
    log(&mock, "document_submissions", &headers, &body, status);
    let payload = if (200..300).contains(&status) {
        let mut g = mock.lock();
        g.guid_counter += 1;
        json!({ "guid": format!("mock-guid-{}", g.guid_counter) })
    } else {
        error_body(status)
    };
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(payload),
    )
}

async fn requests(State(mock): State<StorecoveMock>) -> Json<Vec<StorecoveCall>> {
    Json(mock.requests())
}
