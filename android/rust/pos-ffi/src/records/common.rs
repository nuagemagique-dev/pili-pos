//! Records shared by several facades, by the event payloads or by the money helpers (the `common`
//! rows of the table in [`super`]). Each type mirrors an engine type (PD56): same field names, same
//! serde attributes, `deny_unknown_fields` on records. Differences from the engine type, all
//! forced by UniFFI: derives (`Deserialize` on output types, `Serialize` on input types, so
//! `convert::out` and `convert::inp` work both ways), no unsigned integers, JSON values as text.
//!
//! The engine's `Display` enum (message display rule) is `FdmMessageDisplay` here, so the Kotlin
//! side never meets a type called `Display`.
//!
//! Two functions at the bottom, [`contract_out`] and [`contract_inp`], expose `convert::out` and
//! `convert::inp` to the integration tests (`tests/ffi/common.rs`), which cannot see `pub(crate)`.

use crate::convert::{self, json_text, json_text_opt};
use crate::error::PosError;
use serde::Serialize;
use serde::de::DeserializeOwned;

// ───────────────────────────── VAT and payment ─────────────────────────────

/// `pos_core::vat::VatCode`: the VAT class of a menu item or order line.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
pub enum VatCode {
    /// 21% (beverages with alcohol, soft drinks).
    A,
    /// 12% (restaurant services).
    B,
    /// 6% (takeaway food).
    C,
    /// 0% (exempt).
    D,
    /// Out of scope.
    X,
}

/// `pos_fiscal::payment::PaymentMethod`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    Cash,
    Card,
    /// Electronic meal vouchers, paid on the card terminal.
    MealVoucher,
    /// A multi-purpose gift voucher sold by the restaurant (`VOUCHER_STORE`).
    GiftVoucher,
    /// Paid in an ordering platform's app. Never offered at the till.
    App,
    /// Paid online on the restaurant's own webshop. Never offered at the till.
    Online,
}

/// `pos_fiscal::payment::PaymentPart`: one part of the payment of a sale. A cash part has no
/// amount (it pays what is left, rounded); the other methods carry theirs.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct PaymentPart {
    pub method: PaymentMethod,
    #[serde(default)]
    pub amount_cents: Option<i64>,
    /// The card payment the linked terminal approved for this part (`eft_payments.id`); `None`
    /// for a card typed by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub eft_payment_id: Option<i64>,
    /// The gift voucher this part redeems (`gift_vouchers.id`, from `lookup_gift_voucher`); only
    /// a gift voucher part names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub gift_voucher_id: Option<i64>,
}

// ───────────────────────────── Ticket delivery ─────────────────────────────

/// `pos_engine::commands::delivery::RequestedMedium`: what the customer asked for.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum RequestedMedium {
    #[default]
    Paper,
    Digital,
    PaperDigital,
}

/// `pos_engine::commands::delivery::TicketDeliveryRequest`: how the customer wants the ticket.
#[derive(
    Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TicketDeliveryRequest {
    #[serde(default)]
    pub medium: RequestedMedium,
    #[serde(default)]
    pub email: Option<String>,
}

/// `pos_engine::commands::delivery::EmailStatus`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum EmailStatus {
    NotRequested,
    Sent,
    /// Not sent yet; the background worker keeps trying.
    Queued,
}

/// `pos_fiscal::types::TicketMedium`: the medium the ticket was signed for.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TicketMedium {
    None,
    Paper,
    Digital,
    PaperDigital,
}

/// `pos_engine::commands::delivery::DeliveryOutcome`: how the ticket reached the customer.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeliveryOutcome {
    pub fiscal_event_id: i64,
    pub delivery_id: Option<i64>,
    pub medium: TicketMedium,
    pub email: EmailStatus,
    pub recipient_hint: Option<String>,
    pub paper_printed: bool,
    /// The customer should have a paper ticket but none printed.
    pub needs_paper: bool,
}

/// `pos_engine::commands::bar_slip::SlipOutcome`: what happened to the bar or kitchen slip.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum SlipOutcome {
    Printed,
    /// No new line for the bar or kitchen.
    NothingToPrint,
    /// Neither `kitchen_printer_address` nor `printer_address` is set.
    NoPrinter,
    Failed,
}

