//! CCV Payment API, payment method `terminal` (HTTPS through the CCV
//! cloud, which wakes the terminal).
//!
//! A payment is `POST /api/v1/payment` with `method: terminal` and an
//! `Idempotency-Reference` header carrying the till reference (CCV keeps
//! it for 24 hours: the same call twice is one transaction). The call
//! returns the transaction in status `pending`; the driver then polls
//! `GET /api/v1/transaction?reference=<psp reference>` every 2 seconds
//! until `success` or `failed`. `manualintervention` and `pending` keep
//! the wait going; at the configured timeout the outcome is `Unknown`
//! and `status` asks again with `merchantOrderReference` (the till
//! reference, which the create call sends). `abort` is
//! `POST /api/v1/cancel?reference=<psp reference>`.
//!
//! Verified against CCV's public documentation and OpenAPI
//! (developer.myccv.eu, api.psp.ccv.eu/api/v1/openapi): paths, Basic
//! authentication (API key as user, empty password), the request and
//! response examples of the terminal method, statuses, failure codes,
//! idempotency, the cancel call, the error classes. Inferred: the
//! layout of `details.cardCircuit` ("an escaped JSON string", no
//! example) and of `details.customerReceipt` (a JSON array, no
//! example), so both are read defensively and the card brand and the
//! masked number fall back to the receipt text; `TERMINAL_ALREADY_IN_USE`
//! is recognised anywhere in an error body (the simulator documents it
//! as an HTTP 500). CCV's rule that the API key must not sit on a shop
//! device is for the operator: set `base_url` to a relay that holds it.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;

use pos_core::config::PaymentTerminalConfig;
use serde_json::{Value, json};
use tokio::time::Instant;
use tracing::{info, warn};

use crate::http::{Reply, SendError, error_message, send};
use crate::util::{AMOUNT_MARK, at, cents_from_json, language_iso3, str_at, truncate, with_amount};
use crate::{Approval, EftError, Outcome, PaymentRequest, brand, receipt};

/// CCV's production and test API (the key's mode decides).
const PRODUCTION_URL: &str = "https://api.psp.ccv.eu";
const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// Total timeout of [`CcvPaymentApi::check`].
const CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// A CCV terminal reached through the CCV cloud.
pub struct CcvPaymentApi {
    base: String,
    api_key: Option<String>,
    terminal_id: String,
    management_system_id: String,
    access_protocol: String,
    http: reqwest::Client,
    timeout: Duration,
    poll: Duration,
    /// Till reference to PSP reference, for the payments started here.
    in_flight: Mutex<HashMap<String, String>>,
    /// Till references the till asked to stop.
    aborted: Mutex<HashSet<String>>,
}

