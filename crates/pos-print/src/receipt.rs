use pos_fiscal::ticket::{FiscalTicket, ReceiptLanguage};

use crate::escpos::{Alignment, EscPosBuilder, TextStyle};

/// Fixed receipt labels per ticket language (FR/NL/DE — §VI of the
/// certification manual requires the ticket in a national language).
/// ASCII only: the ESC/POS transport sends raw bytes without a
/// code-page handshake, so accented characters are avoided.
struct Labels {
    vat_no: &'static str,
    est_no: &'static str,
    date: &'static str,
    ticket: &'static str,
    terminal: &'static str,
    subtotal: &'static str,
    rounding: &'static str,
    total: &'static str,
    vat_header: &'static str,
    refund: &'static str,
}

fn labels(lang: ReceiptLanguage) -> &'static Labels {
    match lang {
        ReceiptLanguage::Fr => &Labels {
            vat_no: "TVA",
            est_no: "Etab",
            date: "Date:",
            ticket: "Ticket:",
            terminal: "Terminal:",
            subtotal: "Sous-total",
            rounding: "Arrondi",
            total: "TOTAL",
            vat_header: "TVA    Taux    Base HT    TVA",
            refund: "REMBOURSEMENT",
        },
        ReceiptLanguage::Nl => &Labels {
            vat_no: "BTW",
            est_no: "Vest",
            date: "Datum:",
            ticket: "Ticket:",
            terminal: "Terminal:",
            subtotal: "Subtotaal",
            rounding: "Afronding",
            total: "TOTAAL",
            vat_header: "BTW    Tarief  Basis      BTW",
            refund: "TERUGBETALING",
        },
        ReceiptLanguage::De => &Labels {
            vat_no: "MwSt",
            est_no: "Niederl",
            date: "Datum:",
            ticket: "Bon:",
            terminal: "Terminal:",
            subtotal: "Zwischensumme",
            rounding: "Rundung",
            total: "GESAMT",
            vat_header: "MwSt   Satz    Netto      MwSt",
            refund: "RUECKERSTATTUNG",
        },
    }
}

