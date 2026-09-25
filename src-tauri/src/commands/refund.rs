use serde::Deserialize;
use tauri::State;

use pos_db::queries::fiscal::{self, RefundableEvent};
use pos_fiscal::archive::{self, ArchiveParams};
use pos_fiscal::counter::FiscalCounter;
use pos_fiscal::event::build_sale_input;
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::response::FdmReference;
use pos_fiscal::ticket::{FiscalTicket, TicketLine, TicketParams, TicketPayment};
use pos_fiscal::types::{EventLabel, PaymentType};

use crate::commands::checkout::CheckoutResult;
use crate::state::AppState;

/// One item to refund (sent from the frontend). Price, VAT label,
/// name and department are taken from the original sale's line, never
/// from the frontend.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundItem {
    pub menu_item_id: i64,
    pub quantity: i32, // positive — backend negates it
}

/// List recent closed N-events that can be refunded.
#[tauri::command]
pub fn list_refundable_events(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<RefundableEvent>, String> {
    do_list_refundable_events(state.inner(), limit)
}

pub fn do_list_refundable_events(
    state: &AppState,
    limit: Option<i64>,
) -> Result<Vec<RefundableEvent>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    fiscal::list_refundable_events(&db, limit.unwrap_or(20)).map_err(|e| e.to_string())
}

/// Process a full or partial refund for a previously signed ticket.
///
/// Creates a new Event N with:
/// - negative-quantity lines (negQuantityReason = REFUND)
/// - fdmRef referencing the original ticket
/// - cash payment (refund handed to customer)
///
/// Archives the event, prints a REFUND receipt, opens the drawer.
#[tauri::command]
pub async fn refund_checkout(
    state: State<'_, AppState>,
    original_fiscal_event_id: i64,
    items: Vec<RefundItem>,
    employee_insz: String,
) -> Result<CheckoutResult, String> {
    do_refund_checkout(
        state.inner(),
        original_fiscal_event_id,
        items,
        employee_insz,
    )
    .await
}

