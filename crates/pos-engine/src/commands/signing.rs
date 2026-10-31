//! Every fiscal event is signed under one lock, from the choice of its
//! ticket number to its archive, so two events never share a number and
//! a Z never misses an event signed while it was made.
//!
//! A sign request is stored, byte for byte, before it is sent
//! (`pending_signs`, migration 10). When the FDM's answer is lost
//! (timeout, cut connection, unreadable answer) the request stays open:
//! before anything else is signed it is sent again unchanged, and the
//! FDM, which may have signed it, returns its first answer (POS
//! description v2.2, duplicate requests). The event is then archived
//! with the effects stored with it, so nothing signed is ever lost and
//! no number is used twice. Until the answer comes, nothing is signed.

use pos_fiscal::types::Mode;
use std::time::Duration;

use pos_core::config::PosConfig;
use pos_db::queries::pending_signs::{self, NewPendingSign, PendingSign, Unsigned};
use pos_fiscal::archive::{self, ArchiveParams};
use pos_fiscal::counter::FiscalCounter;
use pos_fiscal::fdm_client::{FdmError, SignOutcome, SignRequest};
use pos_fiscal::response::SignResult;
use pos_fiscal::ticket::{FiscalTicket, TicketDraft};
use pos_fiscal::types::{EventLabel, TicketMedium};
use pos_print::report::{ReportPrintData, UserReportPrintData};
use serde::{Deserialize, Serialize};

use crate::commands::bar_slip::{self, SlipOutcome};
use crate::commands::delivery::{self, DeliveryOutcome, ResolvedDelivery};
use crate::commands::errors;
use crate::commands::gift_vouchers::{self, GiftVoucherPrint, IssuedVoucher, Redemption};
use crate::state::AppState;

/// How long a command waits for another event to be signed.
pub const LOCK_WAIT: Duration = Duration::from_secs(30);

/// The one lock every signature holds.
#[derive(Debug, Default)]
pub struct SigningLock(tokio::sync::Mutex<()>);

/// Proof the signing lock is held: needed to number or sign an event.
#[derive(Debug)]
pub struct SignGuard<'a>(
    #[expect(
        dead_code,
        reason = "held for its Drop: the lock is released with the proof"
    )]
    tokio::sync::MutexGuard<'a, ()>,
);

/// Wait for the signing lock, at most [`LOCK_WAIT`].
pub async fn acquire(state: &AppState) -> Result<SignGuard<'_>, String> {
    tokio::time::timeout(LOCK_WAIT, state.signing.0.lock())
        .await
        .map(SignGuard)
        .map_err(|_| errors::SIGNING_BUSY.to_string())
}

/// The signing lock if it is free now.
pub fn try_acquire(state: &AppState) -> Option<SignGuard<'_>> {
    state.signing.0.try_lock().ok().map(SignGuard)
}

/// The ticket number the next event of `label` takes.
pub(crate) fn peek(
    db: &rusqlite::Connection,
    _guard: &SignGuard<'_>,
    label: EventLabel,
) -> Result<i64, String> {
    FiscalCounter::peek_next(db, label).map_err(errors::internal)
}

// ── Operations ─────────────────────────────────────────────────

/// What a command signs, to recognise its retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Sale,
    Refund,
    TableOrder,
    PreBill,
    TableTransfer,
    PaymentCorrection,
    Discount,
    WebshopOrder,
    ReportX,
    ReportUserX,
    ReportZ,
    Copy,
    WorkIn,
    WorkOut,
    Money,
    Drawer,
    Invoice,
}

impl Kind {
    const ALL: [Kind; 17] = [
        Kind::Sale,
        Kind::Refund,
        Kind::TableOrder,
        Kind::PreBill,
        Kind::TableTransfer,
        Kind::PaymentCorrection,
        Kind::Discount,
        Kind::WebshopOrder,
        Kind::ReportX,
        Kind::ReportUserX,
        Kind::ReportZ,
        Kind::Copy,
        Kind::WorkIn,
        Kind::WorkOut,
        Kind::Money,
        Kind::Drawer,
        Kind::Invoice,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Kind::Sale => "sale",
            Kind::Refund => "refund",
            Kind::TableOrder => "table_order",
            Kind::PreBill => "pre_bill",
            Kind::TableTransfer => "table_transfer",
            Kind::PaymentCorrection => "payment_correction",
            Kind::Discount => "discount",
            Kind::WebshopOrder => "webshop_order",
            Kind::ReportX => "report_x",
            Kind::ReportUserX => "report_user_x",
            Kind::ReportZ => "report_z",
            Kind::Copy => "copy",
            Kind::WorkIn => "work_in",
            Kind::WorkOut => "work_out",
            Kind::Money => "money",
            Kind::Drawer => "drawer",
            Kind::Invoice => "invoice",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.name() == name)
    }
}

/// One operation: its kind and what it is about (an order, the sale a
/// refund reverses, the arguments of a cash movement...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Op {
    pub kind: Kind,
    pub subject: String,
}

