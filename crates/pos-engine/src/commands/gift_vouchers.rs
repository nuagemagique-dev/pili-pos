//! Multi-purpose gift vouchers (decisions V12-D24 to V12-D31).
//!
//! A voucher is sold as a line of the system product (VAT X, spec v2.2
//! section 5: a multi-purpose voucher carries no VAT when it is sold; the
//! VAT is due on the goods bought with it) and redeemed as a
//! `VOUCHER_STORE` payment. Only multi-purpose vouchers are supported
//! (V12-D24): single-purpose vouchers and a refund to a voucher or to
//! cash for a voucher are deferred.
//!
//! The FDM never hears of a voucher beyond the VAT X line and the payment
//! line. A voucher is created, and a redemption debited, in the archive
//! transaction of the signed sale (`signing::apply`), unconditionally: the
//! balance cannot move between the checks (under the signing lock, at
//! prepare) and the commit, since nothing is signed while an answer is
//! awaited. The codes are drawn at prepare and stored in the pending
//! context, so a recovered answer creates the same codes once.
//!
//! The code of a voucher is never part of an archived request, a log line
//! or a ticket: only the voucher slip prints it.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use pos_core::config::PosConfig;
use pos_core::order::{OrderLine, OrderStatus};
use pos_db::queries::gift_vouchers as db;
use pos_fiscal::payment::{PaymentMethod, Settlement};
use pos_fiscal::types::Mode;
use pos_print::gift_voucher::{BalanceSlip, VoucherSlip};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::commands::access::{self, Access};
use crate::commands::bar_slip::SlipOutcome;
use crate::commands::errors;
use crate::commands::owner_settings;
use crate::state::AppState;

/// The key of the gift vouchers in the owner settings store.
pub const KEY_GIFT_VOUCHERS: &str = "gift_vouchers";

/// Validity of a voucher: at least one year (consumer law), at most three.
pub const MIN_VALIDITY_MONTHS: i32 = 12;
pub const MAX_VALIDITY_MONTHS: i32 = 36;
pub const DEFAULT_VALIDITY_MONTHS: i32 = 12;
/// A voucher is worth at least 1,00 EUR.
pub const MIN_AMOUNT_CENTS: i64 = 100;
pub const DEFAULT_MAX_AMOUNT_CENTS: i64 = 50_000;
/// The highest maximum a manager can set.
pub const MAX_SETTING_AMOUNT_CENTS: i64 = 1_000_000;
/// The admin list shows at most this many vouchers.
const LIST_LIMIT: usize = 500;
/// Failed lookups allowed per minute per terminal.
const LOOKUP_FAILURES_PER_MINUTE: usize = 5;
const LOOKUP_WINDOW: Duration = Duration::from_secs(60);

fn default_validity() -> i32 {
    DEFAULT_VALIDITY_MONTHS
}

fn default_max_amount() -> i64 {
    DEFAULT_MAX_AMOUNT_CENTS
}

/// The gift voucher setting: off by default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GiftVoucherSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_validity")]
    pub validity_months: i32,
    #[serde(default = "default_max_amount")]
    pub max_amount_cents: i64,
}

impl Default for GiftVoucherSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            validity_months: DEFAULT_VALIDITY_MONTHS,
            max_amount_cents: DEFAULT_MAX_AMOUNT_CENTS,
        }
    }
}

fn settings(db: &Connection) -> GiftVoucherSettings {
    owner_settings::get::<GiftVoucherSettings>(db, KEY_GIFT_VOUCHERS).unwrap_or_default()
}

/// The settings, for any logged-in operator.
pub fn do_get_gift_voucher_settings(state: &AppState) -> Result<GiftVoucherSettings, String> {
    access::operator(state, Access::Staff)?;
    let db = state.db.lock().map_err(errors::internal)?;
    Ok(settings(&db))
}

