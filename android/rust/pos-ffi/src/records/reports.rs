//! Records of the reports facade: reports, shifts, cash movements and settings (the `reports`
//! rows of the ownership table in [`super`]). Each type mirrors an engine type (PD56): same field
//! names, same serde attributes, `deny_unknown_fields`, `u32`/`u64` as `i64`.
//!
//! Common types are imported, never redefined: `FdmHealth` (and `FdmSeverity`) live in
//! [`super::common`].
//!
//! Types not in the T4 table: none.

/// `pos_engine::commands::report::ReportResult`: the outcome of an X, Z or user X report. A
/// refusal of the FDM is a result with `success = false` and the text in `error`, not an error.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportResult {
    pub success: bool,
    pub report_type: String,
    pub report_no: Option<i64>,
    pub error: Option<String>,
}

/// `pos_db::queries::reports::ZReportSummary`: one row of the Z-report history.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ZReportSummary {
    pub id: i64,
    pub report_date: String,
    pub sequence_number: i64,
    pub created_at: String,
    pub fiscal_event_id: Option<i64>,
}

/// `pos_engine::commands::social::SocialResult`: the outcome of a clock-in or clock-out.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SocialResult {
    pub success: bool,
    pub error: Option<String>,
}

/// `pos_db::queries::shifts::OpenShift`: an employee clocked in and not yet clocked out.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenShift {
    pub user_insz: String,
    /// UTC, RFC 3339.
    pub started_at: String,
}

/// `pos_engine::commands::financial::MoneyDirection`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum MoneyDirection {
    In,
    Out,
    /// The cash counted in the drawer: the amount is what is there, not a movement.
    Declaration,
}

/// `pos_engine::commands::financial::FinancialResult`: the outcome of a cash movement or a
/// fiscal drawer opening.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FinancialResult {
    pub success: bool,
    pub error: Option<String>,
}

/// `pos_engine::commands::payment_correction::SalePaymentLine`: one line of a sale's `financials`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SalePaymentLine {
    /// Position in the sale's `financials`: the index to correct.
    pub index: i32,
    /// `CASH`, `CARD_DEBIT`, `CHEQUE_MEAL`, ...
    pub payment_type: String,
    pub name: String,
    pub amount_cents: i64,
    /// `MANUAL` or `AUTOMATIC` (a card the linked terminal took).
    pub input_method: String,
    /// `PAYMENT` or `ROUNDING`.
    pub amount_type: String,
    pub brand: Option<String>,
}

/// `pos_engine::commands::payment_correction::CorrectionDone`: the correction a sale already has.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CorrectionDone {
    pub from_type: String,
    pub to_type: String,
    pub amount_cents: i64,
    pub at: String,
}

/// `pos_engine::commands::payment_correction::SalePayments`: the payment lines of a signed sale
/// and whether one of them can be corrected.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SalePayments {
    pub fiscal_event_id: i64,
    pub ticket_number: i64,
    pub total_cents: i64,
    pub lines: Vec<SalePaymentLine>,
    pub correctable: bool,
    /// Why not, as an engine error code, when `correctable` is false.
    pub refusal: Option<String>,
    pub corrected: Option<CorrectionDone>,
}

/// `pos_engine::commands::settings::PosSettings`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PosSettings {
    pub training_mode: bool,
    pub printer_address: Option<String>,
    pub fdm_url: String,
    pub pos_id: String,
    pub terminal_id: String,
    pub restaurant_name: String,
    pub language: String,
    /// What the FDM would refuse in `pos.json`.
    pub config_problems: Vec<String>,
    /// The software version (`posSwVersion`), for example `pili-pos-android/1.0.0`.
    pub pos_sw_version: String,
    /// The kind of the linked card payment terminal (`worldline_terminal_api`,
    /// `ccv_payment_api`, `ccv_opi`); `None` when cards are typed by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[uniffi(default = None)]
    pub payment_terminal: Option<String>,
}
