use pos_fiscal::response::FdmReference;
use pos_fiscal::ticket::FiscalTicket;
use pos_fiscal::types::EventLabel;

use crate::escpos::{Alignment, EscPosBuilder, TextStyle, wrap};
use crate::labels::{Labels, labels, payment_label};

/// The FDM control data of one event (POS description v2.2 §6.6), in
/// the wording of the SPF example tickets of 2026-09-24.
fn control_data(p: &mut EscPosBuilder, fdm_ref: &FdmReference, short_signature: Option<&str>) {
    p.textln(&format!("fdmId: {};", fdm_ref.fdm_id));
    p.textln(&format!("fdmDateTime: {};", fdm_ref.fdm_date_time));
    p.textln(&format!(
        "Event: {:?}; eventCounter: {};",
        fdm_ref.event_label, fdm_ref.event_counter
    ));
    match short_signature {
        Some(sig) => {
            p.textln(&format!(
                "totalCounter: {}; shortSignature:",
                fdm_ref.total_counter
            ));
            p.textln(&format!("{sig};"));
        }
        None => {
            p.textln(&format!("totalCounter: {};", fdm_ref.total_counter));
        }
    }
}

/// A boxed section title ("INFORMATIONS FDM").
fn section_title(p: &mut EscPosBuilder, title: &str) {
    p.separator('-');
    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln(title);
    p.style_reset();
    p.separator('-');
}

/// The mentions printed under the title: REMBOURSEMENT, Addition
/// Provisoire (pre-bill), COPIE.
fn subtitles(ticket: &FiscalTicket, l: &Labels) -> Vec<&'static str> {
    let mut out = Vec::new();
    if ticket.is_refund {
        out.push(l.refund);
    }
    if ticket.event_label == EventLabel::P {
        out.push(l.pre_bill);
    }
    if ticket.is_copy() {
        out.push(l.copy);
    }
    out
}

/// Format a FiscalTicket into ESC/POS binary commands, following the
/// SPF example tickets of 2026-09-24 (AM of 29 April 2024 art. 20):
/// title, commercial name, identification rows, product lines, total,
/// payment lines with the rounding, VAT per rate, FDM control data and
/// QR code, legal identity; non-N events end with the spec-mandated
/// invalid-ticket mention (FR/NL/DE).
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

    // ── Title and its mentions ─────────────────────────────────
    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.textln(&ticket.ticket_type_label);
    p.style_reset();
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    for subtitle in subtitles(ticket, l) {
        p.textln(subtitle);
    }
    p.style_reset();

    // ── Commercial name and identification rows ────────────────
    p.align(Alignment::Left);
    p.textln_wrapped(&ticket.restaurant_name);
    if !ticket.restaurant_est_no.is_empty() {
        p.textln(&format!("{} {}", l.est_unit, ticket.restaurant_est_no));
    }
    let (date, time) = date_and_time(&ticket.pos_datetime);
    p.text_columns(&date, &time);
    if !ticket.operator.is_empty() {
        p.textln(&format!("{} {}", l.user, ticket.operator));
    }
    p.textln(&format!("{} {}", l.terminal, ticket.terminal_id));
    p.textln(&format!("{} {:06}", l.ticket_no, ticket.ticket_number));
    p.textln(&format!("{} {}", l.pos_serial, ticket.pos_id));
    // Tickets archived before these fields existed print without them.
    if !ticket.device_id.is_empty() {
        p.textln(&format!("{} {}", l.device, ticket.device_id));
    }
    if !ticket.pos_sw_version.is_empty() {
        p.textln(&format!("{} {}", l.software, ticket.pos_sw_version));
    }
    p.separator('.');

    // ── Product lines: quantity, description, line total, VAT code
    for line in &ticket.lines {
        let qty_desc = format!("{}  {}", format_qty(line.quantity), line.description);
        let price = format!("{}  {}", format_price(line.total), line.vat_code);
        item_line(&mut p, &qty_desc, &price);
    }

    // ── Total due (the sale; the rounding is a payment line) ───
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.text_columns(l.total_row, &format!("€{}", format_price(ticket.subtotal)));
    p.style_reset();
    p.separator('.');

    // ── Payment lines, rounding included ───────────────────────
    if ticket.unpaid {
        // Closed without payment (Advanced Set UC4109): no payment
        // line was signed, the ticket says so instead.
        p.textln(l.payment_header);
        p.textln(l.unpaid);
    } else if !ticket.payments.is_empty() || ticket.rounding.is_some() {
        p.textln(l.payment_header);
        for pay in &ticket.payments {
            let method = payment_label(&pay.method, ticket.lang);
            p.text_columns(&method, &format_price(pay.amount));
        }
        if let Some(rounding) = ticket.rounding {
            p.text_columns(l.rounding, &format_price(rounding));
        }
    }

    // ── VAT per rate, as computed by the FDM (never on a pre-bill)
    let vat_rows: Vec<_> = ticket
        .vat_breakdown
        .iter()
        .filter(|v| !v.out_of_scope)
        .collect();
    if ticket.event_label != EventLabel::P && !vat_rows.is_empty() {
        let [rate, code, base, vat, gross] = l.vat_columns;
        p.textln(&vat_row(rate, code, base, vat, gross));
        for v in vat_rows {
            p.textln(&vat_row(
                &format!("{}%", format_price(v.rate)),
                &format!("{:?}", v.label),
                &format_price(v.taxable_amount),
                &format_price(v.vat_amount),
                &format_price(v.total_amount),
            ));
        }
    }

    // ── FDM control data and QR code ───────────────────────────
    if let Some(ref fdm_ref) = ticket.fdm_ref {
        section_title(&mut p, l.fdm_info);
        p.align(Alignment::Left);
        p.style(TextStyle {
            font_b: true,
            ..Default::default()
        });
        control_data(&mut p, fdm_ref, ticket.short_signature.as_deref());
        p.style_reset();
    }
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
    p.align(Alignment::Left);
    if let Some(ref original) = ticket.original {
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
        let (date, time) = date_and_time(made_at);
        p.textln(&format!("{} {date} {time}", l.copy_made_at));
    }

    // ── Legal identity of the issuer ───────────────────────────
    p.blank_line();
    p.textln_wrapped(&format!(
        "{} {}",
        l.legal_name, ticket.restaurant_legal_name
    ));
    p.textln_wrapped(&ticket.restaurant_address);
    p.textln(&format!("{}: {}", l.vat_no, ticket.restaurant_vat));

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

    // ── End ────────────────────────────────────────────────────
    p.feed(4);
    p.cut();

    p.build()
}

