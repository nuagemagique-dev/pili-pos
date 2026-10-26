//! Thin HTTP wrapper around `reqwest` for `HubRise` API calls.

use reqwest::header::{HeaderMap, HeaderValue};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::errors::{HubriseError, Result};

/// `HubRise` HTTP client. Cheap to clone - `reqwest::Client` is internally `Arc`.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    access_token: String,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("access_token", &"<redacted>")
            .finish_non_exhaustive()
    }
}

/// One page of a paginated list and the cursor of the next, if any
/// (`X-Cursor-Next`, `docs/hubrise/01-general-concepts.md`).
#[derive(Debug)]
pub struct PageOf<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

impl Client {
    pub fn new(
        http: reqwest::Client,
        base_url: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Self {
        Self {
            http,
            base_url: base_url.into(),
            access_token: access_token.into(),
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut h = HeaderMap::new();
        if let Ok(tok) = HeaderValue::from_str(&self.access_token) {
            h.insert("X-Access-Token", tok);
        }
        h
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    /// Send `req`; a non-2xx status becomes an error (401 its own).
    async fn send(&self, req: reqwest::RequestBuilder) -> Result<(HeaderMap, bytes::Bytes)> {
        let resp = req.headers(self.headers()).send().await?;
        let status = resp.status();
        let headers = resp.headers().clone();
        let body = resp.bytes().await?;
        if !status.is_success() {
            if status.as_u16() == 401 {
                return Err(HubriseError::Unauthorized);
            }
            return Err(HubriseError::Api {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&body).into_owned(),
            });
        }
        Ok((headers, body))
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let (_, body) = self.send(self.http.get(self.url(path))).await?;
        Ok(serde_json::from_slice(&body)?)
    }

    /// One page of a paginated list: `path` plus `cursor` when given.
    pub async fn get_page<T: DeserializeOwned>(
        &self,
        path: &str,
        cursor: Option<&str>,
    ) -> Result<PageOf<T>> {
        let mut url = url::Url::parse(&self.url(path))?;
        if let Some(c) = cursor {
            url.query_pairs_mut().append_pair("cursor", c);
        }
        let (headers, body) = self.send(self.http.get(url.as_str())).await?;
        let next_cursor = headers
            .get("X-Cursor-Next")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        Ok(PageOf {
            items: serde_json::from_slice(&body)?,
            next_cursor,
        })
    }

    pub async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let (_, bytes) = self.send(self.http.post(self.url(path)).json(body)).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn put_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let (_, bytes) = self.send(self.http.put(self.url(path)).json(body)).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn patch_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let (_, bytes) = self
            .send(self.http.patch(self.url(path)).json(body))
            .await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        self.send(self.http.delete(self.url(path))).await?;
        Ok(())
    }
}
