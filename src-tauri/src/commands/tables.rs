use serde::Serialize;
use tauri::State;

use pos_core::order::{OrderStatus, OrderType};
use pos_fiscal::event::{build_order_input, build_pre_bill_input};
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::types::EventLabel;

use crate::commands::access::{self, Access};
use crate::commands::bar_slip::{self, SlipOutcome};
use crate::commands::errors;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableInfo {
    pub table_number: i32,
    pub order_id: Option<i64>,
    pub total_cents: i64,
    pub line_count: i64,
    pub status: String,
    /// When the table's order was opened (UTC, RFC 3339).
    pub opened_at: Option<String>,
    /// A provisional bill was printed and the order has not changed since.
    pub pre_bill_printed: bool,
}

/// List tables with their current order status.
///
/// A table is only reported as **occupied** when its open order
/// contains at least one line. Empty open orders (typical after a
/// tap-and-walk-away) are treated as free so the TablesView doesn't
/// surface ghost "0.00 €" cards that add cognitive load for zero
/// informational value (UX Guide §2.1).
#[tauri::command]
pub fn list_tables(state: State<'_, AppState>, table_count: i32) -> Result<Vec<TableInfo>, String> {
    do_list_tables(state.inner(), table_count)
}

pub fn do_list_tables(state: &AppState, table_count: i32) -> Result<Vec<TableInfo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut tables = Vec::new();

    for t in 1..=table_count {
        let row: Option<(i64, i64, i64, String, bool)> = db
            .query_row(
                "SELECT o.id, o.total_cents, COUNT(ol.id), o.created_at, o.pre_bill_at IS NOT NULL
                 FROM orders o
                 LEFT JOIN order_lines ol ON ol.order_id = o.id
                 WHERE o.table_number = ?1 AND o.status = 'open'
                 GROUP BY o.id
                 HAVING COUNT(ol.id) > 0
                 ORDER BY o.id
                 LIMIT 1",
                rusqlite::params![t],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .ok();

        tables.push(match row {
            Some((order_id, total, count, opened_at, pre_bill_printed)) => TableInfo {
                table_number: t,
                order_id: Some(order_id),
                total_cents: total,
                line_count: count,
                status: "occupied".to_string(),
                opened_at: Some(opened_at),
                pre_bill_printed,
            },
            None => TableInfo {
                table_number: t,
                order_id: None,
                total_cents: 0,
                line_count: 0,
                status: "free".to_string(),
                opened_at: None,
                pre_bill_printed: false,
            },
        });
    }

    Ok(tables)
}

/// An open takeaway with at least one line: a direct sale not paid yet.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeawayInfo {
    pub order_id: i64,
    pub total_cents: i64,
    pub line_count: i64,
    pub created_at: String,
}

/// The takeaways left open (nothing is registered for them until they
/// are paid): the Tables screen lists them so each one ends in a sale
/// or is closed unpaid (FAQ Q205). Empty drafts are left out.
#[tauri::command]
pub fn list_open_takeaways(state: State<'_, AppState>) -> Result<Vec<TakeawayInfo>, String> {
    do_list_open_takeaways(state.inner())
}

