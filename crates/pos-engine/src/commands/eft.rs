//! The card payment terminal linked to the till (EFT).
//!
//! With a `payment_terminal` section in `pos.json` the operator no
//! longer types a card on a standalone terminal: the till asks the
//! terminal for an amount ([`do_eft_pay`]) and the sale later settles
//! the approved payment (`checkout::sell`, [`PaymentPart::eft_payment_id`]).
//!
//! Rules:
//! - Every payment is stored (`eft_payments`, status `sent`) BEFORE the
//!   terminal is called, so a crash between the call and the answer
//!   leaves a row saying a card may have been charged. At start-up a
//!   `sent` row becomes `unknown` ([`recover_after_crash`]).
//! - Nothing is charged for a sale that cannot be signed: the FDM must
//!   be operational when the payment is asked (AM art. 14).
//! - A payment waiting for its answer (`sent`, `unknown`) blocks every
//!   other payment and every sale of its order until [`do_eft_check`]
//!   gets the terminal's answer. An approved payment must be used by
//!   the sale of its order, for its exact amount: it is never forgotten.
//! - The terminal gives the card type and brand; the signed payment
//!   line is `AUTOMATIC` (see `pos_fiscal::payment`).
//! - Refunds stay cash; reversing an approved card payment is not
//!   supported yet.
//!
//! The terminal is built per call from the configuration. Tests put a
//! scripted one in `AppState::eft_terminal_override` (feature
//! `eft-scripted`, never in a shipped build).

use std::collections::HashSet;
use std::sync::Arc;

use pos_core::config::PaymentTerminalConfig;
use pos_core::order::OrderStatus;
use pos_db::queries::eft::{self, CardKind, EftPaymentRow, NewPayment, Resolution};
use pos_eft::{CardType, EftError, Outcome, PaymentRequest, Terminal};
use pos_fiscal::payment::{PaymentMethod, PaymentPart, Settlement, TerminalCard};
use pos_fiscal::types::PaymentType;
use rusqlite::Connection;
use serde::Serialize;

use crate::commands::access::{self, Access};
use crate::commands::checkout;
use crate::commands::errors;
use crate::state::AppState;

/// What the FDM check refuses a payment with: it starts with "FDM", so
/// the screen shows it as an FDM message (the checkout's own wording).
const FDM_NOT_CONNECTED: &str = "FDM not connected - cannot complete transaction";

/// A payment asked of the terminal, as the checkout screen shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EftPayment {
    pub id: i64,
    pub order_id: i64,
    pub amount_cents: i64,
    /// `sent`, `approved`, `declined`, `cancelled`, `unknown` or
    /// `not_sent`.
    pub status: String,
    /// `debit`, `credit` or `unknown`; set once approved.
    pub card_type: Option<String>,
    pub brand: Option<String>,
    pub terminal_reference: Option<String>,
    pub authorisation_code: Option<String>,
    pub masked_pan: Option<String>,
    /// The approved card is a meal-voucher card (Edenred, Pluxee,
    /// Monizze, Payflip): the sale signs it as `CHEQUE_MEAL`.
    pub meal_voucher: bool,
    /// Why it was declined, what is missing for an unknown one.
    pub detail: Option<String>,
    pub created_at: String,
    /// A sale has settled it.
    pub used: bool,
}

impl EftPayment {
    fn of(row: EftPaymentRow) -> Self {
        Self {
            id: row.id,
            order_id: row.order_id,
            amount_cents: row.amount_cents,
            status: row.status.as_str().to_string(),
            card_type: row.card_type.map(|c| c.as_str().to_string()),
            meal_voucher: pos_eft::brand::is_meal_voucher(row.brand.as_deref()),
            brand: row.brand,
            terminal_reference: row.terminal_reference,
            authorisation_code: row.authorisation_code,
            masked_pan: row.masked_pan,
            detail: row.detail,
            created_at: row.created_at,
            used: row.used_event_id.is_some(),
        }
    }
}

