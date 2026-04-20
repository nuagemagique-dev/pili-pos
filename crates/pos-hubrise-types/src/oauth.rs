//! OAuth 2.0 flow types for HubRise manager.

use serde::{Deserialize, Serialize};

/// Parameters appended to the HubRise authorize URL.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Request body for `POST /oauth2/v1/revoke`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeRequest {
    pub token: String,
}
