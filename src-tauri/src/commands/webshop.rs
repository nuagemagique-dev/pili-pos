//! Online orders signed by the FDM (feature `hubrise`).
//!
//! Every online order ends in an N event (SPF FAQ Q203):
//! - prepaid (paid in the platform's app or on the webshop): the sale
//!   N is signed when staff accept the order, payment type APP or
//!   ONLINE, and the VAT ticket is printed;
//! - unpaid (cash at the door or at collection): the order is
//!   registered (P) at acceptance, and the sale N is signed at handover
//!   with the same PLATFORM / WEBSHOP cost center, which closes the P
//!   (Q203 b).
//!
//! A rejected order is never signed. The event is created on this
//! terminal, in the name of the staff member who accepts, so the
//! deviceId stays the terminal's own.
//!
//! Acceptance claims the order (`pending` to `signing`) before calling
//! the FDM, so a double tap or a second terminal never signs twice;
//! if the FDM is unreachable or refuses, the order goes back to
//! `pending` and HubRise is told nothing. `accepted` is queued for
//! HubRise in the transaction that records the signature.

use pos_fiscal::event::build_order_input;
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::inputs::CostCenterInput;
use pos_fiscal::payment::{PaymentMethod, PaymentPart};
use pos_fiscal::types::{CostCenterType, EventLabel, PaymentType};
use pos_hubrise::webshop::{self, WebshopState};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;

use crate::commands::access::{self, Access};
use crate::commands::checkout::{CheckoutResult, Payment, SaleOptions, sell};
use crate::commands::delivery::TicketDeliveryRequest;
use crate::commands::errors;
use crate::commands::refund::{RefundLines, RefundSpec, refund_sale};
use crate::state::AppState;

/// What acceptance did.
#[derive(Debug, Serialize)]
pub struct AcceptResult {
    /// `signed_n` (prepaid) or `signed_p` (unpaid, to be collected).
    pub fiscal_state: String,
    pub fiscal_event_id: i64,
    /// The sale, for a prepaid order: its ticket and delivery.
    pub sale: Option<CheckoutResult>,
    /// Whether the kitchen ticket printed.
    pub kitchen_printed: bool,
}

#[derive(Debug, Deserialize)]
pub struct AcceptRequest {
    pub order_id: i64,
    /// The time the customer is told (RFC 3339), sent to HubRise with
    /// `accepted`.
    #[serde(default)]
    pub confirmed_time: Option<String>,
}

#[tauri::command]
pub async fn webshop_accept_order(
    state: State<'_, AppState>,
    req: AcceptRequest,
) -> Result<AcceptResult, String> {
    do_accept_webshop_order(state.inner(), req.order_id, req.confirmed_time.as_deref()).await
}

/// The order's cost center: PLATFORM for a platform's order, WEBSHOP
/// for the restaurant's own webshop; the id names the channel, the
/// reference is the HubRise order, shared by its P and its N.
fn cost_center(state: &WebshopState) -> CostCenterInput {
    let platform = state.channel_kind.as_deref() == Some("platform");
    CostCenterInput {
        id: state.channel.to_uppercase(),
        cost_center_type: if platform {
            CostCenterType::Platform
        } else {
            CostCenterType::Webshop
        },
        reference: state.hubrise_order_id.clone(),
        cost_center: None,
    }
}

fn load(state: &AppState, order_id: i64) -> Result<WebshopState, String> {
    let db = state.db.lock().map_err(errors::internal)?;
    webshop::load(&db, order_id)
        .map_err(errors::internal)?
        .ok_or_else(|| errors::hubrise::ORDER_UNKNOWN.to_string())
}

/// Move the order from `from` to `to`; false when it was not in
/// `from` (someone else got there first).
fn transition(state: &AppState, order_id: i64, from: &str, to: &str) -> Result<bool, String> {
    let db = state.db.lock().map_err(errors::internal)?;
    let n = db
        .execute(
            "UPDATE webshop_orders SET fiscal_state = ?3
             WHERE order_id = ?1 AND fiscal_state = ?2",
            params![order_id, from, to],
        )
        .map_err(errors::internal)?;
    Ok(n == 1)
}