impl Op {
    pub(crate) fn new(kind: Kind, subject: impl ToString) -> Self {
        Self {
            kind,
            subject: subject.to_string(),
        }
    }
}

// ── What is stored with a request ─────────────────────────────

/// One event to archive once signed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ArchiveSpec {
    pub order_id: Option<i64>,
    pub user_insz: String,
    /// The request as archived (`fiscal_events.event_data_json`).
    pub request_json: String,
    /// `(order_id, last_entry_id)`: the order entries it transmits.
    pub sent_entries: Option<(i64, i64)>,
}

/// The ticket's medium and address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredDelivery {
    pub medium: TicketMedium,
    pub email: Option<String>,
}

impl StoredDelivery {
    pub(crate) fn of(resolved: &ResolvedDelivery) -> Self {
        Self {
            medium: resolved.medium,
            email: resolved.email.clone(),
        }
    }

    pub(crate) fn resolved(&self) -> ResolvedDelivery {
        ResolvedDelivery {
            medium: self.medium,
            email: self.email.clone(),
        }
    }
}

/// A payment row of a sale (`payments` table).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredPayment {
    pub method: String,
    pub amount_cents: i64,
}

/// The bar / kitchen slip of the entries an event registers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BarSlip {
    pub operator_insz: String,
    pub table_number: Option<i32>,
}

/// The copy's ticket: the original rebuilt from its archive, or, when
/// it cannot be read, the copy's own data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum CopyTicket {
    Original(Box<FiscalTicket>),
    Draft(Box<TicketDraft>),
    /// The copy of an invoice: the invoice print, the copy's own control data added when the
    /// copy is signed.
    Invoice(Box<pos_print::invoice::InvoicePrint>),
}

/// What an event does besides its archive, stored with its request so
/// the same happens whenever the answer comes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum PendingContext {
    Sale {
        order_id: i64,
        /// No payment row (training).
        training: bool,
        payments: Vec<StoredPayment>,
        cash: bool,
        delivery: StoredDelivery,
        ticket: TicketDraft,
        bar_slip: Option<BarSlip>,
        webshop_order_id: Option<i64>,
        /// The till sale echoed to `HubRise`.
        hubrise_echo: Option<String>,
        /// The card payments the linked terminal took that this sale
        /// settles (`eft_payments.id`): marked used with the sale, in
        /// its archive transaction, also when the answer comes late.
        /// Absent in requests stored before the terminal existed.
        #[serde(default)]
        eft_payment_ids: Vec<i64>,
        /// The gift vouchers this sale sells (codes drawn at prepare) and
        /// redeems: written in its archive transaction, also when the
        /// answer comes late, so a late answer creates the same codes.
        #[serde(default)]
        gift_vouchers_issued: Vec<IssuedVoucher>,
        #[serde(default)]
        gift_voucher_redemptions: Vec<Redemption>,
    },
    Refund {
        refund_order_id: i64,
        training: bool,
        method: String,
        total_cents: i64,
        open_drawer: bool,
        delivery: StoredDelivery,
        ticket: TicketDraft,
        webshop_order_id: Option<i64>,
        /// The gift vouchers the refunded sale sold, cancelled with this
        /// refund (`vouchers_to_cancel`).
        #[serde(default)]
        gift_vouchers_cancelled: Vec<i64>,
    },
    TableOrder {
        bar_slip: BarSlip,
    },
    PreBill {
        order_id: i64,
        ticket: TicketDraft,
    },
    /// An open order moved to another table (`signCostCenterChange`).
    TableTransfer {
        order_id: i64,
        from_table: i32,
        to_table: i32,
        operator_insz: String,
    },
    /// The payment method of sale `corrected_event_id` (its archive
    /// id) corrected (`signPaymentCorrection`). No local table changes:
    /// the reports read the archive; the context only names the sale.
    PaymentCorrection {
        corrected_event_id: i64,
    },
    WebshopOrder {
        order_id: i64,
    },
    ReportX {
        print: Box<ReportPrintData>,
    },
    ReportUserX {
        print: Box<UserReportPrintData>,
    },
    ReportZ {
        period_id: String,
        booking_date: String,
        report_no: i64,
        training: bool,
        print: Box<ReportPrintData>,
        user_print: Box<UserReportPrintData>,
    },
    Copy {
        delivery: StoredDelivery,
        ticket: Option<CopyTicket>,
    },
    Work {
        insz: String,
        is_in: bool,
        /// INSZ of who registered it when not the employee (a manager).
        /// Absent in contexts stored before migration 28.
        #[serde(default)]
        by: Option<String>,
    },
    Money,
    Drawer,
    Invoice {
        invoice_no: String,
        invoice_seq: i64,
        sale_event_ids: Vec<i64>,
        customer_id: i64,
        customer_json: String,
        issue_date: String,
        total_cents: i64,
        vat_json: String,
        print: Box<pos_print::invoice::InvoicePrint>,
        peppol: Option<crate::commands::invoice::PendingPeppol>,
    },
}

