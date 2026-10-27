//! Every report type (X, Z, user X, user Z) in every ticket language
//! (FR/NL/DE), rendered through the real `format_report` /
//! `format_user_report` pipeline: each must fit the 80 mm paper,
//! carry the labels, event names and payment names of its language -
//! no more, no less. Modeled on `receipt_matrix.rs`.

use pos_fiscal::ticket::{ReceiptLanguage, invalid_disclaimer, ticket_type_label};
use pos_fiscal::types::EventLabel;
use pos_print::render::render_plain_text;
use pos_print::report::{
    ReportEventTotal, ReportPaymentTotal, ReportPrintData, ReportVatTotal, UserReportPrintData,
    UserReportUser, format_report, format_user_report,
};

const WIDTH: usize = 48;
const LANGS: [ReceiptLanguage; 3] = [
    ReceiptLanguage::Fr,
    ReceiptLanguage::Nl,
    ReceiptLanguage::De,
];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    X,
    Z,
    UserX,
    UserZ,
}

const KINDS: [Kind; 4] = [Kind::X, Kind::Z, Kind::UserX, Kind::UserZ];

impl Kind {
    fn report_type(self) -> &'static str {
        match self {
            Kind::X | Kind::UserX => "X",
            Kind::Z | Kind::UserZ => "Z",
        }
    }

    fn is_user(self) -> bool {
        matches!(self, Kind::UserX | Kind::UserZ)
    }

    fn is_z(self) -> bool {
        matches!(self, Kind::Z | Kind::UserZ)
    }
}

fn report_data(kind: Kind, lang: ReceiptLanguage) -> ReportPrintData {
    ReportPrintData {
        report_type: kind.report_type().to_string(),
        report_no: kind.is_z().then_some(7),
        restaurant_name: "Restaurant Exemple".to_string(),
        restaurant_address: "Rue de l'Exemple 1, 4000 Liège".to_string(),
        restaurant_vat: "BE0499999960".to_string(),
        pos_id: "CFOD0061234567".to_string(),
        terminal_id: "POS1".to_string(),
        report_date: "2026-07-03".to_string(),
        first_datetime: "2026-07-03T11:02:14+02:00".to_string(),
        last_datetime: "2026-07-03T22:41:09+02:00".to_string(),
        employee_insz: "85073003328".to_string(),
        is_training: false,
        event_totals: vec![
            ReportEventTotal {
                label: "N".to_string(),
                count: 42,
                amount_cents: 152_350,
            },
            ReportEventTotal {
                label: "P".to_string(),
                count: 12,
                amount_cents: 0,
            },
            ReportEventTotal {
                label: "T".to_string(),
                count: 1,
                amount_cents: 0,
            },
        ],
        vat_totals: vec![
            ReportVatTotal {
                code: "A".to_string(),
                rate: 21.0,
                total_incl_cents: 38_450,
                vat_cents: 6_672,
            },
            ReportVatTotal {
                code: "B".to_string(),
                rate: 12.0,
                total_incl_cents: 113_900,
                vat_cents: 12_204,
            },
        ],
        // Raw FDM financials.name ("Cash"/"Card" - see
        // pos_fiscal::event::build_sale_input): the printed label is
        // localized from this at print time (`payment_label`).
        payment_totals: vec![
            ReportPaymentTotal {
                method: "Cash".to_string(),
                count: 18,
                amount_cents: 61_200,
            },
            ReportPaymentTotal {
                method: "Card".to_string(),
                count: 24,
                amount_cents: 91_150,
            },
        ],
        grand_total_cents: 152_350,
        drawer_opens: 3,
        fdm_id: Some("CBX01000042".to_string()),
        fdm_datetime: Some("2026-07-03T22:42:00+02:00".to_string()),
        short_signature: None,
        ticket_number: 3,
        lang,
        details: pos_print::report::ReportDetails::default(),
    }
}

