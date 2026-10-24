//! The HubRise and webshop facade (engine modules hubrise, webshop): 19 cert commands as async
//! methods of [`PosEngine`].
//!
//! Each method mirrors its Tauri wrapper (`src-tauri/src/ipc/{hubrise,webshop}.rs`): the same
//! parameter names (`req` for the request records), the single engine call, run on the blocking
//! pool for a synchronous engine function and on the engine's runtime for an async one. The two
//! wrappers declared `async` over a synchronous engine function (`hubrise_patch_order_status`,
//! `hubrise_set_confirmed_time`, a queue write in SQLite) go through `spawn`, like the Tauri
//! wrapper that runs them on the async runtime: the command map and the parity check classify
//! a command by its wrapper. Inputs go through `convert::inp`, outputs
//! through `convert::out`; `hubrise_set_channel` answers an `i64` (the engine's `usize`).
//!
//! The HubRise commands refuse with the `hubrise_*` and `webshop_*` codes: `hubrise_disabled`
//! while the integration is switched off, `hubrise_not_connected` without a connection,
//! `hubrise_not_configured` for a release build without the `hubrise` section of `pos.json`.

use pos_engine::commands::delivery::TicketDeliveryRequest as EngineDeliveryRequest;
use pos_fiscal::payment::PaymentPart as EnginePaymentPart;

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::common::CheckoutResult;
use crate::records::hubrise::{
    AcceptRequest, AcceptResult, Adoption, ApplyOutcome, AuthorizeLink, Channel, HandoverRequest,
    HubriseConnectionInfo, HubriseStatus, PatchStatusRequest, RefundRequest,
    SetConfirmedTimeRequest, WebshopOrderSummary, count,
};

// The closures name the engine function at the call, which `check_command_parity.py` looks for.
#[allow(clippy::redundant_closure)]
#[uniffi::export]
impl PosEngine {
    /// The current HubRise connection, if any.
    pub async fn hubrise_get_connection_status(&self) -> Result<HubriseConnectionInfo, PosError> {
        let info = self
            .blocking(|state| {
                pos_engine::commands::hubrise::do_hubrise_get_connection_status(state)
            })
            .await?;
        convert::out(&info)
    }

    /// The HubRise page where the installer grants access, as a link and a QR code.
    pub async fn hubrise_start_oauth_flow(&self) -> Result<AuthorizeLink, PosError> {
        let link = self
            .blocking(|state| pos_engine::commands::hubrise::do_hubrise_start_oauth_flow(state))
            .await?;
        convert::out(&link)
    }

    /// Exchange the code HubRise showed for this location's token and start the integration.
    pub async fn hubrise_complete_oauth(
        &self,
        code: String,
    ) -> Result<HubriseConnectionInfo, PosError> {
        let info = self
            .spawn(move |engine| async move {
                pos_engine::commands::hubrise::do_hubrise_complete_oauth(&engine, &code).await
            })
            .await?;
        convert::out(&info)
    }

    /// Forget the HubRise connection (the menu stays as last synced).
    pub async fn hubrise_disconnect(&self) -> Result<(), PosError> {
        self.blocking(|state| pos_engine::commands::hubrise::do_hubrise_disconnect(state))
            .await
    }

    /// The online orders kept locally, newest first.
    pub async fn hubrise_list_webshop_orders(&self) -> Result<Vec<WebshopOrderSummary>, PosError> {
        let orders = self
            .blocking(|state| pos_engine::commands::hubrise::do_list_webshop_orders(state))
            .await?;
        convert::out(&orders)
    }

    /// Queue a status change for HubRise and show it at once on the POS.
    pub async fn hubrise_patch_order_status(
        &self,
        req: PatchStatusRequest,
    ) -> Result<(), PosError> {
        let req: pos_engine::commands::hubrise::PatchStatusRequest = convert::inp(&req)?;
        self.spawn(move |engine| async move {
            pos_engine::commands::hubrise::do_patch_order_status(
                &engine,
                &req.hubrise_order_id,
                &req.new_status,
            )
        })
        .await
    }

    /// Queue a new `confirmed_time` for HubRise and show it at once.
    pub async fn hubrise_set_confirmed_time(
        &self,
        req: SetConfirmedTimeRequest,
    ) -> Result<(), PosError> {
        let req: pos_engine::commands::hubrise::SetConfirmedTimeRequest = convert::inp(&req)?;
        self.spawn(move |engine| async move {
            pos_engine::commands::hubrise::do_set_confirmed_time(
                &engine,
                &req.hubrise_order_id,
                &req.confirmed_time,
            )
        })
        .await
    }

    /// What the status lights and the settings panel show.
    pub async fn hubrise_status(&self) -> Result<HubriseStatus, PosError> {
        let status = self
            .blocking(|state| pos_engine::commands::hubrise::do_hubrise_status(state))
            .await?;
        convert::out(&status)
    }

