//! §VI ticket-example generator.
//!
//! When `TICKET_EXAMPLES_DIR` is set, renders one plain-text
//! transcription per ticket type (N, N-refund, P, C, T) from the REAL
//! `format_receipt` pipeline via the ESC/POS text renderer, so the
//! dossier's `TICKETS_EXEMPLES_FR.md` cannot drift from what the
//! printer produces. No-op under plain `cargo test`.
//!
//! Regenerate with `scripts/generate_ticket_examples.sh`.

use pos_fiscal::response::{FdmReference, VatCalcItem};
use pos_fiscal::ticket::{
    FiscalTicket, OriginalControlData, ReceiptLanguage, TicketLine, TicketPayment,
    ticket_type_label,
};
use pos_fiscal::types::{EventLabel, VatLabel};
use pos_print::receipt::format_receipt;
use pos_print::render::render_plain_text;

fn out_dir() -> Option<std::path::PathBuf> {
    std::env::var("TICKET_EXAMPLES_DIR")
        .ok()
        .map(std::path::PathBuf::from)
}

fn fdm_ref(label: EventLabel, event_counter: i64, total_counter: i64) -> FdmReference {
    FdmReference {
        fdm_id: "FDM02000425".to_string(),
        fdm_date_time: "2026-07-03T12:34:57+02:00".to_string(),
        event_label: label,
        event_counter,
        total_counter,
    }
}

fn base_ticket(label: EventLabel) -> FiscalTicket {
    FiscalTicket {
        restaurant_name: "Restaurant Exemple".to_string(),
        restaurant_address: "Rue de l'Exemple 1, 4000 Liège".to_string(),
        restaurant_vat: "BE0499999960".to_string(),
        restaurant_est_no: "2123456791".to_string(),
        restaurant_legal_name: "Societe Exemple SRL".to_string(),
        pos_id: "CFOD0061234567".to_string(),
        pos_sw_version: "pili-pos/1.0.0".to_string(),
        terminal_id: "POS1".to_string(),
        device_id: "00:1b:44:11:3a:b7".to_string(),
        operator: "Marie Dupont".to_string(),
        pos_datetime: "2026-07-03T12:34:56+02:00".to_string(),
        ticket_number: 15,
        event_label: label,
        is_training: false,
        // Manual M110 basket: 2x Dry Martini (A) + 1x Burger du chef (B).
        lines: vec![
            TicketLine {
                quantity: 2.0,
                description: "Dry Martini".to_string(),
                unit_price: 12.00,
                total: 24.00,
                vat_code: "A".to_string(),
            },
            TicketLine {
                quantity: 1.0,
                description: "Burger du chef".to_string(),
                unit_price: 28.00,
                total: 28.00,
                vat_code: "B".to_string(),
            },
        ],
        subtotal: 52.00,
        total: 52.00,
        vat_breakdown: vec![
            VatCalcItem {
                label: VatLabel::A,
                rate: 21.0,
                taxable_amount: 19.83,
                vat_amount: 4.17,
                total_amount: 24.00,
                out_of_scope: false,
            },
            VatCalcItem {
                label: VatLabel::B,
                rate: 12.0,
                taxable_amount: 25.00,
                vat_amount: 3.00,
                total_amount: 28.00,
                out_of_scope: false,
            },
        ],
        payments: vec![TicketPayment {
            method: "Espèces".to_string(),
            amount: 52.00,
            brand: None,
        }],
        rounding: None,
        fdm_ref: Some(fdm_ref(label, 15, 59)),
        // The FDM returns the short signature and the verification URL
        // for N events only (POS spec v2.2 §2.4): other tickets carry
        // neither, so they print no QR code.
        short_signature: (label == EventLabel::N)
            .then(|| "a3f9c2b1e8d4750616fb92c3a4e5d6f701234567".to_string()),
        verification_url: (label == EventLabel::N)
            .then(|| "https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug".to_string()),
        footer: vec!["www.systemedecaisseenregistreuse.be".to_string()],
        ticket_type_label: ticket_type_label(label, ReceiptLanguage::Fr).to_string(),
        is_refund: false,
        lang: ReceiptLanguage::Fr,
        original: None,
        copy_made_at: None,
        unpaid: false,
    }
}

fn write_example(name: &str, ticket: &FiscalTicket) {
    let text = render_plain_text(&format_receipt(ticket));
    // Only a VAT ticket (N) carries the FDM's QR code; its copy
    // reprints the original's (§2.4, §6.5).
    let vat_ticket = ticket.event_label == EventLabel::N;
    assert_eq!(
        text.contains("[QR"),
        vat_ticket,
        "{name}: QR code only on VAT tickets and their copies"
    );
    if let Some(sig) = &ticket.short_signature {
        assert_eq!(sig.len(), 40, "{name}: short signature is a SHA-1 in hex");
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
    }
    let Some(dir) = out_dir() else { return };
    std::fs::create_dir_all(&dir).expect("create examples dir");
    std::fs::write(dir.join(format!("{name}.txt")), text).expect("write example");
}

