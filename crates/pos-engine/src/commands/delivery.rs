//! Ticket delivery: paper, email, or both (cert manual §III.H).
//!
//! The medium is chosen before signing, because it is part of the
//! signed request (`ticketMedium`, POS spec v2.2 §1). The flow is:
//!
//! 1. [`resolve`] checks the operator's choice against the terminal's
//!    configuration and returns the medium to sign.
//! 2. [`enqueue_in_tx`] writes the email outbox row in the same SQLite
//!    transaction that archives the signed event, so a signed digital
//!    ticket always has a delivery row, even after a crash.
//! 3. [`deliver`] prints and/or sends once the transaction committed.
//!    A DIGITAL ticket that can't be sent within the inline budget is
//!    printed instead: the customer always leaves with a ticket.
//!
//! An original ticket is printed once (POS spec v2.2 §8.1). Every
//! original print is recorded in `original_prints`, and
//! [`print_missing_original`] only prints an original that never was.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use pos_core::config::{DigitalTicketConfig, PosConfig};
use pos_db::queries::deliveries::{self, NewTicketDelivery};
use pos_fiscal::http_tls::HttpTls;
use pos_fiscal::ticket::FiscalTicket;
use pos_fiscal::types::TicketMedium;
use pos_mail::outbox::{self, SendNowOutcome};
use pos_mail::tem::TemClient;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

/// Longest the checkout waits for an email before printing instead.
pub const INLINE_SEND_TIMEOUT: Duration = Duration::from_secs(8);
/// Per-request timeout of the TEM client.
const TEM_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// A probe result younger than this is reused by `check_mail_status`.
const PROBE_CACHE: Duration = Duration::from_secs(60);
/// A DIGITAL-only ticket needs a successful probe younger than this;
/// otherwise the POS prints as well (PAPER_DIGITAL).
const PROBE_MAX_AGE_FOR_DIGITAL: Duration = Duration::from_secs(120);
/// Interval of the background outbox worker.
pub const OUTBOX_INTERVAL: Duration = Duration::from_secs(30);

// ── Mail state ─────────────────────────────────────────────────

/// The email client and the last reachability probe.
pub struct MailState {
    client: Option<TemClient>,
    probe: Mutex<Option<(Instant, bool)>>,
}

impl MailState {
    /// No digital ticket on this terminal.
    pub fn disabled() -> Self {
        Self::new(None)
    }

    pub fn new(client: Option<TemClient>) -> Self {
        Self {
            client,
            probe: Mutex::new(None),
        }
    }

    /// Build the client from `pos.json`. A configured but unusable
    /// setup (missing key file) is logged and leaves email disabled:
    /// the POS then offers paper only.
    pub fn from_config(config: &PosConfig, http_tls: &HttpTls) -> Self {
        let Some(cfg) = config.digital_ticket.as_ref() else {
            return Self::disabled();
        };
        let http = match http_tls.client_builder() {
            Ok(builder) => builder.build().map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        };
        let http = match http {
            Ok(http) => http,
            Err(e) => {
                tracing::error!(%e, "digital ticket: cannot build the HTTP client");
                return Self::disabled();
            }
        };
        match TemClient::from_config(http, cfg, TEM_REQUEST_TIMEOUT) {
            Ok(client) => Self::new(Some(client)),
            Err(e) => {
                tracing::error!(%e, "digital ticket: configured but unusable, paper only");
                Self::disabled()
            }
        }
    }

    pub fn client(&self) -> Option<&TemClient> {
        self.client.as_ref()
    }

    /// Store a probe result (used by `probe`, and by tests).
    pub fn record_probe(&self, reachable: bool) {
        if let Ok(mut p) = self.probe.lock() {
            *p = Some((Instant::now(), reachable));
        }
    }

    fn probe_younger_than(&self, age: Duration) -> Option<bool> {
        let p = self.probe.lock().ok()?;
        p.and_then(|(at, ok)| (at.elapsed() < age).then_some(ok))
    }

    /// Whether the email service answered a probe recently.
    pub fn recently_reachable(&self) -> bool {
        self.probe_younger_than(PROBE_MAX_AGE_FOR_DIGITAL)
            .unwrap_or(false)
    }

