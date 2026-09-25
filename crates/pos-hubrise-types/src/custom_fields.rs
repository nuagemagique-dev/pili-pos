//! Translation encoding extension types.
//!
//! HubRise has no native i18n. We encode translations via two
//! documented extension slots:
//!
//! 1. **SKU `custom_fields`** - carries `vat_category` + product
//!    translations (name, description) for all 4 TSB languages.
//! 2. **Category `description`** - repurposed as a JSON blob with
//!    the shape `{"translations":{"fr":"..","en":"..","nl":"..","zh":".."}}`.
//!
//! Options and option_lists don't have custom_fields - their labels
//! stay French-only (MVP limitation, see `plans/witty-sleeping-nygaard.md`).

use serde::{Deserialize, Serialize};

/// Belgian VAT category carried in SKU `custom_fields.vat_category`.
///
/// The POS derives the SCE 2.0 code (A/B/C/D/X) at checkout time
/// based on the order's service type (dine-in vs takeaway/delivery).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VatCategory {
    /// 12% dine-in (SCE `B`), 6% takeaway/delivery (SCE `C`).
    Food,
    /// 21% all service types (SCE `A`).
    Beverage,
    /// 0% (SCE `D`).
    ZeroRated,
    /// Out of SCE scope (SCE `X`).
    OutOfScope,
}

impl VatCategory {
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Food => "food",
            Self::Beverage => "beverage",
            Self::ZeroRated => "zero_rated",
            Self::OutOfScope => "out_of_scope",
        }
    }
}

/// Contents of `sku.custom_fields` for products pushed by tsb-service.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkuCustomFields {
    pub vat_category: Option<VatCategory>,
    #[serde(default)]
    pub translations: ProductTranslations,
}

/// Product translations (name + optional description) per language.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProductTranslations {
    #[serde(default)]
    pub fr: ProductTranslation,
    #[serde(default)]
    pub en: ProductTranslation,
    #[serde(default)]
    pub nl: ProductTranslation,
    #[serde(default)]
    pub zh: ProductTranslation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProductTranslation {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// JSON payload stored inside `Category.description`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategoryDescriptionPayload {
    pub translations: CategoryTranslations,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategoryTranslations {
    #[serde(default)]
    pub fr: String,
    #[serde(default)]
    pub en: String,
    #[serde(default)]
    pub nl: String,
    #[serde(default)]
    pub zh: String,
}

impl CategoryDescriptionPayload {
    /// Returns true if the given description string looks like a
    /// translation-encoded JSON blob (starts with `{`).
    pub fn looks_like_payload(description: &str) -> bool {
        description.trim_start().starts_with('{')
    }
}
