//! Records of the allergens (engine module `allergens`, v1.2): the legend, the allergen sheet and
//! the local edit. Mirrors the engine types (PD56): same field names, same serde form,
//! `deny_unknown_fields`.

/// `pos_engine::commands::allergens::AllergenNames`: an allergen's name in each screen language.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AllergenNames {
    pub fr: String,
    pub nl: String,
    pub en: String,
    pub zh: String,
}

/// `pos_engine::commands::allergens::AllergenEntry`: one of the 14 allergens of Annex II.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AllergenEntry {
    pub code: String,
    /// The Annex II number, 1 to 14.
    pub number: i32,
    pub names: AllergenNames,
}

/// `pos_engine::commands::allergens::AllergenRef`: an allergen on a sheet, named in its language.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AllergenRef {
    pub code: String,
    pub number: i32,
    pub name: String,
}

/// `pos_engine::commands::allergens::SheetOptionView`: an option on a product's sheet.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct SheetOptionView {
    pub group: String,
    pub name: String,
    pub declared: bool,
    pub contains: Vec<AllergenRef>,
    pub traces: Vec<AllergenRef>,
}

/// `pos_engine::commands::allergens::SheetProductView`: a product on the sheet.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct SheetProductView {
    pub id: i64,
    pub name: String,
    pub name_fr: String,
    /// False: no information (never "no allergen").
    pub declared: bool,
    pub contains: Vec<AllergenRef>,
    pub traces: Vec<AllergenRef>,
    pub options: Vec<SheetOptionView>,
}

/// `pos_engine::commands::allergens::AllergenSheetView`: the sheet of one product or of the menu.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AllergenSheetView {
    pub lang: String,
    pub products: Vec<SheetProductView>,
}

/// `pos_engine::commands::allergens::SetAllergensInput`: the edit of one list.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct SetAllergensInput {
    /// `menu_item` or `modifier`.
    pub entity_type: String,
    pub entity_id: i64,
    pub declared: bool,
    #[serde(default)]
    #[uniffi(default = [])]
    pub contains: Vec<String>,
    #[serde(default)]
    #[uniffi(default = [])]
    pub may_contain: Vec<String>,
}
