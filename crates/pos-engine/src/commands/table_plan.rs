//! The table plan: which table numbers the Tables screen shows. The
//! starting plan comes from `pos.json` (`tables`, or 1 to
//! `table_count`); a manager can change it here. A plan can group its tables
//! in zones written with letters ("A 1-20, B 21-40") and a manager can name
//! the zones ("A · Salle"); a table is then written "A-19" on the screens and
//! on the kitchen and bar slips. The table is still its number: the number
//! alone is in every signed request and on the fiscal ticket. Every change
//! is logged (`table_plan_log`, append-only). A table that still has an
//! open order stays on the Tables screen even when the plan leaves it out,
//! so no unpaid order is ever hidden.

use std::collections::BTreeMap;

use pos_core::table_plan::{self as grammar, PlanTable};
use pos_db::queries::table_plan::{self, PlanChange};
use serde::Serialize;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

/// One zone of the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableZone {
    /// The zone letters ("A").
    pub letter: String,
    /// The name a manager gave it, if any ("Salle").
    pub name: Option<String>,
    /// Its tables, ascending.
    pub numbers: Vec<i32>,
}

#[derive(Debug, Serialize)]
pub struct TablePlan {
    /// The tables as ranges, canonical form ("1-12, 20-25", "A 1-20, B 21-40").
    pub plan: String,
    pub table_count: i64,
    /// The names of the zones, canonical form ("A · Salle, B · Terrasse"); empty for none.
    pub zone_names: String,
    /// The zones of the plan, in alphabetical order; none on a plan without letters.
    pub zones: Vec<TableZone>,
    /// The latest changes first.
    pub log: Vec<PlanChange>,
}

/// The tables of the plan in use, ascending by number, with their zone.
pub fn plan_tables(state: &AppState) -> Result<Vec<PlanTable>, String> {
    let stored = {
        let db = state.db.lock().map_err(errors::internal)?;
        table_plan::current(&db).map_err(errors::internal)?
    };
    let text = match stored {
        Some(text) => text,
        None => state.config.lock().map_err(errors::internal)?.table_plan(),
    };
    Ok(grammar::parse_zoned(&text).unwrap_or_else(|e| {
        tracing::error!(%e, "table plan unreadable, no tables listed");
        Vec::new()
    }))
}

/// The table numbers of the plan in use, ascending.
pub fn plan_numbers(state: &AppState) -> Result<Vec<i32>, String> {
    Ok(plan_tables(state)?
        .into_iter()
        .filter_map(|t| i32::try_from(t.number).ok())
        .collect())
}

/// The names of the zones in use, as `(letters, name)`. Only the letters the
/// plan has: a name left over from an earlier plan is not shown.
pub fn zone_names(state: &AppState) -> Result<BTreeMap<String, String>, String> {
    let stored = {
        let db = state.db.lock().map_err(errors::internal)?;
        table_plan::zone_names(&db).map_err(errors::internal)?
    };
    Ok(grammar::parse_zone_names(&stored.unwrap_or_default())
        .unwrap_or_default()
        .into_iter()
        .collect())
}

/// The zone letters of each table of the plan, by table number. A table
/// that the plan leaves out has none.
pub fn zone_by_table(state: &AppState) -> Result<BTreeMap<i32, String>, String> {
    Ok(plan_tables(state)?
        .into_iter()
        .filter_map(|t| Some((i32::try_from(t.number).ok()?, t.zone?)))
        .collect())
}

/// How table `number` is written on a slip: "A-19" in a zone, "19" without
/// one. Falls back to the number when the plan cannot be read.
pub fn label(state: &AppState, number: i32) -> String {
    let zone = zone_by_table(state)
        .ok()
        .and_then(|zones| zones.get(&number).cloned());
    grammar::label(zone.as_deref(), i64::from(number))
}

/// The zone of table `number`, for the slips.
pub(crate) fn zone_of(state: &AppState, number: i32) -> Option<String> {
    zone_by_table(state).ok()?.remove(&number)
}

pub fn do_get_table_plan(state: &AppState) -> Result<TablePlan, String> {
    let tables = plan_tables(state)?;
    let names = zone_names(state)?;
    let mut zones: BTreeMap<String, Vec<i32>> = BTreeMap::new();
    for t in &tables {
        if let (Some(zone), Ok(n)) = (&t.zone, i32::try_from(t.number)) {
            zones.entry(zone.clone()).or_default().push(n);
        }
    }
    let zones: Vec<TableZone> = zones
        .into_iter()
        .map(|(letter, numbers)| TableZone {
            name: names.get(&letter).cloned(),
            letter,
            numbers,
        })
        .collect();
    let shown_names: Vec<(String, String)> = zones
        .iter()
        .filter_map(|z| Some((z.letter.clone(), z.name.clone()?)))
        .collect();
    let db = state.db.lock().map_err(errors::internal)?;
    Ok(TablePlan {
        plan: grammar::format_zoned(&tables),
        table_count: i64::try_from(tables.len()).map_err(errors::internal)?,
        zone_names: grammar::format_zone_names(&shown_names),
        zones,
        log: table_plan::history(&db, 20).map_err(errors::internal)?,
    })
}

/// Set the plan from `text` ("1-12, 20-25", "A 1-20, B 21-40") and the
/// names of its zones from `zone_names_text` ("A · Salle, B · Terrasse",
/// empty for none). Managers only. A name for a letter the plan does not
/// have is refused.
pub fn do_set_table_plan(
    state: &AppState,
    text: &str,
    zone_names_text: &str,
) -> Result<TablePlan, String> {
    let admin = access::operator(state, Access::Admin)?;
    let tables = grammar::parse_zoned(text).map_err(|e| {
        tracing::info!(%e, "table plan refused");
        errors::TABLE_PLAN_INVALID.to_string()
    })?;
    let names = grammar::parse_zone_names(zone_names_text).map_err(|e| {
        tracing::info!(%e, "table zone names refused");
        errors::TABLE_PLAN_INVALID.to_string()
    })?;
    if let Some((zone, _)) = names.iter().find(|(zone, _)| {
        !tables
            .iter()
            .any(|t| t.zone.as_deref() == Some(zone.as_str()))
    }) {
        tracing::info!(%zone, "a zone name for a zone the plan does not have");
        return Err(errors::TABLE_PLAN_INVALID.to_string());
    }
    let plan = grammar::format_zoned(&tables);
    let names = grammar::format_zone_names(&names);
    {
        let db = state.db.lock().map_err(errors::internal)?;
        if table_plan::set(&db, &plan, &names, &admin.insz).map_err(errors::internal)? {
            tracing::info!(%plan, %names, "table plan changed");
        }
    }
    do_get_table_plan(state)
}
