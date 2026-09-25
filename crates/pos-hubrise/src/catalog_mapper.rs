//! Decoders that reverse the translation encoding we use on the
//! tsb-service side when pushing catalogs.
//!
//! See `plans/witty-sleeping-nygaard.md` - "Translation encoding strategy".

use pos_hubrise_types::{
    CategoryDescriptionPayload, CategoryTranslations as CategoryTrPayload, ProductTranslation,
    ProductTranslations,
};
use serde_json::Value;

/// Translations decoded from a SKU's `custom_fields` object.
#[derive(Debug, Clone, Default)]
pub struct DecodedSkuTranslations {
    pub vat_category: String,
    pub translations: ProductTranslations,
}

/// Translations decoded from a category's `description` field.
#[derive(Debug, Clone, Default)]
pub struct DecodedCategoryTranslations {
    pub fr: String,
    pub en: String,
    pub nl: String,
    pub zh: String,
    pub raw_description: Option<String>,
}

/// Decode a SKU's custom_fields into the VAT category + per-language
/// product translations. Falls back to empty strings if missing.
pub fn decode_sku_translations(custom_fields: &Option<Value>) -> DecodedSkuTranslations {
    let mut out = DecodedSkuTranslations {
        vat_category: "food".to_string(),
        translations: ProductTranslations::default(),
    };
    let Some(cf) = custom_fields else {
        return out;
    };
    if let Some(vat) = cf.get("vat_category").and_then(|v| v.as_str()) {
        out.vat_category = vat.to_string();
    }
    if let Some(translations) = cf.get("translations") {
        out.translations.fr = pick_product_translation(translations, "fr");
        out.translations.en = pick_product_translation(translations, "en");
        out.translations.nl = pick_product_translation(translations, "nl");
        out.translations.zh = pick_product_translation(translations, "zh");
    }
    out
}

fn pick_product_translation(translations: &Value, lang: &str) -> ProductTranslation {
    let node = translations.get(lang).cloned().unwrap_or(Value::Null);
    let name = node
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let description = node
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    ProductTranslation { name, description }
}

/// Decode a category's description into translations. If the
/// description isn't a recognized JSON payload, fall back to treating
/// the whole string as the raw description and leaving translations
/// empty.
pub fn decode_category_translations(description: &Option<String>) -> DecodedCategoryTranslations {
    let Some(desc) = description else {
        return DecodedCategoryTranslations::default();
    };
    if !CategoryDescriptionPayload::looks_like_payload(desc) {
        return DecodedCategoryTranslations {
            raw_description: Some(desc.clone()),
            ..DecodedCategoryTranslations::default()
        };
    }
    match serde_json::from_str::<CategoryDescriptionPayload>(desc) {
        Ok(payload) => {
            let CategoryTrPayload { fr, en, nl, zh } = payload.translations;
            DecodedCategoryTranslations {
                fr,
                en,
                nl,
                zh,
                raw_description: None,
            }
        }
        Err(_) => DecodedCategoryTranslations {
            raw_description: Some(desc.clone()),
            ..DecodedCategoryTranslations::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decode_sku_with_full_custom_fields() {
        let cf = Some(json!({
            "vat_category": "beverage",
            "translations": {
                "fr": { "name": "Coca-Cola 33cl", "description": "Boisson" },
                "en": { "name": "Coke 33cl" },
                "zh": { "name": "可口可乐" }
            }
        }));
        let decoded = decode_sku_translations(&cf);
        assert_eq!(decoded.vat_category, "beverage");
        assert_eq!(decoded.translations.fr.name, "Coca-Cola 33cl");
        assert_eq!(decoded.translations.en.name, "Coke 33cl");
        assert_eq!(decoded.translations.zh.name, "可口可乐");
        assert_eq!(
            decoded.translations.fr.description,
            Some("Boisson".to_string())
        );
    }

    #[test]
    fn decode_sku_with_no_custom_fields() {
        let decoded = decode_sku_translations(&None);
        assert_eq!(decoded.vat_category, "food"); // default
        assert_eq!(decoded.translations.fr.name, "");
    }

    #[test]
    fn decode_sku_with_partial_fields() {
        let cf = Some(json!({ "vat_category": "zero_rated" }));
        let decoded = decode_sku_translations(&cf);
        assert_eq!(decoded.vat_category, "zero_rated");
        assert_eq!(decoded.translations.fr.name, ""); // no translations
    }

    #[test]
    fn decode_category_with_json_description() {
        let desc = Some(
            r#"{"translations":{"fr":"Sushi","en":"Sushi","nl":"Sushi","zh":"寿司"}}"#.to_string(),
        );
        let decoded = decode_category_translations(&desc);
        assert_eq!(decoded.fr, "Sushi");
        assert_eq!(decoded.zh, "寿司");
        assert!(decoded.raw_description.is_none());
    }

    #[test]
    fn decode_category_with_plain_string() {
        let desc = Some("Just a plain description".to_string());
        let decoded = decode_category_translations(&desc);
        assert_eq!(
            decoded.raw_description,
            Some("Just a plain description".to_string())
        );
        assert_eq!(decoded.fr, ""); // empty, not the raw string
    }

    #[test]
    fn decode_category_with_none() {
        let decoded = decode_category_translations(&None);
        assert!(decoded.raw_description.is_none());
        assert_eq!(decoded.fr, "");
    }
}
