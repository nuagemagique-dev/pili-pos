//! `HubRise` money strings (`"9.80 EUR"`, `"-0.05 EUR"`) to and from
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
    /// Beyond [`MAX_AMOUNT_CENTS`] in either direction.
    OutOfRange(String),
}

/// Largest amount accepted from `HubRise`, in cents (10 million euros). A
/// remote value is bounded here so that the arithmetic on it cannot
/// overflow.
pub const MAX_AMOUNT_CENTS: i64 = 1_000_000_000;

/// Largest quantity accepted for a line, in whole units.
pub const MAX_QUANTITY: i64 = 10_000;

impl fmt::Display for MoneyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(s) => write!(f, "malformed amount {s:?}"),
            Self::Currency(s) => write!(f, "currency is not EUR in {s:?}"),
            Self::OutOfRange(s) => write!(f, "amount out of range in {s:?}"),
        }
    }
}

impl std::error::Error for MoneyError {}

/// Parse `"12.50 EUR"` into 1250. EUR only; at most two decimals
/// (`"12.5 EUR"` is 1250, `"12 EUR"` is 1200). An amount beyond
/// [`MAX_AMOUNT_CENTS`] is refused.
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
    let cents = parse_decimal_cents(amount).ok_or_else(malformed)?;
    if cents.abs() > MAX_AMOUNT_CENTS {
        return Err(MoneyError::OutOfRange(money.to_string()));
    }
    Ok(cents)
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

/// A `HubRise` quantity (`"2"`, `"2.0"`) as a whole number of units.
/// Fractional quantities (weighed products) are refused: the POS
/// sells whole units only. So is a quantity beyond [`MAX_QUANTITY`].
pub fn parse_quantity(quantity: &str) -> Option<i64> {
    let hundredths = parse_decimal_cents(quantity.trim())?;
    let units = hundredths / 100;
    (hundredths % 100 == 0 && units.abs() <= MAX_QUANTITY).then_some(units)
}

/// An inventory `stock` decimal string (0 to 3 decimals,
/// non-negative) as thousandths of a unit (`"2.5"` -> 2500,
/// `"0"` -> 0). `None` when malformed, negative, or more than 3
/// decimals.
pub fn parse_stock_thousandths(stock: &str) -> Option<i64> {
    let stock = stock.trim();
    if stock.starts_with('-') {
        return None;
    }
    let (whole, frac) = match stock.split_once('.') {
        Some((w, f)) => (w, f),
        None => (stock, ""),
    };
    if whole.is_empty()
        || frac.len() > 3
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !frac.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let frac_padded = format!("{frac:0<3}");
    let frac: i64 = frac_padded.parse().ok()?;
    whole.checked_mul(1000)?.checked_add(frac)
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
        for c in [-12345, -1, 0, 1, 99, 100, 101, 987_654] {
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
    fn remote_values_are_bounded() {
        assert_eq!(parse_cents("10000000.00 EUR"), Ok(MAX_AMOUNT_CENTS));
        assert_eq!(
            parse_cents("10000000.01 EUR"),
            Err(MoneyError::OutOfRange("10000000.01 EUR".into()))
        );
        assert!(matches!(
            parse_cents("90000000000000.00 EUR"),
            Err(MoneyError::OutOfRange(_))
        ));
        assert!(matches!(
            parse_cents("-90000000000000.00 EUR"),
            Err(MoneyError::OutOfRange(_))
        ));
        assert_eq!(parse_quantity("10000"), Some(MAX_QUANTITY));
        assert_eq!(parse_quantity("10001"), None);
        assert_eq!(parse_quantity("99999999999"), None);
    }

    #[test]
    fn stock_decimals_up_to_three_places() {
        assert_eq!(parse_stock_thousandths("0"), Some(0));
        assert_eq!(parse_stock_thousandths("2"), Some(2000));
        assert_eq!(parse_stock_thousandths("2.5"), Some(2500));
        assert_eq!(parse_stock_thousandths("2.500"), Some(2500));
        assert_eq!(parse_stock_thousandths("0.001"), Some(1));
        assert_eq!(parse_stock_thousandths("-1"), None, "non-negative only");
        assert_eq!(parse_stock_thousandths("1.2345"), None, "max 3 decimals");
        assert_eq!(parse_stock_thousandths("x"), None);
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
