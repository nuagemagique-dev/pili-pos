use serde::Deserialize;
use tauri::State;

use pos_db::queries::fiscal::{self, RefundableEvent};
use pos_fiscal::event::build_sale_input;
use pos_fiscal::fdm_client::{SignMutation, SignRequest};
use pos_fiscal::response::FdmReference;
use pos_fiscal::ticket::{OriginalControlData, TicketDraft, TicketPayment};
use pos_fiscal::types::{EventLabel, PaymentType};

use crate::commands::access::{self, Access};
use crate::commands::checkout::CheckoutResult;
use crate::commands::delivery::{self, TicketDeliveryRequest};
use crate::commands::errors::{self, incomplete};
use crate::commands::signing::{
    self, ArchiveSpec, Begin, Kind, Op, PendingContext, Prepared, StoredDelivery,
};
use crate::state::AppState;

/// One item to refund (sent from the frontend). Price, VAT label,
/// name and department are taken from the original sale's line, never
/// from the frontend.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundItem {
    pub menu_item_id: i64,
    pub quantity: i32, // positive - backend negates it
    /// The original sale's line. Needed when the product was sold at
    /// more than one price.
    #[serde(default)]
    pub line_id: Option<i64>,
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
    delivery: Option<TicketDeliveryRequest>,
) -> Result<CheckoutResult, String> {
    do_refund_checkout_with(
        state.inner(),
        original_fiscal_event_id,
        items,
        employee_insz,
        delivery.as_ref(),
    )
    .await
}

pub async fn do_refund_checkout(
    state: &AppState,
    original_fiscal_event_id: i64,
    items: Vec<RefundItem>,
    employee_insz: String,
) -> Result<CheckoutResult, String> {
    do_refund_checkout_with(state, original_fiscal_event_id, items, employee_insz, None).await
}

/// [`do_refund_checkout`] with the operator's ticket choice.
pub async fn do_refund_checkout_with(
    state: &AppState,
    original_fiscal_event_id: i64,
    items: Vec<RefundItem>,
    employee_insz: String,
    delivery_request: Option<&TicketDeliveryRequest>,
) -> Result<CheckoutResult, String> {
    if let Err(e) = access::operator_named(state, &employee_insz, Access::Admin) {
        return Ok(CheckoutResult::refused(e));
    }
    if items.is_empty() {
        return Ok(CheckoutResult {
            success: false,
            ticket: None,
            error: Some(errors::NOTHING_TO_REFUND.into()),
            fdm_connected: false,
            fiscal_event_id: None,
            delivery: None,
            kitchen_slip: None,
        });
    }
    refund_sale(
        state,
        original_fiscal_event_id,
        &employee_insz,
        delivery_request,
        RefundSpec::cash(RefundLines::Products(&items)),
    )
    .await
}

/// What a refund gives back.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RefundLines<'a> {
    /// The products the operator picked, at the price they were sold.
    Products(&'a [RefundItem]),
    /// Every line of the sale, each at its own price, charges included
    /// (the whole sale of a cancelled online order). Refused once any
    /// part of the sale was refunded.
    #[cfg(feature = "hubrise")]
    Everything,
}

/// How a refund is made: which lines, and how the money goes back.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RefundSpec<'a> {
    pub lines: RefundLines<'a>,
    /// Cash at the till; APP or ONLINE when the platform or the
    /// webshop's payment provider pays the customer back.
    pub payment_type: PaymentType,
    /// Cash leaves the drawer: open it.
    pub open_drawer: bool,
}

impl<'a> RefundSpec<'a> {
    pub fn cash(lines: RefundLines<'a>) -> Self {
        Self {
            lines,
            payment_type: PaymentType::Cash,
            open_drawer: true,
        }
    }
}

