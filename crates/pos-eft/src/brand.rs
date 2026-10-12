//! Card brands as terminals spell them, normalized, and the SCE card
//! type each one implies.

use crate::CardType;

/// The brand `raw` (a terminal's brand, application label or OPI card
/// circuit, any case) in its display spelling, or `raw` trimmed when it
/// is not a known brand. `None` for an empty value.
pub fn normalize(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    let key: String = t
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    let known = match key.as_str() {
        "bancontact" | "bcmc" | "mistercash" | "bancontactmistercash" | "bc" => "Bancontact",
        "maestro" | "maes" => "Maestro",
        "vpay" => "V PAY",
        "visa" | "visacredit" => "Visa",
        "visadebit" | "visaelectron" | "electron" => "Visa Debit",
        "mastercard" | "mc" | "mastercardcredit" | "eurocardmastercard" => "Mastercard",
        "debitmastercard" | "mastercarddebit" | "mcdebit" => "Debit Mastercard",
        "amex" | "americanexpress" => "American Express",
        "diners" | "dinersclub" | "dinersclubinternational" => "Diners Club",
        "jcb" => "JCB",
        "cup" | "unionpay" | "chinaunionpay" => "UnionPay",
        "payconiq" | "payconiqbybancontact" | "bancontactpay" => "Bancontact Pay",
        _ => return Some(t.to_string()),
    };
    Some(known.to_string())
}

/// The card type a normalized brand implies when the terminal does not
/// say debit or credit itself: the Belgian debit schemes are debit,
/// American Express and Diners are credit, Visa and Mastercard can be
/// either and stay unknown.
pub fn card_type_of(brand: Option<&str>) -> CardType {
    match brand {
        Some("Bancontact" | "Maestro" | "V PAY" | "Visa Debit" | "Debit Mastercard") => {
            CardType::Debit
        }
        Some("American Express" | "Diners Club") => CardType::Credit,
        _ => CardType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_spellings_map_to_one_brand() {
        assert_eq!(normalize("BCMC").as_deref(), Some("Bancontact"));
        assert_eq!(normalize(" bancontact ").as_deref(), Some("Bancontact"));
        assert_eq!(normalize("MAESTRO").as_deref(), Some("Maestro"));
        assert_eq!(normalize("MASTER_CARD").as_deref(), Some("Mastercard"));
        assert_eq!(
            normalize("Some Local Card").as_deref(),
            Some("Some Local Card")
        );
        assert_eq!(normalize("  "), None);
    }

    #[test]
    fn debit_schemes_are_debit_and_the_rest_unknown_or_credit() {
        assert_eq!(card_type_of(Some("Bancontact")), CardType::Debit);
        assert_eq!(card_type_of(Some("American Express")), CardType::Credit);
        assert_eq!(card_type_of(Some("Visa")), CardType::Unknown);
        assert_eq!(card_type_of(None), CardType::Unknown);
    }
}
