//! Bar / kitchen order slip for table orders and direct sales.
//!
//! Layout of the SPF's example bar ticket (2026-09-24): date, operator,
//! table, then "quantity x article" lines. A bar ticket never shows a
//! price and carries the "not a VAT ticket" mention. The webshop's
//! kitchen slip (`kitchen_ticket.rs`) is a different, non-certified
//! layout.

use pos_fiscal::ticket::{ReceiptLanguage, invalid_disclaimer};

use crate::escpos::{Alignment, EscPosBuilder, PaperWidth, TextStyle, wrap};
use crate::labels::bar_labels;

/// One line sent to the bar or kitchen: what was just ordered, or a
/// correction (negative quantity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarItem {
    pub quantity: i32,
    pub name: String,
    /// The line's note for the kitchen ("sans wasabi"), printed under
    /// the item. Never an amount, never part of a fiscal record.
    pub note: Option<String>,
    /// Course of the line (kitchen only): `Some` when the slip is
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
pub fn format_bar_ticket(ticket: &BarTicket, paper: PaperWidth) -> Vec<u8> {
    let l = bar_labels(ticket.lang);
    let mut p = EscPosBuilder::for_paper(paper);

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
        if let Some(note) = &item.note {
            indented_note(&mut p, "   > ", note);
        }
    }
    p.separator('-');

    p.align(Alignment::Center);
    p.textln_wrapped(invalid_disclaimer(ticket.lang));
    p.feed(4);
    p.cut();
    p.build()
}

