//! The printed B2B invoice (event I) and its copy (V11-D21).
//!
//! POS description v2.2 section 1.7 and AM of 29 April 2024 art. 32: the print says FACTURE
//! and, at the bottom, that it is not a valid VAT ticket; it carries the mentions of AR
//! no. 1 art. 5 (seller and customer name, address and VAT number), the continuous invoice
//! number, the references of the original tickets (`fdmId-N eventCounter/totalCounter`),
//! their product lines (payment lines are not copied) and the FDM control data of the I event.
//! AM art. 43: no digital signature and no QR code on this print, so the control data stops
//! at the counters. A copy (event C of the I) prints the same layout under COPIE with its own
//! control data and "copie etablie le" (section 8.1).
//!
//! Money is in cents; this module formats, it never computes VAT.

use pos_fiscal::ticket::{ReceiptLanguage, invalid_disclaimer};
use serde::{Deserialize, Serialize};

use crate::escpos::{Alignment, EscPosBuilder, TextStyle, wrap};
use crate::labels::labels;

/// The customer as printed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceParty {
    pub name: String,
    pub street: String,
    pub postal_code: String,
    pub city: String,
    pub country_code: String,
    pub vat_number: String,
}

/// One original ticket (event N) the invoice covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceSaleRef {
    pub ticket_number: i64,
    /// RFC 3339 `posDateTime` of the ticket.
    pub pos_date_time: String,
    pub fdm_id: String,
    pub event_label: String,
    pub event_counter: i64,
    pub total_counter: i64,
    pub total_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceLinePrint {
    /// Quantity in thousandths (2 pieces = 2000). Negative on a refund.
    pub quantity_milli: i64,
    pub description: String,
    pub total_cents: i64,
    pub vat_label: String,
}

/// One VAT row as the FDM computed it for the tickets, summed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceVatRow {
    pub label: String,
    /// The rate in hundredths of a percent (21 % = 2100).
    pub rate_hundredths: i64,
    pub base_cents: i64,
    pub vat_cents: i64,
    pub total_cents: i64,
}

/// The FDM control data of an event, without its signature (AM art. 43).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceControl {
    pub fdm_id: String,
    pub fdm_date_time: String,
    pub event_label: String,
    pub event_counter: i64,
    pub total_counter: i64,
    pub pos_fiscal_ticket_no: i64,
    pub pos_date_time: String,
}

/// What a copy adds to the invoice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceCopy {
    /// `posDateTime` of the copy event.
    pub made_at: String,
    pub control: InvoiceControl,
}

/// Everything the invoice print shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoicePrint {
    pub lang: ReceiptLanguage,
    pub seller_name: String,
    pub seller_legal_name: String,
    pub seller_address: String,
    pub seller_vat: String,
    pub seller_est_no: String,
    pub customer: InvoiceParty,
    pub invoice_no: String,
    /// `YYYY-MM-DD`.
    pub issue_date: String,
    pub sales: Vec<InvoiceSaleRef>,
    pub lines: Vec<InvoiceLinePrint>,
    pub vat: Vec<InvoiceVatRow>,
    pub total_cents: i64,
    pub operator: String,
    pub pos_id: String,
    pub pos_sw_version: String,
    pub terminal_id: String,
    /// The I event's control data. `None` only in a draft kept before the FDM answers.
    pub control: Option<InvoiceControl>,
    /// Set on a copy.
    pub copy: Option<InvoiceCopy>,
}

struct InvoiceLabels {
    title: &'static str,
    invoice_no: &'static str,
    issued: &'static str,
    customer: &'static str,
    originals: &'static str,
    ticket: &'static str,
    already_paid: &'static str,
}

