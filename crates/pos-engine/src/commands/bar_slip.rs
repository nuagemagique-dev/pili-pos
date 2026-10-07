//! Bar / kitchen order slip, printed once the order lines it carries
//! have been signed (table order P, or the sale N of a direct sale).
//!
//! The slip lists only the lines just transmitted, corrections
//! included, for the products flagged `kitchen_printer`. It is not a
//! fiscal document: a printer failure never undoes the signing, the
//! caller reports the outcome to the UI instead.

use serde::Serialize;

use pos_core::config::PosConfig;
use pos_core::order::OrderLine;
use pos_fiscal::event::now_in_belgium;
use pos_fiscal::ticket::ReceiptLanguage;
use pos_print::bar_ticket::{BarItem, BarTicket, format_bar_ticket};

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

/// Print the slip for `entries`, the order entries just signed.
pub(crate) async fn print_bar_slip(
    state: &AppState,
    config: &PosConfig,
    operator_insz: &str,
    table: Option<i32>,
    entries: &[OrderLine],
) -> SlipOutcome {
    let items: Vec<BarItem> = {
        let Ok(db) = state.db.lock() else {
            return SlipOutcome::Failed;
        };
        entries
            .iter()
            .filter(|e| e.quantity != 0)
            .filter(|e| {
                pos_db::queries::menu::sends_to_kitchen(&db, e.menu_item_id).unwrap_or_else(|err| {
                    tracing::warn!(%err, menu_item_id = e.menu_item_id, "kitchen flag unreadable");
                    true
                })
            })
            .map(|e| BarItem {
                quantity: e.quantity,
                name: e.name.clone(),
            })
            .collect()
    };
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
        items,
        lang: ReceiptLanguage::from_code(&config.language),
        is_training: config.training_mode,
    };
    match pos_print::transport::send_to_printer_with(
        addr,
        &format_bar_ticket(&ticket),
        state.timers.printer_status_read,
    )
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