impl PendingContext {
    /// The context once resolved: a customer's address is not kept.
    fn resolved(&self) -> Self {
        let mut context = self.clone();
        match &mut context {
            PendingContext::Sale { delivery, .. }
            | PendingContext::Refund { delivery, .. }
            | PendingContext::Copy { delivery, .. } => delivery.email = None,
            _ => {}
        }
        context
    }
}

/// A signed and archived event.
pub(crate) struct Committed {
    pub event_ids: Vec<i64>,
    pub results: Vec<SignResult>,
    pub ticket: Option<FiscalTicket>,
    pub delivery_id: Option<i64>,
}

impl Committed {
    pub(crate) fn event_id(&self) -> i64 {
        self.event_ids[0]
    }

    pub(crate) fn result(&self) -> &SignResult {
        &self.results[0]
    }
}

/// What a command prepared to sign.
pub(crate) struct Prepared {
    pub op: Op,
    pub request: SignRequest,
    pub label: EventLabel,
    pub first_ticket_no: i64,
    pub events: Vec<ArchiveSpec>,
    pub order_id: Option<i64>,
    pub user_insz: String,
    pub context: PendingContext,
}

/// A signature that did not happen.
#[derive(Debug, Clone)]
pub(crate) struct SignFailure {
    pub error: String,
    /// The FDM could not be reached at all.
    pub not_connected: bool,
}

impl From<String> for SignFailure {
    fn from(error: String) -> Self {
        Self {
            error,
            not_connected: false,
        }
    }
}

/// `fdm`: what the FDM said (its error code, or `FDM_RESPONSE_MISMATCH`),
/// when it answered: the screens show its message first, then the
/// instruction that the same request is sent again.
pub(crate) fn answer_pending(
    label: &str,
    ticket_no: i64,
    fdm: Option<&str>,
    cause: &dyn std::fmt::Display,
) -> String {
    let mut values: Vec<(&str, &dyn std::fmt::Display)> =
        vec![("label", &label), ("ticket", &ticket_no)];
    if let Some(code) = &fdm {
        values.push(("fdm", code));
    }
    values.push(("cause", cause));
    format!(
        "FDM error: {}",
        errors::coded(errors::FDM_ANSWER_PENDING, &values)
    )
}

// ── Signing ────────────────────────────────────────────────────

/// Store `prepared`'s request, send it, and archive the event with its
/// effects. A request the FDM may have signed stays open, to be sent
/// again before anything else.
pub(crate) async fn sign(
    state: &AppState,
    guard: &SignGuard<'_>,
    config: &PosConfig,
    prepared: Prepared,
) -> Result<Committed, SignFailure> {
    let label = format!("{:?}", prepared.label);
    let archive_json = serde_json::to_string(&prepared.events).map_err(errors::internal)?;
    let context_json = serde_json::to_string(&prepared.context).map_err(errors::internal)?;
    let pending_id = {
        let db = state.db.lock().map_err(errors::internal)?;
        pending_signs::insert_open(
            &db,
            &NewPendingSign {
                event_label: &label,
                first_ticket_no: prepared.first_ticket_no,
                ticket_count: i64::try_from(prepared.events.len()).map_err(errors::internal)?,
                mutation: prepared.request.mutation().name(),
                is_training: prepared.request.is_training(),
                request_body: prepared.request.body(),
                archive_json: &archive_json,
                kind: prepared.op.kind.name(),
                subject: &prepared.op.subject,
                order_id: prepared.order_id,
                context_json: &context_json,
                user_insz: &prepared.user_insz,
            },
        )
        .map_err(errors::internal)?
    };

    let sent = state.fdm_client(config).send(&prepared.request).await;
    match sent {
        Ok(results) => commit(
            state,
            guard,
            config,
            pending_id,
            prepared.label,
            prepared.first_ticket_no,
            &prepared.events,
            &prepared.context,
            results,
        )
        .map_err(SignFailure::from),
        Err(e) => Err(failed(
            state,
            pending_id,
            &label,
            prepared.first_ticket_no,
            &e,
        )),
    }
}

/// Record a failed send and say what the operator sees.
fn failed(
    state: &AppState,
    pending_id: i64,
    label: &str,
    ticket_no: i64,
    e: &FdmError,
) -> SignFailure {
    let detail = e.to_string();
    let recorded = state.db.lock().map_err(errors::internal).and_then(|db| {
        match e.outcome() {
            SignOutcome::NotSent => {
                pending_signs::mark_unsigned(&db, pending_id, Unsigned::NotSent, &detail)
            }
            SignOutcome::Refused => {
                pending_signs::mark_unsigned(&db, pending_id, Unsigned::Refused, &detail)
            }
            SignOutcome::Unknown => pending_signs::record_attempt(&db, pending_id, Some(&detail)),
        }
        .map_err(errors::internal)
    });
    if let Err(e) = recorded {
        return e.into();
    }
    match e.outcome() {
        SignOutcome::Unknown => {
            tracing::error!(%detail, label, ticket_no, "FDM answer lost: request kept to send again");
            answer_pending(label, ticket_no, e.fdm_code().as_deref(), &detail).into()
        }
        _ => SignFailure {
            error: format!("FDM error: {e}"),
            not_connected: matches!(e, FdmError::NotConnected),
        },
    }
}

