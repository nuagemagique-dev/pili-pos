use pos_fiscal::types::Mode;
use serde::Deserialize;

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
pub fn do_list_refundable_events(
    state: &AppState,
    limit: Option<i64>,
) -> Result<Vec<RefundableEvent>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    fiscal::list_refundable_events(&db, limit.unwrap_or(20)).map_err(|e| e.to_string())
}

pub async fn do_refund_checkout(
    state: &AppState,
    original_fiscal_event_id: i64,
    items: Vec<RefundItem>,
    employee_insz: String,
) -> Result<CheckoutResult, String> {
    do_refund_checkout_with(state, original_fiscal_event_id, items, employee_insz, None).await
}

/// Process a full or partial refund for a previously signed ticket.
///
/// Creates a new Event N with:
/// - negative-quantity lines (negQuantityReason = REFUND)
/// - fdmRef referencing the original ticket
/// - cash payment (refund handed to customer)
///
/// Archives the event, prints a REFUND receipt, opens the drawer.
///
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
            gift_voucher_prints: Vec::new(),
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
    pub(crate) fn cash(lines: RefundLines<'a>) -> Self {
        Self {
            lines,
            payment_type: PaymentType::Cash,
            open_drawer: true,
        }
    }
}

/// Sign the refund of the sale `original_fiscal_event_id` in
/// `employee_insz`'s name (already checked by the caller).
#[expect(
    clippy::too_many_lines,
    reason = "one refund from the original sale to the signed negative ticket, kept as a single audited sequence"
)]
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
                gift_voucher_prints: Vec::new(),
            });
        }
        Err(e) => return Ok(CheckoutResult::refused(e)),
    };

    // 1. Load original fiscal event → extract FdmReference
    // Without HubRise no sale has discounts to check.
    #[cfg_attr(
        not(feature = "hubrise"),
        expect(
            unused_variables,
            reason = "only the HubRise price-change record reads the changes"
        )
    )]
    let (
        original_ref,
        original_signature,
        original_lines,
        original_changes,
        config,
        bp_id,
        original_order,
    ) = {
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
        // An unreadable record must stop the refund: read as `Null` it
        // would look paid.
        let request: serde_json::Value = serde_json::from_str(&event.event_data_json)
            .map_err(|e| errors::internal(format!("fiscal event {}: {e}", event.id)))?;
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
            original_order_id,
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
    let (refund_order, refund_lines, refund_changes, gift_vouchers_cancelled) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        // Every refund of this sale - this one included - carries the
        // SAME `fdmRef` (the original ticket, never a chain through
        // prior refunds; see FLUX_REMBOURSEMENT_FR.md and the M112B
        // test). That makes `fdmRef` the stable key for "how much of
        // this sale has already been refunded", across any number of
        // prior partial refunds, so a second or third partial refund
        // can't push the aggregate past what was actually sold.
        let already_refunded = already_refunded_quantities(&db, &original_ref)?;

        // (quantity refunded, the original line it comes from)
        let mut refunded: Vec<(i32, &pos_core::order::OrderLine)> = Vec::new();
        match spec.lines {
            RefundLines::Products(items) => {
                let mut requested: std::collections::BTreeMap<Bucket, i64> =
                    std::collections::BTreeMap::new();
                let mut picked = Vec::with_capacity(items.len());
                for item in items {
                    if item.quantity <= 0 {
                        return Err(errors::coded(
                            errors::REFUND_QUANTITY_POSITIVE,
                            &[("product", &item.menu_item_id)],
                        ));
                    }
                    let original = original_line(&original_lines, item)?;
                    // A weighing goes back whole: no refund of a part of it.
                    if original.is_weighed() && item.quantity != 1 {
                        return Err(errors::coded(
                            errors::REFUND_WEIGHED_LINE_WHOLE,
                            &[("product", &item.menu_item_id)],
                        ));
                    }
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
                // A discount is spread over its lines: a part of one
                // would refund a share nobody can tell. The whole sale
                // can go back, each line with its own share negated.
                #[cfg(feature = "hubrise")]
                let whole_sale = already_refunded.is_empty()
                    && original_lines.iter().filter(|l| l.quantity > 0).all(|l| {
                        let bucket = Bucket::of(l);
                        let sold: i64 = original_lines
                            .iter()
                            .filter(|o| o.quantity > 0 && Bucket::of(o) == bucket)
                            .map(|o| i64::from(o.quantity))
                            .sum();
                        requested.get(&bucket).copied().unwrap_or(0) == sold
                    });
                for (quantity, original) in picked {
                    #[cfg(feature = "hubrise")]
                    if !whole_sale && original_changes.contains_key(&original.id) {
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
                // The first bucket in key order, so the message is deterministic.
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
        // A refunded gift voucher sale cancels the vouchers it sold, if none
        // was used (V12-D28); a used one stops the refund here.
        let gift_vouchers_cancelled = super::gift_vouchers::vouchers_to_cancel(
            &db,
            original_order,
            &refunded
                .iter()
                .map(|(q, l)| (l.menu_item_id, i64::from(*q), l.unit_price_cents))
                .collect::<Vec<_>>(),
        )?;
        let order = pos_db::queries::orders::create_order(
            &db,
            pos_core::order::OrderType::Takeaway,
            None,
            &employee_insz,
        )
        .map_err(|e| e.to_string())?;
        for (quantity, original) in refunded {
            let new_line = pos_db::queries::orders::NewOrderLine {
                order_id: order.id,
                menu_item_id: original.menu_item_id,
                quantity: -quantity, // NEGATE for refund
                unit_price_cents: original.unit_price_cents,
                vat_code: original.vat_code,
                name: &original.name,
                modifiers_json: None,
                department_id: &original.department_id,
                department_name: &original.department_name,
            };
            // A weighing goes back with the weight and price signed.
            let written = match (original.weight_grams, original.weighed_amount_cents) {
                (Some(weight_grams), Some(amount_cents)) => {
                    pos_db::queries::orders::add_weighed_order_line(
                        &db,
                        new_line,
                        pos_db::queries::orders::Weighed {
                            weight_grams,
                            amount_cents,
                        },
                    )
                }
                _ => pos_db::queries::orders::add_order_line(&db, new_line),
            }
            .map_err(|e| e.to_string())?;
            // The whole line goes back: its discounts too, negated.
            #[cfg(feature = "hubrise")]
            if let Some(changes) = original_changes.get(&original.id) {
                let negated: Vec<_> = changes
                    .iter()
                    .map(pos_fiscal::price_change::LineChange::negated)
                    .collect();
                super::price_changes::record(&db, written.id, &negated)?;
            }
            // Only the HubRise price-change record needs the new line.
            #[cfg(not(feature = "hubrise"))]
            drop(written);
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
        (updated_order, lines, changes, gift_vouchers_cancelled)
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

        // A cash refund is rounded to 5 cents like a cash sale (SPF
        // refund ticket example: -3,04 paid out as -3,05).
        let rounding_cents = if spec.payment_type == PaymentType::Cash {
            pos_fiscal::ticket::round_to_5_cents(refund_order.total_cents)
        } else {
            0
        };

        // SaleInput with negative lines + original fdm_ref
        let mut sale_input = build_sale_input(
            &config,
            &state.identity,
            &refund_order,
            &refund_lines,
            ticket_number,
            &bp_id,
            spec.payment_type,
            refund_order.total_cents, // negative
            rounding_cents,
            Some(&original_ref),
        );
        pos_fiscal::price_change::apply(
            &mut sale_input.transaction,
            &refund_lines,
            &refund_changes,
        );
        sale_input.common.ticket_medium = resolved.medium;
        let request = SignRequest::new(
            SignMutation::Sale,
            &sale_input,
            Mode::from(config.training_mode),
        )
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
                amount: pos_core::money::eur(refund_order.total_cents),
                brand: None,
            }],
            pos_core::money::eur(refund_order.total_cents),
            &operator,
        );
        ticket.rounding = (rounding_cents != 0).then(|| pos_core::money::eur(rounding_cents));
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
            // What leaves the drawer, rounding included.
            total_cents: refund_order.total_cents + rounding_cents,
            open_drawer: spec.open_drawer,
            delivery: StoredDelivery::of(&resolved),
            ticket,
            webshop_order_id: original_order_id_if_online(state, original_fiscal_event_id),
            gift_vouchers_cancelled: gift_vouchers_cancelled.clone(),
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
                    gift_voucher_prints: Vec::new(),
                }),
                Err(failure) => Err(CheckoutResult {
                    success: false,
                    ticket: None,
                    error: Some(failure.error),
                    fdm_connected: true,
                    fiscal_event_id: None,
                    delivery: None,
                    kitchen_slip: None,
                    gift_voucher_prints: Vec::new(),
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
    let done =
        signing::follow_up(state, &config, &context, &committed, signing::Drawer::Kick).await;
    Ok(CheckoutResult {
        success: true,
        ticket: committed.ticket,
        error: None,
        fdm_connected: true,
        fiscal_event_id: Some(committed.event_ids[0]),
        delivery: done.delivery,
        kitchen_slip: None,
        gift_voucher_prints: Vec::new(),
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
    match state.db.lock() {
        Ok(db) => {
            if let Err(e) = db.execute(
                "UPDATE orders SET status = 'voided' WHERE id = ?1 AND status = 'open'",
                rusqlite::params![order_id],
            ) {
                tracing::error!(order_id, %e, "could not void the unsigned refund order");
            }
        }
        Err(e) => {
            tracing::error!(order_id, %e, "could not void the unsigned refund order: db lock poisoned");
        }
    }
}

pub const ERR_REFUND_IN_TRAINING: &str = errors::REFUND_IN_TRAINING;
pub const ERR_REFUND_OF_REFUND: &str = errors::REFUND_OF_REFUND;

/// Lines of one product sold at one price and VAT label: what a
/// refund is counted against.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Bucket {
    product: i64,
    unit_price_cents: i64,
    vat: String,
    /// A weighed line counts by its weight: two weighings of one product
    /// at one price are two different things sold.
    weight_grams: Option<i64>,
}

impl Bucket {
    fn of(line: &pos_core::order::OrderLine) -> Self {
        Self {
            product: line.menu_item_id,
            unit_price_cents: line.unit_price_cents,
            vat: format!("{:?}", line.vat_code),
            weight_grams: line.weight_grams,
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
) -> Result<std::collections::BTreeMap<Bucket, i64>, String> {
    already_refunded_quantities_inner(conn, original).map_err(errors::internal)
}

fn already_refunded_quantities_inner(
    conn: &rusqlite::Connection,
    original: &pos_fiscal::response::FdmReference,
) -> Result<std::collections::BTreeMap<Bucket, i64>, String> {
    let event_label = serde_json::to_string(&original.event_label)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();
    let mut stmt = conn
        .prepare(
            "SELECT event_data_json FROM fiscal_events
         WHERE json_extract(event_data_json, '$.fdmRef.fdmId') = ?1
           AND json_extract(event_data_json, '$.fdmRef.fdmDateTime') = ?2
           AND json_extract(event_data_json, '$.fdmRef.eventLabel') = ?3
           AND json_extract(event_data_json, '$.fdmRef.eventCounter') = ?4
           AND json_extract(event_data_json, '$.fdmRef.totalCounter') = ?5",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            rusqlite::params![
                original.fdm_id,
                original.fdm_date_time,
                event_label,
                original.event_counter,
                original.total_counter,
            ],
            |row| row.get::<_, String>(0),
        )
        .map_err(|e| e.to_string())?;

    let mut totals: std::collections::BTreeMap<Bucket, i64> = std::collections::BTreeMap::new();
    for row in rows {
        let json = row.map_err(|e| e.to_string())?;
        let value = serde_json::from_str::<serde_json::Value>(&json)
            .map_err(|e| format!("unreadable prior refund record: {e}"))?;
        let Some(lines) = value["transaction"]["transactionLines"].as_array() else {
            continue;
        };
        for line in lines {
            let product = &line["mainProduct"];
            // A line without a quantity cannot be counted: refuse rather
            // than let the refund cap be exceeded.
            let quantity = product["quantity"]
                .as_f64()
                .ok_or_else(|| "a prior refund line has no quantity".to_string())?;
            if quantity >= 0.0 {
                continue; // only negative (refund) lines count
            }
            let Some(product_id) = product["productId"]
                .as_str()
                .and_then(|s| s.parse::<i64>().ok())
            else {
                continue;
            };
            let unit_price_cents = pos_core::money::cents_of(&product["unitPrice"])
                .ok_or_else(|| "a prior refund line has no unit price".to_string())?;
            // A signed weight is one weighing of that weight, whatever its
            // size: rounded as a count, -0.278 would be nothing.
            let weight_grams = (product["quantityType"].as_str() == Some("KILOGRAM"))
                .then(|| pos_core::money::round_to_i64(-quantity * 1000.0));
            let bucket = Bucket {
                product: product_id,
                unit_price_cents,
                vat: product["vats"][0]["label"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                weight_grams,
            };
            let count = if weight_grams.is_some() {
                1
            } else {
                pos_core::money::round_to_i64(-quantity)
            };
            let total = totals.entry(bucket).or_insert(0);
            *total = total.saturating_add(count);
        }
    }
    Ok(totals)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn original() -> FdmReference {
        FdmReference {
            fdm_id: "FDM1".into(),
            fdm_date_time: "20260101120000".into(),
            event_label: EventLabel::N,
            event_counter: 1,
            total_counter: 2,
        }
    }

    /// A connection whose only fiscal event is a refund with `line`.
    fn conn_with_refund_line(line: &serde_json::Value) -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE fiscal_events (id INTEGER PRIMARY KEY, event_data_json TEXT);",
        )
        .unwrap();
        let json = serde_json::json!({
            "fdmRef": {
                "fdmId": "FDM1",
                "fdmDateTime": "20260101120000",
                "eventLabel": "N",
                "eventCounter": 1,
                "totalCounter": 2,
            },
            "transaction": {"transactionLines": [line]},
        });
        conn.execute(
            "INSERT INTO fiscal_events (event_data_json) VALUES (?1)",
            [json.to_string()],
        )
        .unwrap();
        conn
    }

    #[test]
    fn prior_refund_lines_are_counted_per_bucket() {
        let conn = conn_with_refund_line(&serde_json::json!({"mainProduct": {
            "productId": "7", "quantity": -2.0, "unitPrice": 12.5,
            "vats": [{"label": "A"}],
        }}));
        let totals = already_refunded_quantities(&conn, &original()).unwrap();
        assert_eq!(totals.len(), 1);
        let (bucket, qty) = totals.iter().next().unwrap();
        assert_eq!(
            (bucket.product, bucket.unit_price_cents, *qty),
            (7, 1250, 2)
        );
    }

    #[test]
    fn a_prior_refund_line_without_quantity_is_an_error() {
        let conn = conn_with_refund_line(&serde_json::json!({"mainProduct": {
            "productId": "7", "unitPrice": 12.5, "vats": [{"label": "A"}],
        }}));
        let err = already_refunded_quantities(&conn, &original()).unwrap_err();
        assert!(err.contains("no quantity"), "{err}");
    }

    #[test]
    fn a_prior_refund_line_without_unit_price_is_an_error() {
        let conn = conn_with_refund_line(&serde_json::json!({"mainProduct": {
            "productId": "7", "quantity": -1.0, "vats": [{"label": "A"}],
        }}));
        let err = already_refunded_quantities(&conn, &original()).unwrap_err();
        assert!(err.contains("no unit price"), "{err}");
    }

    #[test]
    fn an_unreadable_prior_refund_record_is_an_error() {
        let conn = conn_with_refund_line(&serde_json::json!({}));
        conn.execute("UPDATE fiscal_events SET event_data_json = '{broken'", [])
            .unwrap();
        assert!(already_refunded_quantities(&conn, &original()).is_err());
    }

    /// The buckets come back in key order, so the refund that exceeds the
    /// sold quantity names the lowest product id (a stable message).
    #[test]
    fn prior_refund_buckets_come_back_in_product_order() {
        let line = |product: &str| {
            serde_json::json!({"mainProduct": {
                "productId": product, "quantity": -1.0, "unitPrice": 1.0,
                "vats": [{"label": "A"}],
            }})
        };
        let conn = conn_with_refund_line(&line("9"));
        let json = serde_json::json!({
            "fdmRef": {
                "fdmId": "FDM1", "fdmDateTime": "20260101120000",
                "eventLabel": "N", "eventCounter": 1, "totalCounter": 2,
            },
            "transaction": {"transactionLines": [line("9"), line("3"), line("5")]},
        });
        conn.execute(
            "UPDATE fiscal_events SET event_data_json = ?1",
            [json.to_string()],
        )
        .unwrap();
        let totals = already_refunded_quantities(&conn, &original()).unwrap();
        let products: Vec<i64> = totals.keys().map(|b| b.product).collect();
        assert_eq!(products, vec![3, 5, 9]);
    }
}
