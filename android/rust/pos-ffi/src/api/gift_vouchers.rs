//! The gift vouchers facade (engine module gift_vouchers, v1.2): six commands as async methods of
//! [`PosEngine`].
//!
//! Each method mirrors its Tauri wrapper (`src-tauri/src/ipc/gift_vouchers.rs`): the same
//! parameter names and the single engine call. Reading the setting is open to any logged-in
//! operator; selling and looking up a voucher is for cashiers; the setting, the list and the
//! duplicate are for managers.

use pos_engine::commands::gift_vouchers;

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::common::SlipOutcome;
use crate::records::gift_vouchers::{GiftVoucherSettings, GiftVoucherView};
use crate::records::orders::OrderLine;

#[uniffi::export]
impl PosEngine {
    /// The gift voucher setting (switch, validity, maximum amount).
    #[allow(clippy::redundant_closure)] // check_command_parity.py wants the engine call spelled out
    pub async fn get_gift_voucher_settings(&self) -> Result<GiftVoucherSettings, PosError> {
        let settings = self
            .blocking(|st| gift_vouchers::do_get_gift_voucher_settings(st))
            .await?;
        convert::out(&settings)
    }

    /// Store the gift voucher setting; managers only.
    pub async fn set_gift_voucher_settings(
        &self,
        input: GiftVoucherSettings,
    ) -> Result<GiftVoucherSettings, PosError> {
        let input: gift_vouchers::GiftVoucherSettings = convert::inp(&input)?;
        let stored = self
            .blocking(move |st| gift_vouchers::do_set_gift_voucher_settings(st, input))
            .await?;
        convert::out(&stored)
    }

    /// Add a gift voucher of `amount_cents` to an open order.
    pub async fn add_gift_voucher_line(
        &self,
        order_id: i64,
        amount_cents: i64,
    ) -> Result<OrderLine, PosError> {
        let line = self
            .blocking(move |st| gift_vouchers::do_add_gift_voucher_line(st, order_id, amount_cents))
            .await?;
        convert::out(&line)
    }

    /// Find a gift voucher by its 12-digit code.
    pub async fn lookup_gift_voucher(&self, code: String) -> Result<GiftVoucherView, PosError> {
        let view = self
            .blocking(move |st| gift_vouchers::do_lookup_gift_voucher(st, &code))
            .await?;
        convert::out(&view)
    }

    /// The gift vouchers, outstanding first; managers only.
    pub async fn list_gift_vouchers(
        &self,
        status: Option<String>,
    ) -> Result<Vec<GiftVoucherView>, PosError> {
        let list = self
            .blocking(move |st| gift_vouchers::do_list_gift_vouchers(st, status))
            .await?;
        convert::out(&list)
    }

    /// Print a gift voucher again, marked DUPLICATA; managers only.
    pub async fn print_gift_voucher(&self, voucher_id: i64) -> Result<SlipOutcome, PosError> {
        let outcome = self
            .spawn(
                move |e| async move { gift_vouchers::do_print_gift_voucher(&e, voucher_id).await },
            )
            .await?;
        convert::out(&outcome)
    }
}