/// Advance the counter, archive each signed event, apply its effects
/// and close the request, in one transaction.
#[expect(
    clippy::too_many_arguments,
    reason = "each argument is a distinct input of one fiscal step; a parameter struct would only rename them"
)]
fn commit(
    state: &AppState,
    _guard: &SignGuard<'_>,
    config: &PosConfig,
    pending_id: i64,
    label: EventLabel,
    first_ticket_no: i64,
    events: &[ArchiveSpec],
    context: &PendingContext,
    results: Vec<SignResult>,
) -> Result<Committed, String> {
    if results.len() != events.len() {
        return Err(errors::internal(format!(
            "{} result(s) for {} event(s)",
            results.len(),
            events.len()
        )));
    }
    let db = state.db.lock().map_err(errors::internal)?;
    let tx = db.unchecked_transaction().map_err(errors::internal)?;
    let mut event_ids = Vec::with_capacity(events.len());
    for (i, (spec, result)) in events.iter().zip(&results).enumerate() {
        let ticket_no = first_ticket_no + i64::try_from(i).map_err(errors::internal)?;
        let committed = FiscalCounter::next(&tx, label).map_err(errors::internal)?;
        if committed != ticket_no {
            return Err(errors::concurrent(format_args!(
                "{label:?}: numbered {ticket_no}, counter at {committed}"
            )));
        }
        let id = archive::archive_event(
            &tx,
            ArchiveParams {
                event_type: label,
                sequence_number: ticket_no,
                order_id: spec.order_id,
                pos_id: &config.pos_id,
                terminal_id: &config.terminal_id,
                user_insz: &spec.user_insz,
                request_json: &spec.request_json,
                sign_result: Some(result),
            },
        )
        .map_err(archive_error)?;
        if let Some((order_id, last_entry_id)) = spec.sent_entries {
            pos_db::queries::orders::mark_order_entries_sent(&tx, order_id, last_entry_id, id)
                .map_err(errors::internal)?;
        }
        event_ids.push(id);
    }
    let (ticket, delivery_id) = apply(
        &tx,
        &state.identity,
        label,
        events,
        context,
        &event_ids,
        &results,
    )?;
    let resolved = serde_json::to_string(&context.resolved()).map_err(errors::internal)?;
    pending_signs::mark_signed(&tx, pending_id, event_ids[0], &resolved)
        .map_err(errors::internal)?;
    tx.commit().map_err(errors::internal)?;
    Ok(Committed {
        event_ids,
        results,
        ticket,
        delivery_id,
    })
}

fn archive_error(e: rusqlite::Error) -> String {
    if e.to_string().contains("booking period is closed") {
        errors::PERIOD_CLOSED.to_string()
    } else {
        errors::internal(e)
    }
}