fn invoice_labels(lang: ReceiptLanguage) -> &'static InvoiceLabels {
    match lang {
        ReceiptLanguage::Fr => &InvoiceLabels {
            title: "FACTURE",
            invoice_no: "Facture nr.",
            issued: "Date de facture:",
            customer: "Client :",
            originals: "Tickets d'origine :",
            ticket: "Ticket",
            already_paid: "Deja paye (tickets de caisse ci-dessus)",
        },
        ReceiptLanguage::Nl => &InvoiceLabels {
            title: "FACTUUR",
            invoice_no: "Factuur nr.",
            issued: "Factuurdatum:",
            customer: "Klant :",
            originals: "Oorspronkelijke tickets :",
            ticket: "Ticket",
            already_paid: "Reeds betaald (kastickets hierboven)",
        },
        ReceiptLanguage::De => &InvoiceLabels {
            title: "RECHNUNG",
            invoice_no: "Rechnung Nr.",
            issued: "Rechnungsdatum:",
            customer: "Kunde :",
            originals: "Urspruengliche Bons :",
            ticket: "Bon",
            already_paid: "Bereits bezahlt (Kassenbons oben)",
        },
    }
}

/// `1234` -> `12,34`, `-5` -> `-0,05`.
fn price(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    format!("{sign}{},{:02}", cents.abs() / 100, cents.abs() % 100)
}

fn quantity(milli: i64) -> String {
    if milli % 1000 == 0 {
        (milli / 1000).to_string()
    } else {
        let sign = if milli < 0 { "-" } else { "" };
        format!("{sign}{},{:03}", milli.abs() / 1000, milli.abs() % 1000)
    }
}

fn rate(hundredths: i64) -> String {
    format!("{},{:02}%", hundredths / 100, hundredths % 100)
}

/// "23.06.2026" and "11:35" from an RFC 3339 date-time; anything shorter prints as it is.
fn date_and_time(rfc3339: &str) -> (String, String) {
    match (rfc3339.get(..10), rfc3339.get(11..16)) {
        (Some(date), Some(time)) if date.len() == 10 && date.as_bytes()[4] == b'-' => {
            // `get` again: a non-ASCII date text has no char boundary at these offsets.
            match (date.get(..4), date.get(5..7), date.get(8..10)) {
                (Some(y), Some(m), Some(d)) => (format!("{d}.{m}.{y}"), time.to_string()),
                _ => (rfc3339.to_string(), String::new()),
            }
        }
        _ => (rfc3339.to_string(), String::new()),
    }
}

/// "17.10.2026" from "2026-10-17".
fn date_only(iso: &str) -> String {
    match iso.get(..10) {
        Some(date) if date.len() == 10 && date.as_bytes()[4] == b'-' => {
            match (date.get(8..10), date.get(5..7), date.get(..4)) {
                (Some(d), Some(m), Some(y)) => format!("{d}.{m}.{y}"),
                _ => iso.to_string(),
            }
        }
        _ => iso.to_string(),
    }
}

fn control_data(p: &mut EscPosBuilder, c: &InvoiceControl) {
    p.textln(&format!("fdmId: {};", c.fdm_id));
    p.textln(&format!("fdmDateTime: {};", c.fdm_date_time));
    p.textln(&format!(
        "Event: {}; eventCounter: {};",
        c.event_label, c.event_counter
    ));
    p.textln(&format!("totalCounter: {};", c.total_counter));
}

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

fn vat_row(rate: &str, code: &str, base: &str, vat: &str, gross: &str) -> String {
    format!("{rate:<8}{code:<6}{base:>10}{vat:>10}{gross:>10}")
}