/// What one reading of a transaction says.
enum Reading {
    Final(Outcome),
    Waiting(String),
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl CcvPaymentApi {
    pub fn new(config: &PaymentTerminalConfig, http: reqwest::Client) -> Result<Self, EftError> {
        let required = |value: &Option<String>, key: &str| -> Result<String, EftError> {
            value
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .ok_or_else(|| EftError::Config(format!("payment_terminal.{key}: required")))
        };
        Ok(Self {
            base: config
                .base_url
                .as_deref()
                .unwrap_or(PRODUCTION_URL)
                .trim_end_matches('/')
                .to_string(),
            api_key: config.api_key().map_err(EftError::Config)?,
            terminal_id: required(&config.terminal_id, "terminal_id")?,
            management_system_id: required(&config.management_system_id, "management_system_id")?,
            access_protocol: config
                .access_protocol
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("OPI_NL")
                .to_string(),
            http,
            timeout: crate::timeout(config),
            poll: POLL_INTERVAL,
            in_flight: Mutex::new(HashMap::new()),
            aborted: Mutex::new(HashSet::new()),
        })
    }

    #[cfg(test)]
    pub(crate) fn set_timing(&mut self, timeout: Duration, poll: Duration) {
        self.timeout = timeout;
        self.poll = poll;
    }

    fn request(&self, method: reqwest::Method, path_and_query: &str) -> reqwest::RequestBuilder {
        let mut request = self
            .http
            .request(method, format!("{}{path_and_query}", self.base))
            .timeout(Duration::from_secs(30))
            .header("Accept", "application/json");
        if let Some(key) = &self.api_key {
            request = request.basic_auth(key, Some(""));
        }
        request
    }

    /// The create call's JSON, amount marked for [`with_amount`].
    pub(crate) fn payment_body(&self, request: &PaymentRequest) -> Value {
        let mut body = json!({
            "amount": AMOUNT_MARK,
            "currency": "eur",
            "method": "terminal",
            "merchantOrderReference": request.reference,
            "language": language_iso3(&request.language),
            "details": {
                "terminalId": self.terminal_id,
                "managementSystemId": self.management_system_id,
                "accessProtocol": self.access_protocol,
                "operatingEnvironment": "ATTENDED",
                "merchantLanguage": language_iso3(&request.language).to_ascii_uppercase(),
            },
        });
        let description = truncate(request.description.trim(), 255);
        if !description.is_empty() {
            body["description"] = Value::String(description);
        }
        body
    }

    pub async fn pay(&self, request: &PaymentRequest) -> Result<Outcome, EftError> {
        let body = with_amount(&self.payment_body(request), request.amount_cents);
        info!(
            reference = %request.reference,
            amount_cents = request.amount_cents,
            "ccv payment sent"
        );
        let create = self
            .request(reqwest::Method::POST, "/api/v1/payment")
            .header("Content-Type", "application/json")
            .header("Idempotency-Reference", &request.reference)
            .body(body);
        let reply = match send(create).await {
            Ok(r) => r,
            Err(SendError::NotSent(e)) => return Err(EftError::Unreachable(e)),
            Err(SendError::Lost(e)) => {
                warn!(reference = %request.reference, "ccv create answer lost: {e}");
                return Ok(Outcome::Unknown {
                    detail: format!("no answer from the CCV cloud ({e})"),
                });
            }
        };
        let tx = match self.read_create_reply(&reply, &request.reference)? {
            Ok(tx) => tx,
            Err(outcome) => return Ok(outcome),
        };
        let Some(psp) = str_at(&tx, &["reference"]).map(str::to_string) else {
            return Ok(Outcome::Unknown {
                detail: "CCV accepted the payment without a reference".into(),
            });
        };
        lock(&self.in_flight).insert(request.reference.clone(), psp.clone());
        // A stop asked while the create call was in flight.
        if lock(&self.aborted).contains(&request.reference) {
            let _ = self.cancel(&psp).await;
        }
        self.wait_for_final(&request.reference, &psp, Some(request.amount_cents), tx)
            .await
    }

    /// The transaction of a create answer, or the outcome / error when
    /// it is not one.
    fn read_create_reply(
        &self,
        reply: &Reply,
        reference: &str,
    ) -> Result<Result<Value, Outcome>, EftError> {
        if reply
            .body
            .to_ascii_uppercase()
            .contains("TERMINAL_ALREADY_IN_USE")
        {
            return Err(EftError::Busy);
        }
        match reply.status {
            200..=299 => match serde_json::from_str::<Value>(&reply.body) {
                Ok(v) if v.is_object() => Ok(Ok(v)),
                _ => Ok(Err(Outcome::Unknown {
                    detail: "unreadable CCV answer to the payment request".into(),
                })),
            },
            401 | 403 => Err(EftError::Rejected(format!(
                "CCV refused the API key (HTTP {})",
                reply.status
            ))),
            429 => Err(EftError::Unreachable(
                "CCV rate limit reached (HTTP 429)".into(),
            )),
            400..=499 => Err(EftError::Rejected(format!(
                "HTTP {}: {}",
                reply.status,
                error_message(&reply.body)
            ))),
            code => {
                // A gateway timeout may carry the transaction to ask for.
                let known = serde_json::from_str::<Value>(&reply.body)
                    .ok()
                    .and_then(|v| str_at(&v, &["transactionReference"]).map(str::to_string));
                if let Some(psp) = &known {
                    lock(&self.in_flight).insert(reference.to_string(), psp.clone());
                }
                Ok(Err(Outcome::Unknown {
                    detail: match known {
                        Some(psp) => format!("CCV answered HTTP {code}, transaction {psp}"),
                        None => format!("CCV answered HTTP {code}"),
                    },
                }))
            }
        }
    }

    async fn wait_for_final(
        &self,
        reference: &str,
        psp: &str,
        expected: Option<i64>,
        first: Value,
    ) -> Result<Outcome, EftError> {
        let deadline = Instant::now() + self.timeout;
        let mut last = self.read(&first, reference, expected);
        loop {
            match last {
                Reading::Final(outcome) => return Ok(outcome),
                Reading::Waiting(ref why) => {
                    if Instant::now() >= deadline {
                        return Ok(Outcome::Unknown {
                            detail: format!(
                                "no final answer within {} s ({why})",
                                self.timeout.as_secs()
                            ),
                        });
                    }
                }
            }
            tokio::time::sleep(
                self.poll
                    .min(deadline.saturating_duration_since(Instant::now())),
            )
            .await;
            let query = format!(
                "/api/v1/transaction?reference={}",
                crate::util::path_segment(psp)
            );
            match send(self.request(reqwest::Method::GET, &query)).await {
                Ok(r) if r.status == 200 => {
                    if let Some(tx) = pick_transaction(&r.body, None) {
                        last = self.read(&tx, reference, expected);
                    }
                }
                Ok(r) => warn!(reference, "ccv poll answered HTTP {}", r.status),
                Err(SendError::NotSent(e) | SendError::Lost(e)) => {
                    warn!(reference, "ccv poll failed: {e}");
                }
            }
        }
    }

    /// One reading of a transaction object.
    fn read(&self, tx: &Value, reference: &str, expected: Option<i64>) -> Reading {
        let status = str_at(tx, &["status"]).unwrap_or("").to_ascii_lowercase();
        match status.as_str() {
            "success" => Reading::Final(approval(tx, expected)),
            "failed" => {
                let code = str_at(tx, &["failureCode"]).unwrap_or("unknown");
                let by_till = str_at(tx, &["cancelledBy"]) == Some("merchant")
                    || lock(&self.aborted).contains(reference);
                if code == "cancelled" && by_till {
                    Reading::Final(Outcome::Cancelled)
                } else if code == "cancelled" && str_at(tx, &["cancelledBy"]) == Some("customer") {
                    Reading::Final(Outcome::Declined {
                        reason: "cancelled by the cardholder".into(),
                    })
                } else {
                    Reading::Final(Outcome::Declined {
                        reason: code.to_string(),
                    })
                }
            }
            "manualintervention" => Reading::Waiting("CCV asks for manual intervention".into()),
            "pending" | "" => Reading::Waiting("pending".into()),
            other => Reading::Waiting(format!("status {other}")),
        }
    }

    pub async fn status(&self, reference: &str) -> Result<Outcome, EftError> {
        let query = format!(
            "/api/v1/transaction?merchantOrderReference={}",
            crate::util::path_segment(reference)
        );
        let reply = match send(self.request(reqwest::Method::GET, &query)).await {
            Ok(r) => r,
            Err(SendError::NotSent(e)) => return Err(EftError::Unreachable(e)),
            Err(SendError::Lost(e)) => {
                return Ok(Outcome::Unknown {
                    detail: format!("status request lost ({e})"),
                });
            }
        };
        match reply.status {
            200 => {}
            401 | 403 => {
                return Err(EftError::Rejected(format!(
                    "CCV refused the API key (HTTP {})",
                    reply.status
                )));
            }
            400..=499 => {
                return Err(EftError::Rejected(format!(
                    "HTTP {}: {}",
                    reply.status,
                    error_message(&reply.body)
                )));
            }
            code => {
                return Ok(Outcome::Unknown {
                    detail: format!("CCV answered HTTP {code}"),
                });
            }
        }
        let Some(tx) = pick_transaction(&reply.body, Some(reference)) else {
            return Ok(Outcome::Unknown {
                detail: "CCV has no payment with this reference".into(),
            });
        };
        Ok(match self.read(&tx, reference, None) {
            Reading::Final(outcome) => outcome,
            Reading::Waiting(why) => Outcome::Unknown { detail: why },
        })
    }

    /// Reachability check of the CCV service, not of the terminal: CCV
    /// has no terminal-presence endpoint. A read of a transaction with a
    /// random reference proves the service answers and the key is
    /// accepted (200 or 404 is `Ok`, 401 and 403 are rejected). Nothing
    /// is created, so nothing can be charged.
    pub async fn check(&self) -> Result<(), EftError> {
        let query = format!(
            "/api/v1/transaction?reference={}",
            uuid::Uuid::new_v4().simple()
        );
        let request = self
            .request(reqwest::Method::GET, &query)
            .timeout(CHECK_TIMEOUT);
        match send(request).await {
            Ok(r) => match r.status {
                200 | 404 => Ok(()),
                401 | 403 => Err(EftError::Rejected(format!(
                    "CCV refused the API key (HTTP {})",
                    r.status
                ))),
                400..=499 => Err(EftError::Rejected(format!(
                    "HTTP {}: {}",
                    r.status,
                    error_message(&r.body)
                ))),
                code => Err(EftError::Unreachable(format!("CCV answered HTTP {code}"))),
            },
            Err(SendError::NotSent(e) | SendError::Lost(e)) => Err(EftError::Unreachable(e)),
        }
    }

    pub async fn abort(&self, reference: &str) -> Result<(), EftError> {
        lock(&self.aborted).insert(reference.to_string());
        let known = lock(&self.in_flight).get(reference).cloned();
        let psp = match known {
            Some(p) => p,
            None => {
                // Started by another instance, or before a restart.
                let query = format!(
                    "/api/v1/transaction?merchantOrderReference={}",
                    crate::util::path_segment(reference)
                );
                let found = match send(self.request(reqwest::Method::GET, &query)).await {
                    Ok(r) if r.status == 200 => pick_transaction(&r.body, Some(reference))
                        .and_then(|tx| str_at(&tx, &["reference"]).map(str::to_string)),
                    Ok(r) if matches!(r.status, 401 | 403) => {
                        return Err(EftError::Rejected(format!(
                            "CCV refused the API key (HTTP {})",
                            r.status
                        )));
                    }
                    Ok(_) => None,
                    Err(SendError::NotSent(e) | SendError::Lost(e)) => {
                        return Err(EftError::Unreachable(e));
                    }
                };
                match found {
                    Some(p) => p,
                    // Nothing to stop yet; a create still in flight sees
                    // the flag and cancels itself.
                    None => return Ok(()),
                }
            }
        };
        self.cancel(&psp).await
    }

    async fn cancel(&self, psp: &str) -> Result<(), EftError> {
        let query = format!(
            "/api/v1/cancel?reference={}",
            crate::util::path_segment(psp)
        );
        match send(self.request(reqwest::Method::POST, &query)).await {
            Ok(r) if (200..300).contains(&r.status) => Ok(()),
            // CCV refuses a cancel that comes too late and says the
            // payment goes on: the request was delivered.
            Ok(r) if r.status == 400 => Ok(()),
            Ok(r) if matches!(r.status, 401 | 403) => Err(EftError::Rejected(format!(
                "CCV refused the API key (HTTP {})",
                r.status
            ))),
            Ok(r) => Err(EftError::Unreachable(format!(
                "cancel not delivered (HTTP {})",
                r.status
            ))),
            Err(SendError::NotSent(e) | SendError::Lost(e)) => Err(EftError::Unreachable(e)),
        }
    }
}

/// The transaction of a `GET /transaction` answer: an object, or the
/// array a list query returns. With `merchant_reference`, the sale made
/// for it: a success before any other, else the latest.
fn pick_transaction(body: &str, merchant_reference: Option<&str>) -> Option<Value> {
    let v: Value = serde_json::from_str(body).ok()?;
    let list = match v {
        Value::Array(a) => a,
        Value::Object(_) => vec![v],
        _ => return None,
    };
    let mut sales: Vec<&Value> = list
        .iter()
        .filter(|t| {
            let type_ok = str_at(t, &["type"]).is_none_or(|t| t == "sale");
            let ref_ok = merchant_reference
                .is_none_or(|r| str_at(t, &["merchantOrderReference"]).is_none_or(|m| m == r));
            type_ok && ref_ok
        })
        .collect();
    sales.sort_by_key(|t| t.get("created").and_then(Value::as_i64).unwrap_or(0));
    sales
        .iter()
        .find(|t| str_at(t, &["status"]) == Some("success"))
        .or_else(|| sales.last())
        .map(|t| (*t).clone())
}

/// The card circuit string: an escaped JSON string, whatever the
/// terminal put in it.
fn circuit_of(details: &Value) -> Option<String> {
    let raw = details.get("cardCircuit")?;
    let value = match raw {
        Value::String(s) => serde_json::from_str::<Value>(s).unwrap_or_else(|_| raw.clone()),
        other => other.clone(),
    };
    match value {
        Value::String(s) => Some(s),
        Value::Array(items) => items.iter().find_map(|i| i.as_str().map(str::to_string)),
        Value::Object(map) => ["cardCircuit", "circuit", "name", "brand", "value"]
            .iter()
            .find_map(|k| map.get(*k).and_then(Value::as_str).map(str::to_string)),
        _ => None,
    }
}

/// The customer receipt as plain text: an escaped JSON array of lines,
/// or the text itself.
fn receipt_text(details: &Value, key: &str) -> Option<String> {
    let raw = details.get(key)?.as_str()?;
    let text = match serde_json::from_str::<Value>(raw) {
        Ok(Value::Array(items)) => items
            .iter()
            .filter_map(|i| match i {
                Value::String(s) => Some(s.clone()),
                Value::Object(o) => ["text", "line", "value", "content"]
                    .iter()
                    .find_map(|k| o.get(*k).and_then(Value::as_str).map(str::to_string)),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Ok(Value::String(s)) => s,
        _ => raw.to_string(),
    };
    let text = text.trim_end().to_string();
    (!text.trim().is_empty()).then_some(text)
}

/// An approval from a `success` transaction, or `Unknown` when the
/// amount is not the one asked.
fn approval(tx: &Value, expected: Option<i64>) -> Outcome {
    let details = tx.get("details").unwrap_or(&Value::Null);
    let amount = at(details, &["approvedAmount"])
        .or_else(|| tx.get("amount"))
        .and_then(cents_from_json);
    let amount_cents = match (amount, expected) {
        (Some(a), Some(e)) if a != e => {
            return Outcome::Unknown {
                detail: format!("approved amount {a} cents differs from the {e} cents requested"),
            };
        }
        (Some(a), _) => a,
        (None, Some(e)) => e,
        (None, None) => {
            return Outcome::Unknown {
                detail: "success without an amount".into(),
            };
        }
    };
    if let Some(currency) = str_at(tx, &["currency"])
        && !currency.eq_ignore_ascii_case("eur")
    {
        return Outcome::Unknown {
            detail: format!("approved in {currency}"),
        };
    }
    let customer_receipt = receipt_text(details, "customerReceipt");
    let any_receipt = customer_receipt
        .clone()
        .or_else(|| receipt_text(details, "merchantReceipt"));
    let circuit = circuit_of(details);
    let normalized = circuit
        .as_deref()
        .and_then(brand::normalize)
        .or_else(|| any_receipt.as_deref().and_then(receipt::brand_in));
    let card_type = circuit
        .as_deref()
        .and_then(receipt::explicit_card_type)
        .or_else(|| any_receipt.as_deref().and_then(receipt::account_type))
        .unwrap_or_else(|| brand::card_type_of(normalized.as_deref()));
    Outcome::Approved(Approval {
        amount_cents,
        card_type,
        brand: normalized,
        terminal_reference: str_at(tx, &["reference"]).map(str::to_string),
        authorisation_code: any_receipt.as_deref().and_then(|r| {
            receipt::field(
                r,
                &[
                    "Auth. Code",
                    "Auth Code",
                    "Approval Code",
                    "Autorisation",
                    "Code autorisation",
                ],
            )
        }),
        masked_pan: any_receipt.as_deref().and_then(receipt::masked_pan),
        customer_receipt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardType;

    // Source of the fixtures: CCV's public documentation of the terminal
    // method, https://developer.myccv.eu/documentation/payment_api/online_payments/payment_methods/terminal/
    // (request and response examples, verbatim).
    const CREATE_REQUEST: &str = include_str!("../tests/fixtures/ccv_terminal_create_request.json");
    const CREATE_RESPONSE: &str =
        include_str!("../tests/fixtures/ccv_terminal_create_response.json");
    // Built from the OpenAPI schema (https://api.psp.ccv.eu/api/v1/openapi):
    // field names and enums are CCV's, the values are made up because the
    // public documentation shows no finished terminal transaction.
    const SUCCESS: &str = include_str!("../tests/fixtures/ccv_terminal_success.json");
    const FAILED: &str = include_str!("../tests/fixtures/ccv_terminal_failed_cancelled.json");

    fn terminal() -> CcvPaymentApi {
        let config = pos_core::config::PaymentTerminalConfig {
            kind: pos_core::config::PaymentTerminalKind::CcvPaymentApi,
            base_url: Some("http://127.0.0.1:1".into()),
            api_key_file: None,
            merchant_id: None,
            terminal_id: Some("1234536".into()),
            integrator_id: None,
            management_system_id: Some("000103030".into()),
            access_protocol: None,
            address: None,
            device_port: None,
            timeout_seconds: 60,
        };
        CcvPaymentApi::new(&config, reqwest::Client::new()).unwrap()
    }

    #[test]
    fn the_create_call_has_the_shape_of_the_documented_example() {
        let api = terminal();
        let request = PaymentRequest {
            reference: "6f1f0a52-0000-4000-8000-000000000001".into(),
            amount_cents: 999,
            description: "Tokyo Sushi Bar, table 4".into(),
            language: "nl".into(),
        };
        let body: Value =
            serde_json::from_str(&with_amount(&api.payment_body(&request), 999)).unwrap();
        let documented: Value = serde_json::from_str(CREATE_REQUEST).unwrap();
        assert_eq!(body["amount"], documented["amount"]);
        for key in ["currency", "method"] {
            assert_eq!(body[key], documented[key], "{key}");
        }
        assert_eq!(body["language"], "nld");
        for key in [
            "managementSystemId",
            "accessProtocol",
            "operatingEnvironment",
            "merchantLanguage",
        ] {
            assert_eq!(body["details"][key], documented["details"][key], "{key}");
        }
        assert_eq!(body["merchantOrderReference"], request.reference);
        assert_eq!(body["details"]["terminalId"], "1234536");
    }

    #[test]
    fn the_documented_create_response_is_a_waiting_transaction() {
        let api = terminal();
        let tx: Value = serde_json::from_str(CREATE_RESPONSE).unwrap();
        assert_eq!(tx["reference"], "TL260414072854599CB87E19B.3");
        assert!(matches!(
            api.read(&tx, "123456", Some(999)),
            Reading::Waiting(_)
        ));
    }

    #[test]
    fn a_success_gives_brand_type_pan_code_and_receipt() {
        let tx: Value = serde_json::from_str(SUCCESS).unwrap();
        let api = terminal();
        let Reading::Final(Outcome::Approved(a)) = api.read(&tx, "R1", Some(1250)) else {
            panic!("not an approval")
        };
        assert_eq!(a.amount_cents, 1250);
        assert_eq!(a.brand.as_deref(), Some("Bancontact"));
        assert_eq!(a.card_type, CardType::Debit);
        assert_eq!(a.masked_pan.as_deref(), Some("XXXXXXXXXXXX0119"));
        assert_eq!(a.authorisation_code.as_deref(), Some("057737"));
        assert_eq!(
            a.terminal_reference.as_deref(),
            Some("TL260414072854599CB87E19B.3")
        );
        assert!(a.customer_receipt.unwrap().contains("Total-EFT EUR"));
        // Another amount than the one asked is never an approval.
        assert!(matches!(
            api.read(&tx, "R1", Some(1000)),
            Reading::Final(Outcome::Unknown { .. })
        ));
    }

    #[test]
    fn a_cancelled_payment_is_cancelled_only_when_the_till_stopped_it() {
        let api = terminal();
        let tx: Value = serde_json::from_str(FAILED).unwrap();
        // cancelledBy merchant in the fixture.
        assert!(matches!(
            api.read(&tx, "R1", None),
            Reading::Final(Outcome::Cancelled)
        ));
        let mut by_customer = tx.clone();
        by_customer["cancelledBy"] = json!("customer");
        let Reading::Final(Outcome::Declined { reason }) = api.read(&by_customer, "R1", None)
        else {
            panic!("expected a decline")
        };
        assert!(reason.contains("cardholder"));
        let mut no_actor = tx.clone();
        no_actor.as_object_mut().unwrap().remove("cancelledBy");
        assert!(matches!(
            api.read(&no_actor, "R1", None),
            Reading::Final(Outcome::Declined { .. })
        ));
        lock(&api.aborted).insert("R1".into());
        assert!(matches!(
            api.read(&no_actor, "R1", None),
            Reading::Final(Outcome::Cancelled)
        ));
        let mut refused = tx;
        refused["failureCode"] = json!("card_refused");
        assert!(matches!(
            api.read(&refused, "R2", None),
            Reading::Final(Outcome::Declined { reason }) if reason == "card_refused"
        ));
    }

    #[test]
    fn a_list_answer_gives_the_sale_for_the_reference() {
        let refund =
            json!({"type":"refund","status":"success","merchantOrderReference":"R1","created":3});
        let failed =
            json!({"type":"sale","status":"failed","merchantOrderReference":"R1","created":1});
        let ok = json!({"type":"sale","status":"success","merchantOrderReference":"R1","created":2,"reference":"TL1"});
        let list = Value::Array(vec![refund, failed, ok]).to_string();
        let tx = pick_transaction(&list, Some("R1")).unwrap();
        assert_eq!(tx["reference"], "TL1");
        assert!(pick_transaction("[]", Some("R1")).is_none());
        assert!(pick_transaction("nope", None).is_none());
    }

    #[test]
    fn a_meal_voucher_circuit_or_receipt_gives_the_issuer() {
        let api = terminal();
        // The circuit says it.
        for (circuit, brand) in [
            ("EDENRED", "Edenred"),
            ("Ticket Restaurant", "Edenred"),
            ("SODEXO", "Pluxee"),
        ] {
            let text = SUCCESS.replace("\\\"BCMC\\\"", &format!("\\\"{circuit}\\\""));
            assert_ne!(text, SUCCESS);
            let tx: Value = serde_json::from_str(&text).unwrap();
            let Reading::Final(Outcome::Approved(a)) = api.read(&tx, "R1", Some(1250)) else {
                panic!("not an approval")
            };
            assert_eq!(a.brand.as_deref(), Some(brand), "{circuit}");
        }
        // No circuit: the receipt names it.
        let mut tx: Value = serde_json::from_str(SUCCESS).unwrap();
        tx["details"].as_object_mut().unwrap().remove("cardCircuit");
        let receipt = tx["details"]["customerReceipt"]
            .as_str()
            .unwrap()
            .replace("BANCONTACT", "EDENRED");
        tx["details"]["customerReceipt"] = Value::String(receipt);
        let Reading::Final(Outcome::Approved(a)) = api.read(&tx, "R1", Some(1250)) else {
            panic!("not an approval")
        };
        assert_eq!(a.brand.as_deref(), Some("Edenred"));
    }

    #[test]
    fn card_circuit_and_receipt_layouts_are_read_defensively() {
        let d = json!({"cardCircuit": "\"MAESTRO\""});
        assert_eq!(circuit_of(&d).as_deref(), Some("MAESTRO"));
        let d = json!({"cardCircuit": "{\"cardCircuit\":\"VISA\"}"});
        assert_eq!(circuit_of(&d).as_deref(), Some("VISA"));
        let d = json!({"cardCircuit": "BCMC"});
        assert_eq!(circuit_of(&d).as_deref(), Some("BCMC"));
        let d = json!({"customerReceipt": "[\"a\",{\"text\":\"b\"}]"});
        assert_eq!(receipt_text(&d, "customerReceipt").as_deref(), Some("a\nb"));
        let d = json!({"customerReceipt": "plain text"});
        assert_eq!(
            receipt_text(&d, "customerReceipt").as_deref(),
            Some("plain text")
        );
        assert_eq!(receipt_text(&d, "merchantReceipt"), None);
    }
}
