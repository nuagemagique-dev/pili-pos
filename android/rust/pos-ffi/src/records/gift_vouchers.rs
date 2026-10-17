//! Records of the gift vouchers (engine module `gift_vouchers`, v1.2): the setting, a voucher as
//! the screens show it, and its movements. Mirrors the engine types (PD56): same field names,
//! same serde form, `deny_unknown_fields`. `GiftVoucherPrint` (what became of a voucher slip
//! after a sale) is in `common` with `CheckoutResult`.

/// `pos_engine::commands::gift_vouchers::GiftVoucherSettings`: the gift voucher setting (off by
/// default; validity 12 to 36 months; maximum amount of one voucher).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct GiftVoucherSettings {
    #[serde(default)]
    pub enabled: bool,
    pub validity_months: i32,
    pub max_amount_cents: i64,
}

/// `pos_engine::commands::gift_vouchers::GiftVoucherMovement`: one movement of a voucher.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct GiftVoucherMovement {
    /// `issue`, `redeem` or `cancel`.
    pub kind: String,
    pub amount_cents: i64,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// The ticket that made it (`N 1234`).
    pub ticket: String,
}

/// `pos_engine::commands::gift_vouchers::GiftVoucherView`: a voucher as the screens show it (the
/// code is masked; only the printed voucher carries it whole).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct GiftVoucherView {
    pub id: i64,
    pub code_masked: String,
    pub face_value_cents: i64,
    pub balance_cents: i64,
    /// `YYYY-MM-DD`.
    pub issued_on: String,
    pub expires_on: String,
    /// `active`, `used`, `expired` or `cancelled`.
    pub status: String,
    pub training: bool,
    pub movements: Vec<GiftVoucherMovement>,
}
