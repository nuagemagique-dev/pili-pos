//! Discounts taken at the till (feature `hubrise`, whose price change
//! store they reuse): a manager, confirmed by their PIN, gives a percent,
//! an amount or the whole order ("Offert") off an open till order, or one
//! line for free (Advanced Set UC223, UC225A/B, UC229B).
//!
//! The discount is a `priceChange` (POS spec v2.2): scope EVENT for an
//! order discount, LINE for a free line, type PUBLIC, spread over the
//! lines with `pos_core::money::distribute`. Where it is recorded depends
//! on the line's journal:
//! - one entry, not yet sent to the FDM: the change goes on that entry
//!   (the direct sale shape of UC223 and UC229B);
//! - otherwise a `PRICE_CHANGE` pair: a correction `-q` carrying the line's
//!   existing changes negated, then a re-entry `+q` carrying the new ones
//!   (UC225A step 2, UC225B step 2). The journal is append-only: a recorded
//!   change is never deleted, a removal is the same pair in reverse.
//!
//! A table that was already registered gets its P order signed right
//! away, under the same signing lock: a `PRICE_CHANGE` correction "n'apparait
//! que dans la commande et est donc compte sur le signOrder" (producers
//! FAQ Q303 d). A takeaway or an unregistered table is registered by its
//! N, as a direct sale. The reason of a discount is local (audit table
//! `till_discounts`), never part of a signed payload.

use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use pos_core::money::distribute;
use pos_core::order::{Order, OrderLine, OrderStatus, OrderType};
use pos_db::queries::price_changes::{self as db_changes, NewPriceChange, PriceChangeRow};
use pos_db::queries::{orders, pending_signs, till_discounts};
use pos_fiscal::price_change::{
    TILL_DISCOUNT_ID, TILL_FREE_ID, TILL_FREE_LINE_ID, till_change_name,
};
use pos_fiscal::ticket::ReceiptLanguage;

use crate::commands::access::{self, Access};
use crate::commands::bar_slip::SlipOutcome;
use crate::commands::eft;
use crate::commands::errors;
use crate::commands::signing::{self, Begin, Kind, Op};
use crate::commands::tables::{print_registered, sign_table_order_locked};
use crate::state::AppState;

/// What the discount applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DiscountTarget {
    /// The whole order, spread over its lines.
    Order,
    /// One line, for free (`kind` must be `free`).
    Line { line_id: i64 },
}

/// How much is taken off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscountKind {
    /// An integer percent, 1 to 99 (`value`).
    Percent,
    /// An amount in cents, 1 to the net total (`value`).
    Amount,
    /// Everything ("Offert", 100 %); `value` is ignored.
    Free,
}

/// Why the manager gives it (audit only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscountReason {
    CommercialGesture,
    StaffMeal,
    ServiceIssue,
    Other,
}

impl DiscountReason {
    fn code(self) -> &'static str {
        match self {
            Self::CommercialGesture => "commercial_gesture",
            Self::StaffMeal => "staff_meal",
            Self::ServiceIssue => "service_issue",
            Self::Other => "other",
        }
    }
}

/// A discount to apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscountInput {
    pub target: DiscountTarget,
    pub kind: DiscountKind,
    #[serde(default)]
    pub value: Option<i64>,
    pub reason_code: DiscountReason,
    #[serde(default)]
    pub reason_text: Option<String>,
}

/// What a discount command did.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscountResult {
    /// The order after the change (its total is net).
    pub order: Order,
    /// The bar / kitchen slip of the entries the immediate P registered
    /// (the `PRICE_CHANGE` entries print nothing; other unsent entries do).
    pub kitchen_slip: SlipOutcome,
    /// Set when the immediate P could not be signed (FDM down, say): the
    /// discount is recorded and goes with the next registration or the
    /// sale; the screen shows this as a warning.
    pub registration_error: Option<String>,
}

/// The longest free text of a reason.
const REASON_TEXT_MAX: usize = 120;

/// Apply `input` to the open till order `order_id`. A manager only
/// (`Access::Admin`), confirmed by `pin`.
pub async fn do_apply_discount(
    state: &AppState,
    order_id: i64,
    input: DiscountInput,
    insz: String,
    pin: String,
) -> Result<DiscountResult, String> {
    let manager = access::confirmed_operator(state, &insz, &pin, Access::Admin)?;
    let input = validate(input)?;
    run(state, order_id, &manager, |tx, ctx| apply(tx, ctx, &input)).await
}

