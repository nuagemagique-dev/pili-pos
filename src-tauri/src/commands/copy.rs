use serde::Serialize;
use tauri::State;

use pos_fiscal::event::build_copy_input;
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::response::{FdmReference, SignResult};
use pos_fiscal::ticket::{
    FiscalTicket, ReceiptLanguage, TicketLine, TicketParams, TicketPayment, printed_payments,
};
use pos_fiscal::types::EventLabel;

use crate::commands::access::{self, Access};
use crate::commands::delivery::{self, DeliveryOutcome, TicketDeliveryRequest};
use crate::commands::errors::{self, incomplete};
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyResult {
    pub success: bool,
    pub error: Option<String>,
    /// The copy ticket, when the original event had an order to show.
    pub ticket: Option<FiscalTicket>,
    /// Archive id of the signed C event.
    pub fiscal_event_id: Option<i64>,
    /// How the copy reached the customer (paper and/or email).
    pub delivery: Option<DeliveryOutcome>,
}

impl CopyResult {
    fn failed(error: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(error.into()),
            ticket: None,
            fiscal_event_id: None,
            delivery: None,
        }
    }
}

/// Create a copy (event C) of a previous fiscal event.
/// Per SCE 2.0 spec: references the original via fdmId + eventLabel + counters.
#[tauri::command]
pub async fn copy_ticket(
    state: State<'_, AppState>,
    fiscal_event_id: i64,
    employee_insz: String,
    delivery: Option<TicketDeliveryRequest>,
) -> Result<CopyResult, String> {
    do_copy_ticket_with(
        state.inner(),
        fiscal_event_id,
        employee_insz,
        delivery.as_ref(),
    )
    .await
}

pub async fn do_copy_ticket(
    state: &AppState,
    fiscal_event_id: i64,
    employee_insz: String,
) -> Result<CopyResult, String> {
    do_copy_ticket_with(state, fiscal_event_id, employee_insz, None).await
}