/// Write the resolved lines on the order, once: a retry after a failed
/// signature reuses them (order lines are an append-only journal).
fn write_lines(state: &AppState, ws: &WebshopState) -> Result<(), String> {
    let db = state.db.lock().map_err(errors::internal)?;
    let written: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM order_lines WHERE order_id = ?1)",
            params![ws.order_id],
            |r| r.get(0),
        )
        .map_err(errors::internal)?;
    if written {
        return Ok(());
    }
    let tx = db.unchecked_transaction().map_err(errors::internal)?;
    for line in &ws.lines {
        let quantity = i32::try_from(line.quantity).map_err(errors::internal)?;
        let written = pos_db::queries::orders::add_order_line(
            &tx,
            pos_db::queries::orders::NewOrderLine {
                order_id: ws.order_id,
                menu_item_id: line.menu_item_id,
                quantity,
                unit_price_cents: line.unit_price_cents,
                vat_code: line.vat_code,
                name: &line.name,
                modifiers_json: None,
                department_id: &line.department_id,
                department_name: &line.department_name,
            },
        )
        .map_err(errors::internal)?;
        let discounts: Vec<_> = line
            .discounts
            .iter()
            .map(|d| pos_db::queries::price_changes::NewPriceChange {
                grouping_id: d.grouping_id,
                change_id: &d.id,
                name: &d.name,
                scope: "EVENT",
                change_type: "PUBLIC",
                amount_cents: d.amount_cents,
            })
            .collect();
        pos_db::queries::price_changes::add_to_line(&tx, written.id, &discounts)
            .map_err(errors::internal)?;
    }
    tx.commit().map_err(errors::internal)?;
    Ok(())
}

/// Record the signature and queue the HubRise status in one
/// transaction.
fn record_signed(
    state: &AppState,
    ws: &WebshopState,
    to: &str,
    event_id: i64,
    operator: &str,
    patch: serde_json::Value,
) -> Result<(), String> {
    let db = state.db.lock().map_err(errors::internal)?;
    let tx = db.unchecked_transaction().map_err(errors::internal)?;
    let event_column = if to == "signed_p" {
        "p_event_id"
    } else {
        "n_event_id"
    };
    tx.execute(
        &format!(
            "UPDATE webshop_orders SET fiscal_state = ?2, {event_column} = ?3,
                accepted_by = COALESCE(accepted_by, ?4),
                accepted_at = COALESCE(accepted_at, datetime('now'))
             WHERE order_id = ?1"
        ),
        params![ws.order_id, to, event_id, operator],
    )
    .map_err(errors::internal)?;
    tx.execute(
        "UPDATE orders SET is_fiscal = 1 WHERE id = ?1",
        params![ws.order_id],
    )
    .map_err(errors::internal)?;
    // HubRise cancelled the order while it was being signed: the
    // signature stands and waits for its refund or closing; HubRise is
    // not told anything it already refused.
    if webshop::cancelled_upstream(&tx, ws.order_id).map_err(errors::internal)? {
        webshop::follow_cancellation(&tx, ws.order_id).map_err(errors::internal)?;
        tx.commit().map_err(errors::internal)?;
        return Ok(());
    }
    pos_hubrise::outbox::enqueue_patch(&tx, ws.order_id, &ws.hubrise_order_id, &patch)
        .map_err(errors::internal)?;
    if let Some(status) = patch.get("status").and_then(|s| s.as_str()) {
        tx.execute(
            "UPDATE orders SET hubrise_status = ?2 WHERE id = ?1",
            params![ws.order_id, status],
        )
        .map_err(errors::internal)?;
    }
    tx.commit().map_err(errors::internal)?;
    state.hubrise_outbox_wake.notify_one();
    Ok(())
}

fn accepted_patch(confirmed_time: Option<&str>) -> serde_json::Value {
    match confirmed_time {
        Some(t) => json!({ "status": "accepted", "confirmed_time": t }),
        None => json!({ "status": "accepted" }),
    }
}