#[test]
fn example_n_sale() {
    let ticket = base_ticket(EventLabel::N);
    write_example("ticket_N_vente", &ticket);
}

/// Manual rounding test 3 (ROUND5): 3,13 in meal vouchers, 3,97 by
/// card, the rest 7,76 in cash, paid 7,75. Only the cash is rounded.
#[test]
fn example_n_split_payment() {
    let mut ticket = base_ticket(EventLabel::N);
    ticket.lines = vec![
        TicketLine {
            quantity: 1.0,
            description: "Dame Blanche".to_string(),
            unit_price: 11.52,
            total: 11.52,
            vat_code: "B".to_string(),
        },
        TicketLine {
            quantity: 1.0,
            description: "Tapas Variation".to_string(),
            unit_price: 3.34,
            total: 3.34,
            vat_code: "B".to_string(),
        },
    ];
    ticket.subtotal = 14.86;
    ticket.total = 14.86;
    ticket.vat_breakdown = vec![VatCalcItem {
        label: VatLabel::B,
        rate: 12.0,
        taxable_amount: 13.27,
        vat_amount: 1.59,
        total_amount: 14.86,
        out_of_scope: false,
    }];
    ticket.payments = [("Meal voucher", 3.13), ("Card", 3.97), ("Cash", 7.76)]
        .into_iter()
        .map(|(method, amount)| TicketPayment {
            method: method.to_string(),
            amount,
            brand: None,
        })
        .collect();
    ticket.rounding = Some(-0.01);
    write_example("ticket_N_paiement_mixte", &ticket);
}

/// UC4109: the customer left without paying; the sale is signed with
/// no payment line.
#[test]
fn example_n_unpaid() {
    let mut ticket = base_ticket(EventLabel::N);
    ticket.payments.clear();
    ticket.unpaid = true;
    write_example("ticket_N_non_paye", &ticket);
}

#[test]
fn example_n_refund() {
    let mut ticket = base_ticket(EventLabel::N);
    ticket.is_refund = true;
    ticket.ticket_type_label = ticket_type_label(EventLabel::N, ReceiptLanguage::Fr).into();
    for l in &mut ticket.lines {
        l.quantity = -l.quantity;
        l.total = -l.total;
    }
    // The FDM computes the refund's VAT on the negative lines.
    for v in &mut ticket.vat_breakdown {
        v.taxable_amount = -v.taxable_amount;
        v.vat_amount = -v.vat_amount;
        v.total_amount = -v.total_amount;
    }
    ticket.subtotal = -52.00;
    ticket.total = -52.00;
    ticket.payments[0].amount = -52.00;
    ticket.ticket_number = 16;
    ticket.fdm_ref = Some(fdm_ref(EventLabel::N, 16, 60));
    // The sale it reverses.
    ticket.original = Some(OriginalControlData {
        fdm_ref: fdm_ref(EventLabel::N, 15, 59),
        short_signature: Some("a3f9c2b1e8d4750616fb92c3a4e5d6f701234567".to_string()),
    });
    write_example("ticket_N_remboursement", &ticket);
}

#[test]
fn example_p_pro_forma() {
    let mut ticket = base_ticket(EventLabel::P);
    ticket.ticket_type_label = ticket_type_label(EventLabel::P, ReceiptLanguage::Fr).into();
    // Pro forma: no payment yet, no short signature, no VAT calc.
    ticket.payments.clear();
    ticket.short_signature = None;
    ticket.vat_breakdown.clear();
    ticket.ticket_number = 4;
    ticket.fdm_ref = Some(fdm_ref(EventLabel::P, 4, 61));
    write_example("ticket_P_pro_forma", &ticket);
}

#[test]
fn example_c_copy() {
    // The copy of the sale above: the original ticket, its QR code
    // included, with the copy event's control data and the original's
    // under "Informations FDM d'origine" (POS description v2.2 §1.8).
    let mut ticket = base_ticket(EventLabel::N);
    ticket.original = Some(OriginalControlData {
        fdm_ref: fdm_ref(EventLabel::N, 15, 59),
        short_signature: ticket.short_signature.take(),
    });
    ticket.fdm_ref = Some(fdm_ref(EventLabel::C, 2, 62));
    ticket.copy_made_at = Some("2026-07-03T12:41:08+02:00".to_string());
    write_example("ticket_C_copie", &ticket);
}

#[test]
fn example_t_training() {
    let mut ticket = base_ticket(EventLabel::T);
    ticket.is_training = true;
    ticket.ticket_type_label = ticket_type_label(EventLabel::T, ReceiptLanguage::Fr).into();
    ticket.short_signature = None;
    ticket.ticket_number = 7;
    ticket.fdm_ref = Some(fdm_ref(EventLabel::T, 7, 63));
    write_example("ticket_T_training", &ticket);
}

