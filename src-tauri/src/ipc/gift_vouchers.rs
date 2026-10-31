use pos_core::order::OrderLine;
use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::gift_vouchers::{GiftVoucherSettings, GiftVoucherView};
use tauri::State;

use super::Managed;

/// The gift voucher setting (switch, validity, maximum amount).
#[tauri::command(async)]
pub fn get_gift_voucher_settings(state: State<'_, Managed>) -> Result<GiftVoucherSettings, String> {
    pos_engine::commands::gift_vouchers::do_get_gift_voucher_settings(state.inner())
}

/// Store the gift voucher setting; managers only.
#[tauri::command(async)]
pub fn set_gift_voucher_settings(
    state: State<'_, Managed>,
    input: GiftVoucherSettings,
) -> Result<GiftVoucherSettings, String> {
    pos_engine::commands::gift_vouchers::do_set_gift_voucher_settings(state.inner(), input)
}

/// Add a gift voucher of `amount_cents` to an open order.
#[tauri::command(async)]
pub fn add_gift_voucher_line(
    state: State<'_, Managed>,
    order_id: i64,
    amount_cents: i64,
) -> Result<OrderLine, String> {
    pos_engine::commands::gift_vouchers::do_add_gift_voucher_line(
        state.inner(),
        order_id,
        amount_cents,
    )
}

/// Find a gift voucher by its 12-digit code.
#[tauri::command(async)]
pub fn lookup_gift_voucher(
    state: State<'_, Managed>,
    code: String,
) -> Result<GiftVoucherView, String> {
    pos_engine::commands::gift_vouchers::do_lookup_gift_voucher(state.inner(), &code)
}

/// The gift vouchers, outstanding first; managers only.
#[tauri::command(async)]
pub fn list_gift_vouchers(
    state: State<'_, Managed>,
    status: Option<String>,
) -> Result<Vec<GiftVoucherView>, String> {
    pos_engine::commands::gift_vouchers::do_list_gift_vouchers(state.inner(), status)
}

/// Print a gift voucher again, marked DUPLICATA; managers only.
#[tauri::command]
pub async fn print_gift_voucher(
    state: State<'_, Managed>,
    voucher_id: i64,
) -> Result<SlipOutcome, String> {
    pos_engine::commands::gift_vouchers::do_print_gift_voucher(state.inner(), voucher_id).await
}
