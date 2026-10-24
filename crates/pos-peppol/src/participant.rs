//! VAT numbers and Peppol participant identifiers (V11-D9, V11-D10).
//!
//! A Belgian participant is `0208:<10-digit enterprise number>`: the VAT number without the
//! `BE` prefix (the `0208:BE...` form the first draft used is wrong). The VAT check is
//! offline: Belgian check digits, or for a foreign number a country prefix and 2 to 13
//! alphanumerics. Money never appears here.

/// The Peppol scheme of the Belgian enterprise number (CBE/KBO).
pub const BE_SCHEME: &str = "0208";

/// The typed VAT number does not pass the offline check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidVat;

impl std::fmt::Display for InvalidVat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid VAT number")
    }
}

impl std::error::Error for InvalidVat {}

/// A VAT number that passed the offline check, in its canonical spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatNumber {
    /// ISO 3166-1 alpha-2 country prefix of the number (`BE`, `FR`, ...).
    pub country: String,
    /// The canonical number, e.g. `BE0308357159`.
    pub canonical: String,
}

impl VatNumber {
    pub fn is_belgian(&self) -> bool {
        self.country == "BE"
    }

    /// The Peppol participant id (`0208:0308357159`) of a Belgian number, `None` for a
    /// foreign one (no delivery over Peppol, V11-D9).
    pub fn participant_id(&self) -> Option<String> {
        self.is_belgian()
            .then(|| be_participant_id(&self.canonical))
    }
}

/// `true` when the last two digits are 97 minus the first eight modulo 97.
fn mod97_ok(digits: &str) -> bool {
    let (Ok(base), Ok(check)) = (digits[..8].parse::<u64>(), digits[8..].parse::<u64>()) else {
        return false;
    };
    97 - base % 97 == check
}

/// A Belgian enterprise number: 10 digits, the first 0 or 1, valid check digits.
pub fn is_valid_be_enterprise_number(digits: &str) -> bool {
    digits.len() == 10
        && digits.bytes().all(|b| b.is_ascii_digit())
        && matches!(digits.as_bytes()[0], b'0' | b'1')
        && mod97_ok(digits)
}

/// `0208:<number>` for a Belgian VAT number (`BE0308357159`, with or without the prefix).
pub fn be_participant_id(vat: &str) -> String {
    let digits = vat.strip_prefix("BE").unwrap_or(vat);
    format!("{BE_SCHEME}:{digits}")
}

/// Check a typed VAT number offline and return its canonical form.
///
/// Spaces, dots, dashes and slashes are ignored and the letters are upper-cased. A Belgian
/// number is `BE` and 10 digits (a legacy 9-digit number gets its leading 0) with valid
/// check digits. A foreign number is a two-letter country prefix and 2 to 13 alphanumerics.
/// A number without a prefix is read as Belgian.
pub fn parse_vat_number(input: &str) -> Result<VatNumber, InvalidVat> {
    let cleaned: String = input
        .chars()
        .filter(|c| !matches!(c, ' ' | '.' | '-' | '/'))
        .flat_map(char::to_uppercase)
        .collect();
    if cleaned.is_empty() || !cleaned.is_ascii() {
        return Err(InvalidVat);
    }
    let starts_with_letters = cleaned.as_bytes()[0].is_ascii_alphabetic();
    let (country, rest) = if starts_with_letters {
        if cleaned.len() < 4 {
            return Err(InvalidVat);
        }
        cleaned.split_at(2)
    } else {
        ("BE", cleaned.as_str())
    };
    if !country.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(InvalidVat);
    }
    if country == "BE" {
        let digits = if rest.len() == 9 {
            format!("0{rest}")
        } else {
            rest.to_string()
        };
        if !is_valid_be_enterprise_number(&digits) {
            return Err(InvalidVat);
        }
        return Ok(VatNumber {
            country: "BE".to_string(),
            canonical: format!("BE{digits}"),
        });
    }
    if !(2..=13).contains(&rest.len()) || !rest.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(InvalidVat);
    }
    Ok(VatNumber {
        country: country.to_string(),
        canonical: format!("{country}{rest}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_valid_belgian_number_gets_its_0208_participant_id() {
        let v = parse_vat_number("BE0308357159").unwrap();
        assert_eq!(v.canonical, "BE0308357159");
        assert!(v.is_belgian());
        assert_eq!(v.participant_id().as_deref(), Some("0208:0308357159"));
        // Never the buggy `0208:BE...` form.
        assert!(!be_participant_id("BE0308357159").contains("BE"));
    }

    #[test]
    fn typing_variants_are_normalised() {
        for typed in [
            "be 0308.357.159",
            "BE0308-357-159",
            "0308357159",
            "308357159",
            " be0308357159 ",
        ] {
            assert_eq!(
                parse_vat_number(typed).unwrap().canonical,
                "BE0308357159",
                "{typed}"
            );
        }
    }

    #[test]
    fn wrong_belgian_numbers_are_refused() {
        // One check digit off, a wrong length, a first digit above 1, letters inside.
        for typed in [
            "BE0308357158",
            "BE030835715",
            "BE03083571599",
            "BE2308357159",
            "BE03083571A9",
            "",
            "BE",
            "0",
        ] {
            assert!(parse_vat_number(typed).is_err(), "{typed}");
        }
        assert!(!is_valid_be_enterprise_number("0308357158"));
        assert!(is_valid_be_enterprise_number("0308357159"));
        assert!(is_valid_be_enterprise_number("0499999960"));
    }

    #[test]
    fn a_foreign_number_is_a_prefix_and_two_to_thirteen_alphanumerics() {
        let fr = parse_vat_number("FR 12 345678901").unwrap();
        assert_eq!(fr.canonical, "FR12345678901");
        assert!(!fr.is_belgian());
        assert_eq!(fr.participant_id(), None);
        assert!(parse_vat_number("NL123456789B01").is_ok());
        assert!(parse_vat_number("DE1").is_err());
        assert!(parse_vat_number("DE12345678901234").is_err());
        assert!(parse_vat_number("DE12#456").is_err());
    }
}
