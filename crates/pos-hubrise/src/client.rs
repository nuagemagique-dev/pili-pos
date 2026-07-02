//! Thin HTTP wrapper around `reqwest` for HubRise API calls.

use reqwest::header::{HeaderMap, HeaderValue};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::errors::{HubriseError, Result};

/// HubRise HTTP client. Cheap to clone — `reqwest::Client` is internally `Arc`.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    access_token: String,
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

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self
            .http
            .get(self.url(path))
            .headers(self.headers())
            .send()
            .await?;
        let status = resp.status();
        let body_bytes = resp.bytes().await?;
        if !status.is_success() {
            if status.as_u16() == 401 {
                return Err(HubriseError::Unauthorized);
            }
            return Err(HubriseError::Api {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&body_bytes).into_owned(),
            });
        }
        Ok(serde_json::from_slice(&body_bytes)?)
    }

    pub async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let resp = self
            .http
            .post(self.url(path))
            .headers(self.headers())
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        let body_bytes = resp.bytes().await?;
        if !status.is_success() {
            if status.as_u16() == 401 {
                return Err(HubriseError::Unauthorized);
            }
            return Err(HubriseError::Api {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&body_bytes).into_owned(),
            });
        }
        Ok(serde_json::from_slice(&body_bytes)?)
    }

    pub async fn patch_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let resp = self
            .http
            .patch(self.url(path))
            .headers(self.headers())
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        let body_bytes = resp.bytes().await?;
        if !status.is_success() {
            if status.as_u16() == 401 {
                return Err(HubriseError::Unauthorized);
            }
            return Err(HubriseError::Api {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&body_bytes).into_owned(),
            });
        }
        Ok(serde_json::from_slice(&body_bytes)?)
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        let resp = self
            .http
            .delete(self.url(path))
            .headers(self.headers())
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            if status.as_u16() == 401 {
                return Err(HubriseError::Unauthorized);
            }
            let body_bytes = resp.bytes().await.unwrap_or_default();
            return Err(HubriseError::Api {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&body_bytes).into_owned(),
            });
        }
        Ok(())
    }
}