/// Format an invoice (or its copy) into ESC/POS commands.
pub fn format_invoice(inv: &InvoicePrint) -> Vec<u8> {
    let mut p = EscPosBuilder::new();
    let l = labels(inv.lang);
    let il = invoice_labels(inv.lang);

    // Title and its mentions.
    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.textln(il.title);
    p.style_reset();
    if inv.copy.is_some() {
        p.style(TextStyle {
            bold: true,
            ..Default::default()
        });
        p.textln(l.copy);
        p.style_reset();
    }

    // Seller, number, date, operator.
    p.align(Alignment::Left);
    p.textln_wrapped(&inv.seller_name);
    if !inv.seller_est_no.is_empty() {
        p.textln(&format!("{} {}", l.est_unit, inv.seller_est_no));
    }
    p.textln(&format!("{} {}", il.invoice_no, inv.invoice_no));
    p.textln(&format!("{} {}", il.issued, date_only(&inv.issue_date)));
    if !inv.operator.is_empty() {
        p.textln(&format!("{} {}", l.user, inv.operator));
    }
    p.textln(&format!("{} {}", l.terminal, inv.terminal_id));
    p.textln(&format!("{} {}", l.pos_serial, inv.pos_id));
    if !inv.pos_sw_version.is_empty() {
        p.textln(&format!("{} {}", l.software, inv.pos_sw_version));
    }

    // The customer: name, address, VAT number (AR no. 1 art. 5).
    p.separator('.');
    p.textln(il.customer);
    p.textln_wrapped(&inv.customer.name);
    p.textln_wrapped(&inv.customer.street);
    p.textln_wrapped(&format!(
        "{} {} {}",
        inv.customer.postal_code, inv.customer.city, inv.customer.country_code
    ));
    p.textln(&format!("{}: {}", l.vat_no, inv.customer.vat_number));

    // References of the original tickets.
    p.separator('.');
    p.textln(il.originals);
    for s in &inv.sales {
        let (date, time) = date_and_time(&s.pos_date_time);
        p.textln(&format!(
            "{}-{}{}/{}",
            s.fdm_id, s.event_label, s.event_counter, s.total_counter
        ));
        p.text_columns(
            &format!("  {} {:06}  {date} {time}", il.ticket, s.ticket_number),
            &price(s.total_cents),
        );
    }

    // Product lines of those tickets (payment lines are not copied).
    p.separator('.');
    for line in &inv.lines {
        let left = format!("{}  {}", quantity(line.quantity_milli), line.description);
        let right = format!("{}  {}", price(line.total_cents), line.vat_label);
        let width = p.chars_per_line() as usize;
        let room = width.saturating_sub(right.chars().count() + 1);
        let mut rows = wrap(&left, width);
        let last = rows.pop().unwrap_or_default();
        for r in &rows {
            p.textln(r);
        }
        if last.chars().count() <= room {
            p.text_columns(&last, &right);
        } else {
            p.textln(&last);
            p.text_columns("", &right);
        }
    }

    // Total, and the mention that it is already paid.
    p.style(TextStyle {
        bold: true,
        double_height: true,
        ..Default::default()
    });
    p.text_columns(l.total_row, &format!("EUR {}", price(inv.total_cents)));
    p.style_reset();
    p.textln_wrapped(il.already_paid);
    p.separator('.');

    // VAT per rate, as the FDM computed it for the tickets.
    if !inv.vat.is_empty() {
        let [rate_h, code_h, base_h, vat_h, gross_h] = l.vat_columns;
        p.textln(&vat_row(rate_h, code_h, base_h, vat_h, gross_h));
        for v in &inv.vat {
            p.textln(&vat_row(
                &rate(v.rate_hundredths),
                &v.label,
                &price(v.base_cents),
                &price(v.vat_cents),
                &price(v.total_cents),
            ));
        }
    }

    // FDM control data of the I event: no signature, no QR (AM art. 43).
    if let Some(c) = &inv.control {
        section_title(&mut p, l.fdm_info);
        p.align(Alignment::Left);
        p.style(TextStyle {
            font_b: true,
            ..Default::default()
        });
        control_data(&mut p, c);
        p.style_reset();
    }

    // A copy: its own control data and the time it was made.
    if let Some(copy) = &inv.copy {
        p.textln(l.copy_fdm);
        p.style(TextStyle {
            font_b: true,
            ..Default::default()
        });
        control_data(&mut p, &copy.control);
        p.style_reset();
        let (date, time) = date_and_time(&copy.made_at);
        p.textln(&format!("{} {date} {time}", l.copy_made_at));
    }

    // Seller legal identity.
    p.blank_line();
    p.textln_wrapped(&format!("{} {}", l.legal_name, inv.seller_legal_name));
    p.textln_wrapped(&inv.seller_address);
    p.textln(&format!("{}: {}", l.vat_no, inv.seller_vat));

    // Not a valid VAT ticket.
    p.blank_line();
    p.align(Alignment::Center);
    p.style(TextStyle {
        bold: true,
        ..Default::default()
    });
    p.textln_wrapped(invalid_disclaimer(inv.lang));
    p.style_reset();
    p.feed(4);
    p.cut();
    p.build()
}

