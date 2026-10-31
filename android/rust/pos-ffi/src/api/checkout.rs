//! The checkout facade (engine modules checkout, unpaid, refund, copy, delivery, print): 15
//! cert commands as async methods of [`PosEngine`], plus the money helpers `settle_preview` and
//! `cash_due_cents` (PD62).
//!
//! Each method mirrors its Tauri wrapper (`src-tauri/src/ipc/{checkout,unpaid,refund,copy,
//! delivery,print}.rs`): the same parameter names, the single engine call (`do_checkout_split`,
//! `do_copy_ticket_with`, `do_refund_checkout_with` for the three `_with` commands), run on the
//! blocking pool for a synchronous engine function and on the engine's runtime for an async one.
//! Inputs go through `convert::inp`, outputs through `convert::out`.
//!
//! `check_fdm_status` and `list_fiscal_events` answer a `serde_json::Value` in the engine; here
//! they are typed ([`FdmStatusReport`], `Vec<FiscalEventSummary>`) from that JSON.

use std::collections::HashMap;

use pos_engine::commands::delivery::TicketDeliveryRequest as EngineDeliveryRequest;
use pos_engine::commands::errors;
use pos_engine::commands::refund::RefundItem as EngineRefundItem;
use pos_fiscal::payment::{self as engine_payment, PaymentError, PaymentPart as EnginePaymentPart};

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::checkout::{
    CopyResult, CopyableTicket, DeliveryView, FdmStatusReport, FiscalEventSummary, MailStatus,
    RefundItem, RefundableEvent, SettledPaymentRow, SettlementPreview,
};
use crate::records::common::{CheckoutResult, PaymentPart, TicketDeliveryRequest};

#[uniffi::export]
impl PosEngine {
    /// Sign and close a sale paid in one or several parts.
    pub async fn checkout(
        &self,
        order_id: i64,
        payments: Vec<PaymentPart>,
        delivery: Option<TicketDeliveryRequest>,
    ) -> Result<CheckoutResult, PosError> {
        let payments: Vec<EnginePaymentPart> = convert::inp(&payments)?;
        let delivery: Option<EngineDeliveryRequest> = convert::inp(&delivery)?;
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::checkout::do_checkout_split(
                    &engine,
                    order_id,
                    &payments,
                    delivery.as_ref(),
                )
                .await
            })
            .await?;
        convert::out(&result)
    }

    /// Check the FDM connection and read its status (messages in the operator's language).
    pub async fn check_fdm_status(&self) -> Result<FdmStatusReport, PosError> {
        let value = self
            .spawn(|engine| async move {
                pos_engine::commands::checkout::do_check_fdm_status(&engine).await
            })
            .await?;
        convert::out(&value)
    }

    /// Close an order unpaid, confirmed by a manager's INSZ and PIN.
    pub async fn close_unpaid(
        &self,
        order_id: i64,
        insz: String,
        pin: String,
    ) -> Result<CheckoutResult, PosError> {
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::unpaid::do_close_unpaid(&engine, order_id, &insz, &pin).await
            })
            .await?;
        convert::out(&result)
    }

    /// Recent closed sales that can be refunded.
    pub async fn list_refundable_events(
        &self,
        limit: Option<i64>,
    ) -> Result<Vec<RefundableEvent>, PosError> {
        let events = self
            .blocking(move |state| {
                pos_engine::commands::refund::do_list_refundable_events(state, limit)
            })
            .await?;
        convert::out(&events)
    }

    /// Refund a signed sale, in full or in part.
    pub async fn refund_checkout(
        &self,
        original_fiscal_event_id: i64,
        items: Vec<RefundItem>,
        employee_insz: String,
        delivery: Option<TicketDeliveryRequest>,
    ) -> Result<CheckoutResult, PosError> {
        let items: Vec<EngineRefundItem> = convert::inp(&items)?;
        let delivery: Option<EngineDeliveryRequest> = convert::inp(&delivery)?;
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::refund::do_refund_checkout_with(
                    &engine,
                    original_fiscal_event_id,
                    items,
                    employee_insz,
                    delivery.as_ref(),
                )
                .await
            })
            .await?;
        convert::out(&result)
    }

    /// Create a copy (event C) of a previous fiscal event.
    pub async fn copy_ticket(
        &self,
        fiscal_event_id: i64,
        employee_insz: String,
        delivery: Option<TicketDeliveryRequest>,
    ) -> Result<CopyResult, PosError> {
        let delivery: Option<EngineDeliveryRequest> = convert::inp(&delivery)?;
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::copy::do_copy_ticket_with(
                    &engine,
                    fiscal_event_id,
                    employee_insz,
                    delivery.as_ref(),
                )
                .await
            })
            .await?;
        convert::out(&result)
    }

    /// Recent tickets a customer received, for the copy picker (managers only).
    pub async fn list_copyable_tickets(
        &self,
        limit: Option<i64>,
    ) -> Result<Vec<CopyableTicket>, PosError> {
        let tickets = self
            .blocking(move |state| {
                pos_engine::commands::copy::do_list_copyable_tickets(state, limit)
            })
            .await?;
        convert::out(&tickets)
    }

    /// Recent fiscal events.
    pub async fn list_fiscal_events(
        &self,
        limit: Option<i64>,
    ) -> Result<Vec<FiscalEventSummary>, PosError> {
        let events = self
            .blocking(move |state| pos_engine::commands::copy::do_list_fiscal_events(state, limit))
            .await?;
        convert::out(&events)
    }

    /// Email availability for the ticket picker.
    pub async fn check_mail_status(&self) -> Result<MailStatus, PosError> {
        let status = self
            .spawn(|engine| async move {
                pos_engine::commands::delivery::do_check_mail_status(&engine).await
            })
            .await?;
        convert::out(&status)
    }

    /// Recent ticket deliveries (never the address itself).
    pub async fn list_ticket_deliveries(
        &self,
        limit: Option<i64>,
    ) -> Result<Vec<DeliveryView>, PosError> {
        let deliveries = self
            .blocking(move |state| {
                pos_engine::commands::delivery::do_list_ticket_deliveries(state, limit)
            })
            .await?;
        convert::out(&deliveries)
    }

    /// Print an original that was never printed (printer failure at checkout, or a digital
    /// ticket whose email did not go out).
    pub async fn print_missing_original(&self, fiscal_event_id: i64) -> Result<(), PosError> {
        self.spawn(move |engine| async move {
            pos_engine::commands::delivery::do_print_missing_original(&engine, fiscal_event_id)
                .await
        })
        .await
    }

    /// Whether the ticket printer is reachable.
    pub async fn check_printer(&self) -> Result<bool, PosError> {
        self.spawn(
            |engine| async move { pos_engine::commands::print::do_check_printer(&engine).await },
        )
        .await
    }

    /// Whether the bar / kitchen printer is reachable; `None` when the terminal has none.
    pub async fn check_kitchen_printer(&self) -> Result<Option<bool>, PosError> {
        self.spawn(|engine| async move {
            pos_engine::commands::print::do_check_kitchen_printer(&engine).await
        })
        .await
    }

    /// Print the kitchen ticket of an online order (non-fiscal).
    pub async fn print_webshop_kitchen(&self, order_id: i64) -> Result<(), PosError> {
        self.spawn(move |engine| async move {
            pos_engine::commands::print::do_print_webshop_kitchen(&engine, order_id).await
        })
        .await
    }

    /// Print the customer slip of an online order (non-fiscal).
    pub async fn print_webshop_customer(&self, order_id: i64) -> Result<(), PosError> {
        self.spawn(move |engine| async move {
            pos_engine::commands::print::do_print_webshop_customer(&engine, order_id).await
        })
        .await
    }
}

