use serde::Serialize;

use pos_fiscal::event::build_sale_input_split;
use pos_fiscal::fdm_client::{SignMutation, SignRequest};
use pos_fiscal::payment::{PaymentError, Settlement, payment_line_identity};
use pos_fiscal::ticket::{FiscalTicket, TicketDraft, TicketPayment};
use pos_fiscal::types::EventLabel;

use crate::commands::access::{self, Access};
use crate::commands::bar_slip::SlipOutcome;
use crate::commands::delivery::{self, DeliveryOutcome, TicketDeliveryRequest};
use crate::commands::errors;
#[cfg(feature = "hubrise")]
use crate::commands::hubrise_push;
use crate::commands::signing::{
    self, ArchiveSpec, BarSlip, Begin, Kind, Op, PendingContext, Prepared, SignGuard,
    StoredDelivery, StoredPayment,
};
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

/// Checkout with a single payment method; the `checkout` IPC command
/// takes several payments and calls [`do_checkout_split`].
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
    /// The closing of an online order cancelled after its P: every line
    /// was corrected, so the sale comes to 0,00 EUR with its journal
    /// (additions and corrections) as its only lines.
    #[cfg(feature = "hubrise")]
    pub closes_cancelled: bool,
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
    let op = Op::new(Kind::Sale, order_id);
    let guard = match signing::begin(state, &op).await {
        Ok(Begin::Fresh(guard)) => guard,
        // This sale, sent earlier, was signed: its ticket is printed.
        Ok(Begin::AlreadyDone(recovered)) => return Ok(recovered_sale(*recovered)),
        Err(e) => return Ok(CheckoutResult::refused(e)),
    };
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let signed = sell_locked(
        state,
        &guard,
        &config,
        operator,
        order_id,
        payment,
        delivery_request,
        &options,
        op,
    )
    .await?;
    drop(guard);
    let (context, committed) = match signed {
        Ok(signed) => signed,
        Err(refused) => return Ok(refused),
    };

    // Print and/or email the ticket, open the drawer for cash, print
    // the bar / kitchen slip of the lines not yet sent.
    let done = signing::follow_up(state, &config, &context, &committed, true).await;
    Ok(CheckoutResult {
        success: true,
        ticket: committed.ticket,
        error: None,
        fdm_connected: true,
        fiscal_event_id: Some(committed.event_ids[0]),
        delivery: done.delivery,
        kitchen_slip: done.slip,
    })
}

/// A sale signed on a later send of its request.
fn recovered_sale(recovered: signing::Recovered) -> CheckoutResult {
    CheckoutResult {
        success: true,
        ticket: recovered.committed.ticket,
        error: None,
        fdm_connected: true,
        fiscal_event_id: Some(recovered.committed.event_ids[0]),
        delivery: recovered.follow_up.delivery,
        kitchen_slip: recovered.follow_up.slip,
    }
}

type SaleSigned = Result<(PendingContext, signing::Committed), CheckoutResult>;