// ───────────────────────────── Fiscal ticket ─────────────────────────────

/// `pos_fiscal::types::EventLabel`: the FDM event label.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventLabel {
    N,
    P,
    F,
    S,
    I,
    R,
    C,
    T,
}

/// `pos_fiscal::types::VatLabel`: the VAT label of the FDM.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VatLabel {
    A,
    B,
    C,
    D,
    X,
}

/// `pos_fiscal::ticket::ReceiptLanguage`: the language of the printed mentions.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    uniffi::Enum,
)]
#[serde(rename_all = "lowercase")]
pub enum ReceiptLanguage {
    #[default]
    Fr,
    Nl,
    De,
}

/// `pos_fiscal::response::FdmReference`: the FDM's control data of an event (camelCase, as the
/// FDM answers it).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FdmReference {
    pub fdm_id: String,
    pub fdm_date_time: String,
    pub event_label: EventLabel,
    pub event_counter: i64,
    pub total_counter: i64,
}

/// `pos_fiscal::response::VatCalcItem`: one line of the VAT breakdown. The euro amounts are
/// `f64`, for display only.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VatCalcItem {
    pub label: VatLabel,
    pub rate: f64,
    pub taxable_amount: f64,
    pub vat_amount: f64,
    pub total_amount: f64,
    pub out_of_scope: bool,
}

/// `pos_fiscal::ticket::TicketLine`: one line of the ticket (euro amounts, display only).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct TicketLine {
    pub quantity: f64,
    pub description: String,
    pub unit_price: f64,
    pub total: f64,
    pub vat_code: String,
}

/// `pos_fiscal::ticket::TicketPayment`: one payment row of the ticket (euro amount).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct TicketPayment {
    pub method: String,
    pub amount: f64,
    /// The card brand a linked terminal returned ("Bancontact"); `None` for every other payment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub brand: Option<String>,
}

/// `pos_fiscal::ticket::OriginalControlData`: control data of the event a refund or copy refers to.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct OriginalControlData {
    pub fdm_ref: FdmReference,
    pub short_signature: Option<String>,
}

/// `pos_fiscal::ticket::FiscalTicket`: everything printed on a ticket and shown on the success
/// screen. The euro amounts stay `f64` (display only, the engine and the Vue contract).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct FiscalTicket {
    pub restaurant_name: String,
    pub restaurant_address: String,
    pub restaurant_vat: String,
    pub restaurant_est_no: String,
    #[serde(default)]
    pub restaurant_legal_name: String,

    pub pos_id: String,
    #[serde(default)]
    pub pos_sw_version: String,
    pub terminal_id: String,
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub operator: String,
    pub pos_datetime: String,
    pub ticket_number: i64,
    pub event_label: EventLabel,
    pub is_training: bool,

    pub lines: Vec<TicketLine>,

    pub subtotal: f64,
    pub total: f64,

    pub vat_breakdown: Vec<VatCalcItem>,

    pub payments: Vec<TicketPayment>,
    pub rounding: Option<f64>,

    pub fdm_ref: Option<FdmReference>,
    pub short_signature: Option<String>,
    pub verification_url: Option<String>,
    pub footer: Vec<String>,

    pub ticket_type_label: String,
    pub is_refund: bool,
    #[serde(default)]
    pub lang: ReceiptLanguage,
    #[serde(default)]
    pub original: Option<OriginalControlData>,
    #[serde(default)]
    pub copy_made_at: Option<String>,
    #[serde(default)]
    pub unpaid: bool,
}

/// `pos_engine::commands::checkout::CheckoutResult`: the answer of every signing command that
/// ends a sale or a refund (checkout, unpaid, refund, webshop handover).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct CheckoutResult {
    pub success: bool,
    pub ticket: Option<FiscalTicket>,
    pub error: Option<String>,
    pub fdm_connected: bool,
    /// Archive id of the signed event.
    pub fiscal_event_id: Option<i64>,
    /// How the ticket reached the customer (paper and/or email).
    pub delivery: Option<DeliveryOutcome>,
    /// The bar or kitchen slip of the lines not yet sent (sales only).
    pub kitchen_slip: Option<SlipOutcome>,
    /// The gift vouchers the sale sold and the balance slips it gave; empty otherwise.
    #[serde(default)]
    #[uniffi(default = [])]
    pub gift_voucher_prints: Vec<GiftVoucherPrint>,
}

