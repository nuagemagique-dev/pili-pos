//! HTTPS PUT upload of the export files to the endpoint (decision X1D8).
//!
//! One request per file: `PUT <base url>/<export_id>/<file name>` with the
//! headers `Authorization: Bearer <token>`, `Content-Type`,
//! `X-Content-SHA256` (lowercase hex of the body) and `User-Agent:
//! pili-export/1`. A 2xx answer is success, 401 and 403 mean the token is
//! refused ([`UploadError::Unauthorized`]), any other status is
//! [`UploadError::Http`]. The retry policy (1, 2, 4 ... minutes, capped at 60)
//! belongs to the outbox, not to this module.
//!
//! The caller builds the [`reqwest::Client`] from the engine's `HttpTls` with a
//! 120 second timeout per request (X1D8); this module never builds one, so the
//! trust store, the proxy and the timeout stay the engine's decision. The file
//! is read into memory (`std::fs::read` on a blocking thread): the reqwest `stream` feature is not
//! enabled in the certified build, and a file above [`MAX_UPLOAD_BYTES`] is
//! refused before it is read.
//!
//! The error texts are meant for the operator (the export status shows them):
//! short, English, and never the token, the `Authorization` header or the URL.

use std::path::Path;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue, USER_AGENT};

/// Largest file the client uploads: 512 MiB (X1D8).
pub const MAX_UPLOAD_BYTES: i64 = 512 * 1024 * 1024;

/// Value of the `User-Agent` header.
const AGENT: &str = "pili-export/1";

/// Why one upload failed. `i64` payloads because the FFI forbids unsigned
/// integers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadError {
    /// The endpoint refused the token (HTTP 401 or 403); the payload is the
    /// status.
    Unauthorized(i64),
    /// Any other non-2xx answer; the payload is the status.
    Http(i64),
    /// No answer: connection, TLS, timeout or protocol error. The text is the
    /// reason, without URL.
    Transport(String),
    /// The local file could not be read; the text is the reason.
    Io(String),
    /// The file is larger than [`MAX_UPLOAD_BYTES`]; the payload is its size.
    TooLarge(i64),
}

impl std::fmt::Display for UploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UploadError::Unauthorized(status) => write!(f, "unauthorized (HTTP {status})"),
            UploadError::Http(status) => write!(f, "HTTP {status}"),
            UploadError::Transport(reason) => write!(f, "connection failed: {reason}"),
            UploadError::Io(reason) => write!(f, "file unreadable: {reason}"),
            UploadError::TooLarge(size) => write!(f, "file too large: {size} bytes"),
        }
    }
}

impl std::error::Error for UploadError {}

/// Uploads files to one endpoint.
#[derive(Clone, Debug)]
pub struct Uploader {
    client: reqwest::Client,
    base_url: String,
}

impl Uploader {
    /// `client` comes from the engine's `HttpTls` (120 s timeout); `base_url`
    /// is the configured endpoint, for example `https://export.example/pos`.
    /// A trailing slash is ignored.
    pub fn new(client: reqwest::Client, base_url: &str) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// PUT the file at `path` to `<base url>/<export_id>/<file_name>`.
    ///
    /// `sha256_hex` is the lowercase hex SHA-256 of the file as the manifest
    /// states it; it is sent as `X-Content-SHA256` and the receiver refuses a
    /// body that does not match. `content_type` is `application/octet-stream`
    /// for the data files and `application/json` for the manifest.
    pub async fn put_file(
        &self,
        export_id: &str,
        file_name: &str,
        path: &Path,
        sha256_hex: &str,
        token: &str,
        content_type: &str,
    ) -> Result<(), UploadError> {
        let path = path.to_path_buf();
        let body = tokio::task::spawn_blocking(move || read_checked(&path))
            .await
            .map_err(|e| UploadError::Io(e.to_string()))??;

        // The `|_|` error mappers below are deliberate: the invalid-header error of
        // `http` can echo the offending value, and these values include the bearer
        // token, which must never reach a message, a log or the outbox.
        let mut authorization = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| UploadError::Transport("the token is not a valid header value".into()))?;
        authorization.set_sensitive(true);
        let content_type = HeaderValue::from_str(content_type)
            .map_err(|_| UploadError::Transport("the content type is not valid".into()))?;
        let sha256 = HeaderValue::from_str(sha256_hex)
            .map_err(|_| UploadError::Transport("the hash is not a valid header value".into()))?;

