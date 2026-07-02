//! Peppol Access Point trait + reference implementations.
//!
//! An Access Point (AP) is a Peppol-registered service that takes a
//! signed UBL invoice and delivers it over AS4 transport to the
//! recipient's AP, which in turn passes it to the recipient's
//! accounting system. The POS does NOT speak AS4 directly — that's
//! the AP's job. From our side it's an HTTP POST to the AP's REST
//! API.
//!
//! Two production-relevant providers in Belgium are
//! [Storecove](https://www.storecove.com) and
//! [Unifiedpost](https://www.unifiedpost.com). Both expose simple
//! REST APIs that accept either UBL XML or the AP-vendor's
//! intermediate JSON. The trait below normalises across either.

use async_trait::async_trait;

use crate::errors::PeppolError;

/// Outcome of a successful delivery to an Access Point. We persist this
/// in the local `peppol_deliveries` table for audit.
#[derive(Debug, Clone)]
pub struct DeliveryReceipt {
    /// AP-provided message ID (used for status polling).
    pub message_id: String,
    /// ISO-8601 timestamp at which the AP accepted the document.
    pub accepted_at: String,
    /// Raw response body from the AP (kept for the dossier audit
    /// trail; never displayed to operators).
    pub raw_response: String,
}

/// Common API every Access Point integration must expose.
#[async_trait]
pub trait AccessPoint: Send + Sync {
    /// Submit a UBL XML document to the AP. Returns once the AP has
    /// **accepted** the document for delivery — not when the recipient
    /// has received it. Final delivery is asynchronous and surfaced
    /// via a separate status query (out of scope for cert MVP).
    async fn submit(&self, ubl_xml: &str) -> Result<DeliveryReceipt, PeppolError>;

    /// Short human label describing the AP, for the audit trail.
    fn vendor(&self) -> &'static str;
}

/// No-op AP that just logs the submission. Used in tests and as the
/// default when no AP credentials are configured — operators can issue
/// B2B invoices in dev mode and inspect the UBL output without
/// triggering live Peppol traffic. Production builds (cert) ship
/// without this implementation registered.
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

    fn vendor(&self) -> &'static str {
        "logging-only (no live delivery)"
    }
}

/// Storecove implementation skeleton. Wire-up is the responsibility of
/// the operator: an API key obtained from Storecove's dashboard, a
/// sender `endpointID` registered on Peppol (Tokyo Sushi Bar:
/// `0208:BE0499999960`), and a contract for the chosen
/// commercial plan. **Currently unimplemented** — the `submit` call
/// returns an error pointing operators to the procurement doc.
pub struct StorecoveAccessPoint {
    pub api_key: String,
    pub base_url: String,
}

#[async_trait]
impl AccessPoint for StorecoveAccessPoint {
    async fn submit(&self, _ubl_xml: &str) -> Result<DeliveryReceipt, PeppolError> {
        // Production: wrap the UBL document in Storecove's
        // `LegalDocumentInvoice` JSON envelope and POST to
        // `{base_url}/v2/legal_documents`. The response carries
        // `document_id` which becomes our `message_id`.
        // For now the call returns an explicit error so anyone
        // wiring up the production path notices it immediately.
        Err(PeppolError::AccessPoint(format!(
            "Storecove integration not wired up yet. \
             api_key configured for base_url={}. See \
             docs/SUBMISSION_ARTIFACTS/PEPPOL_AP_DECISION.md for the rollout plan.",
            self.base_url
        )))
    }

    fn vendor(&self) -> &'static str {
        "storecove"
    }
}
