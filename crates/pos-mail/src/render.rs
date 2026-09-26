//! Build the digital ticket email from a signed [`FiscalTicket`].
//!
//! The email body is derived from exactly two persisted primitives -
//! `ticket_text` and `verification_url` - plus a `restaurant_name`
//! supplied by the caller at render time. That's deliberate: those are
//! the only two ticket-specific fields the outbox stores in
//! `ticket_deliveries` (see pos-db migration v5), so the *same*
//! [`build_email`] function both renders the email the first time
//! (checkout path) and rebuilds it identically when the outbox retries
//! a queued delivery from its row - there is no separate "resend"
//! logic to keep in sync.

use pos_fiscal::qr::{self, QrError};
use pos_fiscal::ticket::FiscalTicket;

/// A file attached to the email. TEM's request shape wants base64
/// content - encoding happens at the HTTP layer (`tem.rs`), not here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub name: String,
    pub content_type: String,
    pub content: Vec<u8>,
}

/// A fully-rendered outgoing email, ready to hand to
/// [`crate::tem::TemClient::send`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email {
    pub to: String,
    pub subject: String,
    pub text: String,
    pub html: String,
    pub attachments: Vec<Attachment>,
}

/// Convenience bundle returned by [`build_email_for_ticket`]: the
/// email plus the two fields the caller must persist into
/// `ticket_deliveries` (`ticket_text`, `verification_url`) so a later
/// retry can call [`build_email`] again without the original
/// [`FiscalTicket`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedTicket {
    pub email: Email,
    pub ticket_text: String,
    /// Only N events carry one (POS spec v2.2 §2.4).
    pub verification_url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("QR code generation failed: {0}")]
    Qr(#[from] QrError),
}

/// The digital ticket's plain-text body - byte-for-byte the same
/// content that prints on paper (POS spec §6.5 requires the digital
/// ticket carry the same mandated content, and training tickets must
/// keep their TRAINING markings either way - both come straight from
/// `format_receipt`).
pub fn render_ticket_text(ticket: &FiscalTicket) -> String {
    pos_print::render::render_plain_text(&pos_print::receipt::format_receipt(ticket))
}

/// QR PNG module size used for the email attachment (a full-size
/// image, larger than the printer's raster module - legibility on a
/// phone screen at whatever zoom the customer views the email at).
const EMAIL_QR_MODULE_SIZE: u32 = 8;

/// Build the email from already-extracted primitives. Used both for a
/// freshly-signed ticket (via [`build_email_for_ticket`]) and by the
/// outbox rebuilding the email to retry a queued delivery from its
/// persisted `ticket_deliveries` row.
///
/// The FDM returns a verification URL for N events only (POS spec
/// v2.2 §2.4). With one, the email carries it as a link and as a QR
/// code (§6.5); without one (copies C, training T), the email is the
/// ticket text alone, like the paper ticket.
pub fn build_email(
    restaurant_name: &str,
    to: &str,
    ticket_text: &str,
    verification_url: Option<&str>,
) -> Result<Email, RenderError> {
    let subject = format!("Votre ticket - {restaurant_name}");
    let pre = format!(
        "<pre style=\"font-family: monospace, monospace; white-space: pre-wrap\">{}</pre>",
        html_escape(ticket_text)
    );
    let Some(url) = verification_url.filter(|u| !u.is_empty()) else {
        return Ok(Email {
            to: to.to_string(),
            subject,
            text: format!("{ticket_text}\n"),
            html: format!("<html><body>{pre}</body></html>"),
            attachments: vec![],
        });
    };
    let text = format!("{ticket_text}\n\n{url}\n");
    // The ticket text and the link are escaped independently: the
    // former goes in a <pre> text node, the latter both as the href
    // attribute value and as the link's own text.
    let html = format!(
        "<html><body>{pre}<p><a href=\"{url}\">{url}</a></p></body></html>",
        url = html_escape(url),
    );

    let qr_png = qr::generate_qr_png(url, EMAIL_QR_MODULE_SIZE)?;

    Ok(Email {
        to: to.to_string(),
        subject,
        text,
        html,
        attachments: vec![Attachment {
            name: "ticket-qr.png".to_string(),
            content_type: "image/png".to_string(),
            content: qr_png,
        }],
    })
}

/// Build the email straight from a signed [`FiscalTicket`] - the
/// checkout-time entry point. Returns the email alongside the two
/// fields that must be persisted into `ticket_deliveries` for later
/// retries.
pub fn build_email_for_ticket(
    ticket: &FiscalTicket,
    restaurant_name: &str,
    to: &str,
) -> Result<RenderedTicket, RenderError> {
    let verification_url = ticket.verification_url.clone();
    let ticket_text = render_ticket_text(ticket);
    let email = build_email(
        restaurant_name,
        to,
        &ticket_text,
        verification_url.as_deref(),
    )?;
    Ok(RenderedTicket {
        email,
        ticket_text,
        verification_url,
    })
}