/// The effects of the event, in its archive transaction: the ticket
/// built and its email queued, the order closed, the payments recorded.
#[expect(
    clippy::too_many_lines,
    reason = "one transaction applying a signed answer to every table it touches"
)]
fn apply(
    tx: &rusqlite::Connection,
    identity: &pos_fiscal::identity::SoftwareIdentity,
    label: EventLabel,
    events: &[ArchiveSpec],
    context: &PendingContext,
    event_ids: &[i64],
    results: &[SignResult],
) -> Result<(Option<FiscalTicket>, Option<i64>), String> {
    let close_order = |order_id: i64| {
        tx.execute(
            "UPDATE orders SET status = 'closed', closed_at = datetime('now') WHERE id = ?1",
            rusqlite::params![order_id],
        )
        .map_err(errors::internal)
    };
    let pay = |order_id: i64, method: &str, amount: i64| {
        tx.execute(
            "INSERT INTO payments (order_id, method, amount_cents) VALUES (?1, ?2, ?3)",
            rusqlite::params![order_id, method, amount],
        )
        .map_err(errors::internal)
    };
    match context {
        PendingContext::Sale {
            order_id,
            training,
            payments,
            delivery,
            ticket,
            hubrise_echo,
            eft_payment_ids,
            gift_vouchers_issued,
            gift_voucher_redemptions,
            ..
        } => {
            close_order(*order_id)?;
            crate::commands::eft::mark_used(tx, eft_payment_ids, event_ids[0])?;
            gift_vouchers::apply_sale(
                tx,
                *order_id,
                event_ids[0],
                Mode::from(*training),
                gift_vouchers_issued,
                gift_voucher_redemptions,
            )?;
            // Training moves no real money: no payment row.
            if !training {
                for p in payments {
                    pay(*order_id, &p.method, p.amount_cents)?;
                }
            }
            let ticket = FiscalTicket::from_draft(ticket, &results[0], identity);
            let delivery_id =
                delivery::enqueue_in_tx(tx, event_ids[0], &delivery.resolved(), Some(&ticket))?;
            #[cfg(feature = "hubrise")]
            if let Some(body) = hubrise_echo
                && let Err(e) = pos_hubrise::outbox::enqueue(tx, *order_id, body)
            {
                // Non-fatal: the sale stands, it only misses the echo.
                tracing::warn!(%e, "outbox enqueue failed (non-fatal)");
            }
            #[cfg(not(feature = "hubrise"))]
            let _ = hubrise_echo;
            Ok((Some(ticket), delivery_id))
        }
        PendingContext::Refund {
            refund_order_id,
            training,
            method,
            total_cents,
            delivery,
            ticket,
            gift_vouchers_cancelled,
            ..
        } => {
            // Closed even if it was voided while its answer was awaited.
            close_order(*refund_order_id)?;
            gift_vouchers::apply_refund(
                tx,
                *refund_order_id,
                event_ids[0],
                gift_vouchers_cancelled,
            )?;
            if !training {
                pay(*refund_order_id, method, *total_cents)?;
            }
            let ticket = FiscalTicket::from_draft(ticket, &results[0], identity);
            let delivery_id =
                delivery::enqueue_in_tx(tx, event_ids[0], &delivery.resolved(), Some(&ticket))?;
            Ok((Some(ticket), delivery_id))
        }
        PendingContext::PreBill { order_id, .. } => {
            pos_db::queries::orders::mark_pre_bill_printed(tx, *order_id)
                .map_err(errors::internal)?;
            Ok((None, None))
        }
        PendingContext::TableTransfer {
            order_id, to_table, ..
        } => {
            // The signed move is the fiscal truth, but the local state
            // must never hold two open orders on one table. An answer that
            // arrives late can find the target taken meanwhile: an open
            // order there with a line, a signed event or a payment. Then
            // the order stays on its source table (V1F-D60) and the caller
            // reports `table_transfer_not_moved`. Same query as the check
            // before signing (V1F-D68).
            if pos_db::queries::orders::table_taken(tx, *to_table, *order_id)
                .map_err(errors::internal)?
            {
                return Ok((None, None));
            }
            // A ghost order (opened and left empty) on the target does
            // not block the move; it is voided with it.
            pos_db::queries::orders::void_empty_on_table(tx, *to_table, *order_id)
                .map_err(errors::internal)?;
            tx.execute(
                "UPDATE orders SET table_number = ?1 WHERE id = ?2 AND status = 'open'",
                rusqlite::params![to_table, order_id],
            )
            .map_err(errors::internal)?;
            Ok((None, None))
        }
        PendingContext::ReportZ {
            period_id,
            booking_date,
            report_no,
            training,
            ..
        } => {
            // A training Z neither takes a Z number nor closes the period.
            if !training {
                pos_db::queries::reports::save_z_report(
                    tx,
                    booking_date,
                    *report_no,
                    &events[0].request_json,
                    Some(event_ids[0]),
                )
                .map_err(errors::internal)?;
                pos_db::queries::booking::close_period(tx, period_id, event_ids[0])
                    .map_err(errors::internal)?;
            }
            Ok((None, None))
        }
        PendingContext::Copy { delivery, ticket } => {
            let ticket = ticket.as_ref().and_then(|t| match t {
                CopyTicket::Original(original) => Some((**original).clone().into_copy(&results[0])),
                CopyTicket::Draft(draft) => {
                    Some(FiscalTicket::from_draft(draft, &results[0], identity))
                }
                // Printed as an invoice copy; no digital ticket.
                CopyTicket::Invoice(_) => None,
            });
            let delivery_id =
                delivery::enqueue_in_tx(tx, event_ids[0], &delivery.resolved(), ticket.as_ref())?;
            Ok((ticket, delivery_id))
        }
        PendingContext::Work { insz, is_in, by } => {
            // The local index reflects real presence only: a training
            // (T) event registers none. The screen lock keeps a shift
            // open; only a signed clock-out ends it.
            if label != EventLabel::T {
                if *is_in {
                    pos_db::queries::shifts::open_shift(tx, insz, by.as_deref())
                } else {
                    pos_db::queries::shifts::close_shift(tx, insz, by.as_deref())
                }
                .map_err(errors::internal)?;
            }
            Ok((None, None))
        }
        PendingContext::TableOrder { .. }
        | PendingContext::WebshopOrder { .. }
        | PendingContext::ReportX { .. }
        | PendingContext::ReportUserX { .. }
        | PendingContext::Money
        | PendingContext::Drawer
        | PendingContext::PaymentCorrection { .. } => Ok((None, None)),
        PendingContext::Invoice { .. } => {
            crate::commands::invoice::record_in_tx(
                tx,
                event_ids[0],
                &events[0].user_insz,
                context,
            )?;
            Ok((None, None))
        }
    }
}