/// Remove the order discount (`line_id` `None`), or the free line
/// `line_id`. A manager only, confirmed by `pin`.
pub async fn do_remove_discount(
    state: &AppState,
    order_id: i64,
    line_id: Option<i64>,
    insz: String,
    pin: String,
) -> Result<DiscountResult, String> {
    let manager = access::confirmed_operator(state, &insz, &pin, Access::Admin)?;
    run(state, order_id, &manager, |tx, ctx| {
        remove(tx, ctx, line_id)
    })
    .await
}

/// The shape of the input, before anything is read.
fn validate(mut input: DiscountInput) -> Result<DiscountInput, String> {
    match (input.target, input.kind) {
        (DiscountTarget::Line { .. }, DiscountKind::Free) | (DiscountTarget::Order, _) => {}
        (DiscountTarget::Line { .. }, _) => return Err(errors::DISCOUNT_INVALID.into()),
    }
    match input.kind {
        DiscountKind::Percent => match input.value {
            Some(1..=99) => {}
            _ => return Err(errors::DISCOUNT_INVALID.into()),
        },
        DiscountKind::Amount => match input.value {
            Some(v) if v > 0 => {}
            _ => return Err(errors::DISCOUNT_INVALID.into()),
        },
        DiscountKind::Free => input.value = None,
    }
    let text = input
        .reason_text
        .take()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    if let Some(text) = &text
        && (text.chars().count() > REASON_TEXT_MAX || text.chars().any(char::is_control))
    {
        return Err(errors::DISCOUNT_REASON_INVALID.into());
    }
    input.reason_text = text;
    Ok(input)
}

/// What the closures below work with, read under the signing lock.
struct Ctx {
    order: Order,
    manager_insz: String,
    /// The receipt language code the change names are written in.
    lang: &'static str,
}

/// The common frame: signing lock, checks, one transaction for the
/// journal, the changes and the audit row, then the immediate P.
async fn run(
    state: &AppState,
    order_id: i64,
    manager: &pos_core::user::User,
    work: impl FnOnce(&Connection, &Ctx) -> Result<(), String>,
) -> Result<DiscountResult, String> {
    let op = Op::new(Kind::Discount, order_id);
    let guard = match signing::begin(state, &op).await? {
        Begin::Fresh(guard) => guard,
        // The discount signs nothing under its own name.
        Begin::AlreadyDone(_) => return Err(errors::internal("discount recovered as an event")),
    };
    let config = state.config.lock().map_err(errors::internal)?.clone();
    let lang = ReceiptLanguage::from_code(&config.language).code();

    let register = {
        let db = state.db.lock().map_err(errors::internal)?;
        let order = check(&db, order_id)?;
        let register = order.order_type == OrderType::DineIn
            && till_discounts::any_entry_sent(&db, order_id).map_err(errors::internal)?;
        // A card charged on the terminal bounds how low the total goes.
        let floor = eft::charged_floor(&db, order_id)?;
        let ctx = Ctx {
            order,
            manager_insz: manager.insz.clone(),
            lang,
        };
        let tx = db.unchecked_transaction().map_err(errors::internal)?;
        work(&tx, &ctx)?;
        eft::keep_charged(&tx, order_id, floor)?;
        tx.commit().map_err(errors::internal)?;
        register
    };

    // The immediate P: the PRICE_CHANGE pair (and anything else unsent)
    // of an order the FDM already knows.
    let (kitchen_slip, registration_error) = if register {
        match sign_table_order_locked(state, &guard, &config, order_id, manager).await {
            Ok(registered) => {
                drop(guard);
                (print_registered(state, &config, registered).await, None)
            }
            Err(e) => (SlipOutcome::NothingToPrint, Some(e)),
        }
    } else {
        (SlipOutcome::NothingToPrint, None)
    };
    let order = {
        let db = state.db.lock().map_err(errors::internal)?;
        orders::get_order(&db, order_id).map_err(errors::internal)?
    };
    Ok(DiscountResult {
        order,
        kitchen_slip,
        registration_error,
    })
}

/// The refusals that do not depend on the kind of discount.
fn check(db: &Connection, order_id: i64) -> Result<Order, String> {
    let order = orders::get_order(db, order_id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => errors::ORDER_NOT_OPEN.to_string(),
        other => errors::internal(other),
    })?;
    if order.status != OrderStatus::Open {
        return Err(errors::ORDER_NOT_OPEN.into());
    }
    let online: bool = db
        .query_row(
            "SELECT hubrise_order_id IS NOT NULL FROM orders WHERE id = ?1",
            [order_id],
            |r| r.get(0),
        )
        .map_err(errors::internal)?;
    if online {
        return Err(errors::DISCOUNT_ONLINE_ORDER.into());
    }
    if pending_signs::order_has_open(db, order_id).map_err(errors::internal)? {
        return Err(errors::ORDER_SIGN_PENDING.into());
    }
    Ok(order)
}

