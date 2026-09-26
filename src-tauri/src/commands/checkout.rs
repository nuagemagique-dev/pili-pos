use serde::{Deserialize, Serialize};
use tauri::State;

use pos_fiscal::archive::{self, ArchiveParams};
use pos_fiscal::counter::FiscalCounter;
use pos_fiscal::event::build_sale_input;
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::response::SignResult;
use pos_fiscal::ticket::{self, FiscalTicket, TicketLine, TicketParams, TicketPayment};
use pos_fiscal::types::{EventLabel, PaymentType};

use crate::commands::access::{self, Access};
use crate::commands::delivery::{self, DeliveryOutcome, TicketDeliveryRequest};
use crate::commands::errors;
#[cfg(feature = "hubrise")]
use crate::commands::hubrise_push;
use crate::state::AppState;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutPaymentMethod {
    Cash,
    Card,
}

#[derive(Debug, Serialize)]
pub struct CheckoutResult {
    pub success: bool,
    pub ticket: Option<FiscalTicket>,
    pub error: Option<String>,
    pub fdm_connected: bool,
    /// Archive id of the signed event.
    pub fiscal_event_id: Option<i64>,
    /// How the ticket reached the customer (paper and/or email).
    pub delivery: Option<DeliveryOutcome>,
}

impl CheckoutResult {
    /// A checkout refused before anything was signed.
    pub fn refused(error: impl Into<String>) -> Self {
        Self {
            success: false,
            ticket: None,
            error: Some(error.into()),
            fdm_connected: false,
            fiscal_event_id: None,
            delivery: None,
        }
    }
}

#[tauri::command]
pub async fn checkout(
    state: State<'_, AppState>,
    order_id: i64,
    payment_method: CheckoutPaymentMethod,
    delivery: Option<TicketDeliveryRequest>,
) -> Result<CheckoutResult, String> {
    do_checkout_with(state.inner(), order_id, payment_method, delivery.as_ref()).await
}

/// Inner implementation of [`checkout`], decoupled from Tauri's
/// `State` wrapper so integration tests can drive it directly. The
/// wrapper above is the production entry point; everything below is
/// the same code, just reachable from `cargo test`.
pub async fn do_checkout(
    state: &AppState,
    order_id: i64,
    payment_method: CheckoutPaymentMethod,
) -> Result<CheckoutResult, String> {
    do_checkout_with(state, order_id, payment_method, None).await
}

