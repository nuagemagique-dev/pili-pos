//! HTTP client for the Scaleway Transactional Email (TEM) API.
//!
//! The request and response shapes were checked on 2026-09-25 against
//! Scaleway's API reference
//! (https://www.scaleway.com/en/developers/api/transactional-email/)
//! and the types of its official Go SDK (`scaleway-sdk-go`,
//! `api/tem/v1alpha1`):
//!
//! - send: `POST {api}/transactional-email/v1alpha1/regions/{region}/emails`,
//!   secret key in `X-Auth-Token`; body `from{email,name}`,
//!   `to[{email}]`, `subject`, `text`, `html`, `project_id`,
//!   `attachments[{name,type,content}]` with `content` base64;
//!   response `{"emails":[{"id","message_id","status",...}]}`;
//! - probe: `GET` on the same path with `project_id` and `page_size`
//!   (ListEmails), which needs the same key and project.
//!
//! The tests mock the HTTP layer (`wiremock`) with this shape; the first
//! send against a real project is part of the pre-submission rehearsal.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::DigitalTicketConfig;
use crate::render::{Attachment, Email};

/// Default Scaleway API base URL.
pub const DEFAULT_API_BASE: &str = "https://api.scaleway.com";

/// Errors from talking to TEM. Deliberately distinguishes retryable
/// conditions (network hiccup, timeout, 5xx, 429) from permanent ones
/// (4xx other than 429, malformed config) so `pos-mail::outbox` can
/// decide whether to back off and retry or give up immediately.
#[derive(Debug, thiserror::Error)]
pub enum TemError {
    #[error("request to TEM timed out")]
    Timeout,
    #[error("network error talking to TEM: {0}")]
    Network(String),
    #[error("TEM rejected the request ({status}): {body}")]
    ClientError { status: u16, body: String },
    #[error("TEM is rate-limiting us (429): {body}")]
    RateLimited { body: String },
    #[error("TEM server error ({status}): {body}")]
    ServerError { status: u16, body: String },
    #[error("malformed response from TEM: {0}")]
    MalformedResponse(String),
    #[error("cannot read TEM API key file {path}: {source}")]
    MissingApiKey {
        path: String,
        source: std::io::Error,
    },
    #[error("TEM API key file {path} is empty")]
    EmptyApiKey { path: String },
}

impl TemError {
    /// Whether retrying the same request later has a chance of
    /// succeeding. Permanent errors (bad request, unauthorized,
    /// malformed response, config problems) should not be retried -
    /// retrying them would just burn the attempt budget for nothing.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            TemError::Timeout
                | TemError::Network(_)
                | TemError::ServerError { .. }
                | TemError::RateLimited { .. }
        )
    }

    fn from_reqwest(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            TemError::Timeout
        } else {
            // reqwest's Display never includes header values (so the
            // secret can't leak here), only method/url/cause.
            TemError::Network(e.to_string())
        }
    }

    fn from_status(status: reqwest::StatusCode, body_bytes: &[u8]) -> Self {
        let body = String::from_utf8_lossy(body_bytes).into_owned();
        if status.as_u16() == 429 {
            TemError::RateLimited { body }
        } else if status.is_server_error() {
            TemError::ServerError {
                status: status.as_u16(),
                body,
            }
        } else {
            // Client errors, and the (should-never-happen) fallback
            // for a non-2xx/4xx/5xx status: treat as permanent.
            TemError::ClientError {
                status: status.as_u16(),
                body,
            }
        }
    }
}

#[derive(Serialize)]
struct FromAddr<'a> {
    email: &'a str,
    name: &'a str,
}

#[derive(Serialize)]
struct ToAddr<'a> {
    email: &'a str,
}

#[derive(Serialize)]
struct AttachmentPayload<'a> {
    name: &'a str,
    #[serde(rename = "type")]
    content_type: &'a str,
    /// Base64-encoded content.
    content: String,
}

#[derive(Serialize)]
struct SendRequest<'a> {
    from: FromAddr<'a>,
    to: Vec<ToAddr<'a>>,
    subject: &'a str,
    text: &'a str,
    html: &'a str,
    project_id: &'a str,
    attachments: Vec<AttachmentPayload<'a>>,
}