/// Start-up repair: a payment still `sent` lost its answer with the
/// crash. It becomes `unknown`, to be checked with the terminal.
pub fn recover_after_crash(conn: &Connection) -> rusqlite::Result<usize> {
    eft::sent_to_unknown(conn)
}

// ── Terminal ───────────────────────────────────────────────────

/// The `payment_terminal` section, or `eft_not_configured`.
fn terminal_config(state: &AppState) -> Result<PaymentTerminalConfig, String> {
    state
        .config
        .lock()
        .map_err(errors::internal)?
        .payment_terminal
        .clone()
        .ok_or_else(|| errors::EFT_NOT_CONFIGURED.to_string())
}

/// A training sale never charges a real card (its card parts are typed
/// by hand, as without a terminal).
fn refuse_in_training(state: &AppState) -> Result<(), String> {
    if state.config.lock().map_err(errors::internal)?.training_mode {
        return Err(errors::EFT_TRAINING.to_string());
    }
    Ok(())
}

/// The error a failed terminal call shows. Nothing was charged.
fn eft_error(e: &EftError) -> String {
    match e {
        EftError::Busy => errors::EFT_BUSY.to_string(),
        EftError::Unreachable(detail) => {
            errors::coded(errors::EFT_UNREACHABLE, &[("detail", detail)])
        }
        EftError::Rejected(detail) | EftError::Config(detail) => {
            errors::coded(errors::EFT_REJECTED, &[("detail", detail)])
        }
    }
}

fn build_terminal(state: &AppState, config: &PaymentTerminalConfig) -> Result<Terminal, EftError> {
    #[cfg(feature = "eft-scripted")]
    if let Some(script) = &state.eft_terminal_override {
        return Ok(Terminal::scripted(Arc::clone(script)));
    }
    // Trusts what the rest of the engine trusts (`HttpTls`).
    let http = state
        .http_tls
        .client_builder()
        .and_then(|b| b.build().map_err(Into::into))
        .map_err(|e| EftError::Config(format!("HTTPS client: {e}")))?;
    Terminal::new(config, http, state.identity.as_str())
}

/// The language of the terminal's screens: the till's, within what
/// terminals speak (French, the fiscal reference, otherwise).
fn terminal_language(language: &str) -> &'static str {
    match language {
        "nl" => "nl",
        "de" => "de",
        "en" => "en",
        _ => "fr",
    }
}

fn card_kind(card_type: CardType) -> CardKind {
    match card_type {
        CardType::Debit => CardKind::Debit,
        CardType::Credit => CardKind::Credit,
        CardType::Unknown => CardKind::Unknown,
    }
}

/// What to store for the terminal's answer to a payment of
/// `amount_cents`.
fn resolution_of(outcome: Outcome, amount_cents: i64) -> Resolution {
    match outcome {
        // The drivers never ask for a partial approval: an approval for
        // another amount is for a person to sort out, not to sign.
        Outcome::Approved(a) if a.amount_cents != amount_cents => {
            let detail = format!(
                "approved amount {} differs from {}, check the terminal",
                pos_eft::amount_text(a.amount_cents),
                pos_eft::amount_text(amount_cents)
            );
            tracing::error!(%detail, "card payment approved for another amount: kept unknown");
            Resolution::Unknown { detail }
        }
        Outcome::Approved(a) => Resolution::Approved {
            card_type: card_kind(a.card_type),
            // Normalized once more here (the drivers do it already): the
            // meal-voucher classification is fixed by what is stored.
            brand: a.brand.as_deref().and_then(pos_eft::brand::normalize),
            terminal_reference: a.terminal_reference,
            authorisation_code: a.authorisation_code,
            masked_pan: a.masked_pan,
            customer_receipt: a.customer_receipt,
        },
        Outcome::Declined { reason } => Resolution::Declined { reason },
        Outcome::Cancelled => Resolution::Cancelled,
        Outcome::Unknown { detail } => Resolution::Unknown { detail },
    }
}

