//! Digital ticket delivery: renders a signed [`pos_fiscal::ticket::FiscalTicket`]
//! into an email, sends it via the Scaleway Transactional Email HTTP
//! API, and manages retries/GDPR retention through the
//! `ticket_deliveries` outbox table (`pos-db` migration v5).
//!
//! Certification context: manual §III.H requires a digital ticket
//! option alongside paper; this crate is the "digital" half, with
//! paper always available as a fallback (POS owner requirement: no
//! ticket is ever silently lost).
//!
//! Library only - no Tauri commands, no UI. `src-tauri` wires
//! [`config::DigitalTicketConfig`], [`tem::TemClient`] and
//! [`outbox::send_now`]/[`outbox::run_once`] into the checkout flow
//! and a background scheduler.

pub mod address;
pub mod config;
pub mod errors;
pub mod outbox;
pub mod render;
pub mod tem;

pub use config::DigitalTicketConfig;
pub use errors::MailError;
