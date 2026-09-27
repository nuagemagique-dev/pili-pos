//! HubRise money strings (`"9.80 EUR"`, `"-0.05 EUR"`) to and from
//! integer cents, without floating point.
//!
//! Reference: `docs/hubrise/01-general-concepts.md` (Monetary Values).

use std::fmt;

/// Why a money string was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoneyError {
    /// Not `<amount> <currency>` with at most two decimals.
    Malformed(String),
    /// A currency other than EUR.
    Currency(String),
}

impl fmt::Display for MoneyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(s) => write!(f, "malformed amount {s:?}"),
            Self::Currency(s) => write!(f, "currency is not EUR in {s:?}"),
        }
    }
}

impl std::error::Error for MoneyError {}

/// Parse `"12.50 EUR"` into 1250. EUR only; at most two decimals
/// (`"12.5 EUR"` is 1250, `"12 EUR"` is 1200).
pub fn parse_cents(money: &str) -> Result<i64, MoneyError> {
    let malformed = || MoneyError::Malformed(money.to_string());
    let mut parts = money.split_whitespace();
    let amount = parts.next().ok_or_else(malformed)?;
    let currency = parts.next().ok_or_else(malformed)?;
    if parts.next().is_some() {
        return Err(malformed());
    }
    if currency != "EUR" {
        return Err(MoneyError::Currency(money.to_string()));
    }
    parse_decimal_cents(amount).ok_or_else(malformed)
}

/// Parse a decimal string with at most two decimals (`"-2.5"`) into
/// hundredths (`-250`).
fn parse_decimal_cents(amount: &str) -> Option<i64> {
    let (negative, digits) = match amount.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, amount),
    };
    let (whole, frac) = match digits.split_once('.') {
        Some((w, f)) => (w, f),
        None => (digits, ""),
    };
    if whole.is_empty()
        || frac.len() > 2
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !frac.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let frac: i64 = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>().ok()? * 10,
        _ => frac.parse().ok()?,
    };
    let cents = whole.checked_mul(100)?.checked_add(frac)?;
    Some(if negative { -cents } else { cents })
}

/// Format 1250 as `"12.50 EUR"`.
pub fn format_cents(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    format!("{sign}{}.{:02} EUR", abs / 100, abs % 100)
}

/// A HubRise quantity (`"2"`, `"2.0"`) as a whole number of units.
/// Fractional quantities (weighed products) are refused: the POS
/// sells whole units only.
pub fn parse_quantity(quantity: &str) -> Option<i64> {
    let hundredths = parse_decimal_cents(quantity.trim())?;
    (hundredths % 100 == 0).then_some(hundredths / 100)
}

/// A tax rate (`"12.0"`, `"6"`, `"21.00"`) in hundredths of a
/// percent (1200), or `None` when not a decimal with at most two
/// decimals.
pub fn parse_rate_hundredths(rate: &str) -> Option<i64> {
    let rate = rate.trim();
    // "12.000" style: drop trailing zeros beyond two decimals.
    let normalised = match rate.split_once('.') {
        Some((w, f)) if f.len() > 2 && f[2..].bytes().all(|b| b == b'0') => {
            format!("{w}.{}", &f[..2])
        }
        _ => rate.to_string(),
    };
    parse_decimal_cents(&normalised)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_documented_formats() {
        assert_eq!(parse_cents("8.90 EUR"), Ok(890));
        assert_eq!(parse_cents("-0.05 EUR"), Ok(-5));
        assert_eq!(parse_cents("0.00 EUR"), Ok(0));
        assert_eq!(parse_cents("80000.00 EUR"), Ok(8_000_000));
        assert_eq!(parse_cents("12.5 EUR"), Ok(1250));
        assert_eq!(parse_cents("12 EUR"), Ok(1200));
    }

    #[test]
    fn float_traps_are_exact() {
        // 0.29 * 100 = 28.999999999999996 in f64.
        assert_eq!(parse_cents("0.29 EUR"), Ok(29));
        assert_eq!(parse_cents("1.15 EUR"), Ok(115));
        assert_eq!(parse_cents("4.35 EUR"), Ok(435));
    }

    #[test]
    fn refuses_other_currencies_and_garbage() {
        assert_eq!(
            parse_cents("9.80 USD"),
            Err(MoneyError::Currency("9.80 USD".into()))
        );
        for bad in [
            "",
            "EUR",
            "9.80",
            "abc EUR",
            "9.805 EUR",
            "9,80 EUR",
            "1.2.3 EUR",
            ".50 EUR",
            "9.80 EUR x",
            "+9.80 EUR",
            "- 9.80 EUR",
        ] {
            assert!(parse_cents(bad).is_err(), "{bad:?} accepted");
        }
    }

    #[test]
    fn formats_back() {
        assert_eq!(format_cents(1250), "12.50 EUR");
        assert_eq!(format_cents(5), "0.05 EUR");
        assert_eq!(format_cents(-5), "-0.05 EUR");
        assert_eq!(format_cents(0), "0.00 EUR");
        for c in [-12345, -1, 0, 1, 99, 100, 101, 987654] {
            assert_eq!(parse_cents(&format_cents(c)), Ok(c));
        }
    }

    #[test]
    fn quantities_are_whole_units() {
        assert_eq!(parse_quantity("2"), Some(2));
        assert_eq!(parse_quantity("2.0"), Some(2));
        assert_eq!(parse_quantity("1.00"), Some(1));
        assert_eq!(parse_quantity("1.5"), None);
        assert_eq!(parse_quantity("x"), None);
    }

    #[test]
    fn rates_in_hundredths_of_a_percent() {
        assert_eq!(parse_rate_hundredths("12.0"), Some(1200));
        assert_eq!(parse_rate_hundredths("6"), Some(600));
        assert_eq!(parse_rate_hundredths("21.00"), Some(2100));
        assert_eq!(parse_rate_hundredths("5.5"), Some(550));
        assert_eq!(parse_rate_hundredths("12.000"), Some(1200));
        assert_eq!(parse_rate_hundredths("x"), None);
    }
}
