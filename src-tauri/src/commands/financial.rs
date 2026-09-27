use serde::{Deserialize, Serialize};
use tauri::State;

use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::inputs::*;
use pos_fiscal::types::*;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

#[derive(Debug, Serialize)]
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
    /// The cash counted in the drawer (Basic Set M133): the amount is
    /// what is there, not a movement.
    Declaration,
}

/// Record a cash financial event (Event F): money put in or taken
/// out (cash float, withdrawal, ...), or the count of the drawer.
#[tauri::command]
pub async fn money_in_out(
    state: State<'_, AppState>,
    employee_insz: String,
    amount_cents: i64,
    direction: MoneyDirection,
    reason: String,
) -> Result<FinancialResult, String> {
    do_money_in_out(
        state.inner(),
        employee_insz,
        amount_cents,
        direction,
        reason,
    )
    .await
}

pub async fn do_money_in_out(
    state: &AppState,
    employee_insz: String,
    amount_cents: i64,
    direction: MoneyDirection,
    reason: String,
) -> Result<FinancialResult, String> {
    access::operator_named(state, &employee_insz, Access::Admin)?;
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let bp_id = super::current_booking(state)?;
    let label = EventLabel::F.in_mode(config.training_mode);
    let ticket_number = super::peek_ticket_number(state, label)?;

    let common = pos_fiscal::event::build_common_fields(
        &config,
        ticket_number,
        &employee_insz,
        &bp_id,
        // Drawer events are not printed.
        pos_fiscal::types::TicketMedium::None,
    );

    // A movement's direction is its sign: positive in, negative out.
    // A declaration is the counted amount, never negative.
    if amount_cents < 0 {
        return Err(errors::AMOUNT_NEGATIVE.into());
    }
    let (amount, amount_type) = match direction {
        MoneyDirection::In => (amount_cents as f64 / 100.0, MoneyInOutLineType::MoneyInOut),
        MoneyDirection::Out => (
            -(amount_cents as f64 / 100.0),
            MoneyInOutLineType::MoneyInOut,
        ),
        MoneyDirection::Declaration => (
            amount_cents as f64 / 100.0,
            MoneyInOutLineType::DrawerDeclaration,
        ),
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
            amount_type,
            foreign_currency: None,
            reference: None,
            drawer: None,
        }],
    };

    let request_json = serde_json::to_string(&input).map_err(|e| e.to_string())?;

    let fdm = FdmClient::from_config(&config);
    let sign_result = match fdm.sign_money_in_out(input, config.training_mode).await {
        Ok(result) => result,
        Err(e) => {
            return Ok(FinancialResult {
                success: false,
                error: Some(format!("FDM error: {e}")),
            });
        }
    };

    super::commit_signed_event(
        state,
        &config,
        super::SignedEvent {
            label,
            ticket_number,
            order_id: None,
            user_insz: &employee_insz,
            request_json: &request_json,
            sign_result: &sign_result,
            sent_entries: None,
        },
    )?;

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
    do_open_drawer_fiscal(state.inner(), employee_insz).await
}

pub async fn do_open_drawer_fiscal(
    state: &AppState,
    employee_insz: String,
) -> Result<FinancialResult, String> {
    access::operator_named(state, &employee_insz, Access::Admin)?;
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let bp_id = super::current_booking(state)?;
    let label = EventLabel::F.in_mode(config.training_mode);
    let ticket_number = super::peek_ticket_number(state, label)?;

    let common = pos_fiscal::event::build_common_fields(
        &config,
        ticket_number,
        &employee_insz,
        &bp_id,
        // Drawer events are not printed.
        pos_fiscal::types::TicketMedium::None,
    );

    let input = DrawerOpenInput {
        common,
        drawer: Some(DrawerInput {
            id: "DRAWER1".to_string(),
            name: "Tiroir principal".to_string(),
        }),
    };

    let request_json = serde_json::to_string(&input).map_err(|e| e.to_string())?;

    let fdm = FdmClient::from_config(&config);
    let sign_result = match fdm.sign_drawer_open(input, config.training_mode).await {
        Ok(result) => result,
        Err(e) => {
            return Ok(FinancialResult {
                success: false,
                error: Some(format!("FDM error: {e}")),
            });
        }
    };

    super::commit_signed_event(
        state,
        &config,
        super::SignedEvent {
            label,
            ticket_number,
            order_id: None,
            user_insz: &employee_insz,
            request_json: &request_json,
            sign_result: &sign_result,
            sent_entries: None,
        },
    )?;

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