// ── Commands ───────────────────────────────────────────────────

/// Why a payment of `amount_cents` cannot be asked for `order_id`, or
/// the order's table. Checked under the database lock, so two taps
/// cannot both pass.
fn check_request(db: &Connection, order_id: i64, amount_cents: i64) -> Result<Option<i32>, String> {
    let order = match pos_db::queries::orders::get_order(db, order_id) {
        Ok(order) => order,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Err(errors::ORDER_NOT_OPEN.into()),
        Err(e) => return Err(errors::internal(e)),
    };
    if order.status != OrderStatus::Open {
        return Err(errors::ORDER_NOT_OPEN.into());
    }
    // What the order's other approved payments leave to pay.
    let approved: i64 = eft::usable_for_order(db, order_id)
        .map_err(errors::internal)?
        .iter()
        .map(|p| p.amount_cents)
        .sum();
    let due = order.total_cents.saturating_sub(approved);
    if amount_cents <= 0 || amount_cents > due {
        return Err(errors::EFT_AMOUNT_INVALID.into());
    }
    if !eft::open(db).map_err(errors::internal)?.is_empty() {
        return Err(errors::EFT_PENDING.into());
    }
    Ok(order.table_number)
}

/// Ask the linked terminal to charge `amount_cents` for the order, and
/// store its answer. `Ok` with the payment whatever the terminal said
/// (approved, declined, cancelled or still unknown); `Err` only when
/// nothing was charged (the payment is then kept as `not_sent`) or the
/// request was refused before anything was stored.
///
/// The FDM must be operational now, and no other terminal payment may
/// wait for its answer. Same rights as a checkout.
pub async fn do_eft_pay(
    state: &AppState,
    order_id: i64,
    amount_cents: i64,
) -> Result<EftPayment, String> {
    let operator = access::operator(state, Access::Cashier)?;
    let terminal_config = terminal_config(state)?;
    refuse_in_training(state)?;
    // Fail fast, before the FDM is asked.
    {
        let db = state.db.lock().map_err(errors::internal)?;
        check_request(&db, order_id, amount_cents)?;
    }
    // Never charge a card for a sale that cannot be signed (AM art. 14).
    // An FDM answer still awaited blocks every signature as well.
    let operational = checkout::fdm_operational_now(state)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(%e, "FDM status check failed before a card payment");
            false
        });
    if !operational {
        return Err(FDM_NOT_CONNECTED.to_string());
    }
    refuse_while_fdm_answer_awaited(state)?;
    let (restaurant, language) = {
        let config = state.config.lock().map_err(errors::internal)?;
        (
            config.restaurant_name.clone(),
            terminal_language(&config.language),
        )
    };

    // Stored before the terminal is called.
    let reference = uuid::Uuid::new_v4().to_string();
    let (id, table) = {
        let db = state.db.lock().map_err(errors::internal)?;
        let tx = db.unchecked_transaction().map_err(errors::internal)?;
        let table = check_request(&tx, order_id, amount_cents)?;
        let id = eft::insert_sent(
            &tx,
            &NewPayment {
                reference: &reference,
                order_id,
                terminal_kind: terminal_config.kind.as_str(),
                amount_cents,
                user_insz: &operator.insz,
            },
        )
        .map_err(errors::internal)?;
        tx.commit().map_err(errors::internal)?;
        (id, table)
    };
    tracing::info!(id, order_id, amount_cents, %reference, "card payment asked of the terminal");

    let request = PaymentRequest {
        reference: reference.clone(),
        amount_cents,
        description: match table {
            Some(table) => format!("{restaurant}, table {table}"),
            None => restaurant,
        },
        language: language.to_string(),
    };
    let result = match build_terminal(state, &terminal_config) {
        Ok(terminal) => {
            // The terminal serves one connection: no probe meanwhile.
            let _line = state.eft_line.lock().await;
            // An abort reaches this very instance while it waits.
            let terminal = Arc::new(terminal);
            let _in_flight = InFlight::set(state, &reference, Arc::clone(&terminal));
            terminal.pay(&request).await
        }
        Err(e) => Err(e),
    };
    let (resolution, failure) = match result {
        Ok(outcome) => (resolution_of(outcome, amount_cents), None),
        Err(e) => (
            Resolution::NotSent {
                detail: e.to_string(),
            },
            Some(eft_error(&e)),
        ),
    };
    let row = store(state, id, &resolution).inspect_err(|error| {
        // The terminal's answer is lost with this error: leave a trace to
        // reconcile it by hand (an approved card was charged). The PAN is
        // already masked; the receipt text is left out of the log.
        match &resolution {
            Resolution::Approved {
                brand,
                terminal_reference,
                authorisation_code,
                masked_pan,
                ..
            } => tracing::error!(
                id, order_id, amount_cents, %reference, ?brand, ?terminal_reference,
                ?authorisation_code, ?masked_pan, %error,
                "approved card payment could not be stored"
            ),
            // The fields are listed one by one: a receipt field added to
            // `Resolution` later must not reach the logs by a `Debug`.
            Resolution::Declined { reason } => tracing::error!(
                id, order_id, amount_cents, %reference, outcome = "declined", ?reason, %error,
                "terminal answer could not be stored"
            ),
            Resolution::Cancelled => tracing::error!(
                id, order_id, amount_cents, %reference, outcome = "cancelled", %error,
                "terminal answer could not be stored"
            ),
            Resolution::Unknown { detail } => tracing::error!(
                id, order_id, amount_cents, %reference, outcome = "unknown", ?detail, %error,
                "terminal answer could not be stored"
            ),
            Resolution::NotSent { detail } => tracing::error!(
                id, order_id, amount_cents, %reference, outcome = "not_sent", ?detail, %error,
                "terminal answer could not be stored"
            ),
        }
    })?;
    tracing::info!(id, status = row.status.as_str(), "card payment answered");
    match failure {
        Some(error) => Err(error),
        None => Ok(EftPayment::of(row)),
    }
}

