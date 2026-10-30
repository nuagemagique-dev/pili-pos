use pos_fiscal::ticket::{ReceiptLanguage, invalid_disclaimer, ticket_type_label};
use pos_fiscal::types::EventLabel;

use serde::{Deserialize, Serialize};

use crate::escpos::{Alignment, EscPosBuilder, PaperWidth, TextStyle};
use crate::labels::{event_label_name, labels, payment_label, report_labels};

/// Data needed to print an X or Z report on the thermal printer.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Payment totals (payment lines, rounding excluded)
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
    /// Language of the printed mentions (§VI, like receipts).
    pub lang: ReceiptLanguage,
    /// The rest of the content of AM of 29 April 2024 art. 29.
    pub details: ReportDetails,
}

/// Report content required by art. 29 beyond the totals above (SPF
/// example Z report of 2026-09-24).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportDetails {
    /// Establishment unit number (art. 29 c).
    pub restaurant_est_no: String,
    pub booking_period_id: String,
    /// When the report was made (art. 29 d), Belgian time.
    pub created_at: String,
    /// The first and last event of the FDM in the report (art. 29 f,
    /// x): total counter and FDM time.
    pub fdm_first_event: Option<(i64, String)>,
    pub fdm_last_event: Option<(i64, String)>,
    /// Turnover per department, VAT included (art. 29 j).
    pub departments: Vec<(String, i64)>,
    /// Corrections that lowered the turnover (art. 29 v): amount.
    pub corrections_cents: i64,
    /// Refunds: tickets and amount (art. 29 s).
    pub refunds: (i64, i64),
    /// Training tickets: count and amount (art. 29 r).
    pub training: (i64, i64),
    /// Reductions per type: count and amount (art. 29 u).
    pub line_reductions: (i64, i64),
    pub event_reductions: (i64, i64),
    /// Invoices: number and amount (art. 29 w).
    pub invoices: Vec<(String, i64)>,
    /// Total of the rounding payment lines (art. 29 o).
    pub rounding_cents: i64,
    /// Payment method corrections (event F): how many, and the amount
    /// moved to another method. Absent in reports stored before.
    #[serde(default)]
    pub payment_corrections: (i64, i64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportEventTotal {
    pub label: String,
    pub count: i64,
    pub amount_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportVatTotal {
    pub code: String,
    pub rate: f64,
    pub total_incl_cents: i64,
    pub vat_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportPaymentTotal {
    pub method: String,
    pub count: i64,
    pub amount_cents: i64,
}

/// A centered bold section title.
fn section(p: &mut EscPosBuilder, title: &str) {
    p.separator('-');
    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln(title);
    p.style_reset();
    p.align(Alignment::Left);
}

/// A row: label, optional count, amount on the right.
fn count_row(p: &mut EscPosBuilder, label: &str, count: i64, amount_cents: i64) {
    p.text_columns(&format!("{label} {count}"), &fmt_price(amount_cents));
}

/// The four Belgian VAT rates, in the order of the report.
const VAT_RATES: [(&str, f64); 4] = [("A", 21.0), ("B", 12.0), ("C", 6.0), ("D", 0.0)];

/// Format an X or Z turnover report for the thermal printer, in the
/// order of the SPF example Z report of 2026-09-24 (AM of 29 April
/// 2024 art. 29).
pub fn format_report(data: &ReportPrintData, paper: PaperWidth) -> Vec<u8> {
    let mut p = EscPosBuilder::for_paper(paper);
    let l = labels(data.lang);
    let rl = report_labels(data.lang);
    let d = &data.details;

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

    // ── Title: "Z CHIFFRE D'AFFAIRES" (art. 29) ────────────────
    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.textln_wrapped(&format!("{} {}", data.report_type, rl.turnover));
    p.style_reset();

    // ── Issuer (art. 29 a, b, c) ───────────────────────────────
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln_wrapped(&data.restaurant_name);
    p.style_reset();
    p.textln_wrapped(&data.restaurant_address);
    p.textln_wrapped(&format!("{}: {}", l.vat_no, data.restaurant_vat));
    if !d.restaurant_est_no.is_empty() {
        p.textln_wrapped(&format!("{} {}", l.est_unit, d.restaurant_est_no));
    }
    p.separator('-');

    // ── Report identification (art. 29 d, y) ───────────────────
    p.align(Alignment::Left);
    let report_word = ticket_type_label(EventLabel::R, data.lang);
    p.text_columns(
        &format!("{report_word} {}", data.report_type),
        &d.created_at,
    );
    if let Some(no) = data.report_no {
        p.textln_wrapped(&format!("{} {no}", rl.report_no));
    }
    p.text_columns(l.ticket, &format!("{:06}", data.ticket_number));
    p.text_columns(rl.employee, &data.employee_insz);

    // ── Opening period (art. 29 e, f, g, h, x) ─────────────────
    section(&mut p, rl.opening_period);
    p.text_columns(l.date, &data.report_date);
    if !d.booking_period_id.is_empty() {
        p.textln("BookingPeriodId:");
        p.textln(&d.booking_period_id);
    }
    p.text_columns(rl.start, &fmt_datetime(&data.first_datetime));
    p.text_columns(rl.end, &fmt_datetime(&data.last_datetime));
    p.text_columns("POSID", &data.pos_id);
    p.text_columns("TerminalId", &data.terminal_id);
    if let Some(ref fdm_id) = data.fdm_id {
        p.text_columns("FDMID", fdm_id);
    }
    for (label, event) in [(rl.first, &d.fdm_first_event), (rl.last, &d.fdm_last_event)] {
        if let Some((counter, at)) = event {
            p.text_columns(label, &format!("{counter} {}", fmt_datetime(at)));
        }
    }

    // ── Turnover per event label (art. 29 i, t) ────────────────
    p.separator('-');
    for et in &data.event_totals {
        let name = event_label_name(&et.label, data.lang);
        count_row(
            &mut p,
            &format!("Event {} {name}", et.label),
            et.count,
            et.amount_cents,
        );
    }

    // ── VAT overview, every rate (art. 29 k, l, m) ─────────────
    section(&mut p, rl.vat_overview);
    let [rate, base, vat, total] = rl.vat_columns;
    p.textln(&vat_row(paper, rate, base, vat, total));
    let (mut sum_base, mut sum_vat, mut sum_total) = (0, 0, 0);
    for (code, default_rate) in VAT_RATES {
        match data.vat_totals.iter().find(|v| v.code == code) {
            Some(v) => {
                let base_cents = v.total_incl_cents - v.vat_cents;
                sum_base += base_cents;
                sum_vat += v.vat_cents;
                sum_total += v.total_incl_cents;
                p.textln(&vat_row(
                    paper,
                    &format!("{}%", fmt_rate(v.rate)),
                    &fmt_price(base_cents),
                    &fmt_price(v.vat_cents),
                    &fmt_price(v.total_incl_cents),
                ));
            }
            None => {
                p.textln(&vat_row(
                    paper,
                    &format!("{}%", fmt_rate(default_rate)),
                    "-",
                    "-",
                    "-",
                ));
            }
        }
    }
    p.textln(&vat_row(
        paper,
        rl.total_word,
        &fmt_price(sum_base),
        &fmt_price(sum_vat),
        &fmt_price(sum_total),
    ));

    // ── Departments (art. 29 j) ────────────────────────────────
    if !d.departments.is_empty() {
        section(&mut p, rl.departments);
        for (name, amount) in &d.departments {
            p.text_columns(name, &fmt_price(*amount));
        }
    }

    // ── Corrections and refunds (art. 29 s, v) ─────────────────
    section(&mut p, rl.corrections);
    p.text_columns(rl.corrections, &fmt_price(d.corrections_cents));
    section(&mut p, rl.refunds);
    count_row(&mut p, rl.refunds, d.refunds.0, d.refunds.1);

    // ── Drawer opened without a sale (art. 29 q) ───────────────
    section(&mut p, rl.drawer);
    p.textln_wrapped(&format!("{} {}", rl.drawer, data.drawer_opens));

    // ── Training tickets (art. 29 r) ───────────────────────────
    section(&mut p, rl.training);
    count_row(&mut p, rl.training_tickets, d.training.0, d.training.1);

    // ── Reductions per type (art. 29 u) ────────────────────────
    section(&mut p, rl.reductions);
    count_row(
        &mut p,
        rl.line_reductions,
        d.line_reductions.0,
        d.line_reductions.1,
    );
    count_row(
        &mut p,
        rl.event_reductions,
        d.event_reductions.0,
        d.event_reductions.1,
    );

    // ── Invoices (art. 29 w) ───────────────────────────────────
    section(&mut p, rl.invoices_title);
    if d.invoices.is_empty() {
        p.textln_wrapped(&format!("{} 0", rl.invoices));
    }
    for (no, amount) in &d.invoices {
        p.text_columns(&format!("{} {no}", rl.invoice_no), &fmt_price(*amount));
    }

    // ── Payments recorded, rounding apart (art. 29 n, o) ───────
    section(&mut p, rl.payments_title);
    for pt in &data.payment_totals {
        let method = payment_label(&pt.method, data.lang);
        p.text_columns(&method, &fmt_price(pt.amount_cents));
    }
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.text_columns(rl.payments_total, &fmt_price(data.grand_total_cents));
    p.style_reset();
    p.text_columns(rl.roundings, &fmt_price(d.rounding_cents));
    if d.payment_corrections.0 > 0 {
        count_row(
            &mut p,
            rl.payment_corrections,
            d.payment_corrections.0,
            d.payment_corrections.1,
        );
    }

    // ── FDM control data of the report event ───────────────────
    if data.fdm_datetime.is_some() || data.short_signature.is_some() {
        p.separator('-');
        p.align(Alignment::Center);
        p.style(TextStyle {
            font_b: true,
            ..Default::default()
        });
        if let Some(ref dt) = data.fdm_datetime {
            p.textln_wrapped(&format!("fdmDateTime: {dt};"));
        }
        if let Some(ref sig) = data.short_signature {
            p.textln_wrapped(&format!("shortSignature: {sig};"));
        }
        p.style_reset();
    }

    // ── Not a VAT ticket ───────────────────────────────────────
    p.separator('=');
    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln_wrapped(invalid_disclaimer(data.lang));
    p.style_reset();

    // ── End ────────────────────────────────────────────────────
    p.feed(4);
    p.cut();

    p.build()
}

/// One row of the VAT overview: rate, base, VAT, total, across the
/// whole width of `paper`.
fn vat_row(paper: PaperWidth, rate: &str, base: &str, vat: &str, total: &str) -> String {
    match paper {
        PaperWidth::Mm80 => format!("{rate:<9}{base:>13}{vat:>13}{total:>13}"),
        PaperWidth::Mm58 => format!("{rate:<7}{base:>9}{vat:>9}{total:>10}"),
    }
}

/// A VAT rate without needless decimals ("21", "5,5").
fn fmt_rate(rate: f64) -> String {
    if (rate - rate.round()).abs() < f64::EPSILON {
        format!("{}", rate as i64)
    } else {
        format!("{rate}").replace('.', ",")
    }
}

// ── User report (M182 X / M183 Z / M184 combined Z) ────────────

/// Data needed to print a user report on the thermal printer.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Language of the printed mentions (§VI, like receipts).
    pub lang: ReceiptLanguage,
}

/// Per-operator block of the user report.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub fn format_user_report(data: &UserReportPrintData, paper: PaperWidth) -> Vec<u8> {
    let mut p = EscPosBuilder::for_paper(paper);
    let l = labels(data.lang);
    let rl = report_labels(data.lang);

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
        Some(no) => format!("{} {} #{}", rl.user_report_prefix, data.report_type, no),
        None => format!("{} {}", rl.user_report_prefix, data.report_type),
    };
    p.textln(&title);
    p.style_reset();
    p.blank_line();

    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln_wrapped(&data.restaurant_name);
    p.style_reset();
    p.textln_wrapped(&data.restaurant_address);
    p.textln_wrapped(&format!("{}: {}", l.vat_no, data.restaurant_vat));
    p.blank_line();

    p.align(Alignment::Left);
    p.text_columns(l.date, &data.report_date);
    p.text_columns(l.terminal, &data.terminal_id);
    p.text_columns(rl.employee, &data.employee_insz);
    p.text_columns(l.ticket, &format!("#{}", data.ticket_number));
    p.separator('=');

    for user in &data.users {
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.text_columns(&format!("INSZ {}", user.insz), &fmt_price(user.total_cents));
        p.style_reset();
        p.text_columns(
            &format!("  {}", rl.first_event),
            &fmt_datetime(&user.first_datetime),
        );
        p.text_columns(
            &format!("  {}", rl.last_event),
            &fmt_datetime(&user.last_datetime),
        );
        for (op, dt) in &user.social_events {
            let label = match op.as_str() {
                "WORK_IN" => format!("  {}", rl.punch_in),
                "WORK_OUT" => format!("  {}", rl.punch_out),
                other => other.to_string(),
            };
            p.text_columns(&label, &fmt_datetime(dt));
        }
        for pt in &user.payments {
            let method = payment_label(&pt.method, data.lang);
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
    p.text_columns(l.total, &fmt_price(grand_total));
    p.style_reset();
    p.separator('=');

    if let Some(ref fdm_id) = data.fdm_id {
        p.align(Alignment::Center);
        p.style(TextStyle {
            font_b: true,
            ..Default::default()
        });
        p.textln_wrapped(&format!("FDM: {fdm_id}"));
        if let Some(ref dt) = data.fdm_datetime {
            p.textln(dt);
        }
        if let Some(ref sig) = data.short_signature {
            p.textln_wrapped(&format!("Sig: {sig}"));
        }
        p.style_reset();
    }

    p.feed(4);
    p.cut();
    p.build()
}

// ── Helpers ────────────────────────────────────────────────────

fn fmt_price(cents: i64) -> String {
    // Belgian notation: decimal comma.
    format!("{:.2}", cents as f64 / 100.0).replace('.', ",")
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
            lang: ReceiptLanguage::Fr,
            details: Default::default(),
        }
    }

    #[test]
    fn x_report_contains_header() {
        let data = sample_report_data("X");
        let bytes = format_report(&data, PaperWidth::Mm80);
        assert!(bytes.windows(9).any(|w| w == b"RAPPORT X"));
    }

    #[test]
    fn z_report_contains_sequence_number() {
        let data = sample_report_data("Z");
        let bytes = format_report(&data, PaperWidth::Mm80);
        // Title is printed with double-width/height so look for
        // "RAPPORT" substring which appears in the byte stream.
        assert!(bytes.windows(7).any(|w| w == b"RAPPORT"));
    }

    #[test]
    fn a_report_says_it_is_not_a_vat_ticket() {
        for kind in ["X", "Z"] {
            let text = crate::render::render_plain_text(&format_report(
                &sample_report_data(kind),
                PaperWidth::Mm80,
            ));
            assert!(
                crate::render::one_line(&text)
                    .contains("CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE"),
                "{text}"
            );
        }
    }

    #[test]
    fn report_contains_restaurant_name() {
        let data = sample_report_data("X");
        let bytes = format_report(&data, PaperWidth::Mm80);
        assert!(bytes.windows(15).any(|w| w == b"Tokyo Sushi Bar"));
    }

    #[test]
    fn report_contains_vat_section() {
        let data = sample_report_data("X");
        let bytes = format_report(&data, PaperWidth::Mm80);
        assert!(bytes.windows(3).any(|w| w == b"TVA"));
    }

    #[test]
    fn report_contains_payment_section() {
        let data = sample_report_data("X");
        let bytes = format_report(&data, PaperWidth::Mm80);
        assert!(bytes.windows(9).any(|w| w == b"PAIEMENTS"));
    }

    #[test]
    fn report_ends_with_cut() {
        let data = sample_report_data("X");
        let bytes = format_report(&data, PaperWidth::Mm80);
        assert!(bytes.windows(4).any(|w| w == [0x1D, b'V', 66, 3]));
    }

    #[test]
    fn training_report_has_banner() {
        let mut data = sample_report_data("X");
        data.is_training = true;
        let bytes = format_report(&data, PaperWidth::Mm80);
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
            lang: ReceiptLanguage::Fr,
        }
    }

    #[test]
    fn user_report_contains_insz_and_pointages() {
        let bytes = format_user_report(&sample_user_report("Z"), PaperWidth::Mm80);
        assert!(bytes.windows(16).any(|w| w == b"INSZ 12345678901"));
        assert!(bytes.windows(12).any(|w| w == b"Pointage IN:"));
        assert!(bytes.windows(13).any(|w| w == b"Pointage OUT:"));
    }

    #[test]
    fn user_report_has_fdm_block_and_cut() {
        let bytes = format_user_report(&sample_user_report("X"), PaperWidth::Mm80);
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
        let text = crate::render::render_plain_text(&format_report(&data, PaperWidth::Mm80));
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
        let text = crate::render::render_plain_text(&format_user_report(&data, PaperWidth::Mm80));
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
        let bytes = format_user_report(&data, PaperWidth::Mm80);
        assert!(bytes.windows(8).any(|w| w == b"TRAINING"));
    }

    #[test]
    fn x_report_has_no_report_no_but_z_does() {
        let x = format_report(&sample_report_data("X"), PaperWidth::Mm80);
        let x_text = crate::render::render_plain_text(&x);
        assert!(x_text.contains("X CHIFFRE D'AFFAIRES"));
        assert!(!x_text.contains("Rapport nr."));

        let z = format_report(&sample_report_data("Z"), PaperWidth::Mm80);
        let z_text = crate::render::render_plain_text(&z);
        assert!(z_text.contains("Z CHIFFRE D'AFFAIRES"));
        assert!(z_text.contains("Rapport nr. 1"));
    }
}
