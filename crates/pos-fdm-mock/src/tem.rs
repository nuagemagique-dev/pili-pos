//! Mock of the Scaleway Transactional Email API, for the admission
//! image and screenshots without internet.
//!
//! - `POST /transactional-email/v1alpha1/regions/{region}/emails`
//!   writes the email to `MOCK_TEM_OUTBOX` (default `mock-tem-outbox`)
//!   as `<id>.json` (the request) and `<id>.txt` (the text body), and
//!   answers like TEM: `{"emails":[{"id":…,"message_id":…}]}`.
//! - `GET` on the same path is the POS's reachability probe.
//! - `MOCK_TEM_FAIL=1` answers 503 to both, to show the paper fallback.
//! - A request without `X-Auth-Token` gets 401, like the real API.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use axum::Json;
use axum::Router;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use serde_json::{Value, json};

static EMAIL_COUNTER: AtomicU64 = AtomicU64::new(0);

pub const ROUTE: &str = "/transactional-email/v1alpha1/regions/{region}/emails";

pub fn router() -> Router {
    Router::new().route(ROUTE, get(probe).post(send))
}

fn outbox_dir() -> PathBuf {
    std::env::var("MOCK_TEM_OUTBOX")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("mock-tem-outbox"))
}

fn failing() -> bool {
    std::env::var("MOCK_TEM_FAIL").is_ok_and(|v| v == "1")
}

fn authorized(headers: &HeaderMap) -> bool {
    headers
        .get("x-auth-token")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| !v.trim().is_empty())
}

async fn probe(headers: HeaderMap) -> (StatusCode, Json<Value>) {
    answer_probe(authorized(&headers), failing())
}

async fn send(headers: HeaderMap, Json(body): Json<Value>) -> (StatusCode, Json<Value>) {
    accept_email(authorized(&headers), failing(), &outbox_dir(), &body)
}

pub fn answer_probe(authorized: bool, failing: bool) -> (StatusCode, Json<Value>) {
    if !authorized {
        return unauthorized();
    }
    if failing {
        return unavailable();
    }
    (
        StatusCode::OK,
        Json(json!({ "emails": [], "total_count": 0 })),
    )
}

/// Validate and store one email; the pure part of the POST handler.
pub fn accept_email(
    authorized: bool,
    failing: bool,
    dir: &Path,
    body: &Value,
) -> (StatusCode, Json<Value>) {
    if !authorized {
        return unauthorized();
    }
    if failing {
        return unavailable();
    }
    let to = body["to"][0]["email"].as_str().unwrap_or_default();
    let from = body["from"]["email"].as_str().unwrap_or_default();
    let subject = body["subject"].as_str().unwrap_or_default();
    let text = body["text"].as_str().unwrap_or_default();
    if to.is_empty() || from.is_empty() || subject.is_empty() || text.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "from, to, subject and text are required" })),
        );
    }

    let n = EMAIL_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    let id = format!("mock-{}-{n}", chrono::Utc::now().format("%Y%m%dT%H%M%S"));
    if let Err(e) = std::fs::create_dir_all(dir)
        .and_then(|()| std::fs::write(dir.join(format!("{id}.json")), body.to_string()))
        .and_then(|()| std::fs::write(dir.join(format!("{id}.txt")), text))
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": format!("mock outbox: {e}") })),
        );
    }
    println!("[TEM] email {id} → {to} ({subject})");
    (
        StatusCode::OK,
        Json(json!({
            "emails": [{
                "id": id,
                "message_id": format!("{id}@mock-tem"),
                "status": "new",
                "mail_rcpt": to,
            }]
        })),
    )
}

fn unauthorized() -> (StatusCode, Json<Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "message": "authentication is denied", "type": "denied_authentication" })),
    )
}

fn unavailable() -> (StatusCode, Json<Value>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "message": "service unavailable (MOCK_TEM_FAIL=1)" })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn email() -> Value {
        json!({
            "from": { "email": "tickets@example.be", "name": "Restaurant" },
            "to": [{ "email": "client@example.be" }],
            "subject": "Votre ticket",
            "text": "TICKET\nhttps://www.gs2.be/x\n",
            "html": "<pre>TICKET</pre>",
            "project_id": "p",
            "attachments": []
        })
    }

    fn tmp() -> PathBuf {
        let n = EMAIL_COUNTER.fetch_add(1000, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("pos-fdm-mock-tem-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn an_email_is_stored_and_answered_like_tem() {
        let dir = tmp();
        let (status, Json(body)) = accept_email(true, false, &dir, &email());
        assert_eq!(status, StatusCode::OK);
        let id = body["emails"][0]["id"].as_str().unwrap();
        assert!(
            body["emails"][0]["message_id"]
                .as_str()
                .unwrap()
                .contains(id)
        );
        let text = std::fs::read_to_string(dir.join(format!("{id}.txt"))).unwrap();
        assert!(text.starts_with("TICKET"));
        let stored: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{id}.json"))).unwrap())
                .unwrap();
        assert_eq!(stored["to"][0]["email"], "client@example.be");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn each_email_gets_its_own_id() {
        let dir = tmp();
        let (_, Json(a)) = accept_email(true, false, &dir, &email());
        let (_, Json(b)) = accept_email(true, false, &dir, &email());
        assert_ne!(a["emails"][0]["id"], b["emails"][0]["id"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_token_is_refused() {
        let dir = tmp();
        assert_eq!(
            accept_email(false, false, &dir, &email()).0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(answer_probe(false, false).0, StatusCode::UNAUTHORIZED);
        assert!(!dir.exists(), "nothing written");
    }

    #[test]
    fn the_failure_switch_answers_503() {
        let dir = tmp();
        assert_eq!(
            accept_email(true, true, &dir, &email()).0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(answer_probe(true, true).0, StatusCode::SERVICE_UNAVAILABLE);
        assert!(!dir.exists());
    }

    #[test]
    fn an_incomplete_email_is_a_bad_request() {
        let dir = tmp();
        for field in ["to", "from", "subject", "text"] {
            let mut e = email();
            e.as_object_mut().unwrap().remove(field);
            assert_eq!(
                accept_email(true, false, &dir, &e).0,
                StatusCode::BAD_REQUEST,
                "{field}"
            );
        }
    }

    #[test]
    fn the_probe_answers_ok() {
        let (status, Json(body)) = answer_probe(true, false);
        assert_eq!(status, StatusCode::OK);
        assert!(body["emails"].as_array().unwrap().is_empty());
    }

    #[test]
    fn the_route_matches_the_pos_client_url() {
        // pos-mail's TemClient posts to this path (tem.rs::send_url).
        assert_eq!(
            ROUTE.replace("{region}", "fr-par"),
            "/transactional-email/v1alpha1/regions/fr-par/emails"
        );
    }
}
