//! Every ticket type (N sale, N refund, P, C, T) in every ticket
//! language (FR/NL/DE), rendered through the real `format_receipt`
//! pipeline: each must fit the 80 mm paper, carry the labels of its
//! language, and print the §VI mentions of its type - no more, no less.

use pos_fiscal::response::{FdmReference, VatCalcItem};
use pos_fiscal::ticket::{
    FiscalTicket, ReceiptLanguage, TicketLine, TicketPayment, invalid_disclaimer, ticket_type_label,
};
use pos_fiscal::types::{EventLabel, VatLabel};
use pos_print::receipt::format_receipt;
use pos_print::render::render_plain_text;

const WIDTH: usize = 48;
const LANGS: [ReceiptLanguage; 3] = [
    ReceiptLanguage::Fr,
    ReceiptLanguage::Nl,
    ReceiptLanguage::De,
];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Sale,
    Refund,
    ProForma,
    Copy,
    Training,
}

const KINDS: [Kind; 5] = [
    Kind::Sale,
    Kind::Refund,
    Kind::ProForma,
    Kind::Copy,
    Kind::Training,
];

impl Kind {
    fn label(self) -> EventLabel {
        match self {
            Kind::Sale | Kind::Refund => EventLabel::N,
            Kind::ProForma => EventLabel::P,
            Kind::Copy => EventLabel::C,
            Kind::Training => EventLabel::T,
        }
    }
}

fn line(qty: f64, desc: &str, unit: f64, vat: &str) -> TicketLine {
    TicketLine {
        quantity: qty,
        description: desc.to_string(),
        unit_price: unit,
        total: qty * unit,
        vat_code: vat.to_string(),
    }
}

fn ticket(kind: Kind, lang: ReceiptLanguage) -> FiscalTicket {
    let label = kind.label();
    let sign = if kind == Kind::Refund { -1.0 } else { 1.0 };
    let mut lines = vec![
        line(2.0, "Dry Martini", 12.00, "A"),
        line(
            1.0,
            "Bœuf bourguignon « maison » – sauce au vin",
            28.00,
            "B",
        ),
    ];
    for l in &mut lines {
        l.quantity *= sign;
        l.total *= sign;
    }
    let is_n = label == EventLabel::N;
    FiscalTicket {
        restaurant_name: "Restaurant Exemple".to_string(),
        restaurant_address: "Rue de l’Exemple 1, 4000 Liège".to_string(),
        restaurant_vat: "BE0499999960".to_string(),
        restaurant_est_no: "2123456791".to_string(),
        restaurant_legal_name: "Societe Exemple SRL".to_string(),
        pos_id: "CFOD0061234567".to_string(),
        pos_sw_version: "moyun-pos/1.0.0".to_string(),
        terminal_id: "POS1".to_string(),
        device_id: "00:1b:44:11:3a:b7".to_string(),
        operator: "Marie Dupont".to_string(),
        pos_datetime: "2026-07-03T12:34:56+02:00".to_string(),
        ticket_number: 15,
        event_label: label,
        is_training: kind == Kind::Training,
        lines,
        subtotal: 52.00 * sign,
        total: 52.00 * sign,
        vat_breakdown: if kind == Kind::ProForma {
            vec![]
        } else {
            vec![
                VatCalcItem {
                    label: VatLabel::A,
                    rate: 21.0,
                    taxable_amount: 19.83 * sign,
                    vat_amount: 4.17 * sign,
                    total_amount: 24.00 * sign,
                    out_of_scope: false,
                },
                VatCalcItem {
                    label: VatLabel::B,
                    rate: 12.0,
                    taxable_amount: 25.00 * sign,
                    vat_amount: 3.00 * sign,
                    total_amount: 28.00 * sign,
                    out_of_scope: false,
                },
            ]
        },
        payments: if kind == Kind::ProForma {
            vec![]
        } else {
            // The raw FDM financials.name ("Cash", see
            // pos_fiscal::event::build_sale_input): the printed label
            // is localized from this at print time (see
            // `payment_label` in pos_print::labels).
            vec![TicketPayment {
                method: "Cash".to_string(),
                amount: 52.00 * sign,
            }]
        },
        rounding: None,
        fdm_ref: Some(FdmReference {
            fdm_id: "CBX01000042".to_string(),
            fdm_date_time: "2026-07-03T12:34:57+02:00".to_string(),
            event_label: label,
            event_counter: 15,
            total_counter: 59,
        }),
        short_signature: is_n.then(|| "a3f9c2b1e8d4750616fb92c3a4e5d6f701234567".to_string()),
        verification_url: is_n.then(|| "https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug".to_string()),
        footer: vec!["www.systemedecaisseenregistreuse.be".to_string()],
        ticket_type_label: ticket_type_label(label, lang).to_string(),
        is_refund: kind == Kind::Refund,
        lang,
        original: None,
        copy_made_at: None,
    }
}