    /// Probe the email service, reusing a result younger than 60 s.
    pub async fn probe(&self) -> bool {
        let Some(client) = self.client.as_ref() else {
            return false;
        };
        if let Some(ok) = self.probe_younger_than(PROBE_CACHE) {
            return ok;
        }
        let ok = match client.probe().await {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!(%e, "digital ticket: probe failed");
                false
            }
        };
        self.record_probe(ok);
        ok
    }
}

// ── Choosing the medium ────────────────────────────────────────

/// The medium the operator picked on screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestedMedium {
    #[default]
    Paper,
    Digital,
    PaperDigital,
}

/// The operator's choice, sent with checkout, refund and copy.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketDeliveryRequest {
    #[serde(default)]
    pub medium: RequestedMedium,
    #[serde(default)]
    pub email: Option<String>,
}

impl TicketDeliveryRequest {
    pub fn email(medium: RequestedMedium, email: &str) -> Self {
        Self {
            medium,
            email: Some(email.to_string()),
        }
    }
}

/// The medium that will be signed, and the address to send to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDelivery {
    pub medium: TicketMedium,
    pub email: Option<String>,
}

impl ResolvedDelivery {
    pub fn paper() -> Self {
        Self {
            medium: TicketMedium::Paper,
            email: None,
        }
    }

    pub fn sends_email(&self) -> bool {
        matches!(
            self.medium,
            TicketMedium::Digital | TicketMedium::PaperDigital
        )
    }

    pub fn prints_paper(&self) -> bool {
        self.medium != TicketMedium::Digital
    }
}

/// Refusals returned before anything is signed.
pub const ERR_DIGITAL_UNAVAILABLE: &str = errors::DIGITAL_UNAVAILABLE;
pub const ERR_INVALID_EMAIL: &str = errors::EMAIL_INVALID;

/// Check the operator's choice. Pure, so every rule is unit-tested:
///
/// - no request, or paper: PAPER;
/// - email requested on a terminal without a working email setup:
///   refused;
/// - a missing or invalid address: refused;
/// - DIGITAL only when the configuration allows it and the service
///   answered recently; otherwise PAPER_DIGITAL, so the customer gets
///   paper at once and the email when the service is back.
pub fn resolve_with(
    request: Option<&TicketDeliveryRequest>,
    digital: Option<&DigitalTicketConfig>,
    client_ready: bool,
    recently_reachable: bool,
) -> Result<ResolvedDelivery, String> {
    let Some(request) = request else {
        return Ok(ResolvedDelivery::paper());
    };
    if request.medium == RequestedMedium::Paper {
        return Ok(ResolvedDelivery::paper());
    }
    let Some(digital) = digital.filter(|_| client_ready) else {
        return Err(ERR_DIGITAL_UNAVAILABLE.to_string());
    };
    let email = request
        .email
        .as_deref()
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .ok_or_else(|| ERR_INVALID_EMAIL.to_string())?;
    pos_mail::address::validate(email)
        .map_err(|e| errors::coded(ERR_INVALID_EMAIL, &[("detail", &e)]))?;

    let medium = match request.medium {
        RequestedMedium::Digital if digital.allow_digital_only && recently_reachable => {
            TicketMedium::Digital
        }
        _ => TicketMedium::PaperDigital,
    };
    Ok(ResolvedDelivery {
        medium,
        email: Some(email.to_string()),
    })
}

/// [`resolve_with`] for the terminal's current configuration.
pub fn resolve(
    request: Option<&TicketDeliveryRequest>,
    config: &PosConfig,
    mail: &MailState,
) -> Result<ResolvedDelivery, String> {
    resolve_with(
        request,
        config.digital_ticket.as_ref(),
        mail.client().is_some(),
        mail.recently_reachable(),
    )
}

// ── Outbox row ─────────────────────────────────────────────────

