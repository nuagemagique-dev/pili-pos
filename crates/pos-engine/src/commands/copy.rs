use serde::Serialize;

use pos_core::config::PosConfig;
use pos_db::queries::fiscal::FiscalEventRef;
use pos_fiscal::event::build_copy_input;
use pos_fiscal::fdm_client::{SignMutation, SignRequest};
use pos_fiscal::response::{FdmReference, SignResult};
use pos_fiscal::ticket::{
    FiscalTicket, OriginalControlData, ReceiptLanguage, TicketDraft, TicketParams, TicketPayment,
    is_unpaid_sale, printed_payments,
};
use pos_fiscal::types::{EventLabel, EventOperation};

use crate::commands::access::{self, Access};
use crate::commands::delivery::{self, DeliveryOutcome, TicketDeliveryRequest};
use crate::commands::errors::{self, incomplete};
use crate::commands::signing::{
    self, ArchiveSpec, Begin, CopyTicket, Kind, Op, PendingContext, Prepared, StoredDelivery,
};
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

pub async fn do_copy_ticket(
    state: &AppState,
    fiscal_event_id: i64,
    employee_insz: String,
) -> Result<CopyResult, String> {
    do_copy_ticket_with(state, fiscal_event_id, employee_insz, None).await
}

/// Create a copy (event C) of a previous fiscal event.
/// Per SCE 2.0 spec: references the original via fdmId + eventLabel + counters.
///
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
    let op = Op::new(Kind::Copy, fiscal_event_id);
    let guard = match signing::begin(state, &op).await {
        Ok(Begin::Fresh(guard)) => guard,
        // This copy, sent earlier, was signed and has been printed.
        Ok(Begin::AlreadyDone(recovered)) => {
            return Ok(CopyResult {
                success: true,
                error: None,
                ticket: recovered.committed.ticket,
                fiscal_event_id: Some(recovered.committed.event_ids[0]),
                delivery: recovered.follow_up.delivery,
            });
        }
        Err(e) => return Ok(CopyResult::failed(e)),
    };
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
        let bp = super::booking(&db, &guard, &config)?;
        (fdm_ref, event, config, bp)
    };

    // Ticket medium: refused choices stop here, before any signing.
    let resolved = match delivery::resolve(delivery_request, &config, &state.mail) {
        Ok(r) => r,
        Err(e) => return Ok(CopyResult::failed(e)),
    };

    // 2. The copy's number: the C counter advances only when the signed
    // copy is archived, so a refused copy leaves no gap.
    let label = EventLabel::C.in_mode(config.training_mode);
    let ticket_number = super::peek_ticket_number(state, &guard, label)?;

    // 3. Build the copy event
    let mut copy_input = build_copy_input(
        &config,
        &state.identity,
        ticket_number,
        &employee_insz,
        &bp_id,
        &original_ref,
    );
    copy_input.common.ticket_medium = resolved.medium;
    let request = SignRequest::new(SignMutation::Copy, &copy_input, config.training_mode)
        .map_err(errors::internal)?;
    let request_json = serde_json::to_string(&copy_input).map_err(|e| e.to_string())?;

    // The copy reproduces the original ticket and adds the copy's own
    // control data (POS description v2.2 §1.8).
    let ticket = original_ticket(state, &config, &original_event)?;

    let context = PendingContext::Copy {
        delivery: StoredDelivery::of(&resolved),
        ticket,
    };
    let prepared = Prepared {
        op,
        request,
        label,
        first_ticket_no: ticket_number,
        events: vec![ArchiveSpec {
            order_id: None,
            user_insz: employee_insz.clone(),
            request_json,
            sent_entries: None,
        }],
        order_id: None,
        user_insz: employee_insz.clone(),
        context: context.clone(),
    };
    let committed = match signing::sign(state, &guard, &config, prepared).await {
        Ok(committed) => committed,
        Err(failure) => return Ok(CopyResult::failed(failure.error)),
    };
    drop(guard);

    // Print and/or send with no lock held.
    let done = signing::follow_up(state, &config, &context, &committed, false).await;
    Ok(CopyResult {
        success: true,
        error: None,
        fiscal_event_id: Some(committed.event_ids[0]),
        ticket: committed.ticket,
        delivery: done.delivery,
    })
}

