//! The printed non-fiscal sales report (V11-D20).
//!
//! It is NOT a fiscal report: it carries no FDM data, takes no ticket number and signs nothing.
//! Its title is "RAPPORT DE VENTES" under "DOCUMENT NON FISCAL", never the words of the X or Z
//! reports, and its footer says it is neither a fiscal report nor a valid VAT ticket (certification
//! manual section L, "autres rapports"; producers FAQ Q508). Money is in cents; this module only
//! formats, the figures come from the engine's `sales_report`.

use pos_fiscal::ticket::{ReceiptLanguage, invalid_disclaimer};
use serde::{Deserialize, Serialize};

use crate::escpos::{Alignment, EscPosBuilder, TextStyle};

/// One booking period of the range with its concordance with the Z.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalesDayPrint {
    /// `YYYY-MM-DD`.
    pub booking_date: String,
    pub z_number: Option<i64>,
    pub total_cents: i64,
    pub z_total_cents: Option<i64>,
    /// The period is still open (no Z yet).
    pub open: bool,
    pub matches_z: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalesProductPrint {
    pub name: String,
    /// Thousandths of a unit (2 pieces = 2000), negative for refunds.
    pub quantity_milli: i64,
    pub amount_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalesHourPrint {
    /// Local hour in Belgium, 0 to 23.
    pub hour: u32,
    pub tickets: i64,
    pub amount_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalesReportPrint {
    pub lang: ReceiptLanguage,
    pub restaurant_name: String,
    pub restaurant_address: String,
    pub restaurant_vat: String,
    /// `YYYY-MM-DD`.
    pub from: String,
    pub to: String,
    /// `dd/mm/yyyy hh:mm`, Belgian time.
    pub generated_at: String,
    pub days: Vec<SalesDayPrint>,
    pub products: Vec<SalesProductPrint>,
    pub hours: Vec<SalesHourPrint>,
    pub price_changes_cents: i64,
    pub rounding_cents: i64,
    pub refunds_cents: i64,
    pub ticket_count: i64,
    pub total_cents: i64,
}

struct Words {
    title: &'static str,
    subtitle: &'static str,
    period: &'static str,
    generated: &'static str,
    concordance: &'static str,
    products: &'static str,
    hours: &'static str,
    open: &'static str,
    gap: &'static str,
    equal: &'static str,
    price_changes: &'static str,
    rounding: &'static str,
    refunds: &'static str,
    tickets: &'static str,
    total: &'static str,
    not_fiscal: &'static str,
}

fn words(lang: ReceiptLanguage) -> &'static Words {
    match lang {
        ReceiptLanguage::Fr => &Words {
            title: "RAPPORT DE VENTES",
            subtitle: "DOCUMENT NON FISCAL",
            period: "Periode",
            generated: "Etabli le",
            concordance: "CONCORDANCE AVEC LES Z",
            products: "VENTES PAR PRODUIT",
            hours: "VENTES PAR HEURE",
            open: "periode en cours",
            gap: "ECART",
            equal: "identique",
            price_changes: "Modifications de prix",
            rounding: "Arrondi caisse (hors total)",
            refunds: "dont remboursements",
            tickets: "Tickets",
            total: "TOTAL DES VENTES",
            not_fiscal: "CECI N'EST PAS UN RAPPORT FISCAL",
        },
        ReceiptLanguage::Nl => &Words {
            title: "VERKOOPRAPPORT",
            subtitle: "NIET-FISCAAL DOCUMENT",
            period: "Periode",
            generated: "Opgesteld op",
            concordance: "OVEREENSTEMMING MET DE Z",
            products: "VERKOOP PER PRODUCT",
            hours: "VERKOOP PER UUR",
            open: "lopende periode",
            gap: "VERSCHIL",
            equal: "gelijk",
            price_changes: "Prijswijzigingen",
            rounding: "Kasafronding (buiten totaal)",
            refunds: "waarvan terugbetalingen",
            tickets: "Tickets",
            total: "TOTAAL VERKOPEN",
            not_fiscal: "DIT IS GEEN FISCAAL RAPPORT",
        },
        ReceiptLanguage::De => &Words {
            title: "VERKAUFSBERICHT",
            subtitle: "NICHT-FISKALES DOKUMENT",
            period: "Zeitraum",
            generated: "Erstellt am",
            concordance: "ABGLEICH MIT DEN Z",
            products: "VERKAEUFE PRO PRODUKT",
            hours: "VERKAEUFE PRO STUNDE",
            open: "laufender Zeitraum",
            gap: "ABWEICHUNG",
            equal: "gleich",
            price_changes: "Preisaenderungen",
            rounding: "Kassenrundung (ohne Summe)",
            refunds: "davon Rueckerstattungen",
            tickets: "Tickets",
            total: "SUMME DER VERKAEUFE",
            not_fiscal: "DIES IST KEIN FISKALER BERICHT",
        },
    }
}

/// The footer line "this is not a valid VAT ticket". French is the sentence of the plan (V11-D20);
/// the other languages reuse the fiscal tickets' own.
fn not_a_ticket(lang: ReceiptLanguage) -> &'static str {
    match lang {
        ReceiptLanguage::Fr => "CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE",
        other => invalid_disclaimer(other),
    }
}

