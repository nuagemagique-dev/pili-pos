//! The CCV Payment API endpoints for the `terminal` method: create a
//! payment, read a transaction, cancel. HTTP Basic with the API key as
//! user, idempotency through `Idempotency-Reference`.

use std::collections::HashMap;
use std::time::Instant;

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use serde_json::{Value, json};

use crate::state::{
    Begin, Behaviour, Payment, Phase, Protocol, Shared, circuit, customer_receipt, lock,
};

fn json_response(status: u16, body: &Value) -> Response {
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

fn input_error(status: u16, message: &str, field: Option<&str>) -> Response {
    let mut body = json!({"type": "input_error", "message": message});
    if let Some(f) = field {
        body["field"] = json!(f);
    }
    json_response(status, &body)
}

fn basic_ok(shared: &Shared, headers: &HeaderMap) -> bool {
    let Some(key) = lock(shared).api_key.clone() else {
        return true;
    };
    let expected = base64::engine::general_purpose::STANDARD.encode(format!("{key}:"));
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Basic "))
        .is_some_and(|v| v == expected)
}

fn epoch_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn cents_of(v: &Value) -> Option<i64> {
    let f = v.as_f64()?;
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "a mock amount in euros, far below the i64 range"
    )]
    Some((f * 100.0).round() as i64)
}

#[expect(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    reason = "a mock amount in cents, far below 2^52"
)]
fn number(cents: i64) -> Value {
    serde_json::Number::from_f64(cents as f64 / 100.0).map_or(Value::Null, Value::Number)
}

/// The transaction object as CCV shows it.
fn transaction(p: &Payment, phase: Phase) -> Value {
    let mut details = json!({
        "terminalId": p.workstation,
        "managementSystemId": p.sender,
        "accessProtocol": "OPI_NL",
        "operatingEnvironment": "ATTENDED",
        "merchantLanguage": "fra",
    });
    let status = match phase {
        Phase::InProgress => "pending",
        Phase::Stuck => "manualintervention",
        Phase::Approved => "success",
        Phase::Declined | Phase::Cancelled => "failed",
    };
    let mut tx = json!({
        "reference": p.psp_reference,
        "merchantOrderReference": p.reference,
        "amount": number(p.amount_cents),
        "currency": "eur",
        "method": "terminal",
        "type": "sale",
        "status": status,
        "created": epoch_ms(),
        "lastUpdate": epoch_ms(),
    });
    match phase {
        Phase::Approved => {
            // The circuit is "an escaped JSON string", the receipt a JSON array.
            details["cardCircuit"] = json!(json!(circuit(&p.brand)).to_string());
            details["customerReceipt"] = json!(json!(customer_receipt(p)).to_string());
            if p.approved_cents != p.amount_cents {
                details["approvedAmount"] = number(p.approved_cents);
            }
        }
        Phase::Declined => tx["failureCode"] = json!("card_refused"),
        Phase::Cancelled => {
            tx["failureCode"] = json!("cancelled");
            tx["cancelledBy"] = json!("merchant");
        }
        Phase::InProgress | Phase::Stuck => {}
    }
    tx["details"] = details;
    tx
}

pub(crate) async fn create_payment(
    State(shared): State<Shared>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let parsed = serde_json::from_slice::<Value>(&body);
    let log = lock(&shared).log(
        Protocol::Ccv,
        "POST",
        "/api/v1/payment",
        parsed.as_ref().cloned().unwrap_or(Value::Null),
    );
    let response = create_inner(&shared, &headers, parsed).await;
    lock(&shared).set_status(log, response.status().as_u16());
    response
}

