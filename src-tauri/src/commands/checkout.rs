use serde::Serialize;
use tauri::State;

use pos_fiscal::archive::{self, ArchiveParams};
use pos_fiscal::counter::FiscalCounter;
use pos_fiscal::event::build_sale_input_split;
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::payment::{PaymentError, Settlement, payment_line_identity};
use pos_fiscal::response::SignResult;
use pos_fiscal::ticket::{FiscalTicket, TicketLine, TicketParams, TicketPayment};
use pos_fiscal::types::EventLabel;

use crate::commands::access::{self, Access};
use crate::commands::bar_slip::{self, SlipOutcome};
use crate::commands::delivery::{self, DeliveryOutcome, TicketDeliveryRequest};
use crate::commands::errors;
#[cfg(feature = "hubrise")]
use crate::commands::hubrise_push;
use crate::state::AppState;

/// A payment method the operator takes: cash, card or meal vouchers.
pub use pos_fiscal::payment::PaymentMethod as CheckoutPaymentMethod;
pub use pos_fiscal::payment::PaymentPart;

#[derive(Debug, Serialize)]
pub struct CheckoutResult {
    pub success: bool,
    pub ticket: Option<FiscalTicket>,
    pub error: Option<String>,
    pub fdm_connected: bool,
    /// Archive id of the signed event.
    pub fiscal_event_id: Option<i64>,
    /// How the ticket reached the customer (paper and/or email).
    pub delivery: Option<DeliveryOutcome>,
    /// The bar / kitchen slip of the lines not yet sent (sales only).
    pub kitchen_slip: Option<SlipOutcome>,
}

impl CheckoutResult {
    /// A checkout refused before anything was signed.
    pub fn refused(error: impl Into<String>) -> Self {
        Self {
            success: false,
            ticket: None,
            error: Some(error.into()),
            fdm_connected: false,
            fiscal_event_id: None,
            delivery: None,
            kitchen_slip: None,
        }
    }
}

#[tauri::command]
pub async fn checkout(
    state: State<'_, AppState>,
    order_id: i64,
    payments: Vec<PaymentPart>,
    delivery: Option<TicketDeliveryRequest>,
) -> Result<CheckoutResult, String> {
    do_checkout_split(state.inner(), order_id, &payments, delivery.as_ref()).await
}

/// Inner implementation of [`checkout`], decoupled from Tauri's
/// `State` wrapper so integration tests can drive it directly. The
/// wrapper above is the production entry point; everything below is
/// the same code, just reachable from `cargo test`.
pub async fn do_checkout(
    state: &AppState,
    order_id: i64,
    payment_method: CheckoutPaymentMethod,
) -> Result<CheckoutResult, String> {
    do_checkout_with(state, order_id, payment_method, None).await
}

/// [`do_checkout`] with the operator's ticket choice (paper, email or
/// both). The medium is part of the signed request, so it is settled
/// before the FDM call.
pub async fn do_checkout_with(
    state: &AppState,
    order_id: i64,
    payment_method: CheckoutPaymentMethod,
    delivery_request: Option<&TicketDeliveryRequest>,
) -> Result<CheckoutResult, String> {
    // One method pays the whole sale: its amount is the order total
    // (cash always pays the rest).
    let part = match payment_method {
        CheckoutPaymentMethod::Cash => PaymentPart::cash(),
        method => {
            let total = {
                let db = state.db.lock().map_err(|e| e.to_string())?;
                pos_db::queries::orders::get_order(&db, order_id)
                    .map(|o| o.total_cents)
                    .unwrap_or(0)
            };
            PaymentPart::of(method, total)
        }
    };
    do_checkout_split(state, order_id, &[part], delivery_request).await
}

/// A sale paid in one or several parts (card, meal vouchers, cash for
/// the rest), with the operator's ticket choice. See
/// [`pos_fiscal::payment::settle`] for the rules.
pub async fn do_checkout_split(
    state: &AppState,
    order_id: i64,
    parts: &[PaymentPart],
    delivery_request: Option<&TicketDeliveryRequest>,
) -> Result<CheckoutResult, String> {
    // A sale is signed in the name of the cashier taking the payment.
    let cashier = match access::operator(state, Access::Cashier) {
        Ok(user) => user,
        Err(e) => return Ok(CheckoutResult::refused(e)),
    };
    // App and online payments come with online orders only, never
    // from the till.
    if parts.iter().any(|p| !p.method.is_offered_at_till()) {
        return Ok(CheckoutResult::refused(payment_error(
            PaymentError::Invalid,
        )));
    }
    sell(
        state,
        cashier,
        order_id,
        Payment::Parts(parts),
        delivery_request,
        SaleOptions::default(),
    )
    .await
}