/// One row of the VAT table: rate, code, base, VAT, gross.
fn vat_row(rate: &str, code: &str, base: &str, vat: &str, gross: &str) -> String {
    format!("{rate:<8}{code:<6}{base:>10}{vat:>10}{gross:>10}")
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
    // Belgian notation: decimal comma.
    format!("{:.2}", amount).replace('.', ",")
}

fn format_qty(qty: f64) -> String {
    if (qty - qty.round()).abs() < f64::EPSILON {
        format!("{}", qty as i64)
    } else {
        format_price(qty)
    }
}

/// "23.06.2026" and "11:35" from an RFC 3339 date-time, whatever its
/// offset notation; anything shorter prints as it is.
fn date_and_time(rfc3339: &str) -> (String, String) {
    match (rfc3339.get(..10), rfc3339.get(11..16)) {
        (Some(date), Some(time)) if date.len() == 10 && date.as_bytes()[4] == b'-' => {
            let (y, m, d) = (&date[..4], &date[5..7], &date[8..10]);
            (format!("{d}.{m}.{y}"), time.to_string())
        }
        _ => (rfc3339.to_string(), String::new()),
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
            restaurant_legal_name: "Societe Exemple SRL".to_string(),
            pos_id: "CFOD0061234567".to_string(),
            pos_sw_version: "pili-pos/1.0.0".to_string(),
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
            ticket_type_label: "TICKET DE CAISSE T.V.A.".to_string(),
            is_refund: false,
            lang: ReceiptLanguage::default(),
            original: None,
            copy_made_at: None,
            unpaid: false,
        }
    }

    #[test]
    fn an_unpaid_sale_says_so_instead_of_its_payments() {
        for (lang, word) in [
            (ReceiptLanguage::Fr, "Non paye"),
            (ReceiptLanguage::Nl, "Niet betaald"),
            (ReceiptLanguage::De, "Nicht bezahlt"),
        ] {
            let mut t = sample_ticket();
            t.lang = lang;
            t.payments.clear();
            t.unpaid = true;
            let text = crate::render::render_plain_text(&format_receipt(&t));
            assert!(text.contains(word), "{lang:?}: {text}");
            assert!(!text.contains("Cash"), "{text}");
        }
        let text = crate::render::render_plain_text(&format_receipt(&sample_ticket()));
        assert!(!text.contains("Non paye"));
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
                "Copie etablie le 09.04.2026 19:45",
                "Informations FDM d'origine :",
            ),
            (
                ReceiptLanguage::Nl,
                "KOPIE",
                "Kopie gemaakt op 09.04.2026 19:45",
                "Oorspronkelijke FDM-gegevens :",
            ),
            (
                ReceiptLanguage::De,
                "KOPIE",
                "Kopie erstellt am 09.04.2026 19:45",
                "Urspruengliche FDM-Daten :",
            ),
        ] {
            let text = copy_of_sample(lang);
            for expected in [
                copy,
                made,
                original,
                "Event: N; eventCounter: 41;",
                "Event: C; eventCounter: 3;",
            ] {
                assert!(
                    text.contains(expected),
                    "{lang:?}: {expected:?} missing from:\n{text}"
                );
            }
            // The original's block comes after the copy's own.
            assert!(text.find("Event: C;") < text.find("Event: N;"), "{text}");
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
            "pili-pos/1.0.0",
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
        assert!(bytes.windows(23).any(|w| w == b"TICKET DE CAISSE T.V.A."));
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
        let text = crate::render::one_line(&crate::render::render_plain_text(&bytes));
        assert!(text.contains("CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE"));
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
        // The SPF example: rate with 2 decimals, code, base, VAT, gross.
        assert!(text.contains("% TVA"), "VAT header row missing:\n{text}");
        assert!(text.contains("Brut"), "gross column missing:\n{text}");
        let row = text.lines().find(|l| l.starts_with("12,00%")).expect(&text);
        assert_eq!(
            row.split_whitespace().collect::<Vec<_>>(),
            ["12,00%", "B", "26,34", "3,16", "29,50"]
        );
        assert!(text.contains("21,00%"), "VAT rate 21% missing:\n{text}");
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
        // Nothing but out-of-scope lines: no VAT table at all.
        assert!(!text.contains("% TVA"), "{text}");
        assert!(!text.lines().any(|l| l.starts_with("0,00%")), "{text}");
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