/// Store the settings (managers only).
pub fn do_set_gift_voucher_settings(
    state: &AppState,
    input: GiftVoucherSettings,
) -> Result<GiftVoucherSettings, String> {
    let operator = access::operator(state, Access::Admin)?;
    let invalid =
        |field: &str| errors::coded(errors::GIFT_VOUCHER_SETTINGS_INVALID, &[("field", &field)]);
    if !(MIN_VALIDITY_MONTHS..=MAX_VALIDITY_MONTHS).contains(&input.validity_months) {
        return Err(invalid("validity_months"));
    }
    if !(MIN_AMOUNT_CENTS..=MAX_SETTING_AMOUNT_CENTS).contains(&input.max_amount_cents) {
        return Err(invalid("max_amount_cents"));
    }
    let db = state.db.lock().map_err(errors::internal)?;
    // Switching off hides the payment method and the lookup, so vouchers
    // already sold (valid at least 12 months) would become unredeemable:
    // refused while a real one can still be used, like the HubRise
    // switch-off while online orders are open.
    if !input.enabled && settings(&db).enabled {
        let open = db::count_redeemable(&db, &today()).map_err(errors::internal)?;
        if open > 0 {
            return Err(errors::coded(
                errors::GIFT_VOUCHERS_OUTSTANDING,
                &[("count", &open)],
            ));
        }
    }
    owner_settings::set(&db, KEY_GIFT_VOUCHERS, &input, Some(&operator.insz))?;
    Ok(input)
}

// ── Codes ──────────────────────────────────────────────────────

/// The two check digits of the first ten digits: `97 - (n mod 97)`, in
/// the style of the Belgian structured communication.
fn check_digits(first_ten: u64) -> u64 {
    97 - first_ten % 97
}

/// The 12-digit code made of ten digits and their check digits.
pub fn make_code(first_ten: u64) -> String {
    let first_ten = first_ten % 10_000_000_000;
    format!("{first_ten:010}{:02}", check_digits(first_ten))
}

/// Whether `code` is 12 digits whose last two are the check digits of
/// the first ten.
pub fn valid_code(code: &str) -> bool {
    if code.len() != 12 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let first: u64 = code[..10].parse().unwrap_or(0);
    let check: u64 = code[10..].parse().unwrap_or(0);
    check == check_digits(first)
}

/// `1234 5678 9012` and `1234-5678-9012` read as the bare 12 digits.
fn normalize(code: &str) -> String {
    code.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect()
}

/// `**** **** 9012`: what the screens and logs show of a code.
pub fn mask(code: &str) -> String {
    // Counted in chars: a code that is not 12 ASCII digits cannot panic
    // on a byte boundary.
    let skip = code.chars().count().saturating_sub(4);
    let tail: String = code.chars().skip(skip).collect();
    format!("**** **** {tail}")
}

/// A fresh code that no stored voucher and none of `taken` has.
fn draw_code(conn: &Connection, taken: &[String]) -> Result<String, String> {
    for _ in 0..32 {
        let random = uuid::Uuid::new_v4().as_u128();
        let code = make_code(u64::try_from(random % 10_000_000_000).unwrap_or_default());
        if taken.contains(&code) {
            continue;
        }
        if db::find_by_code(conn, &code)
            .map_err(errors::internal)?
            .is_none()
        {
            return Ok(code);
        }
    }
    Err(errors::internal("no free gift voucher code"))
}

// ── Lookup throttle ────────────────────────────────────────────

/// The failed lookups of the last minute, in memory (a restart clears
/// them): at most five a minute per terminal, so a code cannot be
/// guessed at the till.
#[derive(Debug, Default)]
pub struct LookupThrottle(Mutex<VecDeque<Instant>>);

impl LookupThrottle {
    fn check(&self, now: Instant) -> Result<(), String> {
        let mut failures = self.0.lock().map_err(errors::internal)?;
        while failures
            .front()
            .is_some_and(|at| now.duration_since(*at) >= LOOKUP_WINDOW)
        {
            failures.pop_front();
        }
        if failures.len() >= LOOKUP_FAILURES_PER_MINUTE {
            let oldest = failures.front().copied().unwrap_or(now);
            let left = LOOKUP_WINDOW.saturating_sub(now.duration_since(oldest));
            let seconds = left.as_secs() + u64::from(left.subsec_nanos() > 0);
            return Err(errors::coded(
                errors::GIFT_VOUCHER_LOOKUP_THROTTLED,
                &[("seconds", &seconds)],
            ));
        }
        Ok(())
    }