#[derive(Debug, Deserialize)]
struct SendResponse {
    #[serde(default)]
    emails: Vec<SentEmail>,
}

#[derive(Debug, Deserialize)]
struct SentEmail {
    #[serde(default)]
    message_id: Option<String>,
    #[serde(default)]
    id: Option<String>,
}

/// Client for the Scaleway Transactional Email HTTP API.
pub struct TemClient {
    http: reqwest::Client,
    api_base: String,
    region: String,
    project_id: String,
    secret: String,
    from_email: String,
    from_name: String,
    timeout: Duration,
}

impl std::fmt::Debug for TemClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never derive Debug for this struct - it would print the
        // secret key.
        f.debug_struct("TemClient")
            .field("api_base", &self.api_base)
            .field("region", &self.region)
            .field("project_id", &self.project_id)
            .field("from_email", &self.from_email)
            .field("from_name", &self.from_name)
            .field("secret", &"<redacted>")
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Clone for TemClient {
    fn clone(&self) -> Self {
        Self {
            http: self.http.clone(),
            api_base: self.api_base.clone(),
            region: self.region.clone(),
            project_id: self.project_id.clone(),
            secret: self.secret.clone(),
            from_email: self.from_email.clone(),
            from_name: self.from_name.clone(),
            timeout: self.timeout,
        }
    }
}

impl TemClient {
    /// Build a client directly from its parts (mainly for tests - see
    /// [`TemClient::from_config`] for the normal construction path).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        http: reqwest::Client,
        api_base: impl Into<String>,
        region: impl Into<String>,
        project_id: impl Into<String>,
        secret: impl Into<String>,
        from_email: impl Into<String>,
        from_name: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            http,
            api_base: api_base.into(),
            region: region.into(),
            project_id: project_id.into(),
            secret: secret.into(),
            from_email: from_email.into(),
            from_name: from_name.into(),
            timeout,
        }
    }

    /// Build a client from [`DigitalTicketConfig`], loading the secret
    /// key from `api_key_file`.
    pub fn from_config(
        http: reqwest::Client,
        cfg: &DigitalTicketConfig,
        timeout: Duration,
    ) -> Result<Self, TemError> {
        let secret = load_secret(&cfg.api_key_file)?;
        Ok(Self::new(
            http,
            cfg.api_base
                .clone()
                .unwrap_or_else(|| DEFAULT_API_BASE.to_string()),
            cfg.region.clone(),
            cfg.project_id.clone(),
            secret,
            cfg.from_email.clone(),
            cfg.from_name.clone(),
            timeout,
        ))
    }

    fn send_url(&self) -> String {
        format!(
            "{}/transactional-email/v1alpha1/regions/{}/emails",
            self.api_base.trim_end_matches('/'),
            self.region
        )
    }

    /// Send `email`. Returns the provider's message id, used later to
    /// correlate delivery status if we ever poll for it.
    pub async fn send(&self, email: &Email) -> Result<String, TemError> {
        let attachments = email
            .attachments
            .iter()
            .map(encode_attachment)
            .collect::<Vec<_>>();
        let body = SendRequest {
            from: FromAddr {
                email: &self.from_email,
                name: &self.from_name,
            },
            to: vec![ToAddr { email: &email.to }],
            subject: &email.subject,
            text: &email.text,
            html: &email.html,
            project_id: &self.project_id,
            attachments,
        };

        let resp = self
            .http
            .post(self.send_url())
            .header("X-Auth-Token", &self.secret)
            .timeout(self.timeout)
            .json(&body)
            .send()
            .await
            .map_err(TemError::from_reqwest)?;

        let status = resp.status();
        let bytes = resp.bytes().await.map_err(TemError::from_reqwest)?;
        if !status.is_success() {
            return Err(TemError::from_status(status, &bytes));
        }

        let parsed: SendResponse = serde_json::from_slice(&bytes)
            .map_err(|e| TemError::MalformedResponse(format!("invalid JSON: {e}")))?;
        let first = parsed
            .emails
            .into_iter()
            .next()
            .ok_or_else(|| TemError::MalformedResponse("empty \"emails\" array".to_string()))?;
        first.message_id.or(first.id).ok_or_else(|| {
            TemError::MalformedResponse("neither message_id nor id present".to_string())
        })
    }

    /// A cheap authenticated call used to check the configured
    /// project/region/key actually work, without sending anything.
    /// Lists at most one email for the configured project.
    pub async fn probe(&self) -> Result<(), TemError> {
        let url = format!(
            "{}?project_id={}&page_size=1",
            self.send_url(),
            self.project_id
        );
        let resp = self
            .http
            .get(url)
            .header("X-Auth-Token", &self.secret)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(TemError::from_reqwest)?;
        let status = resp.status();
        if !status.is_success() {
            let bytes = resp.bytes().await.unwrap_or_default();
            return Err(TemError::from_status(status, &bytes));
        }
        Ok(())
    }
}

