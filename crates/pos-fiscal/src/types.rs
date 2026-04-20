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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageCategory {
    SpfFod,
    Fdm,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Display {
    Mandatory,
    Optional,
    Never,
}
