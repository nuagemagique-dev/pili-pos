//! Bar / kitchen order slip for table orders and direct sales.
//!
//! Layout of the SPF's example bar ticket (2026-09-24): date, operator,
//! table, then "quantity x article" lines. A bar ticket never shows a
//! price and carries the "not a VAT ticket" mention. The webshop's
//! kitchen slip (`kitchen_ticket.rs`) is a different, non-certified
//! layout.

use pos_fiscal::ticket::{ReceiptLanguage, invalid_disclaimer};

use crate::escpos::{Alignment, EscPosBuilder, TextStyle};
use crate::labels::bar_labels;

/// One line sent to the bar or kitchen: what was just ordered, or a
/// correction (negative quantity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarItem {
    pub quantity: i32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct BarTicket {
    /// Local date and time of the order, already formatted
    /// ("15/06/2026 12:09:35").
    pub date_time: String,
    /// Name of the operator who sent the order.
    pub operator: String,
    /// Table number; `None` for a direct sale.
    pub table: Option<i32>,
    pub items: Vec<BarItem>,
    pub lang: ReceiptLanguage,
    pub is_training: bool,
}

/// ESC/POS bytes of the slip, ready for a `TcpPrinter`.
pub fn format_bar_ticket(ticket: &BarTicket) -> Vec<u8> {
    let l = bar_labels(ticket.lang);
    let mut p = EscPosBuilder::new();

    p.align(Alignment::Center);
    if ticket.is_training {
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.textln("TRAINING");
        p.style_reset();
    }
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.textln(l.title);
    p.style_reset();
    p.separator('=');

    p.align(Alignment::Left);
    p.text_columns(l.date, &ticket.date_time);
    p.text_columns(l.operator, &ticket.operator);
    let table = match ticket.table {
        Some(t) => format!("{} {t}", l.table),
        None => l.direct_sale.to_string(),
    };
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.textln(&table);
    p.style_reset();
    p.separator('-');

    p.textln(l.items_header);
    for item in &ticket.items {
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.textln_wrapped(&format!("{} x {}", item.quantity, item.name));
        p.style_reset();
    }
    p.separator('-');

    p.align(Alignment::Center);
    p.textln_wrapped(invalid_disclaimer(ticket.lang));
    p.feed(4);
    p.cut();
    p.build()
}

/// The notice that an order moved to another table: kitchen and bar
/// know where the dishes go. Not an order (no item, nothing to
/// prepare), not a VAT ticket (no amount, the usual mention).
#[derive(Debug, Clone)]
pub struct TransferSlip {
    /// Local date and time, already formatted.
    pub date_time: String,
    pub operator: String,
    pub from_table: i32,
    pub to_table: i32,
    pub lang: ReceiptLanguage,
    pub is_training: bool,
}

