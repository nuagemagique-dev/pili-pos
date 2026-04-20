//! Customer slip formatter for HubRise webshop orders.
//!
//! Non-fiscal counterpart to `receipt.rs`. Used by the POS to print
//! the slip handed to the driver / pickup customer. Always carries
//! a "not a valid VAT receipt" disclaimer because online webshop
//! orders bypass the FDM (only dine-in goes through SCE 2.0).

use crate::escpos::{Alignment, EscPosBuilder, TextStyle};

#[derive(Debug, Clone)]
pub struct CustomerLine {
    pub quantity: i64,
    pub product_name: String,
    /// Total for this line in eurocents.
    pub line_total_cents: i64,
}

#[derive(Debug, Clone, Default)]
pub struct CustomerInfo {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub address_1: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CustomerSlip {
    pub restaurant_name: String,
    pub restaurant_address: String,
    /// "2026-04-11 12:30" — already formatted by the caller.
    pub printed_at: String,
    pub source: String,
    pub service_type: String,
    pub short_ref: String,
    pub lines: Vec<CustomerLine>,
    pub total_cents: i64,
    pub customer: CustomerInfo,
}

/// Build the ESC/POS byte stream for a customer slip.
pub fn format_customer_slip(slip: &CustomerSlip) -> Vec<u8> {
    let mut p = EscPosBuilder::new();

    // ── Restaurant header ────────────────────────────────────────
    p.align(Alignment::Center);
    p.style(TextStyle { bold: true, double_height: true, ..Default::default() });
    p.textln(&slip.restaurant_name);
    p.style_reset();
    p.textln(&slip.restaurant_address);
    p.separator('-');

    // ── Order metadata ───────────────────────────────────────────
    p.align(Alignment::Left);
    p.text_columns("Date:", &slip.printed_at);
    p.text_columns("Canal:", &slip.source);
    p.text_columns("Type:", &slip.service_type);
    p.text_columns("Ref:", &slip.short_ref);
    p.separator('-');

    // ── Line items ───────────────────────────────────────────────
    for line in &slip.lines {
        let qty_desc = format!("{}x {}", line.quantity, line.product_name);
        p.text_columns(&qty_desc, &format_eur(line.line_total_cents));
    }
    p.separator('-');

    // ── Total ────────────────────────────────────────────────────
    p.style(TextStyle { bold: true, double_height: true, ..Default::default() });
    p.text_columns("TOTAL", &format_eur(slip.total_cents));
    p.style_reset();
    p.separator('-');

    // ── Customer block ───────────────────────────────────────────
    let c = &slip.customer;
    let has_customer = c.name.is_some()
        || c.phone.is_some()
        || c.address_1.is_some()
        || c.postal_code.is_some()
        || c.city.is_some();
    if has_customer {
        p.align(Alignment::Left);
        if let Some(name) = &c.name {
            p.style(TextStyle { bold: true, ..Default::default() });
            p.textln(name);
            p.style_reset();
        }
        if let Some(phone) = &c.phone {
            p.textln(phone);
        }
        if let Some(addr) = &c.address_1 {
            p.textln(addr);
        }
        let city_line = match (&c.postal_code, &c.city) {
            (Some(pc), Some(city)) => Some(format!("{} {}", pc, city)),
            (Some(pc), None) => Some(pc.clone()),
            (None, Some(city)) => Some(city.clone()),
            (None, None) => None,
        };
        if let Some(line) = city_line {
            p.textln(&line);
        }
        p.separator('-');
    }

    // ── Non-fiscal disclaimer (always) ───────────────────────────
    p.align(Alignment::Center);
    p.style(TextStyle { bold: true, ..Default::default() });
    p.textln("Ce ticket n'est pas");
    p.textln("un recu TVA valide");
    p.style_reset();

    // ── End ──────────────────────────────────────────────────────
    p.feed(4);
    p.cut();
    p.build()
}

/// Format an integer cents amount as `"12.50"` (no currency symbol;
/// the slip layout already implies EUR).
fn format_eur(cents: i64) -> String {
    let euros = cents / 100;
    let frac = (cents % 100).abs();
    format!("{}.{:02}", euros, frac)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> CustomerSlip {
        CustomerSlip {
            restaurant_name: "Tokyo Sushi Bar".to_string(),
            restaurant_address: "Rue Test 1, 1000 Bruxelles".to_string(),
            printed_at: "2026-04-11 12:30".to_string(),
            source: "Deliveroo".to_string(),
            service_type: "Livraison".to_string(),
            short_ref: "abcd1234".to_string(),
            lines: vec![
                CustomerLine {
                    quantity: 2,
                    product_name: "Maki saumon x8".to_string(),
                    line_total_cents: 2400,
                },
                CustomerLine {
                    quantity: 1,
                    product_name: "Ice Tea 33cl".to_string(),
                    line_total_cents: 300,
                },
            ],
            total_cents: 2700,
            customer: CustomerInfo {
                name: Some("Alice Martin".to_string()),
                phone: Some("+32475111111".to_string()),
                address_1: Some("Rue Leopold 12".to_string()),
                postal_code: Some("4000".to_string()),
                city: Some("Liege".to_string()),
            },
        }
    }

    #[test]
    fn contains_restaurant_header() {
        let bytes = format_customer_slip(&sample());
        assert!(bytes.windows(15).any(|w| w == b"Tokyo Sushi Bar"));
    }

    #[test]
    fn contains_total_value() {
        let bytes = format_customer_slip(&sample());
        // 2700 cents → "27.00"
        assert!(bytes.windows(5).any(|w| w == b"27.00"));
    }

    #[test]
    fn contains_disclaimer() {
        let bytes = format_customer_slip(&sample());
        assert!(bytes.windows(18).any(|w| w == b"un recu TVA valide"));
    }

    #[test]
    fn contains_customer_name() {
        let bytes = format_customer_slip(&sample());
        assert!(bytes.windows(12).any(|w| w == b"Alice Martin"));
    }

    #[test]
    fn ends_with_cut() {
        let bytes = format_customer_slip(&sample());
        assert!(bytes.windows(4).any(|w| w == [0x1D, b'V', 66, 3]));
    }

    #[test]
    fn format_eur_cents() {
        assert_eq!(format_eur(0), "0.00");
        assert_eq!(format_eur(50), "0.50");
        assert_eq!(format_eur(2400), "24.00");
        assert_eq!(format_eur(2705), "27.05");
    }
}