/// One line of the order with its consolidated changes.
struct LineState {
    line: OrderLine,
    changes: Vec<PriceChangeRow>,
}

impl LineState {
    /// The line's amount after its changes.
    fn net_cents(&self) -> i64 {
        self.line.line_total() + self.changes.iter().map(|c| c.amount_cents).sum::<i64>()
    }

    fn has_change_of(&self, ids: &[&str]) -> bool {
        self.changes
            .iter()
            .any(|c| ids.contains(&c.change_id.as_str()))
    }
}

fn load_lines(db: &Connection, order_id: i64) -> Result<Vec<LineState>, String> {
    let lines = orders::get_order_lines(db, order_id).map_err(errors::internal)?;
    let mut changes: BTreeMap<i64, Vec<PriceChangeRow>> =
        db_changes::by_line(db, order_id).map_err(errors::internal)?;
    Ok(lines
        .into_iter()
        .filter(|l| l.quantity > 0)
        .map(|line| LineState {
            changes: changes.remove(&line.id).unwrap_or_default(),
            line,
        })
        .collect())
}

/// A change to put on a line.
struct Change<'a> {
    grouping_id: i32,
    id: &'a str,
    name: &'a str,
    scope: &'a str,
    amount_cents: i64,
}

impl<'a> Change<'a> {
    fn new_price_change(&self) -> NewPriceChange<'a> {
        NewPriceChange {
            grouping_id: self.grouping_id,
            change_id: self.id,
            name: self.name,
            scope: self.scope,
            change_type: "PUBLIC",
            amount_cents: self.amount_cents,
        }
    }
}

/// Record `change` on the line, choosing between the direct shape and a
/// `PRICE_CHANGE` pair (see the module comment).
fn place(db: &Connection, line: &LineState, change: &Change<'_>) -> Result<(), String> {
    let (entries, unsent) =
        till_discounts::line_entry_counts(db, line.line.id).map_err(errors::internal)?;
    if entries == 1 && unsent == 1 {
        db_changes::add_to_line(db, line.line.id, &[change.new_price_change()])
            .map_err(errors::internal)?;
        reset_pre_bill(db, line.line.order_id)?;
        return Ok(());
    }
    pair(db, line, &[change.new_price_change()])
}

