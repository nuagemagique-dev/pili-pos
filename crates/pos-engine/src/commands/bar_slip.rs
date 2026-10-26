//! Bar / kitchen order slip, printed once the order lines it carries
//! have been signed (table order P, or the sale N of a direct sale).
//!
//! The slip lists only the lines just transmitted, corrections
//! included, for the products flagged `kitchen_printer`. It is not a
//! fiscal document: a printer failure never undoes the signing, the
//! caller reports the outcome to the UI instead.
//!
//! A line's kitchen note (migration 31) prints under its item; a note
//! changed after its line went to the kitchen prints on its own note
//! slip. The note is local: it never reaches a fiscal message.

use serde::Serialize;

use pos_core::config::PosConfig;
use pos_core::order::OrderLine;
use pos_fiscal::event::now_in_belgium;
use pos_fiscal::ticket::ReceiptLanguage;
use pos_print::bar_ticket::{
    BarItem, BarTicket, NoteSlip, TransferSlip, format_bar_ticket, format_note_slip,
    format_transfer_slip,
};

use crate::state::AppState;

/// What became of the slip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SlipOutcome {
    Printed,
    /// No new line for the bar or kitchen.
    NothingToPrint,
    /// Neither `kitchen_printer_address` nor `printer_address` is set.
    NoPrinter,
    Failed,
}

/// The printer the slip goes to: the kitchen printer, else the ticket
/// printer.
fn slip_printer(config: &PosConfig) -> Option<&str> {
    config
        .kitchen_printer_address
        .as_deref()
        .or(config.printer_address.as_deref())
        .filter(|addr| !addr.is_empty())
}

/// One line of a slip, with the journal entries it stands for (they are
/// recorded as printed once the printer reports `Printed`).
#[derive(Debug, Clone)]
pub(crate) struct SlipItem {
    pub entry_ids: Vec<i64>,
    pub quantity: i32,
    pub name: String,
    /// `Some` groups the slip by course; `None` is the slip as it
    /// was before courses existed.
    pub course: Option<i32>,
    /// The line's kitchen note, printed under the item (migration 31).
    pub note: Option<String>,
    /// The line whose note this item carries: its note is marked printed
    /// once the slip is (never for a reprint).
    pub note_line: Option<i64>,
}

/// How a slip differs from the plain one.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SlipMode {
    /// The course a release slip sends ("SUITE : Plats").
    pub release: Option<i32>,
    /// A reprint (DUPLICATA).
    pub duplicate: bool,
    /// Record the printed entries with this `kitchen_slip_entries.slip`
    /// value (`immediate`, `release`, `sale`); `None` records nothing.
    pub record: Option<&'static str>,
}

/// Print the slip for `entries`, the order entries just signed.
pub(crate) async fn print_bar_slip(
    state: &AppState,
    config: &PosConfig,
    operator_insz: &str,
    table: Option<i32>,
    entries: &[OrderLine],
) -> SlipOutcome {
    let items: Vec<SlipItem> = {
        let Ok(db) = state.db.lock() else {
            return SlipOutcome::Failed;
        };
        // The entries of a PRICE_CHANGE pair (a till discount) order
        // nothing: they never print.
        #[cfg(feature = "hubrise")]
        let price_changes = entries
            .first()
            .map(|e| pos_db::queries::till_discounts::price_change_entry_ids(&db, e.order_id))
            .transpose()
            .unwrap_or_else(|err| {
                tracing::warn!(%err, "price change entries unreadable");
                None
            })
            .unwrap_or_default();
        #[cfg(not(feature = "hubrise"))]
        let price_changes = std::collections::BTreeSet::<i64>::new();
        let notes = pos_db::queries::orders::entry_notes(
            &db,
            &entries.iter().map(|e| e.id).collect::<Vec<_>>(),
        )
        .unwrap_or_else(|err| {
            tracing::warn!(%err, "kitchen notes unreadable");
            std::collections::BTreeMap::default()
        });
        entries
            .iter()
            .filter(|e| e.quantity != 0)
            .filter(|e| !price_changes.contains(&e.id))
            .filter(|e| {
                pos_db::queries::menu::sends_to_kitchen(&db, e.menu_item_id).unwrap_or_else(|err| {
                    tracing::warn!(%err, menu_item_id = e.menu_item_id, "kitchen flag unreadable");
                    true
                })
            })
            .map(|e| {
                // A correction (negative quantity) prints without a note.
                let (note_line, note) = notes
                    .get(&e.id)
                    .filter(|_| e.quantity > 0)
                    .and_then(|(line_id, note)| note.clone().map(|n| (Some(*line_id), Some(n))))
                    .unwrap_or((None, None));
                SlipItem {
                    entry_ids: vec![e.id],
                    quantity: e.quantity,
                    name: e.name.clone(),
                    course: None,
                    note,
                    note_line,
                }
            })
            .collect()
    };
    let record = if table.is_some() { "immediate" } else { "sale" };
    print_items(
        state,
        config,
        operator_insz,
        table,
        items,
        SlipMode {
            record: Some(record),
            ..SlipMode::default()
        },
    )
    .await
}

