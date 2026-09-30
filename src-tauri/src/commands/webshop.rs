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
use pos_fiscal::types::{CostCenterType, EventLabel};
use pos_hubrise::webshop::{self, WebshopState};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;

use crate::commands::access::{self, Access};
use crate::commands::checkout::{CheckoutResult, Payment, SaleOptions, sell};
use crate::commands::delivery::TicketDeliveryRequest;
use crate::commands::errors;
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
        pos_db::queries::orders::add_order_line(
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
    let total: i64 = ws.lines.iter().map(|l| l.total_cents()).sum();
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
    let (mut order, entries, booking) = {
        let db = state.db.lock().map_err(errors::internal)?;
        let order =
            pos_db::queries::orders::get_order(&db, ws.order_id).map_err(errors::internal)?;
        let entries = pos_db::queries::orders::get_order_entries(&db, ws.order_id, true)
            .map_err(errors::internal)?;
        let booking = super::booking(&db, config)?;
        (order, entries, booking)
    };
    let Some(last_entry) = entries.last() else {
        return Err(errors::ORDER_NO_LINES.to_string());
    };
    order.user_insz = operator.insz.clone();

    let label = EventLabel::P;
    let ticket_number = super::peek_ticket_number(state, label)?;
    let mut input = build_order_input(config, &order, &entries, ticket_number, &booking);
    input.cost_center = cost_center(ws);
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

/// Boot recovery for orders a crash left in `signing`: signed if the
/// archive holds its event (and the missing HubRise status is queued),
/// otherwise back to where it was. Returns how many were repaired.
pub fn recover_webshop_orders(conn: &rusqlite::Connection) -> rusqlite::Result<usize> {
    let stuck: Vec<(i64, String, Option<i64>)> = {
        let mut stmt = conn.prepare(
            "SELECT order_id, hubrise_order_id, p_event_id FROM webshop_orders
             WHERE fiscal_state = 'signing'",
        )?;
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?
    };
    let mut repaired = 0;
    for (order_id, hubrise_order_id, p_event_id) in stuck {
        let tx = conn.unchecked_transaction()?;
        let event = |label: &str| -> rusqlite::Result<Option<i64>> {
            tx.query_row(
                "SELECT id FROM fiscal_events WHERE order_id = ?1 AND event_type = ?2
                 ORDER BY id DESC LIMIT 1",
                params![order_id, label],
                |r| r.get(0),
            )
            .optional()
        };
        let n = event("N")?;
        let p = event("P")?;
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
        match event_id {
            Some(id) => tx.execute(
                &format!(
                    "UPDATE webshop_orders SET fiscal_state = ?2, {column} = ?3,
                        accepted_at = COALESCE(accepted_at, datetime('now'))
                     WHERE order_id = ?1"
                ),
                params![order_id, to, id],
            )?,
            None => tx.execute(
                "UPDATE webshop_orders SET fiscal_state = ?2 WHERE order_id = ?1",
                params![order_id, to],
            )?,
        };
        if let Some(status) = status {
            tx.execute(
                "UPDATE orders SET is_fiscal = 1, hubrise_status = ?2 WHERE id = ?1",
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
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            }
        }
        tx.commit()?;
        repaired += 1;
    }
    Ok(repaired)
}