/// Accept an online order: sign it (N if prepaid, P if unpaid), print
/// the kitchen ticket, tell HubRise.
pub async fn do_accept_webshop_order(
    state: &AppState,
    order_id: i64,
    confirmed_time: Option<&str>,
) -> Result<AcceptResult, String> {
    crate::commands::integrations::require_hubrise(state)?;
    let operator = access::operator(state, Access::Cashier)?;
    if let Some(t) = confirmed_time
        && chrono::DateTime::parse_from_rfc3339(t).is_err()
    {
        return Err(errors::hubrise::TIME_INVALID.to_string());
    }
    let config = state.config.lock().map_err(errors::internal)?.clone();
    // Training events are never real sales: an online order is.
    if config.training_mode {
        return Err(errors::hubrise::TRAINING.to_string());
    }

    let ws = load(state, order_id)?;
    if ws.fiscal_state != "pending" {
        return Err(errors::hubrise::ALREADY_HANDLED.to_string());
    }
    if ws.resolution != "ok" {
        return Err(errors::hubrise::NEEDS_REVIEW.to_string());
    }
    if !transition(state, order_id, "pending", "signing")? {
        return Err(errors::hubrise::ALREADY_HANDLED.to_string());
    }

    let signed = match ws.payment_mode.as_deref() {
        Some("prepaid") => sign_prepaid(state, &operator, &ws).await,
        Some("unpaid") => sign_order_p(state, &config, &operator, &ws).await,
        _ => Err(errors::hubrise::NEEDS_REVIEW.to_string()),
    };
    let (to, event_id, sale) = match signed {
        Ok(done) => done,
        Err(e) => {
            // Nothing signed: the order waits again, HubRise hears
            // nothing.
            transition(state, order_id, "signing", "pending")?;
            return Err(e);
        }
    };
    record_signed(
        state,
        &ws,
        to,
        event_id,
        &operator.insz,
        accepted_patch(confirmed_time),
    )?;
    if let Some(t) = confirmed_time {
        mirror_confirmed_time(state, order_id, t)?;
    }

    let kitchen_printed = crate::commands::print::print_webshop_kitchen_ticket(state, order_id)
        .await
        .is_ok();
    Ok(AcceptResult {
        fiscal_state: to.to_string(),
        fiscal_event_id: event_id,
        sale,
        kitchen_printed,
    })
}

fn mirror_confirmed_time(state: &AppState, order_id: i64, time: &str) -> Result<(), String> {
    let db = state.db.lock().map_err(errors::internal)?;
    let raw: Option<String> = db
        .query_row(
            "SELECT raw_hubrise_json FROM orders WHERE id = ?1",
            params![order_id],
            |r| r.get(0),
        )
        .map_err(errors::internal)?;
    if let Some(mut value) = raw.and_then(|r| serde_json::from_str::<serde_json::Value>(&r).ok()) {
        value["confirmed_time"] = json!(time);
        db.execute(
            "UPDATE orders SET raw_hubrise_json = ?2 WHERE id = ?1",
            params![order_id, value.to_string()],
        )
        .map_err(errors::internal)?;
    }
    Ok(())
}