#[test]
fn example_bar_slip() {
    use pos_print::bar_ticket::{BarItem, BarTicket, format_bar_ticket};
    let slip = BarTicket {
        date_time: "03/07/2026 12:31:05".to_string(),
        operator: "Marie Dupont".to_string(),
        table: Some(4),
        items: vec![
            BarItem {
                quantity: 2,
                name: "Dry Martini".to_string(),
            },
            BarItem {
                quantity: 1,
                name: "Burger du chef".to_string(),
            },
        ],
        lang: ReceiptLanguage::Fr,
        is_training: false,
    };
    let text = render_plain_text(&format_bar_ticket(&slip));
    assert!(!text.contains(','), "a bar ticket never shows a price");
    let Some(dir) = out_dir() else { return };
    std::fs::create_dir_all(&dir).expect("create examples dir");
    std::fs::write(dir.join("ticket_bar.txt"), text).expect("write example");
}

// ── X / Z reports ──────────────────────────────────────────────

fn base_report(
    report_type: &str,
    report_no: Option<i64>,
    lang: ReceiptLanguage,
) -> pos_print::report::ReportPrintData {
    use pos_print::report::*;
    ReportPrintData {
        report_type: report_type.to_string(),
        report_no,
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
        fdm_id: Some("FDM02000425".to_string()),
        fdm_datetime: Some("2026-07-03T22:42:00+02:00".to_string()),
        // R events get no short signature from the FDM (§2.4).
        short_signature: None,
        ticket_number: 3,
        lang,
        details: ReportDetails {
            restaurant_est_no: "2123456791".to_string(),
            booking_period_id: "3f6c2a4e-8b1d-4c7a-9e25-7d0b6a1f4c83".to_string(),
            created_at: "03/07/26 23:05:41".to_string(),
            fdm_first_event: Some((41, "2026-07-03T11:02:13+02:00".to_string())),
            fdm_last_event: Some((63, "2026-07-03T22:48:09+02:00".to_string())),
            departments: vec![
                ("Boissons".to_string(), 38450),
                ("Plats".to_string(), 113900),
            ],
            corrections_cents: -1200,
            refunds: (1, -5200),
            training: (1, 5200),
            line_reductions: (0, 0),
            event_reductions: (0, 0),
            invoices: vec![],
            rounding_cents: 2,
            payment_corrections: (0, 0),
        },
    }
}

#[test]
fn example_x_report() {
    let Some(dir) = out_dir() else { return };
    std::fs::create_dir_all(&dir).expect("create examples dir");
    let data = base_report("X", None, ReceiptLanguage::Fr);
    let text = render_plain_text(&pos_print::report::format_report(&data));
    std::fs::write(dir.join("rapport_X.txt"), text).expect("write example");
}

#[test]
fn example_z_report() {
    let Some(dir) = out_dir() else { return };
    std::fs::create_dir_all(&dir).expect("create examples dir");
    let data = base_report("Z", Some(1), ReceiptLanguage::Fr);
    let text = render_plain_text(&pos_print::report::format_report(&data));
    std::fs::write(dir.join("rapport_Z.txt"), text).expect("write example");
}

#[test]
fn example_z_report_nl() {
    let Some(dir) = out_dir() else { return };
    std::fs::create_dir_all(&dir).expect("create examples dir");
    let data = base_report("Z", Some(1), ReceiptLanguage::Nl);
    let text = render_plain_text(&pos_print::report::format_report(&data));
    std::fs::write(dir.join("rapport_Z_nl.txt"), text).expect("write example");
}

#[test]
fn example_z_report_de() {
    let Some(dir) = out_dir() else { return };
    std::fs::create_dir_all(&dir).expect("create examples dir");
    let data = base_report("Z", Some(1), ReceiptLanguage::De);
    let text = render_plain_text(&pos_print::report::format_report(&data));
    std::fs::write(dir.join("rapport_Z_de.txt"), text).expect("write example");
}

#[test]
fn a_z_report_lists_the_payment_corrections_in_every_language() {
    for (lang, label) in [
        (ReceiptLanguage::Fr, "Corrections de paiement 2"),
        (ReceiptLanguage::Nl, "Betalingscorrecties 2"),
        (ReceiptLanguage::De, "Zahlungskorrekturen 2"),
    ] {
        let mut data = base_report("Z", Some(1), lang);
        // Nothing moved: the line is not printed.
        let none = render_plain_text(&pos_print::report::format_report(&data));
        assert!(!none.contains(&label[..label.len() - 2]), "{lang:?}");
        data.details.payment_corrections = (2, 7800);
        let text = render_plain_text(&pos_print::report::format_report(&data));
        let line = text
            .lines()
            .find(|l| l.contains(label))
            .unwrap_or_else(|| panic!("{lang:?}: no corrections line in\n{text}"));
        assert!(line.contains("78,00") || line.contains("78.00"), "{line}");
    }
}