/// The language-specific rows every ticket carries.
fn expected_labels(lang: ReceiptLanguage) -> &'static [&'static str] {
    match lang {
        ReceiptLanguage::Fr => &[
            "TVA: BE0499999960",
            "Unite d'etablissement 2123456791",
            "Utilisateur:",
            "Ticketnr.:",
            "Logiciel de caisse v.:",
            "Denomination legale :",
            "TOTAL :",
        ],
        ReceiptLanguage::Nl => &[
            "BTW: BE0499999960",
            "Vestigingseenheid 2123456791",
            "Gebruiker:",
            "Ticketnr.:",
            "Kassasoftware v.:",
            "Juridische naam :",
            "TOTAAL :",
        ],
        ReceiptLanguage::De => &[
            "MwSt: BE0499999960",
            "Niederlassungseinheit 2123456791",
            "Benutzer:",
            "Bon-Nr.:",
            "Kassensoftware v.:",
            "Rechtlicher Name :",
            "GESAMT :",
        ],
    }
}

/// Labels of the other languages that must never leak onto a ticket.
fn foreign_labels(lang: ReceiptLanguage) -> Vec<&'static str> {
    LANGS
        .iter()
        .filter(|l| **l != lang)
        .flat_map(|l| expected_labels(*l).iter().copied())
        .filter(|w| !expected_labels(lang).contains(w))
        .collect()
}

fn render(kind: Kind, lang: ReceiptLanguage) -> String {
    render_plain_text(&format_receipt(&ticket(kind, lang)))
}

