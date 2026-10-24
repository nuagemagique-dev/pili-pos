//! The table plan: which table numbers the Tables screen shows. The
//! starting plan comes from `pos.json` (`tables`, or 1 to
//! `table_count`); a manager can change it here. Every change is logged
//! (`table_plan_log`, append-only). A table that still has an open
//! order stays on the Tables screen even when the plan leaves it out,
//! so no unpaid order is ever hidden.

use pos_db::queries::table_plan::{self, PlanChange};
use serde::Serialize;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct TablePlan {
    /// The table numbers as ranges, canonical form ("1-12, 20-25").
    pub plan: String,
    pub table_count: i64,
    /// The latest changes first.
    pub log: Vec<PlanChange>,
}

/// The table numbers of the plan in use, ascending.
pub fn plan_numbers(state: &AppState) -> Result<Vec<i32>, String> {
    let stored = {
        let db = state.db.lock().map_err(errors::internal)?;
        table_plan::current(&db).map_err(errors::internal)?
    };
    let text = match stored {
        Some(text) => text,
        None => state.config.lock().map_err(errors::internal)?.table_plan(),
    };
    let numbers = pos_core::table_plan::parse(&text).unwrap_or_default();
    Ok(numbers.into_iter().map(|n| n as i32).collect())
}

pub fn do_get_table_plan(state: &AppState) -> Result<TablePlan, String> {
    let numbers = plan_numbers(state)?;
    let plan = pos_core::table_plan::format(&numbers.iter().map(|&n| n as u32).collect::<Vec<_>>());
    let db = state.db.lock().map_err(errors::internal)?;
    Ok(TablePlan {
        plan,
        table_count: numbers.len() as i64,
        log: table_plan::history(&db, 20).map_err(errors::internal)?,
    })
}

/// Set the plan from `text` ("1-12, 20-25"). Managers only.
pub fn do_set_table_plan(state: &AppState, text: &str) -> Result<TablePlan, String> {
    let admin = access::operator(state, Access::Admin)?;
    let plan = pos_core::table_plan::normalize(text).map_err(|e| {
        tracing::info!(%e, "table plan refused");
        errors::TABLE_PLAN_INVALID.to_string()
    })?;
    {
        let db = state.db.lock().map_err(errors::internal)?;
        if table_plan::set(&db, &plan, &admin.insz).map_err(errors::internal)? {
            tracing::info!(%plan, "table plan changed");
        }
    }
    do_get_table_plan(state)
}
