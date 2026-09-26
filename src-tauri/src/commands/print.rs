use tauri::State;

#[cfg(feature = "hubrise")]
use pos_hubrise_types::{Order, ServiceType};
#[cfg(feature = "hubrise")]
use pos_print::customer_slip::{CustomerInfo, CustomerLine, CustomerSlip, format_customer_slip};
#[cfg(feature = "hubrise")]
use pos_print::kitchen_ticket::{KitchenItem, KitchenTicket, format_kitchen_ticket};
use pos_print::transport;
#[cfg(feature = "hubrise")]
use rusqlite::params;

use crate::state::AppState;

// There is deliberately no command to print a receipt or open the
// drawer on demand: an original ticket is printed once, by the command
// that signed it (a second print is a copy, event C, POS spec v2.2
// §8.1), and the drawer only opens through a signed event (a cash
// sale or refund, or the F event of `financial::open_drawer_fiscal`).

/// Check if the printer is reachable.
#[tauri::command]
pub async fn check_printer(state: State<'_, AppState>) -> Result<bool, String> {
    let printer_address = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .printer_address
        .clone();

    match printer_address {
        Some(address) => {
            let printer = transport::TcpPrinter::new(&address);
            Ok(printer.ping().await)
        }
        None => Ok(false),
    }
}

// ── Webshop (non-fiscal) tickets ──────────────────────────────────
//
// Everything below is HubRise/webshop-specific (loads orders from
// `raw_hubrise_json` and emits non-fiscal kitchen/customer slips).
// The certification build excludes the `hubrise` feature, so this
// whole section disappears from that binary.

#[cfg(feature = "hubrise")]
/// Canonical "Printer not configured" error string. The UI maps
/// this to an i18n toast so it can present a friendly message
/// without parsing transport errors.
const ERR_PRINTER_NOT_CONFIGURED: &str = crate::commands::errors::PRINTER_NOT_CONFIGURED;

/// Load and deserialize the stored HubRise Order JSON for a local
/// `orders.id`. Returns the printer-ready payload or a user-facing
/// error message on failure.
#[cfg(feature = "hubrise")]
fn load_webshop_order(state: &State<'_, AppState>, order_id: i64) -> Result<Order, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let raw: Option<String> = db
        .query_row(
            "SELECT raw_hubrise_json FROM orders WHERE id = ?1 AND is_fiscal = 0",
            params![order_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let raw = raw.ok_or_else(|| "order has no HubRise payload".to_string())?;
    serde_json::from_str::<Order>(&raw).map_err(|e| e.to_string())
}

/// Pull the configured printer address from `AppState.config` or
/// return the canonical not-configured error.
#[cfg(feature = "hubrise")]
fn printer_address(state: &State<'_, AppState>) -> Result<String, String> {
    state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .printer_address
        .clone()
        .ok_or_else(|| ERR_PRINTER_NOT_CONFIGURED.to_string())
}

/// Map the stored HubRise service_type to a French label for
/// printing. Matches the labels used on the POS cards.
#[cfg(feature = "hubrise")]
fn service_type_label(svc: Option<ServiceType>) -> String {
    match svc {
        Some(ServiceType::Delivery) => "Livraison",
        Some(ServiceType::Collection) => "A emporter",
        Some(ServiceType::EatIn) => "Sur place",
        None => "-",
    }
    .to_string()
}

/// First 8 chars of a HubRise order id, or `"--------"` if absent.
#[cfg(feature = "hubrise")]
fn short_ref(order: &Order) -> String {
    order.id.chars().take(8).collect::<String>()
}

/// Parse a HubRise money string like `"12.50 EUR"` into eurocents.
#[cfg(feature = "hubrise")]
fn money_cents(money: &str) -> i64 {
    let n = money.split_whitespace().next().unwrap_or("0");
    let parsed: f64 = n.parse().unwrap_or(0.0);
    (parsed * 100.0).round() as i64
}

/// Extract "HH:MM" from an RFC3339 timestamp, or None on failure.
#[cfg(feature = "hubrise")]
fn hhmm_from_rfc3339(rfc3339: &str) -> Option<String> {
    // Expected shape: YYYY-MM-DDTHH:MM:SS(Z|+HH:MM)
    if rfc3339.len() < 16 {
        return None;
    }
    // "HH:MM" lives at [11..16]
    Some(rfc3339[11..16].to_string())
}

/// Print a kitchen ticket (items + options + notes, non-fiscal).
#[cfg(feature = "hubrise")]
#[tauri::command]
pub async fn print_webshop_kitchen(
    state: State<'_, AppState>,
    order_id: i64,
) -> Result<(), String> {
    let order = load_webshop_order(&state, order_id)?;
    let address = printer_address(&state)?;

    let items: Vec<KitchenItem> = order
        .items
        .iter()
        .map(|it| KitchenItem {
            quantity: it.quantity.parse::<i64>().unwrap_or(1),
            product_name: it.product_name.clone(),
            options: it.options.iter().map(|o| o.name.clone()).collect(),
            notes: it.customer_notes.clone(),
        })
        .collect();

    let ticket = KitchenTicket {
        source: order.channel.clone().unwrap_or_else(|| "Web".to_string()),
        service_type: service_type_label(order.service_type),
        short_ref: short_ref(&order),
        ready_at: order.confirmed_time.as_deref().and_then(hhmm_from_rfc3339),
        items,
        order_notes: order.customer_notes.clone(),
    };

    let bytes = format_kitchen_ticket(&ticket);
    transport::send_to_printer(&address, &bytes)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Print a customer slip (items + total + customer, non-fiscal).
#[cfg(feature = "hubrise")]
#[tauri::command]
pub async fn print_webshop_customer(
    state: State<'_, AppState>,
    order_id: i64,
) -> Result<(), String> {
    let order = load_webshop_order(&state, order_id)?;
    let address = printer_address(&state)?;

    // Restaurant header pulled from AppState.config (configured in
    // `state.rs` at POS boot).
    let (restaurant_name, restaurant_address) = {
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        (cfg.restaurant_name.clone(), cfg.restaurant_address.clone())
    };

    let lines: Vec<CustomerLine> = order
        .items
        .iter()
        .map(|it| {
            let qty = it.quantity.parse::<i64>().unwrap_or(1);
            let unit = money_cents(&it.price);
            CustomerLine {
                quantity: qty,
                product_name: it.product_name.clone(),
                line_total_cents: unit * qty,
            }
        })
        .collect();

    let total_cents = order
        .total
        .as_deref()
        .map(money_cents)
        .unwrap_or_else(|| lines.iter().map(|l| l.line_total_cents).sum());

    let customer = order
        .customer
        .as_ref()
        .map(|c| {
            let name = match (&c.first_name, &c.last_name) {
                (Some(f), Some(l)) => Some(format!("{} {}", f, l)),
                (Some(f), None) => Some(f.clone()),
                (None, Some(l)) => Some(l.clone()),
                (None, None) => None,
            };
            CustomerInfo {
                name,
                phone: c.phone.clone(),
                address_1: c.address_1.clone(),
                postal_code: c.postal_code.clone(),
                city: c.city.clone(),
            }
        })
        .unwrap_or_default();

    let printed_at = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();

    let slip = CustomerSlip {
        restaurant_name,
        restaurant_address,
        printed_at,
        source: order.channel.clone().unwrap_or_else(|| "Web".to_string()),
        service_type: service_type_label(order.service_type),
        short_ref: short_ref(&order),
        lines,
        total_cents,
        customer,
    };

    let bytes = format_customer_slip(&slip);
    transport::send_to_printer(&address, &bytes)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}
