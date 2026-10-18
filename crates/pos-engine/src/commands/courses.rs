//! Courses and "Envoyer la suite" (decisions V12-D22 and D23).
//!
//! Kitchen printing only, never fiscal: the course of a line lives in
//! `order_line_courses` (migration 62) and nowhere else. It never enters
//! `OrderLine`, a product name, a department, a P, N or T request, the
//! pre-bill or the VAT ticket (spec 8.1 and 8.2: kitchen prints may be
//! regrouped, FDM messages never are; FAQ Q210 b: what the POS does for
//! the kitchen display stays at the POS).
//!
//! With the switch off (the default) every slip is the one the till
//! always printed. With it on, the slip that follows a signed P prints
//! the entries whose course is 0 ("Direct") or not above the released
//! course (1 at the start), grouped by course; the others are held until
//! `send_next_course` releases the lowest one.

use std::collections::BTreeMap;

use pos_core::config::PosConfig;
use pos_core::order::OrderType;
use pos_db::queries::courses as db;
use pos_db::queries::courses::KitchenEntry;
use serde::{Deserialize, Serialize};

use crate::commands::access::{self, Access};
use crate::commands::bar_slip::{self, SlipItem, SlipMode, SlipOutcome};
use crate::commands::errors;
use crate::commands::owner_settings;
use crate::commands::signing::{self, Begin, Kind, Op};
use crate::state::AppState;

/// The key of the courses in the owner settings store.
pub const KEY_COURSES: &str = "courses";

/// A category and the course its products take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryCourse {
    pub category_id: i64,
    /// 1 to 3 (an unmapped category, or 0, is "Direct").
    pub course: i32,
}

/// The courses setting: off by default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourseSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub category_courses: Vec<CategoryCourse>,
}

/// A course waiting for "Envoyer la suite".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HeldCourseView {
    pub course: i32,
    pub lines: i64,
    pub quantity: i64,
}

/// What "Envoyer la suite" did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NextCourse {
    /// The course released; `None` when nothing was held and the call
    /// printed again what an earlier slip had not delivered.
    pub course: Option<i32>,
    /// The slip of the P that registered the unsent lines first.
    pub signed: Option<SlipOutcome>,
    /// The release slip.
    pub slip: SlipOutcome,
}

fn settings(db: &rusqlite::Connection) -> CourseSettings {
    owner_settings::get::<CourseSettings>(db, KEY_COURSES).unwrap_or_default()
}

/// Whether courses are switched on.
pub(crate) fn enabled(state: &AppState) -> bool {
    state
        .db
        .lock()
        .map(|db| settings(&db).enabled)
        .unwrap_or(false)
}

fn valid_course(course: i32) -> Result<(), String> {
    if (0..=db::MAX_COURSE).contains(&course) {
        Ok(())
    } else {
        Err(errors::coded(
            errors::COURSE_INVALID,
            &[("course", &course)],
        ))
    }
}

/// The settings, for any logged-in operator.
pub fn do_get_course_settings(state: &AppState) -> Result<CourseSettings, String> {
    access::operator(state, Access::Staff)?;
    let db = state.db.lock().map_err(errors::internal)?;
    Ok(settings(&db))
}

/// Store the settings (managers only). Courses 0 to 3, every category
/// must exist and appear once; a category on course 0 is dropped (an
/// unmapped category is "Direct"), so the stored document is canonical.
pub fn do_set_course_settings(
    state: &AppState,
    input: CourseSettings,
) -> Result<CourseSettings, String> {
    let operator = access::operator(state, Access::Admin)?;
    let db = state.db.lock().map_err(errors::internal)?;
    let mut seen = std::collections::BTreeSet::new();
    let mut mapped = Vec::new();
    for entry in &input.category_courses {
        valid_course(entry.course)?;
        let known: bool = db
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM categories WHERE id = ?1)",
                [entry.category_id],
                |r| r.get(0),
            )
            .map_err(errors::internal)?;
        if !known || !seen.insert(entry.category_id) {
            return Err(errors::coded(
                errors::COURSE_INVALID,
                &[("category", &entry.category_id)],
            ));
        }
        if entry.course > 0 {
            mapped.push(entry.clone());
        }
    }
    mapped.sort_by_key(|c| c.category_id);
    let stored = CourseSettings {
        enabled: input.enabled,
        category_courses: mapped,
    };
    owner_settings::set(&db, KEY_COURSES, &stored, Some(&operator.insz))?;
    Ok(stored)
}