/// Sign the refund of the sale `original_fiscal_event_id` in
/// `employee_insz`'s name (already checked by the caller).
pub(crate) async fn refund_sale(
    state: &AppState,
    original_fiscal_event_id: i64,
    employee_insz: &str,
    delivery_request: Option<&TicketDeliveryRequest>,
    spec: RefundSpec<'_>,
) -> Result<CheckoutResult, String> {
    let employee_insz = employee_insz.to_owned();
    // A retry for the same sale, answered late, returns that refund:
    // the sale is never refunded twice by one request.
    let op = Op::new(Kind::Refund, original_fiscal_event_id);
    let guard = match signing::begin(state, &op).await {
        Ok(Begin::Fresh(guard)) => guard,
        Ok(Begin::AlreadyDone(recovered)) => {
            let committed = recovered.committed;
            return Ok(CheckoutResult {
                success: true,
                ticket: committed.ticket,
                error: None,
                fdm_connected: true,
                fiscal_event_id: Some(committed.event_ids[0]),
                delivery: recovered.follow_up.delivery,
                kitchen_slip: None,
            });
        }
        Err(e) => return Ok(CheckoutResult::refused(e)),
    };

    // 1. Load original fiscal event → extract FdmReference
    // Without HubRise no sale has discounts to check.
    #[cfg_attr(not(feature = "hubrise"), allow(unused_variables))]
    let (original_ref, original_signature, original_lines, original_changes, config, bp_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let event = fiscal::get_fiscal_event(&db, original_fiscal_event_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| errors::EVENT_NOT_FOUND.to_string())?;
        let original_order_id = event.order_id.ok_or_else(|| incomplete("order"))?;
        let original_lines = pos_db::queries::orders::get_order_lines(&db, original_order_id)
            .map_err(|e| e.to_string())?;
        // The discounts of each original line (an online order's).
        #[cfg(feature = "hubrise")]
        let original_changes = super::price_changes::load(&db, original_order_id)?.1;
        #[cfg(not(feature = "hubrise"))]
        let original_changes = pos_fiscal::price_change::LineChanges::new();

        // A sale closed unpaid took no money: none goes back.
        let request: serde_json::Value =
            serde_json::from_str(&event.event_data_json).unwrap_or_default();
        if pos_fiscal::ticket::is_unpaid_sale(&request) {
            return Ok(CheckoutResult::refused(errors::REFUND_UNPAID_SALE));
        }

        let original_signature = event.short_signature.clone();
        let fdm_id = event.fdm_id.ok_or_else(|| incomplete("fdmId"))?;
        let fdm_datetime = event
            .fdm_datetime
            .ok_or_else(|| incomplete("fdmDateTime"))?;
        let event_label_str = event.event_label.ok_or_else(|| incomplete("eventLabel"))?;
        let event_counter = event
            .event_counter
            .ok_or_else(|| incomplete("eventCounter"))?;
        let total_counter = event
            .total_counter
            .ok_or_else(|| incomplete("totalCounter"))?;

        let label = match event_label_str.as_str() {
            "N" => EventLabel::N,
            other => {
                return Err(errors::coded(errors::NOT_REFUNDABLE, &[("label", &other)]));
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
        (
            fdm_ref,
            original_signature,
            original_lines,
            original_changes,
            config,
            bp,
        )
    };
    // Training never touches real sales: a training refund would count
    // against what the real sale may still refund.
    if config.training_mode {
        return Ok(CheckoutResult::refused(ERR_REFUND_IN_TRAINING));
    }
    // A refund is refunded by selling again, never by a refund of it.
    if original_lines
        .iter()
        .map(|l| i64::from(l.quantity))
        .sum::<i64>()
        < 0
    {
        return Ok(CheckoutResult::refused(ERR_REFUND_OF_REFUND));
    }

    // Ticket medium: refused choices stop here, before anything is
    // created or signed.
    let resolved = match delivery::resolve(delivery_request, &config, &state.mail) {
        Ok(r) => r,
        Err(e) => return Ok(CheckoutResult::refused(e)),
    };

    // 2. Create a refund order with negative-quantity lines - once
    // every item is checked, so a refused refund leaves nothing behind.
    let (refund_order, refund_lines, refund_changes) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        // Every refund of this sale - this one included - carries the
        // SAME `fdmRef` (the original ticket, never a chain through
        // prior refunds; see FLUX_REMBOURSEMENT_FR.md and the M112B
        // test). That makes `fdmRef` the stable key for "how much of
        // this sale has already been refunded", across any number of
        // prior partial refunds, so a second or third partial refund
        // can't push the aggregate past what was actually sold.
        let already_refunded =
            already_refunded_quantities(&db, &original_ref).map_err(|e| e.to_string())?;

        // (quantity refunded, the original line it comes from)
        let mut refunded: Vec<(i32, &pos_core::order::OrderLine)> = Vec::new();
        match spec.lines {
            RefundLines::Products(items) => {
                let mut requested: std::collections::HashMap<Bucket, i64> =
                    std::collections::HashMap::new();
                let mut picked = Vec::with_capacity(items.len());
                for item in items {
                    if item.quantity <= 0 {
                        return Err(errors::coded(
                            errors::REFUND_QUANTITY_POSITIVE,
                            &[("product", &item.menu_item_id)],
                        ));
                    }
                    let original = original_line(&original_lines, item)?;
                    let total = requested.entry(Bucket::of(original)).or_default();
                    // i64 sums of i32 quantities: past i64, past any sale.
                    *total = total
                        .checked_add(i64::from(item.quantity))
                        .ok_or_else(|| exceeds_sold(item.menu_item_id, i64::MAX, 0, 0))?;
                    picked.push((item.quantity, original));
                }
                for (bucket, &quantity) in &requested {
                    let prior = already_refunded.get(bucket).copied().unwrap_or(0);
                    check_sold(&original_lines, bucket, quantity, prior)?;
                }
                for (quantity, original) in picked {
                    // A discount is spread over its lines: a part of
                    // one would refund a share nobody can tell.
                    #[cfg(feature = "hubrise")]
                    if original_changes.contains_key(&original.id) {
                        return Err(errors::coded(
                            errors::hubrise::REFUND_DISCOUNTED_LINE,
                            &[("product", &original.menu_item_id)],
                        ));
                    }
                    refunded.push((quantity, original));
                }
            }
            #[cfg(feature = "hubrise")]
            RefundLines::Everything => {
                if let Some((bucket, &prior)) = already_refunded.iter().next() {
                    return Err(exceeds_sold(bucket.product, 0, 0, prior));
                }
                for line in original_lines.iter().filter(|l| l.quantity > 0) {
                    refunded.push((line.quantity, line));
                }
                if refunded.is_empty() {
                    return Err(errors::NOTHING_TO_REFUND.to_string());
                }
            }
        }
        let order = pos_db::queries::orders::create_order(
            &db,
            pos_core::order::OrderType::Takeaway,
            None,
            &employee_insz,
        )
        .map_err(|e| e.to_string())?;
        for (quantity, original) in refunded {
            let _written = pos_db::queries::orders::add_order_line(
                &db,
                pos_db::queries::orders::NewOrderLine {
                    order_id: order.id,
                    menu_item_id: original.menu_item_id,
                    quantity: -quantity, // NEGATE for refund
                    unit_price_cents: original.unit_price_cents,
                    vat_code: original.vat_code,
                    name: &original.name,
                    modifiers_json: None,
                    department_id: &original.department_id,
                    department_name: &original.department_name,
                },
            )
            .map_err(|e| e.to_string())?;
            // The whole line goes back: its discounts too, negated.
            #[cfg(feature = "hubrise")]
            if let Some(changes) = original_changes.get(&original.id) {
                let negated: Vec<_> = changes.iter().map(|c| c.negated()).collect();
                super::price_changes::record(&db, _written.id, &negated)?;
            }
        }

        let updated_order =
            pos_db::queries::orders::get_order(&db, order.id).map_err(|e| e.to_string())?;
        let lines =
            pos_db::queries::orders::get_order_lines(&db, order.id).map_err(|e| e.to_string())?;
        // One entry per refund line: by line is what the FDM receives.
        #[cfg(feature = "hubrise")]
        let changes = super::price_changes::load(&db, order.id)?.1;
        #[cfg(not(feature = "hubrise"))]
        let changes = pos_fiscal::price_change::LineChanges::new();
        (updated_order, lines, changes)
    };

    // From here on the refund order exists: whatever stops the refund
    // (FDM refusal, archive failure) voids it, so it never lingers as
    // an open order that could be completed and paid as a sale. A
    // refund signed on a later send closes it all the same.
    let signed: Result<Result<_, CheckoutResult>, String> = async {
        // The number this refund takes; the counter advances only when
        // the signed refund is archived.
        let label = if config.training_mode {
            EventLabel::T
        } else {
            EventLabel::N
        };
        let ticket_number = {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            signing::peek(&db, &guard, label)?
        };

        // SaleInput with negative lines + original fdm_ref
        let mut sale_input = build_sale_input(
            &config,
            &refund_order,
            &refund_lines,
            ticket_number,
            &bp_id,
            spec.payment_type,
            refund_order.total_cents, // negative
            0,                        // refunds are not rounded
            Some(&original_ref),
        );
        pos_fiscal::price_change::apply(
            &mut sale_input.transaction,
            &refund_lines,
            &refund_changes,
        );
        sale_input.common.ticket_medium = resolved.medium;
        let request = SignRequest::new(SignMutation::Sale, &sale_input, config.training_mode)
            .map_err(errors::internal)?;
        let request_json = serde_json::to_string(&sale_input).map_err(|e| e.to_string())?;

        let operator = super::operator_name(state, &refund_order.user_insz);
        let mut ticket = TicketDraft::new(
            &config,
            pos_fiscal::price_change::ticket_lines(&refund_lines, &refund_changes),
            vec![TicketPayment {
                method: pos_fiscal::payment::payment_line_identity(spec.payment_type)
                    .1
                    .to_string(),
                amount: refund_order.total_cents as f64 / 100.0,
            }],
            refund_order.total_cents as f64 / 100.0,
            &operator,
        );
        ticket.original = Some(OriginalControlData {
            fdm_ref: original_ref.clone(),
            short_signature: original_signature.clone(),
        });

        let context = PendingContext::Refund {
            refund_order_id: refund_order.id,
            training: label == EventLabel::T,
            // Refunds go back in cash, or through the platform.
            method: if spec.payment_type == PaymentType::Cash {
                "cash"
            } else {
                "other"
            }
            .to_string(),
            total_cents: refund_order.total_cents,
            open_drawer: spec.open_drawer,
            delivery: StoredDelivery::of(&resolved),
            ticket,
            webshop_order_id: original_order_id_if_online(state, original_fiscal_event_id),
        };
        let prepared = Prepared {
            op,
            request,
            label,
            first_ticket_no: ticket_number,
            events: vec![ArchiveSpec {
                order_id: Some(refund_order.id),
                user_insz: employee_insz.clone(),
                request_json,
                sent_entries: Some((refund_order.id, i64::MAX)),
            }],
            order_id: Some(refund_order.id),
            user_insz: employee_insz.clone(),
            context: context.clone(),
        };
        Ok(
            match signing::sign(state, &guard, &config, prepared).await {
                Ok(committed) => Ok((context, committed)),
                // No FDM, no sale: the POS blocks (SCE 2.0). There is no
                // offline mode, in any build.
                Err(failure) if failure.not_connected => Err(CheckoutResult {
                    success: false,
                    ticket: None,
                    error: Some("FDM not connected - cannot process refund".into()),
                    fdm_connected: false,
                    fiscal_event_id: None,
                    delivery: None,
                    kitchen_slip: None,
                }),
                Err(failure) => Err(CheckoutResult {
                    success: false,
                    ticket: None,
                    error: Some(failure.error),
                    fdm_connected: true,
                    fiscal_event_id: None,
                    delivery: None,
                    kitchen_slip: None,
                }),
            },
        )
    }
    .await;
    drop(guard);
    let (context, committed) = match signed {
        Ok(Ok(signed)) => signed,
        Ok(Err(refused)) => {
            void_order(state, refund_order.id);
            return Ok(refused);
        }
        Err(e) => {
            void_order(state, refund_order.id);
            return Err(e);
        }
    };

    // Print and/or email the REFUND ticket, then open the drawer.
    let done = signing::follow_up(state, &config, &context, &committed, true).await;
    Ok(CheckoutResult {
        success: true,
        ticket: committed.ticket,
        error: None,
        fdm_connected: true,
        fiscal_event_id: Some(committed.event_ids[0]),
        delivery: done.delivery,
        kitchen_slip: None,
    })
}

/// The online order a sale belongs to, if any.
fn original_order_id_if_online(state: &AppState, sale_event_id: i64) -> Option<i64> {
    #[cfg(feature = "hubrise")]
    {
        let db = state.db.lock().ok()?;
        db.query_row(
            "SELECT w.order_id FROM fiscal_events e JOIN webshop_orders w ON w.order_id = e.order_id
             WHERE e.id = ?1",
            rusqlite::params![sale_event_id],
            |r| r.get(0),
        )
        .ok()
    }
    #[cfg(not(feature = "hubrise"))]
    {
        let _ = (state, sale_event_id);
        None
    }
}

/// Void a refund order that was not signed.
fn void_order(state: &AppState, order_id: i64) {
    if let Ok(db) = state.db.lock() {
        let _ = db.execute(
            "UPDATE orders SET status = 'voided' WHERE id = ?1 AND status = 'open'",
            rusqlite::params![order_id],
        );
    }
}

pub const ERR_REFUND_IN_TRAINING: &str = errors::REFUND_IN_TRAINING;
pub const ERR_REFUND_OF_REFUND: &str = errors::REFUND_OF_REFUND;

/// Lines of one product sold at one price and VAT label: what a
/// refund is counted against.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Bucket {
    product: i64,
    unit_price_cents: i64,
    vat: String,
}