/// No sale can be signed while a sign request waits for the FDM's
/// answer: no card is charged meanwhile.
fn refuse_while_fdm_answer_awaited(state: &AppState) -> Result<(), String> {
    let awaiting = {
        let db = state.db.lock().map_err(errors::internal)?;
        pos_db::queries::pending_signs::open(&db).map_err(errors::internal)?
    };
    match awaiting {
        Some(row) => Err(crate::commands::signing::answer_pending(
            &row.event_label,
            row.first_ticket_no,
            None,
            &"the FDM has not answered yet",
        )),
        None => Ok(()),
    }
}

/// Store `resolution` for payment `id` and read the payment back (a
/// concurrent check may have settled it first: the stored answer wins).
fn store(state: &AppState, id: i64, resolution: &Resolution) -> Result<EftPaymentRow, String> {
    let db = state.db.lock().map_err(errors::internal)?;
    eft::resolve(&db, id, resolution).map_err(errors::internal)?;
    eft::get(&db, id)
        .map_err(errors::internal)?
        .ok_or_else(|| errors::internal("eft payment vanished"))
}

/// Ask the terminal to stop the payment in progress, if there is one.
/// A request, not a guarantee: the payment's own [`do_eft_pay`] returns
/// the outcome (cancelled, or approved when the abort came too late).
pub async fn do_eft_abort(state: &AppState) -> Result<(), String> {
    access::operator(state, Access::Cashier)?;
    let terminal_config = terminal_config(state)?;
    let in_flight = {
        let db = state.db.lock().map_err(errors::internal)?;
        eft::in_flight(&db).map_err(errors::internal)?
    };
    let Some(row) = in_flight else {
        return Ok(());
    };
    // The instance running the payment, else (after a restart) a new one.
    let running = state
        .eft_in_flight
        .lock()
        .map_err(errors::internal)?
        .as_ref()
        .filter(|(reference, _)| *reference == row.reference)
        .map(|(_, terminal)| Arc::clone(terminal));
    let terminal = match running {
        Some(terminal) => terminal,
        None => Arc::new(build_terminal(state, &terminal_config).map_err(|e| eft_error(&e))?),
    };
    terminal
        .abort(&row.reference)
        .await
        .map_err(|e| eft_error(&e))
}

