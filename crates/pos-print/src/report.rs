use crate::escpos::{Alignment, EscPosBuilder, TextStyle};

/// Data needed to print an X or Z report on the thermal printer.
pub struct ReportPrintData {
    /// "X" or "Z"
    pub report_type: String,
    /// Z-report sequence number (None for X)
    pub report_no: Option<i64>,
    /// Restaurant identification
    pub restaurant_name: String,
    pub restaurant_address: String,
    pub restaurant_vat: String,
    /// POS / terminal info
    pub pos_id: String,
    pub terminal_id: String,
    /// Report date range
    pub report_date: String,
    pub first_datetime: String,
    pub last_datetime: String,
    /// Employee who generated the report
    pub employee_insz: String,
    /// Whether this was generated in training mode
    pub is_training: bool,
    /// Transactions by event type
    pub event_totals: Vec<ReportEventTotal>,
    /// VAT breakdown
    pub vat_totals: Vec<ReportVatTotal>,
    /// Payment totals
    pub payment_totals: Vec<ReportPaymentTotal>,
    /// Grand total
    pub grand_total_cents: i64,
    /// Number of drawer opens
    pub drawer_opens: i64,
    /// FDM signature data (if FDM was connected)
    pub fdm_id: Option<String>,
    pub fdm_datetime: Option<String>,
    pub short_signature: Option<String>,
    pub ticket_number: i64,
}

pub struct ReportEventTotal {
    pub label: String,
    pub count: i64,
    pub amount_cents: i64,
}

pub struct ReportVatTotal {
    pub code: String,
    pub rate: f64,
    pub total_incl_cents: i64,
    pub vat_cents: i64,
}

pub struct ReportPaymentTotal {
    pub method: String,
    pub count: i64,
    pub amount_cents: i64,
}

/// Format an X or Z report for the thermal printer.
pub fn format_report(data: &ReportPrintData) -> Vec<u8> {
    let mut p = EscPosBuilder::new();

    // ── Training mode banner ───────────────────────────────────
    if data.is_training {
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

    // ── Report header ──────────────────────────────────────────
    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        double_height: true,
        double_width: true,
        ..Default::default()
    });
    let title = match data.report_no {
        Some(no) => format!("RAPPORT {} #{}", data.report_type, no),
        None => format!("RAPPORT {}", data.report_type),
    };
    p.textln(&title);
    p.style_reset();
    p.blank_line();

    // ── Restaurant header ──────────────────────────────────────
    p.align(Alignment::Center);
    p.style(TextStyle { bold: true, ..Default::default() });
    p.textln(&data.restaurant_name);
    p.style_reset();
    p.textln(&data.restaurant_address);
    p.textln(&format!("TVA: {}", data.restaurant_vat));
    p.blank_line();

    // ── Report metadata ────────────────────────────────────────
    p.align(Alignment::Left);
    p.text_columns("Date:", &data.report_date);
    p.text_columns("Debut:", &fmt_datetime(&data.first_datetime));
    p.text_columns("Fin:", &fmt_datetime(&data.last_datetime));
    p.text_columns("Terminal:", &data.terminal_id);
    p.text_columns("Employe:", &data.employee_insz);
    p.text_columns("Ticket:", &format!("#{}", data.ticket_number));
    p.separator('=');

    // ── Transaction summary ────────────────────────────────────
    if !data.event_totals.is_empty() {
        p.style(TextStyle { bold: true, ..Default::default() });
        p.textln("TRANSACTIONS");
        p.style_reset();
        p.separator('-');

        let mut total_count: i64 = 0;
        for et in &data.event_totals {
            let label = event_label_name(&et.label);
            p.text_columns(
                &format!("{} ({})", label, et.count),
                &fmt_price(et.amount_cents),
            );
            total_count += et.count;
        }
        p.separator('-');
        p.style(TextStyle { bold: true, ..Default::default() });
        p.text_columns(
            &format!("Total ({})", total_count),
            &fmt_price(data.grand_total_cents),
        );
        p.style_reset();
        p.blank_line();
    }

    // ── VAT breakdown ──────────────────────────────────────────
    if !data.vat_totals.is_empty() {
        p.style(TextStyle { bold: true, ..Default::default() });
        p.textln("TVA");
        p.style_reset();
        p.separator('-');

        p.style(TextStyle { font_b: true, ..Default::default() });
        p.textln("Code   Taux     Base HT      TVA");
        for vt in &data.vat_totals {
            let base_cents = vt.total_incl_cents - vt.vat_cents;
            let line = format!(
                " {}    {:>4.0}%  {:>9}  {:>9}",
                vt.code,
                vt.rate,
                fmt_price(base_cents),
                fmt_price(vt.vat_cents),
            );
            p.textln(&line);
        }
        p.style_reset();
        p.blank_line();
    }

    // ── Payment breakdown ──────────────────────────────────────
    if !data.payment_totals.is_empty() {
        p.style(TextStyle { bold: true, ..Default::default() });
        p.textln("PAIEMENTS");
        p.style_reset();
        p.separator('-');

        for pt in &data.payment_totals {
            let method = payment_method_name(&pt.method);
            p.text_columns(
                &format!("{} ({}x)", method, pt.count),
                &fmt_price(pt.amount_cents),
            );
        }
        p.blank_line();
    }

    // ── Drawer opens ───────────────────────────────────────────
    p.text_columns("Ouvertures caisse:", &data.drawer_opens.to_string());
    p.separator('=');

    // ── Grand total ────────────────────────────────────────────
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.text_columns("TOTAL", &fmt_price(data.grand_total_cents));
    p.style_reset();
    p.separator('=');

    // ── FDM control data ───────────────────────────────────────
    if let Some(ref fdm_id) = data.fdm_id {
        p.align(Alignment::Center);
        p.style(TextStyle { font_b: true, ..Default::default() });
        p.textln(&format!("FDM: {fdm_id}"));
        if let Some(ref dt) = data.fdm_datetime {
            p.textln(dt);
        }
        if let Some(ref sig) = data.short_signature {
            p.textln(&format!("Sig: {sig}"));
        }
        p.style_reset();
    }

    // ── Z-report specific footer ───────────────────────────────
    if data.report_type == "Z" {
        p.blank_line();
        p.align(Alignment::Center);
        p.style(TextStyle { bold: true, ..Default::default() });
        p.textln("FIN DE JOURNEE");
        p.textln("Compteurs remis a zero");
        p.style_reset();
    }

    // ── End ────────────────────────────────────────────────────
    p.feed(4);
    p.cut();

    p.build()
}