impl Bucket {
    fn of(line: &pos_core::order::OrderLine) -> Self {
        Self {
            product: line.menu_item_id,
            unit_price_cents: line.unit_price_cents,
            vat: format!("{:?}", line.vat_code),
        }
    }
}

fn exceeds_sold(product: i64, requested: i64, sold: i64, refunded: i64) -> String {
    errors::coded(
        errors::REFUND_EXCEEDS_SOLD,
        &[
            ("product", &product),
            ("requested", &requested),
            ("sold", &sold),
            ("refunded", &refunded),
        ],
    )
}

/// The original sale line `item` refunds: the line it names, else the
/// product's line when it was sold at one price only.
fn original_line<'a>(
    original_lines: &'a [pos_core::order::OrderLine],
    item: &RefundItem,
) -> Result<&'a pos_core::order::OrderLine, String> {
    let not_sold = || {
        errors::coded(
            errors::REFUND_PRODUCT_NOT_SOLD,
            &[("product", &item.menu_item_id)],
        )
    };
    let mut lines = original_lines
        .iter()
        .filter(|l| l.menu_item_id == item.menu_item_id && l.quantity > 0);
    if let Some(line_id) = item.line_id {
        return lines.find(|l| l.id == line_id).ok_or_else(not_sold);
    }
    let first = lines.next().ok_or_else(not_sold)?;
    if lines.any(|l| Bucket::of(l) != Bucket::of(first)) {
        return Err(errors::coded(
            errors::REFUND_LINE_AMBIGUOUS,
            &[("product", &item.menu_item_id)],
        ));
    }
    Ok(first)
}

