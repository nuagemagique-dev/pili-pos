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
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
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
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
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
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.text_columns(
            &format!("Total ({})", total_count),
            &fmt_price(data.grand_total_cents),
        );
        p.style_reset();
        p.blank_line();
    }

    // ── VAT breakdown ──────────────────────────────────────────
    if !data.vat_totals.is_empty() {
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.textln("TVA");
        p.style_reset();
        p.separator('-');

        p.style(TextStyle {
            font_b: true,
            ..Default::default()
        });
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
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
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
        p.style(TextStyle {
            font_b: true,
            ..Default::default()
        });
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
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.textln("FIN DE JOURNEE");
        p.textln("Compteurs remis a zero");
        p.style_reset();
    }

    // ── End ────────────────────────────────────────────────────
    p.feed(4);
    p.cut();

    p.build()
}

// ── User report (M182 X / M183 Z / M184 combined Z) ────────────

/// Data needed to print a user report on the thermal printer.
pub struct UserReportPrintData {
    /// "X" or "Z"
    pub report_type: String,
    /// Z-report sequence number (None for X)
    pub report_no: Option<i64>,
    pub restaurant_name: String,
    pub restaurant_address: String,
    pub restaurant_vat: String,
    pub terminal_id: String,
    pub report_date: String,
    pub employee_insz: String,
    pub is_training: bool,
    pub users: Vec<UserReportUser>,
    pub fdm_id: Option<String>,
    pub fdm_datetime: Option<String>,
    pub short_signature: Option<String>,
    pub ticket_number: i64,
}

/// Per-operator block of the user report.
pub struct UserReportUser {
    pub insz: String,
    pub first_datetime: String,
    pub last_datetime: String,
    pub total_cents: i64,
    pub payments: Vec<ReportPaymentTotal>,
    /// ("WORK_IN"/"WORK_OUT", posDateTime) registrations of the day.
    pub social_events: Vec<(String, String)>,
}

