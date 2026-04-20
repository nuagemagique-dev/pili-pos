use serde::{Deserialize, Serialize};
use tauri::State;

use pos_fiscal::archive::{self, ArchiveParams};
use pos_fiscal::counter::FiscalCounter;
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::inputs::*;
use pos_fiscal::types::*;

use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialResult {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoneyDirection {
    In,
    Out,
}

/// Record a cash in/out financial event (Event F).
/// Used for declaring cash float, tips, withdrawals, etc.
#[tauri::command]
pub async fn money_in_out(
    state: State<'_, AppState>,
    employee_insz: String,
    amount_cents: i64,
    direction: MoneyDirection,
    reason: String,
) -> Result<FinancialResult, String> {
    let (config, bp_id, ticket_number) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = state.booking_period_id.lock().map_err(|e| e.to_string())?.clone();
        let tn = FiscalCounter::next(&db, EventLabel::F).map_err(|e| e.to_string())?;
        (config, bp, tn)
    };

    let common = pos_fiscal::event::build_common_fields(
        &config, ticket_number, &employee_insz, &bp_id,
    );

    // Direction conveyed by sign: positive = money in, negative = money out
    let amount = match direction {
        MoneyDirection::In => amount_cents as f64 / 100.0,
        MoneyDirection::Out => -(amount_cents as f64 / 100.0),
    };

    let input = MoneyInOutInput {
        common,
        drawer: None,
        financials: vec![MoneyInOutLineInput {
            id: "MOV1".to_string(),
            name: reason.clone(),
            payment_type: PaymentType::Cash,
            provider: None,
            input_method: InputMethod::Manual,
            amount,
            amount_type: MoneyInOutLineType::MoneyInOut,
            foreign_currency: None,
            reference: None,
            drawer: None,
        }],
    };

    let request_json = serde_json::to_string(&input).map_err(|e| e.to_string())?;

    let fdm = FdmClient::new(&config.fdm_url);
    let sign_result = fdm.sign_money_in_out(input, config.training_mode).await.ok();

    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        archive::archive_event(&db, ArchiveParams {
            event_type: EventLabel::F,
            sequence_number: ticket_number,
            order_id: None,
            pos_id: &config.pos_id,
            terminal_id: &config.terminal_id,
            user_insz: &employee_insz,
            request_json: &request_json,
            sign_result: sign_result.as_ref(),
        }).map_err(|e| e.to_string())?;
    }

    Ok(FinancialResult {
        success: true,
        error: None,
    })
}

/// Record a fiscal drawer open event (Event F).
/// Tracks when the cash drawer is opened outside of a sale.
#[tauri::command]
pub async fn open_drawer_fiscal(
    state: State<'_, AppState>,
    employee_insz: String,
) -> Result<FinancialResult, String> {
    let (config, bp_id, ticket_number) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = state.booking_period_id.lock().map_err(|e| e.to_string())?.clone();
        let tn = FiscalCounter::next(&db, EventLabel::F).map_err(|e| e.to_string())?;
        (config, bp, tn)
    };

    let common = pos_fiscal::event::build_common_fields(
        &config, ticket_number, &employee_insz, &bp_id,
    );

    let input = DrawerOpenInput {
        common,
        drawer: Some(DrawerInput {
            id: "DRAWER1".to_string(),
            name: "Tiroir principal".to_string(),
        }),
    };

    let request_json = serde_json::to_string(&input).map_err(|e| e.to_string())?;

    let fdm = FdmClient::new(&config.fdm_url);
    let sign_result = fdm.sign_drawer_open(input, config.training_mode).await.ok();

    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        archive::archive_event(&db, ArchiveParams {
            event_type: EventLabel::F,
            sequence_number: ticket_number,
            order_id: None,
            pos_id: &config.pos_id,
            terminal_id: &config.terminal_id,
            user_insz: &employee_insz,
            request_json: &request_json,
            sign_result: sign_result.as_ref(),
        }).map_err(|e| e.to_string())?;
    }

    // Kick the physical drawer open
    if let Some(ref addr) = config.printer_address {
        let drawer_data = pos_print::drawer::kick_drawer_bytes();
        let _ = pos_print::transport::send_to_printer(addr, &drawer_data).await;
    }

    Ok(FinancialResult {
        success: true,
        error: None,
    })
}
