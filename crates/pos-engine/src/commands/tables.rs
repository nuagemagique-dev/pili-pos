use serde::Serialize;

use pos_core::order::{OrderStatus, OrderType};
use pos_fiscal::event::{build_order_input, build_pre_bill_input};
use pos_fiscal::fdm_client::{SignMutation, SignRequest};
use pos_fiscal::ticket::{TicketDraft, TicketLine};
use pos_fiscal::types::EventLabel;

use crate::commands::access::{self, Access};
use crate::commands::bar_slip::SlipOutcome;
use crate::commands::errors;
use crate::commands::signing::{
    self, ArchiveSpec, BarSlip, Begin, Kind, Op, PendingContext, Prepared, SignGuard,
};
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableInfo {
    pub table_number: i32,
    pub order_id: Option<i64>,
    pub total_cents: i64,
    pub line_count: i64,
    pub status: String,
    /// When the table's order was opened (UTC, RFC 3339).
    pub opened_at: Option<String>,
    /// A provisional bill was printed and the order has not changed since.
    pub pre_bill_printed: bool,
    /// The next course waiting for "Envoyer la suite" (1 to 3), when
    /// courses are on and the table holds one.
    pub held_course: Option<i32>,
}

/// List tables with their current order status.
///
/// A table is only reported as **occupied** when its open order
/// contains at least one line. Empty open orders (typical after a
/// tap-and-walk-away) are treated as free so the TablesView doesn't
/// surface ghost "0.00 €" cards that add cognitive load for zero
/// informational value (UX Guide §2.1).
///
/// The tables are those of the table plan ([`super::table_plan`]), plus
/// any table left out of it that still has an open order, so no unpaid
/// order is hidden.
pub fn do_list_configured_tables(state: &AppState) -> Result<Vec<TableInfo>, String> {
    let mut numbers = super::table_plan::plan_numbers(state)?;
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .prepare(
                "SELECT DISTINCT o.table_number FROM orders o
                 JOIN order_lines ol ON ol.order_id = o.id
                 WHERE o.status = 'open' AND o.order_type = 'dine_in'
                   AND o.table_number IS NOT NULL",
            )
            .map_err(|e| e.to_string())?;
        let open: Vec<i32> = stmt
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        numbers.extend(open);
    }
    numbers.sort_unstable();
    numbers.dedup();
    list_tables_numbered(state, &numbers)
}

/// Tables 1 to `table_count`.
pub fn do_list_tables(state: &AppState, table_count: i32) -> Result<Vec<TableInfo>, String> {
    let numbers: Vec<i32> = (1..=table_count).collect();
    list_tables_numbered(state, &numbers)
}

fn list_tables_numbered(state: &AppState, numbers: &[i32]) -> Result<Vec<TableInfo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut tables = Vec::new();

    for &t in numbers {
        let row: Option<(i64, i64, i64, String, bool)> = db
            .query_row(
                "SELECT o.id, o.total_cents, COUNT(ol.id), o.created_at, o.pre_bill_at IS NOT NULL
                 FROM orders o
                 LEFT JOIN order_lines ol ON ol.order_id = o.id
                 WHERE o.table_number = ?1 AND o.status = 'open'
                 GROUP BY o.id
                 HAVING COUNT(ol.id) > 0
                 ORDER BY o.id
                 LIMIT 1",
                rusqlite::params![t],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .ok();

        tables.push(match row {
            Some((order_id, total, count, opened_at, pre_bill_printed)) => TableInfo {
                table_number: t,
                order_id: Some(order_id),
                total_cents: total,
                line_count: count,
                status: "occupied".to_string(),
                opened_at: Some(opened_at),
                pre_bill_printed,
                held_course: super::courses::held_course(&db, order_id),
            },
            None => TableInfo {
                table_number: t,
                order_id: None,
                total_cents: 0,
                line_count: 0,
                status: "free".to_string(),
                opened_at: None,
                pre_bill_printed: false,
                held_course: None,
            },
        });
    }

    Ok(tables)
}

/// An open takeaway with at least one line: a direct sale not paid yet.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeawayInfo {
    pub order_id: i64,
    pub total_cents: i64,
    pub line_count: i64,
    pub created_at: String,
}