/// Queue the email for a signed event. Call inside the transaction
/// that archives the event. Returns `None` when no email is due, or
/// when there is no ticket to send (an event without a ticket body is
/// only ever printed). Only N events carry a verification URL (POS
/// spec v2.2 §2.4); copies and training tickets are sent without one.
pub fn enqueue_in_tx(
    conn: &Connection,
    fiscal_event_id: i64,
    resolved: &ResolvedDelivery,
    ticket: Option<&FiscalTicket>,
) -> Result<Option<i64>, String> {
    if !resolved.sends_email() {
        return Ok(None);
    }
    let (Some(email), Some(ticket)) = (resolved.email.as_deref(), ticket) else {
        return Ok(None);
    };
    let medium = match resolved.medium {
        TicketMedium::Digital => "DIGITAL",
        _ => "PAPER_DIGITAL",
    };
    let ticket_text = pos_mail::render::render_ticket_text(ticket);
    let hint = pos_mail::address::mask(email);
    deliveries::enqueue(
        conn,
        NewTicketDelivery {
            fiscal_event_id,
            medium,
            recipient: email,
            recipient_hint: &hint,
            ticket_text: &ticket_text,
            verification_url: ticket.verification_url.as_deref(),
        },
    )
    .map(Some)
    .map_err(|e| e.to_string())
}

// ── Printing and sending ───────────────────────────────────────

/// Email outcome shown after checkout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailStatus {
    NotRequested,
    Sent,
    /// Not sent yet; the background worker keeps trying.
    Queued,
}

/// What happened to the ticket, for the "done" screen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryOutcome {
    pub fiscal_event_id: i64,
    pub delivery_id: Option<i64>,
    pub medium: TicketMedium,
    pub email: EmailStatus,
    pub recipient_hint: Option<String>,
    pub paper_printed: bool,
    /// The customer should have a paper ticket but none printed: the
    /// screen offers to print it (once).
    pub needs_paper: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrintOutcome {
    Printed,
    NoPrinter,
    AlreadyPrinted,
    Failed(String),
}

/// Print the original of `fiscal_event_id` unless it was printed
/// already, and record the print.
pub async fn print_original_once(
    state: &AppState,
    printer_address: Option<&str>,
    fiscal_event_id: i64,
    ticket: &FiscalTicket,
) -> PrintOutcome {
    let already = {
        let Ok(db) = state.db.lock() else {
            return PrintOutcome::Failed("database lock poisoned".into());
        };
        pos_db::queries::prints::original_printed(&db, fiscal_event_id)
    };
    match already {
        Ok(true) => return PrintOutcome::AlreadyPrinted,
        Ok(false) => {}
        Err(e) => return PrintOutcome::Failed(e.to_string()),
    }
    let Some(addr) = printer_address else {
        return PrintOutcome::NoPrinter;
    };
    let data = pos_print::receipt::format_receipt(ticket);
    match pos_print::transport::send_to_printer(addr, &data).await {
        Ok(status) => {
            if !status.paper_ok {
                tracing::warn!(?status.warnings, "printer: paper issue after receipt");
            }
            if !status.cover_closed {
                tracing::warn!("printer: cover open after receipt");
            }
            if let Ok(db) = state.db.lock()
                && let Err(e) = pos_db::queries::prints::record_original_print(&db, fiscal_event_id)
            {
                tracing::error!(%e, fiscal_event_id, "could not record the original print");
            }
            PrintOutcome::Printed
        }
        Err(e) => {
            tracing::warn!(%e, "receipt print failed");
            PrintOutcome::Failed(e.to_string())
        }
    }
}

