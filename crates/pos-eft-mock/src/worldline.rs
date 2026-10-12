//! The Worldline Terminal API v2 endpoints: synchronous payment, status
//! and abort, with the checks the real API does (bearer key, UUID
//! `ExchangeIdentification`, `SalesSystemInfo`).

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use crate::state::{
    Begin, Behaviour, Payment, Phase, Protocol, Shared, auth_code, brand_upper, customer_receipt,
    lock, wait_done,
};

fn json_response(status: u16, body: &Value) -> Response {
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

fn error(status: u16, message: &str) -> Response {
    json_response(
        status,
        &json!({"message": message, "requestId": uuid::Uuid::new_v4().to_string()}),
    )
}

fn bearer_ok(shared: &Shared, headers: &HeaderMap) -> bool {
    let Some(key) = lock(shared).api_key.clone() else {
        return true;
    };
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|v| v == key)
}

fn text<'a>(v: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(v, |cur, key| cur.get(*key))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
}

/// The nexo header rules of v2.
fn check_header(h: &Value) -> Result<String, String> {
    let exchange =
        text(h, &["ExchangeIdentification"]).ok_or("Header.ExchangeIdentification is required")?;
    if uuid::Uuid::parse_str(exchange).is_err() {
        return Err("ExchangeIdentification must be a valid UUID".into());
    }
    if !text(h, &["ProtocolVersion"]).is_some_and(|v| v.starts_with("5.1-WL2")) {
        return Err("ProtocolVersion must be 5.1-WL2.x.x".into());
    }
    if text(h, &["CreationDateTime"]).is_none() {
        return Err("Header.CreationDateTime is required".into());
    }
    if text(h, &["InitiatingParty", "Identification"]).is_none() {
        return Err("Header.InitiatingParty.Identification is required".into());
    }
    if text(h, &["SalesSystemInfo", "IntegratorId"]).is_none() {
        return Err("Header.SalesSystemInfo.IntegratorId is required".into());
    }
    let software = h
        .get("SalesSystemInfo")
        .and_then(|s| s.get("SaleSoftware"))
        .and_then(Value::as_array);
    let complete = software.is_some_and(|list| {
        !list.is_empty()
            && list.iter().all(|s| {
                text(s, &["Type"]).is_some() && text(s, &["Status", "VersionNumber"]).is_some()
            })
    });
    if !complete {
        return Err("SalesSystemInfo.SaleSoftware needs a Type and a Status.VersionNumber".into());
    }
    Ok(exchange.to_string())
}

fn cents_of(v: &Value) -> Option<i64> {
    let f = v.as_f64()?;
    #[allow(clippy::cast_possible_truncation)]
    Some((f * 100.0).round() as i64)
}

fn number(cents: i64) -> Value {
    serde_json::Number::from_f64(cents as f64 / 100.0).map_or(Value::Null, Value::Number)
}

fn header_echo(function: &str, exchange: &str) -> Value {
    json!({
        "MessageFunction": function,
        "ProtocolVersion": "5.1-WL2.1.1",
        "ExchangeIdentification": exchange,
        "CreationDateTime": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string(),
        "InitiatingParty": {"Identification": "WL000D5T0000004", "Type": "Merchant"},
    })
}

fn environment(p: &Payment, utid: &str) -> Value {
    json!({
        "Acquirer": {"Identification": {"Identification": "00999100022"}},
        "Merchant": {"Identification": {"Identification": "WL000D5T0000004"}},
        "POI": {"Identification": {"Identification": utid}},
        "Card": {"CardBrand": brand_upper(&p.brand), "MaskedPAN": "XXXXXXXXXXXX0119"},
        "Cardholder": {"Language": "FR"},
    })
}

fn payment_block(p: &Payment) -> Value {
    let receipt = customer_receipt(p).join("\n");
    json!({
        "SaleTransactionIdentification": {
            "TransactionDateTime": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string(),
            "TransactionReference": p.reference,
        },
        "POITransactionIdentification": {
            "TransactionDateTime": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string(),
            "TransactionReference": format!("2;00999100022;{:011};{};", 19_940_829_000 + p.id, 79_000 + p.id),
        },
        "RetailerPaymentResult": {
            "TransactionType": "CardPayment",
            "RequestedTransaction": {
                "TransactionType": "CardPayment",
                "TransactionDetails": {"TotalAmount": number(p.approved_cents), "Currency": "EUR"},
            },
            "TransactionResponse": {"AuthorisationResult": {
                "ResponseToAuthorisation": {"Response": "Approved"},
                "AuthorisationCode": format!("{:06}", auth_code(p)),
            }},
            "CustomerLanguage": "FR",
        },
        "PaymentReceipt": [{
            "DocumentQualifier": "CustomerReceipt",
            "OutputContent": {
                "MessageDestination": "CardholderReceipt",
                "Format": "SimpleText",
                "MessageContent": receipt,
            },
        }],
    })
}

