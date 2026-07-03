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
    FiscalTicket, ReceiptLanguage, TicketLine, TicketPayment, ticket_type_label,
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
        fdm_id: "CBX01000042".to_string(),
        fdm_date_time: "2026-07-03T12:34:57+02:00".to_string(),
        event_label: label,
        event_counter,
        total_counter,
    }
}

fn base_ticket(label: EventLabel) -> FiscalTicket {
    FiscalTicket {
        restaurant_name: "Tokyo Sushi Bar".to_string(),
        restaurant_address: "Rue Grétry 42, 4000 Liège".to_string(),
        restaurant_vat: "BE0499999960".to_string(),
        restaurant_est_no: "2123456789".to_string(),
        pos_id: "CFOD0061234567".to_string(),
        terminal_id: "POS1".to_string(),
        pos_datetime: "2026-07-03T12:34:56+02:00".to_string(),
        ticket_number: 15,
        event_label: label,
        is_training: false,
        lines: vec![
            TicketLine {
                quantity: 1.0,
                description: "Saumon roll x8".to_string(),
                unit_price: 8.50,
                total: 8.50,
                vat_code: "B".to_string(),
            },
            TicketLine {
                quantity: 1.0,
                description: "Coca 33cl".to_string(),
                unit_price: 3.50,
                total: 3.50,
                vat_code: "A".to_string(),
            },
        ],
        subtotal: 12.00,
        total: 12.00,
        vat_breakdown: vec![
            VatCalcItem {
                label: VatLabel::A,
                rate: 21.0,
                taxable_amount: 2.89,
                vat_amount: 0.61,
                total_amount: 3.50,
                out_of_scope: false,
            },
            VatCalcItem {
                label: VatLabel::B,
                rate: 12.0,
                taxable_amount: 7.59,
                vat_amount: 0.91,
                total_amount: 8.50,
                out_of_scope: false,
            },
        ],
        payments: vec![TicketPayment {
            method: "Espèces".to_string(),
            amount: 12.00,
        }],
        rounding: None,
        fdm_ref: Some(fdm_ref(label, 15, 59)),
        short_signature: Some("a3f9c2b1e8d4750616fb92c3a4e5d6f70123456789".to_string()),
        verification_url: Some("https://www.gs2.be/Mk9Xa7Qb2Lc4Rd8Se3Tf5Ug".to_string()),
        footer: vec!["www.systemedecaisseenregistreuse.be".to_string()],
        ticket_type_label: ticket_type_label(label, false, ReceiptLanguage::Fr).to_string(),
        is_refund: false,
        lang: ReceiptLanguage::Fr,
    }
}

fn write_example(name: &str, ticket: &FiscalTicket) {
    let Some(dir) = out_dir() else { return };
    std::fs::create_dir_all(&dir).expect("create examples dir");
    let text = render_plain_text(&format_receipt(ticket));
    std::fs::write(dir.join(format!("{name}.txt")), text).expect("write example");
}

#[test]
fn example_n_sale() {
    let ticket = base_ticket(EventLabel::N);
    write_example("ticket_N_vente", &ticket);
}

#[test]
fn example_n_refund() {
    let mut ticket = base_ticket(EventLabel::N);
    ticket.is_refund = true;
    ticket.ticket_type_label = ticket_type_label(EventLabel::N, true, ReceiptLanguage::Fr).into();
    for l in &mut ticket.lines {
        l.quantity = -l.quantity;
        l.total = -l.total;
    }
    ticket.subtotal = -12.00;
    ticket.total = -12.00;
    ticket.payments[0].amount = -12.00;
    ticket.ticket_number = 16;
    ticket.fdm_ref = Some(fdm_ref(EventLabel::N, 16, 60));
    write_example("ticket_N_remboursement", &ticket);
}

#[test]
fn example_p_pro_forma() {
    let mut ticket = base_ticket(EventLabel::P);
    ticket.ticket_type_label = ticket_type_label(EventLabel::P, false, ReceiptLanguage::Fr).into();
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
    let mut ticket = base_ticket(EventLabel::C);
    ticket.ticket_type_label = ticket_type_label(EventLabel::C, false, ReceiptLanguage::Fr).into();
    // Copies never re-print the short signature.
    ticket.short_signature = None;
    ticket.ticket_number = 2;
    ticket.fdm_ref = Some(fdm_ref(EventLabel::C, 2, 62));
    write_example("ticket_C_copie", &ticket);
}

#[test]
fn example_t_training() {
    let mut ticket = base_ticket(EventLabel::T);
    ticket.is_training = true;
    ticket.ticket_type_label = ticket_type_label(EventLabel::T, false, ReceiptLanguage::Fr).into();
    ticket.short_signature = None;
    ticket.ticket_number = 7;
    ticket.fdm_ref = Some(fdm_ref(EventLabel::T, 7, 63));
    write_example("ticket_T_training", &ticket);
}

// ── X / Z reports ──────────────────────────────────────────────

fn base_report(report_type: &str, report_no: Option<i64>) -> pos_print::report::ReportPrintData {
    use pos_print::report::*;
    ReportPrintData {
        report_type: report_type.to_string(),
        report_no,
        restaurant_name: "Tokyo Sushi Bar".to_string(),
        restaurant_address: "Rue Grétry 42, 4000 Liège".to_string(),
        restaurant_vat: "BE0499999960".to_string(),
        pos_id: "CFOD0061234567".to_string(),
        terminal_id: "POS1".to_string(),
        report_date: "2026-07-03".to_string(),
        first_datetime: "2026-07-03T11:02:14+02:00".to_string(),
        last_datetime: "2026-07-03T22:41:09+02:00".to_string(),
        employee_insz: "12345678901".to_string(),
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
        payment_totals: vec![
            ReportPaymentTotal {
                method: "Espèces".to_string(),
                count: 18,
                amount_cents: 61_200,
            },
            ReportPaymentTotal {
                method: "Carte".to_string(),
                count: 24,
                amount_cents: 91_150,
            },
        ],
        grand_total_cents: 152_350,
        drawer_opens: 3,
        fdm_id: Some("CBX01000042".to_string()),
        fdm_datetime: Some("2026-07-03T22:42:00+02:00".to_string()),
        short_signature: Some("7c2e91d4a5b3f60718c9e2d3a4b5c6d7e8f90a1b2c".to_string()),
        ticket_number: 3,
    }
}

#[test]
fn example_x_report() {
    let Some(dir) = out_dir() else { return };
    std::fs::create_dir_all(&dir).expect("create examples dir");
    let data = base_report("X", None);
    let text = render_plain_text(&pos_print::report::format_report(&data));
    std::fs::write(dir.join("rapport_X.txt"), text).expect("write example");
}

#[test]
fn example_z_report() {
    let Some(dir) = out_dir() else { return };
    std::fs::create_dir_all(&dir).expect("create examples dir");
    let data = base_report("Z", Some(1));
    let text = render_plain_text(&pos_print::report::format_report(&data));
    std::fs::write(dir.join("rapport_Z.txt"), text).expect("write example");
}
