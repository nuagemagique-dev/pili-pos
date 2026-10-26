//! Common types shared across `HubRise` resources.

use serde::{Deserialize, Serialize};

/// Monetary value as a string in `HubRise` format (e.g. "12.50 EUR").
/// We keep it as an opaque string at the wire level - parsing to
/// cents happens in the client/mock.
pub type Money = String;

/// Language code - `HubRise` has no native i18n, but we still carry
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

/// `HubRise` order service type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceType {
    Delivery,
    Collection,
    EatIn,
}

/// `#[serde(default, deserialize_with = "null_default")]`: `HubRise`
/// sends `null` for many empty fields (`"deleted": null`,
/// `"coupon_codes": null`), which plain `#[serde(default)]` refuses.
/// Missing and `null` both give the type's default.
pub fn null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// Tax rates of a catalog product per service type, as decimal
/// strings (`"12.0"`), `None` when the catalog leaves one unset.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxRate {
    #[serde(default)]
    pub delivery: Option<String>,
    #[serde(default)]
    pub collection: Option<String>,
    #[serde(default)]
    pub eat_in: Option<String>,
}

/// Availability conditions of a SKU, option, deal, discount or charge
/// (`docs/hubrise/04-catalogs.md` §11). Unknown keys are kept.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Restrictions {
    /// `false` takes the element off sale; missing means enabled.
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(flatten)]
    pub other: serde_json::Map<String, serde_json::Value>,
}

impl Restrictions {
    /// Whether the element is on sale: only an explicit
    /// `enabled: false` disables it.
    pub fn is_enabled(restrictions: &Option<Restrictions>) -> bool {
        restrictions
            .as_ref()
            .and_then(|r| r.enabled)
            .unwrap_or(true)
    }
}

/// One price rule of a SKU or option (`docs/hubrise/04-catalogs.md` §12).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceOverride {
    pub price: Money,
    #[serde(flatten)]
    pub conditions: serde_json::Map<String, serde_json::Value>,
}