    fn fail(&self, now: Instant) {
        if let Ok(mut failures) = self.0.lock() {
            failures.push_back(now);
        }
    }
}

// ── Views ──────────────────────────────────────────────────────

/// One movement of a voucher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GiftVoucherMovement {
    /// `issue`, `redeem` or `cancel`.
    pub kind: String,
    pub amount_cents: i64,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// The ticket that made it (`N 1234`).
    pub ticket: String,
}

/// A voucher as the screens show it. The code is masked: the full code
/// is printed on the voucher only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GiftVoucherView {
    pub id: i64,
    pub code_masked: String,
    pub face_value_cents: i64,
    pub balance_cents: i64,
    /// `YYYY-MM-DD`.
    pub issued_on: String,
    pub expires_on: String,
    /// `active`, `used`, `expired` or `cancelled`; a lookup also answers
    /// `not_usable` for a voucher of the other mode (a training voucher
    /// in normal mode, a real one in training mode).
    pub status: String,
    pub training: bool,
    pub movements: Vec<GiftVoucherMovement>,
}

/// The local date today (Belgian time), `YYYY-MM-DD`.
fn today() -> String {
    pos_fiscal::event::now_in_belgium().date_naive().to_string()
}

/// The lookup status of a voucher that belongs to the other mode.
pub const STATUS_NOT_USABLE: &str = "not_usable";

fn status_of(row: &db::VoucherRow, today: &str) -> &'static str {
    if row.cancelled {
        "cancelled"
    } else if row.balance_cents <= 0 {
        "used"
    } else if row.expires_on.as_str() < today {
        "expired"
    } else {
        "active"
    }
}

/// Whether a voucher view carries its movements.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Movements {
    With,
    Without,
}

