//! Peppol Access Point trait and its implementations.
//!
//! An Access Point (AP) is a Peppol-registered service that takes a UBL invoice and delivers
//! it over AS4 to the recipient's AP. The POS does not speak AS4: from our side it is an HTTP
//! API. [`StorecoveAccessPoint`] speaks the Storecove API v2 shape and is tested only against
//! the local mock (`pos-fdm-mock`, routes under `/storecove/api/v2`); a live check against
//! the Storecove sandbox is an owner task (plan Q2). [`LoggingAccessPoint`] is the
//! development provider: it logs the document and sends nothing.
//!
//! Failures are classified for the queue (V11-D12): [`PeppolError::Transient`] is retried,
//! [`PeppolError::Rejected`] parks the delivery, [`PeppolError::Unauthorized`] parks it and
//! stops the pass.

use std::time::Duration;

use async_trait::async_trait;
use base64::Engine as _;
use serde_json::{Value, json};

use crate::errors::PeppolError;

/// Time allowed to a discovery call (V11-D10): the invoice must never wait on it.
pub const LOOKUP_TIMEOUT: Duration = Duration::from_secs(3);
/// Time allowed to a document submission.
pub const SUBMIT_TIMEOUT: Duration = Duration::from_secs(30);

/// Outcome of a successful delivery to an Access Point. We persist this in the local
/// `peppol_deliveries` table for audit.
#[derive(Debug, Clone)]
pub struct DeliveryReceipt {
    /// AP-provided message ID (used for status polling).
    pub message_id: String,
    /// ISO-8601 timestamp at which the AP accepted the document.
    pub accepted_at: String,
    /// Raw response body from the AP (kept for the dossier audit trail; never displayed to
    /// operators).
    pub raw_response: String,
}

/// Whether a participant can receive invoices over Peppol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupResult {
    Registered,
    NotRegistered,
}

impl LookupResult {
    /// The value stored on the customer and the delivery row.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::NotRegistered => "not_registered",
        }
    }
}

/// Common API every Access Point integration must expose.
#[async_trait]
pub trait AccessPoint: Send + Sync {
    /// Submit a UBL XML document to the AP. Returns once the AP has **accepted** the document
    /// for delivery, not when the recipient has received it.
    async fn submit(&self, ubl_xml: &str) -> Result<DeliveryReceipt, PeppolError>;

    /// Ask whether `participant` (`0208:0308357159`) is registered on the Peppol network.
    async fn lookup(&self, participant: &str) -> Result<LookupResult, PeppolError>;

    /// Submit `ubl_xml` to `participant`. APIs that route on an explicit recipient (Storecove)
    /// override this; the default is [`AccessPoint::submit`]. `idempotency_key` is stable for
    /// one delivery across retries: an AP that supports it uses it so a lost answer followed
    /// by a retry cannot create a second document.
    async fn submit_to(
        &self,
        _participant: &str,
        ubl_xml: &str,
        _idempotency_key: &str,
    ) -> Result<DeliveryReceipt, PeppolError> {
        self.submit(ubl_xml).await
    }

    /// Short human label describing the AP, for the audit trail.
    fn vendor(&self) -> &'static str;
}

/// No-op AP that just logs the submission. The development provider (`peppol.provider` is
/// `logging`): operators can issue B2B invoices and inspect the UBL output without live
/// Peppol traffic. Every participant is "registered".
#[derive(Debug)]
pub struct LoggingAccessPoint;

#[async_trait]
impl AccessPoint for LoggingAccessPoint {
    async fn submit(&self, ubl_xml: &str) -> Result<DeliveryReceipt, PeppolError> {
        tracing::info!(
            bytes = ubl_xml.len(),
            "LoggingAccessPoint: would submit UBL document"
        );
        Ok(DeliveryReceipt {
            message_id: format!("logging-{}", uuid::Uuid::new_v4().simple()),
            accepted_at: chrono::Utc::now().to_rfc3339(),
            raw_response: "{}".into(),
        })
    }

    async fn lookup(&self, _participant: &str) -> Result<LookupResult, PeppolError> {
        Ok(LookupResult::Registered)
    }

    fn vendor(&self) -> &'static str {
        "logging-only (no live delivery)"
    }
}

/// Storecove API v2 access point.
///
/// - Discovery: `POST {base}/api/v2/discovery/receives`, JSON `{documentTypes, network,
///   metaScheme, scheme, identifier}`; HTTP 200 with `{"code": "OK"}` = registered, HTTP 404
///   or any other `code` = not registered.
/// - Submission: `POST {base}/api/v2/document_submissions`, JSON `{legalEntityId, routing,
///   document}` with the base64 UBL in `document.rawDocumentData.document`; the answer's
///   `guid` is our message id.
/// - Both with `Authorization: Bearer <api key>`.
pub struct StorecoveAccessPoint {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    legal_entity_id: i64,
}

