use serde::{Deserialize, Serialize};
use tauri::State;

use pos_fiscal::fdm_client::{SignMutation, SignRequest};
use pos_fiscal::inputs::*;
use pos_fiscal::types::*;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::commands::signing::{self, ArchiveSpec, Begin, Kind, Op, PendingContext, Prepared};
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialResult {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
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
    // A retry of the same movement, answered late, is not made twice.
    let op = Op::new(
        Kind::Money,
        format!("{employee_insz}|{direction:?}|{amount_cents}|{reason}"),
    );
    let guard = match signing::begin(state, &op).await {
        Ok(Begin::Fresh(guard)) => guard,
        Ok(Begin::AlreadyDone(_)) => return Ok(FinancialResult::done()),
        Err(e) => return Ok(FinancialResult::failed(e)),
    };
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let bp_id = super::current_booking(state, &guard)?;
    let label = EventLabel::F.in_mode(config.training_mode);
    let ticket_number = super::peek_ticket_number(state, &guard, label)?;

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

    let request = SignRequest::new(SignMutation::MoneyInOut, &input, config.training_mode)
        .map_err(errors::internal)?;
    let prepared = prepared(op, request, label, ticket_number, &employee_insz, &input)?;
    Ok(
        match signing::sign(state, &guard, &config, prepared).await {
            Ok(_) => FinancialResult::done(),
            Err(failure) => FinancialResult::failed(failure.error),
        },
    )
}

impl FinancialResult {
    fn done() -> Self {
        Self {
            success: true,
            error: None,
        }
    }

    fn failed(error: String) -> Self {
        Self {
            success: false,
            error: Some(error),
        }
    }
}

fn prepared(
    op: Op,
    request: SignRequest,
    label: EventLabel,
    ticket_number: i64,
    employee_insz: &str,
    input: &impl Serialize,
) -> Result<Prepared, String> {
    let context = match op.kind {
        Kind::Drawer => PendingContext::Drawer,
        _ => PendingContext::Money,
    };
    Ok(Prepared {
        op,
        request,
        label,
        first_ticket_no: ticket_number,
        events: vec![ArchiveSpec {
            order_id: None,
            user_insz: employee_insz.to_string(),
            request_json: serde_json::to_string(input).map_err(|e| e.to_string())?,
            sent_entries: None,
        }],
        order_id: None,
        user_insz: employee_insz.to_string(),
        context,
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
    let op = Op::new(Kind::Drawer, &employee_insz);
    let guard = match signing::begin(state, &op).await {
        Ok(Begin::Fresh(guard)) => guard,
        // Answered late: its follow-up opened the drawer.
        Ok(Begin::AlreadyDone(_)) => return Ok(FinancialResult::done()),
        Err(e) => return Ok(FinancialResult::failed(e)),
    };
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let bp_id = super::current_booking(state, &guard)?;
    let label = EventLabel::F.in_mode(config.training_mode);
    let ticket_number = super::peek_ticket_number(state, &guard, label)?;

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

    let request = SignRequest::new(SignMutation::DrawerOpen, &input, config.training_mode)
        .map_err(errors::internal)?;
    let prepared = prepared(op, request, label, ticket_number, &employee_insz, &input)?;
    let context = prepared.context.clone();
    let committed = match signing::sign(state, &guard, &config, prepared).await {
        Ok(committed) => committed,
        Err(failure) => return Ok(FinancialResult::failed(failure.error)),
    };
    drop(guard);
    // Kick the physical drawer open.
    signing::follow_up(state, &config, &context, &committed, true).await;
    Ok(FinancialResult::done())
}