/// The terminal running a payment, known to [`do_eft_abort`] until the
/// payment's call returns (or is dropped).
struct InFlight<'a>(&'a AppState);

impl<'a> InFlight<'a> {
    fn set(state: &'a AppState, reference: &str, terminal: Arc<Terminal>) -> Self {
        if let Ok(mut slot) = state.eft_in_flight.lock() {
            *slot = Some((reference.to_string(), terminal));
        }
        Self(state)
    }
}

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        if let Ok(mut slot) = self.0.eft_in_flight.lock() {
            *slot = None;
        }
    }
}

/// Ask the terminal where the payment `id` stands (`sent` or
/// `unknown`) and store the answer; a payment already answered is
/// returned as it is. Any logged-in user.
pub async fn do_eft_check(state: &AppState, id: i64) -> Result<EftPayment, String> {
    access::operator(state, Access::Staff)?;
    // Behind a payment in progress, whose answer is then the stored one.
    let _line = state.eft_line.lock().await;
    let row = {
        let db = state.db.lock().map_err(errors::internal)?;
        eft::get(&db, id).map_err(errors::internal)?
    }
    .ok_or_else(|| errors::EFT_PAYMENT_NOT_USABLE.to_string())?;
    if !row.status.is_open() {
        return Ok(EftPayment::of(row));
    }
    let terminal_config = terminal_config(state)?;
    let terminal = build_terminal(state, &terminal_config).map_err(|e| eft_error(&e))?;
    let outcome = terminal
        .status(&row.reference)
        .await
        .map_err(|e| eft_error(&e))?;
    let row = store(state, id, &resolution_of(outcome, row.amount_cents))?;
    tracing::info!(id, status = row.status.as_str(), "card payment checked");
    Ok(EftPayment::of(row))
}

/// Whether the linked terminal is configured and answers, as the
/// header light and the start-up checks show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaymentTerminalStatus {
    pub configured: bool,
    /// The driver (`PaymentTerminalKind::as_str`); `None` when not
    /// configured.
    pub kind: Option<String>,
    /// The terminal answered (or a payment is running on it).
    pub connected: bool,
    /// A payment or a status question holds the terminal's line: it was
    /// not probed.
    pub busy: bool,
    /// Why it is not connected.
    pub detail: Option<String>,
}

/// Ask the linked terminal whether it is reachable. Charges nothing and
/// never disturbs a payment: while one holds the terminal's line
/// (`AppState::eft_line`) the answer is "busy, connected" without a
/// probe. A training till probes too. No login needed: the start-up
/// checks ask before anyone is in, like the printers'.
pub async fn do_check_payment_terminal(state: &AppState) -> Result<PaymentTerminalStatus, String> {
    let configured = state
        .config
        .lock()
        .map_err(errors::internal)?
        .payment_terminal
        .clone();
    let Some(config) = configured else {
        return Ok(PaymentTerminalStatus {
            configured: false,
            kind: None,
            connected: false,
            busy: false,
            detail: None,
        });
    };
    let kind = Some(config.kind.as_str().to_string());
    let Ok(_line) = state.eft_line.try_lock() else {
        return Ok(PaymentTerminalStatus {
            configured: true,
            kind,
            connected: true,
            busy: true,
            detail: None,
        });
    };
    let result = match build_terminal(state, &config) {
        Ok(terminal) => terminal.check().await,
        Err(e) => Err(e),
    };
    // A terminal busy with a payment of another till answers: it is there.
    let busy = matches!(result, Err(EftError::Busy));
    let detail = match result {
        Err(EftError::Unreachable(d) | EftError::Rejected(d) | EftError::Config(d)) => Some(d),
        Ok(()) | Err(EftError::Busy) => None,
    };
    Ok(PaymentTerminalStatus {
        configured: true,
        kind,
        connected: detail.is_none(),
        busy,
        detail,
    })
}

