//! The menu editor records (V11-D17): the FFI mirror of `pos_engine::commands::menu_admin`
//! and of the rows of `pos_db::queries::menu_admin`. Names per language are a map of language
//! code to name (the engine's ordered map is a `HashMap` here: `UniFFI` has no ordered map; the
//! serde form is the same JSON object).

use std::collections::HashMap;

/// `pos_db::queries::menu_admin::AdminCategory`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AdminCategory {
    pub id: i64,
    pub names: HashMap<String, String>,
    pub sort_order: i32,
    pub color: Option<String>,
    pub active: bool,
}

/// `pos_db::queries::menu_admin::AdminProduct`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AdminProduct {
    pub id: i64,
    pub category_id: i64,
    pub names: HashMap<String, String>,
    pub price_cents: i64,
    pub vat_code: String,
    pub vat_code_takeaway: Option<String>,
    pub kitchen_printer: bool,
    pub code: Option<String>,
    pub open_price: bool,
    pub sort_order: i32,
    pub active: bool,
}

/// `pos_db::queries::menu_admin::AdminGroup`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AdminGroup {
    pub id: i64,
    pub menu_item_id: i64,
    pub names: HashMap<String, String>,
    pub min_select: i32,
    pub max_select: i32,
    pub sort_order: i32,
    pub active: bool,
}

/// `pos_db::queries::menu_admin::AdminOption`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AdminOption {
    pub id: i64,
    pub group_id: i64,
    pub names: HashMap<String, String>,
    pub price_cents: i64,
    pub sort_order: i32,
    pub active: bool,
}

/// `pos_engine::commands::menu_admin::MenuAdmin`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct MenuAdmin {
    pub managed_by: Option<String>,
    pub languages: Vec<String>,
    pub categories: Vec<AdminCategory>,
    pub products: Vec<AdminProduct>,
    pub groups: Vec<AdminGroup>,
    pub options: Vec<AdminOption>,
    pub allergens: Vec<AdminAllergens>,
}

/// `pos_engine::commands::menu_admin::AdminAllergens`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AdminAllergens {
    pub entity_type: String,
    pub entity_id: i64,
    pub contains: Vec<String>,
    pub may_contain: Vec<String>,
}

/// `pos_engine::commands::menu_admin::MenuSaved`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct MenuSaved {
    pub id: i64,
    pub menu: MenuAdmin,
}

/// `pos_engine::commands::menu_admin::MenuChange`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct MenuChange {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub action: String,
    pub label: String,
    pub changed_fields: Vec<String>,
    pub before_json: Option<String>,
    pub after_json: String,
    pub changed_by: String,
    pub changed_by_name: String,
    pub changed_at: String,
}

/// `pos_engine::commands::menu_admin::SaveCategoryInput`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct SaveCategoryInput {
    pub id: Option<i64>,
    pub names: HashMap<String, String>,
    pub sort_order: i32,
    pub color: Option<String>,
    pub active: bool,
}

/// `pos_engine::commands::menu_admin::SaveProductInput`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct SaveProductInput {
    pub id: Option<i64>,
    pub category_id: i64,
    pub names: HashMap<String, String>,
    pub price_cents: i64,
    pub vat_code: String,
    pub vat_code_takeaway: Option<String>,
    pub kitchen_printer: bool,
    pub code: Option<String>,
    pub open_price: bool,
    pub sort_order: i32,
    pub active: bool,
}

/// `pos_engine::commands::menu_admin::SaveModifierGroupInput`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct SaveModifierGroupInput {
    pub id: Option<i64>,
    pub menu_item_id: i64,
    pub names: HashMap<String, String>,
    pub min_select: i32,
    pub max_select: i32,
    pub sort_order: i32,
    pub active: bool,
}

/// `pos_engine::commands::menu_admin::SaveModifierInput`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct SaveModifierInput {
    pub id: Option<i64>,
    pub group_id: i64,
    pub names: HashMap<String, String>,
    pub price_cents: i64,
    pub sort_order: i32,
    pub active: bool,
}
