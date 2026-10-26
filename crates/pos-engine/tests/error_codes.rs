//! Every error code the backend can return has a text in the three
//! languages of the screen (`ui/src/i18n.ts`, key `err_<code>`), so a
//! refusal is never shown as a bare code or an English sentence.

use pos_engine::commands::errors;

const I18N: &str = include_str!("../../../ui/src/i18n.ts");
/// The webshop texts, merged into the dictionary only with `HubRise`.
#[cfg(feature = "hubrise")]
const I18N_WEBSHOP: &str = include_str!("../../../ui/src/i18n-webshop.ts");

/// The `{ fr: ..., en: ..., zh: ... }` entry of `key`, as text.
fn entry(key: &str) -> Option<&'static str> {
    entry_in(I18N, key)
}

fn entry_in(source: &'static str, key: &str) -> Option<&'static str> {
    let start = key_at(source, key)?;
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

/// Where the entry of `key` starts: `key: {` as oxfmt writes it (quotes only
/// when needed), or `"key": {`. A bare key must not be the tail of a longer one.
fn key_at(source: &str, key: &str) -> Option<usize> {
    if let Some(at) = source.find(&format!("\"{key}\": {{")) {
        return Some(at);
    }
    let bare = format!("{key}: {{");
    source.match_indices(&bare).map(|(at, _)| at).find(|&at| {
        source[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'))
    })
}

#[test]
fn entries_are_found_with_bare_or_quoted_keys() {
    let source =
        "const m = {\n  err_a: { fr: 'a' },\n  \"err_b\": { fr: 'b' },\n  xerr_c: { fr: 'x' },\n}";
    assert_eq!(entry_in(source, "err_a"), Some("err_a: { fr: 'a' }"));
    assert_eq!(entry_in(source, "err_b"), Some("\"err_b\": { fr: 'b' }"));
    assert_eq!(entry_in(source, "err_c"), None);
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
        (errors::GIFT_VOUCHER_AMOUNT_INVALID, &["max"]),
        (errors::GIFT_VOUCHER_LOOKUP_THROTTLED, &["seconds"]),
        (errors::GIFT_VOUCHER_SETTINGS_INVALID, &["field"]),
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

#[test]
fn the_nine_badge_codes_are_listed_and_translated() {
    for code in [
        errors::BADGE_READER_OFF,
        errors::BADGE_TOKEN_INVALID,
        errors::BADGE_KEY_UNAVAILABLE,
        errors::BADGE_UNKNOWN,
        errors::BADGE_REVOKED,
        errors::BADGE_IN_USE,
        errors::BADGE_ALREADY_LINKED,
        errors::BADGE_NONE,
        errors::BADGE_SETTINGS_INVALID,
    ] {
        assert!(errors::ALL.contains(&code), "{code} is not in ALL");
        let text = entry(&format!("err_{code}")).unwrap_or_else(|| panic!("err_{code}"));
        for lang in ["fr:", "en:", "zh:"] {
            assert!(text.contains(lang), "err_{code} {lang}");
        }
    }
}

#[test]
fn the_invoice_and_peppol_codes_are_listed_and_translated() {
    for code in [
        errors::INVOICE_NO_SALES,
        errors::INVOICE_SALE_NOT_FOUND,
        errors::INVOICE_NOT_A_SALE,
        errors::INVOICE_SALE_ALREADY_INVOICED,
        errors::INVOICE_TRAINING_MODE,
        errors::INVOICE_GIFT_VOUCHER_SALE,
        errors::INVOICE_PEPPOL_REQUIRED,
        errors::CUSTOMER_VAT_INVALID,
        errors::CUSTOMER_INCOMPLETE,
        errors::CUSTOMER_NOT_FOUND,
        errors::PEPPOL_NOT_CONFIGURED,
        errors::PEPPOL_DELIVERY_NOT_FOUND,
        errors::PEPPOL_DELIVERY_NOT_PARKED,
        errors::CONFIG_PEPPOL,
    ] {
        assert!(errors::ALL.contains(&code), "{code} is not in errors::ALL");
        let text = entry(&format!("err_{code}")).unwrap_or_else(|| panic!("no text for {code}"));
        for lang in ["fr:", "en:", "zh:"] {
            assert!(text.contains(lang), "{code} {lang}");
        }
    }
}

#[test]
fn the_sold_out_code_is_listed_and_translated() {
    let code = errors::PRODUCT_SOLD_OUT;
    assert!(errors::ALL.contains(&code), "{code} is not in errors::ALL");
    let text = entry(&format!("err_{code}")).unwrap_or_else(|| panic!("no text for {code}"));
    for lang in ["fr:", "en:", "zh:"] {
        assert!(text.contains(lang), "{code} {lang}");
    }
    // The code carries the product id for the screen.
    assert_eq!(
        errors::coded(code, &[("product", &7)]),
        "product_sold_out|product=7"
    );
}

#[test]
fn the_menu_editor_codes_are_listed_translated_and_carry_their_values() {
    for (code, placeholder) in [
        (errors::MENU_INVALID, Some("field")),
        (errors::MENU_NOT_FOUND, None),
    ] {
        assert!(errors::ALL.contains(&code), "{code} is not in errors::ALL");
        let text = entry(&format!("err_{code}")).unwrap_or_else(|| panic!("no text for {code}"));
        for lang in ["fr:", "en:", "zh:"] {
            assert!(text.contains(lang), "{code} {lang}");
        }
        if let Some(name) = placeholder {
            assert!(text.contains(&format!("{{{name}}}")), "{code}: {{{name}}}");
        }
    }
    assert_eq!(
        errors::coded(errors::MENU_INVALID, &[("field", &"price_cents")]),
        "menu_invalid|field=price_cents"
    );
}

#[cfg(feature = "hubrise")]
#[test]
fn the_locked_menu_code_has_its_text() {
    let text = entry_in(
        I18N_WEBSHOP,
        &format!("err_{}", errors::hubrise::MENU_MANAGED),
    )
    .unwrap();
    for lang in ["fr:", "en:", "zh:"] {
        assert!(text.contains(lang), "{lang}");
    }
}

#[test]
fn the_sales_report_code_is_listed_and_translated() {
    let code = errors::SALES_REPORT_RANGE_INVALID;
    assert!(errors::ALL.contains(&code), "{code} is not in errors::ALL");
    let text = entry(&format!("err_{code}")).unwrap_or_else(|| panic!("no text for {code}"));
    for lang in ["fr:", "en:", "zh:"] {
        assert!(text.contains(lang), "{code} {lang}");
    }
}