/// Print and/or email the ticket of a committed event.
pub async fn deliver(
    state: &AppState,
    config: &PosConfig,
    fiscal_event_id: i64,
    resolved: &ResolvedDelivery,
    delivery_id: Option<i64>,
    ticket: Option<&FiscalTicket>,
) -> DeliveryOutcome {
    let mut email = if delivery_id.is_some() {
        EmailStatus::Queued
    } else {
        EmailStatus::NotRequested
    };
    let recipient_hint = resolved.email.as_deref().map(pos_mail::address::mask);

    // DIGITAL: try the email first; paper only if it doesn't go out.
    if resolved.medium == TicketMedium::Digital
        && let (Some(id), Some(client)) = (delivery_id, state.mail.client())
        && let SendNowOutcome::Sent { .. } = outbox::send_now(
            &state.db,
            client,
            &config.restaurant_name,
            id,
            INLINE_SEND_TIMEOUT,
        )
        .await
    {
        return DeliveryOutcome {
            fiscal_event_id,
            delivery_id,
            medium: resolved.medium,
            email: EmailStatus::Sent,
            recipient_hint,
            paper_printed: false,
            needs_paper: false,
        };
    }

    // Paper: always for PAPER and PAPER_DIGITAL, as the fallback for
    // an unsent DIGITAL ticket.
    let print = match ticket {
        Some(t) => {
            print_original_once(state, config.printer_address.as_deref(), fiscal_event_id, t).await
        }
        None => PrintOutcome::NoPrinter,
    };
    let paper_printed = print == PrintOutcome::Printed;
    if resolved.medium == TicketMedium::Digital
        && paper_printed
        && let Some(id) = delivery_id
        && let Ok(db) = state.db.lock()
        && let Err(e) = deliveries::mark_fallback_printed(&db, id)
    {
        tracing::warn!(%e, delivery_id = id, "could not mark the paper fallback");
    }
    if resolved.medium == TicketMedium::Digital && paper_printed {
        // The paper original replaces the email.
        email = EmailStatus::NotRequested;
    }

    // PAPER_DIGITAL: the email is a bonus, sent now if possible.
    if resolved.medium == TicketMedium::PaperDigital
        && let (Some(id), Some(client)) = (delivery_id, state.mail.client())
        && let SendNowOutcome::Sent { .. } = outbox::send_now(
            &state.db,
            client,
            &config.restaurant_name,
            id,
            INLINE_SEND_TIMEOUT,
        )
        .await
    {
        email = EmailStatus::Sent;
    }

    DeliveryOutcome {
        fiscal_event_id,
        delivery_id,
        medium: resolved.medium,
        email,
        recipient_hint,
        paper_printed,
        needs_paper: ticket.is_some() && !paper_printed && print != PrintOutcome::AlreadyPrinted,
    }
}

// ── Commands ───────────────────────────────────────────────────

/// Email availability for the ticket picker.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailStatus {
    pub configured: bool,
    pub reachable: bool,
    pub allow_digital_only: bool,
}

pub async fn do_check_mail_status(state: &AppState) -> Result<MailStatus, String> {
    let allow_digital_only = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .digital_ticket
        .as_ref()
        .is_some_and(|d| d.allow_digital_only);
    let configured = state.mail.client().is_some();
    let reachable = configured && state.mail.probe().await;
    Ok(MailStatus {
        configured,
        reachable,
        allow_digital_only,
    })
}

/// A delivery as shown in Settings. Never carries the address itself.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryView {
    pub id: i64,
    pub fiscal_event_id: i64,
    pub medium: String,
    pub recipient_hint: String,
    pub status: String,
    pub attempts: i64,
    pub last_error: Option<String>,
    pub fallback_printed: bool,
    pub created_at: String,
}

pub fn do_list_ticket_deliveries(
    state: &AppState,
    limit: Option<i64>,
) -> Result<Vec<DeliveryView>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let rows = deliveries::list_recent(&db, limit.unwrap_or(50).clamp(1, 500))
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|d| DeliveryView {
            id: d.id,
            fiscal_event_id: d.fiscal_event_id,
            medium: d.medium,
            recipient_hint: d.recipient_hint,
            status: d.status,
            attempts: d.attempts,
            last_error: d.last_error,
            fallback_printed: d.fallback_printed,
            created_at: d.created_at,
        })
        .collect())
}

pub const ERR_ALREADY_PRINTED: &str = errors::ALREADY_PRINTED;
pub const ERR_EMAIL_SENT: &str = errors::EMAIL_SENT;

