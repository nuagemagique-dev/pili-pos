//! The card payment terminal linked to the till (EFT).
//!
//! The engine asks [`Terminal::pay`] for an amount and records the
//! [`Outcome`]; an approved payment becomes a payment line with
//! `inputMethod: AUTOMATIC`, the card type and brand the terminal
//! returned (POS spec v2.2, `paymentLine.inputMethod`: "le terminal de
//! paiement renvoie lui-même le type de carte utilisé").
//!
//! Every payment carries the till's own [`PaymentRequest::reference`]
//! (a UUID the engine stores before calling [`Terminal::pay`]): it is
//! the idempotency key sent to the terminal and the key
//! [`Terminal::status`] uses to read the final outcome of a payment
//! whose answer was lost (crash, timeout, cut connection).
//!
//! Drivers, one per [`pos_core::config::PaymentTerminalKind`]:
//! - [`worldline`]: Worldline Terminal API (nexo Retailer 5.1 JSON).
//! - [`ccv_cloud`]: CCV Payment API, method `terminal`.
//! - [`ccv_opi`]: OPI XML over the shop's LAN (CCV terminals).

pub mod brand;
pub mod ccv_cloud;
pub mod ccv_opi;
pub mod worldline;

use std::time::Duration;

use pos_core::config::{PaymentTerminalConfig, PaymentTerminalKind};
use serde::{Deserialize, Serialize};

/// One payment asked of the terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentRequest {
    /// The till's reference for this payment (UUID v4, lowercase),
    /// stored by the engine before the call. Idempotency key, status
    /// key, merchant reference on the terminal side.
    pub reference: String,
    /// Amount to charge, in euro cents, > 0.
    pub amount_cents: i64,
    /// Short text the terminal or the PSP may show or keep
    /// ("Tokyo Sushi Bar, table 4").
    pub description: String,
    /// Language of the terminal screens for this payment: `fr`, `nl`,
    /// `de` or `en` (the till's language restricted to these).
    pub language: String,
}

/// Debit or credit, as far as the terminal tells. Maps to the SCE
/// payment types `CARD_DEBIT`, `CARD_CREDIT`, `CARD_UNKNOWN`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardType {
    Debit,
    Credit,
    Unknown,
}

/// An approved payment: what the terminal says about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    /// Approved amount in cents. Equal to the request: drivers never
    /// ask for partial approval, and an approval for another amount is
    /// reported as [`Outcome::Unknown`] for a person to sort out.
    pub amount_cents: i64,
    pub card_type: CardType,
    /// Card brand, normalized by [`brand::normalize`] ("Bancontact",
    /// "Maestro", "Visa", "Mastercard", ...). `None` when the terminal
    /// does not say.
    pub brand: Option<String>,
    /// The terminal's or the payment provider's id of the transaction
    /// (what a refund or a support call refers to).
    pub terminal_reference: Option<String>,
    pub authorisation_code: Option<String>,
    /// Masked card number as the terminal gives it ("XXXX XXXX XXXX 1234").
    pub masked_pan: Option<String>,
    /// The cardholder receipt text, when the terminal hands it to the
    /// till instead of printing it itself.
    pub customer_receipt: Option<String>,
}

/// The final (or not yet final) answer to one payment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    /// The money is charged.
    Approved(Approval),
    /// Refused by the card, the bank or the terminal, or cancelled by
    /// the cardholder on the terminal. Nothing charged. `reason` is the
    /// terminal's or provider's code or text, for the log and the
    /// operator.
    Declined { reason: String },
    /// Stopped by the till ([`Terminal::abort`]) before an approval.
    /// Nothing charged.
    Cancelled,
    /// No final answer yet (timeout, cut connection, provider still
    /// pending, provider asking for manual intervention): the money may
    /// or may not be charged. Ask [`Terminal::status`] again.
    Unknown { detail: String },
}

