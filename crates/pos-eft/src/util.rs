//! Small helpers the three drivers share: amounts read from the
//! provider's answer, language codes, URL path segments.

use std::fmt::Write as _;

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
                let scaled = f * 100.0;
                let rounded = scaled.round();
                // A fraction of a cent is not an amount.
                if (scaled - rounded).abs() > 1.0e-6 {
                    return None;
                }
                // |rounded| <= 1e14, far inside the i64 range.
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::as_conversions,
                    reason = "bounded above by the 1e12 check, an exact whole number"
                )]
                return Some(rounded as i64);
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
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// At most `max` characters of `text`.
pub(crate) fn truncate(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// Whether the wire trace is on (target `pos_eft::wire`, TRACE level:
/// every frame or body exchanged with a terminal or a provider, card
/// numbers masked), to skip building its strings otherwise.
pub(crate) fn wire_enabled() -> bool {
    tracing::enabled!(target: "pos_eft::wire", tracing::Level::TRACE)
}

/// Whether `run` is digit groups in card grouping: every group but the
/// last has exactly 4 digits, the last 1 to 4 ("4111-1111-1111-1111").
/// Only the shape counts, not the digits. A UUID fragment usually has a
/// first group of another length or a 12 digit last group; the rare one
/// that matches (about 1 in 10,000) is over-masked, which only costs a
/// log line, never leaks a card number.
fn card_grouped(run: &[char]) -> bool {
    let groups: Vec<&[char]> = run.split(|c| *c == ' ' || *c == '-').collect();
    let Some((last, head)) = groups.split_last() else {
        return false;
    };
    !head.is_empty() && head.iter().all(|g| g.len() == 4) && (1..=4).contains(&last.len())
}

/// `text` with every run of 13 to 19 digits (single spaces between
/// digits allowed, or dashes: "4111 1111 1111 1111", "4111-1111-1111-1111") replaced by asterisks except
/// its last 4 digits. A run of other lengths is left alone, unless it is
/// made of digit groups of which one has 13 to 19 digits on its own.
pub fn mask_pan(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        // The run: digits, with a single space allowed between two digits.
        let mut end = i;
        while end < chars.len() {
            let c = chars[end];
            let joined_by_space = (c == ' ' || c == '-')
                && end > i
                && chars.get(end + 1).is_some_and(char::is_ascii_digit);
            if c.is_ascii_digit() || joined_by_space {
                end += 1;
            } else {
                break;
            }
        }
        let run = &chars[i..end];
        let digits = run.iter().filter(|c| c.is_ascii_digit()).count();
        // A dash-joined run is a card number only when it stands alone: a
        // run touching letters is a piece of an identifier (a UUID whose
        // hex groups happen to be digits), never a PAN. Card grouping
        // (4-4-4-1..4) is a card number whatever touches it.
        let alone = !run.contains(&'-')
            || card_grouped(run)
            || (i
                .checked_sub(1)
                .and_then(|b| chars.get(b))
                .is_none_or(|c| !c.is_ascii_alphanumeric())
                && chars.get(end).is_none_or(|c| !c.is_ascii_alphanumeric()));
        if alone && (13..=19).contains(&digits) {
            mask_run(run, &mut out);
        } else {
            // Not a card number as a whole: still mask a single long group.
            for (n, group) in run.split(|c| *c == ' ' || *c == '-').enumerate() {
                if n > 0 {
                    // Keep the separator that was there.
                    let at = i + group_start(run, n);
                    out.push(chars[at]);
                }
                if (13..=19).contains(&group.len()) {
                    mask_run(group, &mut out);
                } else {
                    out.extend(group);
                }
            }
        }
        i = end;
    }
    out
}

/// Index in `run` of the separator before its `n`th group.
fn group_start(run: &[char], n: usize) -> usize {
    run.iter()
        .enumerate()
        .filter(|(_, c)| **c == ' ' || **c == '-')
        .nth(n - 1)
        .map_or(0, |(idx, _)| idx)
}

fn mask_run(run: &[char], out: &mut String) {
    let digits = run.iter().filter(|c| c.is_ascii_digit()).count();
    let mut seen = 0;
    for &c in run {
        if c.is_ascii_digit() {
            seen += 1;
            out.push(if seen > digits - 4 { c } else { '*' });
        } else {
            out.push(c);
        }
    }
}

/// One wire trace event: `direction` is `send` or `recv`, `text` is
/// masked here.
pub(crate) fn trace_wire(direction: &str, text: &str) {
    tracing::trace!(target: "pos_eft::wire", direction, "{}", mask_pan(text));
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
        assert_eq!(cents_from_json(&json!(1.2345e1)), None);
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
    fn long_digit_runs_are_masked_but_the_last_four() {
        assert_eq!(mask_pan("4111111111111111"), "************1111");
        assert_eq!(
            mask_pan("PAN 4111 1111 1111 1111 end"),
            "PAN **** **** **** 1111 end"
        );
        assert_eq!(mask_pan("a5555555555554444b"), "a************4444b");
        assert_eq!(mask_pan("1234567890123"), "*********0123");
        assert_eq!(
            mask_pan("PAN 4111-1111-1111-1111 end"),
            "PAN ****-****-****-1111 end"
        );
        assert_eq!(mask_pan("order 2024-10-26"), "order 2024-10-26");
        // A card grouping glued to a letter or digit is still one number.
        assert_eq!(mask_pan("PAN4111-1111-1111-1111"), "PAN****-****-****-1111");
        assert_eq!(mask_pan("4111-1111-1111-1111x"), "****-****-****-1111x");
        // UUIDs whose hex groups happen to be digits stay as they are.
        for uuid in [
            "a1b2c3d4-e5f6-4123-8123-1234a6789012",
            "6f1f0a52-0000-4000-8000-00000000000b",
            "ref c0ffee12-3456-4789-8123-45678901",
        ] {
            assert_eq!(mask_pan(uuid), uuid);
        }
        // The rare UUID in card grouping is over-masked, never the reverse.
        assert_eq!(
            mask_pan("abcd1234-5678-4012-8456-abcdef012345"),
            "abcd****-****-****-8456-abcdef012345"
        );
        assert_eq!(
            mask_pan("<P>6703 0000 0000 0119 0</P>"),
            "<P>**** **** **** *119 0</P>"
        );
    }

    #[test]
    fn short_numbers_and_other_text_are_left_alone() {
        let text = r#"RequestID="123456789" Amount="12.50" at 2026-10-13T10:11:12 code 1234 5678"#;
        assert_eq!(mask_pan(text), text);
        // 12 and 20 digits are not card numbers.
        assert_eq!(mask_pan("123456789012"), "123456789012");
        assert_eq!(mask_pan("12345678901234567890"), "12345678901234567890");
        assert_eq!(mask_pan(""), "");
        assert_eq!(
            mask_pan("épée 4111111111111111 é"),
            "épée ************1111 é"
        );
    }

    #[test]
    fn a_long_group_next_to_other_numbers_is_still_masked() {
        // 22 digits in total, one group of 16.
        assert_eq!(
            mask_pan("123456 4111111111111111"),
            "123456 ************1111"
        );
    }

    #[test]
    fn amount_is_written_from_cents() {
        let body = json!({"a": AMOUNT_MARK, "b": "x"});
        assert_eq!(with_amount(&body, 1250), r#"{"a":12.50,"b":"x"}"#);
        assert_eq!(path_segment("a b/c"), "a%20b%2Fc");
        assert_eq!(truncate("abcdef", 3), "abc");
    }
}
