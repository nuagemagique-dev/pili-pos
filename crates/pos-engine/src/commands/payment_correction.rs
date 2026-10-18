//! Correcting the payment method of a sale signed today (event F,
//! `signPaymentCorrection`, Basic Set M132): "this was paid by card, not
//! cash". The F event takes the line off its old type (negative PAYMENT
//! line) and puts the same amount on the new type (positive PAYMENT
//! line), and names the corrected sale in `fdmRef`. Nothing is printed
//! (`ticketMedium: NONE`) and the drawer is not opened.
//!
//! Conservative rules (the spec says "transaction de vente déjà
//! clôturée" without limiting it, so the till limits it itself):
//! - a sale N (T in training, and only in training), not a refund, not
//!   closed unpaid, not an online order, not refunded since;
//! - of the current booking period (no Z report covers it yet);
//! - one correction per sale, checked in the archive under the signing
//!   lock;
//! - the corrected line is typed by hand: a card the linked terminal
//!   approved is never turned into another method;
//! - the new method is cash, card or meal voucher, different from the
//!   line's; no ROUNDING line is ever written by a correction, so the
//!   two cases that would need one are refused;
//! - a gift voucher line (`VOUCHER_STORE`) is never corrected and no
//!   line is ever corrected to a gift voucher: the voucher movement
//!   written with the sale cannot follow a correction (INT-D19).

use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;

use pos_core::config::PosConfig;
use pos_db::queries::fiscal::{self, FiscalEventRef};
use pos_fiscal::event::build_payment_correction_input;
use pos_fiscal::fdm_client::{SignMutation, SignRequest};
use pos_fiscal::payment::PaymentMethod;
use pos_fiscal::response::FdmReference;
use pos_fiscal::types::{EventLabel, PaymentType};

use crate::commands::access::{self, Access};
use crate::commands::errors::{self, incomplete};
use crate::commands::financial::FinancialResult;
use crate::commands::signing::{self, ArchiveSpec, Begin, Kind, Op, PendingContext, Prepared};
use crate::state::AppState;

/// One line of the sale's `financials`, as the correction dialog shows it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SalePaymentLine {
    /// Position in the sale's `financials`: the index to correct.
    pub index: i32,
    /// `CASH`, `CARD_DEBIT`, `CHEQUE_MEAL`, ...
    pub payment_type: String,
    pub name: String,
    pub amount_cents: i64,
    /// `MANUAL` or `AUTOMATIC` (a card the linked terminal took).
    pub input_method: String,
    /// `PAYMENT` or `ROUNDING`.
    pub amount_type: String,
    pub brand: Option<String>,
}

/// The correction already made to a sale.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionDone {
    pub from_type: String,
    pub to_type: String,
    pub amount_cents: i64,
    /// When the correction was archived.
    pub at: String,
}

/// A sale's payment lines and whether they can be corrected.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SalePayments {
    pub fiscal_event_id: i64,
    pub ticket_number: i64,
    pub total_cents: i64,
    pub lines: Vec<SalePaymentLine>,
    pub correctable: bool,
    /// Why not, as an error code, when `correctable` is false.
    pub refusal: Option<String>,
    pub corrected: Option<CorrectionDone>,
}

/// A signed sale read from the archive.
struct Sale {
    event: FiscalEventRef,
    request: Value,
    fdm_ref: FdmReference,
}

fn cents(v: &Value) -> i64 {
    (v.as_f64().unwrap_or(0.0) * 100.0).round() as i64
}

/// The archived event `fiscal_event_id` and the FDM reference a
/// correction names.
fn load_sale(db: &Connection, fiscal_event_id: i64) -> Result<Sale, String> {
    let event = fiscal::get_fiscal_event(db, fiscal_event_id)
        .map_err(errors::internal)?
        .ok_or_else(|| errors::EVENT_NOT_FOUND.to_string())?;
    let label = match event.event_label.as_deref() {
        Some("N") => EventLabel::N,
        Some("T") => EventLabel::T,
        _ => return Err(errors::PAYMENT_CORRECTION_NOT_A_SALE.into()),
    };
    let fdm_ref = FdmReference {
        fdm_id: event.fdm_id.clone().ok_or_else(|| incomplete("fdmId"))?,
        fdm_date_time: event
            .fdm_datetime
            .clone()
            .ok_or_else(|| incomplete("fdmDateTime"))?,
        event_label: label,
        event_counter: event
            .event_counter
            .ok_or_else(|| incomplete("eventCounter"))?,
        total_counter: event
            .total_counter
            .ok_or_else(|| incomplete("totalCounter"))?,
    };
    let request: Value = serde_json::from_str(&event.event_data_json).map_err(errors::internal)?;
    Ok(Sale {
        event,
        request,
        fdm_ref,
    })
}