fn view_of(
    conn: &Connection,
    row: &db::VoucherRow,
    movements: Movements,
) -> Result<GiftVoucherView, String> {
    let movements = if movements == Movements::With {
        db::movements(conn, row.id)
            .map_err(errors::internal)?
            .into_iter()
            .map(|m| GiftVoucherMovement {
                kind: m.kind,
                amount_cents: m.amount_cents,
                date: m.date,
                ticket: m.ticket,
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(GiftVoucherView {
        id: row.id,
        code_masked: mask(&row.code),
        face_value_cents: row.face_value_cents,
        balance_cents: row.balance_cents,
        issued_on: row.issued_on.clone(),
        expires_on: row.expires_on.clone(),
        status: status_of(row, &today()).to_string(),
        training: row.training,
        movements,
    })
}

/// Find a voucher by the code the customer reads out or shows (spaces
/// allowed). A malformed code, a wrong check digit or an unknown code
/// counts against the lookup throttle (five a minute).
pub fn do_lookup_gift_voucher(state: &AppState, code: &str) -> Result<GiftVoucherView, String> {
    access::operator(state, Access::Cashier)?;
    let now = Instant::now();
    state.gift_voucher_lookups.check(now)?;
    let code = normalize(code);
    if !valid_code(&code) {
        state.gift_voucher_lookups.fail(now);
        return Err(errors::GIFT_VOUCHER_CODE_INVALID.to_string());
    }
    let conn = state.db.lock().map_err(errors::internal)?;
    let Some(row) = db::find_by_code(&conn, &code).map_err(errors::internal)? else {
        state.gift_voucher_lookups.fail(now);
        return Err(errors::GIFT_VOUCHER_NOT_FOUND.to_string());
    };
    let training_mode = state.config.lock().map_err(errors::internal)?.training_mode;
    let mut view = view_of(&conn, &row, Movements::With)?;
    if row.training != training_mode {
        // The sale refuses it (`GIFT_VOUCHER_NOT_USABLE`): the dialog
        // says so before the cashier tries.
        view.status = STATUS_NOT_USABLE.to_string();
    }
    Ok(view)
}

/// The vouchers, outstanding first (managers only). `status` filters:
/// `active`, `used`, `expired` or `cancelled`; none or `all` lists
/// every voucher. At most 500.
pub fn do_list_gift_vouchers(
    state: &AppState,
    status: Option<String>,
) -> Result<Vec<GiftVoucherView>, String> {
    access::operator(state, Access::Admin)?;
    let conn = state.db.lock().map_err(errors::internal)?;
    let filter = status.as_deref().filter(|s| !s.is_empty() && *s != "all");
    let today = today();
    let rows = db::list(&conn, 10_000).map_err(errors::internal)?;
    let mut kept: Vec<(u8, db::VoucherRow)> = rows
        .into_iter()
        .filter_map(|row| {
            let status = status_of(&row, &today);
            if filter.is_some_and(|f| f != status) {
                return None;
            }
            let rank = match status {
                "active" => 0,
                "expired" => 1,
                "used" => 2,
                _ => 3,
            };
            Some((rank, row))
        })
        .collect();
    // Outstanding first, newest first inside a status (the query is newest first).
    kept.sort_by_key(|(rank, _)| *rank);
    kept.truncate(LIST_LIMIT);
    kept.iter()
        .map(|(_, row)| view_of(&conn, row, Movements::Without))
        .collect()
}

// ── Selling a voucher ──────────────────────────────────────────

fn voucher_names(language: &str) -> (&'static str, &'static str) {
    match pos_fiscal::ticket::ReceiptLanguage::from_code(language).code() {
        "nl" => ("Cadeaubon", "Cadeaubonnen"),
        "de" => ("Geschenkgutschein", "Geschenkgutscheine"),
        _ => ("Bon cadeau", "Bons cadeaux"),
    }
}

fn euros_text(cents: i64) -> String {
    format!("{},{:02}", cents / 100, cents % 100)
}

/// Add a voucher of `amount_cents` to an open order: one line of quantity
/// 1, VAT X, department `GV`. Names are fiscal data, in the
/// establishment's language.
pub fn do_add_gift_voucher_line(
    state: &AppState,
    order_id: i64,
    amount_cents: i64,
) -> Result<OrderLine, String> {
    access::operator(state, Access::Cashier)?;
    let language = state
        .config
        .lock()
        .map_err(errors::internal)?
        .language
        .clone();
    let conn = state.db.lock().map_err(errors::internal)?;
    let setting = settings(&conn);
    if !setting.enabled {
        return Err(errors::GIFT_VOUCHERS_DISABLED.to_string());
    }
    if amount_cents < MIN_AMOUNT_CENTS || amount_cents > setting.max_amount_cents {
        return Err(errors::coded(
            errors::GIFT_VOUCHER_AMOUNT_INVALID,
            &[("max", &euros_text(setting.max_amount_cents))],
        ));
    }
    let order = pos_db::queries::orders::get_order(&conn, order_id).map_err(errors::internal)?;
    if order.status != OrderStatus::Open {
        return Err(errors::ORDER_NOT_OPEN.to_string());
    }
    super::order::unchanged_while_pending(&conn, order_id)?;
    let (name, department_name) = voucher_names(&language);
    let item = db::ensure_system_item(&conn).map_err(errors::internal)?;
    pos_db::queries::orders::add_order_line(
        &conn,
        pos_db::queries::orders::NewOrderLine {
            order_id,
            menu_item_id: item,
            quantity: 1,
            unit_price_cents: amount_cents,
            vat_code: pos_core::vat::VatCode::X,
            name,
            modifiers_json: None,
            department_id: db::DEPARTMENT_ID,
            department_name,
        },
    )
    .map_err(errors::internal)
}

/// Whether the line sells a voucher (its quantity is fixed).
pub(crate) fn is_voucher_line(conn: &Connection, line_id: i64) -> bool {
    db::is_voucher_line(conn, line_id).unwrap_or(false)
}

// ── Paying a sale ──────────────────────────────────────────────

/// A voucher a sale creates: stored in the pending context so a late
/// answer creates the same one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct IssuedVoucher {
    pub line_id: i64,
    pub code: String,
    pub face_value_cents: i64,
    pub issued_on: String,
    pub expires_on: String,
}

/// A voucher a sale redeems, and how much.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Redemption {
    pub voucher_id: i64,
    pub amount_cents: i64,
}

/// What a sale does to vouchers.
#[derive(Debug, Default)]
pub(crate) struct SalePlan {
    pub issued: Vec<IssuedVoucher>,
    pub redemptions: Vec<Redemption>,
}

/// Local date + `months`, clamped to the end of a shorter month.
fn add_months(date: chrono::NaiveDate, months: i32) -> chrono::NaiveDate {
    date.checked_add_months(chrono::Months::new(
        u32::try_from(months.max(0)).unwrap_or_default(),
    ))
    .unwrap_or(date)
}

/// Check the vouchers a sale sells and redeems, under the signing lock,
/// and draw the codes of the new ones. `settlement` is `None` for a sale
/// closed unpaid. Refused before anything is signed.
pub(crate) fn plan_sale(
    conn: &Connection,
    config: &PosConfig,
    order_id: i64,
    settlement: Option<&Settlement>,
) -> Result<SalePlan, String> {
    let lines = db::voucher_lines(conn, order_id).map_err(errors::internal)?;
    let gift_parts: Vec<_> = settlement
        .map(|s| {
            s.payments
                .iter()
                .filter(|p| p.method == PaymentMethod::GiftVoucher)
                .collect()
        })
        .unwrap_or_default();
    let meal_parts = settlement.is_some_and(|s| {
        s.payments
            .iter()
            .any(|p| p.method == PaymentMethod::MealVoucher)
    });
    let payment_invalid = || errors::GIFT_VOUCHER_PAYMENT_INVALID.to_string();

    if !lines.is_empty() {
        // A voucher is paid for: never left unpaid, never paid with
        // meal vouchers or another gift voucher.
        if settlement.is_none() || meal_parts || !gift_parts.is_empty() {
            return Err(payment_invalid());
        }
    }

    let today_date = pos_fiscal::event::now_in_belgium().date_naive();
    let today = today_date.to_string();

    let mut redemptions: Vec<Redemption> = Vec::new();
    for part in gift_parts {
        let Some(voucher_id) = part.gift_voucher_id else {
            return Err(payment_invalid());
        };
        if redemptions.iter().any(|r| r.voucher_id == voucher_id) {
            return Err(payment_invalid());
        }
        let Some(row) = db::get(conn, voucher_id).map_err(errors::internal)? else {
            return Err(errors::GIFT_VOUCHER_NOT_FOUND.to_string());
        };
        if row.cancelled || row.training != config.training_mode {
            return Err(errors::GIFT_VOUCHER_NOT_USABLE.to_string());
        }
        if row.expires_on.as_str() < today.as_str() {
            return Err(errors::GIFT_VOUCHER_EXPIRED.to_string());
        }
        if part.amount_cents <= 0 || part.amount_cents > row.balance_cents {
            return Err(errors::GIFT_VOUCHER_INSUFFICIENT.to_string());
        }
        redemptions.push(Redemption {
            voucher_id,
            amount_cents: part.amount_cents,
        });
    }

    let validity = settings(conn).validity_months;
    let expires_on = add_months(today_date, validity).to_string();
    let mut issued: Vec<IssuedVoucher> = Vec::new();
    for (line_id, face_value_cents) in lines {
        let taken: Vec<String> = issued.iter().map(|i| i.code.clone()).collect();
        issued.push(IssuedVoucher {
            line_id,
            code: draw_code(conn, &taken)?,
            face_value_cents,
            issued_on: today.clone(),
            expires_on: expires_on.clone(),
        });
    }
    Ok(SalePlan {
        issued,
        redemptions,
    })
}

/// Write the vouchers a signed sale sold and the redemptions it took, in
/// its archive transaction. Unconditional: the sale is signed.
pub(crate) fn apply_sale(
    tx: &Connection,
    order_id: i64,
    event_id: i64,
    mode: Mode,
    issued: &[IssuedVoucher],
    redemptions: &[Redemption],
) -> Result<(), String> {
    for voucher in issued {
        db::insert_voucher(
            tx,
            &db::NewVoucher {
                code: &voucher.code,
                face_value_cents: voucher.face_value_cents,
                issued_on: &voucher.issued_on,
                expires_on: &voucher.expires_on,
                training: mode.is_training(),
                issue_event_id: event_id,
                issue_order_id: order_id,
                issue_line_id: voucher.line_id,
            },
        )
        .map_err(errors::internal)?;
    }
    for redemption in redemptions {
        db::insert_movement(
            tx,
            redemption.voucher_id,
            "redeem",
            -redemption.amount_cents,
            event_id,
            Some(order_id),
        )
        .map_err(errors::internal)?;
    }
    Ok(())
}

// ── Refunds ────────────────────────────────────────────────────

/// The vouchers a refund cancels: for each voucher line refunded (the
/// quantity at a unit price), as many unused vouchers of that face value
/// as were sold on the original order. A refund of a voucher that was
/// used (even in part) is refused, and nothing is signed. Vouchers are
/// never re-credited: refunds stay cash (V12-D28).
pub(crate) fn vouchers_to_cancel(
    conn: &Connection,
    original_order_id: i64,
    refunded: &[(i64, i64, i64)],
) -> Result<Vec<i64>, String> {
    let item = db::system_item_id(conn).map_err(errors::internal)?;
    let mut cancelled: Vec<i64> = Vec::new();
    for &(menu_item_id, quantity, unit_price_cents) in refunded {
        if menu_item_id != item {
            continue;
        }
        let mut candidates: Vec<db::VoucherRow> = Vec::new();
        for (line_id, price) in
            db::voucher_lines(conn, original_order_id).map_err(errors::internal)?
        {
            if price != unit_price_cents {
                continue;
            }
            // A database error is not "no voucher": it would read as a used one.
            if let Some(row) = db::find_by_line(conn, line_id).map_err(errors::internal)? {
                candidates.push(row);
            }
        }
        let mut remaining = quantity;
        for row in candidates {
            if remaining == 0 {
                break;
            }
            if row.cancelled || cancelled.contains(&row.id) {
                continue;
            }
            if row.balance_cents != row.face_value_cents {
                continue;
            }
            cancelled.push(row.id);
            remaining -= 1;
        }
        if remaining > 0 {
            return Err(errors::GIFT_VOUCHER_USED.to_string());
        }
    }
    Ok(cancelled)
}

/// Cancel the vouchers of a refunded sale, in the refund's archive
/// transaction: a `cancel` movement of the whole remaining balance.
pub(crate) fn apply_refund(
    tx: &Connection,
    refund_order_id: i64,
    event_id: i64,
    voucher_ids: &[i64],
) -> Result<(), String> {
    for &id in voucher_ids {
        let Some(row) = db::get(tx, id).map_err(errors::internal)? else {
            continue;
        };
        if row.cancelled || row.balance_cents <= 0 {
            continue;
        }
        db::insert_movement(
            tx,
            id,
            "cancel",
            -row.balance_cents,
            event_id,
            Some(refund_order_id),
        )
        .map_err(errors::internal)?;
    }
    Ok(())
}

// ── Printing ───────────────────────────────────────────────────

/// What became of one voucher slip after a sale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GiftVoucherPrint {
    pub voucher_id: i64,
    /// `voucher` (the voucher itself) or `balance` (what is left on it).
    pub kind: String,
    pub outcome: SlipOutcome,
}

