use serde::{Deserialize, Serialize};
use tauri::State;

use pos_fiscal::archive::{self, ArchiveParams};
use pos_fiscal::counter::FiscalCounter;
use pos_fiscal::event::build_sale_input;
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::response::SignResult;
use pos_fiscal::ticket::{self, FiscalTicket, TicketLine, TicketParams, TicketPayment};
use pos_fiscal::types::{EventLabel, PaymentType};

#[cfg(feature = "hubrise")]
use crate::commands::hubrise_push;
use crate::state::AppState;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutPaymentMethod {
    Cash,
    Card,
}

#[derive(Serialize)]
pub struct CheckoutResult {
    pub success: bool,
    pub ticket: Option<FiscalTicket>,
    pub error: Option<String>,
    pub fdm_connected: bool,
}

#[tauri::command]
pub async fn checkout(
    state: State<'_, AppState>,
    order_id: i64,
    payment_method: CheckoutPaymentMethod,
) -> Result<CheckoutResult, String> {
    do_checkout(state.inner(), order_id, payment_method).await
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
        let bp = super::booking_period(&db)?.id;
        (order, lines, entries, config, bp)
    };

    if lines.is_empty() {
        return Ok(CheckoutResult {
            success: false,
            ticket: None,
            error: Some("Order has no lines".into()),
            fdm_connected: false,
        });
    }

    // Guard: no zero-total sale. Together with the empty-order check
    // above this makes a "zero registration" impossible, as declared
    // in the certification dossier (manual §III.D).
    if order.total_cents <= 0 {
        return Ok(CheckoutResult {
            success: false,
            ticket: None,
            error: Some("Order total must be greater than zero".into()),
            fdm_connected: false,
        });
    }

    // Guard: reject if order is already closed (prevents double
    // fiscal events when the operator taps "Payer" twice before
    // the Vue loading guard kicks in).
    if order.status != pos_core::order::OrderStatus::Open {
        return Ok(CheckoutResult {
            success: false,
            ticket: None,
            error: Some("Order already closed".into()),
            fdm_connected: false,
        });
    }

    // 2. RESERVE (peek, don't commit) the next sequence number.
    //
    // We deliberately do NOT call `FiscalCounter::next` here yet.
    // Allocating before the FDM round-trip would create a gap in
    // the sequence whenever the FDM call fails — a direct violation
    // of the SCE 2.0 "strictly sequential, no gaps" requirement.
    //
    // Instead we peek (read-only) to obtain the value we'll send to
    // the FDM, then commit the increment inside the same SQLite
    // transaction that writes the archive (step 6-8 below). If the
    // FDM fails, the counter stays untouched and the next attempt
    // sees the same value — combined with the FDM's idempotency on
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

    let sale_input = build_sale_input(
        &config,
        &order,
        &entries,
        ticket_number,
        &booking_period_id,
        payment_type,
        payment_total_cents,
        None, // no fdm_ref for normal sales
    );

    // 4. Serialize request for archival
    let request_json = serde_json::to_string(&sale_input).map_err(|e| e.to_string())?;

    // 5. Send to FDM
    let fdm = FdmClient::new(&config.fdm_url);
    let fdm_result: Result<SignResult, _> = fdm.sign_sale(sale_input, config.training_mode).await;

    let (sign_result, fdm_connected) = match fdm_result {
        Ok(result) => (Some(result), true),
        Err(pos_fiscal::fdm_client::FdmError::NotConnected) => {
            // FDM not connected — per SCE 2.0 spec, POS MUST block.
            // The allow-offline feature flag is for development only;
            // production builds (cargo tauri build) never include it.
            #[cfg(feature = "allow-offline")]
            {
                (None, false)
            }
            #[cfg(not(feature = "allow-offline"))]
            {
                return Ok(CheckoutResult {
                    success: false,
                    ticket: None,
                    error: Some("FDM not connected — cannot complete transaction".into()),
                    fdm_connected: false,
                });
            }
        }
        Err(e) => {
            return Ok(CheckoutResult {
                success: false,
                ticket: None,
                error: Some(format!("FDM error: {e}")),
                fdm_connected: true,
            });
        }
    };

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
    // the counter advance — so no gap is ever observable.
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;

        let label = if config.training_mode {
            EventLabel::T
        } else {
            EventLabel::N
        };
        let committed_no = FiscalCounter::next(&tx, label).map_err(|e| e.to_string())?;
        // Graceful error, not a panic: see the identical comment in
        // `commands/mod.rs::commit_signed_event` — a panic here would
        // unwind while the `state.db` MutexGuard is still held and
        // poison it for the whole process. Concurrent checkouts of the
        // same order are exactly the scenario this guards against; the
        // transaction rolls back, so the loser sees a clean error and
        // can retry.
        if committed_no != ticket_number {
            return Err(format!(
                "counter drifted between peek and commit for {label:?} (peeked \
                 {ticket_number}, committed {committed_no}) — likely a concurrent \
                 checkout of the same order; retry"
            ));
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

        // Record the payment so X/Z reports (`get_payment_totals`,
        // `get_user_payment_totals`) have real data to sum — without
        // this row the FDM-facing `payments` block of every turnover
        // report would always be empty, no matter how much was sold.
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

        tx.commit().map_err(|e| e.to_string())?;
    }

    // 7. Build fiscal ticket for display/print
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

    // 9. Auto-print receipt + open drawer for cash
    if let Some(ref t) = ticket {
        let printer_address = config.printer_address.clone();
        if let Some(ref addr) = printer_address {
            let receipt_data = pos_print::receipt::format_receipt(t);
            match pos_print::transport::send_to_printer(addr, &receipt_data).await {
                Ok(status) => {
                    if !status.paper_ok {
                        tracing::warn!(?status.warnings, "printer: paper issue after receipt");
                    }
                    if !status.cover_closed {
                        tracing::warn!("printer: cover open after receipt");
                    }
                }
                Err(e) => {
                    tracing::warn!(%e, "receipt print failed (non-fatal)");
                }
            }

            // Open cash drawer for cash payments
            if matches!(payment_method, CheckoutPaymentMethod::Cash) {
                let drawer_data = pos_print::drawer::kick_drawer_bytes();
                let _ = pos_print::transport::send_to_printer(addr, &drawer_data).await;
            }
        }
    }

    // 10. Enqueue the completed order for async push to HubRise.
    // This is a pure SQLite write — zero network, zero latency, never
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
            // Enqueue failure is non-fatal — the order is already
            // committed locally, just won't appear in tsb-dashboard.
            tracing::warn!(%e, "outbox enqueue failed (non-fatal)");
        }
    }

    Ok(CheckoutResult {
        success: true,
        ticket,
        error: None,
        fdm_connected,
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
    // languages (zh) fall back to FR — the fiscal reference language.
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