/// Format a FiscalTicket into ESC/POS binary commands.
/// Follows SCE 2.0 printing requirements:
/// - Event N: "TICKET DE CAISSE TVA" header, QR code, FDM control data
/// - Event T: "TRAINING" watermark + invalid-ticket mention
/// - Other events: spec-mandated invalid-ticket mention at bottom (FR/NL/DE)
pub fn format_receipt(ticket: &FiscalTicket) -> Vec<u8> {
    let mut p = EscPosBuilder::new();
    let l = labels(ticket.lang);

    // ── Training mode banner ───────────────────────────────────
    if ticket.is_training {
        p.align(Alignment::Center);
        p.style(TextStyle {
            bold: true,
            double_height: true,
            double_width: true,
            ..Default::default()
        });
        p.textln("TRAINING");
        p.style_reset();
        p.separator('=');
    }

    // ── Ticket type label ──────────────────────────────────────
    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.textln(&ticket.ticket_type_label);
    p.style_reset();
    p.blank_line();

    // ── Restaurant header ──────────────────────────────────────
    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln(&ticket.restaurant_name);
    p.style_reset();
    p.textln(&ticket.restaurant_address);
    p.textln(&format!("{}: {}", l.vat_no, ticket.restaurant_vat));
    if !ticket.restaurant_est_no.is_empty() {
        p.textln(&format!("{}: {}", l.est_no, ticket.restaurant_est_no));
    }
    p.blank_line();

    // ── Date/time + ticket number ──────────────────────────────
    p.align(Alignment::Left);
    p.text_columns(l.date, &format_datetime(&ticket.pos_datetime));
    p.text_columns(l.ticket, &format!("#{}", ticket.ticket_number));
    p.text_columns(l.terminal, &ticket.terminal_id);
    p.separator('-');

    // ── Line items ─────────────────────────────────────────────
    for line in &ticket.lines {
        let qty_desc = if (line.quantity - 1.0).abs() < f64::EPSILON {
            line.description.clone()
        } else {
            format!("{}x {}", format_qty(line.quantity), line.description)
        };
        let price = format_price(line.total);
        p.text_columns(&qty_desc, &format!("{price} {}", line.vat_code));
    }

    p.separator('-');

    // ── Subtotal / Rounding / Total ────────────────────────────
    if let Some(rounding) = ticket.rounding {
        p.text_columns(l.subtotal, &format_price(ticket.subtotal));
        p.text_columns(l.rounding, &format_price(rounding));
    }

    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.text_columns(l.total, &format_price(ticket.total));
    p.style_reset();

    p.separator('-');

    // ── Payments ───────────────────────────────────────────────
    for pay in &ticket.payments {
        p.text_columns(&pay.method, &format_price(pay.amount));
    }

    // ── VAT breakdown ──────────────────────────────────────────
    if !ticket.vat_breakdown.is_empty() {
        p.blank_line();
        p.style(TextStyle {
            font_b: true,
            ..Default::default()
        });
        // Header row
        p.textln(l.vat_header);
        for vat in &ticket.vat_breakdown {
            if vat.out_of_scope {
                continue;
            }
            let label = format!("{:?}", vat.label);
            let base = format_price(vat.taxable_amount);
            let vat_amt = format_price(vat.vat_amount);
            let line = format!(" {label}    {:>4.0}%  {base:>8}  {vat_amt:>8}", vat.rate,);
            p.textln(&line);
        }
        p.style_reset();
    }

    p.separator('=');

    // ── FDM control data ───────────────────────────────────────
    if let Some(ref fdm_ref) = ticket.fdm_ref {
        p.align(Alignment::Center);
        p.style(TextStyle {
            font_b: true,
            ..Default::default()
        });
        p.textln(&format!("FDM: {}", fdm_ref.fdm_id));
        p.textln(&fdm_ref.fdm_date_time);
        let label = format!("{:?}", fdm_ref.event_label);
        p.textln(&format!(
            "{label} {}/{}",
            fdm_ref.event_counter, fdm_ref.total_counter
        ));
        if let Some(ref sig) = ticket.short_signature {
            p.textln(&format!("Sig: {sig}"));
        }
        p.style_reset();
    }

    // ── QR Code ────────────────────────────────────────────────
    if let Some(ref url) = ticket.verification_url {
        p.blank_line();
        p.align(Alignment::Center);
        // Use printer's built-in QR command — module size 6 for clear scan
        p.qr_code(url, 6);
        p.blank_line();
    }

    // ── Footer (from FDM / SPF Finances) ───────────────────────
    if !ticket.footer.is_empty() {
        p.align(Alignment::Center);
        p.style(TextStyle {
            font_b: true,
            ..Default::default()
        });
        for line in &ticket.footer {
            p.textln(line);
        }
        p.style_reset();
    }

    // ── Invalid VAT receipt disclaimer ─────────────────────────
    if ticket.needs_invalid_disclaimer() {
        p.blank_line();
        p.align(Alignment::Center);
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.textln(ticket.invalid_disclaimer());
        p.style_reset();
    }

    // ── Refund label ───────────────────────────────────────────
    if ticket.is_refund {
        p.blank_line();
        p.align(Alignment::Center);
        p.style(TextStyle {
            bold: true,
            double_height: true,
            ..Default::default()
        });
        p.textln(l.refund);
        p.style_reset();
    }

    // ── End ────────────────────────────────────────────────────
    p.feed(4);
    p.cut();

    p.build()
}

/// Format a receipt that just opens the cash drawer (no print content).
pub fn format_drawer_open() -> Vec<u8> {
    let mut p = EscPosBuilder::new();
    p.open_drawer();
    p.build()
}

// ── Formatting helpers ─────────────────────────────────────────

fn format_price(amount: f64) -> String {
    format!("{:.2}", amount)
}

fn format_qty(qty: f64) -> String {
    if (qty - qty.round()).abs() < f64::EPSILON {
        format!("{}", qty as i64)
    } else {
        format!("{:.2}", qty)
    }
}