fn user_report_data(kind: Kind, lang: ReceiptLanguage) -> UserReportPrintData {
    UserReportPrintData {
        report_type: kind.report_type().to_string(),
        report_no: kind.is_z().then_some(7),
        restaurant_name: "Restaurant Exemple".to_string(),
        restaurant_address: "Rue de l'Exemple 1, 4000 Liège".to_string(),
        restaurant_vat: "BE0499999960".to_string(),
        terminal_id: "POS1".to_string(),
        report_date: "2026-07-03".to_string(),
        employee_insz: "85073003328".to_string(),
        is_training: false,
        users: vec![UserReportUser {
            insz: "12345678901".to_string(),
            first_datetime: "2026-07-03T11:02:14+02:00".to_string(),
            last_datetime: "2026-07-03T22:41:09+02:00".to_string(),
            total_cents: 61_200,
            payments: vec![
                ReportPaymentTotal {
                    method: "Cash".to_string(),
                    count: 5,
                    amount_cents: 30_000,
                },
                ReportPaymentTotal {
                    method: "Card".to_string(),
                    count: 3,
                    amount_cents: 31_200,
                },
            ],
            social_events: vec![
                (
                    "WORK_IN".to_string(),
                    "2026-07-03T11:00:00+02:00".to_string(),
                ),
                (
                    "WORK_OUT".to_string(),
                    "2026-07-03T22:45:00+02:00".to_string(),
                ),
            ],
        }],
        fdm_id: Some("CBX01000042".to_string()),
        fdm_datetime: Some("2026-07-03T22:42:00+02:00".to_string()),
        short_signature: None,
        ticket_number: 4,
        lang,
    }
}

fn render(kind: Kind, lang: ReceiptLanguage) -> String {
    if kind.is_user() {
        render_plain_text(&format_user_report(&user_report_data(kind, lang)))
    } else {
        render_plain_text(&format_report(&report_data(kind, lang)))
    }
}

/// Labels every report carries in this language, whatever its kind
/// (X, Z, user X or user Z): shared metadata words + the localized
/// cash/card payment names.
fn expected_labels(lang: ReceiptLanguage) -> &'static [&'static str] {
    match lang {
        ReceiptLanguage::Fr => &[
            "TVA: BE0499999960",
            "Employe:",
            "Ticket:",
            "Especes",
            "Carte",
        ],
        ReceiptLanguage::Nl => &[
            "BTW: BE0499999960",
            "Werknemer:",
            "Ticket:",
            "Contant",
            "Kaart",
        ],
        ReceiptLanguage::De => &["MwSt: BE0499999960", "Mitarbeiter:", "Bon:", "Bar", "Karte"],
    }
}

/// Labels of the other languages that must never leak onto a report.
fn foreign_labels(lang: ReceiptLanguage) -> Vec<&'static str> {
    LANGS
        .iter()
        .filter(|l| **l != lang)
        .flat_map(|l| expected_labels(*l).iter().copied())
        .filter(|w| !expected_labels(lang).contains(w))
        .collect()
}

#[test]
fn every_report_fits_the_paper() {
    for kind in KINDS {
        for lang in LANGS {
            let text = render(kind, lang);
            for l in text.lines() {
                assert!(
                    l.chars().count() <= WIDTH,
                    "{kind:?}/{lang:?}: {} columns: {l:?}",
                    l.chars().count()
                );
            }
        }
    }
}

#[test]
fn every_report_is_in_its_language() {
    for kind in KINDS {
        for lang in LANGS {
            let text = render(kind, lang);
            for label in expected_labels(lang) {
                assert!(
                    text.contains(label),
                    "{kind:?}/{lang:?}: missing {label:?}\n{text}"
                );
            }
            for label in foreign_labels(lang) {
                assert!(
                    !text.contains(label),
                    "{kind:?}/{lang:?}: foreign {label:?}\n{text}"
                );
            }
        }
    }
}

#[test]
fn the_turnover_report_title_follows_the_ticket_language() {
    for kind in [Kind::X, Kind::Z] {
        for lang in LANGS {
            let text = render(kind, lang);
            let word = ticket_type_label(EventLabel::R, lang);
            assert!(
                text.contains(word),
                "{kind:?}/{lang:?}: missing report title word {word:?}\n{text}"
            );
        }
    }
}

#[test]
fn event_type_names_follow_the_ticket_language() {
    fn expected(lang: ReceiptLanguage) -> [&'static str; 3] {
        match lang {
            ReceiptLanguage::Fr => ["Ventes", "Pro forma", "Formation"],
            ReceiptLanguage::Nl => ["Verkoop", "Pro forma", "Opleiding"],
            ReceiptLanguage::De => ["Umsaetze", "Pro forma", "Schulung"],
        }
    }
    for kind in [Kind::X, Kind::Z] {
        for lang in LANGS {
            let text = render(kind, lang);
            for word in expected(lang) {
                assert!(
                    text.contains(word),
                    "{kind:?}/{lang:?}: missing event name {word:?}\n{text}"
                );
            }
        }
    }
}

