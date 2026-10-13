//! Worldline Terminal API v2 (nexo Retailer 5.1 as JSON over HTTPS).
//!
//! The till talks to the Worldline cloud, which holds a WebSocket to the
//! terminal (an Android SmartPOS terminal running the "Terminal API"
//! app). A payment is one synchronous call:
//!
//! ```text
//! POST {base}/api/v2/merchants/{umid}/terminals/{utid}/payments/sync
//! ```
//!
//! with a `SaleToPOIServiceRequest` / `FinancialPaymentRequest`. The
//! till's payment reference (a UUID) is the `Header.ExchangeIdentification`,
//! which Worldline keeps as an idempotency key for 7 days: sending the
//! same payment twice returns the first answer, never a second charge.
//! The cloud holds the call open until the terminal answers or 4 minutes
//! pass; past that it answers HTTP 202 with only a `requestId`, and the
//! outcome is read later with `/status/sync` and the same
//! `ExchangeIdentification`. `/payments/abort` stops a payment in
//! progress.
//!
//! Verified against the public OpenAPI v2 (`5.1-WL2.1.1`) and its
//! examples (docs.terminal.worldline-solutions.com): paths, envelope
//! (`{data, requestId}`), header and `SalesSystemInfo` rules, the
//! payment, abort and status messages, the response examples. Inferred:
//! `Environment.Cardholder.Language` carries the screen language (the
//! schema allows it, no example shows it), the exact shape of a status
//! answer for a payment that was declined or aborted (only the approved
//! case has an example), and that Bancontact arrives in
//! `Card.CardBrand` as free text. The spec itself is inconsistent about
//! `Response.Result` / `Response.Response` and `MaskedPAN` / `MaskedPin`,
//! so both spellings are read.

use std::time::Duration;

use pos_core::config::PaymentTerminalConfig;
use serde_json::{Value, json};
use tracing::{info, warn};

use crate::http::{Reply, SendError, error_message, send};
use crate::util::{
    AMOUNT_MARK, at, cents_from_json, language_upper2, path_segment, str_at, with_amount,
};
use crate::{Approval, EftError, Outcome, PaymentRequest, brand, receipt};

/// Worldline's production API.
const PRODUCTION_URL: &str = "https://api.terminal.global.worldline-solutions.com";
const PROTOCOL_VERSION: &str = "5.1-WL2.1.1";
/// Total timeout of [`WorldlineTerminalApi::check`].
const CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// A Worldline terminal reached through the Terminal API.
pub struct WorldlineTerminalApi {
    base: String,
    merchant_id: String,
    terminal_id: String,
    integrator_id: String,
    api_key: Option<String>,
    http: reqwest::Client,
    timeout: Duration,
    software_name: String,
    software_version: String,
}

impl WorldlineTerminalApi {
    pub fn new(
        config: &PaymentTerminalConfig,
        http: reqwest::Client,
        software: &str,
    ) -> Result<Self, EftError> {
        let required = |value: &Option<String>, key: &str| -> Result<String, EftError> {
            value
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .ok_or_else(|| EftError::Config(format!("payment_terminal.{key}: required")))
        };
        let (name, version) = software
            .split_once('/')
            .map_or((software, "unknown"), |(n, v)| (n, v));
        Ok(Self {
            base: config
                .base_url
                .as_deref()
                .unwrap_or(PRODUCTION_URL)
                .trim_end_matches('/')
                .to_string(),
            merchant_id: required(&config.merchant_id, "merchant_id")?,
            terminal_id: required(&config.terminal_id, "terminal_id")?,
            integrator_id: required(&config.integrator_id, "integrator_id")?,
            api_key: config.api_key().map_err(EftError::Config)?,
            http,
            timeout: crate::timeout(config),
            software_name: name.trim().to_string(),
            software_version: version.trim().to_string(),
        })
    }

    #[cfg(test)]
    pub(crate) fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    fn url(&self, suffix: &str) -> String {
        format!(
            "{}/api/v2/merchants/{}/terminals/{}/{suffix}",
            self.base,
            path_segment(&self.merchant_id),
            path_segment(&self.terminal_id)
        )
    }

