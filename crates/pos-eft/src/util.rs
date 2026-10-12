//! Small helpers the three drivers share: amounts read from the
//! provider's answer, language codes, URL path segments.

use serde_json::Value;

/// A decimal amount as text ("12.50", "12,5", "104.11", "7") in euro
/// cents. `None` when the text is not a plain decimal or carries more
/// than two significant decimals. Never goes through a float.
pub(crate) fn cents_from_text(text: &str) -> Option<i64> {
    let t = text.trim();
    let (negative, digits) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let (whole, frac) = match digits.split_once(['.', ',']) {
        Some((w, f)) => (w, f),
        None => (digits, ""),
    };
    if whole.is_empty() && frac.is_empty() {
        return None;
    }
    if !whole.bytes().all(|b| b.is_ascii_digit()) || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // More than two decimals is only fine when the extra ones are zeros.
    if frac.len() > 2 && !frac[2..].bytes().all(|b| b == b'0') {
        return None;
    }
    let whole_cents = if whole.is_empty() {
        0
    } else {
        whole.parse::<i64>().ok()?.checked_mul(100)?
    };
    let mut f = frac.chars().take(2).collect::<String>();
    while f.len() < 2 {
        f.push('0');
    }
    let cents = whole_cents.checked_add(f.parse::<i64>().ok()?)?;
    Some(if negative { -cents } else { cents })
}

/// A JSON amount (number or numeric text) in euro cents. The number is
/// read through its shortest decimal text ("104.11"), so the usual
/// binary float error ("10410.999999") never shows up; only an exponent
/// form falls back to a rounded multiplication.
pub(crate) fn cents_from_json(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => {
            let text = n.to_string();
            if text.contains(['e', 'E']) {
                let f = n.as_f64()?;
                if !f.is_finite() || f.abs() > 1.0e12 {
                    return None;
                }
                // Rounding to the nearest cent is the only sane reading.
                #[allow(clippy::cast_possible_truncation)]
                return Some((f * 100.0).round() as i64);
            }
            cents_from_text(&text)
        }
        Value::String(s) => cents_from_text(s),
        _ => None,
    }
}

/// The value at `path` (object keys) in `v`.
pub(crate) fn at<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(v, |cur, key| cur.get(*key))
}

/// The non-empty text at `path`.
pub(crate) fn str_at<'a>(v: &'a Value, path: &[&str]) -> Option<&'a str> {
    at(v, path)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// The language as ISO 639-1 upper case (`FR`), the nexo spelling.
pub(crate) fn language_upper2(language: &str) -> &'static str {
    match language.trim().to_ascii_lowercase().as_str() {
        "nl" => "NL",
        "de" => "DE",
        "en" => "EN",
        _ => "FR",
    }
}

/// The language as ISO 639-2/T (`fra`), the CCV spelling.
pub(crate) fn language_iso3(language: &str) -> &'static str {
    match language.trim().to_ascii_lowercase().as_str() {
        "nl" => "nld",
        "de" => "deu",
        "en" => "eng",
        _ => "fra",
    }
}

/// `value` as one URL path segment (unreserved characters kept, the
/// rest percent-encoded).
pub(crate) fn path_segment(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// At most `max` characters of `text`.
pub(crate) fn truncate(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// The JSON `body` with `AMOUNT_MARK` written as the number
/// `amount_text(cents)`: the amount goes out as the exact decimal text,
/// never as a float.
pub(crate) const AMOUNT_MARK: &str = "@@AMOUNT@@";

pub(crate) fn with_amount(body: &Value, cents: i64) -> String {
    body.to_string()
        .replace(&format!("\"{AMOUNT_MARK}\""), &crate::amount_text(cents))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decimal_text_is_read_exactly() {
        assert_eq!(cents_from_text("12.50"), Some(1250));
        assert_eq!(cents_from_text("12,5"), Some(1250));
        assert_eq!(cents_from_text("1000,00"), Some(100_000));
        assert_eq!(cents_from_text("104.11"), Some(10_411));
        assert_eq!(cents_from_text("7"), Some(700));
        assert_eq!(cents_from_text("0.05"), Some(5));
        assert_eq!(cents_from_text(".5"), Some(50));
        assert_eq!(cents_from_text("-3.00"), Some(-300));
        assert_eq!(cents_from_text("12.500"), Some(1250));
        assert_eq!(cents_from_text("12.505"), None);
        assert_eq!(cents_from_text("abc"), None);
        assert_eq!(cents_from_text(""), None);
        assert_eq!(cents_from_text("1.2.3"), None);
    }

    #[test]
    fn json_numbers_do_not_pick_up_float_error() {
        // 104.11 * 100.0 is 10410.999999999998 in binary floating point.
        assert_eq!(cents_from_json(&json!(104.11)), Some(10_411));
        assert_eq!(cents_from_json(&json!(100)), Some(10_000));
        assert_eq!(cents_from_json(&json!(0.3)), Some(30));
        assert_eq!(cents_from_json(&json!("12.50")), Some(1250));
        // A fraction of a cent is not an amount.
        assert_eq!(cents_from_json(&json!(1e-5)), None);
        assert_eq!(cents_from_json(&json!(1.5e2)), Some(15_000));
        assert_eq!(cents_from_json(&json!(null)), None);
    }

    #[test]
    fn languages_map_to_the_protocol_codes() {
        assert_eq!(language_upper2("nl"), "NL");
        assert_eq!(language_upper2("zh"), "FR");
        assert_eq!(language_iso3("de"), "deu");
        assert_eq!(language_iso3("EN"), "eng");
        assert_eq!(language_iso3("fr"), "fra");
    }

    #[test]
    fn amount_is_written_from_cents() {
        let body = json!({"a": AMOUNT_MARK, "b": "x"});
        assert_eq!(with_amount(&body, 1250), r#"{"a":12.50,"b":"x"}"#);
        assert_eq!(path_segment("a b/c"), "a%20b%2Fc");
        assert_eq!(truncate("abcdef", 3), "abc");
    }
}