fn encode_attachment(a: &Attachment) -> AttachmentPayload<'_> {
    use base64::Engine;
    AttachmentPayload {
        name: &a.name,
        content_type: &a.content_type,
        content: base64::engine::general_purpose::STANDARD.encode(&a.content),
    }
}

fn load_secret(path: &str) -> Result<String, TemError> {
    let raw = std::fs::read_to_string(path).map_err(|source| TemError::MissingApiKey {
        path: path.to_string(),
        source,
    })?;
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        return Err(TemError::EmptyApiKey {
            path: path.to_string(),
        });
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_errors() {
        assert!(TemError::Timeout.is_retryable());
        assert!(TemError::Network("x".into()).is_retryable());
        assert!(
            TemError::ServerError {
                status: 503,
                body: String::new()
            }
            .is_retryable()
        );
        assert!(
            TemError::RateLimited {
                body: String::new()
            }
            .is_retryable()
        );
    }

    #[test]
    fn permanent_errors() {
        assert!(
            !TemError::ClientError {
                status: 400,
                body: String::new()
            }
            .is_retryable()
        );
        assert!(!TemError::MalformedResponse("x".into()).is_retryable());
        assert!(!TemError::EmptyApiKey { path: "x".into() }.is_retryable());
    }

    #[test]
    fn from_status_maps_429_to_rate_limited() {
        let e = TemError::from_status(reqwest::StatusCode::TOO_MANY_REQUESTS, b"slow down");
        assert!(matches!(e, TemError::RateLimited { .. }));
    }

    #[test]
    fn from_status_maps_5xx_to_server_error() {
        let e = TemError::from_status(reqwest::StatusCode::SERVICE_UNAVAILABLE, b"down");
        assert!(matches!(e, TemError::ServerError { status: 503, .. }));
    }

    #[test]
    fn from_status_maps_4xx_to_client_error() {
        let e = TemError::from_status(reqwest::StatusCode::BAD_REQUEST, b"bad");
        assert!(matches!(e, TemError::ClientError { status: 400, .. }));
    }

    #[test]
    fn load_secret_trims_whitespace() {
        let dir = std::env::temp_dir().join(format!("pos-mail-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("key.txt");
        std::fs::write(&path, "  secret-value\n\n").unwrap();
        let secret = load_secret(path.to_str().unwrap()).unwrap();
        assert_eq!(secret, "secret-value");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_secret_rejects_missing_file() {
        let err = load_secret("/nonexistent/path/to/tem.key").unwrap_err();
        assert!(matches!(err, TemError::MissingApiKey { .. }));
    }

    #[test]
    fn load_secret_rejects_empty_file() {
        let dir = std::env::temp_dir().join(format!("pos-mail-test-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("key.txt");
        std::fs::write(&path, "   \n").unwrap();
        let err = load_secret(path.to_str().unwrap()).unwrap_err();
        assert!(matches!(err, TemError::EmptyApiKey { .. }));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn debug_impl_redacts_the_secret() {
        let client = TemClient::new(
            reqwest::Client::new(),
            DEFAULT_API_BASE,
            "fr-par",
            "project-1",
            "super-secret-value",
            "from@example.com",
            "From",
            Duration::from_secs(5),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("super-secret-value"));
        assert!(debug.contains("<redacted>"));
    }
}