type Signed = (&'static str, i64, Option<CheckoutResult>);

/// A prepaid order: the sale N, paid APP or ONLINE for its total.
async fn sign_prepaid(
    state: &AppState,
    operator: &pos_core::user::User,
    ws: &WebshopState,
) -> Result<Signed, String> {
    let method = match ws.payment_type.as_deref() {
        Some("APP") => PaymentMethod::App,
        Some("ONLINE") => PaymentMethod::Online,
        _ => return Err(errors::hubrise::NEEDS_REVIEW.to_string()),
    };
    write_lines(state, ws)?;
    let total = pos_hubrise::webshop::net_total_cents(&ws.lines);
    let parts = [PaymentPart::of(method, total)];
    let result = sell(
        state,
        operator.clone(),
        ws.order_id,
        Payment::Parts(&parts),
        None,
        SaleOptions {
            cost_center: Some(cost_center(ws)),
            online: true,
            ..SaleOptions::default()
        },
    )
    .await?;
    match (result.success, result.fiscal_event_id) {
        (true, Some(id)) => Ok(("signed_n", id, Some(result))),
        _ => Err(result.error.unwrap_or_else(|| errors::internal("sale"))),
    }
}

/// An unpaid order: registered (P) with its cost center; the order
/// stays open until the money is collected.
async fn sign_order_p(
    state: &AppState,
    config: &pos_core::config::PosConfig,
    operator: &pos_core::user::User,
    ws: &WebshopState,
) -> Result<Signed, String> {
    write_lines(state, ws)?;
    let (mut order, entries, booking, changes) = {
        let db = state.db.lock().map_err(errors::internal)?;
        let order =
            pos_db::queries::orders::get_order(&db, ws.order_id).map_err(errors::internal)?;
        let entries = pos_db::queries::orders::get_order_entries(&db, ws.order_id, true)
            .map_err(errors::internal)?;
        let booking = super::booking(&db, config)?;
        let (changes, _) = super::price_changes::load(&db, ws.order_id)?;
        (order, entries, booking, changes)
    };
    let Some(last_entry) = entries.last() else {
        return Err(errors::ORDER_NO_LINES.to_string());
    };
    order.user_insz = operator.insz.clone();

    let label = EventLabel::P;
    let ticket_number = super::peek_ticket_number(state, label)?;
    let mut input = build_order_input(config, &order, &entries, ticket_number, &booking);
    input.cost_center = cost_center(ws);
    pos_fiscal::price_change::apply(&mut input.transaction, &entries, &changes);
    let request_json = serde_json::to_string(&input).map_err(|e| e.to_string())?;
    let sign_result = FdmClient::from_config(config)
        .sign_order(input, false)
        .await
        .map_err(|e| format!("FDM error: {e}"))?;
    let event_id = super::commit_signed_event(
        state,
        config,
        super::SignedEvent {
            label,
            ticket_number,
            order_id: Some(ws.order_id),
            user_insz: &order.user_insz,
            request_json: &request_json,
            sign_result: &sign_result,
            sent_entries: Some((ws.order_id, last_entry.id)),
        },
    )?;
    Ok(("signed_p", event_id, None))
}

#[derive(Debug, Deserialize)]
pub struct HandoverRequest {
    pub order_id: i64,
    /// What the customer paid, as at the till (cash, card, meal
    /// vouchers).
    pub payments: Vec<PaymentPart>,
    #[serde(default)]
    pub delivery: Option<TicketDeliveryRequest>,
}

#[tauri::command]
pub async fn webshop_handover_order(
    state: State<'_, AppState>,
    req: HandoverRequest,
) -> Result<CheckoutResult, String> {
    do_handover_webshop_order(
        state.inner(),
        req.order_id,
        &req.payments,
        req.delivery.as_ref(),
    )
    .await
}

/// Collect an unpaid online order ("Encaisser"): the sale N, with the
/// order's cost center, closes its P; HubRise hears `completed`.
pub async fn do_handover_webshop_order(
    state: &AppState,
    order_id: i64,
    parts: &[PaymentPart],
    delivery: Option<&TicketDeliveryRequest>,
) -> Result<CheckoutResult, String> {
    let cashier = match access::operator(state, Access::Cashier) {
        Ok(user) => user,
        Err(e) => return Ok(CheckoutResult::refused(e)),
    };
    if parts.iter().any(|p| !p.method.is_offered_at_till()) {
        return Ok(CheckoutResult::refused(
            errors::hubrise::PAYMENT_NOT_AT_TILL,
        ));
    }
    let ws = load(state, order_id)?;
    if ws.fiscal_state != "signed_p" {
        return Ok(CheckoutResult::refused(
            errors::hubrise::NOT_AWAITING_PAYMENT,
        ));
    }
    // The same claim as acceptance, for the second half.
    if !transition(state, order_id, "signed_p", "signing")? {
        return Ok(CheckoutResult::refused(errors::hubrise::ALREADY_HANDLED));
    }
    let result = sell(
        state,
        cashier.clone(),
        order_id,
        Payment::Parts(parts),
        delivery,
        SaleOptions {
            cost_center: Some(cost_center(&ws)),
            online: true,
            ..SaleOptions::default()
        },
    )
    .await;
    match result {
        Ok(r) if r.success => {
            let event_id = r.fiscal_event_id.ok_or_else(|| errors::internal("sale"))?;
            record_signed(
                state,
                &ws,
                "signed_n",
                event_id,
                &cashier.insz,
                json!({ "status": "completed" }),
            )?;
            Ok(r)
        }
        other => {
            transition(state, order_id, "signing", "signed_p")?;
            other
        }
    }
}

#[tauri::command]
pub fn webshop_reject_order(state: State<'_, AppState>, order_id: i64) -> Result<(), String> {
    do_reject_webshop_order(state.inner(), order_id)
}

/// Refuse an order not yet accepted: voided here, `rejected` for
/// HubRise, nothing signed.
pub fn do_reject_webshop_order(state: &AppState, order_id: i64) -> Result<(), String> {
    crate::commands::integrations::require_hubrise(state)?;
    access::operator(state, Access::Cashier)?;
    let ws = load(state, order_id)?;
    let db = state.db.lock().map_err(errors::internal)?;
    let tx = db.unchecked_transaction().map_err(errors::internal)?;
    let n = tx
        .execute(
            "UPDATE webshop_orders SET fiscal_state = 'rejected'
             WHERE order_id = ?1 AND fiscal_state = 'pending'",
            params![order_id],
        )
        .map_err(errors::internal)?;
    if n == 0 {
        return Err(errors::hubrise::ALREADY_HANDLED.to_string());
    }
    tx.execute(
        "UPDATE orders SET status = 'voided', closed_at = datetime('now'),
            hubrise_status = 'rejected'
         WHERE id = ?1 AND status = 'open'",
        params![order_id],
    )
    .map_err(errors::internal)?;
    pos_hubrise::outbox::enqueue_patch(
        &tx,
        order_id,
        &ws.hubrise_order_id,
        &json!({ "status": "rejected" }),
    )
    .map_err(errors::internal)?;
    tx.commit().map_err(errors::internal)?;
    state.hubrise_outbox_wake.notify_one();
    Ok(())
}

#[tauri::command]
pub fn webshop_complete_order(state: State<'_, AppState>, order_id: i64) -> Result<(), String> {
    do_complete_webshop_order(state.inner(), order_id)
}

/// Hand over a prepaid order whose sale is signed: HubRise hears
/// `completed`. An unpaid order is completed by its handover sale.
pub fn do_complete_webshop_order(state: &AppState, order_id: i64) -> Result<(), String> {
    access::operator(state, Access::Cashier)?;
    let ws = load(state, order_id)?;
    if ws.fiscal_state != "signed_n" || ws.payment_mode.as_deref() != Some("prepaid") {
        return Err(errors::hubrise::NOT_AWAITING_PAYMENT.to_string());
    }
    let db = state.db.lock().map_err(errors::internal)?;
    let tx = db.unchecked_transaction().map_err(errors::internal)?;
    let n = tx
        .execute(
            "UPDATE orders SET hubrise_status = 'completed', status = 'closed',
                closed_at = COALESCE(closed_at, datetime('now'))
             WHERE id = ?1 AND hubrise_status IS NOT 'completed'",
            params![order_id],
        )
        .map_err(errors::internal)?;
    if n == 0 {
        return Err(errors::hubrise::ALREADY_HANDLED.to_string());
    }
    pos_hubrise::outbox::enqueue_patch(
        &tx,
        order_id,
        &ws.hubrise_order_id,
        &json!({ "status": "completed" }),
    )
    .map_err(errors::internal)?;
    tx.commit().map_err(errors::internal)?;
    state.hubrise_outbox_wake.notify_one();
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct RefundRequest {
    pub order_id: i64,
    /// The manager confirming the refund.
    pub employee_insz: String,
    #[serde(default)]
    pub delivery: Option<TicketDeliveryRequest>,
}

#[tauri::command]
pub async fn webshop_refund_order(
    state: State<'_, AppState>,
    req: RefundRequest,
) -> Result<CheckoutResult, String> {
    do_refund_webshop_order(
        state.inner(),
        req.order_id,
        &req.employee_insz,
        req.delivery.as_ref(),
    )
    .await
}

/// The sale an order's refund cites, and whether it is still to be
/// refunded or closed.
fn signed_events(state: &AppState, order_id: i64) -> Result<(Option<i64>, Option<i64>), String> {
    let db = state.db.lock().map_err(errors::internal)?;
    db.query_row(
        "SELECT n_event_id, p_event_id FROM webshop_orders WHERE order_id = ?1",
        params![order_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map_err(errors::internal)
}

/// Refund an order HubRise cancelled after its sale was signed: every
/// line at its price, charges included, citing the sale. A manager
/// confirms it; nothing refunds on its own. The platform or the payment
/// provider pays a prepaid order back (APP / ONLINE, the drawer stays
/// shut); an order paid at the till is refunded in cash.
pub async fn do_refund_webshop_order(
    state: &AppState,
    order_id: i64,
    employee_insz: &str,
    delivery: Option<&TicketDeliveryRequest>,
) -> Result<CheckoutResult, String> {
    if let Err(e) = access::operator_named(state, employee_insz, Access::Admin) {
        return Ok(CheckoutResult::refused(e));
    }
    if state.config.lock().map_err(errors::internal)?.training_mode {
        return Ok(CheckoutResult::refused(errors::hubrise::TRAINING));
    }
    let ws = load(state, order_id)?;
    let (Some(sale), _) = signed_events(state, order_id)? else {
        return Ok(CheckoutResult::refused(errors::hubrise::NOT_CANCELLED));
    };
    if ws.fiscal_state != "refund_pending" {
        return Ok(CheckoutResult::refused(errors::hubrise::NOT_CANCELLED));
    }
    if !transition(state, order_id, "refund_pending", "signing")? {
        return Ok(CheckoutResult::refused(errors::hubrise::ALREADY_HANDLED));
    }
    let spec = match ws.payment_type.as_deref() {
        Some(t) if ws.payment_mode.as_deref() == Some("prepaid") => RefundSpec {
            lines: RefundLines::Everything,
            payment_type: if t == "APP" {
                PaymentType::App
            } else {
                PaymentType::Online
            },
            open_drawer: false,
        },
        _ => RefundSpec::cash(RefundLines::Everything),
    };
    let result = refund_sale(state, sale, employee_insz, delivery, spec).await;
    match result {
        Ok(r) if r.success => {
            let db = state.db.lock().map_err(errors::internal)?;
            db.execute(
                "UPDATE webshop_orders SET fiscal_state = 'refunded', refund_event_id = ?2
                 WHERE order_id = ?1",
                params![order_id, r.fiscal_event_id],
            )
            .map_err(errors::internal)?;
            Ok(r)
        }
        other => {
            transition(state, order_id, "signing", "refund_pending")?;
            other
        }
    }
}

#[tauri::command]
pub async fn webshop_close_cancelled_order(
    state: State<'_, AppState>,
    order_id: i64,
) -> Result<CheckoutResult, String> {
    do_close_cancelled_webshop_order(state.inner(), order_id).await
}

/// Close an unpaid order HubRise cancelled after its P: every line is
/// corrected, and the sale N at 0,00 EUR (payment 0, FAQ Q206 / Q209)
/// closes the P with the same cost center. Its ticket is printed.
pub async fn do_close_cancelled_webshop_order(
    state: &AppState,
    order_id: i64,
) -> Result<CheckoutResult, String> {
    let cashier = match access::operator(state, Access::Cashier) {
        Ok(user) => user,
        Err(e) => return Ok(CheckoutResult::refused(e)),
    };
    if state.config.lock().map_err(errors::internal)?.training_mode {
        return Ok(CheckoutResult::refused(errors::hubrise::TRAINING));
    }
    let ws = load(state, order_id)?;
    if ws.fiscal_state != "cancel_pending" {
        return Ok(CheckoutResult::refused(errors::hubrise::NOT_CANCELLED));
    }
    if !transition(state, order_id, "cancel_pending", "signing")? {
        return Ok(CheckoutResult::refused(errors::hubrise::ALREADY_HANDLED));
    }
    let closed = async {
        {
            // Corrections, journalled once: a retry finds no line left.
            let db = state.db.lock().map_err(errors::internal)?;
            let tx = db.unchecked_transaction().map_err(errors::internal)?;
            let lines = pos_db::queries::orders::get_order_lines(&tx, order_id)
                .map_err(errors::internal)?;
            for line in lines {
                pos_db::queries::orders::remove_order_line(&tx, line.id)
                    .map_err(errors::internal)?;
            }
            tx.commit().map_err(errors::internal)?;
        }
        sell(
            state,
            cashier,
            order_id,
            Payment::Parts(&[]),
            None,
            SaleOptions {
                cost_center: Some(cost_center(&ws)),
                online: true,
                closes_cancelled: true,
            },
        )
        .await
    }
    .await;
    match closed {
        Ok(r) if r.success => {
            let db = state.db.lock().map_err(errors::internal)?;
            db.execute(
                "UPDATE webshop_orders SET fiscal_state = 'closed', n_event_id = ?2
                 WHERE order_id = ?1",
                params![order_id, r.fiscal_event_id],
            )
            .map_err(errors::internal)?;
            Ok(r)
        }
        other => {
            transition(state, order_id, "signing", "cancel_pending")?;
            other
        }
    }
}

/// Boot recovery for orders a crash left in `signing`: signed if the
/// archive holds its event (and the missing HubRise status is queued),
/// otherwise back to where it was. A refund or a closing interrupted
/// the same way is finished or offered again. Returns how many were
/// repaired.
pub fn recover_webshop_orders(conn: &rusqlite::Connection) -> rusqlite::Result<usize> {
    type Stuck = (i64, String, Option<i64>, Option<i64>);
    let stuck: Vec<Stuck> = {
        let mut stmt = conn.prepare(
            "SELECT order_id, hubrise_order_id, p_event_id, n_event_id FROM webshop_orders
             WHERE fiscal_state = 'signing'",
        )?;
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<rusqlite::Result<_>>()?
    };
    let to_sql =
        |e: pos_hubrise::errors::HubriseError| rusqlite::Error::ToSqlConversionFailure(Box::new(e));
    let mut repaired = 0;
    for (order_id, hubrise_order_id, p_event_id, n_event_id) in stuck {
        let tx = conn.unchecked_transaction()?;
        let cancelled = webshop::cancelled_upstream(&tx, order_id).map_err(to_sql)?;
        let set = |state: &str, column: Option<&str>, id: Option<i64>| -> rusqlite::Result<()> {
            match (column, id) {
                (Some(column), Some(id)) => tx.execute(
                    &format!(
                        "UPDATE webshop_orders SET fiscal_state = ?2, {column} = ?3,
                            accepted_at = COALESCE(accepted_at, datetime('now'))
                         WHERE order_id = ?1"
                    ),
                    params![order_id, state, id],
                )?,
                _ => tx.execute(
                    "UPDATE webshop_orders SET fiscal_state = ?2 WHERE order_id = ?1",
                    params![order_id, state],
                )?,
            };
            Ok(())
        };
        let event = |label: &str| -> rusqlite::Result<Option<i64>> {
            tx.query_row(
                "SELECT id FROM fiscal_events WHERE order_id = ?1 AND event_type = ?2
                 ORDER BY id DESC LIMIT 1",
                params![order_id, label],
                |r| r.get(0),
            )
            .optional()
        };

        // A refund of the signed sale, interrupted.
        if cancelled && let Some(sale) = n_event_id {
            match refund_citing(&tx, sale)? {
                Some(refund) => set("refunded", Some("refund_event_id"), Some(refund))?,
                None => set("refund_pending", None, None)?,
            }
            tx.commit()?;
            repaired += 1;
            continue;
        }

        let n = event("N")?;
        let p = event("P")?;
        // The closing of a cancelled P, interrupted: its sale at 0,00
        // EUR has no line left.
        if cancelled && p_event_id.is_some() {
            let lines_left: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM order_lines WHERE order_id = ?1)",
                params![order_id],
                |r| r.get(0),
            )?;
            match n {
                Some(n) if !lines_left => set("closed", Some("n_event_id"), Some(n))?,
                Some(n) => {
                    set("signed_n", Some("n_event_id"), Some(n))?;
                    webshop::follow_cancellation(&tx, order_id).map_err(to_sql)?;
                }
                None => set("cancel_pending", None, None)?,
            }
            tx.commit()?;
            repaired += 1;
            continue;
        }

        let (to, event_id, status) = match (n, p_event_id, p) {
            // Handover of an unpaid order signed: completed.
            (Some(n), Some(_), _) => ("signed_n", Some(n), Some("completed")),
            // Prepaid acceptance signed.
            (Some(n), None, _) => ("signed_n", Some(n), Some("accepted")),
            // Handover interrupted before its sale: still to collect.
            (None, Some(_), _) => ("signed_p", None, None),
            // Unpaid acceptance signed.
            (None, None, Some(p)) => ("signed_p", Some(p), Some("accepted")),
            (None, None, None) => ("pending", None, None),
        };
        let column = if to == "signed_p" {
            "p_event_id"
        } else {
            "n_event_id"
        };
        set(to, Some(column), event_id)?;
        if event_id.is_some() {
            tx.execute(
                "UPDATE orders SET is_fiscal = 1 WHERE id = ?1",
                params![order_id],
            )?;
        }
        if cancelled {
            // Signed while HubRise cancelled it: nothing to tell HubRise.
            webshop::follow_cancellation(&tx, order_id).map_err(to_sql)?;
        } else if let Some(status) = status {
            tx.execute(
                "UPDATE orders SET hubrise_status = ?2 WHERE id = ?1",
                params![order_id, status],
            )?;
            let queued: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM hubrise_outbox WHERE hubrise_order_id = ?1
                 AND body_json LIKE ?2)",
                params![hubrise_order_id, format!("%\"{status}\"%")],
                |r| r.get(0),
            )?;
            if !queued {
                pos_hubrise::outbox::enqueue_patch(
                    &tx,
                    order_id,
                    &hubrise_order_id,
                    &json!({ "status": status }),
                )
                .map_err(to_sql)?;
            }
        }
        tx.commit()?;
        repaired += 1;
    }
    Ok(repaired)
}

/// The refund archived against the sale `sale`: the N whose `fdmRef`
/// is that sale's FDM identity.
fn refund_citing(conn: &rusqlite::Connection, sale: i64) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        "SELECT r.id FROM fiscal_events r, fiscal_events o
         WHERE o.id = ?1 AND r.id > o.id
           AND json_extract(r.event_data_json, '$.fdmRef.fdmId') = o.fdm_id
           AND json_extract(r.event_data_json, '$.fdmRef.fdmDateTime') = o.fdm_datetime
           AND json_extract(r.event_data_json, '$.fdmRef.eventLabel') = o.event_label
           AND json_extract(r.event_data_json, '$.fdmRef.eventCounter')
               = json_extract(o.fdm_response_json, '$.fdmRef.eventCounter')
           AND json_extract(r.event_data_json, '$.fdmRef.totalCounter')
               = json_extract(o.fdm_response_json, '$.fdmRef.totalCounter')
         ORDER BY r.id LIMIT 1",
        params![sale],
        |r| r.get(0),
    )
    .optional()
}
