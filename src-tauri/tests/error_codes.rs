//! Every error code the backend can return has a text in the three
//! languages of the screen (`ui/src/i18n.ts`, key `err_<code>`), so a
//! refusal is never shown as a bare code or an English sentence.

use pili_pos::commands::errors;

const I18N: &str = include_str!("../../ui/src/i18n.ts");
/// The webshop texts, merged into the dictionary only with HubRise.
#[cfg(feature = "hubrise")]
const I18N_WEBSHOP: &str = include_str!("../../ui/src/i18n-webshop.ts");

/// The `{ fr: ..., en: ..., zh: ... }` entry of `key`, as text.
fn entry(key: &str) -> Option<&'static str> {
    entry_in(I18N, key)
}

fn entry_in(source: &'static str, key: &str) -> Option<&'static str> {
    let start = source.find(&format!("\"{key}\": {{"))?;
    let rest = &source[start..];
    let mut depth = 0;
    for (i, c) in rest.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&rest[..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

#[test]
fn every_error_code_has_a_text_in_every_screen_language() {
    let mut missing = vec![];
    for code in errors::ALL {
        let key = format!("err_{code}");
        match entry(&key) {
            None => missing.push(key),
            Some(text) => {
                for lang in ["fr:", "en:", "zh:"] {
                    if !text.contains(lang) {
                        missing.push(format!("{key} {lang}"));
                    }
                }
            }
        }
    }
    assert!(missing.is_empty(), "missing translations: {missing:?}");
}

#[cfg(feature = "hubrise")]
#[test]
fn every_hubrise_error_code_has_a_webshop_text_in_every_language() {
    let mut missing = vec![];
    for code in errors::hubrise::ALL {
        let key = format!("err_{code}");
        match entry_in(I18N_WEBSHOP, &key) {
            None => missing.push(key),
            Some(text) => {
                for lang in ["fr:", "en:", "zh:"] {
                    if !text.contains(lang) {
                        missing.push(format!("{key} {lang}"));
                    }
                }
            }
        }
    }
    assert!(missing.is_empty(), "missing translations: {missing:?}");
}

#[test]
fn the_fallback_texts_exist() {
    for key in ["err_failed", "err_unexpected"] {
        assert!(entry(key).is_some(), "{key}");
    }
}

#[test]
fn a_text_with_values_names_them() {
    // Values are filled by name (`{product}`), so each placeholder of a
    // coded error must be one the backend sends.
    for (code, names) in [
        (errors::FIXED_PRICE, &["product"][..]),
        (errors::PRODUCT_UNAVAILABLE, &["product"]),
        (errors::ORIGINAL_INCOMPLETE, &["field"]),
        (errors::EVENT_LABEL_UNKNOWN, &["label"]),
        (errors::NOT_REFUNDABLE, &["label"]),
        (errors::REFUND_QUANTITY_POSITIVE, &["product"]),
        (errors::REFUND_PRODUCT_NOT_SOLD, &["product"]),
        (
            errors::REFUND_EXCEEDS_SOLD,
            &["product", "requested", "sold", "refunded"],
        ),
        (errors::PRINT_FAILED, &["detail"]),
        (errors::SUPPORT_START_FAILED, &["detail"]),
        (errors::INTERNAL_ERROR, &["detail"]),
    ] {
        let text = entry(&format!("err_{code}")).unwrap();
        for name in names {
            assert!(text.contains(&format!("{{{name}}}")), "{code}: {{{name}}}");
        }
    }
}
