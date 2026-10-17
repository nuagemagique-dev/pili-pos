//! Moving the open order of a table to another table (event P,
//! `signCostCenterChange`, Basic Set M122). The order keeps its id, its
//! lines, its `reference` and any approved terminal payment; the FDM is
//! told that every consolidated line left `T<X>` and arrived on `T<Y>`.
//!
//! Merging two tables (UC250/251) is not supported: a target that holds
//! an order with lines is refused. An order left empty on the target (a
//! tap and walk away) does not count; it is voided with the move.

use serde::Serialize;

use pos_core::order::{OrderStatus, OrderType};
use pos_fiscal::event::build_cost_center_change_input;
use pos_fiscal::fdm_client::{SignMutation, SignRequest};
use pos_fiscal::types::EventLabel;

use crate::commands::access::{self, Access};
use crate::commands::bar_slip::SlipOutcome;
use crate::commands::errors;
use crate::commands::signing::{
    self, ArchiveSpec, Begin, Kind, Op, PendingContext, Prepared, SignGuard,
};
use crate::commands::tables::{print_registered, sign_table_order_locked};
use crate::state::AppState;

/// Highest table number the Tables screen accepts.
const MAX_TABLE: i32 = 999;

/// What a transfer did.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableTransferResult {
    pub order_id: i64,
    pub from_table: i32,
    pub to_table: i32,
    /// The bar / kitchen slip of the entries registered before the move
    /// (nothing to print when everything was already registered).
    pub kitchen_slip: SlipOutcome,
    /// The notice that the order moved.
    pub transfer_slip: SlipOutcome,
}

/// Move the open dine-in order `order_id` to the free table `to_table`.
///
/// Under one signing lock: the entries not yet registered are sent as
/// an order first (so the move carries everything the FDM knows), then
/// the move is signed. The table changes in the archive transaction of
/// the move, so an answer that comes late has the same effect. Printing
/// never undoes the signing; both outcomes go back to the screen.
pub async fn do_transfer_table(
    state: &AppState,
    order_id: i64,
    to_table: i32,
) -> Result<TableTransferResult, String> {
    let operator = access::operator(state, Access::Staff)?;
    // The target is part of the subject: a retry to another table after
    // a lost answer is a new request, not the one that was answered.
    let op = Op::new(Kind::TableTransfer, format!("{order_id}>{to_table}"));
    let guard = match signing::begin(state, &op).await? {
        Begin::Fresh(guard) => guard,
        Begin::AlreadyDone(recovered) => {
            let (from_table, to_table) = match &recovered.context {
                PendingContext::TableTransfer {
                    from_table,
                    to_table,
                    ..
                } => (*from_table, *to_table),
                _ => (0, to_table),
            };
            return Ok(TableTransferResult {
                order_id,
                from_table,
                to_table,
                kitchen_slip: SlipOutcome::NothingToPrint,
                transfer_slip: recovered
                    .follow_up
                    .slip
                    .unwrap_or(SlipOutcome::NothingToPrint),
            });
        }
    };
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let from_table = check(state, order_id, to_table)?;

    // Entries not yet registered go first, on the old table.
    let registered = sign_table_order_locked(state, &guard, &config, order_id, &operator).await?;
    let moved = sign_transfer_locked(
        state, &guard, &config, order_id, from_table, to_table, &operator, op,
    )
    .await;
    drop(guard);

    let kitchen_slip = print_registered(state, &config, registered).await;
    let (context, committed) = moved?;
    let transfer_slip = signing::follow_up(state, &config, &context, &committed, false)
        .await
        .slip
        .unwrap_or(SlipOutcome::NothingToPrint);
    Ok(TableTransferResult {
        order_id,
        from_table,
        to_table,
        kitchen_slip,
        transfer_slip,
    })
}

