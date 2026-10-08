//! Records of the checkout domain (engine modules checkout, unpaid, refund, copy, delivery, print):
//! the types the ownership table in [`super`] assigns to `checkout`, plus the money helpers'
//! answer. Each mirrors an engine type (PD56): same field names, same serde attributes,
//! `deny_unknown_fields`. The members shared with other domains (`CheckoutResult`, `FiscalTicket`,
//! `DeliveryOutcome`, `PaymentPart`, `MessageItem`, ...) are in `records/common.rs` and imported,
//! never redefined.
//!
//! Two records have no engine struct: the engine answers `check_fdm_status` and
//! `list_fiscal_events` with a `serde_json::Value`, so [`FdmStatusReport`] and
//! [`FiscalEventSummary`] are typed from the JSON those commands build.
//!
//! Types not in the T4 table (they only exist for the money helpers, PD62, and are defined here by
//! the card that implements the helpers): [`SettlementPreview`] and [`SettledPaymentRow`].

use super::common::{DeliveryOutcome, FiscalTicket, MessageItem, PaymentMethod};

// ───────────────────────────── FDM status ─────────────────────────────

/// `pos_fiscal::response::FdmDevice`: the identity and clock of the FDM.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FdmDevice {
    pub fdm_id: String,
    pub fdm_sw_version: String,
    pub fdm_date_time: String,
    /// Share of the FDM's buffer in use, in percent.
    pub buffer_capacity_used: f64,
}

/// The answer of `check_fdm_status`, typed from the JSON of `do_check_fdm_status`. Two shapes
/// share it, so every field is optional and an absent key is `None`:
///
/// - the FDM answered (`pos_fiscal::response::FdmStatus`): `device`, `initialized` and the three
///   message lists (`None` when the FDM sent `null`), no `connected`, no `error`;
/// - the FDM did not answer: `connected = Some(false)` and `error` (the transport or GraphQL
///   error text), nothing else.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FdmStatusReport {
    #[serde(default)]
    pub connected: Option<bool>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub device: Option<FdmDevice>,
    #[serde(default)]
    pub initialized: Option<bool>,
    #[serde(default)]
    pub informations: Option<Vec<MessageItem>>,
    #[serde(default)]
    pub warnings: Option<Vec<MessageItem>>,
    #[serde(default)]
    pub errors: Option<Vec<MessageItem>>,
}

// ───────────────────────────── Fiscal events, refund, copy ─────────────────────────────

/// One element of the array `do_list_fiscal_events` builds (keys `id`, `eventType`,
/// `sequenceNumber`, `orderId`, `fdmId`, `fdmDatetime`, `eventLabel`, `shortSignature`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FiscalEventSummary {
    pub id: i64,
    pub event_type: String,
    pub sequence_number: i64,
    pub order_id: Option<i64>,
    pub fdm_id: Option<String>,
    /// Serialized as `fdmDatetime` (lowercase t), like the engine's JSON.
    pub fdm_datetime: Option<String>,
    pub event_label: Option<String>,
    pub short_signature: Option<String>,
}

/// `pos_db::queries::fiscal::RefundableEvent`: a closed sale the refund picker lists.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RefundableEvent {
    pub fiscal_event_id: i64,
    pub ticket_number: i64,
    pub order_id: i64,
    pub table_number: Option<i32>,
    pub total_cents: i64,
    pub created_at: String,
}

/// `pos_engine::commands::refund::RefundItem`: one item to refund. Price, VAT label, name and
/// department come from the original sale's line, never from the screen.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RefundItem {
    pub menu_item_id: i64,
    /// Positive: the engine negates it.
    pub quantity: i32,
    /// The original sale's line, needed when the product was sold at more than one price.
    #[serde(default)]
    pub line_id: Option<i64>,
}

/// `pos_db::queries::fiscal::CopyableTicket`: a ticket a customer received, for the copy picker.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyableTicket {
    pub fiscal_event_id: i64,
    /// N (sale or refund), P (pre-bill) or T (training).
    pub event_label: String,
    /// `posFiscalTicketNo`, as printed on the ticket.
    pub ticket_number: i64,
    pub table_number: Option<i32>,
    /// The ticket's own total, negative for a refund.
    pub total_cents: i64,
    pub created_at: String,
}

/// `pos_engine::commands::copy::CopyResult`: the answer of `copy_ticket`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyResult {
    pub success: bool,
    pub error: Option<String>,
    /// The copy ticket, when the original event had an order to show.
    pub ticket: Option<FiscalTicket>,
    /// Archive id of the signed C event.
    pub fiscal_event_id: Option<i64>,
    /// How the copy reached the customer (paper and/or email).
    pub delivery: Option<DeliveryOutcome>,
}

// ───────────────────────────── Ticket delivery ─────────────────────────────

/// `pos_engine::commands::delivery::MailStatus`: email availability for the ticket picker.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MailStatus {
    pub configured: bool,
    pub reachable: bool,
    pub allow_digital_only: bool,
}

/// `pos_engine::commands::delivery::DeliveryView`: a delivery as the settings screen shows it.
/// Never carries the address itself.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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

// ───────────────────────────── Money helpers (PD62) ─────────────────────────────

/// One payment line of a settled sale: `pos_fiscal::payment::SettledPayment`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct SettledPaymentRow {
    pub method: PaymentMethod,
    pub amount_cents: i64,
}

/// How `pos_fiscal::payment::settle` settles a sale: what the screen shows before the operator
/// confirms. `payments` and `rounding_cents` are the fields of `Settlement`; `cash_due_cents` and
/// `has_cash` are its two methods of the same names.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct SettlementPreview {
    /// Non-cash parts first, in the order taken, then the cash rest.
    pub payments: Vec<SettledPaymentRow>,
    /// Delta added to the cash rest (negative when rounded down).
    pub rounding_cents: i64,
    /// The cash amount to hand over, rounding included.
    pub cash_due_cents: i64,
    /// Part of the sale is paid in cash (the drawer opens).
    pub has_cash: bool,
}