/// A `PRICE_CHANGE` pair on the line: the correction carries every change
/// the line has, negated; the re-entry carries `after`.
fn pair(db: &Connection, line: &LineState, after: &[NewPriceChange<'_>]) -> Result<(), String> {
    let (correction, reentry) =
        orders::add_price_change_pair(db, line.line.id).map_err(errors::internal)?;
    db_changes::negate_line_on_entry(db, line.line.id, correction).map_err(errors::internal)?;
    db_changes::add_to_entry(db, reentry, after).map_err(errors::internal)?;
    reset_pre_bill(db, line.line.order_id)
}

/// A provisional bill no longer matches the order once a discount changes.
fn reset_pre_bill(db: &Connection, order_id: i64) -> Result<(), String> {
    db.execute(
        "UPDATE orders SET pre_bill_at = NULL WHERE id = ?1",
        [order_id],
    )
    .map(|_| ())
    .map_err(errors::internal)
}

#[expect(
    clippy::too_many_lines,
    reason = "one transaction applying every discount rule in order, clearer unsplit"
)]
fn apply(db: &Connection, ctx: &Ctx, input: &DiscountInput) -> Result<(), String> {
    let order_id = ctx.order.id;
    let lines = load_lines(db, order_id)?;
    if lines.is_empty() {
        return Err(errors::ORDER_NO_LINES.into());
    }
    let order_discount = lines
        .iter()
        .any(|l| l.has_change_of(&[TILL_DISCOUNT_ID, TILL_FREE_ID]));
    let grouping_id = till_discounts::max_grouping_id(db, order_id).map_err(errors::internal)? + 1;
    let reason = (input.reason_code.code(), input.reason_text.as_deref());

    match input.target {
        DiscountTarget::Line { line_id } => {
            let line = lines
                .iter()
                .find(|l| l.line.id == line_id)
                .ok_or(errors::LINE_NOT_FOUND)?;
            // A gift voucher line is the voucher's face value: never
            // discounted (its value must equal what is paid for it).
            if super::gift_vouchers::is_voucher_line(db, line_id) {
                return Err(errors::DISCOUNT_INVALID.into());
            }
            // One change per line, and none under an order discount.
            if order_discount || !line.changes.is_empty() {
                return Err(errors::DISCOUNT_ALREADY_APPLIED.into());
            }
            let net = line.net_cents();
            if net <= 0 {
                return Err(errors::DISCOUNT_INVALID.into());
            }
            let name = till_change_name(ctx.lang, TILL_FREE_LINE_ID, None);
            place(
                db,
                line,
                &Change {
                    grouping_id,
                    id: TILL_FREE_LINE_ID,
                    name: &name,
                    scope: "LINE",
                    amount_cents: -net,
                },
            )?;
            audit(
                db,
                ctx,
                Some(line_id),
                &Audit {
                    kind: Some("free"),
                    value: None,
                    amount_cents: -net,
                    change_id: TILL_FREE_LINE_ID,
                    grouping_id,
                    reason,
                },
            )
        }
        DiscountTarget::Order => {
            if order_discount {
                return Err(errors::DISCOUNT_ALREADY_APPLIED.into());
            }
            // Only lines with something left to discount share it (a
            // free line is already at 0,00); a gift voucher line never
            // shares an order discount.
            let weights: Vec<i64> = lines
                .iter()
                .map(|l| {
                    if super::gift_vouchers::is_voucher_line(db, l.line.id) {
                        0
                    } else {
                        l.net_cents().max(0)
                    }
                })
                .collect();
            let net_total: i64 = weights.iter().sum();
            if net_total <= 0 {
                return Err(errors::DISCOUNT_INVALID.into());
            }
            let (id, percent, discount) = match (input.kind, input.value) {
                // Half up: 15 % of 1,10 EUR is 0,17 (0,165 rounded).
                (DiscountKind::Percent, Some(p)) => {
                    (TILL_DISCOUNT_ID, Some(p), (net_total * p + 50) / 100)
                }
                (DiscountKind::Amount, Some(v)) if v <= net_total => (TILL_DISCOUNT_ID, None, v),
                (DiscountKind::Free, _) => (TILL_FREE_ID, None, net_total),
                _ => return Err(errors::DISCOUNT_INVALID.into()),
            };
            if discount <= 0 {
                return Err(errors::DISCOUNT_INVALID.into());
            }
            let name = till_change_name(ctx.lang, id, percent);
            let shares = distribute(&weights, -discount);
            for (line, share) in lines.iter().zip(&shares) {
                if *share == 0 {
                    continue;
                }
                place(
                    db,
                    line,
                    &Change {
                        grouping_id,
                        id,
                        name: &name,
                        scope: "EVENT",
                        amount_cents: *share,
                    },
                )?;
            }
            audit(
                db,
                ctx,
                None,
                &Audit {
                    kind: Some(match input.kind {
                        DiscountKind::Percent => "percent",
                        DiscountKind::Amount => "amount",
                        DiscountKind::Free => "free",
                    }),
                    value: input.value.filter(|_| input.kind != DiscountKind::Free),
                    amount_cents: -discount,
                    change_id: id,
                    grouping_id,
                    reason,
                },
            )
        }
    }
}

fn remove(db: &Connection, ctx: &Ctx, line_id: Option<i64>) -> Result<(), String> {
    let order_id = ctx.order.id;
    let lines = load_lines(db, order_id)?;
    // The ids this removal takes away.
    let ids: &[&str] = match line_id {
        None => &[TILL_DISCOUNT_ID, TILL_FREE_ID],
        Some(_) => &[TILL_FREE_LINE_ID],
    };
    let targets: Vec<&LineState> = lines
        .iter()
        .filter(|l| line_id.is_none_or(|id| l.line.id == id))
        .filter(|l| l.has_change_of(ids))
        .collect();
    if targets.is_empty() {
        if let Some(id) = line_id
            && !lines.iter().any(|l| l.line.id == id)
        {
            return Err(errors::LINE_NOT_FOUND.into());
        }
        return Err(errors::DISCOUNT_NONE.into());
    }

    let mut given_back = 0;
    let mut taken: Option<(String, i32)> = None;
    for line in &targets {
        let kept: Vec<&PriceChangeRow> = line
            .changes
            .iter()
            .filter(|c| !ids.contains(&c.change_id.as_str()))
            .collect();
        for c in line
            .changes
            .iter()
            .filter(|c| ids.contains(&c.change_id.as_str()))
        {
            given_back -= c.amount_cents;
            taken = Some((c.change_id.clone(), c.grouping_id));
        }
        let after: Vec<NewPriceChange<'_>> = kept
            .iter()
            .map(|c| NewPriceChange {
                grouping_id: c.grouping_id,
                change_id: &c.change_id,
                name: &c.name,
                scope: &c.scope,
                change_type: &c.change_type,
                amount_cents: c.amount_cents,
            })
            .collect();
        pair(db, line, &after)?;
    }
    let (change_id, grouping_id) = taken.unwrap_or_default();
    audit(
        db,
        ctx,
        line_id,
        &Audit {
            kind: None,
            value: None,
            amount_cents: given_back,
            change_id: &change_id,
            grouping_id,
            reason: ("removal", None),
        },
    )
}

