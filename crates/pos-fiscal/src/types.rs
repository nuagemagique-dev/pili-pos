use serde::{Deserialize, Serialize};

// ── GraphQL Enums ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Language {
    En,
    Nl,
    Fr,
    De,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TicketMedium {
    None,
    Paper,
    Digital,
    PaperDigital,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InOut {
    In,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VatLabel {
    A,
    B,
    C,
    D,
    X,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

impl EventLabel {
    /// Label under which an event is signed, archived and counted.
    /// Every event recorded in training mode is a training event and
    /// always carries label T (POS detailed description v2.2 §1.5).
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "training is the POS-wide training-mode flag every caller already holds"
    )]
    #[must_use]
    pub fn in_mode(self, training: bool) -> Self {
        if training { Self::T } else { self }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QuantityType {
    Piece,
    Kilogram,
    Meter,
    Litre,
    Hour,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionLineType {
    SingleProduct,
    CompositeProduct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NegQuantityReason {
    Refund,
    Correction,
    PriceChange,
    CostCenterChange,
    ProductSubstitution,
    Voucher,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentType {
    Unknown,
    Cash,
    CardUnknown,
    CardDebit,
    CardCredit,
    CardOther,
    ChequeMeal,
    ChequeOther,
    App,
    Online,
    CustomerCredit,
    RoomCredit,
    LoyaltyRewards,
    VoucherStore,
    VoucherSupplier,
    VoucherOther,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InputMethod {
    Manual,
    Automatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentLineType {
    Payment,
    Tip,
    Rounding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MoneyInOutLineType {
    MoneyInOut,
    Tip,
    DrawerDeclaration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CostCenterType {
    Table,
    Chair,
    Room,
    Customer,
    OnHold,
    Kiosk,
    Platform,
    Webshop,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventOperation {
    WorkIn,
    WorkOut,
    Sale,
    Invoice,
    CostCenterChange,
    Order,
    PreBill,
    MoneyInOut,
    DrawerOpen,
    PaymentCorrection,
    Copy,
    ReportTurnoverX,
    ReportTurnoverZ,
    ReportUserX,
    ReportUserZ,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PriceChangeType {
    Public,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PriceChangeScope {
    Line,
    Event,
}

/// Message code (POS description v2.2, `enum Code`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    /// A code outside the spec's list (an FDM of another version, or a
    /// vendor code). The code itself is not kept: serde's `other` only
    /// allows a unit variant, and the message text stays in the archived
    /// response. Kept parseable so one unknown message never makes
    /// the whole Status unreadable, which would show the FDM as down.
    #[serde(other)]
    Unrecognized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageCategory {
    SpfFod,
    Fdm,
    /// Also any category outside the spec's list.
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShowPos {
    Mandatory,
    Optional,
    Never,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn training_events_are_always_labelled_t() {
        for label in [
            EventLabel::N,
            EventLabel::P,
            EventLabel::F,
            EventLabel::S,
            EventLabel::C,
            EventLabel::R,
        ] {
            assert_eq!(label.in_mode(true), EventLabel::T);
            assert_eq!(label.in_mode(false), label);
        }
    }
}
