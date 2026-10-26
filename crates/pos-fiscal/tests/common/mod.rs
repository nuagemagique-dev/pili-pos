//! Shared wiremock responder for the pos-fiscal client tests.
//!
//! The client refuses any sign response that does not echo its request
//! (`pos_fiscal::response_check`). Canned responses built by these
//! tests use fixed identities, so they are replayed through
//! [`EchoRequest`], which copies the identity of the request actually
//! sent, as a real FDM does. The app harness has the same responder
//! (`src-tauri/tests/common/harness.rs`).

#![expect(
    dead_code,
    reason = "each test crate includes this module and uses only part of it"
)]

use serde_json::Value;
use wiremock::{Request, Respond, ResponseTemplate};

pub(crate) struct EchoRequest(pub Value);

impl Respond for EchoRequest {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(answer(&self.0, &request.body))
    }
}

/// The envelope with each sign result's identity taken from the request.
pub(crate) fn answer(envelope: &Value, request_body: &[u8]) -> Value {
    let mut envelope = envelope.clone();
    let Ok(request) = serde_json::from_slice::<Value>(request_body) else {
        return envelope;
    };
    let variables = &request["variables"];
    let Some(fields) = envelope.get_mut("data").and_then(Value::as_object_mut) else {
        return envelope;
    };
    for (field, result) in fields.iter_mut() {
        let input = match field.as_str() {
            "signReportTurnoverZ" if variables.get("turnover").is_some() => &variables["turnover"],
            "signReportUserZ" if variables.get("user").is_some() => &variables["user"],
            _ => &variables["data"],
        };
        echo_identity(result, input);
    }
    envelope
}

pub(crate) fn echo_identity(result: &mut Value, input: &Value) {
    let (Some(result), Some(input)) = (result.as_object_mut(), input.as_object()) else {
        return;
    };
    if !result.contains_key("digitalSignature") {
        return;
    }
    for key in ["posId", "posFiscalTicketNo", "posDateTime", "deviceId"] {
        if let Some(value) = input.get(key) {
            result.insert(key.to_string(), value.clone());
        }
    }
    match input.get("terminalId") {
        Some(value) => result.insert("terminalId".to_string(), value.clone()),
        None => result.remove("terminalId"),
    };
}