/// `pos_engine::commands::gift_vouchers::GiftVoucherPrint`: what became of one voucher slip after a
/// sale.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct GiftVoucherPrint {
    pub voucher_id: i64,
    /// `voucher` (the voucher itself) or `balance` (what is left on it).
    pub kind: String,
    pub outcome: SlipOutcome,
}

// ───────────────────────────── FDM health and messages ─────────────────────────────

/// `pos_engine::commands::fdm_monitor::FdmSeverity`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "lowercase")]
pub enum FdmSeverity {
    Blocking,
    Warning,
    Info,
}

/// `pos_engine::commands::fdm_monitor::FdmHealth`: the classified state of the FDM.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FdmHealth {
    pub connected: bool,
    pub answered: bool,
    pub severity: FdmSeverity,
    /// The code to show, or the raw transport error text.
    pub code: Option<String>,
    pub mandatory: bool,
    pub checked_at: String,
    /// When the monitor checks next; `None` without a session.
    pub next_check_at: Option<String>,
}

/// `pos_fiscal::types::Display` (renamed): whether the POS must show a message.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FdmMessageDisplay {
    Mandatory,
    Optional,
    Never,
}

/// `pos_fiscal::types::MessageCategory`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageCategory {
    SpfFod,
    Fdm,
    /// Also any category outside the spec's list.
    #[serde(other)]
    Other,
}

/// `pos_fiscal::types::FdmWarningCode`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FdmWarningCode {
    ClientCertNearExpiration,
    ClientCertExpired,
    RtcSyncFailed,
    UpdateUrlsFailed,
    UpdateTrustCertFailed,
    UpdateTaskListFailed,
    TaskFeedbackFailed,
    NopFailed,
    BufferNearFull,
    ServerCertResolveFailed,
    TransactionUploadFailed,
    InitializationFailed,
    RtcNotInitialized,
    CorruptRecordEncountered,
    UpdateParamsFailed,
    UpdateClientCertFailed,
    UpdateVatRatesFailed,
    UpdatePosAllowlistFailed,
    DuplicateRequest,
    BufferFull,
    FdmLocked,
    Unauthorized,
    InvalidRequest,
    InternalError,
    UndefinedError,
    UndefinedOther,
    FdmNotOperational,
    UnknownPos,
    UpdatePosVatno,
    UpdatePosEstno,
    /// A code outside the spec's list.
    #[serde(other)]
    Unrecognized,
}

/// `pos_fiscal::response::DataItem`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DataItem {
    pub name: String,
    pub value: String,
}

/// `pos_fiscal::response::LocationItem`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocationItem {
    pub line: i32,
    pub column: i32,
}

/// `pos_fiscal::response::ExtensionItem`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionItem {
    pub category: MessageCategory,
    pub code: FdmWarningCode,
    pub data: Option<Vec<DataItem>>,
    pub show_pos: FdmMessageDisplay,
}

/// `pos_fiscal::response::MessageItem`: one warning or information of the FDM.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessageItem {
    pub message: String,
    pub locations: Option<Vec<LocationItem>>,
    pub extensions: ExtensionItem,
}

/// `pos_fiscal::fdm_client::SignMessages`: what the FDM attached to a signed event.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SignMessages {
    pub mutation: String,
    /// The request's `bookingDate` (`reportBookingDate` for a report).
    pub booking_date: Option<String>,
    pub warnings: Vec<MessageItem>,
    pub informations: Vec<MessageItem>,
}

// ───────────────────────────── Session and presence events ─────────────────────────────

/// `pos_engine::commands::session::LockReason`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum LockReason {
    /// The operator pressed the lock button.
    Manual,
    /// The idle delay ran out.
    Idle,
}

/// `pos_engine::commands::session::SessionLocked`: payload of the `session-locked` event.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionLocked {
    pub reason: LockReason,
    pub order_error: Option<String>,
}