/// A new line takes the course of its product's category (0 when
/// unmapped, or when courses are off). Called by the commands that add
/// a line, with the database lock held.
pub(crate) fn assign_new_line(
    db: &rusqlite::Connection,
    line_id: i64,
    order_id: i64,
    menu_item_id: i64,
) -> Result<(), String> {
    // A new line may reuse the id of a removed one.
    db::clear_line_course(db, line_id).map_err(errors::internal)?;
    let settings = settings(db);
    if !settings.enabled {
        return Ok(());
    }
    let category: Option<i64> = db
        .query_row(
            "SELECT category_id FROM menu_items WHERE id = ?1",
            [menu_item_id],
            |r| r.get(0),
        )
        .ok();
    let course = category
        .and_then(|c| {
            settings
                .category_courses
                .iter()
                .find(|m| m.category_id == c)
        })
        .map_or(0, |m| m.course);
    db::set_line_course(db, line_id, order_id, course, None).map_err(errors::internal)
}

/// Change a line's course, while no slip carried it.
pub fn do_set_line_course(state: &AppState, line_id: i64, course: i32) -> Result<(), String> {
    let operator = access::operator(state, Access::Staff)?;
    valid_course(course)?;
    let db = state.db.lock().map_err(errors::internal)?;
    if !settings(&db).enabled {
        return Err(errors::COURSES_DISABLED.into());
    }
    let (order_id, status): (i64, String) = db
        .query_row(
            "SELECT ol.order_id, o.status FROM order_lines ol
             JOIN orders o ON o.id = ol.order_id WHERE ol.id = ?1",
            [line_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => errors::LINE_NOT_FOUND.to_string(),
            other => errors::internal(other),
        })?;
    if status != "open" {
        return Err(errors::ORDER_NOT_OPEN.into());
    }
    if pos_db::queries::pending_signs::order_has_open(&db, order_id).map_err(errors::internal)? {
        return Err(errors::ORDER_SIGN_PENDING.into());
    }
    if db::line_was_printed(&db, line_id).map_err(errors::internal)? {
        return Err(errors::COURSE_ALREADY_SENT.into());
    }
    db::set_line_course(&db, line_id, order_id, course, Some(&operator.insz))
        .map_err(errors::internal)
}

/// Courses in the open table order that wait for "Envoyer la suite",
/// lowest first. Empty when courses are off or for a takeaway.
pub fn do_list_held_courses(
    state: &AppState,
    order_id: i64,
) -> Result<Vec<HeldCourseView>, String> {
    access::operator(state, Access::Staff)?;
    let db = state.db.lock().map_err(errors::internal)?;
    held(&db, order_id)
}

fn held(db: &rusqlite::Connection, order_id: i64) -> Result<Vec<HeldCourseView>, String> {
    if !settings(db).enabled {
        return Ok(Vec::new());
    }
    let order = pos_db::queries::orders::get_order(db, order_id).map_err(errors::internal)?;
    if order.order_type != OrderType::DineIn || order.table_number.is_none() {
        return Ok(Vec::new());
    }
    Ok(db::held_courses(db, order_id)
        .map_err(errors::internal)?
        .into_iter()
        .map(|h| HeldCourseView {
            course: h.course,
            lines: h.lines,
            quantity: h.quantity,
        })
        .collect())
}

/// The course waiting next at a table, for the table plan.
pub(crate) fn held_course(db: &rusqlite::Connection, order_id: i64) -> Option<i32> {
    held(db, order_id)
        .ok()
        .and_then(|h| h.first().map(|c| c.course))
}

