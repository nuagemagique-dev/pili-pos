use pos_engine::commands::takeaway::{TakeawayNumbering, TakeawayTag};
use tauri::State;

use super::Managed;

/// The numbering of the takeaways (the first number of the day), for any logged-in operator.
#[tauri::command(async)]
pub fn get_takeaway_numbering(state: State<'_, Managed>) -> Result<TakeawayNumbering, String> {
    pos_engine::commands::takeaway::do_get_takeaway_numbering(state.inner())
}

/// Store the numbering; managers only.
#[tauri::command(async)]
pub fn set_takeaway_numbering(
    state: State<'_, Managed>,
    input: TakeawayNumbering,
) -> Result<TakeawayNumbering, String> {
    pos_engine::commands::takeaway::do_set_takeaway_numbering(state.inner(), input)
}

/// The number of the day and the customer of a takeaway; `None` for an order without a tag.
#[tauri::command(async)]
pub fn get_takeaway_tag(
    state: State<'_, Managed>,
    order_id: i64,
) -> Result<Option<TakeawayTag>, String> {
    pos_engine::commands::takeaway::do_get_takeaway_tag(state.inner(), order_id)
}

/// Set (or clear) the name and the phone of the customer of an open takeaway.
#[tauri::command(async)]
pub fn set_takeaway_customer(
    state: State<'_, Managed>,
    order_id: i64,
    name: Option<String>,
    phone: Option<String>,
) -> Result<TakeawayTag, String> {
    pos_engine::commands::takeaway::do_set_takeaway_customer(state.inner(), order_id, name, phone)
}
