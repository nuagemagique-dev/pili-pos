//! Records of the card payment terminal (engine module `eft`): the payment as the checkout screen
//! shows it. Mirrors the engine type (PD56): same field names, same serde form,
//! `deny_unknown_fields`.

/// `pos_engine::commands::eft::EftPayment`: a card payment asked of the linked terminal.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct EftPayment {
    pub id: i64,
    pub order_id: i64,
    pub amount_cents: i64,
    /// `sent`, `approved`, `declined`, `cancelled`, `unknown` or `not_sent`.
    pub status: String,
    /// `debit`, `credit` or `unknown`; set once approved.
    pub card_type: Option<String>,
    pub brand: Option<String>,
    pub terminal_reference: Option<String>,
    pub authorisation_code: Option<String>,
    pub masked_pan: Option<String>,
    /// Why it was declined, what is missing for an unknown one.
    pub detail: Option<String>,
    pub created_at: String,
    /// A sale has settled it.
    pub used: bool,
}

/// `pos_engine::commands::eft::PaymentTerminalStatus`: whether the linked terminal answers.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct PaymentTerminalStatus {
    pub configured: bool,
    /// The driver; `None` when not configured.
    pub kind: Option<String>,
    /// The terminal answered (or a payment is running on it).
    pub connected: bool,
    /// A payment holds the terminal's line: it was not probed.
    pub busy: bool,
    /// Why it is not connected.
    pub detail: Option<String>,
}

/// `pos_engine::commands::eft::UnsettledCardPayment`: a card payment the terminal approved that no
/// sale has settled yet.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct UnsettledCardPayment {
    pub payment_id: i64,
    pub order_id: i64,
    /// `dine_in` or `takeaway`.
    pub order_type: String,
    /// The table of a dine-in order.
    pub table_number: Option<i32>,
    pub amount_cents: i64,
    pub brand: Option<String>,
    pub created_at: String,
}