        let url = format!("{}/{export_id}/{file_name}", self.base_url);
        let response = self
            .client
            .put(url)
            .header(AUTHORIZATION, authorization)
            .header(CONTENT_TYPE, content_type)
            .header("X-Content-SHA256", sha256)
            .header(USER_AGENT, AGENT)
            .body(body)
            .send()
            .await
            .map_err(|e| UploadError::Transport(transport_reason(e)))?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
        {
            Err(UploadError::Unauthorized(i64::from(status.as_u16())))
        } else {
            Err(UploadError::Http(i64::from(status.as_u16())))
        }
    }
}

/// Size check, then read the whole file. Runs on a blocking thread: the
/// `tokio` `fs` feature is not enabled in this crate (and `std::fs` would
/// block the executor). Returns the bytes.
///
/// Memory note: the body is held in memory, so an upload can take up to
/// [`MAX_UPLOAD_BYTES`] (512 MiB) of RAM. That is deliberate: the `stream`
/// feature of `reqwest` is off in the certified build, and a file that is
/// re-sent whole on retry keeps the `X-Content-SHA256` check simple. The
/// size check bounds it.
fn read_checked(path: &Path) -> Result<Vec<u8>, UploadError> {
    let size = std::fs::metadata(path)
        .map_err(|e| UploadError::Io(e.to_string()))?
        .len();
    if size > MAX_UPLOAD_BYTES as u64 {
        return Err(UploadError::TooLarge(clamp_i64(size)));
    }
    let body = std::fs::read(path).map_err(|e| UploadError::Io(e.to_string()))?;
    // The file may have grown between the size check and the read.
    if body.len() as u64 > MAX_UPLOAD_BYTES as u64 {
        return Err(UploadError::TooLarge(clamp_i64(body.len() as u64)));
    }
    Ok(body)
}

fn clamp_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// The reason of a reqwest error without the URL, which can carry a query:
/// the error itself, then its causes (`timed out`, `Connection refused`, ...).
fn transport_reason(error: reqwest::Error) -> String {
    let error = error.without_url();
    let mut reason = error.to_string();
    let mut source = std::error::Error::source(&error);
    while let Some(cause) = source {
        let text = cause.to_string();
        if !reason.contains(&text) {
            reason.push_str(": ");
            reason.push_str(&text);
        }
        source = cause.source();
    }
    reason
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_texts() {
        assert_eq!(
            UploadError::Unauthorized(401).to_string(),
            "unauthorized (HTTP 401)"
        );
        assert_eq!(UploadError::Http(503).to_string(), "HTTP 503");
        assert_eq!(
            UploadError::Transport("timed out".into()).to_string(),
            "connection failed: timed out"
        );
        assert_eq!(
            UploadError::Io("No such file".into()).to_string(),
            "file unreadable: No such file"
        );
        assert_eq!(
            UploadError::TooLarge(600_000_000).to_string(),
            "file too large: 600000000 bytes"
        );
    }

    #[test]
    fn base_url_trailing_slashes_are_ignored() {
        let uploader = Uploader::new(reqwest::Client::new(), "http://127.0.0.1:1//");
        assert_eq!(uploader.base_url, "http://127.0.0.1:1");
    }

    #[test]
    fn the_limit_is_512_mib() {
        assert_eq!(MAX_UPLOAD_BYTES, 536_870_912);
    }
}