/// How a sale differs from a sale at the till. The default is the
/// till's; only online orders (feature `hubrise`) set anything.
#[derive(Debug, Clone, Default)]
pub(crate) struct SaleOptions {
    /// The online order's cost center (platform or webshop), in place
    /// of the table's.
    pub cost_center: Option<pos_fiscal::inputs::CostCenterInput>,
    /// An online order: no echo to HubRise (it came from there), no
    /// bar slip (the caller prints the online kitchen ticket).
    pub online: bool,
}

/// How a sale is paid.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Payment<'a> {
    /// The parts the operator took (see [`pos_fiscal::payment::settle`]).
    Parts(&'a [PaymentPart]),
    /// Closed without payment: the customer left without paying
    /// (Advanced Set UC4109). The sale is signed with no payment line.
    Unpaid,
}

/// Sign the sale of an open order in `operator`'s name and close it.
pub(crate) async fn sell(
    state: &AppState,
    operator: pos_core::user::User,
    order_id: i64,
    payment: Payment<'_>,
    delivery_request: Option<&TicketDeliveryRequest>,
    options: SaleOptions,
) -> Result<CheckoutResult, String> {
    let cashier = operator;
    // 1. Read order + lines from DB
    // `lines` is the consolidated order (printed); `entries` the journal
    // of every addition and correction, sent to the FDM (§2).
    // `unsent` are the entries no table order has registered yet: the
    // bar / kitchen slip prints them once the sale is signed.
    let (order, lines, entries, unsent, config, booking_period_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let order = pos_db::queries::orders::get_order(&db, order_id).map_err(|e| e.to_string())?;
        let lines =
            pos_db::queries::orders::get_order_lines(&db, order_id).map_err(|e| e.to_string())?;
        let entries = pos_db::queries::orders::get_order_entries(&db, order_id, false)
            .map_err(|e| e.to_string())?;
        let unsent = pos_db::queries::orders::get_order_entries(&db, order_id, true)
            .map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = super::booking(&db, &config)?;
        (order, lines, entries, unsent, config, bp)
    };
    let mut order = order;
    order.user_insz = cashier.insz;

    if lines.is_empty() {
        return Ok(CheckoutResult {
            success: false,
            ticket: None,
            error: Some(errors::ORDER_NO_LINES.into()),
            fdm_connected: false,
            fiscal_event_id: None,
            delivery: None,
            kitchen_slip: None,
        });
    }

    // Guard: no negative sale (a refund goes through `refund.rs`). A
    // sale that comes to 0,00 EUR is signed like any other: event N
    // and a VAT ticket (producers FAQ Q206, Q209). Together with the
    // empty-order check above, no record without a product line can
    // be made (manual §III.D).
    if order.total_cents < 0 {
        return Ok(CheckoutResult {
            success: false,
            ticket: None,
            error: Some(errors::ORDER_TOTAL_NEGATIVE.into()),
            fdm_connected: false,
            fiscal_event_id: None,
            delivery: None,
            kitchen_slip: None,
        });
    }

    // Guard: reject if order is already closed (prevents double
    // fiscal events when the operator taps "Payer" twice before
    // the Vue loading guard kicks in).
    if order.status != pos_core::order::OrderStatus::Open {
        return Ok(CheckoutResult {
            success: false,
            ticket: None,
            error: Some(errors::ORDER_ALREADY_CLOSED.into()),
            fdm_connected: false,
            fiscal_event_id: None,
            delivery: None,
            kitchen_slip: None,
        });
    }

    // An online order is signed by its own commands (acceptance,
    // handover), with its cost center: never as a till sale.
    #[cfg(feature = "hubrise")]
    if !options.online {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let online: bool = db
            .query_row(
                "SELECT hubrise_order_id IS NOT NULL FROM orders WHERE id = ?1",
                rusqlite::params![order_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if online {
            return Ok(CheckoutResult::refused(errors::ORDER_NOT_OPEN));
        }
    }

    // Ticket medium: refused choices stop here, before any signing.
    let resolved = match delivery::resolve(delivery_request, &config, &state.mail) {
        Ok(r) => r,
        Err(e) => return Ok(CheckoutResult::refused(e)),
    };

    // The payment: card and meal voucher parts are exact, cash pays the
    // rest, rounded to 5 cents (a sale at 0,00 EUR is a cash payment of
    // 0, FAQ Q206). Refused before anything is signed.
    let settlement = match payment {
        Payment::Parts(parts) => match pos_fiscal::payment::settle(order.total_cents, parts) {
            Ok(s) => s,
            Err(e) => return Ok(CheckoutResult::refused(payment_error(e))),
        },
        // Nothing paid: no payment line at all (UC4109 sends
        // `financials: []`). A sale of 0,00 EUR is paid normally.
        Payment::Unpaid if order.total_cents > 0 => Settlement {
            payments: Vec::new(),
            rounding_cents: 0,
        },
        Payment::Unpaid => return Ok(CheckoutResult::refused(errors::UNPAID_NOTHING_DUE)),
    };
    let unpaid = matches!(payment, Payment::Unpaid);

    // 2. RESERVE (peek, don't commit) the next sequence number.
    //
    // We deliberately do NOT call `FiscalCounter::next` here yet.
    // Allocating before the FDM round-trip would create a gap in
    // the sequence whenever the FDM call fails - a direct violation
    // of the SCE 2.0 "strictly sequential, no gaps" requirement.
    //
    // Instead we peek (read-only) to obtain the value we'll send to
    // the FDM, then commit the increment inside the same SQLite
    // transaction that writes the archive (step 6-8 below). If the
    // FDM fails, the counter stays untouched and the next attempt
    // sees the same value - combined with the FDM's idempotency on
    // `posFiscalTicketNo`, this is safe to retry indefinitely.
    let ticket_number = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let label = if config.training_mode {
            EventLabel::T
        } else {
            EventLabel::N
        };
        FiscalCounter::peek_next(&db, label).map_err(|e| e.to_string())?
    };

    let mut sale_input = build_sale_input_split(
        &config,
        &order,
        &entries,
        ticket_number,
        &booking_period_id,
        &settlement,
    );
    sale_input.common.ticket_medium = resolved.medium;
    if let Some(cost_center) = options.cost_center.clone() {
        sale_input.cost_center = Some(cost_center);
    }

    // 4. Serialize request for archival
    let request_json = serde_json::to_string(&sale_input).map_err(|e| e.to_string())?;

    // 5. Send to FDM
    let fdm = FdmClient::from_config(&config);
    let fdm_result: Result<SignResult, _> = fdm.sign_sale(sale_input, config.training_mode).await;

    let (sign_result, fdm_connected) = match fdm_result {
        Ok(result) => (Some(result), true),
        Err(pos_fiscal::fdm_client::FdmError::NotConnected) => {
            // No FDM, no sale: the POS blocks (SCE 2.0). There is no
            // offline mode, in any build.
            return Ok(CheckoutResult {
                success: false,
                ticket: None,
                error: Some("FDM not connected - cannot complete transaction".into()),
                fdm_connected: false,
                fiscal_event_id: None,
                delivery: None,
                kitchen_slip: None,
            });
        }
        Err(e) => {
            return Ok(CheckoutResult {
                success: false,
                ticket: None,
                error: Some(format!("FDM error: {e}")),
                fdm_connected: true,
                fiscal_event_id: None,
                delivery: None,
                kitchen_slip: None,
            });
        }
    };

    // 6. Build the fiscal ticket (display, print, email) before the
    // commit: its email is queued in the same transaction.
    let operator = super::operator_name(state, &order.user_insz);
    let mut ticket = sign_result.as_ref().map(|result| {
        let ticket_lines: Vec<TicketLine> = lines
            .iter()
            .map(|l| TicketLine {
                quantity: l.quantity as f64,
                description: l.name.clone(),
                unit_price: l.unit_price_cents as f64 / 100.0,
                total: (l.unit_price_cents * l.quantity as i64) as f64 / 100.0,
                vat_code: l.vat_code.as_str().to_string(),
            })
            .collect();

        // One row per payment, the sale amount it pays; the cash
        // rounding is its own line (FAQ Q403, SPF example VAT ticket).
        let payments = ticket_payments(&settlement);
        let rounding_eur =
            (settlement.rounding_cents != 0).then(|| settlement.rounding_cents as f64 / 100.0);

        FiscalTicket::from_sign_result(TicketParams {
            result,
            restaurant_name: &config.restaurant_name,
            restaurant_address: &config.restaurant_address,
            restaurant_vat: &config.restaurant_vat,
            restaurant_est_no: &config.restaurant_est_no,
            restaurant_legal_name: config.legal_name(),
            lines: ticket_lines,
            payments,
            total: order.total_cents as f64 / 100.0,
            is_training: config.training_mode,
            rounding: rounding_eur,
            lang: pos_fiscal::ticket::ReceiptLanguage::from_code(&config.language),
            operator: &operator,
        })
    });
    if let Some(t) = ticket.as_mut() {
        t.unpaid = unpaid;
    }

    // 6-8. COMMIT counter + archive fiscal event + close order
    // ATOMICALLY in a single SQLite transaction.
    //
    // Now that the FDM has returned (or returned a non-fatal error
    // we've decided to record), we commit the counter increment via
    // `FiscalCounter::next` inside the same tx that archives the
    // event. Three rows of state are kept consistent:
    //   - `fiscal_counters.current_value` advances by 1
    //   - `fiscal_events` gets the new row
    //   - `orders.status` flips to closed
    // If any of the three fails the whole tx rolls back, including
    // the counter advance - so no gap is ever observable.
    let (event_id, delivery_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;

        let label = if config.training_mode {
            EventLabel::T
        } else {
            EventLabel::N
        };
        let committed_no = FiscalCounter::next(&tx, label).map_err(|e| e.to_string())?;
        // Graceful error, not a panic: see the identical comment in
        // `commands/mod.rs::commit_signed_event` - a panic here would
        // unwind while the `state.db` MutexGuard is still held and
        // poison it for the whole process. Concurrent checkouts of the
        // same order are exactly the scenario this guards against; the
        // transaction rolls back, so the loser sees a clean error and
        // can retry.
        if committed_no != ticket_number {
            return Err(errors::concurrent(format_args!(
                "checkout {label:?}: peeked {ticket_number}, committed {committed_no}"
            )));
        }

        let event_id = archive::archive_event(
            &tx,
            ArchiveParams {
                event_type: label,
                sequence_number: ticket_number,
                order_id: Some(order_id),
                pos_id: &config.pos_id,
                terminal_id: &config.terminal_id,
                user_insz: &order.user_insz,
                request_json: &request_json,
                sign_result: sign_result.as_ref(),
            },
        )
        .map_err(|e| e.to_string())?;
        pos_db::queries::orders::mark_order_entries_sent(&tx, order_id, i64::MAX, event_id)
            .map_err(|e| e.to_string())?;

        tx.execute(
            "UPDATE orders SET status = 'closed', closed_at = datetime('now') WHERE id = ?1",
            rusqlite::params![order_id],
        )
        .map_err(|e| e.to_string())?;

        // Record the payment in the order's payment history. (The X/Z
        // reports total the financials of the signed events instead.)
        // Training-mode "sales" move no real money, so they are not
        // recorded here (mirrors the FDM's T-vs-N counter split).
        if label != EventLabel::T {
            for p in &settlement.payments {
                let (method, amount) = match p.method {
                    CheckoutPaymentMethod::Cash => {
                        ("cash", p.amount_cents + settlement.rounding_cents)
                    }
                    CheckoutPaymentMethod::Card => ("card", p.amount_cents),
                    CheckoutPaymentMethod::MealVoucher => ("meal_voucher", p.amount_cents),
                    // Paid to the platform or the webshop's provider.
                    CheckoutPaymentMethod::App | CheckoutPaymentMethod::Online => {
                        ("other", p.amount_cents)
                    }
                };
                tx.execute(
                    "INSERT INTO payments (order_id, method, amount_cents) VALUES (?1, ?2, ?3)",
                    rusqlite::params![order_id, method, amount],
                )
                .map_err(|e| e.to_string())?;
            }
        }

        let delivery_id = delivery::enqueue_in_tx(&tx, event_id, &resolved, ticket.as_ref())?;

        tx.commit().map_err(|e| e.to_string())?;
        (event_id, delivery_id)
    };

    // 9. Print and/or email the ticket, then open the drawer for cash.
    let delivery_outcome = delivery::deliver(
        state,
        &config,
        event_id,
        &resolved,
        delivery_id,
        ticket.as_ref(),
    )
    .await;
    if settlement.has_cash()
        && let Some(ref addr) = config.printer_address
    {
        let drawer_data = pos_print::drawer::kick_drawer_bytes();
        let _ = pos_print::transport::send_to_printer(addr, &drawer_data).await;
    }
    let kitchen_slip = if options.online {
        SlipOutcome::NothingToPrint
    } else {
        bar_slip::print_bar_slip(
            state,
            &config,
            &order.user_insz,
            order.table_number,
            &unsent,
        )
        .await
    };

    // 10. Enqueue the completed order for async push to HubRise.
    // This is a pure SQLite write - zero network, zero latency, never
    // blocked by a HubRise or internet outage. The background outbox
    // worker (started in main.rs alongside the poller) drains the
    // queue when connectivity is available, with exponential backoff
    // on transient failures. Gated behind the `hubrise` feature so the
    // certification build (no HubRise) skips this entirely.
    #[cfg(feature = "hubrise")]
    if !unpaid && !options.online {
        let payments: Vec<(&str, i64)> = settlement
            .payments
            .iter()
            .map(|p| {
                (
                    payment_line_identity(p.method.payment_type()).1,
                    p.amount_cents,
                )
            })
            .collect();
        let body = hubrise_push::build_hubrise_order_body(&order, &lines, &payments);
        let body_json = serde_json::to_string(&body).unwrap_or_default();
        let db = state.db.lock().map_err(|e| e.to_string())?;
        if let Err(e) = pos_hubrise::outbox::enqueue(&db, order_id, &body_json) {
            // Enqueue failure is non-fatal - the order is already
            // committed locally, just won't appear in tsb-dashboard.
            tracing::warn!(%e, "outbox enqueue failed (non-fatal)");
        }
    }

    Ok(CheckoutResult {
        success: true,
        ticket,
        error: None,
        fdm_connected,
        fiscal_event_id: Some(event_id),
        delivery: Some(delivery_outcome),
        kitchen_slip: Some(kitchen_slip),
    })
}

/// The payment rows of a sale's ticket: the name the printer
/// translates ("Cash", "Card", "Meal voucher") and the amount paid.
fn ticket_payments(settlement: &Settlement) -> Vec<TicketPayment> {
    settlement
        .payments
        .iter()
        .map(|p| TicketPayment {
            method: payment_line_identity(p.method.payment_type()).1.to_string(),
            amount: p.amount_cents as f64 / 100.0,
        })
        .collect()
}

fn payment_error(e: PaymentError) -> &'static str {
    match e {
        PaymentError::Invalid => errors::PAYMENT_INVALID,
        PaymentError::ExceedsTotal => errors::PAYMENT_EXCEEDS_TOTAL,
        PaymentError::Incomplete => errors::PAYMENT_INCOMPLETE,
    }
}