/// `2026-10-17` as `17.10.2026`.
fn display_date(iso: &str) -> String {
    match iso.split('-').collect::<Vec<_>>().as_slice() {
        [y, m, d] => format!("{d}.{m}.{y}"),
        _ => iso.to_string(),
    }
}

fn printer_language(config: &PosConfig) -> String {
    pos_fiscal::ticket::ReceiptLanguage::from_code(&config.language)
        .code()
        .to_string()
}

async fn send(state: &AppState, config: &PosConfig, bytes: &[u8]) -> SlipOutcome {
    let Some(address) = config.printer_address.as_deref().filter(|a| !a.is_empty()) else {
        return SlipOutcome::NoPrinter;
    };
    match pos_print::transport::send_to_printer_with(
        address,
        bytes,
        state.timers.printer_status_read,
    )
    .await
    {
        Ok(status) if status.ready() => SlipOutcome::Printed,
        Ok(status) => {
            tracing::warn!(?status.warnings, "gift voucher slip: printer not ready");
            SlipOutcome::Failed
        }
        Err(err) => {
            tracing::warn!(%err, "gift voucher slip not printed");
            SlipOutcome::Failed
        }
    }
}

fn address_lines(config: &PosConfig) -> Vec<String> {
    config
        .restaurant_address
        .split('\n')
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// A voucher slip is the original or a reprint.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SlipKind {
    Original,
    Duplicate,
}