// ── After the archive ──────────────────────────────────────────

/// What the follow-up of an event did.
#[derive(Default)]
pub(crate) struct FollowUp {
    pub delivery: Option<DeliveryOutcome>,
    pub slip: Option<SlipOutcome>,
    /// An invoice reached the printer (`None` for other events).
    pub invoice_printed: Option<bool>,
    /// The voucher and balance slips of a sale that sold or redeemed
    /// gift vouchers.
    pub vouchers: Vec<GiftVoucherPrint>,
}

/// Whether the cash drawer opens when an event is followed up.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Drawer {
    Kick,
    Leave,
}

/// Print and send what an archived event gives: its ticket, its bar
/// slip, its report. The drawer opens only for `with_drawer`: the
/// operator is there to take or give the money.
#[expect(
    clippy::too_many_lines,
    reason = "the recovery of an unfinished signature, one decision tree clearer unsplit"
)]
pub(crate) async fn follow_up(
    state: &AppState,
    config: &PosConfig,
    context: &PendingContext,
    committed: &Committed,
    drawer: Drawer,
) -> FollowUp {
    let with_drawer = drawer == Drawer::Kick;
    let mut done = FollowUp::default();
    let kick = |open: bool| async move {
        if open && let Some(addr) = config.printer_address.as_deref() {
            let _ = pos_print::transport::send_to_printer_with(
                addr,
                &pos_print::drawer::kick_drawer_bytes(),
                state.timers.printer_status_read,
            )
            .await;
        }
    };
    match context {
        PendingContext::Sale {
            cash,
            delivery,
            bar_slip,
            gift_vouchers_issued,
            gift_voucher_redemptions,
            ..
        } => {
            done.delivery = Some(
                delivery::deliver(
                    state,
                    config,
                    committed.event_id(),
                    &delivery.resolved(),
                    committed.delivery_id,
                    committed.ticket.as_ref(),
                )
                .await,
            );
            kick(with_drawer && *cash).await;
            done.slip = Some(match bar_slip {
                Some(slip) => print_slip_of(state, config, slip, committed.event_id()).await,
                None => SlipOutcome::NothingToPrint,
            });
            done.vouchers = gift_vouchers::print_after_sale(
                state,
                config,
                committed.event_id(),
                gift_vouchers_issued,
                gift_voucher_redemptions,
            )
            .await;
        }
        PendingContext::Refund {
            open_drawer,
            delivery,
            ..
        } => {
            done.delivery = Some(
                delivery::deliver(
                    state,
                    config,
                    committed.event_id(),
                    &delivery.resolved(),
                    committed.delivery_id,
                    committed.ticket.as_ref(),
                )
                .await,
            );
            kick(with_drawer && *open_drawer).await;
        }
        PendingContext::Copy {
            ticket: Some(CopyTicket::Invoice(print)),
            ..
        } => {
            let mut print = (**print).clone();
            let control = crate::commands::invoice::control_of(committed.result());
            print.copy = Some(pos_print::invoice::InvoiceCopy {
                made_at: control.pos_date_time.clone(),
                control,
            });
            done.invoice_printed =
                Some(crate::commands::invoice::print_invoice(state, config, &print).await);
        }
        PendingContext::Copy { delivery, .. } => {
            done.delivery = Some(
                delivery::deliver(
                    state,
                    config,
                    committed.event_id(),
                    &delivery.resolved(),
                    committed.delivery_id,
                    committed.ticket.as_ref(),
                )
                .await,
            );
        }
        PendingContext::TableOrder { bar_slip } => {
            done.slip = Some(print_slip_of(state, config, bar_slip, committed.event_id()).await);
        }
        PendingContext::TableTransfer {
            order_id,
            from_table,
            to_table,
            operator_insz,
        } => {
            // No "moved" notice when a late commit kept the order put.
            let moved = state.db.lock().is_ok_and(|db| {
                db.query_row(
                    "SELECT table_number = ?2 FROM orders WHERE id = ?1",
                    rusqlite::params![order_id, to_table],
                    |r| r.get::<_, Option<bool>>(0),
                )
                .ok()
                .flatten()
                .unwrap_or(false)
            });
            if moved {
                done.slip = Some(
                    bar_slip::print_transfer_slip(
                        state,
                        config,
                        operator_insz,
                        *from_table,
                        *to_table,
                    )
                    .await,
                );
            }
        }
        PendingContext::PreBill { ticket, .. } => {
            if let Some(addr) = config.printer_address.as_deref() {
                let ticket = FiscalTicket::from_draft(ticket, committed.result(), &state.identity);
                let data = pos_print::receipt::format_receipt(&ticket, config.paper_for(addr));
                let _ = pos_print::transport::send_to_printer_with(
                    addr,
                    &data,
                    state.timers.printer_status_read,
                )
                .await;
            }
        }
        PendingContext::ReportX { print } => {
            crate::commands::report::print_report(
                config,
                (**print).clone(),
                Some(committed.result()),
                state.timers.printer_status_read,
            )
            .await;
        }
        PendingContext::ReportUserX { print } => {
            crate::commands::report::print_user_report(
                config,
                (**print).clone(),
                Some(committed.result()),
                state.timers.printer_status_read,
            )
            .await;
        }
        PendingContext::ReportZ {
            print,
            user_print,
            training,
            ..
        } => {
            // Best effort: a failed backup never undoes the Z.
            if let Ok(db) = state.db.lock()
                && let Err(e) =
                    crate::commands::report::backup_database_impl(&db, config, &state.paths.backups)
            {
                tracing::warn!(%e, "post-Z backup failed (non-fatal)");
            }
            crate::commands::report::print_report(
                config,
                (**print).clone(),
                Some(committed.result()),
                state.timers.printer_status_read,
            )
            .await;
            crate::commands::report::print_user_report(
                config,
                (**user_print).clone(),
                committed.results.get(1),
                state.timers.printer_status_read,
            )
            .await;
            // A real Z feeds the nightly export; a training Z takes no
            // number and exports nothing.
            if !training {
                crate::commands::export::wake_after_z(state);
            }
        }
        PendingContext::Drawer => kick(with_drawer).await,
        #[cfg(feature = "hubrise")]
        PendingContext::WebshopOrder { .. } => {}
        #[cfg(not(feature = "hubrise"))]
        PendingContext::WebshopOrder { .. } => {}
        PendingContext::Work { .. }
        | PendingContext::Money
        | PendingContext::PaymentCorrection { .. } => {}
        PendingContext::Invoice { print, peppol, .. } => {
            let mut print = (**print).clone();
            print.control = Some(crate::commands::invoice::control_of(committed.result()));
            done.invoice_printed =
                Some(crate::commands::invoice::print_invoice(state, config, &print).await);
            if peppol.is_some() {
                state.peppol_wake.notify_one();
            }
        }
    }
    done
}