/// Format a user report (per-operator turnover + presence) for the
/// thermal printer. Printed as a second sequential ticket after the
/// financial Z (M184) or on demand for the user X (M182).
pub fn format_user_report(data: &UserReportPrintData) -> Vec<u8> {
    let mut p = EscPosBuilder::new();

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

    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        double_height: true,
        double_width: true,
        ..Default::default()
    });
    let title = match data.report_no {
        Some(no) => format!("RAPPORT UTIL. {} #{}", data.report_type, no),
        None => format!("RAPPORT UTIL. {}", data.report_type),
    };
    p.textln(&title);
    p.style_reset();
    p.blank_line();

    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln(&data.restaurant_name);
    p.style_reset();
    p.textln(&data.restaurant_address);
    p.textln(&format!("TVA: {}", data.restaurant_vat));
    p.blank_line();

    p.align(Alignment::Left);
    p.text_columns("Date:", &data.report_date);
    p.text_columns("Terminal:", &data.terminal_id);
    p.text_columns("Employe:", &data.employee_insz);
    p.text_columns("Ticket:", &format!("#{}", data.ticket_number));
    p.separator('=');

    for user in &data.users {
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.text_columns(&format!("INSZ {}", user.insz), &fmt_price(user.total_cents));
        p.style_reset();
        p.text_columns("  Premier event:", &fmt_datetime(&user.first_datetime));
        p.text_columns("  Dernier event:", &fmt_datetime(&user.last_datetime));
        for (op, dt) in &user.social_events {
            let label = match op.as_str() {
                "WORK_IN" => "  Pointage IN:",
                "WORK_OUT" => "  Pointage OUT:",
                other => other,
            };
            p.text_columns(label, &fmt_datetime(dt));
        }
        for pt in &user.payments {
            let method = payment_method_name(&pt.method);
            p.text_columns(
                &format!("  {} ({}x)", method, pt.count),
                &fmt_price(pt.amount_cents),
            );
        }
        p.separator('-');
    }

    let grand_total: i64 = data.users.iter().map(|u| u.total_cents).sum();
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.text_columns("TOTAL", &fmt_price(grand_total));
    p.style_reset();
    p.separator('=');

    if let Some(ref fdm_id) = data.fdm_id {
        p.align(Alignment::Center);
        p.style(TextStyle {
            font_b: true,
            ..Default::default()
        });
        p.textln(&format!("FDM: {fdm_id}"));
        if let Some(ref dt) = data.fdm_datetime {
            p.textln(dt);
        }
        if let Some(ref sig) = data.short_signature {
            p.textln(&format!("Sig: {sig}"));
        }
        p.style_reset();
    }

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

    fn sample_user_report(report_type: &str) -> UserReportPrintData {
        UserReportPrintData {
            report_type: report_type.to_string(),
            report_no: if report_type == "Z" { Some(1) } else { None },
            restaurant_name: "Tokyo Sushi Bar".to_string(),
            restaurant_address: "Rue Test 1, 1000 Bruxelles".to_string(),
            restaurant_vat: "BE0499999960".to_string(),
            terminal_id: "POS1".to_string(),
            report_date: "2026-04-12".to_string(),
            employee_insz: "94060242576".to_string(),
            is_training: false,
            users: vec![UserReportUser {
                insz: "12345678901".to_string(),
                first_datetime: "2026-04-12 11:30:00".to_string(),
                last_datetime: "2026-04-12 22:15:00".to_string(),
                total_cents: 12500,
                payments: vec![ReportPaymentTotal {
                    method: "cash".to_string(),
                    count: 3,
                    amount_cents: 12500,
                }],
                social_events: vec![
                    ("WORK_IN".to_string(), "2026-04-12 11:30:00".to_string()),
                    ("WORK_OUT".to_string(), "2026-04-12 22:15:00".to_string()),
                ],
            }],
            fdm_id: Some("CBX01000042".to_string()),
            fdm_datetime: Some("2026-04-12T22:16:00+02:00".to_string()),
            short_signature: None,
            ticket_number: 46,
        }
    }

    #[test]
    fn user_report_contains_insz_and_pointages() {
        let bytes = format_user_report(&sample_user_report("Z"));
        assert!(bytes.windows(16).any(|w| w == b"INSZ 12345678901"));
        assert!(bytes.windows(12).any(|w| w == b"Pointage IN:"));
        assert!(bytes.windows(13).any(|w| w == b"Pointage OUT:"));
    }

    #[test]
    fn user_report_has_fdm_block_and_cut() {
        let bytes = format_user_report(&sample_user_report("X"));
        assert!(bytes.windows(16).any(|w| w == b"FDM: CBX01000042"));
        assert!(bytes.windows(4).any(|w| w == [0x1D, b'V', 66, 3]));
    }

    // ── Width invariant on transactional rows ───────────────────

    #[test]
    fn report_rows_never_exceed_paper_width_even_with_long_payment_names() {
        let mut data = sample_report_data("X");
        data.payment_totals.push(ReportPaymentTotal {
            method: "Some Very Long Custom Payment Provider Name".to_string(),
            count: 999999,
            amount_cents: i64::MAX / 2,
        });
        let text = crate::render::render_plain_text(&format_report(&data));
        for line in text.lines() {
            assert!(
                line.chars().count() <= 48,
                "line exceeds 48 columns ({} chars): {line:?}",
                line.chars().count()
            );
        }
    }

    #[test]
    fn user_report_rows_never_exceed_paper_width_even_with_long_insz_context() {
        let mut data = sample_user_report("Z");
        data.users[0].payments.push(ReportPaymentTotal {
            method: "Another Extremely Long Payment Method Label".to_string(),
            count: 1,
            amount_cents: 999999999,
        });
        let text = crate::render::render_plain_text(&format_user_report(&data));
        for line in text.lines() {
            assert!(
                line.chars().count() <= 48,
                "line exceeds 48 columns ({} chars): {line:?}",
                line.chars().count()
            );
        }
    }

    #[test]
    fn training_user_report_has_banner() {
        let mut data = sample_user_report("X");
        data.is_training = true;
        let bytes = format_user_report(&data);
        assert!(bytes.windows(8).any(|w| w == b"TRAINING"));
    }

    #[test]
    fn x_report_has_no_report_no_but_z_does() {
        let x = format_report(&sample_report_data("X"));
        let x_text = crate::render::render_plain_text(&x);
        assert!(x_text.contains("RAPPORT X"));
        assert!(!x_text.contains("RAPPORT X #"));

        let z = format_report(&sample_report_data("Z"));
        let z_text = crate::render::render_plain_text(&z);
        assert!(z_text.contains("RAPPORT Z #1"));
    }

    #[test]
    fn x_report_has_no_end_of_day_footer() {
        let bytes = format_report(&sample_report_data("X"));
        assert!(!bytes.windows(14).any(|w| w == b"FIN DE JOURNEE"));
    }
}
