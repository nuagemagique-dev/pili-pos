use thiserror::Error;

#[derive(Debug, Error)]
pub enum PeppolError {
    #[error("XML serialization failed: {0}")]
    Xml(String),
    #[error("missing required field for BIS Billing 3.0: {0}")]
    MissingField(&'static str),
    #[error("access point error: {0}")]
    AccessPoint(String),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}
