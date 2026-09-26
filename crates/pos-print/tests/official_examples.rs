//! Golden tests of the six SPF example tickets of 2026-09-24
//! (`docs/20260904_Example_tickets/`): the example's own data goes
//! through the real print pipeline, and every mention the example
//! labels (AM of 29 April 2024 art. 20 for tickets, art. 29 for the Z
//! report) must be printed, in the example's order.
//!
//! Known wording differences, asked to the SPF (plan section 3): the
//! examples write "TICKET DE CAISSE T.V.A.", "PROFORMA" and "... T.V.A.
//! VALIDE", the POS description v2.2 "TICKET DE CAISSE TVA", "PRO
//! FORMA" and "... TVA VALABLE"; we print the latter. Fixed labels are
//! ASCII ("Unite d'etablissement").

use pos_fiscal::response::{FdmReference, VatCalcItem};
use pos_fiscal::ticket::{
    FiscalTicket, OriginalControlData, ReceiptLanguage, TicketLine, TicketPayment,
    ticket_type_label,
};
use pos_fiscal::types::{EventLabel, VatLabel};
use pos_print::bar_ticket::{BarItem, BarTicket, format_bar_ticket};
use pos_print::receipt::format_receipt;
use pos_print::render::render_plain_text;
use pos_print::report::{
    ReportDetails, ReportEventTotal, ReportPaymentTotal, ReportPrintData, ReportVatTotal,
    format_report,
};

const SIGNATURE: &str = "9b31ef04a7c2119d0053f6a8b7c04e6d1a2f8b91";