/// The order's payments the checkout screen shows: those approved and
/// not yet settled, and those still waiting for an answer (after a
/// crash too), oldest first. Any logged-in user.
pub fn do_list_eft_payments(state: &AppState, order_id: i64) -> Result<Vec<EftPayment>, String> {
    access::operator(state, Access::Staff)?;
    let db = state.db.lock().map_err(errors::internal)?;
    Ok(eft::pending_for_order(&db, order_id)
        .map_err(errors::internal)?
        .into_iter()
        .map(EftPayment::of)
        .collect())
}

/// A card payment approved by the terminal that no sale has settled
/// yet: money taken that no ticket records. It is settled later by the
/// sale of its order, which can come after a Z.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnsettledCardPayment {
    pub payment_id: i64,
    pub order_id: i64,
    /// `dine_in` or `takeaway`.
    pub order_type: String,
    /// The table of a dine-in order.
    pub table_number: Option<i32>,
    pub amount_cents: i64,
    pub brand: Option<String>,
    pub created_at: String,
}

/// The approved card payments of every order that no sale has settled,
/// oldest first. The Z lists them to the operator (it is never blocked
/// by them); the table plan and the takeaway list mark their orders.
/// Any logged-in user.
pub fn do_list_unsettled_card_payments(
    state: &AppState,
) -> Result<Vec<UnsettledCardPayment>, String> {
    access::operator(state, Access::Staff)?;
    let db = state.db.lock().map_err(errors::internal)?;
    let mut out = Vec::new();
    for row in eft::unsettled(&db).map_err(errors::internal)? {
        let order =
            pos_db::queries::orders::get_order(&db, row.order_id).map_err(errors::internal)?;
        out.push(UnsettledCardPayment {
            payment_id: row.id,
            order_id: row.order_id,
            order_type: order.order_type.as_str().to_string(),
            table_number: order.table_number,
            amount_cents: row.amount_cents,
            brand: row.brand,
            created_at: row.created_at,
        });
    }
    Ok(out)
}

// ── The sale ───────────────────────────────────────────────────

/// Check the card parts a sale of `order_id` takes against the
/// terminal payments. Returns, for each payment the sale settles, what
/// the terminal returned.
///
/// Refused (`eft_pending`) while a payment of the order waits for its
/// answer: the card may be charged. Refused (`eft_payment_not_usable`)
/// when a part names a payment that is not an approved, unused one of
/// this order, taken by card for that exact amount, or when an approved
/// payment of the order is left out: a charged card is never forgotten
/// (a sale closed unpaid has no parts, so it is refused too).
pub(crate) fn terminal_cards(
    db: &Connection,
    order_id: i64,
    parts: &[PaymentPart],
) -> Result<Vec<(i64, TerminalCard)>, String> {
    let open = eft::open_for_order(db, order_id).map_err(errors::internal)?;
    if !open.is_empty() {
        return Err(errors::EFT_PENDING.into());
    }
    let not_usable = || errors::EFT_PAYMENT_NOT_USABLE.to_string();
    let mut cards = Vec::new();
    let mut seen = HashSet::new();
    for part in parts {
        let Some(id) = part.eft_payment_id else {
            continue;
        };
        let row = eft::get(db, id)
            .map_err(errors::internal)?
            .ok_or_else(not_usable)?;
        if !seen.insert(id)
            || row.order_id != order_id
            || !row.is_usable()
            || part.method != PaymentMethod::Card
            || part.amount_cents != Some(row.amount_cents)
        {
            return Err(not_usable());
        }
        cards.push((
            id,
            TerminalCard {
                card_type: match row.card_type {
                    // A meal-voucher card is told by its brand; the
                    // stored card type stays `unknown` (L1F-B1).
                    _ if pos_eft::brand::is_meal_voucher(row.brand.as_deref()) => {
                        PaymentType::ChequeMeal
                    }
                    Some(CardKind::Debit) => PaymentType::CardDebit,
                    Some(CardKind::Credit) => PaymentType::CardCredit,
                    Some(CardKind::Unknown) | None => PaymentType::CardUnknown,
                },
                // The spec allows 600 characters.
                brand: row.brand.map(|b| b.chars().take(600).collect()),
                reference: row
                    .terminal_reference
                    .map(|r| r.chars().take(600).collect()),
            },
        ));
    }
    for row in eft::usable_for_order(db, order_id).map_err(errors::internal)? {
        if !seen.contains(&row.id) {
            return Err(not_usable());
        }
    }
    Ok(cards)
}

