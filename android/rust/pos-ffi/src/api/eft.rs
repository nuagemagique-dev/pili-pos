//! The card payment terminal facade (engine module eft): six cert commands as async methods of
//! [`PosEngine`].
//!
//! Each method mirrors its Tauri wrapper (`src-tauri/src/ipc/eft.rs`): the same parameter names
//! and the single engine call. `eft_pay` may wait minutes for the customer: it runs on the
//! engine's runtime and holds no lock across the terminal wait (the engine takes the database
//! lock only around its own statements). Dropping the Kotlin coroutine does NOT abort the
//! payment: the runtime task is detached and still stores the terminal's answer; the operator
//! stops a payment with `eft_abort`.

use pos_engine::commands::eft::{
    EftPayment as EngineEftPayment, PaymentTerminalStatus as EnginePaymentTerminalStatus,
    UnsettledCardPayment as EngineUnsettledCardPayment,
};

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::eft::{EftPayment, PaymentTerminalStatus, UnsettledCardPayment};

#[uniffi::export]
impl PosEngine {
    /// Ask the linked card terminal to charge `amount_cents` for an order. The answer is the
    /// stored payment whatever the terminal said (approved, declined, cancelled, unknown). An
    /// error is a refusal before the terminal was asked (nothing was charged) or, in the unlikely
    /// case the stored payment cannot be converted to the record, a contract error after the
    /// terminal answered: a caller that gets `Internal` must read `list_eft_payments` before it
    /// charges again.
    pub async fn eft_pay(&self, order_id: i64, amount_cents: i64) -> Result<EftPayment, PosError> {
        let payment: EngineEftPayment = self
            .spawn(move |engine| async move {
                pos_engine::commands::eft::do_eft_pay(&engine, order_id, amount_cents).await
            })
            .await?;
        convert::out(&payment)
    }

    /// Stop the card payment in progress on the terminal, if any (a request: the payment's own
    /// call ends cancelled, or approved when the abort came too late).
    pub async fn eft_abort(&self) -> Result<(), PosError> {
        self.spawn(|engine| async move { pos_engine::commands::eft::do_eft_abort(&engine).await })
            .await
    }

    /// Ask the terminal where a payment that has no final answer stands.
    pub async fn eft_check(&self, id: i64) -> Result<EftPayment, PosError> {
        let payment: EngineEftPayment = self
            .spawn(move |engine| async move {
                pos_engine::commands::eft::do_eft_check(&engine, id).await
            })
            .await?;
        convert::out(&payment)
    }

    /// An order's card payments to settle or to check, oldest first.
    pub async fn list_eft_payments(&self, order_id: i64) -> Result<Vec<EftPayment>, PosError> {
        let payments: Vec<EngineEftPayment> = self
            .blocking(move |state| pos_engine::commands::eft::do_list_eft_payments(state, order_id))
            .await?;
        convert::out(&payments)
    }

    /// Whether the linked card terminal answers. Charges nothing; while a payment runs it
    /// answers busy (and connected) without probing the terminal.
    pub async fn check_payment_terminal(&self) -> Result<PaymentTerminalStatus, PosError> {
        let status: EnginePaymentTerminalStatus = self
            .spawn(|engine| async move {
                pos_engine::commands::eft::do_check_payment_terminal(&engine).await
            })
            .await?;
        convert::out(&status)
    }

    /// The approved card payments no sale has settled yet, of every order, oldest first: the Z
    /// warns about them and the order lists mark their orders.
    #[expect(
        clippy::redundant_closure,
        reason = "check_command_parity.py wants the engine call spelled out"
    )]
    pub async fn list_unsettled_card_payments(
        &self,
    ) -> Result<Vec<UnsettledCardPayment>, PosError> {
        let payments: Vec<EngineUnsettledCardPayment> = self
            .blocking(|state| pos_engine::commands::eft::do_list_unsettled_card_payments(state))
            .await?;
        convert::out(&payments)
    }
}