fn voucher_slip(
    config: &PosConfig,
    row: &db::VoucherRow,
    reference: Option<String>,
    kind: SlipKind,
) -> VoucherSlip {
    VoucherSlip {
        establishment: config.restaurant_name.clone(),
        address: address_lines(config),
        face_value_cents: row.face_value_cents,
        code: row.code.clone(),
        issued_on: display_date(&row.issued_on),
        expires_on: display_date(&row.expires_on),
        reference,
        lang: printer_language(config),
        duplicate: kind == SlipKind::Duplicate,
        training: row.training,
    }
}

/// Print what a signed sale gives: each voucher it sold, then a balance
/// slip for each voucher it redeemed that still has a balance. Best
/// effort: the sale stands whatever the printer says.
pub(crate) async fn print_after_sale(
    state: &AppState,
    config: &PosConfig,
    event_id: i64,
    issued: &[IssuedVoucher],
    redemptions: &[Redemption],
) -> Vec<GiftVoucherPrint> {
    let mut done = Vec::new();
    for voucher in issued {
        let (row, reference) = {
            let Ok(conn) = state.db.lock() else { continue };
            let Ok(Some(row)) = db::find_by_code(&conn, &voucher.code) else {
                continue;
            };
            let reference = db::event_ticket(&conn, event_id).ok();
            (row, reference)
        };
        let bytes = pos_print::gift_voucher::format_gift_voucher(
            &voucher_slip(config, &row, reference, SlipKind::Original),
            config.printer_paper_mm,
        );
        let outcome = send(state, config, &bytes).await;
        if outcome == SlipOutcome::Printed
            && let Ok(conn) = state.db.lock()
        {
            let _ = db::record_print(&conn, row.id, "original", None);
        }
        done.push(GiftVoucherPrint {
            voucher_id: row.id,
            kind: "voucher".into(),
            outcome,
        });
    }
    for redemption in redemptions {
        let row = {
            let Ok(conn) = state.db.lock() else { continue };
            match db::get(&conn, redemption.voucher_id) {
                Ok(Some(row)) if row.balance_cents > 0 => row,
                _ => continue,
            }
        };
        let slip = BalanceSlip {
            establishment: config.restaurant_name.clone(),
            // Stored codes are 12 ASCII digits: the byte slice is safe.
            code_last4: row.code[row.code.len().saturating_sub(4)..].to_string(),
            balance_cents: row.balance_cents,
            expires_on: display_date(&row.expires_on),
            lang: printer_language(config),
            training: row.training,
        };
        let outcome = send(
            state,
            config,
            &pos_print::gift_voucher::format_balance_slip(&slip, config.printer_paper_mm),
        )
        .await;
        if outcome == SlipOutcome::Printed
            && let Ok(conn) = state.db.lock()
        {
            let _ = db::record_print(&conn, row.id, "balance", None);
        }
        done.push(GiftVoucherPrint {
            voucher_id: row.id,
            kind: "balance".into(),
            outcome,
        });
    }
    done
}