#[test]
fn every_ticket_fits_the_paper() {
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
fn every_ticket_is_in_its_language() {
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
// covers: FAQ_Q507
fn every_ticket_carries_its_type_label_and_identification() {
    for kind in KINDS {
        for lang in LANGS {
            let text = render(kind, lang);
            let type_label = ticket_type_label(kind.label(), lang);
            assert!(
                text.contains(type_label),
                "{kind:?}/{lang:?}: {type_label:?}"
            );
            // Rows I–VIII of the manual §VI ticket table.
            for row in [
                "Restaurant Exemple",
                "Rue de l'Exemple 1, 4000 Liège",
                "CFOD0061234567",
                "moyun-pos/1.0.0",
                "POS1",
                "00:1b:44:11:3a:b7",
                "Marie Dupont",
                "03.07.2026",
                "12:34",
                "000015",
                "Societe Exemple SRL",
                "fdmId: CBX01000042;",
                "www.systemedecaisseenregistreuse.be",
            ] {
                assert!(
                    text.contains(row),
                    "{kind:?}/{lang:?}: missing {row:?}\n{text}"
                );
            }
            for counters in [
                format!("Event: {:?}; eventCounter: 15;", kind.label()),
                "totalCounter: 59;".to_string(),
            ] {
                assert!(text.contains(&counters), "{kind:?}/{lang:?}: {counters:?}");
            }
        }
    }
}

#[test]
fn only_a_real_sale_is_a_valid_vat_ticket() {
    for kind in KINDS {
        for lang in LANGS {
            let text = render(kind, lang);
            let disclaimer = invalid_disclaimer(lang);
            let valid = matches!(kind, Kind::Sale | Kind::Refund);
            assert_eq!(
                !text.contains(disclaimer),
                valid,
                "{kind:?}/{lang:?}\n{text}"
            );
            assert_eq!(
                text.contains("[QR"),
                valid,
                "{kind:?}/{lang:?}: QR on N only"
            );
            assert_eq!(
                text.contains("shortSignature:"),
                valid,
                "{kind:?}/{lang:?}: signature on N only"
            );
            assert_eq!(
                text.contains("TRAINING"),
                kind == Kind::Training,
                "{kind:?}/{lang:?}"
            );
        }
    }
}

#[test]
fn refunds_are_negative_and_say_so() {
    for lang in LANGS {
        let text = render(Kind::Refund, lang);
        assert!(text.contains("-52,00"), "{lang:?}\n{text}");
        assert!(text.contains("-24,00  A"), "{lang:?}\n{text}");
        let refund = match lang {
            ReceiptLanguage::Fr => "REMBOURSEMENT",
            ReceiptLanguage::Nl => "TERUGBETALING",
            ReceiptLanguage::De => "RUECKERSTATTUNG",
        };
        assert!(text.lines().any(|l| l.trim() == refund), "{lang:?}\n{text}");
        let sale = render(Kind::Sale, lang);
        assert!(!sale.contains(refund), "{lang:?}: a sale is no refund");
        assert!(!sale.contains("-52,00"));
    }
}

#[test]
fn the_long_product_name_is_printed_whole() {
    for lang in LANGS {
        let text = render(Kind::Sale, lang);
        // "Bœuf bourguignon « maison » – sauce au vin" + "28,00 B" does
        // not fit one line: it wraps between words, amount on the last.
        let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            flat.contains("Boeuf bourguignon « maison » - sauce au vin 28,00 B"),
            "{lang:?}\n{text}"
        );
        assert!(
            !text.contains('?'),
            "{lang:?}: unprintable character\n{text}"
        );
    }
}

/// The printed cash-payment label per language (see
/// `pos_print::labels::payment_label`).
fn cash_label(lang: ReceiptLanguage) -> &'static str {
    match lang {
        ReceiptLanguage::Fr => "Especes",
        ReceiptLanguage::Nl => "Contant",
        ReceiptLanguage::De => "Bar",
    }
}

#[test]
fn a_pro_forma_shows_no_payment_or_vat_table() {
    for lang in LANGS {
        let text = render(Kind::ProForma, lang);
        assert!(!text.contains(cash_label(lang)), "{lang:?}");
        let vat_header = match lang {
            ReceiptLanguage::Fr => "% TVA",
            ReceiptLanguage::Nl => "% BTW",
            ReceiptLanguage::De => "% MwSt",
        };
        assert!(!text.contains(vat_header), "{lang:?}");
        assert!(render(Kind::Sale, lang).contains(vat_header), "{lang:?}");
    }
}

#[test]
fn the_payment_name_follows_the_ticket_language() {
    for lang in LANGS {
        let text = render(Kind::Sale, lang);
        assert!(
            text.contains(cash_label(lang)),
            "{lang:?}: missing {:?}\n{text}",
            cash_label(lang)
        );
        for other in LANGS.iter().filter(|l| **l != lang) {
            assert!(
                !text.contains(cash_label(*other)),
                "{lang:?}: leaked {:?} from {other:?}\n{text}",
                cash_label(*other)
            );
        }
    }
}

#[test]
fn a_restaurant_name_longer_than_the_paper_wraps() {
    for lang in LANGS {
        let mut t = ticket(Kind::Sale, lang);
        t.restaurant_name = "Le Grand Restaurant Gastronomique de la Place Saint-Lambert".into();
        t.restaurant_address =
            "Boulevard de la Sauvenière 123 boîte 4, 4000 Liège, Belgique".into();
        let text = render_plain_text(&format_receipt(&t));
        assert!(text.lines().all(|l| l.chars().count() <= WIDTH), "{text}");
        let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(flat.contains("Le Grand Restaurant Gastronomique de la Place Saint-Lambert"));
        assert!(flat.contains("Boulevard de la Sauvenière 123 boîte 4, 4000 Liège, Belgique"));
    }
}