    /// Fetch and apply the HubRise catalog now; `None` when nothing was applied.
    pub async fn hubrise_sync_catalog(&self) -> Result<Option<ApplyOutcome>, PosError> {
        let outcome =
            self.spawn(|engine| async move {
                pos_engine::commands::hubrise::do_sync_catalog(&engine).await
            })
            .await?;
        convert::out(&outcome)
    }

    /// The local products that can keep their id when HubRise takes over the menu.
    pub async fn hubrise_adoption_preview(&self) -> Result<Vec<Adoption>, PosError> {
        let preview = self
            .blocking(|state| pos_engine::commands::hubrise::do_adoption_preview(state))
            .await?;
        convert::out(&preview)
    }

    /// The manager confirms the take-over: the chosen products keep their id and the menu mirrors
    /// HubRise.
    pub async fn hubrise_adopt_catalog(
        &self,
        adoptions: Vec<Adoption>,
    ) -> Result<ApplyOutcome, PosError> {
        let adoptions: Vec<pos_catalog::Adoption> = convert::inp(&adoptions)?;
        let outcome = self
            .blocking(move |state| {
                pos_engine::commands::hubrise::do_adopt_catalog(state, &adoptions)
            })
            .await?;
        convert::out(&outcome)
    }

    /// How each HubRise channel is signed.
    pub async fn hubrise_list_channels(&self) -> Result<Vec<Channel>, PosError> {
        let channels = self
            .blocking(|state| pos_engine::commands::hubrise::do_list_channels(state))
            .await?;
        convert::out(&channels)
    }

    /// Add or change a channel; pending orders resolve again. Answers how many of them now
    /// resolve cleanly.
    pub async fn hubrise_set_channel(&self, channel: Channel) -> Result<i64, PosError> {
        let channel: pos_hubrise::webshop::Channel = convert::inp(&channel)?;
        let resolved = self
            .blocking(move |state| pos_engine::commands::hubrise::do_set_channel(state, &channel))
            .await?;
        count(resolved)
    }

    /// Accept an online order: sign it (prepaid) or register it (unpaid) and tell HubRise.
    pub async fn webshop_accept_order(&self, req: AcceptRequest) -> Result<AcceptResult, PosError> {
        let req: pos_engine::commands::webshop::AcceptRequest = convert::inp(&req)?;
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::webshop::do_accept_webshop_order(
                    &engine,
                    req.order_id,
                    req.confirmed_time.as_deref(),
                )
                .await
            })
            .await?;
        convert::out(&result)
    }

    /// Collect an unpaid online order ("Encaisser").
    pub async fn webshop_handover_order(
        &self,
        req: HandoverRequest,
    ) -> Result<CheckoutResult, PosError> {
        let req: pos_engine::commands::webshop::HandoverRequest = convert::inp(&req)?;
        let result = self
            .spawn(move |engine| async move {
                let payments: &[EnginePaymentPart] = &req.payments;
                let delivery: Option<&EngineDeliveryRequest> = req.delivery.as_ref();
                pos_engine::commands::webshop::do_handover_webshop_order(
                    &engine,
                    req.order_id,
                    payments,
                    delivery,
                )
                .await
            })
            .await?;
        convert::out(&result)
    }

    /// Reject an online order (never signed).
    pub async fn webshop_reject_order(&self, order_id: i64) -> Result<(), PosError> {
        self.blocking(move |state| {
            pos_engine::commands::webshop::do_reject_webshop_order(state, order_id)
        })
        .await
    }

    /// Mark an online order completed.
    pub async fn webshop_complete_order(&self, order_id: i64) -> Result<(), PosError> {
        self.blocking(move |state| {
            pos_engine::commands::webshop::do_complete_webshop_order(state, order_id)
        })
        .await
    }

    /// Refund a signed online order HubRise cancelled (a manager confirms).
    pub async fn webshop_refund_order(
        &self,
        req: RefundRequest,
    ) -> Result<CheckoutResult, PosError> {
        let req: pos_engine::commands::webshop::RefundRequest = convert::inp(&req)?;
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::webshop::do_refund_webshop_order(
                    &engine,
                    req.order_id,
                    &req.employee_insz,
                    req.delivery.as_ref(),
                )
                .await
            })
            .await?;
        convert::out(&result)
    }

    /// Close a cancelled online order that was only registered (P) with corrections and an N at 0.
    pub async fn webshop_close_cancelled_order(
        &self,
        order_id: i64,
    ) -> Result<CheckoutResult, PosError> {
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::webshop::do_close_cancelled_webshop_order(&engine, order_id)
                    .await
            })
            .await?;
        convert::out(&result)
    }
}