/// Print a slip made of `items`. `NothingToPrint` for no item, `NoPrinter`
/// without an address; the entries are recorded as printed only when the
/// printer reports it ready.
pub(crate) async fn print_items(
    state: &AppState,
    config: &PosConfig,
    operator_insz: &str,
    table: Option<i32>,
    items: Vec<SlipItem>,
    mode: SlipMode,
) -> SlipOutcome {
    if items.is_empty() {
        return SlipOutcome::NothingToPrint;
    }
    let Some(addr) = slip_printer(config) else {
        return SlipOutcome::NoPrinter;
    };

    let ticket = BarTicket {
        date_time: now_in_belgium().format("%d/%m/%Y %H:%M:%S").to_string(),
        operator: super::operator_name(state, operator_insz),
        table,
        items: items
            .iter()
            .map(|i| BarItem {
                quantity: i.quantity,
                name: i.name.clone(),
                note: i.note.clone(),
                course: i.course,
            })
            .collect(),
        lang: ReceiptLanguage::from_code(&config.language),
        is_training: config.training_mode,
        release: mode.release,
        duplicate: mode.duplicate,
    };
    let outcome = send(state, addr, &format_bar_ticket(&ticket)).await;
    if outcome == SlipOutcome::Printed {
        if let Some(kind) = mode.record {
            let ids: Vec<i64> = items.iter().flat_map(|i| i.entry_ids.clone()).collect();
            record_printed(state, &ids, kind);
        }
        let noted_lines: Vec<i64> = items.iter().filter_map(|i| i.note_line).collect();
        if !mode.duplicate && !noted_lines.is_empty() {
            mark_notes_printed(state, &noted_lines);
        }
    }
    outcome
}

/// The notes of `line_ids` have reached the kitchen.
fn mark_notes_printed(state: &AppState, line_ids: &[i64]) {
    let Ok(db) = state.db.lock() else { return };
    for line_id in line_ids {
        if let Err(err) = pos_db::queries::orders::mark_kitchen_note_printed(&db, *line_id) {
            tracing::warn!(%err, line_id, "kitchen note print mark not stored");
        }
    }
}

/// Print the slip of a note changed after its line went to the
/// kitchen: the product, the new note or the removal; no quantity, no
/// amount. On `Printed` the line's printed copy follows the note, so
/// confirming an unchanged note again prints nothing.
pub(crate) async fn print_note_slip(
    state: &AppState,
    config: &PosConfig,
    operator_insz: &str,
    line_id: i64,
    table: Option<i32>,
    product_name: &str,
    note: Option<&str>,
) -> SlipOutcome {
    let Some(addr) = slip_printer(config) else {
        return SlipOutcome::NoPrinter;
    };
    let slip = NoteSlip {
        date_time: now_in_belgium().format("%d/%m/%Y %H:%M:%S").to_string(),
        operator: super::operator_name(state, operator_insz),
        table,
        product_name: product_name.to_string(),
        note: note.map(str::to_string),
        lang: ReceiptLanguage::from_code(&config.language),
        is_training: config.training_mode,
    };
    let outcome = send(state, addr, &format_note_slip(&slip)).await;
    if outcome == SlipOutcome::Printed {
        mark_notes_printed(state, &[line_id]);
    }
    outcome
}

/// Print the notice that the order of table `from_table` moved to
/// `to_table`: no item, no amount, the kitchen only learns where the
/// dishes go.
pub(crate) async fn print_transfer_slip(
    state: &AppState,
    config: &PosConfig,
    operator_insz: &str,
    from_table: i32,
    to_table: i32,
) -> SlipOutcome {
    let Some(addr) = slip_printer(config) else {
        return SlipOutcome::NoPrinter;
    };
    let slip = TransferSlip {
        date_time: now_in_belgium().format("%d/%m/%Y %H:%M:%S").to_string(),
        operator: super::operator_name(state, operator_insz),
        from_table,
        to_table,
        lang: ReceiptLanguage::from_code(&config.language),
        is_training: config.training_mode,
    };
    send(state, addr, &format_transfer_slip(&slip)).await
}

async fn send(state: &AppState, addr: &str, bytes: &[u8]) -> SlipOutcome {
    match pos_print::transport::send_to_printer_with(addr, bytes, state.timers.printer_status_read)
        .await
    {
        Ok(status) if status.ready() => SlipOutcome::Printed,
        Ok(status) => {
            tracing::warn!(?status.warnings, addr, "bar slip printer not ready");
            SlipOutcome::Failed
        }
        Err(err) => {
            tracing::warn!(%err, addr, "bar slip not printed");
            SlipOutcome::Failed
        }
    }
}

/// Remember which journal entries a slip printed. Kitchen bookkeeping:
/// a failure is logged and never undoes the slip.
pub(crate) fn record_printed(state: &AppState, entry_ids: &[i64], kind: &str) {
    let Ok(db) = state.db.lock() else {
        return;
    };
    if let Err(err) = pos_db::queries::courses::record_printed(&db, entry_ids, kind) {
        tracing::warn!(%err, "printed kitchen entries not recorded");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vi_bar_the_kitchen_printer_comes_first() {
        let mut config = PosConfig {
            printer_address: Some("10.0.0.2:9100".into()),
            kitchen_printer_address: Some("10.0.0.3:9100".into()),
            ..Default::default()
        };
        assert_eq!(slip_printer(&config), Some("10.0.0.3:9100"));
        config.kitchen_printer_address = None;
        assert_eq!(slip_printer(&config), Some("10.0.0.2:9100"));
        config.printer_address = None;
        assert_eq!(slip_printer(&config), None);
    }
}