/// Signed event `event`'s ticket as it was printed, rebuilt from its
/// archive: the FDM's answer, the signed request, the order's lines
/// and their discounts, the sale a refund refunds. `None` for an event
/// without an order (reports, cash movements); a draft when the FDM's
/// answer is unreadable (never expected).
pub(crate) fn original_ticket(
    state: &AppState,
    config: &PosConfig,
    event: &FiscalEventRef,
) -> Result<Option<CopyTicket>, String> {
    let Some(order_id) = event.order_id else {
        return Ok(None);
    };
    let request: serde_json::Value =
        serde_json::from_str(&event.event_data_json).unwrap_or(serde_json::Value::Null);
    let (order, lines, changes, refunded) = {
        let db = state.db.lock().map_err(errors::internal)?;
        let order = pos_db::queries::orders::get_order(&db, order_id).map_err(errors::internal)?;
        let lines =
            pos_db::queries::orders::get_order_lines(&db, order_id).map_err(errors::internal)?;
        #[cfg(feature = "hubrise")]
        let changes = super::price_changes::load(&db, order_id)?.1;
        #[cfg(not(feature = "hubrise"))]
        let changes = pos_fiscal::price_change::LineChanges::new();
        // A refund names the sale it refunds.
        let refunded = serde_json::from_value::<FdmReference>(request["fdmRef"].clone())
            .ok()
            .map(|fdm_ref| {
                let label = serde_json::to_value(fdm_ref.event_label)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default();
                let short_signature = pos_db::queries::fiscal::find_by_fdm_ref(
                    &db,
                    &fdm_ref.fdm_id,
                    &label,
                    fdm_ref.event_counter,
                    fdm_ref.total_counter,
                )
                .ok()
                .flatten()
                .and_then(|e| e.short_signature);
                OriginalControlData {
                    fdm_ref,
                    short_signature,
                }
            });
        (order, lines, changes, refunded)
    };
    let ticket_lines = pos_fiscal::price_change::ticket_lines(&lines, &changes);
    let total = order.total_cents as f64 / 100.0;
    let operator = super::operator_name(state, &event.user_insz);
    let result: Option<SignResult> = event
        .fdm_response_json
        .as_deref()
        .and_then(|json| serde_json::from_str(json).ok());
    Ok(Some(match result {
        Some(result) => {
            let (payments, rounding) = printed_payments(&request);
            let mut ticket = FiscalTicket::from_sign_result(TicketParams {
                identity: &state.identity,
                result: &result,
                restaurant_name: &config.restaurant_name,
                restaurant_address: &config.restaurant_address,
                restaurant_vat: &config.restaurant_vat,
                restaurant_est_no: &config.restaurant_est_no,
                restaurant_legal_name: config.legal_name(),
                lines: ticket_lines,
                payments,
                total,
                is_training: result.fdm_ref.event_label == EventLabel::T,
                rounding,
                lang: ReceiptLanguage::from_code(&config.language),
                operator: &operator,
            });
            // A sale closed unpaid reprints as unpaid.
            ticket.unpaid = is_unpaid_sale(&request);
            if let Some(refunded) = refunded {
                ticket = ticket.with_original(refunded);
            }
            CopyTicket::Original(Box::new(ticket))
        }
        None => {
            let mut draft = TicketDraft::new(
                config,
                ticket_lines,
                vec![TicketPayment {
                    method: "-".to_string(),
                    amount: total,
                    brand: None,
                }],
                total,
                &operator,
            );
            draft.original = refunded;
            CopyTicket::Draft(Box::new(draft))
        }
    }))
}

/// The ticket of signed sale or copy `fiscal_event_id`, exactly as it
/// was printed: an original that never was is printed from this, never
/// from what the screen sends.
pub(crate) fn printable_original(
    state: &AppState,
    fiscal_event_id: i64,
) -> Result<FiscalTicket, String> {
    let config = state.config.lock().map_err(errors::internal)?.clone();
    let event = {
        let db = state.db.lock().map_err(errors::internal)?;
        pos_db::queries::fiscal::get_fiscal_event(&db, fiscal_event_id)
            .map_err(errors::internal)?
            .ok_or_else(|| errors::EVENT_NOT_FOUND.to_string())?
    };
    let result: SignResult = event
        .fdm_response_json
        .as_deref()
        .and_then(|json| serde_json::from_str(json).ok())
        .ok_or_else(|| incomplete("fdmResponse"))?;
    let unsupported = || errors::PRINT_ORIGINAL_UNSUPPORTED.to_string();
    let sale_ticket = |event: &FiscalEventRef| match original_ticket(state, &config, event)? {
        Some(CopyTicket::Original(ticket)) => Ok(*ticket),
        Some(CopyTicket::Draft(_)) => Err(incomplete("fdmResponse")),
        None => Err(unsupported()),
    };
    match result.event_operation {
        EventOperation::Sale => sale_ticket(&event),
        // A copy: the ticket it copies, with the copy's control data.
        EventOperation::Copy => {
            let request: serde_json::Value =
                serde_json::from_str(&event.event_data_json).map_err(errors::internal)?;
            let fdm_ref: FdmReference = serde_json::from_value(request["fdmRef"].clone())
                .map_err(|_| incomplete("fdmRef"))?;
            let label = serde_json::to_value(fdm_ref.event_label)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default();
            let copied = {
                let db = state.db.lock().map_err(errors::internal)?;
                pos_db::queries::fiscal::find_by_fdm_ref(
                    &db,
                    &fdm_ref.fdm_id,
                    &label,
                    fdm_ref.event_counter,
                    fdm_ref.total_counter,
                )
                .map_err(errors::internal)?
                .ok_or_else(|| errors::EVENT_NOT_FOUND.to_string())?
            };
            Ok(sale_ticket(&copied)?.into_copy(&result))
        }
        _ => Err(unsupported()),
    }
}

/// Recent tickets a customer received, for the copy picker: managers
/// only, like the copy itself.
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
