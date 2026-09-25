use serde::Serialize;
use tauri::State;

use pos_fiscal::archive::{self, ArchiveParams};
use pos_fiscal::counter::FiscalCounter;
use pos_fiscal::event::{build_order_input, build_pre_bill_input};
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::types::EventLabel;

use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableInfo {
    pub table_number: i32,
    pub order_id: Option<i64>,
    pub total_cents: i64,
    pub line_count: i64,
    pub status: String,
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
        let row: Option<(i64, i64, i64)> = db
            .query_row(
                "SELECT o.id, o.total_cents, COUNT(ol.id)
                 FROM orders o
                 LEFT JOIN order_lines ol ON ol.order_id = o.id
                 WHERE o.table_number = ?1 AND o.status = 'open'
                 GROUP BY o.id
                 HAVING COUNT(ol.id) > 0
                 LIMIT 1",
                rusqlite::params![t],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();

        tables.push(match row {
            Some((order_id, total, count)) => TableInfo {
                table_number: t,
                order_id: Some(order_id),
                total_cents: total,
                line_count: count,
                status: "occupied".to_string(),
            },
            None => TableInfo {
                table_number: t,
                order_id: None,
                total_cents: 0,
                line_count: 0,
                status: "free".to_string(),
            },
        });
    }

    Ok(tables)
}

/// Send a signOrder (pro forma P event) for an open table.
/// Called when items are added to an open table to register with FDM.
#[tauri::command]
pub async fn sign_table_order(
    state: State<'_, AppState>,
    order_id: i64,
    cost_center_ref: String,
) -> Result<(), String> {
    do_sign_table_order(state.inner(), order_id, cost_center_ref).await
}

pub async fn do_sign_table_order(
    state: &AppState,
    order_id: i64,
    cost_center_ref: String,
) -> Result<(), String> {
    let (order, lines, config, bp_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let order = pos_db::queries::orders::get_order(&db, order_id).map_err(|e| e.to_string())?;
        let lines =
            pos_db::queries::orders::get_order_lines(&db, order_id).map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = state
            .booking_period_id
            .lock()
            .map_err(|e| e.to_string())?
            .clone();
        (order, lines, config, bp)
    };

    if lines.is_empty() {
        return Ok(());
    }

    let ticket_number = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        FiscalCounter::next(&db, EventLabel::P.in_mode(config.training_mode))
            .map_err(|e| e.to_string())?
    };

    let order_input = build_order_input(
        &config,
        &order,
        &lines,
        ticket_number,
        &bp_id,
        &cost_center_ref,
    );
    let request_json = serde_json::to_string(&order_input).map_err(|e| e.to_string())?;

    let fdm = FdmClient::new(&config.fdm_url);
    let sign_result = fdm.sign_order(order_input, config.training_mode).await.ok();

    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        archive::archive_event(
            &db,
            ArchiveParams {
                event_type: EventLabel::P.in_mode(config.training_mode),
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
    }

    Ok(())
}

/// Send a signPreBill (provisional bill P event).
#[tauri::command]
pub async fn sign_pre_bill(state: State<'_, AppState>, order_id: i64) -> Result<(), String> {
    do_sign_pre_bill(state.inner(), order_id).await
}

pub async fn do_sign_pre_bill(state: &AppState, order_id: i64) -> Result<(), String> {
    let (order, lines, config, bp_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let order = pos_db::queries::orders::get_order(&db, order_id).map_err(|e| e.to_string())?;
        let lines =
            pos_db::queries::orders::get_order_lines(&db, order_id).map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = state
            .booking_period_id
            .lock()
            .map_err(|e| e.to_string())?
            .clone();
        (order, lines, config, bp)
    };

    let ticket_number = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        FiscalCounter::next(&db, EventLabel::P.in_mode(config.training_mode))
            .map_err(|e| e.to_string())?
    };

    let pre_bill = build_pre_bill_input(&config, &order, &lines, ticket_number, &bp_id);
    let request_json = serde_json::to_string(&pre_bill).map_err(|e| e.to_string())?;

    let fdm = FdmClient::new(&config.fdm_url);
    let sign_result = fdm.sign_pre_bill(pre_bill, config.training_mode).await.ok();

    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        archive::archive_event(
            &db,
            ArchiveParams {
                event_type: EventLabel::P.in_mode(config.training_mode),
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
    }

    // Print pro forma receipt
    if let (Some(addr), Some(result)) = (config.printer_address.as_ref(), sign_result.as_ref()) {
        let operator = super::operator_name(state, &order.user_insz);
        let ticket =
            pos_fiscal::ticket::FiscalTicket::from_sign_result(pos_fiscal::ticket::TicketParams {
                result,
                restaurant_name: &config.restaurant_name,
                restaurant_address: &config.restaurant_address,
                restaurant_vat: &config.restaurant_vat,
                restaurant_est_no: &config.restaurant_est_no,
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

    Ok(())
}