/// The refusals, read under the signing lock. Returns the table the
/// order is on.
fn check(state: &AppState, order_id: i64, to_table: i32) -> Result<i32, String> {
    let plan = super::table_plan::plan_numbers(state)?;
    let db = state.db.lock().map_err(errors::internal)?;
    let order = pos_db::queries::orders::get_order(&db, order_id).map_err(|e| match e {
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
    let from_table = match (order.order_type, order.table_number) {
        (OrderType::DineIn, Some(table)) if !online => table,
        _ => return Err(errors::TABLE_TRANSFER_NOT_A_TABLE.into()),
    };
    if to_table == from_table {
        return Err(errors::TABLE_TRANSFER_SAME_TABLE.into());
    }
    if !(1..=MAX_TABLE).contains(&to_table) || !plan.contains(&to_table) {
        return Err(errors::TABLE_TRANSFER_TARGET_INVALID.into());
    }
    let lines: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM order_lines WHERE order_id = ?1 AND quantity > 0",
            [order_id],
            |r| r.get(0),
        )
        .map_err(errors::internal)?;
    if lines == 0 {
        return Err(errors::ORDER_NO_LINES.into());
    }
    let busy: bool = db
        .query_row(
            "SELECT EXISTS (
                 SELECT 1 FROM orders o
                 WHERE o.status = 'open' AND o.order_type = 'dine_in'
                   AND o.table_number = ?1 AND o.id <> ?2 AND o.hubrise_order_id IS NULL
                   AND EXISTS (SELECT 1 FROM order_lines ol WHERE ol.order_id = o.id))",
            rusqlite::params![to_table, order_id],
            |r| r.get(0),
        )
        .map_err(errors::internal)?;
    if busy {
        return Err(errors::TABLE_TRANSFER_TARGET_BUSY.into());
    }
    if pos_db::queries::pending_signs::order_has_open(&db, order_id).map_err(errors::internal)? {
        return Err(errors::ORDER_SIGN_PENDING.into());
    }
    Ok(from_table)
}

#[allow(clippy::too_many_arguments)]
async fn sign_transfer_locked(
    state: &AppState,
    guard: &SignGuard<'_>,
    config: &pos_core::config::PosConfig,
    order_id: i64,
    from_table: i32,
    to_table: i32,
    operator: &pos_core::user::User,
    op: Op,
) -> Result<(PendingContext, signing::Committed), String> {
    let (mut order, lines, changes, bp_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let order = pos_db::queries::orders::get_order(&db, order_id).map_err(|e| e.to_string())?;
        let lines: Vec<_> = pos_db::queries::orders::get_order_lines(&db, order_id)
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|l| l.quantity > 0)
            .collect();
        // Price changes travel with their line (FAQ Q304 d).
        #[cfg(feature = "hubrise")]
        let changes = super::price_changes::load(&db, order_id)?.1;
        #[cfg(not(feature = "hubrise"))]
        let changes = pos_fiscal::price_change::LineChanges::new();
        let bp = super::booking(&db, guard, config)?;
        (order, lines, changes, bp)
    };
    order.user_insz = operator.insz.clone();
    if lines.is_empty() {
        return Err(errors::ORDER_NO_LINES.into());
    }

    let label = EventLabel::P.in_mode(config.training_mode);
    let ticket_number = super::peek_ticket_number(state, guard, label)?;
    let input = build_cost_center_change_input(
        config,
        &state.identity,
        &order,
        &lines,
        &changes,
        from_table,
        to_table,
        ticket_number,
        &bp_id,
    );
    let request = SignRequest::new(SignMutation::CostCenterChange, &input, config.training_mode)
        .map_err(errors::internal)?;
    let context = PendingContext::TableTransfer {
        order_id,
        from_table,
        to_table,
        operator_insz: operator.insz.clone(),
    };
    let prepared = Prepared {
        op,
        request,
        label,
        first_ticket_no: ticket_number,
        events: vec![ArchiveSpec {
            order_id: Some(order_id),
            user_insz: order.user_insz.clone(),
            request_json: serde_json::to_string(&input).map_err(|e| e.to_string())?,
            // The move adds no journal entry: the sale later sends the
            // order's journal under the new table, same reference.
            sent_entries: None,
        }],
        order_id: Some(order_id),
        user_insz: order.user_insz.clone(),
        context: context.clone(),
    };
    let committed = signing::sign(state, guard, config, prepared)
        .await
        .map_err(|f| f.error)?;
    Ok((context, committed))
}