fn failure(reason: &str, info: &str) -> Value {
    json!({"Response": "Failed", "ResponseReason": reason, "AdditionalResponseInformation": info})
}

/// The payment answer for a payment that reached a final phase.
fn payment_answer(p: &Payment, phase: Phase, utid: &str) -> Value {
    let (response, payment) = match phase {
        Phase::Approved => (json!({"Response": "Success"}), Some(payment_block(p))),
        Phase::Cancelled => (
            failure("Aborted", "WPI_ERR_COND_ABORTED:Payment aborted!"),
            None,
        ),
        _ => (failure("Refusal", "Declined by the issuer"), None),
    };
    let mut service = json!({
        "Environment": environment(p, utid),
        "Context": {"SaleContext": {"SaleIdentification": "pili-pos"}},
        "ServiceContent": "FinancialPaymentResponse",
        "Response": response,
    });
    if let Some(payment) = payment {
        service["PaymentResponse"] = payment;
    }
    json!({"data": {"SaleToPOIServiceResponse": {
        "Header": header_echo("SaleFinancialServiceResponse", &p.reference),
        "ServiceResponse": service,
    }}, "requestId": uuid::Uuid::new_v4().to_string()})
}

fn busy_answer(exchange: &str, utid: &str) -> Value {
    json!({"data": {"SaleToPOIServiceResponse": {
        "Header": header_echo("SaleFinancialServiceResponse", exchange),
        "ServiceResponse": {
            "Environment": {"POI": {"Identification": {"Identification": utid}}},
            "Context": {"SaleContext": {"SaleIdentification": "pili-pos"}},
            "ServiceContent": "FinancialPaymentResponse",
            "Response": failure("Busy", "Terminal is processing another transaction"),
        },
    }}, "requestId": uuid::Uuid::new_v4().to_string()})
}

fn parse(body: &Bytes) -> Option<Value> {
    serde_json::from_slice::<Value>(body).ok()
}

