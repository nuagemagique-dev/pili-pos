use thiserror::Error;

#[derive(Debug, Error)]
pub enum PeppolError {
    #[error("XML serialization failed: {0}")]
    Xml(String),
    #[error("missing required field for BIS Billing 3.0: {0}")]
    MissingField(&'static str),
    /// The access point answered something unusable, or the configuration of the
    /// access point is wrong.
    #[error("access point error: {0}")]
    AccessPoint(String),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    /// A failure that may go away: no answer, a timeout, HTTP 429 or 5xx. The queue
    /// retries it with a backoff.
    #[error("transient access point failure (HTTP {status:?}): {detail}")]
    Transient { status: Option<u16>, detail: String },
    /// The access point refused the document or the request for good (HTTP 4xx other
    /// than 401 and 429). The queue parks the delivery.
    #[error("access point refused (HTTP {status}): {detail}")]
    Rejected { status: u16, detail: String },
    /// The API key is wrong or expired (HTTP 401 or 403): every later call would fail
    /// the same way, so the queue parks the delivery and stops its pass.
    #[error("access point authentication failed (HTTP {status}): {detail}")]
    Unauthorized { status: u16, detail: String },
}

impl PeppolError {
    /// The HTTP status behind this error, when there was an answer.
    pub fn http_status(&self) -> Option<u16> {
        match self {
            Self::Transient { status, .. } => *status,
            Self::Rejected { status, .. } | Self::Unauthorized { status, .. } => Some(*status),
            _ => None,
        }
    }
}
