//! The Telegram channel: `POST {api_base}/bot{token}/sendMessage`.
//!
//! The bot token is in the URL, so nothing here ever prints a URL: a
//! transport error goes through `reqwest::Error::without_url` before it
//! becomes text, and the answers are read as a status code and the
//! `ok` flag of the Bot API, never echoed. Each attempt has a 10 s
//! timeout; a message is dropped after 3 attempts. A refusal that
//! repeating cannot fix (a bad token, an unknown chat: 4xx other than
//! 429) is not retried.

use std::time::Duration;

use pos_fiscal::http_tls::HttpTls;

/// The timeout of one attempt.
pub const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(10);
/// The attempts of one message.
pub const MAX_ATTEMPTS: u32 = 3;
/// The wait before the second attempt (doubled before the third).
pub const DEFAULT_BACKOFF: Duration = Duration::from_secs(2);

/// How a send ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendOutcome {
    pub sent: bool,
    /// The HTTP status of the last attempt, if the endpoint answered.
    pub http_status: Option<i64>,
    /// A short reason for the log and the test message: a status
    /// description or the kind of transport error. Never a URL.
    pub detail: String,
}

/// A Telegram bot that can send to a chat.
pub struct TelegramChannel {
    client: reqwest::Client,
    api_base: String,
    token: String,
    backoff: Duration,
}

impl TelegramChannel {
    pub fn new(
        http_tls: &HttpTls,
        api_base: &str,
        token: String,
    ) -> Result<TelegramChannel, String> {
        let client = http_tls
            .client_builder()
            .and_then(|b| b.timeout(ATTEMPT_TIMEOUT).build().map_err(Into::into))
            .map_err(|e| format!("HTTPS client: {e}"))?;
        Ok(TelegramChannel {
            client,
            api_base: api_base.trim_end_matches('/').to_string(),
            token,
            backoff: DEFAULT_BACKOFF,
        })
    }

    /// The wait between attempts (tests make it short).
    pub fn with_backoff(mut self, backoff: Duration) -> Self {
        self.backoff = backoff;
        self
    }

    /// Send `text` to `chat_id`, trying up to `attempts` times.
    pub async fn send(&self, chat_id: &str, text: &str, attempts: u32) -> SendOutcome {
        let attempts = attempts.clamp(1, MAX_ATTEMPTS);
        let mut last = SendOutcome {
            sent: false,
            http_status: None,
            detail: "not sent".to_string(),
        };
        for attempt in 1..=attempts {
            if attempt > 1 {
                tokio::time::sleep(self.backoff * (1 << (attempt - 2))).await;
            }
            last = self.attempt(chat_id, text).await;
            if last.sent || !retryable(&last) {
                break;
            }
            tracing::warn!(
                attempt,
                detail = %last.detail,
                "owner alert: Telegram did not take the message"
            );
        }
        last
    }

    async fn attempt(&self, chat_id: &str, text: &str) -> SendOutcome {
        let url = format!("{}/bot{}/sendMessage", self.api_base, self.token);
        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "disable_web_page_preview": true,
        });
        let response = match self.client.post(url).json(&body).send().await {
            Ok(r) => r,
            Err(e) => {
                return SendOutcome {
                    sent: false,
                    http_status: None,
                    detail: transport_detail(e),
                };
            }
        };
        let status = response.status();
        let ok_flag = response
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| v.get("ok").and_then(serde_json::Value::as_bool));
        // 2xx with ok = false is a refusal too; 2xx without a readable
        // body counts as sent, the Bot API always answers JSON.
        let sent = status.is_success() && ok_flag != Some(false);
        SendOutcome {
            sent,
            http_status: Some(i64::from(status.as_u16())),
            detail: if sent {
                "ok".to_string()
            } else {
                format!("HTTP {}", status.as_u16())
            },
        }
    }
}

/// A transport error as text without its URL (the token is in it).
fn transport_detail(error: reqwest::Error) -> String {
    let error = error.without_url();
    let kind = if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connection failed"
    } else {
        "request failed"
    };
    // Belt and braces: whatever remains of a path carrying the token
    // never reaches a log.
    redact_bot_path(&format!("{kind}: {error}"))
}

/// Replace every `/bot<token>/` of a text by `/bot***/`.
fn redact_bot_path(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("/bot") {
        out.push_str(&rest[..at]);
        out.push_str("/bot***");
        let after = &rest[at + 4..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == ':' || c == '_' || c == '-'))
            .unwrap_or(after.len());
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// A failure repeating can cure: no answer, 429 and the 5xx.
fn retryable(outcome: &SendOutcome) -> bool {
    match outcome.http_status {
        None => true,
        Some(429) => true,
        Some(s) => s >= 500,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bot_path_is_redacted_whatever_the_token_looks_like() {
        assert_eq!(
            redact_bot_path("error sending request for url (http://h/bot123:AA-bb_cc/sendMessage)"),
            "error sending request for url (http://h/bot***/sendMessage)"
        );
        assert_eq!(redact_bot_path("no path here"), "no path here");
        assert_eq!(redact_bot_path("/bot"), "/bot***");
    }

    #[test]
    fn only_a_transient_failure_is_retried() {
        let of = |s: Option<i64>| SendOutcome {
            sent: false,
            http_status: s,
            detail: String::new(),
        };
        assert!(retryable(&of(None)));
        assert!(retryable(&of(Some(429))));
        assert!(retryable(&of(Some(502))));
        assert!(!retryable(&of(Some(400))));
        assert!(!retryable(&of(Some(401))));
        assert!(!retryable(&of(Some(403))));
    }
}