pub fn do_list_open_takeaways(state: &AppState) -> Result<Vec<TakeawayInfo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT o.id, o.total_cents, COUNT(ol.id), o.created_at
             FROM orders o
             JOIN order_lines ol ON ol.order_id = o.id
             WHERE o.order_type = 'takeaway' AND o.status = 'open'
               AND o.hubrise_order_id IS NULL
             GROUP BY o.id
             ORDER BY o.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(TakeawayInfo {
                order_id: r.get(0)?,
                total_cents: r.get(1)?,
                line_count: r.get(2)?,
                created_at: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// Register what was ordered at a table since the last registration
/// (event P, "vente interrompue", POS description v2.2 §1.3): the
/// order's journal entries not yet transmitted, corrections included,
/// in chronological order. A no-op when nothing changed. The UI calls
/// it when the operator leaves a table. Once signed, the new lines go
/// to the bar / kitchen slip; the result says what became of it.
#[tauri::command]
pub async fn sign_table_order(
    state: State<'_, AppState>,
    order_id: i64,
) -> Result<SlipOutcome, String> {
    do_sign_table_order(state.inner(), order_id).await
}

pub async fn do_sign_table_order(state: &AppState, order_id: i64) -> Result<SlipOutcome, String> {
    sign_table_order_as(state, order_id, Access::Staff).await
}

/// Register the order's unsent entries (event P) in the name of the
/// logged-in operator, if their role allows `access`, then print them
/// on the bar / kitchen slip.
async fn sign_table_order_as(
    state: &AppState,
    order_id: i64,
    access: Access,
) -> Result<SlipOutcome, String> {
    let operator = access::operator(state, access)?;
    let (mut order, entries, config, bp_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let order = pos_db::queries::orders::get_order(&db, order_id).map_err(|e| e.to_string())?;
        let entries = pos_db::queries::orders::get_order_entries(&db, order_id, true)
            .map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = super::booking(&db, &config)?;
        (order, entries, config, bp)
    };

    // A takeaway is a direct sale: it stays a draft until it is paid
    // (or closed unpaid) and ends in an N event, never a P (FAQ Q205).
    if order.order_type == OrderType::Takeaway {
        return Err(errors::TAKEAWAY_NOT_REGISTERED.into());
    }
    let Some(last_entry) = entries.last() else {
        return Ok(SlipOutcome::NothingToPrint);
    };
    order.user_insz = operator.insz;
    if order.status != OrderStatus::Open {
        return Err(errors::ORDER_NOT_OPEN.into());
    }

    let label = EventLabel::P.in_mode(config.training_mode);
    let ticket_number = super::peek_ticket_number(state, label)?;
    let order_input = build_order_input(&config, &order, &entries, ticket_number, &bp_id);
    let request_json = serde_json::to_string(&order_input).map_err(|e| e.to_string())?;

    let fdm = FdmClient::from_config(&config);
    let sign_result = fdm
        .sign_order(order_input, config.training_mode)
        .await
        .map_err(|e| format!("FDM error: {e}"))?;

    super::commit_signed_event(
        state,
        &config,
        super::SignedEvent {
            label,
            ticket_number,
            order_id: Some(order_id),
            user_insz: &order.user_insz,
            request_json: &request_json,
            sign_result: &sign_result,
            sent_entries: Some((order_id, last_entry.id)),
        },
    )?;
    Ok(bar_slip::print_bar_slip(
        state,
        &config,
        &order.user_insz,
        order.table_number,
        &entries,
    )
    .await)
}

/// Send a signPreBill (provisional bill P event).
#[tauri::command]
pub async fn sign_pre_bill(
    state: State<'_, AppState>,
    order_id: i64,
) -> Result<SlipOutcome, String> {
    do_sign_pre_bill(state.inner(), order_id).await
}

/// Provisional bill ("addition provisoire", §1.4): the consolidated
/// order, printed PRO FORMA. Entries not yet registered are sent as an
/// order first, since a pre-bill may not add or change lines; the
/// result is the bar / kitchen slip of those entries.
pub async fn do_sign_pre_bill(state: &AppState, order_id: i64) -> Result<SlipOutcome, String> {
    let operator = access::operator(state, Access::Cashier)?;
    let slip = sign_table_order_as(state, order_id, Access::Cashier).await?;

    let (mut order, lines, config, bp_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let order = pos_db::queries::orders::get_order(&db, order_id).map_err(|e| e.to_string())?;
        let lines =
            pos_db::queries::orders::get_order_lines(&db, order_id).map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = super::booking(&db, &config)?;
        (order, lines, config, bp)
    };
    order.user_insz = operator.insz;
    if lines.is_empty() {
        return Err(errors::ORDER_NO_LINES.into());
    }

    let label = EventLabel::P.in_mode(config.training_mode);
    let ticket_number = super::peek_ticket_number(state, label)?;
    let pre_bill = build_pre_bill_input(&config, &order, &lines, ticket_number, &bp_id);
    let request_json = serde_json::to_string(&pre_bill).map_err(|e| e.to_string())?;

    let fdm = FdmClient::from_config(&config);
    let result = fdm
        .sign_pre_bill(pre_bill, config.training_mode)
        .await
        .map_err(|e| format!("FDM error: {e}"))?;

    super::commit_signed_event(
        state,
        &config,
        super::SignedEvent {
            label,
            ticket_number,
            order_id: Some(order_id),
            user_insz: &order.user_insz,
            request_json: &request_json,
            sign_result: &result,
            sent_entries: None,
        },
    )?;
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        pos_db::queries::orders::mark_pre_bill_printed(&db, order_id).map_err(|e| e.to_string())?;
    }

    // Print pro forma receipt
    if let Some(addr) = config.printer_address.as_ref() {
        let operator = super::operator_name(state, &order.user_insz);
        let ticket =
            pos_fiscal::ticket::FiscalTicket::from_sign_result(pos_fiscal::ticket::TicketParams {
                result: &result,
                restaurant_name: &config.restaurant_name,
                restaurant_address: &config.restaurant_address,
                restaurant_vat: &config.restaurant_vat,
                restaurant_est_no: &config.restaurant_est_no,
                restaurant_legal_name: config.legal_name(),
                lines: lines
                    .iter()
                    .map(|l| pos_fiscal::ticket::TicketLine {
                        quantity: l.quantity as f64,
                        description: l.name.clone(),
                        unit_price: l.unit_price_cents as f64 / 100.0,
                        total: (l.unit_price_cents * l.quantity as i64) as f64 / 100.0,
                        vat_code: l.vat_code.as_str().to_string(),
                    })
                    .collect(),
                payments: vec![],
                total: order.total_cents as f64 / 100.0,
                is_training: config.training_mode,
                rounding: None,
                lang: pos_fiscal::ticket::ReceiptLanguage::from_code(&config.language),
                operator: &operator,
            });
        let data = pos_print::receipt::format_receipt(&ticket);
        let _ = pos_print::transport::send_to_printer(addr, &data).await;
    }

    Ok(slip)
}