/// `pos_engine::commands::social::PresenceFailed`: payload of the `presence-failed` event.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresenceFailed {
    pub insz: String,
    /// Always true for now.
    pub clock_in: bool,
    pub error: String,
}

// ───────────────────────────── HubRise callback event ─────────────────────────────

/// `pos_hubrise_types::CallbackEvent`: the envelope of every HubRise callback. The engine type
/// flattens the fields it does not know (`#[serde(flatten)]` map), which a serde mirror cannot
/// reproduce, so the conversion is explicit: [`CallbackEvent::from_engine`] and
/// [`CallbackEvent::to_engine`]. `previous_state` and `new_state` are JSON text (`None` for null),
/// `extra` is the JSON text of an object holding the flattened remaining fields (`{}` for none).
///
/// The derived serde form of this record (used by nothing but the generic derive checks) is the
/// FFI form: `previous_state`, `new_state` and `extra` as nested JSON.
// contract: custom
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
pub struct CallbackEvent {
    pub id: String,
    pub resource_type: String,
    pub event_type: String,
    pub created_at: String,
    #[serde(default, with = "json_text_opt")]
    pub previous_state: Option<String>,
    #[serde(default, with = "json_text_opt")]
    pub new_state: Option<String>,
    #[serde(with = "json_text")]
    pub extra: String,
}

fn callback_contract_error(what: &str, err: impl std::fmt::Display) -> PosError {
    let detail = format!("contract: CallbackEvent: {what}: {err}");
    tracing::error!(%detail, "FFI contract failure");
    PosError::Internal { detail }
}

impl CallbackEvent {
    /// Engine envelope to record. Fails only if a JSON value cannot be rendered as text.
    pub fn from_engine(event: &pos_hubrise_types::CallbackEvent) -> Result<Self, PosError> {
        let text = |what: &str, v: &serde_json::Value| {
            serde_json::to_string(v).map_err(|e| callback_contract_error(what, e))
        };
        let state =
            |what: &str, v: &Option<serde_json::Value>| -> Result<Option<String>, PosError> {
                v.as_ref().map(|v| text(what, v)).transpose()
            };
        Ok(Self {
            id: event.id.clone(),
            resource_type: event.resource_type.clone(),
            event_type: event.event_type.clone(),
            created_at: event.created_at.clone(),
            previous_state: state("previous_state", &event.previous_state)?,
            new_state: state("new_state", &event.new_state)?,
            extra: text("extra", &serde_json::Value::Object(event.extra.clone()))?,
        })
    }

    /// Record to engine envelope. `extra` must be the text of a JSON object.
    pub fn to_engine(&self) -> Result<pos_hubrise_types::CallbackEvent, PosError> {
        let parse = |what: &str, text: &str| {
            serde_json::from_str::<serde_json::Value>(text)
                .map_err(|e| callback_contract_error(what, e))
        };
        let state =
            |what: &str, v: &Option<String>| -> Result<Option<serde_json::Value>, PosError> {
                v.as_deref().map(|t| parse(what, t)).transpose()
            };
        let serde_json::Value::Object(extra) = parse("extra", &self.extra)? else {
            return Err(callback_contract_error("extra", "not a JSON object"));
        };
        Ok(pos_hubrise_types::CallbackEvent {
            id: self.id.clone(),
            resource_type: self.resource_type.clone(),
            event_type: self.event_type.clone(),
            created_at: self.created_at.clone(),
            previous_state: state("previous_state", &self.previous_state)?,
            new_state: state("new_state", &self.new_state)?,
            extra,
        })
    }
}

// ───────────────────────────── Test access ─────────────────────────────

/// `convert::out` for the integration tests: an engine value to its FFI record.
#[doc(hidden)]
pub fn contract_out<E: Serialize, F: DeserializeOwned>(v: &E) -> Result<F, PosError> {
    convert::out(v)
}

/// `convert::inp` for the integration tests: an FFI record to its engine value.
#[doc(hidden)]
pub fn contract_inp<F: Serialize, E: DeserializeOwned>(v: &F) -> Result<E, PosError> {
    convert::inp(v)
}