/// Print a voucher again, marked DUPLICATA (managers only). A cancelled
/// voucher is not printed.
pub async fn do_print_gift_voucher(
    state: &AppState,
    voucher_id: i64,
) -> Result<SlipOutcome, String> {
    let operator = access::operator(state, Access::Admin)?;
    let config = state.config.lock().map_err(errors::internal)?.clone();
    let (row, reference) = {
        let conn = state.db.lock().map_err(errors::internal)?;
        let Some(row) = db::get(&conn, voucher_id).map_err(errors::internal)? else {
            return Err(errors::GIFT_VOUCHER_NOT_FOUND.to_string());
        };
        if row.cancelled {
            return Err(errors::GIFT_VOUCHER_NOT_USABLE.to_string());
        }
        let reference = db::event_ticket(&conn, row.issue_event_id).ok();
        (row, reference)
    };
    let bytes = pos_print::gift_voucher::format_gift_voucher(
        &voucher_slip(&config, &row, reference, SlipKind::Duplicate),
        config.printer_paper_mm,
    );
    let outcome = send(state, &config, &bytes).await;
    if outcome == SlipOutcome::Printed {
        let conn = state.db.lock().map_err(errors::internal)?;
        db::record_print(&conn, row.id, "duplicate", Some(&operator.insz))
            .map_err(errors::internal)?;
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_has_ten_digits_and_two_check_digits() {
        assert_eq!(make_code(1_234_567_890), "123456789095");
        assert!(valid_code("123456789095"));
        assert!(!valid_code("123456789012"));
        // n mod 97 = 0 gives 97, never 00.
        assert_eq!(make_code(97), "000000009797");
        assert!(valid_code("000000009797"));
    }

    #[test]
    fn every_generated_code_validates() {
        for n in [0_u64, 1, 96, 97, 98, 9_999_999_999, 5_000_000_000] {
            let code = make_code(n);
            assert_eq!(code.len(), 12);
            assert!(valid_code(&code), "{code}");
        }
    }

    #[test]
    fn a_wrong_check_digit_or_shape_is_invalid() {
        let good = make_code(4_242_424_242);
        assert!(valid_code(&good));
        for bad in [
            "",
            "12345678901",
            "1234567890123",
            "12345678901a",
            "abcdefghijkl",
        ] {
            assert!(!valid_code(bad), "{bad}");
        }
        let mut wrong = good;
        let last = wrong.pop().unwrap();
        wrong.push(if last == '0' { '1' } else { '0' });
        assert!(!valid_code(&wrong));
    }

    #[test]
    fn spaces_and_dashes_are_ignored_and_the_mask_keeps_four_digits() {
        assert_eq!(normalize("1234 5678 9095"), "123456789095");
        assert_eq!(normalize(" 1234-5678-9095 "), "123456789095");
        assert_eq!(mask("123456789095"), "**** **** 9095");
        // Never a panic on a code that is not ASCII, or too short.
        assert_eq!(mask("12é4567"), "**** **** 4567");
        assert_eq!(mask("é"), "**** **** é");
    }

    #[test]
    fn the_sixth_failure_in_a_minute_is_throttled() {
        let throttle = LookupThrottle::default();
        let start = Instant::now();
        for _ in 0..5 {
            assert!(throttle.check(start).is_ok());
            throttle.fail(start);
        }
        let err = throttle.check(start).unwrap_err();
        assert!(
            err.starts_with("gift_voucher_lookup_throttled|seconds="),
            "{err}"
        );
        // A minute later the window is clear.
        assert!(throttle.check(start + Duration::from_secs(61)).is_ok());
    }

    #[test]
    fn expiry_is_the_issue_date_plus_the_validity_clamped_to_the_month() {
        let d = |y, m, day| chrono::NaiveDate::from_ymd_opt(y, m, day).unwrap();
        assert_eq!(add_months(d(2026, 10, 17), 12), d(2027, 10, 17));
        assert_eq!(add_months(d(2028, 2, 29), 12), d(2029, 2, 28));
        assert_eq!(add_months(d(2026, 1, 31), 36), d(2029, 1, 31));
    }

    #[test]
    fn the_names_follow_the_establishment_language() {
        assert_eq!(voucher_names("fr"), ("Bon cadeau", "Bons cadeaux"));
        assert_eq!(voucher_names("nl").0, "Cadeaubon");
        assert_eq!(voucher_names("de").0, "Geschenkgutschein");
        assert_eq!(voucher_names("en").0, "Bon cadeau");
    }

    #[test]
    fn dates_print_day_first() {
        assert_eq!(display_date("2026-10-17"), "17.10.2026");
        assert_eq!(euros_text(50_000), "500,00");
    }
}