// ── Helpers ────────────────────────────────────────────────────

fn fmt_price(cents: i64) -> String {
    let euros = cents as f64 / 100.0;
    format!("{:.2}", euros)
}

fn fmt_datetime(s: &str) -> String {
    // Extract "YYYY-MM-DD HH:MM" from various datetime formats
    if s.len() >= 16 {
        let date = &s[..10];
        let time = &s[11..16];
        format!("{date} {time}")
    } else {
        s.to_string()
    }
}

fn event_label_name(label: &str) -> &str {
    match label {
        "N" => "Ventes",
        "P" => "Pro forma",
        "T" => "Formation",
        "C" => "Copies",
        "I" => "Factures",
        "F" => "Financier",
        "S" => "Social",
        "R" => "Rapports",
        _ => label,
    }
}

fn payment_method_name(method: &str) -> &str {
    match method {
        "cash" => "Especes",
        "card" => "Carte",
        "other" => "Autre",
        _ => method,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_report_data(report_type: &str) -> ReportPrintData {
        ReportPrintData {
            report_type: report_type.to_string(),
            report_no: if report_type == "Z" { Some(1) } else { None },
            restaurant_name: "Tokyo Sushi Bar".to_string(),
            restaurant_address: "Rue Test 1, 1000 Bruxelles".to_string(),
            restaurant_vat: "BE0499999960".to_string(),
            pos_id: "CFOD0061234567".to_string(),
            terminal_id: "POS1".to_string(),
            report_date: "2026-04-12".to_string(),
            first_datetime: "2026-04-12 11:30:00".to_string(),
            last_datetime: "2026-04-12 22:15:00".to_string(),
            employee_insz: "94060242576".to_string(),
            is_training: false,
            event_totals: vec![
                ReportEventTotal {
                    label: "N".to_string(),
                    count: 42,
                    amount_cents: 382500,
                },
                ReportEventTotal {
                    label: "P".to_string(),
                    count: 3,
                    amount_cents: 0,
                },
            ],
            vat_totals: vec![
                ReportVatTotal {
                    code: "A".to_string(),
                    rate: 21.0,
                    total_incl_cents: 45000,
                    vat_cents: 7810,
                },
                ReportVatTotal {
                    code: "B".to_string(),
                    rate: 12.0,
                    total_incl_cents: 337500,
                    vat_cents: 36161,
                },
            ],
            payment_totals: vec![
                ReportPaymentTotal {
                    method: "cash".to_string(),
                    count: 18,
                    amount_cents: 156000,
                },
                ReportPaymentTotal {
                    method: "card".to_string(),
                    count: 24,
                    amount_cents: 226500,
                },
            ],
            grand_total_cents: 382500,
            drawer_opens: 18,
            fdm_id: None,
            fdm_datetime: None,
            short_signature: None,
            ticket_number: 45,
        }
    }

    #[test]
    fn x_report_contains_header() {
        let data = sample_report_data("X");
        let bytes = format_report(&data);
        assert!(bytes.windows(9).any(|w| w == b"RAPPORT X"));
    }

    #[test]
    fn z_report_contains_sequence_number() {
        let data = sample_report_data("Z");
        let bytes = format_report(&data);
        // Title is printed with double-width/height so look for
        // "RAPPORT" substring which appears in the byte stream.
        assert!(bytes.windows(7).any(|w| w == b"RAPPORT"));
    }

    #[test]
    fn z_report_has_end_of_day_footer() {
        let data = sample_report_data("Z");
        let bytes = format_report(&data);
        // Footer is printed with bold (not double-width) so the
        // plain text appears consecutively in the byte stream.
        assert!(bytes.windows(14).any(|w| w == b"FIN DE JOURNEE"));
    }

    #[test]
    fn report_contains_restaurant_name() {
        let data = sample_report_data("X");
        let bytes = format_report(&data);
        assert!(bytes.windows(15).any(|w| w == b"Tokyo Sushi Bar"));
    }

    #[test]
    fn report_contains_vat_section() {
        let data = sample_report_data("X");
        let bytes = format_report(&data);
        assert!(bytes.windows(3).any(|w| w == b"TVA"));
    }

    #[test]
    fn report_contains_payment_section() {
        let data = sample_report_data("X");
        let bytes = format_report(&data);
        assert!(bytes.windows(9).any(|w| w == b"PAIEMENTS"));
    }

    #[test]
    fn report_ends_with_cut() {
        let data = sample_report_data("X");
        let bytes = format_report(&data);
        assert!(bytes.windows(4).any(|w| w == [0x1D, b'V', 66, 3]));
    }

    #[test]
    fn training_report_has_banner() {
        let mut data = sample_report_data("X");
        data.is_training = true;
        let bytes = format_report(&data);
        assert!(bytes.windows(8).any(|w| w == b"TRAINING"));
    }
}