/// An audit row, bar the order and the manager.
struct Audit<'a> {
    kind: Option<&'a str>,
    value: Option<i64>,
    amount_cents: i64,
    change_id: &'a str,
    grouping_id: i32,
    reason: (&'a str, Option<&'a str>),
}

fn audit(db: &Connection, ctx: &Ctx, line_id: Option<i64>, a: &Audit<'_>) -> Result<(), String> {
    till_discounts::insert(
        db,
        &till_discounts::NewTillDiscount {
            order_id: ctx.order.id,
            line_id,
            action: if a.kind.is_some() { "apply" } else { "remove" },
            kind: a.kind,
            value: a.value,
            amount_cents: a.amount_cents,
            change_id: a.change_id,
            grouping_id: a.grouping_id,
            reason_code: a.reason.0,
            reason_text: a.reason.1,
            manager_insz: &ctx.manager_insz,
        },
    )
    .map_err(errors::internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(kind: DiscountKind, value: Option<i64>) -> DiscountInput {
        DiscountInput {
            target: DiscountTarget::Order,
            kind,
            value,
            reason_code: DiscountReason::Other,
            reason_text: None,
        }
    }

    #[test]
    fn the_percent_is_one_to_ninety_nine() {
        for bad in [None, Some(0), Some(100), Some(-5)] {
            assert_eq!(
                validate(input(DiscountKind::Percent, bad)).unwrap_err(),
                errors::DISCOUNT_INVALID
            );
        }
        assert!(validate(input(DiscountKind::Percent, Some(99))).is_ok());
        assert!(validate(input(DiscountKind::Percent, Some(1))).is_ok());
    }

    #[test]
    fn an_amount_is_positive_and_free_ignores_its_value() {
        assert!(validate(input(DiscountKind::Amount, Some(0))).is_err());
        assert!(validate(input(DiscountKind::Amount, None)).is_err());
        assert!(validate(input(DiscountKind::Amount, Some(1))).is_ok());
        let free = validate(input(DiscountKind::Free, Some(7))).unwrap();
        assert_eq!(free.value, None);
    }

    #[test]
    fn only_a_free_kind_applies_to_a_line() {
        let mut i = input(DiscountKind::Free, None);
        i.target = DiscountTarget::Line { line_id: 3 };
        assert!(validate(i.clone()).is_ok());
        i.kind = DiscountKind::Percent;
        i.value = Some(10);
        assert_eq!(validate(i).unwrap_err(), errors::DISCOUNT_INVALID);
    }

    #[test]
    fn the_reason_text_is_trimmed_and_bounded() {
        let mut i = input(DiscountKind::Free, None);
        i.reason_text = Some("  geste  ".into());
        assert_eq!(
            validate(i.clone()).unwrap().reason_text.as_deref(),
            Some("geste")
        );
        i.reason_text = Some("   ".into());
        assert_eq!(validate(i.clone()).unwrap().reason_text, None);
        i.reason_text = Some("x".repeat(REASON_TEXT_MAX + 1));
        assert_eq!(
            validate(i.clone()).unwrap_err(),
            errors::DISCOUNT_REASON_INVALID
        );
        i.reason_text = Some("a\nb".into());
        assert_eq!(validate(i).unwrap_err(), errors::DISCOUNT_REASON_INVALID);
    }

    #[test]
    fn the_input_has_a_stable_json_shape() {
        let json = serde_json::json!({
            "target": { "type": "line", "line_id": 4 },
            "kind": "free",
            "reason_code": "staff_meal",
        });
        let parsed: DiscountInput = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.target, DiscountTarget::Line { line_id: 4 });
        assert_eq!(parsed.reason_code, DiscountReason::StaffMeal);
        let order = serde_json::json!({ "target": { "type": "order" }, "kind": "percent",
            "value": 20, "reason_code": "other", "reason_text": "x" });
        assert!(serde_json::from_value::<DiscountInput>(order).is_ok());
    }
}