/// Minimal HTML escaping for text embedded in the email body. No
/// external dependency needed for five substitutions.
fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pos_fiscal::response::{FdmReference, SignResult};
    use pos_fiscal::ticket::{ReceiptLanguage, TicketLine, TicketParams, TicketPayment};
    use pos_fiscal::types::{EventLabel, EventOperation};

    fn mock_sign_result(event_label: EventLabel, verification_url: Option<&str>) -> SignResult {
        SignResult {
            pos_id: "CFOD0061234567".to_string(),
            pos_fiscal_ticket_no: 1,
            pos_date_time: "2026-04-12T19:30:00+02:00".to_string(),
            terminal_id: Some("POS1".to_string()),
            device_id: "DEV001".to_string(),
            event_operation: EventOperation::Sale,
            fdm_ref: FdmReference {
                fdm_id: "FDM001".to_string(),
                fdm_date_time: "2026-04-12T19:30:01+02:00".to_string(),
                event_label,
                event_counter: 1,
                total_counter: 1,
            },
            fdm_sw_version: "1.0".to_string(),
            digital_signature: "AAAA".to_string(),
            short_signature: Some("abc123".to_string()),
            verification_url: verification_url.map(str::to_string),
            vat_calc: None,
            buffer_capacity_used: 0.01,
            warnings: None,
            informations: None,
            footer: vec!["www.minfin.fgov.be".to_string()],
        }
    }

    fn ticket_with(lines: Vec<TicketLine>, verification_url: Option<&str>) -> FiscalTicket {
        let result = mock_sign_result(EventLabel::N, verification_url);
        FiscalTicket::from_sign_result(TicketParams {
            result: &result,
            restaurant_name: "Restaurant Exemple",
            restaurant_address: "Rue Test 1",
            restaurant_vat: "BE0499999960",
            restaurant_est_no: "",
            restaurant_legal_name: "",
            lines,
            payments: vec![TicketPayment {
                method: "Cash".into(),
                amount: 10.0,
            }],
            total: 10.0,
            is_training: false,
            rounding: None,
            lang: ReceiptLanguage::default(),
            operator: "Marie",
        })
    }

    fn sample_ticket() -> FiscalTicket {
        ticket_with(
            vec![TicketLine {
                quantity: 1.0,
                description: "Saumon roll x8".into(),
                unit_price: 10.0,
                total: 10.0,
                vat_code: "B".into(),
            }],
            Some("https://www.gs2.be/1234567890abcdefghi012345678901234"),
        )
    }

    // ── build_email_for_ticket ─────────────────────────────────

    #[test]
    fn a_ticket_without_url_is_the_text_alone() {
        // Copies (C) and training tickets (T) carry no verification URL.
        let ticket = ticket_with(vec![], None);
        let rendered =
            build_email_for_ticket(&ticket, "Restaurant Exemple", "customer@example.be").unwrap();
        assert!(rendered.verification_url.is_none());
        assert!(rendered.email.attachments.is_empty(), "no QR code");
        assert!(!rendered.email.html.contains("<a "), "no link");
        assert_eq!(rendered.email.text, format!("{}\n", rendered.ticket_text));
    }

    #[test]
    fn an_empty_url_counts_as_none() {
        let email = build_email("R", "customer@example.be", "TICKET", Some("")).unwrap();
        assert!(email.attachments.is_empty());
        assert!(!email.html.contains("<a "));
    }

    #[test]
    fn builds_the_subject_from_restaurant_name() {
        let ticket = sample_ticket();
        let rendered =
            build_email_for_ticket(&ticket, "Restaurant Exemple", "customer@example.be").unwrap();
        assert_eq!(rendered.email.subject, "Votre ticket - Restaurant Exemple");
    }

    #[test]
    fn sets_the_recipient() {
        let ticket = sample_ticket();
        let rendered =
            build_email_for_ticket(&ticket, "Restaurant Exemple", "customer@example.be").unwrap();
        assert_eq!(rendered.email.to, "customer@example.be");
    }

    #[test]
    fn text_body_contains_the_receipt_and_the_link() {
        let ticket = sample_ticket();
        let rendered =
            build_email_for_ticket(&ticket, "Restaurant Exemple", "customer@example.be").unwrap();
        assert!(rendered.email.text.contains("Saumon roll x8"));
        assert!(
            rendered
                .email
                .text
                .contains(rendered.verification_url.as_deref().unwrap())
        );
        // Link must appear on its own plain-text line (POS spec §6.5).
        assert!(
            rendered
                .email
                .text
                .lines()
                .any(|line| Some(line.trim()) == rendered.verification_url.as_deref())
        );
    }

    #[test]
    fn html_body_contains_the_receipt_and_a_real_anchor() {
        let ticket = sample_ticket();
        let rendered =
            build_email_for_ticket(&ticket, "Restaurant Exemple", "customer@example.be").unwrap();
        assert!(rendered.email.html.contains("Saumon roll x8"));
        assert!(rendered.email.html.contains(&format!(
            "<a href=\"{}\">{}</a>",
            rendered.verification_url.as_deref().unwrap(),
            rendered.verification_url.as_deref().unwrap()
        )));
        assert!(rendered.email.html.contains("<pre"));
    }

    #[test]
    fn html_escapes_a_malicious_looking_product_name() {
        let ticket = ticket_with(
            vec![TicketLine {
                quantity: 1.0,
                description: "<b>&".into(),
                unit_price: 1.0,
                total: 1.0,
                vat_code: "B".into(),
            }],
            Some("https://www.gs2.be/1234567890abcdefghi012345678901234"),
        );
        let rendered =
            build_email_for_ticket(&ticket, "Restaurant Exemple", "customer@example.be").unwrap();
        // The raw tag must never appear unescaped in the HTML body...
        assert!(!rendered.email.html.contains("<b>&"));
        // ...but the escaped form must be present.
        assert!(rendered.email.html.contains("&lt;b&gt;&amp;"));
        // The plain-text body is untouched (no escaping needed there).
        assert!(rendered.email.text.contains("<b>&"));
    }

    #[test]
    fn training_ticket_keeps_the_training_markings() {
        let result = mock_sign_result(
            EventLabel::T,
            Some("https://www.gs2.be/1234567890abcdefghi012345678901234"),
        );
        let ticket = FiscalTicket::from_sign_result(TicketParams {
            result: &result,
            restaurant_name: "Restaurant Exemple",
            restaurant_address: "",
            restaurant_vat: "",
            restaurant_est_no: "",
            restaurant_legal_name: "",
            lines: vec![],
            payments: vec![],
            total: 0.0,
            is_training: true,
            rounding: None,
            lang: ReceiptLanguage::default(),
            operator: "Marie",
        });
        let rendered =
            build_email_for_ticket(&ticket, "Restaurant Exemple", "customer@example.be").unwrap();
        assert!(rendered.email.text.contains("TRAINING"));
        assert!(rendered.email.html.contains("TRAINING"));
    }

    #[test]
    fn qr_attachment_is_a_valid_png_encoding_the_verification_url() {
        let ticket = sample_ticket();
        let rendered =
            build_email_for_ticket(&ticket, "Restaurant Exemple", "customer@example.be").unwrap();
        assert_eq!(rendered.email.attachments.len(), 1);
        let attachment = &rendered.email.attachments[0];
        assert_eq!(attachment.name, "ticket-qr.png");
        assert_eq!(attachment.content_type, "image/png");
        let decoded = image::load_from_memory(&attachment.content)
            .expect("attachment must decode as a valid image");
        assert!(decoded.width() > 0 && decoded.height() > 0);
    }

    #[test]
    fn no_pdf_attachment_is_ever_produced() {
        let ticket = sample_ticket();
        let rendered =
            build_email_for_ticket(&ticket, "Restaurant Exemple", "customer@example.be").unwrap();
        assert!(
            rendered
                .email
                .attachments
                .iter()
                .all(|a| a.content_type != "application/pdf")
        );
    }

    // ── build_email rebuilds identically from stored primitives ──

    #[test]
    fn build_email_from_stored_primitives_matches_build_email_for_ticket() {
        let ticket = sample_ticket();
        let rendered =
            build_email_for_ticket(&ticket, "Restaurant Exemple", "customer@example.be").unwrap();

        // Simulates the outbox retrying from a `ticket_deliveries` row:
        // only ticket_text + verification_url survive, no FiscalTicket.
        let rebuilt = build_email(
            "Restaurant Exemple",
            "customer@example.be",
            &rendered.ticket_text,
            rendered.verification_url.as_deref(),
        )
        .unwrap();

        assert_eq!(rebuilt.subject, rendered.email.subject);
        assert_eq!(rebuilt.text, rendered.email.text);
        assert_eq!(rebuilt.html, rendered.email.html);
    }

    // ── render_ticket_text ─────────────────────────────────────

    #[test]
    fn render_ticket_text_matches_the_printed_receipt() {
        let ticket = sample_ticket();
        let text = render_ticket_text(&ticket);
        assert!(text.contains("TICKET DE CAISSE TVA"));
        assert!(text.contains("Saumon roll x8"));
    }
}