fn fmt_price(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    format!("{sign}{},{:02}", abs / 100, abs % 100)
}

/// `2026-10-17` as `17/10/2026`; anything else as is.
fn fmt_date(date: &str) -> String {
    match date.split('-').collect::<Vec<_>>().as_slice() {
        [y, m, d] if y.len() == 4 => format!("{d}/{m}/{y}"),
        _ => date.to_string(),
    }
}

/// Quantity in thousandths as `2` or `0,350`.
fn fmt_quantity(milli: i64) -> String {
    let sign = if milli < 0 { "-" } else { "" };
    let abs = milli.abs();
    if abs % 1000 == 0 {
        format!("{sign}{}", abs / 1000)
    } else {
        format!("{sign}{},{:03}", abs / 1000, abs % 1000)
    }
}

fn section(p: &mut EscPosBuilder, title: &str) {
    p.separator('-');
    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln(title);
    p.style_reset();
    p.align(Alignment::Left);
}

/// Format the sales report for the thermal printer.
pub fn format_sales_report(data: &SalesReportPrint) -> Vec<u8> {
    let w = words(data.lang);
    let mut p = EscPosBuilder::new();

    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        double_height: true,
        double_width: true,
        ..Default::default()
    });
    p.textln(w.title);
    p.style_reset();
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln(w.subtitle);
    p.style_reset();
    p.blank_line();
    p.textln_wrapped(&data.restaurant_name);
    p.textln_wrapped(&data.restaurant_address);
    p.textln(&data.restaurant_vat);
    p.align(Alignment::Left);
    p.separator('=');
    p.textln(&format!(
        "{}: {} - {}",
        w.period,
        fmt_date(&data.from),
        fmt_date(&data.to)
    ));
    p.textln(&format!("{}: {}", w.generated, data.generated_at));

    section(&mut p, w.concordance);
    for day in &data.days {
        let z = day
            .z_number
            .map_or_else(|| "Z -".to_string(), |n| format!("Z {n}"));
        p.text_columns(
            &format!("{} {z}", fmt_date(&day.booking_date)),
            &fmt_price(day.total_cents),
        );
        let status = if day.open {
            w.open.to_string()
        } else if day.matches_z {
            w.equal.to_string()
        } else {
            let gap = day.total_cents - day.z_total_cents.unwrap_or(0);
            format!("{} {}", w.gap, fmt_price(gap))
        };
        p.textln(&format!("  {status}"));
    }

    section(&mut p, w.products);
    for product in &data.products {
        p.text_columns(
            &format!(
                "{} x {}",
                fmt_quantity(product.quantity_milli),
                product.name
            ),
            &fmt_price(product.amount_cents),
        );
    }

    section(&mut p, w.hours);
    for hour in &data.hours {
        p.text_columns(
            &format!("{:02}h  {} {}", hour.hour, hour.tickets, w.tickets),
            &fmt_price(hour.amount_cents),
        );
    }

    p.separator('=');
    if data.price_changes_cents != 0 {
        p.text_columns(w.price_changes, &fmt_price(data.price_changes_cents));
    }
    if data.refunds_cents != 0 {
        p.text_columns(w.refunds, &fmt_price(data.refunds_cents));
    }
    if data.rounding_cents != 0 {
        p.text_columns(w.rounding, &fmt_price(data.rounding_cents));
    }
    p.text_columns(w.tickets, &data.ticket_count.to_string());
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.text_columns(w.total, &fmt_price(data.total_cents));
    p.style_reset();
    p.separator('=');

    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln_wrapped(w.not_fiscal);
    p.textln_wrapped(not_a_ticket(data.lang));
    p.style_reset();
    p.feed(3);
    p.cut();
    p.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::render_plain_text;

    fn sample(lang: ReceiptLanguage) -> SalesReportPrint {
        SalesReportPrint {
            lang,
            restaurant_name: "Resto".into(),
            restaurant_address: "1 Rue Test, 4000 Liege".into(),
            restaurant_vat: "BE0123456789".into(),
            from: "2026-10-16".into(),
            to: "2026-10-17".into(),
            generated_at: "17/10/2026 18:30".into(),
            days: vec![
                SalesDayPrint {
                    booking_date: "2026-10-16".into(),
                    z_number: Some(12),
                    total_cents: 12_550,
                    z_total_cents: Some(12_550),
                    open: false,
                    matches_z: true,
                },
                SalesDayPrint {
                    booking_date: "2026-10-17".into(),
                    z_number: None,
                    total_cents: 4_000,
                    z_total_cents: None,
                    open: true,
                    matches_z: false,
                },
            ],
            products: vec![
                SalesProductPrint {
                    name: "Maki saumon".into(),
                    quantity_milli: 3_000,
                    amount_cents: 1_950,
                },
                SalesProductPrint {
                    name: "Sashimi".into(),
                    quantity_milli: -1_000,
                    amount_cents: -1_200,
                },
            ],
            hours: vec![SalesHourPrint {
                hour: 19,
                tickets: 4,
                amount_cents: 9_000,
            }],
            price_changes_cents: -100,
            rounding_cents: 2,
            refunds_cents: -1_200,
            ticket_count: 6,
            total_cents: 16_550,
        }
    }

    fn text(lang: ReceiptLanguage) -> String {
        render_plain_text(&format_sales_report(&sample(lang)))
    }

    #[test]
    fn the_title_and_both_disclaimers_are_printed() {
        let out = text(ReceiptLanguage::Fr);
        assert!(out.contains("RAPPORT DE VENTES"));
        assert!(out.contains("DOCUMENT NON FISCAL"));
        assert!(out.contains("CECI N'EST PAS UN RAPPORT FISCAL"));
        assert!(out.contains("CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE"));
    }

    #[test]
    fn the_title_never_uses_the_words_of_the_fiscal_reports() {
        for lang in [
            ReceiptLanguage::Fr,
            ReceiptLanguage::Nl,
            ReceiptLanguage::De,
        ] {
            let out = text(lang);
            let head: Vec<&str> = out.lines().take(3).collect();
            for line in head {
                let upper = line.to_uppercase();
                assert!(!upper.contains("CHIFFRE D'AFFAIRES"), "{line}");
                assert!(!upper.contains("OMZET"), "{line}");
                assert!(
                    !upper.split_whitespace().any(|w| w == "X" || w == "Z"),
                    "{line}"
                );
            }
        }
    }

    #[test]
    fn the_concordance_marks_equal_gap_and_open_periods() {
        let out = text(ReceiptLanguage::Fr);
        assert!(out.contains("16/10/2026 Z 12"));
        assert!(out.contains("identique"));
        assert!(out.contains("periode en cours"));
        let mut data = sample(ReceiptLanguage::Fr);
        data.days[0].matches_z = false;
        data.days[0].z_total_cents = Some(12_000);
        let out = render_plain_text(&format_sales_report(&data));
        assert!(out.contains("ECART 5,50"), "{out}");
    }

    #[test]
    fn the_amounts_use_a_decimal_comma_and_negative_refunds() {
        let out = text(ReceiptLanguage::Fr);
        assert!(out.contains("3 x Maki saumon"));
        assert!(out.contains("19,50"));
        assert!(out.contains("-1 x Sashimi"));
        assert!(out.contains("-12,00"));
        assert!(out.contains("TOTAL DES VENTES"));
        assert!(out.contains("165,50"));
        assert!(out.contains("19h  4 Tickets"));
    }

    #[test]
    fn the_report_is_never_a_fiscal_ticket() {
        let out = text(ReceiptLanguage::Fr);
        for needle in ["FDM", "Signature", "TICKET DE CAISSE T.V.A."] {
            assert!(!out.contains(needle), "{needle}");
        }
    }

    #[test]
    fn the_other_languages_have_their_own_texts() {
        assert!(text(ReceiptLanguage::Nl).contains("VERKOOPRAPPORT"));
        assert!(text(ReceiptLanguage::De).contains("VERKAUFSBERICHT"));
    }

    #[test]
    fn fractional_quantities_keep_their_decimals() {
        assert_eq!(fmt_quantity(350), "0,350");
        assert_eq!(fmt_quantity(-2_000), "-2");
        assert_eq!(fmt_price(-5), "-0,05");
    }
}
