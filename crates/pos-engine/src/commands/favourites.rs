//! The favourites tab of the order screen (UX audit C12): the first tab,
//! before the categories, with the products the staff reach for most.
//!
//! Two modes, a per-installation setting (key `favourites` of the owner
//! settings store, so every change is logged in the append-only
//! `owner_settings_log`):
//!
//! - `top_sold` (the default): the 18 best sellers of the last 30 booking
//!   days, computed from the archived signed sales (the same N events the
//!   sales report reads, refunds counted negative, training left out). The
//!   ranking is cached (migration 92) and computed again at most once an
//!   hour, or as soon as a Z report was made since. Products that are not on
//!   sale (inactive, off the grid) or sold out right now are skipped, so the
//!   18 places go to products that can be added.
//! - `selection`: the products a manager starred in the menu editor, in the
//!   order they were added. A sold-out product stays in the list, marked
//!   "Épuisé" like in its category.
//!
//! The tab is empty (and the UIs hide it) when there is nothing to show.
//! Any logged-in operator reads, only a manager changes the setting.
//!
//! Local data, never fiscal: the setting and the cache feed a tab of the
//! screen and nothing else. A product is added through the same order
//! commands as from a category, so no signed request, ticket or export
//! changes (`tests/favourites.rs` proves it).

use std::collections::{BTreeMap, HashMap};

use chrono::NaiveDate;
use pos_core::menu::MenuItem;
use pos_db::queries::reports::{self as report_queries, PeriodZ};
use pos_db::queries::{favourites as cache, menu as menu_queries};
use pos_fiscal::archive;
use pos_fiscal::report_totals::ReportEvent;
use pos_fiscal::types::EventLabel;
use serde::{Deserialize, Serialize};

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::commands::owner_settings;
use crate::state::AppState;

/// The key of the favourites in the owner settings store.
pub const KEY_FAVOURITES: &str = "favourites";

/// How many products the "most sold" tab shows.
pub const TOP_SOLD_COUNT: usize = 18;

/// How many booking days the ranking looks back over (today included).
pub const WINDOW_DAYS: i64 = 30;

/// The most products a manager's selection holds.
pub const MAX_SELECTION: usize = 30;

/// How many ranked products are kept in the cache: enough to fill the 18
/// places when some of the best sellers are sold out.
const CACHED_RANKS: usize = 100;

/// The system product that sells a gift voucher: never a favourite.
const GIFT_VOUCHER_ITEM_ID: i64 = 990_001;

/// Where the tab takes its products from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FavouritesMode {
    /// The best sellers of the last 30 days.
    #[default]
    TopSold,
    /// The manager's own list.
    Selection,
}

/// The favourites setting of this installation.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FavouritesSettings {
    /// Which list the tab shows.
    #[serde(default)]
    pub mode: FavouritesMode,
    /// The manager's selection (product ids) in the order they were added.
    /// Kept while the mode is `top_sold`, so switching back loses nothing.
    #[serde(default)]
    pub selection: Vec<i64>,
}

/// The setting, for any logged-in operator.
pub fn do_get_favourites_settings(state: &AppState) -> Result<FavouritesSettings, String> {
    access::operator(state, Access::Staff)?;
    let db = state.db.lock().map_err(errors::internal)?;
    owner_settings::get::<FavouritesSettings>(&db, KEY_FAVOURITES)
}

/// Whether `id` is a product the menu knows and a favourite may point to.
fn is_known_product(db: &rusqlite::Connection, id: i64) -> Result<bool, String> {
    if id == GIFT_VOUCHER_ITEM_ID {
        return Ok(false);
    }
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM menu_items WHERE id = ?1)",
        [id],
        |r| r.get::<_, i32>(0),
    )
    .map(|v| v != 0)
    .map_err(errors::internal)
}

/// Check a selection: known products, a repeat keeps its first place, at
/// most [`MAX_SELECTION`].
fn clean_selection(db: &rusqlite::Connection, ids: &[i64]) -> Result<Vec<i64>, String> {
    let mut selection: Vec<i64> = Vec::new();
    for id in ids {
        if selection.contains(id) {
            continue;
        }
        if !is_known_product(db, *id)? {
            return Err(errors::FAVOURITES_INVALID.to_string());
        }
        selection.push(*id);
    }
    if selection.len() > MAX_SELECTION {
        return Err(errors::coded(
            errors::FAVOURITES_TOO_MANY,
            &[("max", &MAX_SELECTION)],
        ));
    }
    Ok(selection)
}

/// Store the setting (managers only): the mode and the selection in its
/// order. Returns it as stored.
pub fn do_set_favourites_settings(
    state: &AppState,
    input: FavouritesSettings,
) -> Result<FavouritesSettings, String> {
    let operator = access::operator(state, Access::Admin)?;
    let db = state.db.lock().map_err(errors::internal)?;
    let stored = FavouritesSettings {
        mode: input.mode,
        selection: clean_selection(&db, &input.selection)?,
    };
    owner_settings::set(&db, KEY_FAVOURITES, &stored, Some(&operator.insz))?;
    Ok(stored)
}