/// ESC/POS bytes of the transfer notice.
pub fn format_transfer_slip(slip: &TransferSlip) -> Vec<u8> {
    let l = bar_labels(slip.lang);
    let mut p = EscPosBuilder::new();

    p.align(Alignment::Center);
    if slip.is_training {
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.textln("TRAINING");
        p.style_reset();
    }
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.textln(l.transfer_title);
    p.style_reset();
    p.separator('=');

    p.align(Alignment::Left);
    p.text_columns(l.date, &slip.date_time);
    p.text_columns(l.operator, &slip.operator);
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.textln_wrapped(&format!(
        "{} {} -> {} {}",
        l.table, slip.from_table, l.table, slip.to_table
    ));
    p.style_reset();
    p.separator('-');

    p.align(Alignment::Center);
    p.textln_wrapped(invalid_disclaimer(slip.lang));
    p.feed(4);
    p.cut();
    p.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::render_plain_text;

    fn ticket(lang: ReceiptLanguage) -> BarTicket {
        BarTicket {
            date_time: "15/06/2026 12:09:35".into(),
            operator: "Beheerder".into(),
            table: Some(3),
            items: vec![
                BarItem {
                    quantity: 1,
                    name: "Dry Martini".into(),
                },
                BarItem {
                    quantity: 1,
                    name: "Plat du jour".into(),
                },
            ],
            lang,
            is_training: false,
        }
    }

    fn text(t: &BarTicket) -> String {
        render_plain_text(&format_bar_ticket(t))
    }

    #[test]
    fn vi_bar_the_slip_follows_the_official_example() {
        let out = crate::render::one_line(&text(&ticket(ReceiptLanguage::Fr)));
        for expected in [
            "BON DE COMMANDE",
            "15/06/2026 12:09:35",
            "Operateur",
            "Beheerder",
            "Table 3",
            "1 x Dry Martini",
            "1 x Plat du jour",
            "CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE",
        ] {
            assert!(out.contains(expected), "{expected:?} missing from:\n{out}");
        }
    }

    #[test]
    fn vi_bar_a_bar_ticket_never_shows_a_price() {
        let out = text(&ticket(ReceiptLanguage::Fr));
        assert!(!out.contains('€') && !out.contains("EUR"), "{out}");
        // No amount: the only digits are the date, time, quantities and
        // the table number.
        assert!(!out.contains(",00") && !out.contains(".00"), "{out}");
    }

    #[test]
    fn vi_bar_a_correction_prints_its_negative_quantity() {
        let mut t = ticket(ReceiptLanguage::Fr);
        t.items = vec![BarItem {
            quantity: -1,
            name: "Dry Martini".into(),
        }];
        assert!(text(&t).contains("-1 x Dry Martini"));
    }

    #[test]
    fn vi_bar_a_direct_sale_has_no_table() {
        let mut t = ticket(ReceiptLanguage::Fr);
        t.table = None;
        let out = text(&t);
        assert!(out.contains("Vente directe"), "{out}");
        assert!(!out.contains("Table 3"), "{out}");
    }

    #[test]
    fn vi_bar_training_is_marked() {
        let mut t = ticket(ReceiptLanguage::Fr);
        t.is_training = true;
        assert!(text(&t).contains("TRAINING"));
    }

    #[test]
    fn vi_bar_labels_follow_the_ticket_language() {
        let nl = text(&ticket(ReceiptLanguage::Nl));
        assert!(nl.contains("BESTELBON") && nl.contains("Tafel 3"), "{nl}");
        assert!(nl.contains("DIT IS GEEN GELDIG BTW-KASTICKET"), "{nl}");
        assert!(!nl.contains("Table"), "{nl}");
        let de = text(&ticket(ReceiptLanguage::De));
        assert!(
            de.contains("BESTELLSCHEIN") && de.contains("Tisch 3"),
            "{de}"
        );
        assert!(!de.contains("Table"), "{de}");
    }

    #[test]
    fn vi_bar_lines_fit_the_paper() {
        let mut t = ticket(ReceiptLanguage::Fr);
        t.items[0].name = "Un article au nom vraiment tres long qui depasse la largeur".into();
        for line in text(&t).lines() {
            assert!(line.chars().count() <= 48, "too wide: {line:?}");
        }
    }

    fn transfer(lang: ReceiptLanguage) -> TransferSlip {
        TransferSlip {
            date_time: "15/06/2026 12:09:35".into(),
            operator: "Beheerder".into(),
            from_table: 3,
            to_table: 12,
            lang,
            is_training: false,
        }
    }

    #[test]
    fn vi_bar_the_transfer_notice_names_both_tables_and_no_item() {
        let out = crate::render::one_line(&render_plain_text(&format_transfer_slip(&transfer(
            ReceiptLanguage::Fr,
        ))));
        for expected in [
            "TRANSFERT DE TABLE",
            "15/06/2026 12:09:35",
            "Beheerder",
            "Table 3 -> Table 12",
            "CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE",
        ] {
            assert!(out.contains(expected), "{expected:?} missing from:\n{out}");
        }
        assert!(!out.contains(" x "), "{out}");
        assert!(
            !out.contains('€') && !out.contains("EUR") && !out.contains(",00"),
            "{out}"
        );
    }

    #[test]
    fn vi_bar_the_transfer_notice_follows_the_ticket_language_and_training() {
        let nl = render_plain_text(&format_transfer_slip(&transfer(ReceiptLanguage::Nl)));
        assert!(
            nl.contains("TAFELVERPLAATSING") && nl.contains("Tafel 3 -> Tafel 12"),
            "{nl}"
        );
        let mut t = transfer(ReceiptLanguage::De);
        t.is_training = true;
        let de = render_plain_text(&format_transfer_slip(&t));
        assert!(
            de.contains("TISCHWECHSEL") && de.contains("Tisch 3 -> Tisch 12"),
            "{de}"
        );
        assert!(de.contains("TRAINING"), "{de}");
        for line in de.lines() {
            assert!(line.chars().count() <= 48, "too wide: {line:?}");
        }
    }
}