    fn post(&self, suffix: &str, body: String, timeout: Duration) -> reqwest::RequestBuilder {
        let mut request = self
            .http
            .post(self.url(suffix))
            .header("Content-Type", "application/json")
            .timeout(timeout)
            .body(body);
        if let Some(key) = &self.api_key {
            request = request.bearer_auth(key);
        }
        request
    }

    fn header(&self, function: &str, exchange_id: &str) -> Value {
        json!({
            "MessageFunction": function,
            "ProtocolVersion": PROTOCOL_VERSION,
            "ExchangeIdentification": exchange_id,
            "CreationDateTime": now_text(),
            "InitiatingParty": {"Identification": self.merchant_id, "Type": "Merchant"},
            "SalesSystemInfo": {
                "IntegratorId": self.integrator_id,
                "SaleSoftware": [{
                    "Type": "ECR",
                    "SubTypeInformation": "Checkout",
                    "Identification": {
                        "ProviderIdentification": "Nuage Magique",
                        "Identification": self.software_name,
                    },
                    "Status": {"VersionNumber": self.software_version},
                }],
            },
        })
    }

    fn poi(&self) -> Value {
        json!({"Identification": {"Identification": self.terminal_id}})
    }

    /// The `SaleToPOIServiceRequest` for `request`, amount marked for
    /// [`with_amount`].
    pub(crate) fn payment_body(&self, request: &PaymentRequest) -> Value {
        json!({"SaleToPOIServiceRequest": {
            "Header": self.header("SaleFinancialServiceRequest", &request.reference),
            "ServiceRequest": {
                "Environment": {
                    "POI": self.poi(),
                    "Cardholder": {"Language": language_upper2(&request.language)},
                },
                "ServiceContent": "FinancialPaymentRequest",
                "PaymentRequest": {"PaymentTransaction": {
                    "TransactionType": "CardPayment",
                    "TransactionIdentification": {
                        "TransactionDateTime": now_text(),
                        "TransactionReference": request.reference,
                    },
                    "TransactionDetails": {"Currency": "EUR", "TotalAmount": AMOUNT_MARK},
                }},
            },
        }})
    }

    pub async fn pay(&self, request: &PaymentRequest) -> Result<Outcome, EftError> {
        let body = with_amount(&self.payment_body(request), request.amount_cents);
        info!(
            reference = %request.reference,
            amount_cents = request.amount_cents,
            "worldline payment sent"
        );
        let reply = match send(self.post("payments/sync", body, self.timeout)).await {
            Ok(r) => r,
            Err(SendError::NotSent(e)) => return Err(EftError::Unreachable(e)),
            Err(SendError::Lost(e)) => {
                warn!(reference = %request.reference, "worldline answer lost: {e}");
                return Ok(Outcome::Unknown {
                    detail: format!("no answer from the Worldline cloud ({e})"),
                });
            }
        };
        self.read_payment_reply(&reply, &request.reference, Some(request.amount_cents))
    }

    fn read_payment_reply(
        &self,
        reply: &Reply,
        reference: &str,
        expected: Option<i64>,
    ) -> Result<Outcome, EftError> {
        match reply.status {
            200 => match serde_json::from_str::<Value>(&reply.body) {
                Ok(v) => interpret_payment(&v, reference, expected),
                Err(e) => Ok(Outcome::Unknown {
                    detail: format!("unreadable Worldline answer ({e})"),
                }),
            },
            202 => Ok(Outcome::Unknown {
                detail: "the terminal had not answered when the cloud call ended (HTTP 202)".into(),
            }),
            code => Self::refuse_or_unknown(code, &reply.body),
        }
    }

