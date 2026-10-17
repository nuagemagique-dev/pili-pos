use serde::{Deserialize, Serialize};

use crate::vat::VatCode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    /// French and Chinese names, whatever the UI language: product
    /// buttons and cart lines show both, the UI language first.
    #[serde(default)]
    pub name_fr: Option<String>,
    #[serde(default)]
    pub name_zh: Option<String>,
    pub sort_order: i32,
    pub color: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub id: i64,
    pub category_id: i64,
    pub name: String,
    /// French and Chinese names, whatever the UI language: product
    /// buttons and cart lines show both, the UI language first.
    #[serde(default)]
    pub name_fr: Option<String>,
    #[serde(default)]
    pub name_zh: Option<String>,
    /// The menu code ("A2") from a mirrored catalog; display only.
    #[serde(default)]
    pub code: Option<String>,
    pub price_cents: i64,
    pub vat_code: VatCode,
    /// VAT label for takeaway sales (manual §III product lines, column
    /// "À emporter"). `None` means the dine-in label applies.
    #[serde(default)]
    pub vat_code_takeaway: Option<VatCode>,
    /// Free-price product: the operator enters the unit price.
    #[serde(default)]
    pub open_price: bool,
    pub kitchen_printer: bool,
    pub sort_order: i32,
    pub active: bool,
    /// Allergen codes the product contains (Annex II codes); empty when
    /// none or when the list is not declared (see `allergens_declared`).
    #[serde(default)]
    pub allergens: Vec<String>,
    /// Allergen codes the product may contain as traces.
    #[serde(default)]
    pub traces: Vec<String>,
    /// The allergen list is complete (possibly empty). False: no
    /// information, the till says so instead of "no allergen".
    #[serde(default)]
    pub allergens_declared: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifierGroup {
    pub id: i64,
    pub menu_item_id: i64,
    pub name: String,
    pub min_select: i32,
    pub max_select: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Modifier {
    pub id: i64,
    pub group_id: i64,
    pub name: String,
    pub price_cents: i64,
    pub sort_order: i32,
}