/// The bar / kitchen slip of the entries event `event_id` registered.
async fn print_slip_of(
    state: &AppState,
    config: &PosConfig,
    slip: &BarSlip,
    event_id: i64,
) -> SlipOutcome {
    let entries = match state.db.lock() {
        Ok(db) => pos_db::queries::orders::get_entries_sent_in(&db, event_id).unwrap_or_else(|e| {
            tracing::warn!(event_id, %e, "slip not printed: sent entries unreadable");
            Vec::new()
        }),
        Err(e) => {
            tracing::warn!(event_id, %e, "slip not printed: db lock poisoned");
            Vec::new()
        }
    };
    // Courses on: the slip is grouped by course and holds the
    // courses above the released one. Off: the slip of v1.0.0.
    if crate::commands::courses::enabled(state)
        && let Some(order_id) = entries.first().map(|e| e.order_id)
    {
        return crate::commands::courses::print_after_sign(
            state,
            config,
            &slip.operator_insz,
            slip.table_number,
            order_id,
        )
        .await;
    }
    bar_slip::print_bar_slip(
        state,
        config,
        &slip.operator_insz,
        slip.table_number,
        &entries,
    )
    .await
}

// ── Recovery ───────────────────────────────────────────────────

/// An event whose answer came on a later send.
pub(crate) struct Recovered {
    pub op: Op,
    pub context: PendingContext,
    pub committed: Committed,
    pub follow_up: FollowUp,
}

/// How a command starts signing.
pub(crate) enum Begin<'a> {
    Fresh(SignGuard<'a>),
    /// The command's own event, sent earlier, has been signed (and
    /// followed up): nothing left to sign.
    AlreadyDone(Box<Recovered>),
}

/// Take the signing lock for `op`, after the open request, if any, is
/// resolved: nothing is signed while an answer is awaited.
pub(crate) async fn begin<'a>(state: &'a AppState, op: &Op) -> Result<Begin<'a>, String> {
    let guard = acquire(state).await?;
    match resolve_open(state, &guard, Some(op)).await? {
        Some(recovered) if recovered.op == *op => Ok(Begin::AlreadyDone(Box::new(recovered))),
        _ => Ok(Begin::Fresh(guard)),
    }
}