fn label_str(label: EventLabel) -> &'static str {
    match label {
        EventLabel::T => "T",
        _ => "N",
    }
}

/// The archived events that carry `fdm_ref` as their `fdmRef`, among
/// the FDM event operations `operations`: refunds (`SALE`) or
/// corrections (`PAYMENT_CORRECTION`). The operation, not the label,
/// tells them apart: in training mode every event is labelled T.
fn referencing(
    db: &Connection,
    fdm_ref: &FdmReference,
    operations: &[&str],
) -> Result<Vec<(String, String)>, String> {
    let mut stmt = db
        .prepare(
            "SELECT event_data_json, created_at, fdm_response_json FROM fiscal_events
             WHERE json_extract(event_data_json, '$.fdmRef.fdmId') = ?1
               AND json_extract(event_data_json, '$.fdmRef.fdmDateTime') = ?2
               AND json_extract(event_data_json, '$.fdmRef.eventLabel') = ?3
               AND json_extract(event_data_json, '$.fdmRef.eventCounter') = ?4
               AND json_extract(event_data_json, '$.fdmRef.totalCounter') = ?5
             ORDER BY id",
        )
        .map_err(errors::internal)?;
    let rows = stmt
        .query_map(
            rusqlite::params![
                fdm_ref.fdm_id,
                fdm_ref.fdm_date_time,
                label_str(fdm_ref.event_label),
                fdm_ref.event_counter,
                fdm_ref.total_counter,
            ],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .map_err(errors::internal)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(errors::internal)?;
    Ok(rows
        .into_iter()
        .filter(|(_, _, response)| {
            response
                .as_deref()
                .and_then(|j| serde_json::from_str::<Value>(j).ok())
                .and_then(|v| v["eventOperation"].as_str().map(str::to_string))
                .is_some_and(|op| operations.contains(&op.as_str()))
        })
        .map(|(data, at, _)| (data, at))
        .collect())
}

/// The correction already made to the sale, if any.
fn existing_correction(db: &Connection, sale: &Sale) -> Result<Option<CorrectionDone>, String> {
    let found = referencing(db, &sale.fdm_ref, &["PAYMENT_CORRECTION"])?
        .into_iter()
        .find_map(|(json, at)| {
            let v: Value = serde_json::from_str(&json).ok()?;
            let f = v["financials"].as_array()?;
            if f.len() != 2 {
                return None;
            }
            Some(CorrectionDone {
                from_type: f[0]["type"].as_str()?.to_string(),
                to_type: f[1]["type"].as_str()?.to_string(),
                amount_cents: cents(&f[1]["amount"]),
                at,
            })
        });
    Ok(found)
}

fn financials(sale: &Sale) -> &[Value] {
    sale.request["financials"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// Whether the sale's booking period is the open one: no Z covers it.
fn period_is_open(db: &Connection, sale: &Sale) -> Result<bool, String> {
    let Some(id) = sale.request["bookingPeriodId"].as_str() else {
        return Ok(false);
    };
    match db.query_row(
        "SELECT closed_at IS NULL FROM booking_periods WHERE id = ?1",
        [id],
        |r| r.get::<_, bool>(0),
    ) {
        Ok(open) => Ok(open),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(errors::internal(e)),
    }
}

/// The refusals that concern the sale as a whole, in the order the
/// operator should hear about them.
fn sale_refusal(
    db: &Connection,
    config: &PosConfig,
    sale: &Sale,
) -> Result<Option<&'static str>, String> {
    // A sale: signed SALE, not a refund (a refund names its sale in
    // `fdmRef`, and sells a negative total).
    let is_sale = sale.event.fdm_response_json.as_deref().is_some_and(|j| {
        serde_json::from_str::<Value>(j)
            .ok()
            .is_some_and(|v| v["eventOperation"] == "SALE")
    });
    let total = cents(&sale.request["transaction"]["transactionTotal"]);
    if !is_sale || sale.request.get("fdmRef").is_some() || total < 0 {
        return Ok(Some(errors::PAYMENT_CORRECTION_NOT_A_SALE));
    }
    if (sale.fdm_ref.event_label == EventLabel::T) != config.training_mode {
        return Ok(Some(errors::PAYMENT_CORRECTION_TRAINING));
    }
    if !period_is_open(db, sale)? {
        return Ok(Some(errors::PAYMENT_CORRECTION_PERIOD_CLOSED));
    }
    if financials(sale).is_empty() {
        return Ok(Some(errors::PAYMENT_CORRECTION_UNPAID));
    }
    let online_line = financials(sale)
        .iter()
        .any(|l| matches!(l["type"].as_str(), Some("APP" | "ONLINE")));
    let online_order = match sale.event.order_id {
        Some(order_id) => db
            .query_row(
                "SELECT hubrise_order_id IS NOT NULL FROM orders WHERE id = ?1",
                [order_id],
                |r| r.get::<_, bool>(0),
            )
            .unwrap_or(false),
        None => false,
    };
    if online_line || online_order {
        return Ok(Some(errors::PAYMENT_CORRECTION_ONLINE));
    }
    if !referencing(db, &sale.fdm_ref, &["SALE"])?.is_empty() {
        return Ok(Some(errors::PAYMENT_CORRECTION_REFUNDED));
    }
    if existing_correction(db, sale)?.is_some() {
        return Ok(Some(errors::PAYMENT_CORRECTION_ALREADY_DONE));
    }
    Ok(None)
}

/// Whether a `financials` line may be corrected at all: a PAYMENT line
/// typed by hand that did not debit a gift voucher.
fn is_correctable_line(line: &Value) -> bool {
    line["amountType"] == "PAYMENT"
        && line["inputMethod"] == "MANUAL"
        && !is_gift_voucher_line(line)
}

/// A payment line that debited a gift voucher.
fn is_gift_voucher_line(line: &Value) -> bool {
    line["type"] == "VOUCHER_STORE"
}

/// The signed sale sold a gift voucher (a line of department `GV`).
fn sells_gift_voucher(request: &Value) -> bool {
    request["transaction"]["transactionLines"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|l| l["mainProduct"]["departmentId"] == pos_db::queries::gift_vouchers::DEPARTMENT_ID)
}

/// The payment lines of the signed sale `fiscal_event_id` and whether
/// (and why not) one can be corrected. Managers only.
pub fn do_list_sale_payments(
    state: &AppState,
    fiscal_event_id: i64,
) -> Result<SalePayments, String> {
    access::operator(state, Access::Admin)?;
    let config = state.config.lock().map_err(errors::internal)?.clone();
    let db = state.db.lock().map_err(errors::internal)?;
    let sale = load_sale(&db, fiscal_event_id)?;
    let lines: Vec<SalePaymentLine> = financials(&sale)
        .iter()
        .enumerate()
        .map(|(i, l)| SalePaymentLine {
            index: i as i32,
            payment_type: l["type"].as_str().unwrap_or_default().to_string(),
            name: l["name"].as_str().unwrap_or_default().to_string(),
            amount_cents: cents(&l["amount"]),
            input_method: l["inputMethod"].as_str().unwrap_or_default().to_string(),
            amount_type: l["amountType"].as_str().unwrap_or_default().to_string(),
            brand: l["provider"].as_str().map(str::to_string),
        })
        .collect();
    let mut refusal = sale_refusal(&db, &config, &sale)?.map(str::to_string);
    // Every payment is a terminal card or a gift voucher: nothing to pick.
    if refusal.is_none() && !financials(&sale).iter().any(is_correctable_line) {
        let only_vouchers = financials(&sale)
            .iter()
            .filter(|l| l["amountType"] == "PAYMENT")
            .all(is_gift_voucher_line);
        refusal = Some(if only_vouchers {
            errors::PAYMENT_CORRECTION_GIFT_VOUCHER.to_string()
        } else {
            errors::PAYMENT_CORRECTION_TERMINAL_CARD.to_string()
        });
    }
    // The correction a sale already has is shown whatever else holds.
    let corrected = existing_correction(&db, &sale)?;
    Ok(SalePayments {
        fiscal_event_id,
        ticket_number: sale.request["posFiscalTicketNo"].as_i64().unwrap_or(0),
        total_cents: cents(&sale.request["transaction"]["transactionTotal"]),
        lines,
        correctable: refusal.is_none(),
        refusal,
        corrected,
    })
}

/// Correct the method of payment line `line_index` of sale
/// `fiscal_event_id` to `method`, in the name of the logged-in manager
/// `insz`, confirmed by `pin`.
///
/// Refusals come back as `success: false` with their error code,
/// nothing signed. A retry whose first answer came late returns
/// success without signing twice.
pub async fn do_correct_payment(
    state: &AppState,
    fiscal_event_id: i64,
    line_index: i32,
    method: PaymentMethod,
    insz: String,
    pin: String,
) -> Result<FinancialResult, String> {
    let manager = match access::confirmed_operator(state, &insz, &pin, Access::Admin) {
        Ok(user) => user,
        Err(e) => return Ok(FinancialResult::failed(e)),
    };
    if !method.is_offered_at_till() || method == PaymentMethod::GiftVoucher {
        return Ok(FinancialResult::failed(
            errors::PAYMENT_CORRECTION_METHOD.into(),
        ));
    }
    // The line and the method are part of the subject: a retry that
    // asks for another correction is a new request, not the answered one.
    let op = Op::new(
        Kind::PaymentCorrection,
        format!("{fiscal_event_id}:{line_index}:{method:?}"),
    );
    let guard = match signing::begin(state, &op).await {
        Ok(Begin::Fresh(guard)) => guard,
        Ok(Begin::AlreadyDone(_)) => return Ok(FinancialResult::done()),
        Err(e) => return Ok(FinancialResult::failed(e)),
    };
    match sign_correction(
        state,
        &guard,
        op,
        fiscal_event_id,
        line_index,
        method,
        &manager.insz,
    )
    .await
    {
        Ok(()) => Ok(FinancialResult::done()),
        Err(e) => Ok(FinancialResult::failed(e)),
    }
}

/// Everything under the signing lock: the checks, the build, the sign.
async fn sign_correction(
    state: &AppState,
    guard: &signing::SignGuard<'_>,
    op: Op,
    fiscal_event_id: i64,
    line_index: i32,
    method: PaymentMethod,
    manager_insz: &str,
) -> Result<(), String> {
    let config = state.config.lock().map_err(errors::internal)?.clone();
    let (sale, booking) = {
        let db = state.db.lock().map_err(errors::internal)?;
        let sale = load_sale(&db, fiscal_event_id)?;
        if let Some(code) = sale_refusal(&db, &config, &sale)? {
            return Err(code.to_string());
        }
        let booking = super::booking(&db, guard, &config)?;
        // The open period is the sale's: a Z made while the dialog was
        // open must not let the correction through.
        if sale.request["bookingPeriodId"].as_str() != Some(booking.period_id.as_str()) {
            return Err(errors::PAYMENT_CORRECTION_PERIOD_CLOSED.into());
        }
        (sale, booking)
    };

    let line = usize::try_from(line_index)
        .ok()
        .and_then(|i| financials(&sale).get(i))
        .filter(|l| l["amountType"] == "PAYMENT")
        .ok_or_else(|| errors::PAYMENT_CORRECTION_BAD_LINE.to_string())?;
    if line["inputMethod"] == "AUTOMATIC" {
        return Err(errors::PAYMENT_CORRECTION_TERMINAL_CARD.into());
    }
    if is_gift_voucher_line(line) {
        return Err(errors::PAYMENT_CORRECTION_GIFT_VOUCHER.into());
    }
    let from: PaymentType = serde_json::from_value(line["type"].clone())
        .map_err(|_| errors::PAYMENT_CORRECTION_BAD_LINE.to_string())?;
    let amount_cents = cents(&line["amount"]);
    if amount_cents <= 0 {
        return Err(errors::PAYMENT_CORRECTION_BAD_LINE.into());
    }
    let to = method.payment_type();
    if from == to {
        return Err(errors::PAYMENT_CORRECTION_SAME_METHOD.into());
    }
    // A sale that sold a gift voucher is paid in cash or by card only (V12-D27): a
    // correction keeps that rule.
    if method == PaymentMethod::MealVoucher && sells_gift_voucher(&sale.request) {
        return Err(errors::GIFT_VOUCHER_PAYMENT_INVALID.into());
    }
    let sale_is_rounded = financials(&sale)
        .iter()
        .any(|l| l["amountType"] == "ROUNDING");
    if (from == PaymentType::Cash && sale_is_rounded)
        || (to == PaymentType::Cash && amount_cents % 5 != 0)
    {
        return Err(errors::PAYMENT_CORRECTION_ROUNDING.into());
    }

    let label = EventLabel::F.in_mode(config.training_mode);
    let ticket_number = super::peek_ticket_number(state, guard, label)?;
    let input = build_payment_correction_input(
        &config,
        &state.identity,
        ticket_number,
        manager_insz,
        &booking,
        &sale.fdm_ref,
        from,
        to,
        amount_cents,
    );
    let request = SignRequest::new(
        SignMutation::PaymentCorrection,
        &input,
        config.training_mode,
    )
    .map_err(errors::internal)?;
    let prepared = Prepared {
        op,
        request,
        label,
        first_ticket_no: ticket_number,
        events: vec![ArchiveSpec {
            order_id: None,
            user_insz: manager_insz.to_string(),
            request_json: serde_json::to_string(&input).map_err(errors::internal)?,
            sent_entries: None,
        }],
        order_id: None,
        user_insz: manager_insz.to_string(),
        context: PendingContext::PaymentCorrection {
            corrected_event_id: fiscal_event_id,
        },
    };
    signing::sign(state, guard, &config, prepared)
        .await
        .map_err(|f| f.error)?;
    Ok(())
}
