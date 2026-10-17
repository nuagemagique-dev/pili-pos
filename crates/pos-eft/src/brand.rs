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
        "edenred"
        | "ticketrestaurant"
        | "edenredticketrestaurant"
        | "edenredmeal"
        | "edenredmealvoucher"
        | "edenredrestaurant"
        | "edrd" => "Edenred",
        "pluxee" | "sodexo" | "sodexopass" | "sodexolunchpass" | "lunchpass" | "sodexomeal"
        | "sodexomealvoucher" | "sodexocard" | "pluxeemeal" | "pluxeemealvoucher"
        | "pluxeecard" | "pluxeelunchpass" | "sodx" => "Pluxee",
        "monizze" | "monizzemeal" | "monizzemealvoucher" | "monizzecard" => "Monizze",
        "payflip" | "payflipmeal" | "payflipmealvoucher" | "payf" => "Payflip",
        _ => return Some(t.to_string()),
    };
    Some(known.to_string())
}

/// The four meal-voucher issuers (normalized names). A terminal card of
/// one of these brands is signed as `CHEQUE_MEAL`. Exact match only:
/// eco, gift, sport and culture variants stay raw and are not vouchers.
pub const MEAL_VOUCHER_BRANDS: [&str; 4] = ["Edenred", "Pluxee", "Monizze", "Payflip"];

/// True when `brand` (as stored, already normalized) is a meal-voucher
/// issuer.
pub fn is_meal_voucher(brand: Option<&str>) -> bool {
    brand.is_some_and(|b| MEAL_VOUCHER_BRANDS.contains(&b))
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

    #[test]
    fn every_meal_voucher_spelling_maps_to_its_issuer() {
        let edenred = [
            "edenred",
            "EDENRED",
            "Ticket Restaurant",
            "Edenred Ticket Restaurant",
            "edenred_meal",
            "Edenred Meal Voucher",
            "Edenred Restaurant",
            "EDRD",
        ];
        let pluxee = [
            "pluxee",
            "SODEXO",
            "Sodexo Pass",
            "Sodexo Lunch Pass",
            "Lunch Pass",
            "Sodexo Meal",
            "Sodexo Meal Voucher",
            "Sodexo Card",
            "Pluxee Meal",
            "Pluxee Meal Voucher",
            "Pluxee Card",
            "Pluxee Lunch Pass",
            "SODX",
        ];
        let monizze = [
            "Monizze",
            "MONIZZE MEAL",
            "Monizze Meal Voucher",
            "monizze card",
        ];
        let payflip = ["Payflip", "PAYFLIP MEAL", "Payflip Meal Voucher", "PAYF"];
        for (list, name) in [
            (&edenred[..], "Edenred"),
            (&pluxee[..], "Pluxee"),
            (&monizze[..], "Monizze"),
            (&payflip[..], "Payflip"),
        ] {
            for raw in list {
                let n = normalize(raw);
                assert_eq!(n.as_deref(), Some(name), "{raw}");
                assert!(is_meal_voucher(n.as_deref()), "{raw}");
            }
        }
    }

    #[test]
    fn eco_gift_and_other_variants_are_not_meal_vouchers() {
        for raw in [
            "Edenred Eco",
            "Sodexo Eco Cheque",
            "Monizze Eco",
            "Pluxee Gift",
            "Edenred Cadeau",
            "Sodexo Sport",
            "Pluxee Culture",
            "Edenred Consumption",
            "Payflip Gift",
            "Edenred2",
        ] {
            let n = normalize(raw);
            assert_eq!(n.as_deref(), Some(raw), "{raw} stays raw");
            assert!(!is_meal_voucher(n.as_deref()), "{raw}");
        }
        assert!(!is_meal_voucher(Some("Bancontact")));
        assert!(!is_meal_voucher(Some("Visa")));
        assert!(!is_meal_voucher(None));
        assert!(
            !is_meal_voucher(Some("edenred")),
            "stored value is normalized"
        );
    }
}