/// The quantity refunded from `bucket`, this request and every prior
/// refund of the same sale together, may not exceed what was sold in
/// it.
fn check_sold(
    original_lines: &[pos_core::order::OrderLine],
    bucket: &Bucket,
    requested: i64,
    already_refunded: i64,
) -> Result<(), String> {
    let sold: i64 = original_lines
        .iter()
        .filter(|l| Bucket::of(l) == *bucket)
        .map(|l| i64::from(l.quantity))
        .sum();
    match requested.checked_add(already_refunded) {
        Some(total) if total <= sold => Ok(()),
        _ => Err(exceeds_sold(
            bucket.product,
            requested,
            sold,
            already_refunded,
        )),
    }
}

/// Sum, per product, price and VAT label, the quantity already
/// refunded against the sale identified by `original`. Every refund of a sale carries the
/// SAME `fdmRef` (the original ticket's FDM identity - see the comment
/// at the call site), so a prior refund is any archived N/T event whose
/// request JSON's `fdmRef` matches all four of the original's FDM
/// reference fields. Reading `event_data_json` (what the POS actually
/// sent) rather than re-deriving quantities from any local order table
/// is deliberate: it is the one record that can never be edited after
/// the fact.
fn already_refunded_quantities(
    conn: &rusqlite::Connection,
    original: &pos_fiscal::response::FdmReference,
) -> Result<std::collections::HashMap<Bucket, i64>, rusqlite::Error> {
    let event_label = serde_json::to_string(&original.event_label)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();
    let mut stmt = conn.prepare(
        "SELECT event_data_json FROM fiscal_events
         WHERE json_extract(event_data_json, '$.fdmRef.fdmId') = ?1
           AND json_extract(event_data_json, '$.fdmRef.fdmDateTime') = ?2
           AND json_extract(event_data_json, '$.fdmRef.eventLabel') = ?3
           AND json_extract(event_data_json, '$.fdmRef.eventCounter') = ?4
           AND json_extract(event_data_json, '$.fdmRef.totalCounter') = ?5",
    )?;
    let rows = stmt.query_map(
        rusqlite::params![
            original.fdm_id,
            original.fdm_date_time,
            event_label,
            original.event_counter,
            original.total_counter,
        ],
        |row| row.get::<_, String>(0),
    )?;

    let mut totals: std::collections::HashMap<Bucket, i64> = std::collections::HashMap::new();
    for row in rows {
        let json = row?;
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) else {
            continue;
        };
        let Some(lines) = value["transaction"]["transactionLines"].as_array() else {
            continue;
        };
        for line in lines {
            let product = &line["mainProduct"];
            let quantity = product["quantity"].as_f64().unwrap_or(0.0);
            if quantity >= 0.0 {
                continue; // only negative (refund) lines count
            }
            let Some(product_id) = product["productId"]
                .as_str()
                .and_then(|s| s.parse::<i64>().ok())
            else {
                continue;
            };
            let bucket = Bucket {
                product: product_id,
                unit_price_cents: (product["unitPrice"].as_f64().unwrap_or(0.0) * 100.0).round()
                    as i64,
                vat: product["vats"][0]["label"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            };
            let total = totals.entry(bucket).or_insert(0);
            *total = total.saturating_add((-quantity).round() as i64);
        }
    }
    Ok(totals)
}