#[expect(
    clippy::too_many_lines,
    reason = "one linear validation of the CCV create request, clearer unsplit"
)]
async fn create_inner(
    shared: &Shared,
    headers: &HeaderMap,
    parsed: Result<Value, serde_json::Error>,
) -> Response {
    if !basic_ok(shared, headers) {
        return json_response(
            401,
            &json!({"type": "input_error", "message": "Unauthorized"}),
        );
    }
    let is_json = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"));
    if !is_json {
        return json_response(
            415,
            &json!({"type": "input_error", "message": "Content-Type must be application/json"}),
        );
    }
    let Ok(request) = parsed else {
        return input_error(400, "Invalid JSON", None);
    };
    let Some(cents) = request.get("amount").and_then(cents_of).filter(|c| *c > 0) else {
        return input_error(400, "An amount is required", Some("amount"));
    };
    if request.get("currency").and_then(Value::as_str) != Some("eur") {
        return input_error(400, "currency must be eur", Some("currency"));
    }
    if request.get("method").and_then(Value::as_str) != Some("terminal") {
        return input_error(400, "Only the terminal method is mocked", Some("method"));
    }
    let details = &request["details"];
    let text = |key: &str| {
        details
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
    };
    let (Some(terminal_id), Some(management_id)) = (text("terminalId"), text("managementSystemId"))
    else {
        return input_error(
            400,
            "terminalId and managementSystemId are required",
            Some("details"),
        );
    };
    let Some(merchant_reference) = request
        .get("merchantOrderReference")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    else {
        return input_error(
            400,
            "merchantOrderReference is required",
            Some("merchantOrderReference"),
        );
    };
    let key = headers
        .get("Idempotency-Reference")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty() && v.len() <= 50)
        .unwrap_or(merchant_reference)
        .to_string();

    let begun = lock(shared).begin(
        Protocol::Ccv,
        &key,
        &format!("ccv:{terminal_id}"),
        cents,
        management_id,
        terminal_id,
    );
    let (index, created) = match begun {
        Begin::Busy => {
            return json_response(
                500,
                &json!({"type": "process_error", "message": "TERMINAL_ALREADY_IN_USE"}),
            );
        }
        Begin::Existing(i) => (i, false),
        Begin::Created(i) => (i, true),
    };
    let (payment, delay) = {
        let inner = lock(shared);
        let p = inner.payments[index].clone();
        (
            p.clone(),
            p.done_at.saturating_duration_since(Instant::now()),
        )
    };
    if created && payment.behaviour == Behaviour::Timeout {
        // The gateway gives up: the payment goes on, the answer is lost.
        tokio::time::sleep(delay).await;
        return json_response(
            504,
            &json!({
                "type": "unexpected_error",
                "message": "Gateway Timeout",
                "transactionReference": payment.psp_reference,
            }),
        );
    }
    let phase = payment.phase(Instant::now());
    json_response(200, &transaction(&payment, phase))
}

pub(crate) async fn get_transaction(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let log = lock(&shared).log(Protocol::Ccv, "GET", "/api/v1/transaction", json!(query));
    let response = get_inner(&shared, &headers, &query);
    lock(&shared).set_status(log, response.status().as_u16());
    response
}

fn get_inner(shared: &Shared, headers: &HeaderMap, query: &HashMap<String, String>) -> Response {
    if !basic_ok(shared, headers) {
        return json_response(
            401,
            &json!({"type": "input_error", "message": "Unauthorized"}),
        );
    }
    let inner = lock(shared);
    let now = Instant::now();
    if let Some(reference) = query.get("reference") {
        return match inner.find_psp(reference) {
            Some(i) => {
                let p = &inner.payments[i];
                json_response(200, &transaction(p, p.phase(now)))
            }
            None => json_response(
                404,
                &json!({"type": "input_error", "message": "unknown reference", "failureCode": "unknown_reference"}),
            ),
        };
    }
    if let Some(merchant) = query.get("merchantOrderReference") {
        let list: Vec<Value> = inner
            .payments
            .iter()
            .filter(|p| p.protocol == Protocol::Ccv && &p.reference == merchant)
            .map(|p| transaction(p, p.phase(now)))
            .collect();
        return json_response(200, &Value::Array(list));
    }
    input_error(400, "reference or merchantOrderReference is required", None)
}

pub(crate) async fn cancel(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let log = lock(&shared).log(Protocol::Ccv, "POST", "/api/v1/cancel", json!(query));
    let response = cancel_inner(&shared, &headers, &query);
    lock(&shared).set_status(log, response.status().as_u16());
    response
}

fn cancel_inner(shared: &Shared, headers: &HeaderMap, query: &HashMap<String, String>) -> Response {
    if !basic_ok(shared, headers) {
        return json_response(
            401,
            &json!({"type": "input_error", "message": "Unauthorized"}),
        );
    }
    let Some(reference) = query.get("reference") else {
        return input_error(400, "reference is required", Some("reference"));
    };
    let mut inner = lock(shared);
    let Some(index) = inner.find_psp(reference) else {
        return input_error(400, "unknown reference", Some("reference"));
    };
    // A cancel that comes too late is refused and changes nothing.
    if inner.abort(index) {
        let p = inner.payments[index].clone();
        json_response(200, &transaction(&p, p.phase(Instant::now())))
    } else {
        json_response(
            400,
            &json!({"type": "process_error", "message": "The payment can no longer be cancelled"}),
        )
    }
}