/// Send the open request again, if any, and archive its event. `None`
/// when nothing was open or the FDM refused it (its number was never
/// used); an error while the answer is still awaited.
pub(crate) async fn resolve_open(
    state: &AppState,
    guard: &SignGuard<'_>,
    current: Option<&Op>,
) -> Result<Option<Recovered>, String> {
    let row = {
        let db = state.db.lock().map_err(errors::internal)?;
        pending_signs::open(&db).map_err(errors::internal)?
    };
    let Some(row) = row else {
        return Ok(None);
    };
    let config = state.config.lock().map_err(errors::internal)?.clone();
    let stored = Stored::read(&row)?;
    let request = SignRequest::from_stored(
        &row.mutation,
        Mode::from(row.is_training),
        row.request_body.clone(),
    )
    .map_err(errors::internal)?;

    let results = match state.fdm_client(&config).send(&request).await {
        Ok(results) => results,
        Err(e) if e.outcome() == SignOutcome::Refused => {
            {
                let db = state.db.lock().map_err(errors::internal)?;
                pending_signs::mark_unsigned(&db, row.id, Unsigned::Refused, &e.to_string())
                    .map_err(errors::internal)?;
            }
            tracing::warn!(%e, label = %row.event_label, ticket = row.first_ticket_no, "pending request refused: its number is free");
            #[cfg(feature = "hubrise")]
            reconcile_webshop(state, &stored.context);
            return Ok(None);
        }
        Err(e) => {
            let db = state.db.lock().map_err(errors::internal)?;
            pending_signs::record_attempt(&db, row.id, Some(&e.to_string()))
                .map_err(errors::internal)?;
            tracing::warn!(%e, label = %row.event_label, ticket = row.first_ticket_no, "pending request still unanswered");
            return Err(answer_pending(
                &row.event_label,
                row.first_ticket_no,
                e.fdm_code().as_deref(),
                &e,
            ));
        }
    };

    let committed = commit(
        state,
        guard,
        &config,
        row.id,
        stored.label,
        row.first_ticket_no,
        &stored.events,
        &stored.context,
        results,
    )?;
    tracing::warn!(
        label = %row.event_label,
        ticket = row.first_ticket_no,
        kind = %row.kind,
        "pending request answered and archived"
    );
    #[cfg(feature = "hubrise")]
    reconcile_webshop(state, &stored.context);

    let op = Op {
        kind: stored.kind,
        subject: row.subject.clone(),
    };
    let same = current == Some(&op);
    let follow_up = follow_up(
        state,
        &config,
        &stored.context,
        &committed,
        if same { Drawer::Kick } else { Drawer::Leave },
    )
    .await;
    Ok(Some(Recovered {
        op,
        context: stored.context,
        committed,
        follow_up,
    }))
}

/// An online order whose event was archived late: its state follows.
#[cfg(feature = "hubrise")]
fn reconcile_webshop(state: &AppState, context: &PendingContext) {
    let order_id = match context {
        PendingContext::Sale {
            webshop_order_id, ..
        }
        | PendingContext::Refund {
            webshop_order_id, ..
        } => *webshop_order_id,
        PendingContext::WebshopOrder { order_id } => Some(*order_id),
        _ => None,
    };
    if let Some(order_id) = order_id
        && let Ok(db) = state.db.lock()
        && let Err(e) = crate::commands::webshop::reconcile_order(&db, order_id)
    {
        tracing::error!(%e, order_id, "online order state not repaired");
    }
    state.hubrise_outbox_wake.notify_one();
}

/// A stored request, read back.
struct Stored {
    label: EventLabel,
    kind: Kind,
    events: Vec<ArchiveSpec>,
    context: PendingContext,
}

impl Stored {
    fn read(row: &PendingSign) -> Result<Self, String> {
        let label = serde_json::from_value(serde_json::Value::String(row.event_label.clone()))
            .map_err(errors::internal)?;
        let kind = Kind::from_name(&row.kind)
            .ok_or_else(|| errors::internal(format!("unknown kind {}", row.kind)))?;
        Ok(Self {
            label,
            kind,
            events: serde_json::from_str(&row.archive_json).map_err(errors::internal)?,
            context: serde_json::from_str(&row.context_json).map_err(errors::internal)?,
        })
    }
}

#[cfg(test)]
mod work_context_tests {
    use super::PendingContext;

    /// A `pending_signs` row written before migration 28 has no `by`.
    #[test]
    fn a_stored_work_context_without_by_still_parses() {
        for is_in in [true, false] {
            let old = format!(r#"{{"type":"work","insz":"12345678958","is_in":{is_in}}}"#);
            let context: PendingContext = serde_json::from_str(&old).expect("old context");
            match context {
                PendingContext::Work {
                    insz,
                    is_in: parsed,
                    by,
                } => {
                    assert_eq!(insz, "12345678958");
                    assert_eq!(parsed, is_in);
                    assert_eq!(by, None);
                }
                _ => panic!("not a work context"),
            }
        }
    }

    #[test]
    fn the_manager_survives_a_round_trip() {
        let context = PendingContext::Work {
            insz: "12345678958".into(),
            is_in: true,
            by: Some("94060242576".into()),
        };
        let json = serde_json::to_string(&context).unwrap();
        let back: PendingContext = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, PendingContext::Work { by: Some(b), .. } if b == "94060242576"));
    }
}
