//! The neutral catalog: what the POS menu is made of, whatever the
//! source (`HubRise`, the owner's catalog service). A source maps its
//! own format into this model; validation and apply work on it only.

use std::collections::BTreeMap;

use pos_core::allergen::AllergenSet;
use pos_core::vat::VatCode;
use serde::{Deserialize, Serialize};

/// Names per language code (`"fr"`, `"zh"`, ...). Sorted, so the
/// catalog hash does not depend on insertion order.
pub type Names = BTreeMap<String, String>;

/// A whole catalog, in display order.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    pub categories: Vec<Category>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Category {
    /// The source's stable key (`categories.external_id`).
    pub key: String,
    pub names: Names,
    /// Shown as a tab on the POS. Charges live in a category that is
    /// not.
    pub visible: bool,
}

/// What an item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// Something sold: a product (one size of it).
    Product,
    /// A fee of an online order (delivery, service): never on the
    /// POS grid, only on the lines of the orders that charge it.
    Charge,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the catalog model mirrors the source's independent flags, and it is part of the hashed JSON"
)]
pub struct Item {
    /// The source's stable key (`menu_items.external_id`): the same
    /// key keeps the same POS product id, the `productId` sent to the
    /// FDM.
    pub key: String,
    pub category_key: String,
    pub kind: ItemKind,
    pub names: Names,
    /// The menu code staff know the product by ("A2", "B14"), shown in
    /// place of the name when the till is set to. Display only: never
    /// printed nor sent to the FDM. Left out of the JSON when absent,
    /// so a catalog without codes keeps its hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub price_cents: i64,
    /// VAT label eaten in, and taken away or delivered.
    pub vat_eat_in: VatCode,
    pub vat_takeaway: VatCode,
    /// The operator enters the price.
    pub open_price: bool,
    /// Weighed at the till: `price_cents` is the price per kilo. Left out
    /// of the hash when false, so a catalog without it keeps its hash.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sold_by_weight: bool,
    /// Printed on the bar / kitchen slip.
    pub kitchen: bool,
    /// On the POS grid (a product can be sold online only).
    pub on_pos: bool,
    /// For sale now (a source can take an item off sale).
    pub available: bool,
    /// Option groups, in display order.
    pub groups: Vec<Group>,
    /// The 14 EU allergens: `None` is "no information", `Some` a
    /// complete list (possibly empty). Left out of the JSON when
    /// absent, so a catalog without allergens keeps its hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allergens: Option<AllergenSet>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    /// Unique across the catalog: the same option list attached to two
    /// items makes two groups.
    pub key: String,
    pub names: Names,
    pub min: u32,
    /// `None`: no maximum.
    pub max: Option<u32>,
    pub options: Vec<Choice>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    /// Unique across the catalog.
    pub key: String,
    pub names: Names,
    pub price_cents: i64,
    pub available: bool,
    /// As [`Item::allergens`], for the option.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allergens: Option<AllergenSet>,
}

/// The name printed and sent to the FDM: French, then English, then
/// any.
pub fn fiscal_name(names: &Names) -> Option<&str> {
    names
        .get("fr")
        .or_else(|| names.get("en"))
        .or_else(|| names.values().next())
        .map(String::as_str)
        .filter(|n| !n.trim().is_empty())
}