/// The printed text, each line's words joined by one space.
fn normalized(bytes: &[u8]) -> String {
    render_plain_text(bytes)
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `(mention, text)` is printed, after the one before it.
fn assert_in_order(text: &str, mentions: &[(&str, &str)]) {
    let mut from = 0;
    for (mention, needle) in mentions {
        match text[from..].find(needle) {
            Some(at) => from += at + needle.len(),
            None => panic!(
                "mention {mention}: {needle:?} missing or out of order in:\n{text}\n\
                 (searched after byte {from})"
            ),
        }
    }
}

fn fdm_ref(label: EventLabel, event_counter: i64, total_counter: i64, at: &str) -> FdmReference {
    FdmReference {
        fdm_id: "FDM01122334".to_string(),
        fdm_date_time: at.to_string(),
        event_label: label,
        event_counter,
        total_counter,
    }
}

/// The example VAT ticket: one Cola at 3,04 paid cash, rounded by 0,01.
fn vat_ticket() -> FiscalTicket {
    FiscalTicket {
        restaurant_name: "Café North Galaxy".to_string(),
        restaurant_address: "Koning Albert II-laan 33, 1030 Schaarbeek (Brussel)".to_string(),
        restaurant_vat: "BE0123456789".to_string(),
        restaurant_est_no: "2000000042".to_string(),
        restaurant_legal_name: "Café North Galaxy BV".to_string(),
        pos_id: "CAAA0099887766".to_string(),
        pos_sw_version: "4.7.2".to_string(),
        terminal_id: "KASSA-1".to_string(),
        device_id: "3C:5A:B4:11:9F:02".to_string(),
        operator: "kassier03".to_string(),
        pos_datetime: "2026-06-23T11:35:18+02:00".to_string(),
        ticket_number: 433,
        event_label: EventLabel::N,
        is_training: false,
        lines: vec![TicketLine {
            quantity: 1.0,
            description: "Cola".to_string(),
            unit_price: 3.04,
            total: 3.04,
            vat_code: "A".to_string(),
        }],
        subtotal: 3.04,
        total: 3.05,
        vat_breakdown: vec![VatCalcItem {
            label: VatLabel::A,
            rate: 21.0,
            taxable_amount: 2.51,
            vat_amount: 0.53,
            total_amount: 3.04,
            out_of_scope: false,
        }],
        payments: vec![TicketPayment {
            method: "Cash".to_string(),
            amount: 3.04,
        }],
        rounding: Some(0.01),
        fdm_ref: Some(fdm_ref(EventLabel::N, 214, 1587, "2026-06-23T11:35:18.472")),
        short_signature: Some(SIGNATURE.to_string()),
        verification_url: Some("https://www.gs2.be/Xk3Qa7Pb2Lc4Rd8Se3Tf5Ug".to_string()),
        footer: vec![],
        ticket_type_label: ticket_type_label(EventLabel::N, ReceiptLanguage::Fr).to_string(),
        is_refund: false,
        lang: ReceiptLanguage::Fr,
        original: None,
        copy_made_at: None,
    }
}

/// The identification rows of every example ticket (art. 20 c to j).
fn identification(ticket_no: &'static str) -> Vec<(&'static str, &'static str)> {
    vec![
        ("c", "Café North Galaxy"),
        ("d", "Unite d'etablissement 2000000042"),
        ("i", "23.06.2026 11:35"),
        ("h", "Utilisateur: kassier03"),
        ("f", "Terminal: KASSA-1"),
        ("j", ticket_no),
        ("e", "POS Serial: CAAA0099887766"),
        ("g", "Input device MAC: 3C:5A:B4:11:9F:02"),
        ("e", "Logiciel de caisse v.: 4.7.2"),
    ]
}

/// The issuer's legal identity at the bottom (art. 20 b). The address
/// is longer than the paper is wide: it wraps between words.
const LEGAL: [(&str, &str); 4] = [
    ("b", "Denomination legale : Café North Galaxy BV"),
    ("b", "Koning Albert II-laan 33,"),
    ("b", "(Brussel)"),
    ("b", "TVA: BE0123456789"),
];

#[test]
fn vi_n_the_official_vat_ticket_example() {
    let text = normalized(&format_receipt(&vat_ticket()));
    let mut mentions = vec![("a", "TICKET DE CAISSE TVA")];
    mentions.extend(identification("Ticketnr.: 000433"));
    mentions.extend([
        ("k", "1 Cola 3,04 A"),
        ("l", "TOTAL : €3,04"),
        ("p", "Mode de paiement :"),
        ("p", "Especes 3,04"),
        ("p", "Arrondi 0,01"),
        ("m", "% TVA CODE HT TVA Brut"),
        ("m n", "21,00% A 2,51 0,53 3,04"),
        ("o", "INFORMATIONS FDM"),
        ("o", "fdmId: FDM01122334;"),
        ("o", "fdmDateTime: 2026-06-23T11:35:18.472;"),
        ("o", "Event: N; eventCounter: 214;"),
        ("o", "totalCounter: 1587; shortSignature:"),
        ("o", "9b31ef04a7c2119d0053f6a8b7c04e6d1a2f8b91;"),
        ("q", "[QR: https://www.gs2.be/Xk3Qa7Pb2Lc4Rd8Se3Tf5Ug]"),
    ]);
    mentions.extend(LEGAL);
    assert_in_order(&text, &mentions);
    assert!(!text.contains("CECI N'EST PAS"), "{text}");
}

#[test]
fn vi_copy_the_official_copy_example() {
    let mut ticket = vat_ticket();
    ticket.ticket_number = 434;
    // The copy event C 216/1590 of the original N 214/1587.
    ticket.original = Some(OriginalControlData {
        fdm_ref: ticket.fdm_ref.take().unwrap(),
        short_signature: ticket.short_signature.take(),
    });
    ticket.fdm_ref = Some(fdm_ref(EventLabel::C, 216, 1590, "2026-06-23T11:35:19.472"));
    ticket.short_signature = Some(SIGNATURE.to_string());
    ticket.copy_made_at = Some("2026-06-23T11:36:02+02:00".to_string());
    let text = normalized(&format_receipt(&ticket));

    let mut mentions = vec![("A", "TICKET DE CAISSE TVA"), ("A", "COPIE")];
    mentions.extend(identification("Ticketnr.: 000434"));
    mentions.extend([
        ("B", "1 Cola 3,04 A"),
        ("B", "TOTAL : €3,04"),
        ("B", "Especes 3,04"),
        ("B", "Arrondi 0,01"),
        ("B", "21,00% A 2,51 0,53 3,04"),
        ("C", "INFORMATIONS FDM"),
        ("C", "fdmDateTime: 2026-06-23T11:35:19.472;"),
        ("C", "Event: C; eventCounter: 216;"),
        ("C", "totalCounter: 1590;"),
        ("B", "[QR: https://www.gs2.be/Xk3Qa7Pb2Lc4Rd8Se3Tf5Ug]"),
        ("C", "Informations FDM d'origine :"),
        ("C", "fdmDateTime: 2026-06-23T11:35:18.472;"),
        ("C", "Event: N; eventCounter: 214;"),
        ("C", "totalCounter: 1587;"),
    ]);
    mentions.extend(LEGAL);
    assert_in_order(&text, &mentions);
    // A copy of a VAT ticket is not declared invalid (§1.8).
    assert!(!text.contains("CECI N'EST PAS"), "{text}");
}

#[test]
fn vi_refund_the_official_refund_example() {
    let mut ticket = vat_ticket();
    ticket.ticket_number = 435;
    ticket.is_refund = true;
    ticket.lines[0].quantity = 1.0;
    ticket.lines[0].total = -3.04;
    ticket.subtotal = -3.04;
    ticket.total = -3.05;
    ticket.payments[0].amount = -3.04;
    ticket.rounding = Some(-0.01);
    let vat = &mut ticket.vat_breakdown[0];
    vat.taxable_amount = -2.51;
    vat.vat_amount = -0.53;
    vat.total_amount = -3.04;
    ticket.original = Some(OriginalControlData {
        fdm_ref: ticket.fdm_ref.take().unwrap(),
        short_signature: None,
    });
    ticket.fdm_ref = Some(fdm_ref(EventLabel::N, 215, 1588, "2026-06-23T11:35:19.472"));
    let text = normalized(&format_receipt(&ticket));

    let mut mentions = vec![("A", "TICKET DE CAISSE TVA"), ("B", "REMBOURSEMENT")];
    mentions.extend(identification("Ticketnr.: 000435"));
    mentions.extend([
        ("A", "1 Cola -3,04 A"),
        ("A", "TOTAL : €-3,04"),
        ("A", "Especes -3,04"),
        ("A", "Arrondi -0,01"),
        ("A", "21,00% A -2,51 -0,53 -3,04"),
        ("A", "INFORMATIONS FDM"),
        ("A", "Event: N; eventCounter: 215;"),
        ("A", "totalCounter: 1588; shortSignature:"),
        ("A", "[QR: https://www.gs2.be/Xk3Qa7Pb2Lc4Rd8Se3Tf5Ug]"),
        ("C", "Informations FDM d'origine :"),
        ("C", "fdmId: FDM01122334;"),
        ("C", "fdmDateTime: 2026-06-23T11:35:18.472;"),
        ("C", "Event: N; eventCounter: 214;"),
        ("C", "totalCounter: 1587;"),
    ]);
    mentions.extend(LEGAL);
    assert_in_order(&text, &mentions);
    assert_eq!(text.matches("REMBOURSEMENT").count(), 1, "{text}");
}

#[test]
fn vi_proforma_the_official_pre_bill_example() {
    let mut ticket = vat_ticket();
    ticket.event_label = EventLabel::P;
    ticket.ticket_type_label = ticket_type_label(EventLabel::P, ReceiptLanguage::Fr).to_string();
    ticket.ticket_number = 437;
    ticket.total = 3.04;
    ticket.payments.clear();
    ticket.rounding = None;
    ticket.vat_breakdown.clear();
    ticket.short_signature = None;
    ticket.verification_url = None;
    ticket.fdm_ref = Some(fdm_ref(EventLabel::P, 230, 1601, "2026-06-23T11:35:19.472"));
    let text = normalized(&format_receipt(&ticket));

    let mut mentions = vec![("A", "PRO FORMA"), ("A", "Addition Provisoire")];
    mentions.extend(identification("Ticketnr.: 000437"));
    mentions.extend([
        ("C", "1 Cola 3,04 A"),
        ("C", "TOTAL : €3,04"),
        ("", "INFORMATIONS FDM"),
        ("", "fdmId: FDM01122334;"),
        ("", "fdmDateTime: 2026-06-23T11:35:19.472;"),
        ("", "Event: P; eventCounter: 230;"),
        ("", "totalCounter: 1601;"),
    ]);
    mentions.extend(LEGAL);
    mentions.push(("B", "CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE"));
    assert_in_order(&text, &mentions);
    // D: no VAT breakdown on a pre-bill; no payment, no QR code.
    assert!(!text.contains("% TVA"), "{text}");
    assert!(!text.contains("Mode de paiement"), "{text}");
    assert!(!text.contains("[QR"), "{text}");
}

#[test]
fn vi_bar_the_official_bar_ticket_example() {
    let text = normalized(&format_bar_ticket(&BarTicket {
        date_time: "15/06/2026 12:09:35".to_string(),
        operator: "Beheerder".to_string(),
        table: Some(3),
        items: vec![
            BarItem {
                quantity: 1,
                name: "Dry Martini".to_string(),
            },
            BarItem {
                quantity: 1,
                name: "Plat du jour".to_string(),
            },
        ],
        lang: ReceiptLanguage::Fr,
        is_training: false,
    }));
    assert_in_order(
        &text,
        &[
            ("", "Date: 15/06/2026 12:09:35"),
            ("", "Operateur: Beheerder"),
            ("", "Table 3"),
            ("A", "Qte Article"),
            ("", "1 x Dry Martini"),
            ("", "1 x Plat du jour"),
            ("B", "CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE"),
        ],
    );
    // A: never a price.
    assert!(!text.contains(','), "{text}");
}

#[test]
fn vi_z_the_official_z_report_example() {
    let data = ReportPrintData {
        report_type: "Z".to_string(),
        report_no: Some(18),
        restaurant_name: "Café North Galaxy".to_string(),
        restaurant_address: "Koning Albert II-laan 33, 1030 Schaarbeek (Brussel)".to_string(),
        restaurant_vat: "BE0123456789".to_string(),
        pos_id: "CAAA0099887766".to_string(),
        terminal_id: "KASSA-1".to_string(),
        report_date: "2026-01-26".to_string(),
        first_datetime: "2026-01-26T11:18:27.104+01:00".to_string(),
        last_datetime: "2026-01-26T11:24:21.936+01:00".to_string(),
        employee_insz: "kassier03".to_string(),
        is_training: false,
        event_totals: vec![
            ReportEventTotal {
                label: "N".to_string(),
                count: 5,
                amount_cents: 5922,
            },
            ReportEventTotal {
                label: "P".to_string(),
                count: 3,
                amount_cents: 9064,
            },
        ],
        vat_totals: vec![
            ReportVatTotal {
                code: "A".to_string(),
                rate: 21.0,
                total_incl_cents: 2888,
                vat_cents: 501,
            },
            ReportVatTotal {
                code: "B".to_string(),
                rate: 12.0,
                total_incl_cents: 3034,
                vat_cents: 325,
            },
        ],
        payment_totals: vec![
            ReportPaymentTotal {
                method: "Cash".to_string(),
                count: 3,
                amount_cents: 2942,
            },
            ReportPaymentTotal {
                method: "Bancontact".to_string(),
                count: 1,
                amount_cents: 200,
            },
            ReportPaymentTotal {
                method: "MPV".to_string(),
                count: 1,
                amount_cents: 2780,
            },
        ],
        grand_total_cents: 5922,
        drawer_opens: 4,
        fdm_id: Some("FDM01122334".to_string()),
        fdm_datetime: None,
        short_signature: None,
        ticket_number: 792,
        lang: ReceiptLanguage::Fr,
        details: ReportDetails {
            restaurant_est_no: "2000000042".to_string(),
            booking_period_id: "2249358a-526b-43cb-a1f2-40a4d8430751".to_string(),
            created_at: "26/01/26 11:25:27".to_string(),
            fdm_first_event: Some((787, "2026-01-26T11:18:21.558".to_string())),
            fdm_last_event: Some((791, "2026-01-26T11:24:15.207".to_string())),
            departments: vec![
                ("Aperitifs".to_string(), 2280),
                ("Desserts".to_string(), 900),
                ("Soft Drinks".to_string(), 608),
                ("Starters".to_string(), 1800),
                ("Tapas".to_string(), 334),
            ],
            corrections_cents: -3200,
            refunds: (2, -2900),
            training: (4, 4304),
            line_reductions: (0, 0),
            event_reductions: (0, 0),
            invoices: vec![("314".to_string(), 1204)],
            rounding_cents: 3,
        },
    };
    let text = normalized(&format_report(&data));
    assert_in_order(
        &text,
        &[
            ("", "Z CHIFFRE D'AFFAIRES"),
            ("a", "Café North Galaxy"),
            ("b", "TVA: BE0123456789"),
            ("c", "Unite d'etablissement 2000000042"),
            ("d", "RAPPORT Z 26/01/26 11:25:27"),
            ("y", "Rapport nr. 18"),
            ("g", "Periode d'ouverture"),
            ("g", "2026-01-26"),
            ("h", "2249358a-526b-43cb-a1f2-40a4d8430751"),
            ("h", "Debut 2026-01-26 11:18"),
            ("h", "Fin 2026-01-26 11:24"),
            ("e", "POSID CAAA0099887766"),
            ("e", "TerminalId KASSA-1"),
            ("f", "FDMID FDM01122334"),
            ("x", "Premier 787 2026-01-26 11:18"),
            ("x", "Dernier 791 2026-01-26 11:24"),
            ("i", "Event N Ventes 5 59,22"),
            ("t", "Event P Pro forma 3 90,64"),
            ("k", "Apercu TVA"),
            ("k l m", "21% 23,87 5,01 28,88"),
            ("k l m", "12% 27,09 3,25 30,34"),
            ("k", "6% - - -"),
            ("k", "0% - - -"),
            ("m", "Total 50,96 8,26 59,22"),
            ("j", "Departements du chiffre d'affaires"),
            ("j", "Aperitifs 22,80"),
            ("j", "Tapas 3,34"),
            ("v", "Corrections -32,00"),
            ("s", "Remboursements 2 -29,00"),
            ("q", "Tiroir-caisse 4"),
            ("r", "Tickets de formation 4 43,04"),
            ("u", "Reductions de lignes 0 0,00"),
            ("u", "Reductions d'evenements 0 0,00"),
            ("w", "Facture nr. 314 12,04"),
            ("n", "PAIEMENTS ENREGISTRES"),
            ("n", "Especes 29,42"),
            ("n", "Bancontact 2,00"),
            ("n", "MPV 27,80"),
            ("n", "Total des paiements 59,22"),
            ("o", "Arrondis 0,03"),
            ("", "CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE"),
        ],
    );
}