    /// The error classes of the cloud API, shared by all calls.
    fn refuse_or_unknown(code: u16, body: &str) -> Result<Outcome, EftError> {
        let message = error_message(body);
        match code {
            401 | 403 => Err(EftError::Rejected(format!(
                "Worldline refused the API key (HTTP {code})"
            ))),
            404 => Err(EftError::Unreachable(format!(
                "terminal unknown or not connected to the Worldline cloud ({message})"
            ))),
            429 => Err(EftError::Unreachable(
                "Worldline rate limit reached (HTTP 429)".into(),
            )),
            408 => Ok(Outcome::Unknown {
                detail: "the Worldline cloud timed out (HTTP 408)".into(),
            }),
            400..=499 => Err(EftError::Rejected(format!("HTTP {code}: {message}"))),
            _ => Ok(Outcome::Unknown {
                detail: format!("Worldline server error (HTTP {code})"),
            }),
        }
    }

    pub async fn status(&self, reference: &str) -> Result<Outcome, EftError> {
        let body = json!({"SaleToPOIMessageStatusRequest": {
            "Header": self.header("MessageStatusRequest", &uuid::Uuid::new_v4().to_string()),
            "StatusRequest": {
                "Environment": {"POI": self.poi()},
                "MessageStatusRequestData": {
                    "ExchangeIdentification": reference,
                    "InitiatingParty": {"Identification": self.merchant_id, "Type": "Merchant"},
                },
            },
        }});
        let request = self.post("status/sync", body.to_string(), self.timeout);
        let reply = match send(request).await {
            Ok(r) => r,
            Err(SendError::NotSent(e)) => return Err(EftError::Unreachable(e)),
            Err(SendError::Lost(e)) => {
                return Ok(Outcome::Unknown {
                    detail: format!("status request lost ({e})"),
                });
            }
        };
        match reply.status {
            200 => match serde_json::from_str::<Value>(&reply.body) {
                Ok(v) => interpret_status(&v, reference),
                Err(e) => Ok(Outcome::Unknown {
                    detail: format!("unreadable Worldline status answer ({e})"),
                }),
            },
            202 => Ok(Outcome::Unknown {
                detail: "the terminal had not answered the status request (HTTP 202)".into(),
            }),
            code => Self::refuse_or_unknown(code, &reply.body),
        }
    }

    /// Reachability check: `GET {base}/api/v2/merchants/{umid}` answers
    /// the merchant's `connectedTerminals`; the terminal must be listed.
    /// Nothing is charged. The v1 API documents this call ("verify that a
    /// terminal is connected before sending requests"); the v2 notes
    /// describe the same answer (`{data: {umid, connectedTerminals}}`)
    /// but its path is not in the v2 OpenAPI, so it is not verified live.
    pub async fn check(&self) -> Result<(), EftError> {
        let mut request = self
            .http
            .get(format!(
                "{}/api/v2/merchants/{}",
                self.base,
                path_segment(&self.merchant_id)
            ))
            .header("Accept", "application/json")
            .timeout(CHECK_TIMEOUT);
        if let Some(key) = &self.api_key {
            request = request.bearer_auth(key);
        }
        let reply = match send(request).await {
            Ok(r) => r,
            Err(SendError::NotSent(e) | SendError::Lost(e)) => {
                return Err(EftError::Unreachable(e));
            }
        };
        if !(200..300).contains(&reply.status) {
            return match Self::refuse_or_unknown(reply.status, &reply.body) {
                Err(e) => Err(e),
                Ok(_) => Err(EftError::Unreachable(format!(
                    "Worldline answered HTTP {}",
                    reply.status
                ))),
            };
        }
        let listed = serde_json::from_str::<Value>(&reply.body)
            .ok()
            .and_then(|v| {
                at(payload(&v), &["connectedTerminals"])
                    .and_then(Value::as_array)
                    .map(|list| list.iter().any(|t| t.as_str() == Some(&self.terminal_id)))
            });
        match listed {
            Some(true) => Ok(()),
            Some(false) => Err(EftError::Unreachable(
                "terminal not connected to Worldline".into(),
            )),
            None => Err(EftError::Unreachable(
                "unreadable Worldline answer to the terminal list".into(),
            )),
        }
    }