/// Print an original that was never printed: the printer failed at
/// checkout, or a DIGITAL ticket's email didn't go out. The ticket is
/// rebuilt from the archive (a sale or a copy only). Refused once the
/// original was printed (a second print must be a copy, event C) and
/// for a DIGITAL ticket whose email was sent.
pub async fn do_print_missing_original(
    state: &AppState,
    fiscal_event_id: i64,
) -> Result<(), String> {
    access::operator(state, Access::Cashier)?;
    let ticket = &super::copy::printable_original(state, fiscal_event_id)?;
    let (printer, digital_delivery) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        pos_db::queries::fiscal::get_fiscal_event(&db, fiscal_event_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| errors::EVENT_NOT_FOUND.to_string())?;
        let delivery: Option<(i64, String)> = db
            .query_row(
                "SELECT id, status FROM ticket_deliveries
                 WHERE fiscal_event_id = ?1 AND medium = 'DIGITAL'",
                rusqlite::params![fiscal_event_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();
        let printer = state
            .config
            .lock()
            .map_err(|e| e.to_string())?
            .printer_address
            .clone();
        (printer, delivery)
    };
    if let Some((_, status)) = &digital_delivery
        && status == "sent"
    {
        return Err(ERR_EMAIL_SENT.to_string());
    }
    match print_original_once(state, printer.as_deref(), fiscal_event_id, ticket).await {
        PrintOutcome::Printed => {
            if let Some((id, _)) = digital_delivery
                && let Ok(db) = state.db.lock()
                && let Err(e) = deliveries::mark_fallback_printed(&db, id)
            {
                tracing::warn!(%e, delivery_id = id, "could not mark the paper fallback");
            }
            Ok(())
        }
        PrintOutcome::AlreadyPrinted => Err(ERR_ALREADY_PRINTED.to_string()),
        PrintOutcome::NoPrinter => Err(errors::PRINTER_NOT_CONFIGURED.to_string()),
        PrintOutcome::Failed(e) => Err(errors::coded(errors::PRINT_FAILED, &[("detail", &e)])),
    }
}

/// One pass of the background worker: retries queued emails and
/// clears addresses older than 72 h.
pub async fn run_outbox_once(state: &AppState) {
    let Some(client) = state.mail.client() else {
        return;
    };
    let restaurant_name = match state.config.lock() {
        Ok(c) => c.restaurant_name.clone(),
        Err(_) => return,
    };
    match outbox::run_once(&state.db, client, &restaurant_name).await {
        Ok(report) if report != outbox::RunReport::default() => {
            tracing::info!(?report, "digital ticket outbox");
        }
        Ok(_) => {}
        Err(e) => tracing::warn!(%e, "digital ticket outbox pass failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digital(allow_digital_only: bool) -> DigitalTicketConfig {
        DigitalTicketConfig {
            region: "fr-par".into(),
            project_id: "p".into(),
            from_email: "tickets@example.be".into(),
            from_name: "Restaurant".into(),
            api_base: None,
            api_key_file: "/dev/null".into(),
            allow_digital_only,
        }
    }

    fn req(medium: RequestedMedium, email: Option<&str>) -> TicketDeliveryRequest {
        TicketDeliveryRequest {
            medium,
            email: email.map(str::to_string),
        }
    }

    #[test]
    fn no_request_is_paper() {
        let r = resolve_with(None, Some(&digital(true)), true, true).unwrap();
        assert_eq!(r, ResolvedDelivery::paper());
        assert!(r.prints_paper());
        assert!(!r.sends_email());
    }

    #[test]
    fn paper_request_ignores_the_email_field() {
        let r = resolve_with(
            Some(&req(RequestedMedium::Paper, Some("a@b.be"))),
            None,
            false,
            false,
        )
        .unwrap();
        assert_eq!(r, ResolvedDelivery::paper());
    }

    #[test]
    fn email_is_refused_without_configuration() {
        for medium in [RequestedMedium::Digital, RequestedMedium::PaperDigital] {
            let err =
                resolve_with(Some(&req(medium, Some("a@b.be"))), None, false, false).unwrap_err();
            assert_eq!(err, ERR_DIGITAL_UNAVAILABLE);
        }
    }

    #[test]
    fn email_is_refused_when_the_client_is_unusable() {
        let err = resolve_with(
            Some(&req(RequestedMedium::PaperDigital, Some("a@b.be"))),
            Some(&digital(true)),
            false,
            true,
        )
        .unwrap_err();
        assert_eq!(err, ERR_DIGITAL_UNAVAILABLE);
    }

    #[test]
    fn a_missing_or_blank_address_is_refused() {
        for email in [None, Some(""), Some("   ")] {
            let err = resolve_with(
                Some(&req(RequestedMedium::PaperDigital, email)),
                Some(&digital(true)),
                true,
                true,
            )
            .unwrap_err();
            assert!(err.starts_with(ERR_INVALID_EMAIL), "{err}");
        }
    }

    #[test]
    fn a_malformed_address_is_refused() {
        for email in ["nope", "a@", "@b.be", "a b@c.be"] {
            let err = resolve_with(
                Some(&req(RequestedMedium::PaperDigital, Some(email))),
                Some(&digital(true)),
                true,
                true,
            )
            .unwrap_err();
            assert!(err.starts_with(ERR_INVALID_EMAIL), "{email}: {err}");
        }
    }

    #[test]
    fn the_address_is_trimmed() {
        let r = resolve_with(
            Some(&req(RequestedMedium::PaperDigital, Some("  a@b.be "))),
            Some(&digital(false)),
            true,
            true,
        )
        .unwrap();
        assert_eq!(r.email.as_deref(), Some("a@b.be"));
    }

    #[test]
    fn digital_only_when_allowed_and_reachable() {
        let r = resolve_with(
            Some(&req(RequestedMedium::Digital, Some("a@b.be"))),
            Some(&digital(true)),
            true,
            true,
        )
        .unwrap();
        assert_eq!(r.medium, TicketMedium::Digital);
        assert!(!r.prints_paper());
        assert!(r.sends_email());
    }

    #[test]
    fn digital_becomes_paper_digital_when_not_allowed() {
        let r = resolve_with(
            Some(&req(RequestedMedium::Digital, Some("a@b.be"))),
            Some(&digital(false)),
            true,
            true,
        )
        .unwrap();
        assert_eq!(r.medium, TicketMedium::PaperDigital);
        assert!(r.prints_paper());
    }

    #[test]
    fn digital_becomes_paper_digital_when_offline() {
        let r = resolve_with(
            Some(&req(RequestedMedium::Digital, Some("a@b.be"))),
            Some(&digital(true)),
            true,
            false,
        )
        .unwrap();
        assert_eq!(r.medium, TicketMedium::PaperDigital);
    }

    #[test]
    fn paper_digital_is_kept_even_offline() {
        let r = resolve_with(
            Some(&req(RequestedMedium::PaperDigital, Some("a@b.be"))),
            Some(&digital(true)),
            true,
            false,
        )
        .unwrap();
        assert_eq!(r.medium, TicketMedium::PaperDigital);
        assert!(r.sends_email());
    }

    #[test]
    fn request_deserializes_from_the_ui_shape() {
        let r: TicketDeliveryRequest =
            serde_json::from_str(r#"{"medium":"paper_digital","email":"a@b.be"}"#).unwrap();
        assert_eq!(r.medium, RequestedMedium::PaperDigital);
        let r: TicketDeliveryRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(r.medium, RequestedMedium::Paper);
        assert!(r.email.is_none());
        assert!(serde_json::from_str::<TicketDeliveryRequest>(r#"{"medium":"fax"}"#).is_err());
    }

    #[test]
    fn a_disabled_mail_state_is_never_reachable() {
        let m = MailState::disabled();
        assert!(m.client().is_none());
        assert!(!m.recently_reachable());
    }

    #[test]
    fn a_recorded_probe_is_remembered() {
        let m = MailState::disabled();
        m.record_probe(true);
        assert!(m.recently_reachable());
        m.record_probe(false);
        assert!(!m.recently_reachable());
    }

    #[test]
    fn a_config_without_digital_ticket_disables_mail() {
        let m = MailState::from_config(&PosConfig::default(), &HttpTls::Platform);
        assert!(m.client().is_none());
    }

    #[test]
    fn a_missing_key_file_disables_mail() {
        let mut d = digital(true);
        d.api_key_file = "/nonexistent/pili-pos/tem.key".into();
        let config = PosConfig {
            kitchen_printer_address: None,
            fdm_token_file: None,
            fdm_ca_file: None,
            fdm_client_cert_file: None,
            fdm_client_key_file: None,
            restaurant_legal_name: None,
            day_end_hour: 5,
            digital_ticket: Some(d),
            ..PosConfig::default()
        };
        assert!(
            MailState::from_config(&config, &HttpTls::Platform)
                .client()
                .is_none()
        );
    }
}