/// Check FDM connection status.
#[tauri::command]
pub async fn check_fdm_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    do_check_fdm_status(state.inner()).await
}

/// How long a status answer is reused: two screens polling at once, or
/// a screen mounting again, cost the FDM one query, not several.
const STATUS_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(20);

/// The last status answer, keyed by everything that reaches the FDM:
/// its URL, the token and TLS files, the message language. A changed
/// setting queries the FDM again at once.
static STATUS_CACHE: std::sync::Mutex<Option<(String, std::time::Instant, serde_json::Value)>> =
    std::sync::Mutex::new(None);

pub async fn do_check_fdm_status(state: &AppState) -> Result<serde_json::Value, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let key = format!(
        "{}|{:?}|{:?}|{:?}|{:?}|{}",
        config.fdm_url,
        config.fdm_token_file,
        config.fdm_ca_file,
        config.fdm_client_cert_file,
        config.fdm_client_key_file,
        config.language
    );
    if let Ok(cache) = STATUS_CACHE.lock()
        && let Some((cached_key, at, value)) = cache.as_ref()
        && *cached_key == key
        && at.elapsed() < STATUS_CACHE_TTL
    {
        return Ok(value.clone());
    }
    let value = query_fdm_status(&config).await?;
    if let Ok(mut cache) = STATUS_CACHE.lock() {
        *cache = Some((key, std::time::Instant::now(), value.clone()));
    }
    Ok(value)
}

async fn query_fdm_status(
    config: &pos_core::config::PosConfig,
) -> Result<serde_json::Value, String> {
    let fdm = FdmClient::from_config(config);

    // FDM messages come back localized to the operator's UI language
    // (Q100/Q110). The FDM only speaks FR/NL/DE/EN, so unsupported UI
    // languages (zh) fall back to FR - the fiscal reference language.
    let lang = match config.language.as_str() {
        "nl" => pos_fiscal::types::Language::Nl,
        "de" => pos_fiscal::types::Language::De,
        "en" => pos_fiscal::types::Language::En,
        _ => pos_fiscal::types::Language::Fr,
    };

    match fdm.status_with_language(lang).await {
        Ok(status) => serde_json::to_value(status).map_err(|e| e.to_string()),
        Err(e) => Ok(serde_json::json!({
            "connected": false,
            "error": e.to_string()
        })),
    }
}