    pub async fn abort(&self, reference: &str) -> Result<(), EftError> {
        let body = json!({"SaleToPOIAbort": {
            "Header": self.header("Abort", &uuid::Uuid::new_v4().to_string()),
            "Abort": {
                "Environment": {"POI": self.poi()},
                "SystemAbort": {
                    "ExchangeIdentification": reference,
                    "AbortReason": "Stopped from the till",
                },
            },
        }});
        let request = self.post("payments/abort", body.to_string(), Duration::from_secs(20));
        match send(request).await {
            Ok(r) if (200..300).contains(&r.status) => Ok(()),
            Ok(r) => match Self::refuse_or_unknown(r.status, &r.body) {
                Err(e) => Err(e),
                Ok(_) => Err(EftError::Unreachable(format!(
                    "abort not delivered (HTTP {})",
                    r.status
                ))),
            },
            Err(SendError::NotSent(e) | SendError::Lost(e)) => Err(EftError::Unreachable(e)),
        }
    }
}

/// The current time in Brussels as nexo writes it (`2020-05-04T18:13:51.0+01:00`).
fn now_text() -> String {
    chrono::Utc::now()
        .with_timezone(&chrono_tz::Europe::Brussels)
        .format("%Y-%m-%dT%H:%M:%S%.3f%:z")
        .to_string()
}

/// How a `Response` block reads.
#[derive(PartialEq, Eq)]
enum Result_ {
    Success,
    Failure,
    Other,
}

fn result_of(response: &Value) -> Result_ {
    let text = str_at(response, &["Result"]).or_else(|| str_at(response, &["Response"]));
    match text.map(str::to_ascii_lowercase).as_deref() {
        Some("success" | "successful") => Result_::Success,
        Some("failed" | "failure") => Result_::Failure,
        _ => Result_::Other,
    }
}

/// The body of an answer: the `data` envelope of v2 when present.
fn payload(root: &Value) -> &Value {
    match root.get("data") {
        Some(d) if d.is_object() => d,
        _ => root,
    }
}

/// The outcome of a synchronous payment answer (`{data, requestId}` or
/// the bare nexo message). `expected` is the amount asked, `reference`
/// the till reference sent as `TransactionReference`.
pub(crate) fn interpret_payment(
    root: &Value,
    reference: &str,
    expected: Option<i64>,
) -> Result<Outcome, EftError> {
    let root = payload(root);
    if let Some(rejection) = root.get("SaleToPOIMessageRejection") {
        let reason = str_at(rejection, &["Reject", "RejectReason"]).unwrap_or("unknown");
        return if reason == "DuplicateMessage" {
            Ok(Outcome::Unknown {
                detail: "Worldline reports a duplicate message: the first call is still running"
                    .into(),
            })
        } else {
            Err(EftError::Rejected(format!("message rejected: {reason}")))
        };
    }
    let Some(service) = at(root, &["SaleToPOIServiceResponse", "ServiceResponse"]) else {
        return Ok(Outcome::Unknown {
            detail: "unrecognised Worldline answer".into(),
        });
    };
    answer(
        service.get("Response"),
        service.get("PaymentResponse"),
        service.get("Environment"),
        reference,
        expected,
    )
}

/// The outcome of a `MessageStatusResponse`.
pub(crate) fn interpret_status(root: &Value, reference: &str) -> Result<Outcome, EftError> {
    let root = payload(root);
    if let Some(rejection) = root.get("SaleToPOIMessageRejection") {
        let reason = str_at(rejection, &["Reject", "RejectReason"]).unwrap_or("unknown");
        return Err(EftError::Rejected(format!("message rejected: {reason}")));
    }
    let Some(status) = at(root, &["SaleToPOIMessageStatusResponse", "StatusResponse"]) else {
        return Ok(Outcome::Unknown {
            detail: "unrecognised Worldline status answer".into(),
        });
    };
    let Some(data) = status.get("MessageStatusResponseData") else {
        return Ok(Outcome::Unknown {
            detail: "status answer without data".into(),
        });
    };
    let lookup = data.get("TransactionResponse");
    // `Successful` here only says the lookup worked; the payment's own
    // result is in RepeatedPaymentResponse. A failed lookup carries the
    // payment's failure reason (or NotFound / InProgress).
    let lookup_failed = lookup.is_some_and(|r| result_of(r) == Result_::Failure);
    let Some(payment) = data.get("RepeatedPaymentResponse") else {
        if lookup_failed {
            return answer(lookup, None, status.get("Environment"), reference, None);
        }
        return Ok(Outcome::Unknown {
            detail: "status answer without a payment".into(),
        });
    };
    answer(
        if lookup_failed { lookup } else { None },
        Some(payment),
        status.get("Environment"),
        reference,
        None,
    )
}