/// Read, check and sign the sale, under the signing lock. `Err` inside
/// `Ok`: refused, nothing signed.
#[allow(clippy::too_many_arguments)]
async fn sell_locked(
    state: &AppState,
    guard: &SignGuard<'_>,
    config: &pos_core::config::PosConfig,
    cashier: pos_core::user::User,
    order_id: i64,
    payment: Payment<'_>,
    delivery_request: Option<&TicketDeliveryRequest>,
    options: &SaleOptions,
    op: Op,
) -> Result<SaleSigned, String> {
    // `lines` is the consolidated order (printed); `entries` the journal
    // of every addition and correction, sent to the FDM (§2).
    let (order, lines, entries, booking_period_id) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let order = pos_db::queries::orders::get_order(&db, order_id).map_err(|e| e.to_string())?;
        let lines =
            pos_db::queries::orders::get_order_lines(&db, order_id).map_err(|e| e.to_string())?;
        let entries = pos_db::queries::orders::get_order_entries(&db, order_id, false)
            .map_err(|e| e.to_string())?;
        let bp = super::booking(&db, guard, config)?;
        (order, lines, entries, bp)
    };
    // The lines' discounts (an online order's), by journal entry and by
    // line. Only the HubRise build records any.
    #[cfg(feature = "hubrise")]
    let (entry_changes, line_changes) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        super::price_changes::load(&db, order_id)?
    };
    #[cfg(not(feature = "hubrise"))]
    let (entry_changes, line_changes) = (
        pos_fiscal::price_change::LineChanges::new(),
        pos_fiscal::price_change::LineChanges::new(),
    );
    let mut order = order;
    order.user_insz = cashier.insz;

    #[cfg(feature = "hubrise")]
    let nothing_to_sign = if options.closes_cancelled {
        entries.is_empty() || order.total_cents != 0
    } else {
        lines.is_empty()
    };
    #[cfg(not(feature = "hubrise"))]
    let nothing_to_sign = lines.is_empty();
    if nothing_to_sign {
        return Ok(Err(CheckoutResult::refused(errors::ORDER_NO_LINES)));
    }

    // Guard: no negative sale (a refund goes through `refund.rs`). A
    // sale that comes to 0,00 EUR is signed like any other: event N
    // and a VAT ticket (producers FAQ Q206, Q209). Together with the
    // empty-order check above, no record without a product line can
    // be made (manual §III.D).
    if order.total_cents < 0 {
        return Ok(Err(CheckoutResult::refused(errors::ORDER_TOTAL_NEGATIVE)));
    }

    // Guard: an order is sold once. Read under the signing lock, so a
    // second tap waits for the first sale and then finds it closed.
    if order.status != pos_core::order::OrderStatus::Open {
        return Ok(Err(CheckoutResult::refused(errors::ORDER_ALREADY_CLOSED)));
    }

    // An online order is signed by its own commands (acceptance,
    // handover), with its cost center: never as a till sale.
    #[cfg(feature = "hubrise")]
    let webshop_order_id = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let online: bool = db
            .query_row(
                "SELECT hubrise_order_id IS NOT NULL FROM orders WHERE id = ?1",
                rusqlite::params![order_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if online && !options.online {
            return Ok(Err(CheckoutResult::refused(errors::ORDER_NOT_OPEN)));
        }
        online.then_some(order_id)
    };
    #[cfg(not(feature = "hubrise"))]
    let webshop_order_id = None;

    // Ticket medium: refused choices stop here, before any signing.
    let resolved = match delivery::resolve(delivery_request, config, &state.mail) {
        Ok(r) => r,
        Err(e) => return Ok(Err(CheckoutResult::refused(e))),
    };

    // The payment: card and meal voucher parts are exact, cash pays the
    // rest, rounded to 5 cents (a sale at 0,00 EUR is a cash payment of
    // 0, FAQ Q206). Refused before anything is signed.
    let settlement = match payment {
        Payment::Parts(parts) => match pos_fiscal::payment::settle(order.total_cents, parts) {
            Ok(s) => s,
            Err(e) => return Ok(Err(CheckoutResult::refused(payment_error(e)))),
        },
        // Nothing paid: no payment line at all (UC4109 sends
        // `financials: []`). A sale of 0,00 EUR is paid normally.
        Payment::Unpaid if order.total_cents > 0 => Settlement {
            payments: Vec::new(),
            rounding_cents: 0,
        },
        Payment::Unpaid => return Ok(Err(CheckoutResult::refused(errors::UNPAID_NOTHING_DUE))),
    };
    let unpaid = matches!(payment, Payment::Unpaid);

    // The number this sale takes; the counter advances only when the
    // signed sale is archived, so a refused sale leaves no gap.
    let label = if config.training_mode {
        EventLabel::T
    } else {
        EventLabel::N
    };
    let ticket_number = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        signing::peek(&db, guard, label)?
    };

    let mut sale_input = build_sale_input_split(
        config,
        &order,
        &entries,
        ticket_number,
        &booking_period_id,
        &settlement,
    );
    pos_fiscal::price_change::apply(&mut sale_input.transaction, &entries, &entry_changes);
    sale_input.common.ticket_medium = resolved.medium;
    if let Some(cost_center) = options.cost_center.clone() {
        sale_input.cost_center = Some(cost_center);
    }
    let request = SignRequest::new(SignMutation::Sale, &sale_input, config.training_mode)
        .map_err(errors::internal)?;
    let request_json = serde_json::to_string(&sale_input).map_err(|e| e.to_string())?;

    // The ticket: each line, then its discounts (none at the till); one
    // row per payment, the cash rounding its own line (FAQ Q403).
    let operator = super::operator_name(state, &order.user_insz);
    let mut ticket = TicketDraft::new(
        config,
        pos_fiscal::price_change::ticket_lines(&lines, &line_changes),
        ticket_payments(&settlement),
        order.total_cents as f64 / 100.0,
        &operator,
    );
    ticket.rounding =
        (settlement.rounding_cents != 0).then(|| settlement.rounding_cents as f64 / 100.0);
    ticket.unpaid = unpaid;

    // The order's payment history (the reports total the signed events'
    // financials instead).
    let payments = settlement
        .payments
        .iter()
        .map(|p| {
            let (method, amount_cents) = match p.method {
                CheckoutPaymentMethod::Cash => ("cash", p.amount_cents + settlement.rounding_cents),
                CheckoutPaymentMethod::Card => ("card", p.amount_cents),
                CheckoutPaymentMethod::MealVoucher => ("meal_voucher", p.amount_cents),
                // Paid to the platform or the webshop's provider.
                CheckoutPaymentMethod::App | CheckoutPaymentMethod::Online => {
                    ("other", p.amount_cents)
                }
            };
            StoredPayment {
                method: method.to_string(),
                amount_cents,
            }
        })
        .collect();

    // A till sale is echoed to HubRise through the outbox, queued with
    // the sale.
    #[cfg(feature = "hubrise")]
    let hubrise_echo = (!unpaid && !options.online && state.hubrise_switch.is_on()).then(|| {
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
        serde_json::to_string(&body).unwrap_or_default()
    });
    #[cfg(not(feature = "hubrise"))]
    let hubrise_echo = None;

    let context = PendingContext::Sale {
        order_id,
        training: label == EventLabel::T,
        payments,
        cash: settlement.has_cash(),
        delivery: StoredDelivery::of(&resolved),
        ticket,
        // An online order's kitchen ticket is printed by its caller.
        bar_slip: (!options.online).then(|| BarSlip {
            operator_insz: order.user_insz.clone(),
            table_number: order.table_number,
        }),
        webshop_order_id,
        hubrise_echo,
    };
    let prepared = Prepared {
        op,
        request,
        label,
        first_ticket_no: ticket_number,
        events: vec![ArchiveSpec {
            order_id: Some(order_id),
            user_insz: order.user_insz.clone(),
            request_json,
            sent_entries: Some((order_id, i64::MAX)),
        }],
        order_id: Some(order_id),
        user_insz: order.user_insz.clone(),
        context: context.clone(),
    };
    match signing::sign(state, guard, config, prepared).await {
        Ok(committed) => Ok(Ok((context, committed))),
        // No FDM, no sale: the POS blocks (SCE 2.0). There is no
        // offline mode, in any build.
        Err(failure) if failure.not_connected => Ok(Err(CheckoutResult {
            success: false,
            ticket: None,
            error: Some("FDM not connected - cannot complete transaction".into()),
            fdm_connected: false,
            fiscal_event_id: None,
            delivery: None,
            kitchen_slip: None,
        })),
        Err(failure) => Ok(Err(CheckoutResult {
            success: false,
            ticket: None,
            error: Some(failure.error),
            fdm_connected: true,
            fiscal_event_id: None,
            delivery: None,
            kitchen_slip: None,
        })),
    }
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

/// How long a status answer is reused: two screens polling at once, or
/// a screen mounting again, cost the FDM one query, not several.
const STATUS_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(20);

/// Check FDM connection status.
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
    if let Ok(cache) = state.fdm_status_cache.lock()
        && let Some((cached_key, at, value)) = cache.as_ref()
        && *cached_key == key
        && at.elapsed() < STATUS_CACHE_TTL
    {
        return Ok(value.clone());
    }
    let value = query_fdm_status(state, &config).await?;
    if let Ok(mut cache) = state.fdm_status_cache.lock() {
        *cache = Some((key, std::time::Instant::now(), value.clone()));
    }
    Ok(value)
}

async fn query_fdm_status(
    state: &AppState,
    config: &pos_core::config::PosConfig,
) -> Result<serde_json::Value, String> {
    let fdm = state.fdm_client(config);

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