#[cfg(test)]
mod tests {
    #[test]
    fn dates_with_non_ascii_text_do_not_panic() {
        let (d, t) = super::date_and_time("2026-10-17T11:35:00+02:00");
        assert_eq!((d.as_str(), t.as_str()), ("17.10.2026", "11:35"));
        // `-` at byte 4 and 10 bytes long, but a multi-byte char inside.
        let odd = "2026-1é-17T11:35";
        assert_eq!(super::date_and_time(odd), (odd.to_string(), String::new()));
    }

    use super::*;
    use crate::render::{one_line, render_plain_text};

    fn control(label: &str, counter: i64) -> InvoiceControl {
        InvoiceControl {
            fdm_id: "MCK00000001".into(),
            fdm_date_time: "2026-10-17T11:00:00+02:00".into(),
            event_label: label.into(),
            event_counter: counter,
            total_counter: counter + 10,
            pos_fiscal_ticket_no: 5,
            pos_date_time: "2026-10-17T11:00:00+02:00".into(),
        }
    }

    pub(crate) fn sample() -> InvoicePrint {
        InvoicePrint {
            lang: ReceiptLanguage::Fr,
            seller_name: "Restaurant Exemple".into(),
            seller_legal_name: "Exemple SRL".into(),
            seller_address: "Rue de l'Exemple 1, 4000 Liege".into(),
            seller_vat: "BE0499999960".into(),
            seller_est_no: "8789456149".into(),
            customer: InvoiceParty {
                name: "Acme SA".into(),
                street: "Rue Acme 42".into(),
                postal_code: "4000".into(),
                city: "Liege".into(),
                country_code: "BE".into(),
                vat_number: "BE0308357159".into(),
            },
            invoice_no: "20260001".into(),
            issue_date: "2026-10-17".into(),
            sales: vec![
                InvoiceSaleRef {
                    ticket_number: 42,
                    pos_date_time: "2026-10-16T19:30:00+02:00".into(),
                    fdm_id: "MCK00000001".into(),
                    event_label: "N".into(),
                    event_counter: 7,
                    total_counter: 19,
                    total_cents: 2420,
                },
                InvoiceSaleRef {
                    ticket_number: 43,
                    pos_date_time: "2026-10-16T20:10:00+02:00".into(),
                    fdm_id: "MCK00000001".into(),
                    event_label: "N".into(),
                    event_counter: 8,
                    total_counter: 20,
                    total_cents: 1120,
                },
            ],
            lines: vec![
                InvoiceLinePrint {
                    quantity_milli: 2000,
                    description: "Biere Asahi".into(),
                    total_cents: 2420,
                    vat_label: "A".into(),
                },
                InvoiceLinePrint {
                    quantity_milli: 1000,
                    description: "Saumon roll x8".into(),
                    total_cents: 1120,
                    vat_label: "B".into(),
                },
            ],
            vat: vec![
                InvoiceVatRow {
                    label: "A".into(),
                    rate_hundredths: 2100,
                    base_cents: 2000,
                    vat_cents: 420,
                    total_cents: 2420,
                },
                InvoiceVatRow {
                    label: "B".into(),
                    rate_hundredths: 1200,
                    base_cents: 1000,
                    vat_cents: 120,
                    total_cents: 1120,
                },
            ],
            total_cents: 3540,
            operator: "Marie Dupont".into(),
            pos_id: "CFOD0061234567".into(),
            pos_sw_version: "pili-pos/1.0.0".into(),
            terminal_id: "POS1".into(),
            control: Some(control("I", 3)),
            copy: None,
        }
    }

