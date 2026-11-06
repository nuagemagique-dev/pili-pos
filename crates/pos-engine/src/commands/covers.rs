//! The covers of a table order (UX audit T5): how many guests sit at the
//! table. Optional, set from the cart header ("2 couverts", a pad from 1 to
//! 30), shown on the table card ("4 couv.") and offered at checkout as a HINT
//! for the number of equal parts: the matching button is marked and the pad of
//! "Autre" starts on it, but the sale is never split by itself.
//!
//! Local data, never fiscal: the number lives in its own table
//! (`order_covers`) and reaches no FDM request, signed ticket, archived event
//! or export (`tests/table_zones.rs` proves it).

use pos_core::order::{OrderStatus, OrderType};
use pos_db::queries::covers;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

/// The covers of the order, `None` when the operator set none.
pub fn do_get_order_covers(state: &AppState, order_id: i64) -> Result<Option<i32>, String> {
    access::operator(state, Access::Staff)?;
    let db = state.db.lock().map_err(errors::internal)?;
    Ok(covers::get(&db, order_id)
        .map_err(errors::internal)?
        .and_then(|n| i32::try_from(n).ok()))
}

/// Set (`Some`, 1 to 30) or clear (`None`) the covers of the open table
/// order `order_id`; answers what is stored. Any staff member may; a
/// takeaway has no covers.
pub fn do_set_order_covers(
    state: &AppState,
    order_id: i64,
    value: Option<i32>,
) -> Result<Option<i32>, String> {
    access::operator(state, Access::Staff)?;
    if let Some(n) = value
        && !(1..=i32::try_from(covers::MAX_COVERS).unwrap_or(30)).contains(&n)
    {
        return Err(errors::coded(
            errors::COVERS_INVALID,
            &[("max", &covers::MAX_COVERS)],
        ));
    }
    let db = state.db.lock().map_err(errors::internal)?;
    let order = pos_db::queries::orders::get_order(&db, order_id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => errors::ORDER_NOT_OPEN.to_string(),
        other => errors::internal(other),
    })?;
    if order.status != OrderStatus::Open {
        return Err(errors::ORDER_NOT_OPEN.into());
    }
    if order.order_type != OrderType::DineIn {
        return Err(errors::TAKEAWAY_HAS_NO_TABLE.into());
    }
    covers::set(&db, order_id, value.map(i64::from)).map_err(errors::internal)?;
    Ok(value)
}