/// Why a call to the terminal did not produce an [`Outcome`].
#[derive(Debug, thiserror::Error)]
pub enum EftError {
    /// The configuration cannot drive this terminal (missing id, key
    /// file unreadable). Nothing was sent.
    #[error("payment terminal configuration: {0}")]
    Config(String),
    /// The terminal or the provider could not be reached and the
    /// request surely did not leave (connection refused, DNS, TLS
    /// handshake before sending). Nothing was charged.
    #[error("payment terminal unreachable: {0}")]
    Unreachable(String),
    /// The terminal is busy with another payment.
    #[error("payment terminal busy")]
    Busy,
    /// The provider refused the request itself (bad credentials,
    /// invalid field, unknown terminal). Nothing was charged.
    #[error("payment terminal refused the request: {0}")]
    Rejected(String),
}

/// A configured terminal. Cheap to build: the engine builds one per
/// payment from the current configuration.
pub struct Terminal {
    driver: Driver,
}

enum Driver {
    Worldline(worldline::WorldlineTerminalApi),
    CcvCloud(ccv_cloud::CcvPaymentApi),
    CcvOpi(ccv_opi::CcvOpi),
}

impl Terminal {
    /// The terminal described by `config`. `http` is the engine's HTTP
    /// client (its TLS roots, `HttpTls`); `software` is the till's
    /// software identity sent where a protocol asks for it
    /// (`pili-pos/1.0.0`).
    pub fn new(
        config: &PaymentTerminalConfig,
        http: reqwest::Client,
        software: &str,
    ) -> Result<Self, EftError> {
        let problems = config.problems();
        if let Some(p) = problems.first() {
            return Err(EftError::Config(p.clone()));
        }
        let driver = match config.kind {
            PaymentTerminalKind::WorldlineTerminalApi => Driver::Worldline(
                worldline::WorldlineTerminalApi::new(config, http, software)?,
            ),
            PaymentTerminalKind::CcvPaymentApi => {
                Driver::CcvCloud(ccv_cloud::CcvPaymentApi::new(config, http)?)
            }
            PaymentTerminalKind::CcvOpi => Driver::CcvOpi(ccv_opi::CcvOpi::new(config, software)?),
        };
        Ok(Self { driver })
    }

    /// Charge `request.amount_cents` on the terminal and wait for the
    /// answer, at most the configured timeout. `Err` only when nothing
    /// can have been charged; once the request may have reached the
    /// terminal, every failure is [`Outcome::Unknown`].
    pub async fn pay(&self, request: &PaymentRequest) -> Result<Outcome, EftError> {
        match &self.driver {
            Driver::Worldline(d) => d.pay(request).await,
            Driver::CcvCloud(d) => d.pay(request).await,
            Driver::CcvOpi(d) => d.pay(request).await,
        }
    }

    /// The outcome of the payment `reference` (an earlier
    /// [`PaymentRequest::reference`]), asked again: after a lost
    /// answer, a crash, or an [`Outcome::Unknown`].
    pub async fn status(&self, reference: &str) -> Result<Outcome, EftError> {
        match &self.driver {
            Driver::Worldline(d) => d.status(reference).await,
            Driver::CcvCloud(d) => d.status(reference).await,
            Driver::CcvOpi(d) => d.status(reference).await,
        }
    }

    /// Ask the terminal to stop the payment `reference` in progress.
    /// The payment's own [`Terminal::pay`] then returns
    /// [`Outcome::Cancelled`], or an approval when the abort came too
    /// late: the abort is a request, never a guarantee.
    pub async fn abort(&self, reference: &str) -> Result<(), EftError> {
        match &self.driver {
            Driver::Worldline(d) => d.abort(reference).await,
            Driver::CcvCloud(d) => d.abort(reference).await,
            Driver::CcvOpi(d) => d.abort(reference).await,
        }
    }
}

/// The configured wait for one payment's answer.
pub fn timeout(config: &PaymentTerminalConfig) -> Duration {
    Duration::from_secs(config.timeout_seconds)
}

/// `cents` as a decimal amount with exactly two decimals ("12.50"),
/// never through a float.
pub fn amount_text(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let c = cents.unsigned_abs();
    format!("{sign}{}.{:02}", c / 100, c % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_are_written_with_two_decimals() {
        assert_eq!(amount_text(1250), "12.50");
        assert_eq!(amount_text(5), "0.05");
        assert_eq!(amount_text(-300), "-3.00");
    }
}