/// Before an edit that may lower the total of `order_id`: refuse while
/// a terminal payment of the order waits for its answer (its amount is
/// not final), and return what approved payments no sale has settled
/// hold of the order (0 without any). The order's total may not drop
/// under it ([`keep_charged`]), since the sale settles each payment for
/// its exact amount.
pub(crate) fn charged_floor(db: &Connection, order_id: i64) -> Result<i64, String> {
    if !eft::open_for_order(db, order_id)
        .map_err(errors::internal)?
        .is_empty()
    {
        return Err(errors::EFT_EDIT_PENDING.into());
    }
    Ok(eft::usable_for_order(db, order_id)
        .map_err(errors::internal)?
        .iter()
        .map(|p| p.amount_cents)
        .sum())
}

/// The error of a total under what the card was charged.
fn below_charged(charged_cents: i64) -> String {
    errors::coded(
        errors::EFT_BELOW_CHARGED,
        &[(
            "amount",
            &format!("{},{:02}", charged_cents / 100, charged_cents % 100),
        )],
    )
}

/// After an edit, inside its transaction: the order's total must still
/// cover `floor` ([`charged_floor`]). The caller rolls back on `Err`.
pub(crate) fn keep_charged(db: &Connection, order_id: i64, floor: i64) -> Result<(), String> {
    if floor == 0 {
        return Ok(());
    }
    let total = pos_db::queries::orders::get_order(db, order_id)
        .map_err(errors::internal)?
        .total_cents;
    if total < floor {
        return Err(below_charged(floor));
    }
    Ok(())
}

/// Before an action that drops the whole order (reject, close as
/// cancelled): refused while a payment waits for its answer or an
/// approved one is not settled, the card having been charged.
pub(crate) fn refuse_if_charged(db: &Connection, order_id: i64) -> Result<(), String> {
    match charged_floor(db, order_id)? {
        0 => Ok(()),
        charged => Err(below_charged(charged)),
    }
}

/// Give each settled payment the terminal data of its card.
pub(crate) fn attach_cards(settlement: &mut Settlement, cards: &[(i64, TerminalCard)]) {
    for payment in &mut settlement.payments {
        payment.terminal = payment
            .eft_payment_id
            .and_then(|id| cards.iter().find(|(card_id, _)| *card_id == id))
            .map(|(_, card)| card.clone());
    }
}

/// Mark the payments `ids` as settled by the sale `event_id`, in the
/// sale's archive transaction.
pub(crate) fn mark_used(tx: &Connection, ids: &[i64], event_id: i64) -> Result<(), String> {
    for &id in ids {
        match eft::mark_used(tx, id, event_id) {
            Ok(true) => {}
            // The sale is signed: it is archived either way.
            Ok(false) => tracing::error!(id, event_id, "card payment could not be marked used"),
            Err(e) => return Err(errors::internal(e)),
        }
    }
    Ok(())
}
