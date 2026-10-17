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
    /// Course of the line (v1.2, kitchen only): `Some` when the slip is
    /// grouped by course, a header is printed whenever it changes.
    /// `None` prints the slip as before courses existed.
    pub course: Option<i32>,
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
    /// The course a release slip sends ("SUITE : Plats"). Same title and
    /// mentions as any other slip, no amount.
    pub release: Option<i32>,
    /// A reprint: the slip says DUPLICATA.
    pub duplicate: bool,
}

/// Name of a course on the slip, in the slip's language.
fn course_name(lang: ReceiptLanguage, course: i32) -> &'static str {
    let names = bar_labels(lang).courses;
    names[usize::try_from(course).unwrap_or(0).min(3)]
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
    if ticket.duplicate {
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.textln(l.duplicate);
        p.style_reset();
    }

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
    if let Some(course) = ticket.release {
        p.style(TextStyle {
            bold: true,
            double_height: true,
            ..Default::default()
        });
        p.textln_wrapped(&format!(
            "{} : {}",
            l.release,
            course_name(ticket.lang, course)
        ));
        p.style_reset();
    }
    p.separator('-');

    p.textln(l.items_header);
    let mut heading: Option<i32> = None;
    for item in &ticket.items {
        if let Some(course) = item.course
            && heading != Some(course)
        {
            heading = Some(course);
            p.textln_wrapped(&format!("-- {} --", course_name(ticket.lang, course)));
        }
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
                    course: None,
                },
                BarItem {
                    quantity: 1,
                    name: "Plat du jour".into(),
                    course: None,
                },
            ],
            lang,
            is_training: false,
            release: None,
            duplicate: false,
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
            course: None,
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

    /// The slip as it was before courses existed (v1.0.0), rebuilt by
    /// hand: a slip without course, release or duplicate mention must
    /// still be these exact bytes.
    fn legacy_bytes(t: &BarTicket) -> Vec<u8> {
        let l = bar_labels(t.lang);
        let mut p = EscPosBuilder::new();
        p.align(Alignment::Center);
        if t.is_training {
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
        p.text_columns(l.date, &t.date_time);
        p.text_columns(l.operator, &t.operator);
        let table = match t.table {
            Some(n) => format!("{} {n}", l.table),
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
        for item in &t.items {
            p.style(TextStyle {
                bold: true,
                ..Default::default()
            });
            p.textln_wrapped(&format!("{} x {}", item.quantity, item.name));
            p.style_reset();
        }
        p.separator('-');
        p.align(Alignment::Center);
        p.textln_wrapped(invalid_disclaimer(t.lang));
        p.feed(4);
        p.cut();
        p.build()
    }

    #[test]
    fn vi_course_a_slip_without_course_is_byte_identical_to_the_one_before() {
        for lang in [
            ReceiptLanguage::Fr,
            ReceiptLanguage::Nl,
            ReceiptLanguage::De,
        ] {
            for table in [Some(3), None] {
                for is_training in [false, true] {
                    let mut t = ticket(lang);
                    t.table = table;
                    t.is_training = is_training;
                    assert_eq!(format_bar_ticket(&t), legacy_bytes(&t));
                }
            }
        }
    }

    fn courses_ticket() -> BarTicket {
        let mut t = ticket(ReceiptLanguage::Fr);
        t.items = vec![
            BarItem {
                quantity: 2,
                name: "Sake".into(),
                course: Some(0),
            },
            BarItem {
                quantity: 1,
                name: "Salade".into(),
                course: Some(1),
            },
            BarItem {
                quantity: 3,
                name: "Maki".into(),
                course: Some(1),
            },
            BarItem {
                quantity: 1,
                name: "Mochi".into(),
                course: Some(3),
            },
        ];
        t
    }

    #[test]
    fn vi_course_a_header_is_printed_when_the_course_changes() {
        let out = text(&courses_ticket());
        let at = |needle: &str| {
            out.find(needle)
                .unwrap_or_else(|| panic!("{needle} in {out}"))
        };
        assert!(at("-- Direct --") < at("2 x Sake"));
        assert!(at("2 x Sake") < at("-- Entrees --"));
        assert!(at("-- Entrees --") < at("1 x Salade"));
        assert!(at("3 x Maki") < at("-- Desserts --"));
        assert_eq!(out.matches("-- Entrees --").count(), 1, "{out}");
        assert!(!out.contains("-- Plats --"), "{out}");
    }

    #[test]
    fn vi_course_a_release_slip_keeps_the_title_and_the_mention_and_says_which_course() {
        let mut t = courses_ticket();
        t.release = Some(2);
        let out = crate::render::one_line(&text(&t));
        for expected in [
            "BON DE COMMANDE",
            "Table 3",
            "SUITE : Plats",
            "CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE",
        ] {
            assert!(out.contains(expected), "{expected:?} missing from:\n{out}");
        }
        assert!(
            !out.contains('€') && !out.contains("EUR") && !out.contains(",00"),
            "{out}"
        );
    }

    #[test]
    fn vi_course_a_reprint_says_duplicata_right_under_the_title() {
        let mut t = courses_ticket();
        t.duplicate = true;
        let out = text(&t);
        let title = out.find("BON DE COMMANDE").unwrap();
        let duplicata = out.find("DUPLICATA").unwrap();
        let date = out.find("Date:").unwrap();
        assert!(title < duplicata && duplicata < date, "{out}");
        assert!(!text(&courses_ticket()).contains("DUPLICATA"));
    }

    #[test]
    fn vi_course_names_follow_the_slip_language() {
        let mut t = courses_ticket();
        t.release = Some(2);
        t.lang = ReceiptLanguage::Nl;
        let nl = text(&t);
        assert!(nl.contains("VOLGENDE : Hoofdgerechten"), "{nl}");
        assert!(nl.contains("-- Voorgerechten --"), "{nl}");
        t.lang = ReceiptLanguage::De;
        let de = text(&t);
        assert!(de.contains("FOLGEGANG : Hauptgerichte"), "{de}");
        assert!(de.contains("-- Vorspeisen --"), "{de}");
    }

    #[test]
    fn vi_course_lines_fit_the_paper() {
        let mut t = courses_ticket();
        t.release = Some(3);
        t.duplicate = true;
        t.lang = ReceiptLanguage::Nl;
        for line in text(&t).lines() {
            assert!(line.chars().count() <= 48, "too wide: {line:?}");
        }
    }
}
