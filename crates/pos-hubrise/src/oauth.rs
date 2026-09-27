//! OAuth 2.0 helper for the HubRise authorization code flow.

use serde::{Deserialize, Serialize};
use url::Url;

use crate::errors::{HubriseError, Result};

/// Configuration for the HubRise OAuth flow on the POS side.
///
/// Unlike the webshop backend, the POS cannot host a server-side
/// redirect. We use the "installed app" flow where the mock returns
/// an auth code in a response body and the user copy-pastes it back
/// into Settings.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub oauth_base_url: String,
    pub base_url: String,
    pub client_id: String,
    /// Required by the real HubRise token exchange. Optional for the
    /// mock (which doesn't verify secrets).
    pub client_secret: String,
    pub redirect_uri: String,
    pub scope: String,
}

/// Build the authorize URL to open in an external browser.
pub fn build_authorize_url(cfg: &OAuthConfig, state: Option<&str>) -> Result<String> {
    let mut url = Url::parse(&format!("{}/authorize", cfg.oauth_base_url))?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("redirect_uri", &cfg.redirect_uri);
        q.append_pair("client_id", &cfg.client_id);
        q.append_pair("scope", &cfg.scope);
        if let Some(s) = state {
            q.append_pair("state", s);
        }
    }
    Ok(url.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenExchangeResponse {
    pub access_token: String,
    #[serde(default)]
    pub token_type: String,
    pub account_id: String,
    pub location_id: String,
    #[serde(default)]
    pub catalog_id: Option<String>,
    #[serde(default)]
    pub customer_list_id: Option<String>,
    pub account_name: String,
    pub location_name: String,
    #[serde(default)]
    pub catalog_name: Option<String>,
    #[serde(default)]
    pub customer_list_name: Option<String>,
}

/// Exchange an auth code for a HubRise access token.
///
/// As `docs/hubrise/02-authentication.md` describes it: a form body
/// `code=...`, the client in HTTP Basic auth (`client_id:client_secret`).
pub async fn exchange_code(
    http: &reqwest::Client,
    cfg: &OAuthConfig,
    code: &str,
) -> Result<TokenExchangeResponse> {
    use base64::Engine;
    let url = format!("{}/token", cfg.oauth_base_url);
    let basic = base64::engine::general_purpose::STANDARD
        .encode(format!("{}:{}", cfg.client_id, cfg.client_secret));
    let body = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("code", code)
        .finish();
    let resp = http
        .post(&url)
        .header("Authorization", format!("Basic {basic}"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await?;
    let status = resp.status();
    let bytes = resp.bytes().await?;
    if !status.is_success() {
        return Err(HubriseError::Api {
            status: status.as_u16(),
            body: String::from_utf8_lossy(&bytes).into_owned(),
        });
    }
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> OAuthConfig {
        OAuthConfig {
            oauth_base_url: "https://manager.hubrise.com/oauth2/v1".to_string(),
            base_url: "https://api.hubrise.com/v1".to_string(),
            client_id: "123456.clients.hubrise.com".to_string(),
            client_secret: "secret123".to_string(),
            redirect_uri: "urn:ietf:wg:oauth:2.0:oob".to_string(),
            scope: "location[orders.write,catalog.read]".to_string(),
        }
    }

    #[test]
    fn authorize_url_contains_required_params() {
        let url = build_authorize_url(&test_config(), None).unwrap();
        assert!(url.starts_with("https://manager.hubrise.com/oauth2/v1/authorize?"));
        assert!(url.contains("client_id=123456.clients.hubrise.com"));
        assert!(url.contains("redirect_uri=urn"));
        assert!(url.contains("scope=location"));
    }

    #[test]
    fn authorize_url_includes_state_when_provided() {
        let url = build_authorize_url(&test_config(), Some("mystate")).unwrap();
        assert!(url.contains("state=mystate"));
    }

    #[test]
    fn authorize_url_omits_state_when_none() {
        let url = build_authorize_url(&test_config(), None).unwrap();
        assert!(!url.contains("state="));
    }

    #[test]
    fn token_exchange_response_deserializes() {
        let json = r#"{
            "access_token": "tok-abc",
            "token_type": "Bearer",
            "account_id": "acc-1",
            "location_id": "loc-1",
            "catalog_id": "cat-1",
            "customer_list_id": null,
            "account_name": "Test",
            "location_name": "Liège",
            "catalog_name": "Menu",
            "customer_list_name": null
        }"#;
        let resp: TokenExchangeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.access_token, "tok-abc");
        assert_eq!(resp.location_name, "Liège");
        assert_eq!(resp.catalog_id, Some("cat-1".to_string()));
        assert!(resp.customer_list_id.is_none());
    }
}
