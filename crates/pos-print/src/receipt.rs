use pos_fiscal::response::FdmReference;
use pos_fiscal::ticket::FiscalTicket;

use crate::escpos::{Alignment, EscPosBuilder, TextStyle, wrap};
use crate::labels::{labels, payment_label};

/// The FDM control data of one event (POS description v2.2 §6.6).
fn control_data(p: &mut EscPosBuilder, fdm_ref: &FdmReference, short_signature: Option<&str>) {
    p.textln(&format!("FDM: {}", fdm_ref.fdm_id));
    p.textln(&fdm_ref.fdm_date_time);
    let label = format!("{:?}", fdm_ref.event_label);
    p.textln(&format!(
        "{label} {}/{}",
        fdm_ref.event_counter, fdm_ref.total_counter
    ));
    if let Some(sig) = short_signature {
        p.textln(&format!("Sig: {sig}"));
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
    if ticket.is_copy() {
        p.textln(l.copy);
    }
    p.style_reset();
    p.blank_line();

    // ── Restaurant header ──────────────────────────────────────
    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln_wrapped(&ticket.restaurant_name);
    p.style_reset();
    p.textln_wrapped(&ticket.restaurant_address);
    p.textln(&format!("{}: {}", l.vat_no, ticket.restaurant_vat));
    if !ticket.restaurant_est_no.is_empty() {
        p.textln(&format!("{}: {}", l.est_no, ticket.restaurant_est_no));
    }
    p.blank_line();

    // ── Date/time + ticket number ──────────────────────────────
    p.align(Alignment::Left);
    p.text_columns(l.date, &format_datetime(&ticket.pos_datetime));
    p.text_columns(l.ticket, &format!("#{}", ticket.ticket_number));
    // Identification rows of the manual §VI ticket table: POS number
    // and software version (V), terminal (VI), input device (VII) and
    // user (VIII). Tickets archived before these fields existed have
    // them empty and print without them.
    p.text_columns(l.pos, &ticket.pos_id);
    if !ticket.pos_sw_version.is_empty() {
        p.text_columns(l.software, &ticket.pos_sw_version);
    }
    p.text_columns(l.terminal, &ticket.terminal_id);
    if !ticket.device_id.is_empty() {
        p.text_columns(l.device, &ticket.device_id);
    }
    if !ticket.operator.is_empty() {
        p.text_columns(l.user, &ticket.operator);
    }
    p.separator('-');

    // ── Line items ─────────────────────────────────────────────
    for line in &ticket.lines {
        let qty_desc = if (line.quantity - 1.0).abs() < f64::EPSILON {
            line.description.clone()
        } else {
            format!("{}x {}", format_qty(line.quantity), line.description)
        };
        let price = format!("{} {}", format_price(line.total), line.vat_code);
        item_line(&mut p, &qty_desc, &price);
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
        let method = payment_label(&pay.method, ticket.lang);
        p.text_columns(&method, &format_price(pay.amount));
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
        control_data(&mut p, fdm_ref, ticket.short_signature.as_deref());
        p.style_reset();
    }

    // ── QR Code ────────────────────────────────────────────────
    if let Some(ref url) = ticket.verification_url {
        p.blank_line();
        p.align(Alignment::Center);
        // Use printer's built-in QR command - module size 6 for clear scan
        p.qr_code(url, 6);
        p.blank_line();
    }

    // ── Control data of the original event ─────────────────────
    // The sale a refund reverses, the event a copy reproduces (SPF
    // example copy and refund tickets of 2026-09-24).
    if let Some(ref original) = ticket.original {
        p.align(Alignment::Center);
        p.style(TextStyle {
            font_b: true,
            ..Default::default()
        });
        p.textln(l.original_fdm);
        control_data(
            &mut p,
            &original.fdm_ref,
            original.short_signature.as_deref(),
        );
        p.style_reset();
    }
    if let Some(ref made_at) = ticket.copy_made_at {
        p.align(Alignment::Center);
        p.textln(&format!("{} {}", l.copy_made_at, format_datetime(made_at)));
    }

    // ── Footer (from FDM / SPF Finances) ───────────────────────
    if !ticket.footer.is_empty() {
        p.align(Alignment::Center);
        p.style(TextStyle {
            font_b: true,
            ..Default::default()
        });
        for line in &ticket.footer {
            p.textln_wrapped(line);
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
        p.textln_wrapped(ticket.invalid_disclaimer());
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

/// One article: description left, amount and VAT code right. A
/// description too long for one line wraps between words, with the
/// amount on its last line: the full product name is printed, never
/// cut.
fn item_line(p: &mut EscPosBuilder, desc: &str, price: &str) {
    let width = p.chars_per_line() as usize;
    let room = width.saturating_sub(price.chars().count() + 1);
    let mut lines = wrap(desc, width);
    let last = lines.pop().unwrap_or_default();
    for line in &lines {
        p.textln(line);
    }
    if last.chars().count() <= room {
        p.text_columns(&last, price);
    } else {
        p.textln(&last);
        p.text_columns("", price);
    }
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
    use pos_fiscal::ticket::{ReceiptLanguage, TicketLine, TicketPayment};
    use pos_fiscal::types::EventLabel;

    fn sample_ticket() -> FiscalTicket {
        FiscalTicket {
            restaurant_name: "Tokyo Sushi Bar".to_string(),
            restaurant_address: "Rue Test 1, 1000 Bruxelles".to_string(),
            restaurant_vat: "BE0499999960".to_string(),
            restaurant_est_no: String::new(),
            pos_id: "CFOD0061234567".to_string(),
            pos_sw_version: "moyun-pos/1.0.0".to_string(),
            terminal_id: "POS1".to_string(),
            device_id: "aa:bb:cc:dd:ee:ff".to_string(),
            operator: "Marie Dupont".to_string(),
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
            original: None,
            copy_made_at: None,
        }
    }

    fn copy_of_sample(lang: ReceiptLanguage) -> String {
        use pos_fiscal::response::FdmReference;
        use pos_fiscal::ticket::OriginalControlData;
        let mut t = sample_ticket();
        t.lang = lang;
        t.fdm_ref = Some(FdmReference {
            fdm_id: "CBX01000042".into(),
            fdm_date_time: "2026-04-09T19:45:00".into(),
            event_label: EventLabel::C,
            event_counter: 3,
            total_counter: 60,
        });
        t.original = Some(OriginalControlData {
            fdm_ref: FdmReference {
                fdm_id: "CBX01000042".into(),
                fdm_date_time: "2026-04-09T19:30:01".into(),
                event_label: EventLabel::N,
                event_counter: 41,
                total_counter: 57,
            },
            short_signature: Some("a3f9c2b1e8d4750616fb92c3a4e5d6f701234567".into()),
        });
        t.copy_made_at = Some("2026-04-09T19:45:00+02:00".into());
        crate::render::render_plain_text(&format_receipt(&t))
    }

    #[test]
    fn vi_copy_the_copy_mark_and_the_original_block_follow_the_language() {
        for (lang, copy, made, original) in [
            (
                ReceiptLanguage::Fr,
                "COPIE",
                "Copie etablie le 2026-04-09 19:45",
                "Informations FDM d'origine :",
            ),
            (
                ReceiptLanguage::Nl,
                "KOPIE",
                "Kopie gemaakt op 2026-04-09 19:45",
                "Oorspronkelijke FDM-gegevens :",
            ),
            (
                ReceiptLanguage::De,
                "KOPIE",
                "Kopie erstellt am 2026-04-09 19:45",
                "Urspruengliche FDM-Daten :",
            ),
        ] {
            let text = copy_of_sample(lang);
            for expected in [copy, made, original, "N 41/57", "C 3/60"] {
                assert!(
                    text.contains(expected),
                    "{lang:?}: {expected:?} missing from:\n{text}"
                );
            }
            // The original's block comes after the copy's own.
            assert!(text.find("C 3/60") < text.find("N 41/57"), "{text}");
        }
    }

    #[test]
    fn receipt_contains_restaurant_name() {
        let ticket = sample_ticket();
        let bytes = format_receipt(&ticket);
        assert!(bytes.windows(15).any(|w| w == b"Tokyo Sushi Bar"));
    }

    #[test]
    fn receipt_prints_the_identification_rows() {
        // Manual §VI rows V-VIII: POS number + software version,
        // terminal, input device, user.
        let text = crate::render::render_plain_text(&format_receipt(&sample_ticket()));
        for expected in [
            "CFOD0061234567",
            "moyun-pos/1.0.0",
            "POS1",
            "aa:bb:cc:dd:ee:ff",
            "Marie Dupont",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
        }
        assert!(
            !text.contains("12345678901"),
            "the INSZ must never be printed on a customer ticket"
        );
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

    // ── VAT breakdown (mandatory ticket row, spec §VI) ──────────

    #[test]
    fn receipt_prints_vat_breakdown_when_present() {
        use pos_fiscal::response::VatCalcItem;
        use pos_fiscal::types::VatLabel;

        let mut ticket = sample_ticket();
        ticket.vat_breakdown = vec![
            VatCalcItem {
                label: VatLabel::B,
                rate: 12.0,
                taxable_amount: 26.34,
                vat_amount: 3.16,
                total_amount: 29.50,
                out_of_scope: false,
            },
            VatCalcItem {
                label: VatLabel::A,
                rate: 21.0,
                taxable_amount: 4.13,
                vat_amount: 0.87,
                total_amount: 5.00,
                out_of_scope: false,
            },
        ];
        let text = crate::render::render_plain_text(&format_receipt(&ticket));
        assert!(text.contains("Taux"), "VAT header row missing:\n{text}");
        assert!(text.contains("B"), "VAT code B missing:\n{text}");
        assert!(text.contains("12%"), "VAT rate 12% missing:\n{text}");
        assert!(text.contains("21%"), "VAT rate 21% missing:\n{text}");
    }

    #[test]
    fn receipt_skips_out_of_scope_vat_lines() {
        use pos_fiscal::response::VatCalcItem;
        use pos_fiscal::types::VatLabel;

        let mut ticket = sample_ticket();
        ticket.vat_breakdown = vec![VatCalcItem {
            label: VatLabel::X,
            rate: 0.0,
            taxable_amount: 0.0,
            vat_amount: 0.0,
            total_amount: 0.0,
            out_of_scope: true,
        }];
        let text = crate::render::render_plain_text(&format_receipt(&ticket));
        // The header row still prints (breakdown is non-empty), but
        // the out-of-scope X line itself must be skipped.
        assert!(text.contains("Taux"));
        assert!(!text.contains(" X    "));
    }

    // ── CJK / accented character handling ───────────────────────

    #[test]
    fn cjk_characters_in_a_line_description_degrade_to_question_marks_not_panic() {
        // CP858 (the code page this thermal printer speaks) has no
        // CJK glyphs - this is a real hardware/protocol constraint,
        // not a bug. Pinning down the actual behavior (each unmapped
        // character becomes '?', nothing vanishes, nothing panics)
        // documents it instead of leaving it an untested assumption.
        let mut ticket = sample_ticket();
        ticket.lines.push(TicketLine {
            quantity: 1.0,
            description: "寿司".to_string(),
            unit_price: 5.0,
            total: 5.0,
            vat_code: "B".to_string(),
        });
        let text = crate::render::render_plain_text(&format_receipt(&ticket));
        assert!(
            text.contains("??"),
            "unsupported CJK chars must degrade to '?', not vanish or panic:\n{text}"
        );
    }

    #[test]
    fn accented_french_characters_round_trip_through_cp858() {
        let mut ticket = sample_ticket();
        ticket.lines.push(TicketLine {
            quantity: 1.0,
            description: "Creme brulee a l'erable".to_string(),
            unit_price: 6.5,
            total: 6.5,
            vat_code: "B".to_string(),
        });
        // Accents actually present in CP858_HIGH (escpos.rs).
        let accented = "Fondant au chocolat, crème brûlée, café";
        ticket.lines.push(TicketLine {
            quantity: 1.0,
            description: accented.to_string(),
            unit_price: 7.0,
            total: 7.0,
            vat_code: "B".to_string(),
        });
        let text = crate::render::render_plain_text(&format_receipt(&ticket));
        assert!(
            text.contains(accented),
            "accented chars must round-trip through CP858:\n{text}"
        );
    }

    // ── Width invariant on transactional rows ───────────────────

    #[test]
    fn line_item_rows_never_exceed_paper_width_even_with_a_very_long_product_name() {
        // Menu item names are operator-authored and could in theory
        // be very long; the text_columns() row they're printed
        // through must always clamp to the 48-column paper width
        // (see escpos::tests::text_columns_never_exceeds_the_*).
        let mut ticket = sample_ticket();
        ticket.lines.push(TicketLine {
            quantity: 3.0,
            description: "A".repeat(120),
            unit_price: 999999.99,
            total: 2999999.97,
            vat_code: "B".to_string(),
        });
        let text = crate::render::render_plain_text(&format_receipt(&ticket));
        for line in text.lines() {
            assert!(
                line.chars().count() <= 48,
                "line exceeds 48 columns ({} chars): {line:?}",
                line.chars().count()
            );
        }
    }
}