/// The cart as the screen shows it: with courses off every line is
/// "Direct" and nothing is held; a takeaway holds nothing either.
pub(crate) fn present_cart(
    db: &rusqlite::Connection,
    order_id: i64,
    lines: &mut [pos_db::queries::orders::CartLine],
) {
    let on = settings(db).enabled;
    let can_hold = on
        && pos_db::queries::orders::get_order(db, order_id)
            .is_ok_and(|o| o.order_type == OrderType::DineIn && o.table_number.is_some());
    for line in lines {
        if !on {
            line.course = 0;
        }
        if !can_hold && line.kitchen_state == "held" {
            line.kitchen_state = "unsent".into();
        }
    }
}

/// How a slip is built from the entries that wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Print {
    /// After a P: the entries up to the released course, one line per
    /// entry (corrections as negative lines).
    Immediate,
    /// After the N of a direct sale: every course at once.
    Sale,
    /// A release (`Some`: the course just released) or a retry (`None`):
    /// the entries up to that course, netted per line.
    Release(Option<i32>),
}

/// Print the slip of what is signed and not printed yet, for `order_id`,
/// following `how`. Entries are recorded only when the slip is printed.
pub(crate) async fn print_signed(
    state: &AppState,
    config: &PosConfig,
    operator_insz: &str,
    order_id: i64,
    table: Option<i32>,
    how: Print,
) -> SlipOutcome {
    let (pending, released) = {
        let Ok(db) = state.db.lock() else {
            return SlipOutcome::Failed;
        };
        let pending = db::unprinted_entries(&db, order_id).unwrap_or_default();
        let released = db::released_course(&db, order_id).unwrap_or(1);
        (pending, released)
    };
    let limit = match how {
        Print::Sale => db::MAX_COURSE,
        Print::Immediate | Print::Release(None) => released,
        Print::Release(Some(course)) => course,
    };
    // A PRICE_CHANGE entry is never held: whatever its course, it is consumed by the first
    // slip pass after its P (it prints nothing), so it never waits for a release (INT-D6).
    let eligible: Vec<KitchenEntry> = pending
        .into_iter()
        .filter(|e| e.signed && (e.price_change || e.course == 0 || e.course <= limit))
        .collect();
    if eligible.is_empty() {
        return SlipOutcome::NothingToPrint;
    }

    let (mut items, consumed) = match how {
        Print::Immediate | Print::Sale => per_entry(&eligible),
        Print::Release(_) => net_per_line(&eligible),
    };
    // Headers only when a course above "Direct" is on the slip.
    if items.iter().all(|i| i.course == Some(0)) {
        for item in &mut items {
            item.course = None;
        }
    }
    let record = match how {
        Print::Immediate => "immediate",
        Print::Sale => "sale",
        Print::Release(_) => "release",
    };
    if items.is_empty() {
        // Entries that cancel out print nothing and are done with.
        bar_slip::record_printed(state, &consumed, record);
        return SlipOutcome::NothingToPrint;
    }
    let outcome = bar_slip::print_items(
        state,
        config,
        operator_insz,
        table,
        items,
        SlipMode {
            release: match how {
                Print::Release(course) => course,
                _ => None,
            },
            duplicate: false,
            record: Some(record),
        },
    )
    .await;
    if outcome == SlipOutcome::Printed {
        bar_slip::record_printed(state, &consumed, record);
    }
    outcome
}

/// The note an item carries: the line's note, under an item that orders
/// something (a correction prints without it).
fn note_of(e: &KitchenEntry, quantity: i32) -> (Option<String>, Option<i64>) {
    match &e.note {
        Some(note) if quantity > 0 => (Some(note.clone()), Some(e.line_id)),
        _ => (None, None),
    }
}