/// The takeaways left open (nothing is registered for them until they
/// are paid): the Tables screen lists them so each one ends in a sale
/// or is closed unpaid (FAQ Q205). Empty drafts are left out.
pub fn do_list_open_takeaways(state: &AppState) -> Result<Vec<TakeawayInfo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT o.id, o.total_cents, COUNT(ol.id), o.created_at
             FROM orders o
             JOIN order_lines ol ON ol.order_id = o.id
             WHERE o.order_type = 'takeaway' AND o.status = 'open'
               AND o.hubrise_order_id IS NULL
             GROUP BY o.id
             ORDER BY o.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(TakeawayInfo {
                order_id: r.get(0)?,
                total_cents: r.get(1)?,
                line_count: r.get(2)?,
                created_at: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// Register what was ordered at a table since the last registration
/// (event P, "vente interrompue", POS description v2.2 §1.3): the
/// order's journal entries not yet transmitted, corrections included,
/// in chronological order. A no-op when nothing changed. The UI calls
/// it when the operator leaves a table. Once signed, the new lines go
/// to the bar / kitchen slip; the result says what became of it.
pub async fn do_sign_table_order(state: &AppState, order_id: i64) -> Result<SlipOutcome, String> {
    let operator = access::operator(state, Access::Staff)?;
    let op = Op::new(Kind::TableOrder, order_id);
    let guard = match signing::begin(state, &op).await? {
        Begin::Fresh(guard) => guard,
        Begin::AlreadyDone(recovered) => {
            return Ok(recovered
                .follow_up
                .slip
                .unwrap_or(SlipOutcome::NothingToPrint));
        }
    };
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let registered = sign_table_order_locked(state, &guard, &config, order_id, &operator).await?;
    drop(guard);
    let nothing_new = registered.is_none();
    let outcome = print_registered(state, &config, registered).await;
    // Courses on: nothing new to sign still prints what is signed and
    // waited for a printer, or for a course moved to an earlier one.
    if nothing_new && super::courses::enabled(state) {
        let table = state
            .db
            .lock()
            .ok()
            .and_then(|db| pos_db::queries::orders::get_order(&db, order_id).ok())
            .and_then(|o| o.table_number);
        return Ok(super::courses::print_signed(
            state,
            &config,
            &operator.insz,
            order_id,
            table,
            super::courses::Print::Immediate,
        )
        .await);
    }
    Ok(outcome)
}

/// An order's entries registered by a P, to print on the bar slip.
type Registered = Option<(PendingContext, signing::Committed)>;

/// Register the order's unsent entries (event P) in `operator`'s name.
/// `None` when nothing changed since the last registration.
pub(crate) async fn sign_table_order_locked(
    state: &AppState,
    guard: &SignGuard<'_>,
    config: &pos_core::config::PosConfig,
    order_id: i64,
    operator: &pos_core::user::User,
) -> Result<Registered, String> {
    let (mut order, entries, bp_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let order = pos_db::queries::orders::get_order(&db, order_id).map_err(|e| e.to_string())?;
        let entries = pos_db::queries::orders::get_order_entries(&db, order_id, true)
            .map_err(|e| e.to_string())?;
        let bp = super::booking(&db, guard, config)?;
        (order, entries, bp)
    };

    // A takeaway is a direct sale: it stays a draft until it is paid
    // (or closed unpaid) and ends in an N event, never a P (FAQ Q205).
    if order.order_type == OrderType::Takeaway {
        return Err(errors::TAKEAWAY_NOT_REGISTERED.into());
    }
    let Some(last_entry) = entries.last() else {
        return Ok(None);
    };
    order.user_insz = operator.insz.clone();
    if order.status != OrderStatus::Open {
        return Err(errors::ORDER_NOT_OPEN.into());
    }

    let label = EventLabel::P.in_mode(config.training_mode);
    let ticket_number = super::peek_ticket_number(state, guard, label)?;
    let order_input = build_order_input(
        config,
        &state.identity,
        &order,
        &entries,
        ticket_number,
        &bp_id,
    );
    let request = SignRequest::new(SignMutation::Order, &order_input, config.training_mode)
        .map_err(errors::internal)?;
    let context = PendingContext::TableOrder {
        bar_slip: BarSlip {
            operator_insz: order.user_insz.clone(),
            table_number: order.table_number,
        },
    };
    let prepared = Prepared {
        op: Op::new(Kind::TableOrder, order_id),
        request,
        label,
        first_ticket_no: ticket_number,
        events: vec![ArchiveSpec {
            order_id: Some(order_id),
            user_insz: order.user_insz.clone(),
            request_json: serde_json::to_string(&order_input).map_err(|e| e.to_string())?,
            sent_entries: Some((order_id, last_entry.id)),
        }],
        order_id: Some(order_id),
        user_insz: order.user_insz.clone(),
        context: context.clone(),
    };
    let committed = signing::sign(state, guard, config, prepared)
        .await
        .map_err(|f| f.error)?;
    Ok(Some((context, committed)))
}

/// The bar / kitchen slip of what a P registered.
pub(crate) async fn print_registered(
    state: &AppState,
    config: &pos_core::config::PosConfig,
    registered: Registered,
) -> SlipOutcome {
    match registered {
        Some((context, committed)) => {
            signing::follow_up(state, config, &context, &committed, false)
                .await
                .slip
                .unwrap_or(SlipOutcome::NothingToPrint)
        }
        None => SlipOutcome::NothingToPrint,
    }
}

/// Send a signPreBill (provisional bill P event).
///
/// Provisional bill ("addition provisoire", §1.4): the consolidated
/// order, printed PRO FORMA. Entries not yet registered are sent as an
/// order first, since a pre-bill may not add or change lines; the
/// result is the bar / kitchen slip of those entries. Both are signed
/// under one lock, so no line slips in between.
pub async fn do_sign_pre_bill(state: &AppState, order_id: i64) -> Result<SlipOutcome, String> {
    let operator = access::operator(state, Access::Cashier)?;
    let op = Op::new(Kind::PreBill, order_id);
    let guard = match signing::begin(state, &op).await? {
        Begin::Fresh(guard) => guard,
        // Answered late: its follow-up printed it.
        Begin::AlreadyDone(_) => return Ok(SlipOutcome::NothingToPrint),
    };
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let registered = sign_table_order_locked(state, &guard, &config, order_id, &operator).await?;
    let pre_bill = sign_pre_bill_locked(state, &guard, &config, order_id, &operator, op).await;
    drop(guard);

    let slip = print_registered(state, &config, registered).await;
    let (context, committed) = pre_bill?;
    signing::follow_up(state, &config, &context, &committed, false).await;
    Ok(slip)
}

async fn sign_pre_bill_locked(
    state: &AppState,
    guard: &SignGuard<'_>,
    config: &pos_core::config::PosConfig,
    order_id: i64,
    operator: &pos_core::user::User,
    op: Op,
) -> Result<(PendingContext, signing::Committed), String> {
    let (mut order, lines, bp_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let order = pos_db::queries::orders::get_order(&db, order_id).map_err(|e| e.to_string())?;
        let lines =
            pos_db::queries::orders::get_order_lines(&db, order_id).map_err(|e| e.to_string())?;
        let bp = super::booking(&db, guard, config)?;
        (order, lines, bp)
    };
    order.user_insz = operator.insz.clone();
    if lines.is_empty() {
        return Err(errors::ORDER_NO_LINES.into());
    }

    let label = EventLabel::P.in_mode(config.training_mode);
    let ticket_number = super::peek_ticket_number(state, guard, label)?;
    let pre_bill = build_pre_bill_input(
        config,
        &state.identity,
        &order,
        &lines,
        ticket_number,
        &bp_id,
    );
    let request = SignRequest::new(SignMutation::PreBill, &pre_bill, config.training_mode)
        .map_err(errors::internal)?;

    let operator_name = super::operator_name(state, &order.user_insz);
    let ticket_lines = lines
        .iter()
        .map(|l| TicketLine {
            quantity: l.quantity as f64,
            description: l.name.clone(),
            unit_price: l.unit_price_cents as f64 / 100.0,
            total: (l.unit_price_cents * l.quantity as i64) as f64 / 100.0,
            vat_code: l.vat_code.as_str().to_string(),
        })
        .collect();
    let context = PendingContext::PreBill {
        order_id,
        ticket: TicketDraft::new(
            config,
            ticket_lines,
            vec![],
            order.total_cents as f64 / 100.0,
            &operator_name,
        ),
    };
    let prepared = Prepared {
        op,
        request,
        label,
        first_ticket_no: ticket_number,
        events: vec![ArchiveSpec {
            order_id: Some(order_id),
            user_insz: order.user_insz.clone(),
            request_json: serde_json::to_string(&pre_bill).map_err(|e| e.to_string())?,
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
