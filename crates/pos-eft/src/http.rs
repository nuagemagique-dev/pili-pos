//! One HTTP exchange, with the failure kinds the drivers care about.

use reqwest::RequestBuilder;

/// The provider answered.
pub(crate) struct Reply {
    pub status: u16,
    pub body: String,
}

/// Why there is no answer.
#[derive(Debug)]
pub(crate) enum SendError {
    /// The request surely did not leave (connection refused, DNS, TLS
    /// handshake, request that could not be built).
    NotSent(String),
    /// The request may have reached the provider (timeout, cut
    /// connection, unreadable answer).
    Lost(String),
}

pub(crate) async fn send(request: RequestBuilder) -> Result<Reply, SendError> {
    let response = match request.send().await {
        Ok(r) => r,
        Err(e) if e.is_connect() || e.is_builder() => {
            return Err(SendError::NotSent(describe(&e)));
        }
        Err(e) => return Err(SendError::Lost(describe(&e))),
    };
    let status = response.status().as_u16();
    match response.text().await {
        Ok(body) => Ok(Reply { status, body }),
        Err(e) => Err(SendError::Lost(format!(
            "answer cut after HTTP {status}: {}",
            describe(&e)
        ))),
    }
}

/// The error with its causes but without the URL (it may carry ids).
fn describe(e: &reqwest::Error) -> String {
    let mut out = String::new();
    if e.is_timeout() {
        out.push_str("timeout");
    } else if e.is_connect() {
        out.push_str("connection failed");
    } else {
        out.push_str("request failed");
    }
    let mut source = std::error::Error::source(e);
    while let Some(s) = source {
        out.push_str(": ");
        out.push_str(&s.to_string());
        source = s.source();
    }
    out
}

/// The `message` of a JSON error body, else the body itself shortened.
pub(crate) fn error_message(body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body)
        && let Some(m) = v.get("message").and_then(|m| m.as_str())
    {
        return crate::util::truncate(m, 300);
    }
    crate::util::truncate(body.trim(), 300)
}