impl std::fmt::Debug for StorecoveAccessPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorecoveAccessPoint")
            .field("base_url", &self.base_url)
            .field("api_key", &"<redacted>")
            .field("legal_entity_id", &self.legal_entity_id)
            .finish_non_exhaustive()
    }
}

impl StorecoveAccessPoint {
    pub fn new(
        client: reqwest::Client,
        base_url: &str,
        api_key: &str,
        legal_entity_id: i64,
    ) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            legal_entity_id,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v2/{path}", self.base_url)
    }

    async fn post(
        &self,
        path: &str,
        body: &Value,
        timeout: Duration,
    ) -> Result<(u16, String), PeppolError> {
        let response = self
            .client
            .post(self.url(path))
            .bearer_auth(&self.api_key)
            .timeout(timeout)
            .json(body)
            .send()
            .await
            .map_err(|e| PeppolError::Transient {
                status: None,
                detail: e.without_url().to_string(),
            })?;
        let status = response.status().as_u16();
        let text = response.text().await.map_err(|e| PeppolError::Transient {
            status: Some(status),
            detail: e.without_url().to_string(),
        })?;
        Ok((status, text))
    }
}

/// The `idempotencyGuid` of a submission: a UUID derived from the legal entity, the caller's
/// stable key and the document itself, so the same delivery always sends the same GUID while
/// another document never reuses it (the key is a local row id that repeats after a database
/// reset or on a second till). Storecove answers a repeated GUID with the first submission's
/// result instead of creating a second document.
/// (The field name is from Storecove's `document_submissions` API; the mock ignores it.)
pub(crate) fn idempotency_guid(legal_entity_id: i64, key: &str, document: &str) -> String {
    fn fnv1a(seed: u64, text: &str) -> u64 {
        text.bytes().fold(seed, |hash, b| {
            (hash ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }
    let text = format!("{legal_entity_id}\u{1f}{key}\u{1f}{document}");
    let high = fnv1a(0xcbf2_9ce4_8422_2325, &text);
    let low = fnv1a(0x8422_2325_cbf2_9ce4, &text);
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&high.to_be_bytes());
    bytes[8..].copy_from_slice(&low.to_be_bytes());
    uuid::Builder::from_random_bytes(bytes)
        .into_uuid()
        .to_string()
}

/// Split a Peppol participant id `scheme:value`.
fn split_participant(participant: &str) -> Result<(&str, &str), PeppolError> {
    participant
        .split_once(':')
        .filter(|(s, v)| !s.is_empty() && !v.is_empty())
        .ok_or_else(|| PeppolError::AccessPoint(format!("bad participant id {participant:?}")))
}

/// Map a non-2xx answer to the error the queue acts on.
fn classify(status: u16, body: &str) -> PeppolError {
    let detail: String = body.chars().take(500).collect();
    match status {
        401 | 403 => PeppolError::Unauthorized { status, detail },
        408 | 429 | 500..=599 => PeppolError::Transient {
            status: Some(status),
            detail,
        },
        _ => PeppolError::Rejected { status, detail },
    }
}

#[async_trait]
impl AccessPoint for StorecoveAccessPoint {
    async fn submit(&self, _ubl_xml: &str) -> Result<DeliveryReceipt, PeppolError> {
        Err(PeppolError::AccessPoint(
            "Storecove needs the recipient: use submit_to".to_string(),
        ))
    }

    async fn submit_to(
        &self,
        participant: &str,
        ubl_xml: &str,
        idempotency_key: &str,
    ) -> Result<DeliveryReceipt, PeppolError> {
        let (scheme, value) = split_participant(participant)?;
        let body = json!({
            "legalEntityId": self.legal_entity_id,
            "idempotencyGuid": idempotency_guid(self.legal_entity_id, idempotency_key, ubl_xml),
            "routing": {
                "eIdentifiers": [{ "scheme": "iso6523-actorid-upis", "id": format!("{scheme}:{value}") }]
            },
            "document": {
                "documentType": "invoice",
                "rawDocumentData": {
                    "document": base64::engine::general_purpose::STANDARD.encode(ubl_xml),
                    "parse": true,
                    "parseStrategy": "ubl"
                }
            }
        });
        let (status, text) = self
            .post("document_submissions", &body, SUBMIT_TIMEOUT)
            .await?;
        if !(200..300).contains(&status) {
            return Err(classify(status, &text));
        }
        let guid = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v["guid"].as_str().map(str::to_string))
            .ok_or_else(|| {
                PeppolError::AccessPoint(format!("no guid in the AP answer: {text:.200}"))
            })?;
        Ok(DeliveryReceipt {
            message_id: guid,
            accepted_at: chrono::Utc::now().to_rfc3339(),
            raw_response: text,
        })
    }

    async fn lookup(&self, participant: &str) -> Result<LookupResult, PeppolError> {
        split_participant(participant)?;
        let body = json!({
            "documentTypes": ["invoice"],
            "network": "peppol",
            "metaScheme": "iso6523-actorid-upis",
            "scheme": "iso6523-actorid-upis",
            "identifier": participant
        });
        let (status, text) = self
            .post("discovery/receives", &body, LOOKUP_TIMEOUT)
            .await?;
        match status {
            200..=299 => {
                let code = serde_json::from_str::<Value>(&text)
                    .ok()
                    .and_then(|v| v["code"].as_str().map(str::to_string));
                Ok(if code.as_deref() == Some("OK") {
                    LookupResult::Registered
                } else {
                    LookupResult::NotRegistered
                })
            }
            404 => Ok(LookupResult::NotRegistered),
            _ => Err(classify(status, &text)),
        }
    }

    fn vendor(&self) -> &'static str {
        "storecove"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_are_classified_for_the_queue() {
        assert!(matches!(
            classify(401, "x"),
            PeppolError::Unauthorized { status: 401, .. }
        ));
        assert!(matches!(
            classify(403, "x"),
            PeppolError::Unauthorized { .. }
        ));
        for s in [408, 429, 500, 502, 503, 599] {
            assert!(
                matches!(
                    classify(s, "x"),
                    PeppolError::Transient {
                        status: Some(_),
                        ..
                    }
                ),
                "{s}"
            );
        }
        for s in [400, 404, 409, 422] {
            assert!(
                matches!(classify(s, "x"), PeppolError::Rejected { .. }),
                "{s}"
            );
        }
        assert_eq!(classify(422, "x").http_status(), Some(422));
        assert_eq!(PeppolError::AccessPoint("x".into()).http_status(), None);
    }

    #[test]
    fn a_participant_id_is_scheme_and_value() {
        assert_eq!(
            split_participant("0208:0308357159").unwrap(),
            ("0208", "0308357159")
        );
        assert!(split_participant("0308357159").is_err());
        assert!(split_participant(":x").is_err());
        assert!(split_participant("0208:").is_err());
    }

    #[tokio::test]
    async fn the_logging_access_point_accepts_everything() {
        let ap = LoggingAccessPoint;
        assert_eq!(
            ap.lookup("0208:0308357159").await.unwrap(),
            LookupResult::Registered
        );
        let r = ap.submit("<x/>").await.unwrap();
        assert!(r.message_id.starts_with("logging-"));
        assert_eq!(LookupResult::NotRegistered.as_str(), "not_registered");
        assert_eq!(LookupResult::Registered.as_str(), "registered");
    }

    #[test]
    fn the_same_delivery_always_sends_the_same_guid() {
        let a = idempotency_guid(7, "delivery-12", "<Invoice>1</Invoice>");
        assert_eq!(
            a,
            idempotency_guid(7, "delivery-12", "<Invoice>1</Invoice>")
        );
        assert_ne!(
            a,
            idempotency_guid(7, "delivery-13", "<Invoice>1</Invoice>")
        );
        assert_ne!(
            a,
            idempotency_guid(8, "delivery-12", "<Invoice>1</Invoice>")
        );
        assert!(uuid::Uuid::parse_str(&a).is_ok());
    }

    /// After a database reset the same delivery id comes back for another invoice.
    #[test]
    fn another_document_under_the_same_key_gets_another_guid() {
        assert_ne!(
            idempotency_guid(7, "delivery-7", "<Invoice>old</Invoice>"),
            idempotency_guid(7, "delivery-7", "<Invoice>new</Invoice>")
        );
    }

    #[tokio::test]
    async fn storecove_without_a_reachable_host_is_transient() {
        let ap = StorecoveAccessPoint::new(reqwest::Client::new(), "http://127.0.0.1:9", "k", 1);
        let err = ap.lookup("0208:0308357159").await.unwrap_err();
        assert!(
            matches!(err, PeppolError::Transient { status: None, .. }),
            "{err:?}"
        );
        let err = ap
            .submit_to("0208:0308357159", "<x/>", "k")
            .await
            .unwrap_err();
        assert!(
            matches!(err, PeppolError::Transient { status: None, .. }),
            "{err:?}"
        );
        // Plain submit has no recipient for Storecove.
        assert!(matches!(
            ap.submit("<x/>").await,
            Err(PeppolError::AccessPoint(_))
        ));
    }
}