/// One item per entry; the entries of a PRICE_CHANGE pair print nothing
/// and are returned apart, to be recorded as done.
fn per_entry(entries: &[KitchenEntry]) -> (Vec<SlipItem>, Vec<i64>) {
    let (price_changes, ordered): (Vec<&KitchenEntry>, Vec<&KitchenEntry>) =
        entries.iter().partition(|e| e.price_change);
    let mut sorted = ordered;
    sorted.sort_by_key(|e| (e.course, e.entry_id));
    let items = sorted
        .into_iter()
        .map(|e| {
            let (note, note_line) = note_of(e, e.quantity);
            SlipItem {
                entry_ids: vec![e.entry_id],
                quantity: e.quantity,
                name: e.name.clone(),
                course: Some(e.course),
                note,
                note_line,
            }
        })
        .collect();
    (items, price_changes.iter().map(|e| e.entry_id).collect())
}

/// One item per line with the net quantity of its entries (a line
/// corrected back to 0 prints nothing); the entries of those lines and
/// of PRICE_CHANGE pairs are returned apart, to be recorded as done.
fn net_per_line(entries: &[KitchenEntry]) -> (Vec<SlipItem>, Vec<i64>) {
    let mut by_line: BTreeMap<i64, (SlipItem, &KitchenEntry)> = BTreeMap::new();
    let mut consumed = Vec::new();
    for e in entries {
        if e.price_change {
            consumed.push(e.entry_id);
            continue;
        }
        let (item, _) = by_line.entry(e.line_id).or_insert_with(|| {
            (
                SlipItem {
                    entry_ids: Vec::new(),
                    quantity: 0,
                    name: e.name.clone(),
                    course: Some(e.course),
                    note: None,
                    note_line: None,
                },
                e,
            )
        });
        item.entry_ids.push(e.entry_id);
        item.quantity += e.quantity;
    }
    let mut items = Vec::new();
    for (mut item, first) in by_line.into_values() {
        if item.quantity == 0 {
            consumed.extend(item.entry_ids);
        } else {
            (item.note, item.note_line) = note_of(first, item.quantity);
            items.push(item);
        }
    }
    items.sort_by_key(|i| (i.course, i.entry_ids.first().copied()));
    (items, consumed)
}

/// The slip that follows a signed P or the N of a sale, when courses are
/// on. `entries` are the journal entries that event registered.
pub(crate) async fn print_after_sign(
    state: &AppState,
    config: &PosConfig,
    operator_insz: &str,
    table: Option<i32>,
    order_id: i64,
) -> SlipOutcome {
    let how = if table.is_none() {
        Print::Sale
    } else {
        Print::Immediate
    };
    print_signed(state, config, operator_insz, order_id, table, how).await
}

/// "Envoyer la suite": sign what is not registered yet (so the kitchen
/// only ever gets signed lines), release the lowest held course and
/// print it. With the FDM down and unsent lines, nothing is signed,
/// released or printed. A course whose lines are already signed prints
/// without the FDM.
pub async fn do_send_next_course(state: &AppState, order_id: i64) -> Result<NextCourse, String> {
    let operator = access::operator(state, Access::Staff)?;
    let config = state.config.lock().map_err(errors::internal)?.clone();
    let (table, target, has_unsent) = {
        let db = state.db.lock().map_err(errors::internal)?;
        if !settings(&db).enabled {
            return Err(errors::COURSES_DISABLED.into());
        }
        let order = pos_db::queries::orders::get_order(&db, order_id).map_err(errors::internal)?;
        if order.status != pos_core::order::OrderStatus::Open {
            return Err(errors::ORDER_NOT_OPEN.into());
        }
        let target = held(&db, order_id)?.first().map(|h| h.course);
        let unsent = pos_db::queries::orders::get_order_entries(&db, order_id, true)
            .map_err(errors::internal)?;
        (order.table_number, target, !unsent.is_empty())
    };

    let mut signed = None;
    if target.is_some() && has_unsent {
        let op = Op::new(Kind::TableOrder, order_id);
        signed = match signing::begin(state, &op).await? {
            Begin::Fresh(guard) => {
                let registered = crate::commands::tables::sign_table_order_locked(
                    state, &guard, &config, order_id, &operator,
                )
                .await?;
                drop(guard);
                Some(crate::commands::tables::print_registered(state, &config, registered).await)
            }
            Begin::AlreadyDone(recovered) => recovered.follow_up.slip,
        };
    }

    let Some(course) = target else {
        // Nothing is held: print again what an earlier slip did not
        // deliver (a printer that failed), or say there is nothing.
        let slip = print_signed(
            state,
            &config,
            &operator.insz,
            order_id,
            table,
            Print::Release(None),
        )
        .await;
        if slip == SlipOutcome::NothingToPrint {
            return Err(errors::NO_COURSE_HELD.into());
        }
        return Ok(NextCourse {
            course: None,
            signed: None,
            slip,
        });
    };
    {
        let db = state.db.lock().map_err(errors::internal)?;
        db::release(&db, order_id, course, Some(&operator.insz)).map_err(errors::internal)?;
    }
    let slip = print_signed(
        state,
        &config,
        &operator.insz,
        order_id,
        table,
        Print::Release(Some(course)),
    )
    .await;
    Ok(NextCourse {
        course: Some(course),
        signed,
        slip,
    })
}

