//! OAuth 2.0 flow types for `HubRise` manager.

use serde::{Deserialize, Serialize};

/// Parameters appended to the `HubRise` authorize URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizeParams {
    pub redirect_uri: String,
    pub client_id: String,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Response body for `POST /oauth2/v1/token`.
#[derive(Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub account_id: String,
    pub location_id: String,
    pub catalog_id: Option<String>,
    pub customer_list_id: Option<String>,
    pub account_name: String,
    pub location_name: String,
    #[serde(default)]
    pub catalog_name: Option<String>,
    #[serde(default)]
    pub customer_list_name: Option<String>,
}

/// What a secret prints as in a `Debug` output.
const REDACTED: &str = "<redacted>";

impl std::fmt::Debug for OAuthTokenResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthTokenResponse")
            .field("access_token", &REDACTED)
            .field("token_type", &self.token_type)
            .field("account_id", &self.account_id)
            .field("location_id", &self.location_id)
            .field("catalog_id", &self.catalog_id)
            .field("customer_list_id", &self.customer_list_id)
            .field("account_name", &self.account_name)
            .field("location_name", &self.location_name)
            .field("catalog_name", &self.catalog_name)
            .field("customer_list_name", &self.customer_list_name)
            .finish()
    }
}

/// Request body for `POST /oauth2/v1/revoke`.
#[derive(Clone, Serialize, Deserialize)]
pub struct RevokeRequest {
    pub token: String,
}

impl std::fmt::Debug for RevokeRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RevokeRequest")
            .field("token", &REDACTED)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_hides_the_tokens() {
        let revoke = RevokeRequest {
            token: "tok-very-secret".into(),
        };
        assert!(!format!("{revoke:?}").contains("tok-very-secret"));
        assert!(format!("{revoke:?}").contains("<redacted>"));
        let resp: OAuthTokenResponse = serde_json::from_str(
            r#"{"access_token":"tok-very-secret","token_type":"Bearer","account_id":"a","location_id":"l","catalog_id":null,"customer_list_id":null,"account_name":"n","location_name":"m"}"#,
        )
        .unwrap();
        assert!(!format!("{resp:?}").contains("tok-very-secret"));
        assert!(format!("{resp:?}").contains("<redacted>"));
        // The wire form is unchanged.
        assert!(
            serde_json::to_string(&revoke)
                .unwrap()
                .contains("tok-very-secret")
        );
    }
}