/// Star or unstar one product (managers only): a starred product joins the
/// end of the selection, an unstarred one leaves it. Starring a product that
/// is already in the list changes nothing. Works whatever the mode is.
/// Returns the setting as stored.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the star toggle: the flag is part of the command's wire form"
)]
pub fn do_set_favourite(
    state: &AppState,
    menu_item_id: i64,
    favourite: bool,
) -> Result<FavouritesSettings, String> {
    let operator = access::operator(state, Access::Admin)?;
    let db = state.db.lock().map_err(errors::internal)?;
    let mut settings = owner_settings::get::<FavouritesSettings>(&db, KEY_FAVOURITES)?;
    if favourite {
        if !is_known_product(&db, menu_item_id)? {
            return Err(errors::FAVOURITES_INVALID.to_string());
        }
        if !settings.selection.contains(&menu_item_id) {
            settings.selection.push(menu_item_id);
        }
        if settings.selection.len() > MAX_SELECTION {
            return Err(errors::coded(
                errors::FAVOURITES_TOO_MANY,
                &[("max", &MAX_SELECTION)],
            ));
        }
    } else {
        settings.selection.retain(|id| *id != menu_item_id);
    }
    owner_settings::set(&db, KEY_FAVOURITES, &settings, Some(&operator.insz))?;
    Ok(settings)
}

/// The net units of each product sold in the booking days `from..=to`, read
/// from the archived N events (refunds negative). A weighed line counts as
/// one unit, whatever its weight.
fn units_sold(
    db: &rusqlite::Connection,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<BTreeMap<i64, i64>, String> {
    let periods: Vec<PeriodZ> =
        report_queries::periods_with_z(db, &from.to_string(), &to.to_string())
            .map_err(errors::internal)?;
    let mut units: BTreeMap<i64, i64> = BTreeMap::new();
    for period in &periods {
        for archived in archive::events_in_booking_period(db, &period.period_id)
            .map_err(errors::internal)?
            .iter()
            .filter(|e| e.event_type == EventLabel::N)
        {
            let Some(event) = ReportEvent::from_archive(archived) else {
                continue;
            };
            if !event.has_transaction() {
                continue;
            }
            for line in event.lines() {
                let product = &line["mainProduct"];
                let Some(id) = product["productId"]
                    .as_str()
                    .and_then(|s| s.parse::<i64>().ok())
                else {
                    continue;
                };
                let quantity = errors::archived_milli(&product["quantity"], "quantity")?;
                let counted = if product["quantityType"] == "KILOGRAM" {
                    quantity.signum() * 1000
                } else {
                    quantity
                };
                *units.entry(id).or_default() += counted;
            }
        }
    }
    Ok(units)
}

/// Compute the ranking of the last 30 booking days now: product id and
/// thousandths of units, best first (ties: the lower id first), only
/// products sold on balance.
fn compute_ranking(state: &AppState, db: &rusqlite::Connection) -> Result<Vec<(i64, i64)>, String> {
    let day_end_hour = state.config.lock().map_err(errors::internal)?.day_end_hour;
    let today = super::business_date(pos_fiscal::event::now_in_belgium(), day_end_hour);
    let to = NaiveDate::parse_from_str(&today, "%Y-%m-%d").map_err(errors::internal)?;
    let from = to - chrono::Duration::days(WINDOW_DAYS - 1);
    let mut ranking: Vec<(i64, i64)> = units_sold(db, from, to)?
        .into_iter()
        .filter(|(id, milli)| *milli > 0 && *id != GIFT_VOUCHER_ITEM_ID)
        .collect();
    ranking.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    ranking.truncate(CACHED_RANKS);
    Ok(ranking)
}

/// The cached ranking, computed again first when it is an hour old or a Z
/// was made since. Units are stored in thousandths.
fn ranking(state: &AppState, db: &rusqlite::Connection) -> Result<Vec<i64>, String> {
    if cache::is_due(db).map_err(errors::internal)? {
        let fresh = compute_ranking(state, db)?;
        cache::replace(db, &fresh).map_err(errors::internal)?;
    }
    Ok(cache::ranking(db)
        .map_err(errors::internal)?
        .into_iter()
        .map(|(id, _)| id)
        .collect())
}

/// The products of the "Favoris" tab, in display order. Any logged-in
/// operator. Empty when there is nothing to show (the UIs then hide the tab).
pub fn do_list_favourites(state: &AppState, lang: Option<String>) -> Result<Vec<MenuItem>, String> {
    access::operator(state, Access::Staff)?;
    let lang = lang.as_deref().unwrap_or("fr");
    let db = state.db.lock().map_err(errors::internal)?;
    let settings = owner_settings::get::<FavouritesSettings>(&db, KEY_FAVOURITES)?;
    // The sold-out products whose end of day is past get their `expired` log row now.
    super::sold_out::expire_due(&db);
    let menu = menu_queries::get_menu_items(&db, None, lang).map_err(errors::internal)?;
    let mut by_id: HashMap<i64, MenuItem> = menu.into_iter().map(|m| (m.id, m)).collect();
    let items = match settings.mode {
        FavouritesMode::Selection => settings
            .selection
            .iter()
            .filter_map(|id| by_id.remove(id))
            .collect(),
        FavouritesMode::TopSold => ranking(state, &db)?
            .into_iter()
            .filter_map(|id| by_id.remove(&id))
            .filter(|m| m.active && !m.sold_out)
            .take(TOP_SOLD_COUNT)
            .collect(),
    };
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_most_sold_with_no_selection() {
        let defaults = FavouritesSettings::default();
        assert_eq!(defaults.mode, FavouritesMode::TopSold);
        assert_eq!(defaults.selection, Vec::<i64>::new());
    }

    #[test]
    fn the_setting_has_a_stable_json_form() {
        let json = serde_json::to_string(&FavouritesSettings {
            mode: FavouritesMode::Selection,
            selection: vec![3, 1],
        })
        .unwrap();
        assert_eq!(json, r#"{"mode":"selection","selection":[3,1]}"#);
        let back: FavouritesSettings = serde_json::from_str(r#"{"mode":"top_sold"}"#).unwrap();
        assert_eq!(back, FavouritesSettings::default());
    }
}