#[test]
fn the_turnover_report_follows_article_29() {
    fn words(lang: ReceiptLanguage) -> [&'static str; 9] {
        match lang {
            ReceiptLanguage::Fr => [
                "CHIFFRE D'AFFAIRES",
                "Periode d'ouverture",
                "Apercu TVA",
                "Corrections",
                "Remboursements",
                "Tiroir-caisse",
                "Tickets de formation",
                "Reductions de lignes",
                "Total des paiements",
            ],
            ReceiptLanguage::Nl => [
                "OMZET",
                "Openingsperiode",
                "BTW-overzicht",
                "Correcties",
                "Terugbetalingen",
                "Kassalade",
                "Opleidingstickets",
                "Lijnkortingen",
                "Totaal betalingen",
            ],
            ReceiptLanguage::De => [
                "UMSATZ",
                "Oeffnungszeitraum",
                "MwSt-Uebersicht",
                "Korrekturen",
                "Rueckerstattungen",
                "Kassenschublade",
                "Schulungsbons",
                "Zeilenrabatte",
                "Summe Zahlungen",
            ],
        }
    }
    for kind in [Kind::X, Kind::Z] {
        for lang in LANGS {
            let text = render(kind, lang);
            for word in words(lang) {
                assert!(
                    text.contains(word),
                    "{kind:?}/{lang:?}: missing {word:?}\n{text}"
                );
            }
            // Every rate is listed, even without turnover.
            for rate in ["21%", "12%", "6%", "0%"] {
                assert!(
                    text.lines().any(|l| l.starts_with(rate)),
                    "{kind:?}/{lang:?}: no {rate} row\n{text}"
                );
            }
            assert!(
                pos_print::render::one_line(&text).contains(invalid_disclaimer(lang)),
                "{kind:?}/{lang:?}: a report is not a VAT ticket\n{text}"
            );
        }
    }
}

#[test]
fn only_the_z_report_is_numbered() {
    for lang in LANGS {
        let z = render(Kind::Z, lang);
        let x = render(Kind::X, lang);
        let no = match lang {
            ReceiptLanguage::De => "Bericht Nr.",
            _ => "Rapport nr.",
        };
        assert!(z.contains(no), "{lang:?}\n{z}");
        assert!(!x.contains(no), "{lang:?}\n{x}");
        let title = |k: &str| format!("{k} {}", z_word(lang));
        assert!(z.contains(&title("Z")), "{lang:?}\n{z}");
        assert!(x.contains(&title("X")), "{lang:?}\n{x}");
    }
}

fn z_word(lang: ReceiptLanguage) -> &'static str {
    match lang {
        ReceiptLanguage::Fr => "CHIFFRE D'AFFAIRES",
        ReceiptLanguage::Nl => "OMZET",
        ReceiptLanguage::De => "UMSATZ",
    }
}

#[test]
fn the_user_report_title_and_punch_labels_follow_the_ticket_language() {
    fn prefix(lang: ReceiptLanguage) -> &'static str {
        match lang {
            ReceiptLanguage::Fr => "RAPPORT UTIL.",
            ReceiptLanguage::Nl => "RAPPORT GEBR.",
            ReceiptLanguage::De => "BERICHT BENUTZER",
        }
    }
    fn punch(lang: ReceiptLanguage) -> (&'static str, &'static str) {
        match lang {
            ReceiptLanguage::Fr => ("Pointage IN:", "Pointage OUT:"),
            ReceiptLanguage::Nl => ("Klok IN:", "Klok OUT:"),
            ReceiptLanguage::De => ("Zeit IN:", "Zeit OUT:"),
        }
    }
    for kind in [Kind::UserX, Kind::UserZ] {
        for lang in LANGS {
            let text = render(kind, lang);
            assert!(
                text.contains(prefix(lang)),
                "{kind:?}/{lang:?}: missing user-report title {:?}\n{text}",
                prefix(lang)
            );
            let (in_label, out_label) = punch(lang);
            assert!(
                text.contains(in_label),
                "{kind:?}/{lang:?}: missing {in_label:?}\n{text}"
            );
            assert!(
                text.contains(out_label),
                "{kind:?}/{lang:?}: missing {out_label:?}\n{text}"
            );
        }
    }
}

#[test]
fn the_insz_number_is_never_translated() {
    // "INSZ" is the same official abbreviation in all three national
    // languages: it must print unchanged, unlike every other label.
    for kind in [Kind::UserX, Kind::UserZ] {
        for lang in LANGS {
            let text = render(kind, lang);
            assert!(
                text.contains("INSZ 12345678901"),
                "{kind:?}/{lang:?}\n{text}"
            );
        }
    }
}
