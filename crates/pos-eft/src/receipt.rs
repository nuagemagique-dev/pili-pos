//! What can be read out of a terminal's receipt text when the protocol
//! does not give it as a field: debit or credit, the masked card
//! number, the brand, a labelled value.

use crate::CardType;

/// Debit or credit when `text` says exactly one of the two.
pub(crate) fn explicit_card_type(text: &str) -> Option<CardType> {
    let t = text.to_lowercase();
    let credit = ["credit", "crédit", "kredit"].iter().any(|w| t.contains(w));
    let debit = ["debit", "débit", "debet"].iter().any(|w| t.contains(w));
    match (credit, debit) {
        (true, false) => Some(CardType::Credit),
        (false, true) => Some(CardType::Debit),
        _ => None,
    }
}

/// The `Account Type:` line of a receipt as a card type.
pub(crate) fn account_type(receipt: &str) -> Option<CardType> {
    receipt.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.trim().eq_ignore_ascii_case("account type") {
            explicit_card_type(value)
        } else {
            None
        }
    })
}

/// The value after the first `label:` line (labels compared ignoring
/// case and surrounding blanks).
pub(crate) fn field(receipt: &str, labels: &[&str]) -> Option<String> {
    receipt.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        let key = key.trim();
        let value = value.trim();
        if !value.is_empty() && labels.iter().any(|l| key.eq_ignore_ascii_case(l)) {
            Some(value.to_string())
        } else {
            None
        }
    })
}

/// A masked card number: a token of mask characters and digits that
/// hides at least four positions and ends in at least four digits.
pub(crate) fn masked_pan(receipt: &str) -> Option<String> {
    receipt.split_whitespace().find_map(|token| {
        let ok_chars = token
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'X' | 'x' | '*' | '•' | '#' | '-'));
        let masks = token
            .chars()
            .filter(|c| matches!(c, 'X' | 'x' | '*' | '•' | '#'))
            .count();
        let tail = token.chars().rev().take_while(char::is_ascii_digit).count();
        (token.chars().count() >= 8 && ok_chars && masks >= 4 && tail >= 4)
            .then(|| token.to_string())
    })
}

/// The first card brand a receipt names, in the display spelling.
pub(crate) fn brand_in(receipt: &str) -> Option<String> {
    const NAMES: [(&str, &str); 19] = [
        ("BANCONTACT", "Bancontact"),
        ("BCMC", "Bancontact"),
        ("MAESTRO", "Maestro"),
        ("V PAY", "V PAY"),
        ("VPAY", "V PAY"),
        ("VISA", "Visa"),
        ("MASTERCARD", "Mastercard"),
        ("MASTER CARD", "Mastercard"),
        ("AMERICAN EXPRESS", "American Express"),
        ("AMEX", "American Express"),
        ("DINERS", "Diners Club"),
        ("JCB", "JCB"),
        ("UNIONPAY", "UnionPay"),
        ("EDENRED", "Edenred"),
        ("TICKET RESTAURANT", "Edenred"),
        ("PLUXEE", "Pluxee"),
        ("SODEXO", "Pluxee"),
        ("MONIZZE", "Monizze"),
        ("PAYFLIP", "Payflip"),
    ];
    let upper = receipt.to_uppercase();
    NAMES
        .iter()
        .filter_map(|(needle, brand)| upper.find(needle).map(|at| (at, *brand)))
        .min_by_key(|(at, _)| *at)
        .map(|(_, brand)| brand.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECEIPT: &str = "   Purchase\n   VISA CREDIT contactless\n   XXXXXXXXXXXX0119\n   \
        Account Type:             Credit\n   Auth. Code:               057737\n";

    #[test]
    fn a_receipt_gives_its_fields() {
        assert_eq!(account_type(RECEIPT), Some(CardType::Credit));
        assert_eq!(masked_pan(RECEIPT).as_deref(), Some("XXXXXXXXXXXX0119"));
        assert_eq!(brand_in(RECEIPT).as_deref(), Some("Visa"));
        assert_eq!(
            field(RECEIPT, &["Auth. Code", "Approval Code"]).as_deref(),
            Some("057737")
        );
        assert_eq!(field(RECEIPT, &["Nothing"]), None);
    }

    #[test]
    fn a_meal_voucher_receipt_names_its_issuer() {
        for (line, brand) in [
            ("EDENRED", "Edenred"),
            ("Ticket Restaurant", "Edenred"),
            ("PLUXEE", "Pluxee"),
            ("SODEXO PASS", "Pluxee"),
            ("Monizze", "Monizze"),
            ("PAYFLIP", "Payflip"),
        ] {
            let receipt = format!("   Purchase\n   {line}\n   XXXXXXXXXXXX0119\n");
            assert_eq!(brand_in(&receipt).as_deref(), Some(brand), "{line}");
        }
    }

    #[test]
    fn debit_and_credit_words_decide_only_when_alone() {
        assert_eq!(explicit_card_type("Débit"), Some(CardType::Debit));
        assert_eq!(explicit_card_type("VISA CREDIT"), Some(CardType::Credit));
        assert_eq!(explicit_card_type("credit or debit"), None);
        assert_eq!(explicit_card_type("MAESTRO"), None);
        assert_eq!(masked_pan("1234 5678"), None);
        assert_eq!(
            masked_pan("************1234").as_deref(),
            Some("************1234")
        );
    }
}
