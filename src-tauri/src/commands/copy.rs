use serde::Serialize;
use tauri::State;

use pos_fiscal::archive::{self, ArchiveParams};
use pos_fiscal::counter::FiscalCounter;
use pos_fiscal::event::build_copy_input;
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::response::FdmReference;
use pos_fiscal::ticket::{FiscalTicket, TicketLine, TicketParams, TicketPayment};
use pos_fiscal::types::EventLabel;

use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyResult {
    pub success: bool,
    pub error: Option<String>,
}

/// Create a copy (event C) of a previous fiscal event.
/// Per SCE 2.0 spec: references the original via fdmId + eventLabel + counters.
#[tauri::command]
pub async fn copy_ticket(
    state: State<'_, AppState>,
    fiscal_event_id: i64,
    employee_insz: String,
) -> Result<CopyResult, String> {
    do_copy_ticket(state.inner(), fiscal_event_id, employee_insz).await
}

pub async fn do_copy_ticket(
    state: &AppState,
    fiscal_event_id: i64,
    employee_insz: String,
) -> Result<CopyResult, String> {
    // 1. Load the original event
    let (original_ref, config, bp_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let event = pos_db::queries::fiscal::get_fiscal_event(&db, fiscal_event_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Fiscal event not found".to_string())?;

        let fdm_id = event
            .fdm_id
            .ok_or_else(|| "Original event has no FDM ID".to_string())?;
        let fdm_datetime = event
            .fdm_datetime
            .ok_or_else(|| "Original event has no FDM datetime".to_string())?;
        let event_label_str = event
            .event_label
            .ok_or_else(|| "Original event has no label".to_string())?;
        let event_counter = event
            .event_counter
            .ok_or_else(|| "Original event has no counter".to_string())?;
        let total_counter = event
            .total_counter
            .ok_or_else(|| "Original event has no total counter".to_string())?;

        let label = match event_label_str.as_str() {
            "N" => EventLabel::N,
            "P" => EventLabel::P,
            "F" => EventLabel::F,
            "S" => EventLabel::S,
            "I" => EventLabel::I,
            "R" => EventLabel::R,
            "C" => EventLabel::C,
            "T" => EventLabel::T,
            _ => return Err(format!("Unknown event label: {event_label_str}")),
        };

        let fdm_ref = FdmReference {
            fdm_id,
            fdm_date_time: fdm_datetime,
            event_label: label,
            event_counter,
            total_counter,
        };

        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = state
            .booking_period_id
            .lock()
            .map_err(|e| e.to_string())?
            .clone();
        (fdm_ref, config, bp)
    };

    // 2. Get sequence number for copy event
    let ticket_number = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        FiscalCounter::next(&db, EventLabel::C).map_err(|e| e.to_string())?
    };

    // 3. Build and send copy event
    let copy_input = build_copy_input(
        &config,
        ticket_number,
        &employee_insz,
        &bp_id,
        &original_ref,
    );
    let request_json = serde_json::to_string(&copy_input).map_err(|e| e.to_string())?;

    let fdm = FdmClient::new(&config.fdm_url);
    let sign_result = fdm.sign_copy(copy_input, config.training_mode).await;

    match sign_result {
        Ok(result) => {
            // Archive + load print data under a single DB lock,
            // then DROP the lock before any async I/O (the printer
            // send) — required because MutexGuard is not Send.
            let print_data = {
                let db = state.db.lock().map_err(|e| e.to_string())?;
                archive::archive_event(
                    &db,
                    ArchiveParams {
                        event_type: EventLabel::C,
                        sequence_number: ticket_number,
                        order_id: None,
                        pos_id: &config.pos_id,
                        terminal_id: &config.terminal_id,
                        user_insz: &employee_insz,
                        request_json: &request_json,
                        sign_result: Some(&result),
                    },
                )
                .map_err(|e| e.to_string())?;

                // Load the original order for the copy receipt.
                let evt = pos_db::queries::fiscal::get_fiscal_event(&db, fiscal_event_id)
                    .ok()
                    .flatten();
                evt.and_then(|e| e.order_id).and_then(|oid| {
                    let order = pos_db::queries::orders::get_order(&db, oid).ok()?;
                    let lines = pos_db::queries::orders::get_order_lines(&db, oid).ok()?;
                    Some((order, lines))
                })
            }; // db lock dropped here

            // Best-effort print — no async I/O under the lock.
            if let Some((order, lines)) = print_data {
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
                let ticket = FiscalTicket::from_sign_result(TicketParams {
                    result: &result,
                    restaurant_name: &config.restaurant_name,
                    restaurant_address: &config.restaurant_address,
                    restaurant_vat: &config.restaurant_vat,
                    restaurant_est_no: &config.restaurant_est_no,
                    lines: ticket_lines,
                    payments: vec![TicketPayment {
                        method: "—".to_string(),
                        amount: order.total_cents as f64 / 100.0,
                    }],
                    total: order.total_cents as f64 / 100.0,
                    is_training: config.training_mode,
                    rounding: None, // copies don't re-round
                    lang: pos_fiscal::ticket::ReceiptLanguage::from_code(&config.language),
                });
                if let Some(ref addr) = config.printer_address {
                    let data = pos_print::receipt::format_receipt(&ticket);
                    let _ = pos_print::transport::send_to_printer(addr, &data).await;
                }
            }

            Ok(CopyResult {
                success: true,
                error: None,
            })
        }
        Err(e) => Ok(CopyResult {
            success: false,
            error: Some(format!("FDM error: {e}")),
        }),
    }
}

/// List recent fiscal events for copy selection.
#[tauri::command]
pub fn list_fiscal_events(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<serde_json::Value, String> {
    do_list_fiscal_events(state.inner(), limit)
}

pub fn do_list_fiscal_events(
    state: &AppState,
    limit: Option<i64>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let events = pos_db::queries::fiscal::list_recent_events(&db, limit.unwrap_or(50))
        .map_err(|e| e.to_string())?;

    let result: Vec<serde_json::Value> = events
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "eventType": e.event_type,
                "sequenceNumber": e.sequence_number,
                "orderId": e.order_id,
                "fdmId": e.fdm_id,
                "fdmDatetime": e.fdm_datetime,
                "eventLabel": e.event_label,
                "shortSignature": e.short_signature,
            })
        })
        .collect();

    serde_json::to_value(result).map_err(|e| e.to_string())
}