/// [`do_copy_ticket`] with the operator's ticket choice.
pub async fn do_copy_ticket_with(
    state: &AppState,
    fiscal_event_id: i64,
    employee_insz: String,
    delivery_request: Option<&TicketDeliveryRequest>,
) -> Result<CopyResult, String> {
    if let Err(e) = access::operator_named(state, &employee_insz, Access::Admin) {
        return Ok(CopyResult::failed(e));
    }
    // 1. Load the original event
    let (original_ref, original_event, config, bp_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let event = pos_db::queries::fiscal::get_fiscal_event(&db, fiscal_event_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| errors::EVENT_NOT_FOUND.to_string())?;

        let fdm_id = event.fdm_id.clone().ok_or_else(|| incomplete("fdmId"))?;
        let fdm_datetime = event
            .fdm_datetime
            .clone()
            .ok_or_else(|| incomplete("fdmDateTime"))?;
        let event_label_str = event
            .event_label
            .clone()
            .ok_or_else(|| incomplete("eventLabel"))?;
        let event_counter = event
            .event_counter
            .ok_or_else(|| incomplete("eventCounter"))?;
        let total_counter = event
            .total_counter
            .ok_or_else(|| incomplete("totalCounter"))?;

        let label = match event_label_str.as_str() {
            "N" => EventLabel::N,
            "P" => EventLabel::P,
            "F" => EventLabel::F,
            "S" => EventLabel::S,
            "I" => EventLabel::I,
            "R" => EventLabel::R,
            "C" => EventLabel::C,
            "T" => EventLabel::T,
            _ => {
                return Err(errors::coded(
                    errors::EVENT_LABEL_UNKNOWN,
                    &[("label", &event_label_str)],
                ));
            }
        };

        let fdm_ref = FdmReference {
            fdm_id,
            fdm_date_time: fdm_datetime,
            event_label: label,
            event_counter,
            total_counter,
        };

        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = super::booking(&db, &config)?;
        (fdm_ref, event, config, bp)
    };

    // Ticket medium: refused choices stop here, before any signing.
    let resolved = match delivery::resolve(delivery_request, &config, &state.mail) {
        Ok(r) => r,
        Err(e) => return Ok(CopyResult::failed(e)),
    };

    // The original order, for the copy's lines. Events without an
    // order (reports, cash movements) have no ticket body to show.
    let original_order = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        original_event.order_id.and_then(|oid| {
            let order = pos_db::queries::orders::get_order(&db, oid).ok()?;
            let lines = pos_db::queries::orders::get_order_lines(&db, oid).ok()?;
            Some((order, lines))
        })
    };
    let operator = super::operator_name(state, &employee_insz);

    // 2. RESERVE (peek, don't commit) the sequence number for the copy
    // event. Same sign-then-commit pattern as every other fiscal
    // command (see `commands/mod.rs::commit_signed_event`): the C
    // counter only advances once the FDM has signed, so a refused or
    // unreachable FDM leaves no gap in the C sequence.
    let label = EventLabel::C.in_mode(config.training_mode);
    let ticket_number = super::peek_ticket_number(state, label)?;

    // 3. Build and send copy event
    let mut copy_input = build_copy_input(
        &config,
        ticket_number,
        &employee_insz,
        &bp_id,
        &original_ref,
    );
    copy_input.common.ticket_medium = resolved.medium;
    let request_json = serde_json::to_string(&copy_input).map_err(|e| e.to_string())?;

    let fdm = FdmClient::new(&config.fdm_url);
    let result = match fdm.sign_copy(copy_input, config.training_mode).await {
        Ok(result) => result,
        Err(e) => return Ok(CopyResult::failed(format!("FDM error: {e}"))),
    };

    // The copy reproduces the original ticket, rebuilt from its
    // archive (the FDM's response and the signed request), and adds
    // the copy's own control data (POS description v2.2 §1.8).
    let lang = ReceiptLanguage::from_code(&config.language);
    let original_result: Option<SignResult> = original_event
        .fdm_response_json
        .as_deref()
        .and_then(|json| serde_json::from_str(json).ok());
    let original_operator = super::operator_name(state, &original_event.user_insz);
    let ticket = original_order.map(|(order, lines)| {
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
        let total = order.total_cents as f64 / 100.0;
        match original_result.as_ref() {
            Some(original) => {
                let request = serde_json::from_str(&original_event.event_data_json)
                    .unwrap_or(serde_json::Value::Null);
                let (payments, rounding) = printed_payments(&request);
                FiscalTicket::from_sign_result(TicketParams {
                    result: original,
                    restaurant_name: &config.restaurant_name,
                    restaurant_address: &config.restaurant_address,
                    restaurant_vat: &config.restaurant_vat,
                    restaurant_est_no: &config.restaurant_est_no,
                    restaurant_legal_name: config.legal_name(),
                    lines: ticket_lines,
                    payments,
                    total,
                    is_training: original.fdm_ref.event_label == EventLabel::T,
                    rounding,
                    lang,
                    operator: &original_operator,
                })
                .into_copy(&result)
            }
            // No readable archive of the original (never expected: the
            // FDM response is archived with every signed event): the
            // copy's own data, as a last resort.
            None => FiscalTicket::from_sign_result(TicketParams {
                result: &result,
                restaurant_name: &config.restaurant_name,
                restaurant_address: &config.restaurant_address,
                restaurant_vat: &config.restaurant_vat,
                restaurant_est_no: &config.restaurant_est_no,
                restaurant_legal_name: config.legal_name(),
                lines: ticket_lines,
                payments: vec![TicketPayment {
                    method: "-".to_string(),
                    amount: total,
                }],
                total,
                is_training: config.training_mode,
                rounding: None,
                lang,
                operator: &operator,
            }),
        }
    });

    // Commit the counter + archive + the email row atomically, then
    // print/send with no lock held.
    let (event_id, delivery_id) = super::commit_signed_event_then(
        state,
        &config,
        super::SignedEvent {
            label,
            ticket_number,
            order_id: None,
            user_insz: &employee_insz,
            request_json: &request_json,
            sign_result: &result,
            sent_entries: None,
        },
        |tx, event_id| delivery::enqueue_in_tx(tx, event_id, &resolved, ticket.as_ref()),
    )?;

    let outcome = delivery::deliver(
        state,
        &config,
        event_id,
        &resolved,
        delivery_id,
        ticket.as_ref(),
    )
    .await;

    Ok(CopyResult {
        success: true,
        error: None,
        ticket,
        fiscal_event_id: Some(event_id),
        delivery: Some(outcome),
    })
}

/// Recent tickets a customer received, for the copy picker: managers
/// only, like the copy itself.
#[tauri::command]
pub fn list_copyable_tickets(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<pos_db::queries::fiscal::CopyableTicket>, String> {
    do_list_copyable_tickets(state.inner(), limit)
}

pub fn do_list_copyable_tickets(
    state: &AppState,
    limit: Option<i64>,
) -> Result<Vec<pos_db::queries::fiscal::CopyableTicket>, String> {
    access::operator(state, Access::Admin)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::fiscal::list_copyable_tickets(&db, limit.unwrap_or(30))
        .map_err(|e| e.to_string())
}

/// List recent fiscal events.
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