// ───────────────────────────── Money helpers (PD62) ─────────────────────────────

/// The error `pos_engine::commands::checkout` answers for a refused payment: the code alone, no
/// values.
fn payment_error(e: PaymentError) -> PosError {
    let code = match e {
        PaymentError::Invalid => errors::PAYMENT_INVALID,
        PaymentError::ExceedsTotal => errors::PAYMENT_EXCEEDS_TOTAL,
        PaymentError::Incomplete => errors::PAYMENT_INCOMPLETE,
    };
    PosError::Coded {
        code: code.to_string(),
        params: HashMap::new(),
    }
}

/// How `parts` settle a sale of `total_cents`: the payment lines, the cash rounding and the cash
/// to hand over. Refused with the codes `checkout` answers: `payment_invalid`,
/// `payment_exceeds_total` or `payment_incomplete`. Like `checkout`, a part whose method is not
/// offered at the till (app, online) is `payment_invalid`.
#[uniffi::export]
#[expect(
    clippy::unused_async,
    reason = "UniFFI exports it as a Kotlin suspend fun: the signature is the contract"
)]
pub async fn settle_preview(
    total_cents: i64,
    parts: Vec<PaymentPart>,
) -> Result<SettlementPreview, PosError> {
    let parts: Vec<EnginePaymentPart> = convert::inp(&parts)?;
    if parts.iter().any(|p| !p.method.is_offered_at_till()) {
        return Err(payment_error(PaymentError::Invalid));
    }
    let settlement = engine_payment::settle(total_cents, &parts).map_err(payment_error)?;
    let payments: Vec<SettledPaymentRow> = settlement
        .payments
        .iter()
        .map(|p| {
            Ok(SettledPaymentRow {
                method: convert::out(&p.method)?,
                amount_cents: p.amount_cents,
            })
        })
        .collect::<Result<_, PosError>>()?;
    Ok(SettlementPreview {
        payments,
        rounding_cents: settlement.rounding_cents,
        cash_due_cents: settlement.cash_due_cents(),
        has_cash: settlement.has_cash(),
    })
}

/// The cash to hand over for a sale of `total_cents` paid with `parts`: `settle_preview`'s
/// `cash_due_cents`.
#[uniffi::export]
pub async fn cash_due_cents(total_cents: i64, parts: Vec<PaymentPart>) -> Result<i64, PosError> {
    Ok(settle_preview(total_cents, parts).await?.cash_due_cents)
}