pub async fn payments_sync(
    State(shared): State<Shared>,
    Path((umid, utid)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let path = format!("/api/v2/merchants/{umid}/terminals/{utid}/payments/sync");
    let parsed = serde_json::from_slice::<Value>(&body);
    let log = lock(&shared).log(
        Protocol::Worldline,
        "POST",
        &path,
        parsed.as_ref().cloned().unwrap_or(Value::Null),
    );
    let response = payments_sync_inner(&shared, &umid, &utid, &headers, parsed).await;
    lock(&shared).set_status(log, response.status().as_u16());
    response
}

async fn payments_sync_inner(
    shared: &Shared,
    umid: &str,
    utid: &str,
    headers: &HeaderMap,
    parsed: Result<Value, serde_json::Error>,
) -> Response {
    if !bearer_ok(shared, headers) {
        return error(401, "Unauthorized");
    }
    let Ok(request) = parsed else {
        return error(400, "Invalid JSON");
    };
    let Some(root) = request.get("SaleToPOIServiceRequest") else {
        return error(400, "SaleToPOIServiceRequest is required");
    };
    let exchange = match check_header(&root["Header"]) {
        Ok(e) => e,
        Err(m) => return error(400, &m),
    };
    let service = &root["ServiceRequest"];
    if text(service, &["ServiceContent"]) != Some("FinancialPaymentRequest") {
        return error(400, "ServiceContent must be FinancialPaymentRequest");
    }
    if text(
        service,
        &["Environment", "POI", "Identification", "Identification"],
    )
    .is_none()
    {
        return error(400, "Environment.POI.Identification is required");
    }
    let transaction = &service["PaymentRequest"]["PaymentTransaction"];
    if text(transaction, &["TransactionType"]) != Some("CardPayment") {
        return error(400, "Only CardPayment is mocked");
    }
    if text(
        transaction,
        &["TransactionIdentification", "TransactionReference"],
    )
    .is_none()
    {
        return error(400, "TransactionReference is required");
    }
    let details = &transaction["TransactionDetails"];
    let Some(cents) = details
        .get("TotalAmount")
        .and_then(cents_of)
        .filter(|c| *c > 0)
    else {
        return error(400, "TotalAmount must be a positive number");
    };
    if text(details, &["Currency"]) != Some("EUR") {
        return error(400, "Currency must be EUR");
    }

    let terminal = format!("wl:{umid}:{utid}");
    let begun = lock(shared).begin(Protocol::Worldline, &exchange, &terminal, cents, "", "");
    let index = match begun {
        Begin::Busy => return json_response(200, &busy_answer(&exchange, utid)),
        Begin::Existing(i) | Begin::Created(i) => i,
    };
    let (payment, phase) = wait_done(shared, index).await;
    let lost = matches!(payment.behaviour, Behaviour::Timeout | Behaviour::Unknown)
        && phase != Phase::Cancelled;
    if lost {
        // The cloud gave up waiting: only a requestId.
        return json_response(202, &json!({"requestId": uuid::Uuid::new_v4().to_string()}));
    }
    json_response(200, &payment_answer(&payment, phase, utid))
}

pub async fn status_sync(
    State(shared): State<Shared>,
    Path((umid, utid)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let path = format!("/api/v2/merchants/{umid}/terminals/{utid}/status/sync");
    let parsed = parse(&body);
    let log = lock(&shared).log(
        Protocol::Worldline,
        "POST",
        &path,
        parsed.clone().unwrap_or(Value::Null),
    );
    let response = status_inner(&shared, &utid, &headers, parsed);
    lock(&shared).set_status(log, response.status().as_u16());
    response
}

fn status_inner(
    shared: &Shared,
    utid: &str,
    headers: &HeaderMap,
    parsed: Option<Value>,
) -> Response {
    if !bearer_ok(shared, headers) {
        return error(401, "Unauthorized");
    }
    let Some(request) = parsed else {
        return error(400, "Invalid JSON");
    };
    let Some(root) = request.get("SaleToPOIMessageStatusRequest") else {
        return error(400, "SaleToPOIMessageStatusRequest is required");
    };
    if let Err(m) = check_header(&root["Header"]) {
        return error(400, &m);
    }
    let Some(target) = text(
        root,
        &[
            "StatusRequest",
            "MessageStatusRequestData",
            "ExchangeIdentification",
        ],
    ) else {
        return error(
            400,
            "MessageStatusRequestData.ExchangeIdentification is required",
        );
    };
    let inner = lock(shared);
    let found = inner.find(Protocol::Worldline, target).map(|i| {
        let p = inner.payments[i].clone();
        let phase = p.phase(std::time::Instant::now());
        (p, phase)
    });
    drop(inner);
    let lookup = match &found {
        None => failure(
            "NotFound",
            "No transaction with this ExchangeIdentification",
        ),
        Some((_, Phase::InProgress | Phase::Stuck)) => {
            failure("InProgress", "Transaction is still in progress")
        }
        Some((_, Phase::Declined)) => failure("Refusal", "Declined by the issuer"),
        Some((_, Phase::Cancelled)) => failure("Aborted", "WPI_ERR_COND_ABORTED:Payment aborted!"),
        Some((_, Phase::Approved)) => json!({"Response": "Successful"}),
    };
    let mut data = json!({
        "ExchangeIdentification": target,
        "InitiatingParty": {"Identification": "WL000D5T0000004", "Type": "Merchant"},
        "TransactionResponse": lookup,
    });
    let mut environment_block = json!({"POI": {"Identification": {"Identification": utid}}});
    if let Some((p, Phase::Approved)) = &found {
        data["RepeatedPaymentResponse"] = payment_block(p);
        environment_block = environment(p, utid);
    }
    json_response(
        200,
        &json!({"data": {"SaleToPOIMessageStatusResponse": {
            "Header": header_echo("MessageStatusResponse", target),
            "StatusResponse": {
                "Environment": environment_block,
                "MessageStatusResponseData": data,
            },
        }}, "requestId": uuid::Uuid::new_v4().to_string()}),
    )
}

pub async fn abort(
    State(shared): State<Shared>,
    Path((umid, utid)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let path = format!("/api/v2/merchants/{umid}/terminals/{utid}/payments/abort");
    let parsed = parse(&body);
    let log = lock(&shared).log(
        Protocol::Worldline,
        "POST",
        &path,
        parsed.clone().unwrap_or(Value::Null),
    );
    let response = abort_inner(&shared, &headers, parsed);
    lock(&shared).set_status(log, response.status().as_u16());
    response
}

fn abort_inner(shared: &Shared, headers: &HeaderMap, parsed: Option<Value>) -> Response {
    if !bearer_ok(shared, headers) {
        return error(401, "Unauthorized");
    }
    let Some(request) = parsed else {
        return error(400, "Invalid JSON");
    };
    let Some(root) = request.get("SaleToPOIAbort") else {
        return error(400, "SaleToPOIAbort is required");
    };
    if let Err(m) = check_header(&root["Header"]) {
        return error(400, &m);
    }
    let Some(target) = text(root, &["Abort", "SystemAbort", "ExchangeIdentification"]) else {
        return error(400, "Abort.SystemAbort.ExchangeIdentification is required");
    };
    let mut inner = lock(shared);
    if let Some(i) = inner.find(Protocol::Worldline, target) {
        inner.abort(i);
    }
    // An abort with nothing in progress is ignored and still acknowledged.
    json_response(200, &json!({"requestId": uuid::Uuid::new_v4().to_string()}))
}
