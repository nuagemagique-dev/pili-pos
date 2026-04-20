//! HubRise Catalog resource.
//!
//! Reference: `docs/hubrise/04-catalogs.md`.

use serde::{Deserialize, Serialize};

use crate::common::Money;

/// Catalog summary returned by list endpoints (without `data`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogSummary {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

/// Full catalog, with optional `data` (omitted when `hide_data=true`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub id: String,
    #[serde(default)]
    pub location_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub data: Option<CatalogData>,
}

/// Catalog payload used on CREATE/PUT.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CatalogData {
    #[serde(default)]
    pub variants: Vec<Variant>,
    #[serde(default)]
    pub categories: Vec<Category>,
    #[serde(default)]
    pub products: Vec<Product>,
    #[serde(default, alias = "options_lists")]
    pub option_lists: Vec<OptionList>,
    #[serde(default)]
    pub deals: Vec<Deal>,
    #[serde(default)]
    pub discounts: Vec<Discount>,
    #[serde(default)]
    pub charges: Vec<Charge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    pub ref_: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "ref")]
    pub ref_field: String,
    #[serde(default)]
    pub parent_ref: Option<String>,
    pub name: String,
    /// HubRise doesn't have `custom_fields` on categories, so we repurpose
    /// `description` as a JSON blob carrying translations. See
    /// `CategoryDescriptionPayload` in `custom_fields` module.
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub image_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, rename = "ref")]
    pub ref_field: Option<String>,
    pub category_ref: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub image_ids: Vec<String>,
    pub skus: Vec<Sku>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sku {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, rename = "ref")]
    pub ref_field: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    pub price: Money,
    #[serde(default)]
    pub option_list_refs: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub barcodes: Vec<String>,
    /// HubRise-documented custom_fields slot. We store `vat_category`
    /// and multi-language translations here. See `SkuCustomFields`.
    #[serde(default)]
    pub custom_fields: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionList {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "ref")]
    pub ref_field: String,
    pub name: String,
    #[serde(default)]
    pub min_selections: Option<u32>,
    #[serde(default)]
    pub max_selections: Option<u32>,
    #[serde(default)]
    pub multiple_selection: Option<bool>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub options: Vec<SkuOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuOption {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, rename = "ref")]
    pub ref_field: Option<String>,
    pub name: String,
    #[serde(default)]
    pub price: Option<Money>,
    #[serde(default)]
    pub default: Option<bool>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deal {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, rename = "ref")]
    pub ref_field: Option<String>,
    #[serde(default)]
    pub category_ref: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub coupon_codes: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub lines: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discount {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, rename = "ref")]
    pub ref_field: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub pricing_effect: String,
    #[serde(default)]
    pub pricing_value: Option<String>,
    #[serde(default)]
    pub coupon_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Charge {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, rename = "ref")]
    pub ref_field: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub charge_type: String,
    #[serde(default)]
    pub price: Option<Money>,
}
