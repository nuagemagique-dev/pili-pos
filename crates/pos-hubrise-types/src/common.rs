//! Common types shared across HubRise resources.

use serde::{Deserialize, Serialize};

/// Monetary value as a string in HubRise format (e.g. "12.50 EUR").
/// We keep it as an opaque string at the wire level — parsing to
/// cents happens in the client/mock.
pub type Money = String;

/// Language code — HubRise has no native i18n, but we still carry
/// the `Language` enum here for the translation extension payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Fr,
    En,
    Nl,
    Zh,
}

impl Language {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fr => "fr",
            Self::En => "en",
            Self::Nl => "nl",
            Self::Zh => "zh",
        }
    }

    pub fn all() -> [Language; 4] {
        [Self::Fr, Self::En, Self::Nl, Self::Zh]
    }
}

/// HubRise order service type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceType {
    Delivery,
    Collection,
    EatIn,
}