/// [`do_checkout`] with the operator's ticket choice (paper, email or
/// both). The medium is part of the signed request, so it is settled
/// before the FDM call.
pub async fn do_checkout_with(
    state: &AppState,
    order_id: i64,
    payment_method: CheckoutPaymentMethod,
    delivery_request: Option<&TicketDeliveryRequest>,
) -> Result<CheckoutResult, String> {
    // A sale is signed in the name of the cashier taking the payment.
    let cashier = match access::operator(state, Access::Cashier) {
        Ok(user) => user,
        Err(e) => return Ok(CheckoutResult::refused(e)),
    };
    // 1. Read order + lines from DB
    // `lines` is the consolidated order (printed); `entries` the journal
    // of every addition and correction, sent to the FDM (§2).
    let (order, lines, entries, config, booking_period_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let order = pos_db::queries::orders::get_order(&db, order_id).map_err(|e| e.to_string())?;
        let lines =
            pos_db::queries::orders::get_order_lines(&db, order_id).map_err(|e| e.to_string())?;
        let entries = pos_db::queries::orders::get_order_entries(&db, order_id, false)
            .map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = super::booking(&db)?;
        (order, lines, entries, config, bp)
    };
    let mut order = order;
    order.user_insz = cashier.insz;

    if lines.is_empty() {
        return Ok(CheckoutResult {
            success: false,
            ticket: None,
            error: Some(errors::ORDER_NO_LINES.into()),
            fdm_connected: false,
            fiscal_event_id: None,
            delivery: None,
        });
    }

    // Guard: no negative sale (a refund goes through `refund.rs`). A
    // sale that comes to 0,00 EUR is signed like any other: event N
    // and a VAT ticket (producers FAQ Q206, Q209). Together with the
    // empty-order check above, no record without a product line can
    // be made (manual §III.D).
    if order.total_cents < 0 {
        return Ok(CheckoutResult {
            success: false,
            ticket: None,
            error: Some(errors::ORDER_TOTAL_NEGATIVE.into()),
            fdm_connected: false,
            fiscal_event_id: None,
            delivery: None,
        });
    }

    // Guard: reject if order is already closed (prevents double
    // fiscal events when the operator taps "Payer" twice before
    // the Vue loading guard kicks in).
    if order.status != pos_core::order::OrderStatus::Open {
        return Ok(CheckoutResult {
            success: false,
            ticket: None,
            error: Some(errors::ORDER_ALREADY_CLOSED.into()),
            fdm_connected: false,
            fiscal_event_id: None,
            delivery: None,
        });
    }

    // Ticket medium: refused choices stop here, before any signing.
    let resolved = match delivery::resolve(delivery_request, &config, &state.mail) {
        Ok(r) => r,
        Err(e) => return Ok(CheckoutResult::refused(e)),
    };

    // 2. RESERVE (peek, don't commit) the next sequence number.
    //
    // We deliberately do NOT call `FiscalCounter::next` here yet.
    // Allocating before the FDM round-trip would create a gap in
    // the sequence whenever the FDM call fails - a direct violation
    // of the SCE 2.0 "strictly sequential, no gaps" requirement.
    //
    // Instead we peek (read-only) to obtain the value we'll send to
    // the FDM, then commit the increment inside the same SQLite
    // transaction that writes the archive (step 6-8 below). If the
    // FDM fails, the counter stays untouched and the next attempt
    // sees the same value - combined with the FDM's idempotency on
    // `posFiscalTicketNo`, this is safe to retry indefinitely.
    let ticket_number = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let label = if config.training_mode {
            EventLabel::T
        } else {
            EventLabel::N
        };
        FiscalCounter::peek_next(&db, label).map_err(|e| e.to_string())?
    };

    // A sale at 0,00 EUR moves no money: it is registered as a cash
    // payment of 0 (FAQ Q206) and opens no drawer.
    let payment_method = if order.total_cents == 0 {
        CheckoutPaymentMethod::Cash
    } else {
        payment_method
    };

    // 3. Belgian 5-cent rounding (cash only, per Belgian law).
    let rounding_cents: i64 = match payment_method {
        CheckoutPaymentMethod::Cash => ticket::round_to_5_cents(order.total_cents),
        CheckoutPaymentMethod::Card => 0,
    };
    let payment_total_cents = order.total_cents + rounding_cents;

    // 4. Build the SaleInput
    let payment_type = match payment_method {
        CheckoutPaymentMethod::Cash => PaymentType::Cash,
        CheckoutPaymentMethod::Card => PaymentType::CardDebit,
    };

    let mut sale_input = build_sale_input(
        &config,
        &order,
        &entries,
        ticket_number,
        &booking_period_id,
        payment_type,
        order.total_cents,
        rounding_cents,
        None, // no fdm_ref for normal sales
    );
    sale_input.common.ticket_medium = resolved.medium;

    // 4. Serialize request for archival
    let request_json = serde_json::to_string(&sale_input).map_err(|e| e.to_string())?;

    // 5. Send to FDM
    let fdm = FdmClient::new(&config.fdm_url);
    let fdm_result: Result<SignResult, _> = fdm.sign_sale(sale_input, config.training_mode).await;

    let (sign_result, fdm_connected) = match fdm_result {
        Ok(result) => (Some(result), true),
        Err(pos_fiscal::fdm_client::FdmError::NotConnected) => {
            // No FDM, no sale: the POS blocks (SCE 2.0). There is no
            // offline mode, in any build.
            return Ok(CheckoutResult {
                success: false,
                ticket: None,
                error: Some("FDM not connected - cannot complete transaction".into()),
                fdm_connected: false,
                fiscal_event_id: None,
                delivery: None,
            });
        }
        Err(e) => {
            return Ok(CheckoutResult {
                success: false,
                ticket: None,
                error: Some(format!("FDM error: {e}")),
                fdm_connected: true,
                fiscal_event_id: None,
                delivery: None,
            });
        }
    };

    // 6. Build the fiscal ticket (display, print, email) before the
    // commit: its email is queued in the same transaction.
    let operator = super::operator_name(state, &order.user_insz);
    let ticket = sign_result.as_ref().map(|result| {
        let ticket_lines: Vec<TicketLine> = lines
            .iter()
            .map(|l| TicketLine {
                quantity: l.quantity as f64,
                description: l.name.clone(),
                unit_price: l.unit_price_cents as f64 / 100.0,
                total: (l.unit_price_cents * l.quantity as i64) as f64 / 100.0,
                vat_code: l.vat_code.as_str().to_string(),
            })
            .collect();

        let payments = vec![TicketPayment {
            method: match payment_method {
                CheckoutPaymentMethod::Cash => "Cash".to_string(),
                CheckoutPaymentMethod::Card => "Card".to_string(),
            },
            amount: payment_total_cents as f64 / 100.0,
        }];

        let rounding_eur = if rounding_cents != 0 {
            Some(rounding_cents as f64 / 100.0)
        } else {
            None
        };

        FiscalTicket::from_sign_result(TicketParams {
            result,
            restaurant_name: &config.restaurant_name,
            restaurant_address: &config.restaurant_address,
            restaurant_vat: &config.restaurant_vat,
            restaurant_est_no: &config.restaurant_est_no,
            lines: ticket_lines,
            payments,
            total: order.total_cents as f64 / 100.0,
            is_training: config.training_mode,
            rounding: rounding_eur,
            lang: pos_fiscal::ticket::ReceiptLanguage::from_code(&config.language),
            operator: &operator,
        })
    });

    // 6-8. COMMIT counter + archive fiscal event + close order
    // ATOMICALLY in a single SQLite transaction.
    //
    // Now that the FDM has returned (or returned a non-fatal error
    // we've decided to record), we commit the counter increment via
    // `FiscalCounter::next` inside the same tx that archives the
    // event. Three rows of state are kept consistent:
    //   - `fiscal_counters.current_value` advances by 1
    //   - `fiscal_events` gets the new row
    //   - `orders.status` flips to closed
    // If any of the three fails the whole tx rolls back, including
    // the counter advance - so no gap is ever observable.
    let (event_id, delivery_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;

        let label = if config.training_mode {
            EventLabel::T
        } else {
            EventLabel::N
        };
        let committed_no = FiscalCounter::next(&tx, label).map_err(|e| e.to_string())?;
        // Graceful error, not a panic: see the identical comment in
        // `commands/mod.rs::commit_signed_event` - a panic here would
        // unwind while the `state.db` MutexGuard is still held and
        // poison it for the whole process. Concurrent checkouts of the
        // same order are exactly the scenario this guards against; the
        // transaction rolls back, so the loser sees a clean error and
        // can retry.
        if committed_no != ticket_number {
            return Err(errors::concurrent(format_args!(
                "checkout {label:?}: peeked {ticket_number}, committed {committed_no}"
            )));
        }

        let event_id = archive::archive_event(
            &tx,
            ArchiveParams {
                event_type: label,
                sequence_number: ticket_number,
                order_id: Some(order_id),
                pos_id: &config.pos_id,
                terminal_id: &config.terminal_id,
                user_insz: &order.user_insz,
                request_json: &request_json,
                sign_result: sign_result.as_ref(),
            },
        )
        .map_err(|e| e.to_string())?;
        pos_db::queries::orders::mark_order_entries_sent(&tx, order_id, i64::MAX, event_id)
            .map_err(|e| e.to_string())?;

        tx.execute(
            "UPDATE orders SET status = 'closed', closed_at = datetime('now') WHERE id = ?1",
            rusqlite::params![order_id],
        )
        .map_err(|e| e.to_string())?;

        // Record the payment in the order's payment history. (The X/Z
        // reports total the financials of the signed events instead.)
        // Training-mode "sales" move no real money, so they are not
        // recorded here (mirrors the FDM's T-vs-N counter split).
        if label != EventLabel::T {
            let method = match payment_method {
                CheckoutPaymentMethod::Cash => "cash",
                CheckoutPaymentMethod::Card => "card",
            };
            tx.execute(
                "INSERT INTO payments (order_id, method, amount_cents) VALUES (?1, ?2, ?3)",
                rusqlite::params![order_id, method, payment_total_cents],
            )
            .map_err(|e| e.to_string())?;
        }

        let delivery_id = delivery::enqueue_in_tx(&tx, event_id, &resolved, ticket.as_ref())?;

        tx.commit().map_err(|e| e.to_string())?;
        (event_id, delivery_id)
    };

    // 9. Print and/or email the ticket, then open the drawer for cash.
    let delivery_outcome = delivery::deliver(
        state,
        &config,
        event_id,
        &resolved,
        delivery_id,
        ticket.as_ref(),
    )
    .await;
    if matches!(payment_method, CheckoutPaymentMethod::Cash)
        && order.total_cents != 0
        && let Some(ref addr) = config.printer_address
    {
        let drawer_data = pos_print::drawer::kick_drawer_bytes();
        let _ = pos_print::transport::send_to_printer(addr, &drawer_data).await;
    }

    // 10. Enqueue the completed order for async push to HubRise.
    // This is a pure SQLite write - zero network, zero latency, never
    // blocked by a HubRise or internet outage. The background outbox
    // worker (started in main.rs alongside the poller) drains the
    // queue when connectivity is available, with exponential backoff
    // on transient failures. Gated behind the `hubrise` feature so the
    // certification build (no HubRise) skips this entirely.
    #[cfg(feature = "hubrise")]
    {
        let payment_name = match payment_method {
            CheckoutPaymentMethod::Cash => "Cash",
            CheckoutPaymentMethod::Card => "Card",
        };
        let body = hubrise_push::build_hubrise_order_body(&order, &lines, payment_name);
        let body_json = serde_json::to_string(&body).unwrap_or_default();
        let db = state.db.lock().map_err(|e| e.to_string())?;
        if let Err(e) = pos_hubrise::outbox::enqueue(&db, order_id, &body_json) {
            // Enqueue failure is non-fatal - the order is already
            // committed locally, just won't appear in tsb-dashboard.
            tracing::warn!(%e, "outbox enqueue failed (non-fatal)");
        }
    }

    Ok(CheckoutResult {
        success: true,
        ticket,
        error: None,
        fdm_connected,
        fiscal_event_id: Some(event_id),
        delivery: Some(delivery_outcome),
    })
}

/// Check FDM connection status.
#[tauri::command]
pub async fn check_fdm_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    do_check_fdm_status(state.inner()).await
}

pub async fn do_check_fdm_status(state: &AppState) -> Result<serde_json::Value, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let fdm = FdmClient::new(&config.fdm_url);

    // FDM messages come back localized to the operator's UI language
    // (Q100/Q110). The FDM only speaks FR/NL/DE/EN, so unsupported UI
    // languages (zh) fall back to FR - the fiscal reference language.
    let lang = match config.language.as_str() {
        "nl" => pos_fiscal::types::Language::Nl,
        "de" => pos_fiscal::types::Language::De,
        "en" => pos_fiscal::types::Language::En,
        _ => pos_fiscal::types::Language::Fr,
    };

    match fdm.status_with_language(lang).await {
        Ok(status) => serde_json::to_value(status).map_err(|e| e.to_string()),
        Err(e) => Ok(serde_json::json!({
            "connected": false,
            "error": e.to_string()
        })),
    }
}