/// Print again what was printed for `course` of the order, marked
/// DUPLICATA. `nothing_printed` when no slip carried that course.
pub async fn do_reprint_course_slip(
    state: &AppState,
    order_id: i64,
    course: i32,
) -> Result<SlipOutcome, String> {
    let operator = access::operator(state, Access::Staff)?;
    valid_course(course)?;
    let config = state.config.lock().map_err(errors::internal)?.clone();
    let (printed, table, was_released) = {
        let db = state.db.lock().map_err(errors::internal)?;
        let order = pos_db::queries::orders::get_order(&db, order_id).map_err(errors::internal)?;
        (
            db::printed_entries(&db, order_id, course).map_err(errors::internal)?,
            order.table_number,
            db::was_released(&db, order_id, course).map_err(errors::internal)?,
        )
    };
    let (items, _) = net_per_line(&printed);
    if items.is_empty() {
        return Err(errors::NOTHING_PRINTED.into());
    }
    let items = items
        .into_iter()
        .map(|mut i| {
            i.course = None;
            i
        })
        .collect();
    Ok(bar_slip::print_items(
        state,
        &config,
        &operator.insz,
        table,
        items,
        SlipMode {
            release: was_released.then_some(course),
            duplicate: true,
            record: None,
        },
    )
    .await)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: i64, line: i64, quantity: i32, course: i32) -> KitchenEntry {
        KitchenEntry {
            entry_id: id,
            line_id: line,
            quantity,
            name: format!("l{line}"),
            course,
            signed: true,
            price_change: false,
            note: None,
        }
    }

    #[test]
    fn a_line_corrected_back_to_zero_prints_nothing_and_is_consumed() {
        let (items, consumed) = net_per_line(&[
            entry(1, 10, 2, 2),
            entry(2, 10, -2, 2),
            entry(3, 11, 3, 2),
            entry(4, 11, -1, 2),
        ]);
        assert_eq!(items.len(), 1);
        assert_eq!(
            (items[0].quantity, items[0].entry_ids.clone()),
            (2, vec![3, 4])
        );
        assert_eq!(consumed, vec![1, 2]);
    }

    #[test]
    fn netted_items_come_out_by_course_then_age() {
        let (items, _) = net_per_line(&[entry(5, 1, 1, 3), entry(6, 2, 1, 2), entry(7, 3, 1, 0)]);
        let order: Vec<_> = items.iter().map(|i| i.course).collect();
        assert_eq!(order, vec![Some(0), Some(2), Some(3)]);
    }

    #[test]
    fn course_zero_is_not_a_category_course_and_the_defaults_are_off() {
        let s = CourseSettings::default();
        assert!(!s.enabled && s.category_courses.is_empty());
        assert!(valid_course(0).is_ok() && valid_course(3).is_ok());
        assert!(valid_course(4).is_err() && valid_course(-1).is_err());
    }
}