fn format_datetime(rfc3339: &str) -> String {
    // Try to extract "YYYY-MM-DD HH:MM" from an RFC3339 string
    if rfc3339.len() >= 16 {
        let date = &rfc3339[..10];
        let time = &rfc3339[11..16];
        format!("{date} {time}")
    } else {
        rfc3339.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pos_fiscal::ticket::{TicketLine, TicketPayment};
    use pos_fiscal::types::EventLabel;

    fn sample_ticket() -> FiscalTicket {
        FiscalTicket {
            restaurant_name: "Tokyo Sushi Bar".to_string(),
            restaurant_address: "Rue Test 1, 1000 Bruxelles".to_string(),
            restaurant_vat: "BE0499999960".to_string(),
            restaurant_est_no: String::new(),
            pos_id: "CFOD0061234567".to_string(),
            terminal_id: "POS1".to_string(),
            pos_datetime: "2026-04-09T19:30:00+02:00".to_string(),
            ticket_number: 42,
            event_label: EventLabel::N,
            is_training: false,
            lines: vec![
                TicketLine {
                    quantity: 2.0,
                    description: "Saumon roll x8".to_string(),
                    unit_price: 12.50,
                    total: 25.00,
                    vat_code: "B".to_string(),
                },
                TicketLine {
                    quantity: 1.0,
                    description: "Biere Asahi".to_string(),
                    unit_price: 4.50,
                    total: 4.50,
                    vat_code: "A".to_string(),
                },
            ],
            subtotal: 29.50,
            total: 29.50,
            vat_breakdown: vec![],
            payments: vec![TicketPayment {
                method: "Cash".to_string(),
                amount: 29.50,
            }],
            rounding: None,
            fdm_ref: None,
            short_signature: None,
            verification_url: None,
            footer: vec![],
            ticket_type_label: "TICKET DE CAISSE TVA".to_string(),
            is_refund: false,
            lang: ReceiptLanguage::default(),
        }
    }

    #[test]
    fn receipt_contains_restaurant_name() {
        let ticket = sample_ticket();
        let bytes = format_receipt(&ticket);
        assert!(bytes.windows(15).any(|w| w == b"Tokyo Sushi Bar"));
    }

    #[test]
    fn receipt_contains_vat_receipt_label() {
        let ticket = sample_ticket();
        let bytes = format_receipt(&ticket);
        assert!(bytes.windows(20).any(|w| w == b"TICKET DE CAISSE TVA"));
    }

    #[test]
    fn receipt_contains_line_items() {
        let ticket = sample_ticket();
        let bytes = format_receipt(&ticket);
        assert!(bytes.windows(14).any(|w| w == b"Saumon roll x8"));
        assert!(bytes.windows(11).any(|w| w == b"Biere Asahi"));
    }

    #[test]
    fn receipt_ends_with_cut() {
        let ticket = sample_ticket();
        let bytes = format_receipt(&ticket);
        // GS V 66 3 (partial cut) should be near the end
        assert!(bytes.windows(4).any(|w| w == [0x1D, b'V', 66, 3]));
    }

    #[test]
    fn training_receipt_has_banner() {
        let mut ticket = sample_ticket();
        ticket.is_training = true;
        ticket.event_label = EventLabel::T;
        ticket.ticket_type_label = "TRAINING".to_string();
        let bytes = format_receipt(&ticket);
        assert!(bytes.windows(8).any(|w| w == b"TRAINING"));
        assert!(
            bytes
                .windows(46)
                .any(|w| w == b"CECI N'EST PAS UN TICKET DE CAISSE TVA VALABLE")
        );
    }

    #[test]
    fn receipt_with_qr_code() {
        let mut ticket = sample_ticket();
        ticket.verification_url = Some("https://www.gs2.be/1234567890abcdefghi".to_string());
        let bytes = format_receipt(&ticket);
        // Should contain the QR code store command
        assert!(bytes.windows(5).any(|w| w == b"https"));
    }

    #[test]
    fn drawer_open_command() {
        let bytes = format_drawer_open();
        assert!(
            bytes
                .windows(5)
                .any(|w| w == [0x1B, b'p', 0x00, 0x19, 0xFA])
        );
    }
}