    #[test]
    fn the_invoice_says_facture_and_carries_every_mention_of_art_32() {
        let text = one_line(&render_plain_text(&format_invoice(&sample())));
        for needle in [
            "FACTURE",
            "Facture nr. 20260001",
            "17.10.2026",
            "Acme SA",
            "Rue Acme 42",
            "BE0308357159",
            "Restaurant Exemple",
            "BE0499999960",
            "MCK00000001-N7/19",
            "MCK00000001-N8/20",
            "000042",
            "Biere Asahi",
            "Saumon roll x8",
            "EUR 35,40",
            "Deja paye",
            "21,00%",
            "fdmId: MCK00000001;",
            "Event: I; eventCounter: 3;",
            "CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE",
        ] {
            assert!(text.contains(needle), "{needle}\n{text}");
        }
        // No payment line is copied from the tickets.
        assert!(!text.contains("Mode de paiement"));
    }

    #[test]
    fn the_invoice_has_no_signature_and_no_qr_code() {
        let bytes = format_invoice(&sample());
        let text = render_plain_text(&bytes);
        assert!(!text.to_lowercase().contains("signature"), "{text}");
        // GS ( k is the QR command family of ESC/POS.
        assert!(!bytes.windows(3).any(|w| w == [0x1d, 0x28, 0x6b]));
    }

    #[test]
    fn a_copy_prints_the_same_layout_under_copie_with_its_own_control_data() {
        let original = render_plain_text(&format_invoice(&sample()));
        let mut inv = sample();
        inv.copy = Some(InvoiceCopy {
            made_at: "2026-10-18T09:05:00+02:00".into(),
            control: control("C", 2),
        });
        let text = render_plain_text(&format_invoice(&inv));
        assert!(text.contains("COPIE"));
        assert!(text.contains("Copie etablie le 18.10.2026 09:05"));
        assert!(text.contains("Event: C; eventCounter: 2;"));
        // The C counters sit under the copy's own label, never under the
        // "original" one, and the I counters stay under the main block.
        let copy_at = text.find("Informations FDM de la copie :").unwrap();
        let c_at = text.find("Event: C; eventCounter: 2;").unwrap();
        let i_at = text.find("Event: I; eventCounter: 3;").unwrap();
        assert!(i_at < copy_at && copy_at < c_at);
        assert!(!text.contains("Informations FDM d'origine"));
        // Everything the original shows is still there.
        for line in [
            "Facture nr. 20260001",
            "Acme SA",
            "MCK00000001-N7/19",
            "Event: I; eventCounter: 3;",
        ] {
            assert!(original.contains(line) && text.contains(line), "{line}");
        }
        assert!(!original.contains("COPIE"));
    }

    #[test]
    fn the_title_and_mentions_follow_the_ticket_language() {
        for (lang, title) in [
            (ReceiptLanguage::Fr, "FACTURE"),
            (ReceiptLanguage::Nl, "FACTUUR"),
            (ReceiptLanguage::De, "RECHNUNG"),
        ] {
            let mut inv = sample();
            inv.lang = lang;
            let text = one_line(&render_plain_text(&format_invoice(&inv)));
            assert!(text.contains(title), "{lang:?}");
            assert!(text.contains(invalid_disclaimer(lang)), "{lang:?}");
        }
    }

    #[test]
    fn a_refund_line_prints_negative_and_a_long_name_wraps() {
        let mut inv = sample();
        inv.lines.push(InvoiceLinePrint {
            quantity_milli: -1000,
            description: "Un tres long nom de produit qui doit passer a la ligne sans etre coupe"
                .into(),
            total_cents: -1120,
            vat_label: "B".into(),
        });
        let text = render_plain_text(&format_invoice(&inv));
        assert!(text.contains("-11,20"), "{text}");
        assert!(text.contains("sans etre coupe"), "{text}");
    }

    #[test]
    fn helpers_format_money_quantities_and_dates() {
        assert_eq!(price(5), "0,05");
        assert_eq!(price(-1120), "-11,20");
        assert_eq!(quantity(2000), "2");
        assert_eq!(quantity(350), "0,350");
        assert_eq!(quantity(-1500), "-1,500");
        assert_eq!(rate(600), "6,00%");
        assert_eq!(date_only("2026-10-17"), "17.10.2026");
        assert_eq!(date_and_time("x"), ("x".to_string(), String::new()));
    }
}