/// Print `note` after `prefix` on the first line, wrapped lines
/// aligned under it.
fn indented_note(p: &mut EscPosBuilder, prefix: &str, note: &str) {
    let width = usize::from(p.chars_per_line()).saturating_sub(prefix.chars().count());
    let pad = " ".repeat(prefix.chars().count());
    for (i, line) in wrap(note, width.max(1)).iter().enumerate() {
        let lead = if i == 0 { prefix } else { pad.as_str() };
        p.textln(&format!("{lead}{line}"));
    }
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
pub fn format_transfer_slip(slip: &TransferSlip, paper: PaperWidth) -> Vec<u8> {
    let l = bar_labels(slip.lang);
    let mut p = EscPosBuilder::for_paper(paper);

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

/// A kitchen note written, changed or removed after its line went to
/// the kitchen. Not an order (no quantity, nothing new to prepare), not
/// a VAT ticket (no amount, the usual mention).
#[derive(Debug, Clone)]
pub struct NoteSlip {
    /// Local date and time, already formatted.
    pub date_time: String,
    pub operator: String,
    /// Table number; `None` for a direct sale.
    pub table: Option<i32>,
    pub product_name: String,
    /// The new note; `None` when it was removed.
    pub note: Option<String>,
    pub lang: ReceiptLanguage,
    pub is_training: bool,
}

/// ESC/POS bytes of the note change slip.
pub fn format_note_slip(slip: &NoteSlip, paper: PaperWidth) -> Vec<u8> {
    let l = bar_labels(slip.lang);
    let mut p = EscPosBuilder::for_paper(paper);

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
    p.textln(l.note_title);
    p.style_reset();
    p.separator('=');

    p.align(Alignment::Left);
    p.text_columns(l.date, &slip.date_time);
    p.text_columns(l.operator, &slip.operator);
    let table = match slip.table {
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

    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln_wrapped(&slip.product_name);
    p.style_reset();
    match &slip.note {
        Some(note) => indented_note(&mut p, &format!("{} ", l.note), note),
        None => {
            p.textln(l.note_removed);
        }
    }
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
                    note: None,
                    course: None,
                },
                BarItem {
                    quantity: 1,
                    name: "Plat du jour".into(),
                    note: None,
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
        render_plain_text(&format_bar_ticket(t, PaperWidth::Mm80))
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
            note: None,
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
    fn every_slip_fits_58_mm_paper() {
        for lang in [
            ReceiptLanguage::Fr,
            ReceiptLanguage::Nl,
            ReceiptLanguage::De,
        ] {
            crate::render::assert_58_mm_prints_the_80_mm_words(|paper| {
                format_bar_ticket(&ticket(lang), paper)
            });
            crate::render::assert_58_mm_prints_the_80_mm_words(|paper| {
                format_transfer_slip(&transfer(lang), paper)
            });
            crate::render::assert_58_mm_prints_the_80_mm_words(|paper| {
                format_note_slip(&note_slip(lang, Some("Sans wasabi, sauce à part")), paper)
            });
        }
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
        let out = crate::render::one_line(&render_plain_text(&format_transfer_slip(
            &transfer(ReceiptLanguage::Fr),
            PaperWidth::Mm80,
        )));
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
        let nl = render_plain_text(&format_transfer_slip(
            &transfer(ReceiptLanguage::Nl),
            PaperWidth::Mm80,
        ));
        assert!(
            nl.contains("TAFELVERPLAATSING") && nl.contains("Tafel 3 -> Tafel 12"),
            "{nl}"
        );
        let mut t = transfer(ReceiptLanguage::De);
        t.is_training = true;
        let de = render_plain_text(&format_transfer_slip(&t, PaperWidth::Mm80));
        assert!(
            de.contains("TISCHWECHSEL") && de.contains("Tisch 3 -> Tisch 12"),
            "{de}"
        );
        assert!(de.contains("TRAINING"), "{de}");
        for line in de.lines() {
            assert!(line.chars().count() <= 48, "too wide: {line:?}");
        }
    }

    #[test]
    fn vi_bar_an_item_note_prints_indented_under_its_item() {
        let mut t = ticket(ReceiptLanguage::Fr);
        t.items[1].note = Some("sans wasabi".into());
        let out = text(&t);
        let lines: Vec<&str> = out.lines().map(str::trim_end).collect();
        let at = lines
            .iter()
            .position(|l| l.contains("1 x Plat du jour"))
            .unwrap();
        assert_eq!(lines[at + 1], "   > sans wasabi", "{out}");
        // The other item has no note line.
        let martini = lines
            .iter()
            .position(|l| l.contains("Dry Martini"))
            .unwrap();
        assert!(!lines[martini + 1].contains('>'), "{out}");
        assert!(!out.contains('€') && !out.contains(",00"), "{out}");
    }

    #[test]
    fn vi_bar_a_long_note_wraps_under_the_first_word() {
        let mut t = ticket(ReceiptLanguage::Fr);
        t.items[0].note =
            Some("allergie arachide et fruits a coque, bien separer les ustensiles".into());
        let out = text(&t);
        for line in out.lines() {
            assert!(line.chars().count() <= 48, "too wide: {line:?}");
        }
        let lines: Vec<&str> = out.lines().collect();
        let at = lines
            .iter()
            .position(|l| l.contains("   > allergie"))
            .unwrap();
        assert!(lines[at + 1].starts_with("     "), "{out}");
    }

    fn note_slip(lang: ReceiptLanguage, note: Option<&str>) -> NoteSlip {
        NoteSlip {
            date_time: "15/06/2026 12:09:35".into(),
            operator: "Beheerder".into(),
            table: Some(3),
            product_name: "Plat du jour".into(),
            note: note.map(str::to_string),
            lang,
            is_training: false,
        }
    }

    #[test]
    fn vi_bar_the_note_slip_names_the_item_and_the_note_without_quantity_or_amount() {
        let out = text_of(&note_slip(ReceiptLanguage::Fr, Some("sans wasabi")));
        for expected in [
            "MODIFICATION",
            "15/06/2026 12:09:35",
            "Beheerder",
            "Table 3",
            "Plat du jour",
            "Note : sans wasabi",
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

    fn text_of(slip: &NoteSlip) -> String {
        crate::render::one_line(&render_plain_text(&format_note_slip(
            slip,
            PaperWidth::Mm80,
        )))
    }

    #[test]
    fn vi_bar_a_removed_note_says_so() {
        let out = text_of(&note_slip(ReceiptLanguage::Fr, None));
        assert!(out.contains("Note supprimee"), "{out}");
        assert!(!out.contains("Note :"), "{out}");
    }

    #[test]
    fn vi_bar_the_note_slip_follows_the_ticket_language_and_training() {
        let nl = text_of(&note_slip(ReceiptLanguage::Nl, Some("zonder wasabi")));
        assert!(
            nl.contains("WIJZIGING") && nl.contains("Opmerking: zonder wasabi"),
            "{nl}"
        );
        assert!(nl.contains("Tafel 3"), "{nl}");
        let mut s = note_slip(ReceiptLanguage::De, None);
        s.table = None;
        s.is_training = true;
        let de = text_of(&s);
        assert!(
            de.contains("AENDERUNG") && de.contains("Hinweis geloescht"),
            "{de}"
        );
        assert!(
            de.contains("Direktverkauf") && de.contains("TRAINING"),
            "{de}"
        );
        for line in render_plain_text(&format_note_slip(&s, PaperWidth::Mm80)).lines() {
            assert!(line.chars().count() <= 48, "too wide: {line:?}");
        }
    }

    #[test]
    fn vi_bar_a_note_prints_in_cp858_not_as_question_marks() {
        let bytes = format_note_slip(
            &note_slip(ReceiptLanguage::Fr, Some("allergie: crème, œuf, café")),
            PaperWidth::Mm80,
        );
        let out = render_plain_text(&bytes);
        assert!(
            out.contains("oeuf") && out.contains("crème") && out.contains("café"),
            "{out}"
        );
        assert!(!out.contains('?'), "{out}");
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
                    assert_eq!(format_bar_ticket(&t, PaperWidth::Mm80), legacy_bytes(&t));
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
                note: None,
            },
            BarItem {
                quantity: 1,
                name: "Salade".into(),
                course: Some(1),
                note: None,
            },
            BarItem {
                quantity: 3,
                name: "Maki".into(),
                course: Some(1),
                note: None,
            },
            BarItem {
                quantity: 1,
                name: "Mochi".into(),
                course: Some(3),
                note: None,
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