fn failure_text(response: &Value) -> String {
    let reason = str_at(response, &["ResponseReason"]).unwrap_or("Failed");
    match str_at(response, &["AdditionalResponseInformation"]) {
        Some(info) => format!("{reason}: {info}"),
        None => reason.to_string(),
    }
}

/// The shared reading of a result block, a payment and the environment.
fn answer(
    response: Option<&Value>,
    payment: Option<&Value>,
    env: Option<&Value>,
    reference: &str,
    expected: Option<i64>,
) -> Result<Outcome, EftError> {
    let authorisation = payment.and_then(|p| {
        at(
            p,
            &[
                "RetailerPaymentResult",
                "TransactionResponse",
                "AuthorisationResult",
            ],
        )
        .or_else(|| {
            at(
                p,
                &[
                    "PaymentTransaction",
                    "TransactionResponse",
                    "AuthorisationResult",
                ],
            )
        })
        .or_else(|| at(p, &["TransactionResponse", "AuthorisationResult"]))
    });
    let verdict = authorisation.and_then(|a| str_at(a, &["ResponseToAuthorisation", "Response"]));

    if let Some(r) = response
        && result_of(r) == Result_::Failure
    {
        if verdict == Some("Approved") {
            return Ok(Outcome::Unknown {
                detail: "the answer says both failed and approved".into(),
            });
        }
        return match str_at(r, &["ResponseReason"]) {
            Some("Aborted") => Ok(Outcome::Cancelled),
            Some("Busy") => Err(EftError::Busy),
            Some("InProgress") => Ok(Outcome::Unknown {
                detail: "the payment is still in progress on the terminal".into(),
            }),
            Some("NotFound") => Ok(Outcome::Unknown {
                detail: "the terminal does not know this payment".into(),
            }),
            _ => Ok(Outcome::Declined {
                reason: failure_text(r),
            }),
        };
    }

    match verdict {
        Some("Approved") => {}
        Some("Declined") => {
            return Ok(Outcome::Declined {
                reason: response
                    .and_then(|r| str_at(r, &["AdditionalResponseInformation"]))
                    .unwrap_or("Declined")
                    .to_string(),
            });
        }
        Some(other) => {
            return Ok(Outcome::Unknown {
                detail: format!("authorisation result {other}"),
            });
        }
        None => {
            return Ok(Outcome::Unknown {
                detail: "answer without an authorisation result".into(),
            });
        }
    }
    let payment = payment.unwrap_or(&Value::Null);

    // The answer must be about this payment.
    if let Some(echo) = str_at(
        payment,
        &["SaleTransactionIdentification", "TransactionReference"],
    ) && echo != reference
    {
        return Ok(Outcome::Unknown {
            detail: "the answer is for another payment reference".into(),
        });
    }
    let details = at(
        payment,
        &[
            "RetailerPaymentResult",
            "RequestedTransaction",
            "TransactionDetails",
        ],
    )
    .or_else(|| at(payment, &["PaymentTransaction", "TransactionDetails"]));
    if let Some(currency) = details.and_then(|d| str_at(d, &["Currency"]))
        && !currency.eq_ignore_ascii_case("EUR")
    {
        return Ok(Outcome::Unknown {
            detail: format!("approved in {currency}"),
        });
    }
    let approved = details
        .and_then(|d| d.get("TotalAmount"))
        .and_then(cents_from_json);
    let amount_cents = match (approved, expected) {
        (Some(a), Some(e)) if a != e => {
            return Ok(Outcome::Unknown {
                detail: format!("approved amount {a} cents differs from the {e} cents requested"),
            });
        }
        (Some(a), _) => a,
        (None, Some(e)) => e,
        (None, None) => {
            return Ok(Outcome::Unknown {
                detail: "approved without an amount".into(),
            });
        }
    };

    let card = env.and_then(|e| e.get("Card"));
    let brand_text = card.and_then(|c| str_at(c, &["CardBrand"]));
    let customer_receipt = payment
        .get("PaymentReceipt")
        .and_then(Value::as_array)
        .and_then(|list| {
            list.iter().find(|r| {
                str_at(r, &["DocumentQualifier"]) == Some("CustomerReceipt")
                    || str_at(r, &["OutputContent", "MessageDestination"])
                        == Some("CardholderReceipt")
            })
        })
        .and_then(|r| str_at(r, &["OutputContent", "MessageContent"]))
        .map(str::to_string);
    let normalized = brand_text.and_then(brand::normalize);
    let card_type = brand_text
        .and_then(receipt::explicit_card_type)
        .or_else(|| customer_receipt.as_deref().and_then(receipt::account_type))
        .unwrap_or_else(|| brand::card_type_of(normalized.as_deref()));

    Ok(Outcome::Approved(Approval {
        amount_cents,
        card_type,
        brand: normalized,
        terminal_reference: str_at(
            payment,
            &["POITransactionIdentification", "TransactionReference"],
        )
        .map(str::to_string),
        authorisation_code: authorisation
            .and_then(|a| str_at(a, &["AuthorisationCode"]))
            .map(str::to_string),
        masked_pan: card
            .and_then(|c| {
                str_at(c, &["MaskedPAN"])
                    .or_else(|| str_at(c, &["MaskedPan"]))
                    .or_else(|| str_at(c, &["MaskedPin"]))
            })
            .map(str::to_string),
        customer_receipt,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardType;

    fn parsed(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    // Source of the fixtures: the examples of the public OpenAPI v2,
    // https://wjx44tdpp7.execute-api.eu-west-3.amazonaws.com/OpenAPI/terminal-api-openapi-spec/openapi-v2.yaml
    // (rendered at https://docs.terminal.worldline-solutions.com/api-reference).
    const SIMPLE: &str = include_str!("../tests/fixtures/worldline_simple_payment_response.json");
    const TIP: &str =
        include_str!("../tests/fixtures/worldline_payment_with_receipts_response.json");
    const ABORTED: &str = include_str!("../tests/fixtures/worldline_aborted_response.json");
    const REFUSAL: &str = include_str!("../tests/fixtures/worldline_refusal_response.json");
    const FAILED: &str =
        include_str!("../tests/fixtures/worldline_simple_payment_error_response.json");
    const STATUS: &str = include_str!("../tests/fixtures/worldline_status_response.json");
    const REJECTION: &str = include_str!("../tests/fixtures/worldline_message_rejection.json");

    #[test]
    fn the_simple_example_is_an_approval_without_card_data() {
        let out = interpret_payment(&parsed(SIMPLE), "R26197055", Some(10_411)).unwrap();
        let Outcome::Approved(a) = out else {
            panic!("{out:?}")
        };
        assert_eq!(a.amount_cents, 10_411);
        assert_eq!(a.authorisation_code.as_deref(), Some("678803"));
        assert_eq!(a.brand, None);
        assert_eq!(a.card_type, CardType::Unknown);
    }

    #[test]
    fn the_receipt_example_gives_brand_type_pan_and_receipt() {
        let out = interpret_payment(&parsed(TIP), "R26197055", Some(10_411)).unwrap();
        let Outcome::Approved(a) = out else {
            panic!("{out:?}")
        };
        assert_eq!(a.amount_cents, 10_411);
        assert_eq!(a.brand.as_deref(), Some("Visa"));
        // "VISA CREDIT" says credit.
        assert_eq!(a.card_type, CardType::Credit);
        assert_eq!(a.masked_pan.as_deref(), Some("XXXXXXXXXXXX0119"));
        assert_eq!(a.authorisation_code.as_deref(), Some("00"));
        assert_eq!(
            a.terminal_reference.as_deref(),
            Some("2;00999100022;19940829991;79429;010099910002219940829991;")
        );
        let receipt = a.customer_receipt.unwrap();
        assert!(receipt.contains("Cardholder Receipt"));
        assert!(receipt.contains("Total-EFT EUR:             104.11"));
        assert!(
            !receipt.contains("E4B00E9F"),
            "the merchant copy is not the customer copy"
        );
    }

    #[test]
    fn an_amount_that_differs_from_the_request_is_unknown() {
        let out = interpret_payment(&parsed(TIP), "R26197055", Some(10_000)).unwrap();
        assert!(matches!(out, Outcome::Unknown { .. }), "{out:?}");
        let other = interpret_payment(&parsed(TIP), "someone-else", Some(10_411)).unwrap();
        assert!(matches!(other, Outcome::Unknown { .. }), "{other:?}");
    }

    #[test]
    fn failures_map_to_cancelled_declined_and_busy() {
        assert_eq!(
            interpret_payment(&parsed(ABORTED), "x", Some(100)).unwrap(),
            Outcome::Cancelled
        );
        let refused = interpret_payment(&parsed(REFUSAL), "x", Some(100)).unwrap();
        assert!(
            matches!(&refused, Outcome::Declined { reason } if reason.starts_with("Refusal")),
            "{refused:?}"
        );
        let failed = interpret_payment(&parsed(FAILED), "x", Some(100)).unwrap();
        assert!(
            matches!(&failed, Outcome::Declined { reason } if reason.starts_with("NotAllowed")),
            "{failed:?}"
        );
        let busy = parsed(
            r#"{"data":{"SaleToPOIServiceResponse":{"ServiceResponse":{"Response":{"Response":"Failed","ResponseReason":"Busy"}}}}}"#,
        );
        assert!(matches!(
            interpret_payment(&busy, "x", Some(100)),
            Err(EftError::Busy)
        ));
    }

    #[test]
    fn a_message_rejection_is_a_rejected_request() {
        let out = interpret_payment(&parsed(REJECTION), "x", Some(100));
        assert!(matches!(out, Err(EftError::Rejected(m)) if m.contains("UnableToProcess")));
        let dup = parsed(
            r#"{"SaleToPOIMessageRejection":{"Reject":{"RejectReason":"DuplicateMessage"}}}"#,
        );
        assert!(matches!(
            interpret_payment(&dup, "x", Some(100)),
            Ok(Outcome::Unknown { .. })
        ));
    }

    #[test]
    fn the_status_example_gives_the_approval() {
        let out = interpret_status(&parsed(STATUS), "R26197055").unwrap();
        let Outcome::Approved(a) = out else {
            panic!("{out:?}")
        };
        assert_eq!(a.amount_cents, 1011);
        assert_eq!(a.authorisation_code.as_deref(), Some("678803"));
        assert_eq!(
            a.terminal_reference.as_deref(),
            Some("2;00000000002;24863174332;13126;010000000000224863174332;")
        );
    }

    #[test]
    fn a_status_for_a_payment_in_progress_or_unknown_stays_unknown() {
        for reason in ["InProgress", "NotFound"] {
            let body = parsed(&format!(
                r#"{{"data":{{"SaleToPOIMessageStatusResponse":{{"StatusResponse":{{"MessageStatusResponseData":{{"TransactionResponse":{{"Response":"Failed","ResponseReason":"{reason}"}}}}}}}}}}}}"#
            ));
            assert!(
                matches!(interpret_status(&body, "x"), Ok(Outcome::Unknown { .. })),
                "{reason}"
            );
        }
    }

    #[test]
    fn garbage_is_unknown_not_an_error() {
        assert!(matches!(
            interpret_payment(&parsed("{}"), "x", Some(1)),
            Ok(Outcome::Unknown { .. })
        ));
        assert!(matches!(
            interpret_status(&parsed("[]"), "x"),
            Ok(Outcome::Unknown { .. })
        ));
    }
}