pub async fn do_refund_checkout(
    state: &AppState,
    original_fiscal_event_id: i64,
    items: Vec<RefundItem>,
    employee_insz: String,
) -> Result<CheckoutResult, String> {
    if items.is_empty() {
        return Ok(CheckoutResult {
            success: false,
            ticket: None,
            error: Some("No items to refund".into()),
            fdm_connected: false,
        });
    }

    // 1. Load original fiscal event → extract FdmReference
    let (original_ref, original_lines, config, bp_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let event = fiscal::get_fiscal_event(&db, original_fiscal_event_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Original fiscal event not found".to_string())?;
        let original_order_id = event.order_id.ok_or("Original event has no order")?;
        let original_lines = pos_db::queries::orders::get_order_lines(&db, original_order_id)
            .map_err(|e| e.to_string())?;

        let fdm_id = event.fdm_id.ok_or("Original event has no FDM ID")?;
        let fdm_datetime = event
            .fdm_datetime
            .ok_or("Original event has no FDM datetime")?;
        let event_label_str = event.event_label.ok_or("Original event has no label")?;
        let event_counter = event.event_counter.ok_or("Original event has no counter")?;
        let total_counter = event
            .total_counter
            .ok_or("Original event has no total counter")?;

        let label = match event_label_str.as_str() {
            "N" => EventLabel::N,
            other => return Err(format!("Cannot refund event type {other}")),
        };

        let fdm_ref = FdmReference {
            fdm_id,
            fdm_date_time: fdm_datetime,
            event_label: label,
            event_counter,
            total_counter,
        };

        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = super::booking_period(&db)?.id;
        (fdm_ref, original_lines, config, bp)
    };

    // 2. Create a refund order with negative-quantity lines
    let (refund_order, refund_lines) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let order = pos_db::queries::orders::create_order(
            &db,
            pos_core::order::OrderType::Takeaway,
            None,
            &employee_insz,
        )
        .map_err(|e| e.to_string())?;

        let mut requested: std::collections::HashMap<i64, i32> = std::collections::HashMap::new();
        for item in &items {
            if item.quantity <= 0 {
                return Err(format!(
                    "Refund quantity for product {} must be positive",
                    item.menu_item_id
                ));
            }
            *requested.entry(item.menu_item_id).or_default() += item.quantity;
        }
        for item in &items {
            let original = refunded_line(
                &original_lines,
                item.menu_item_id,
                requested[&item.menu_item_id],
            )?;
            pos_db::queries::orders::add_order_line(
                &db,
                pos_db::queries::orders::NewOrderLine {
                    order_id: order.id,
                    menu_item_id: item.menu_item_id,
                    quantity: -(item.quantity), // NEGATE for refund
                    unit_price_cents: original.unit_price_cents,
                    vat_code: original.vat_code,
                    name: &original.name,
                    modifiers_json: None,
                    department_id: &original.department_id,
                    department_name: &original.department_name,
                },
            )
            .map_err(|e| e.to_string())?;
        }

        let updated_order =
            pos_db::queries::orders::get_order(&db, order.id).map_err(|e| e.to_string())?;
        let lines =
            pos_db::queries::orders::get_order_lines(&db, order.id).map_err(|e| e.to_string())?;
        (updated_order, lines)
    };

    // 3. RESERVE the next ticket number (peek, no commit). See the
    // identical pattern in `checkout.rs` — the counter is committed
    // inside the archive transaction below, after the FDM accepts
    // the refund.
    let ticket_number = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let label = if config.training_mode {
            EventLabel::T
        } else {
            EventLabel::N
        };
        FiscalCounter::peek_next(&db, label).map_err(|e| e.to_string())?
    };

    // 4. Build SaleInput with negative lines + original fdm_ref
    let sale_input = build_sale_input(
        &config,
        &refund_order,
        &refund_lines,
        ticket_number,
        &bp_id,
        PaymentType::Cash,
        refund_order.total_cents, // negative
        Some(&original_ref),
    );

    let request_json = serde_json::to_string(&sale_input).map_err(|e| e.to_string())?;

    // 5. Send to FDM
    let fdm = FdmClient::new(&config.fdm_url);
    let fdm_result = fdm.sign_sale(sale_input, config.training_mode).await;

    let (sign_result, fdm_connected) = match fdm_result {
        Ok(result) => (Some(result), true),
        Err(pos_fiscal::fdm_client::FdmError::NotConnected) => {
            #[cfg(feature = "allow-offline")]
            {
                (None, false)
            }
            #[cfg(not(feature = "allow-offline"))]
            {
                return Ok(CheckoutResult {
                    success: false,
                    ticket: None,
                    error: Some("FDM not connected — cannot process refund".into()),
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

    // 6. COMMIT counter + archive + close refund order atomically.
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;

        let label = if config.training_mode {
            EventLabel::T
        } else {
            EventLabel::N
        };
        let committed_no = FiscalCounter::next(&tx, label).map_err(|e| e.to_string())?;
        debug_assert_eq!(committed_no, ticket_number);
        let event_id = archive::archive_event(
            &tx,
            ArchiveParams {
                event_type: label,
                sequence_number: ticket_number,
                order_id: Some(refund_order.id),
                pos_id: &config.pos_id,
                terminal_id: &config.terminal_id,
                user_insz: &employee_insz,
                request_json: &request_json,
                sign_result: sign_result.as_ref(),
            },
        )
        .map_err(|e| e.to_string())?;
        pos_db::queries::orders::mark_order_entries_sent(&tx, refund_order.id, i64::MAX, event_id)
            .map_err(|e| e.to_string())?;

        tx.execute(
            "UPDATE orders SET status = 'closed', closed_at = datetime('now') WHERE id = ?1",
            rusqlite::params![refund_order.id],
        )
        .map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;
    }

    // 7. Build fiscal ticket for display/print
    let operator = super::operator_name(state, &refund_order.user_insz);
    let ticket = sign_result.as_ref().map(|result| {
        let ticket_lines: Vec<TicketLine> = refund_lines
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
            method: "Cash".to_string(),
            amount: refund_order.total_cents as f64 / 100.0,
        }];

        FiscalTicket::from_sign_result(TicketParams {
            result,
            restaurant_name: &config.restaurant_name,
            restaurant_address: &config.restaurant_address,
            restaurant_vat: &config.restaurant_vat,
            restaurant_est_no: &config.restaurant_est_no,
            lines: ticket_lines,
            payments,
            total: refund_order.total_cents as f64 / 100.0,
            is_training: config.training_mode,
            rounding: None,
            lang: pos_fiscal::ticket::ReceiptLanguage::from_code(&config.language),
            operator: &operator,
        })
    });

    // 8. Auto-print REFUND receipt + open drawer
    if let Some(ref t) = ticket
        && let Some(ref addr) = config.printer_address
    {
        let receipt_data = pos_print::receipt::format_receipt(t);
        if let Err(e) = pos_print::transport::send_to_printer(addr, &receipt_data).await {
            tracing::warn!(%e, "refund receipt print failed (non-fatal)");
        }
        // Open drawer for cash refund
        let drawer_data = pos_print::drawer::kick_drawer_bytes();
        let _ = pos_print::transport::send_to_printer(addr, &drawer_data).await;
    }

    Ok(CheckoutResult {
        success: true,
        ticket,
        error: None,
        fdm_connected,
    })
}

/// The original sale line a refunded product comes from. The quantity
/// refunded for a product (summed over the request) may not exceed the
/// quantity sold for it on the original ticket.
fn refunded_line(
    original_lines: &[pos_core::order::OrderLine],
    menu_item_id: i64,
    requested: i32,
) -> Result<&pos_core::order::OrderLine, String> {
    let line = original_lines
        .iter()
        .find(|l| l.menu_item_id == menu_item_id)
        .ok_or_else(|| format!("Product {menu_item_id} is not on the original ticket"))?;
    let sold: i32 = original_lines
        .iter()
        .filter(|l| l.menu_item_id == menu_item_id)
        .map(|l| l.quantity)
        .sum();
    if requested > sold {
        return Err(format!(
            "Cannot refund {requested} of product {menu_item_id}: {sold} sold on the original ticket"
        ));
    }
    Ok(line)
}
