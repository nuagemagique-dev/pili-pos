use chrono::Timelike;
use pos_core::money::cents_from_eur;
use pos_db::queries::reports::{self as report_queries, ZReportSummary};
use pos_fiscal::types::Mode;
use pos_print::report::{
    ReportDetails, ReportEventTotal, ReportPaymentTotal, ReportPrintData, ReportVatTotal,
    UserReportPrintData, UserReportUser,
};
use serde::Serialize;

use pos_db::queries::booking::BookingPeriod;
use pos_fiscal::archive;
use pos_fiscal::event::{
    TurnoverReportData, UserReportData, build_report_turnover_x, build_report_turnover_z,
    build_report_user_x, build_report_user_z,
};
use pos_fiscal::fdm_client::{SignMutation, SignRequest};
use pos_fiscal::inputs::{PaymentTotalInput, PosDeviceInput};
use pos_fiscal::report_totals::{self, ReportEvent};
use pos_fiscal::types::{EventLabel, InOut, NegQuantityReason, PaymentLineType};

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::commands::signing::{
    self, ArchiveSpec, Begin, Kind, Op, PendingContext, Prepared, SignGuard,
};
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportResult {
    pub success: bool,
    pub report_type: String,
    pub report_no: Option<i64>,
    pub error: Option<String>,
}

/// Generate an X-report (mid-day, non-resetting).
pub async fn do_generate_x_report(
    state: &AppState,
    employee_insz: String,
) -> Result<ReportResult, String> {
    access::operator_named(state, &employee_insz, Access::Admin)?;
    let op = Op::new(Kind::ReportX, &employee_insz);
    let guard = match signing::begin(state, &op).await {
        Ok(Begin::Fresh(guard)) => guard,
        // Answered late: its follow-up printed it.
        Ok(Begin::AlreadyDone(_)) => return Ok(ReportResult::done("X", None)),
        Err(e) => return Ok(ReportResult::failed("X", None, e)),
    };
    let label = EventLabel::R;
    let (config, period, turnover_data, ticket_number, print_data) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let period = super::booking_period(&db, &guard, &config)?;
        let events = period_events(&db, &period.id)?;
        let data = turnover_data(&config, &events);
        let tn = signing::peek(&db, &guard, label.in_mode(Mode::from(config.training_mode)))?;
        let pd = build_print_data(
            &config,
            &period,
            &events,
            &data,
            "X",
            None,
            tn,
            &employee_insz,
        );
        (config, period, data, tn, pd)
    };

    let report = build_report_turnover_x(
        &config,
        &state.identity,
        ticket_number,
        &employee_insz,
        &as_booking(&period),
        turnover_data,
    );
    let request = SignRequest::new(
        SignMutation::ReportTurnoverX,
        &report,
        Mode::from(config.training_mode),
    )
    .map_err(errors::internal)?;
    let context = PendingContext::ReportX {
        print: Box::new(print_data),
    };
    let prepared = report_prepared(
        op,
        request,
        label.in_mode(Mode::from(config.training_mode)),
        ticket_number,
        &employee_insz,
        vec![serde_json::to_string(&report).map_err(|e| e.to_string())?],
        context.clone(),
    );
    let committed = match signing::sign(state, &guard, &config, prepared).await {
        Ok(committed) => committed,
        Err(failure) => return Ok(ReportResult::failed("X", None, failure.error)),
    };
    drop(guard);

    // Auto-print report on thermal printer
    signing::follow_up(state, &config, &context, &committed, signing::Drawer::Leave).await;
    Ok(ReportResult::done("X", None))
}

impl ReportResult {
    fn done(report_type: &str, report_no: Option<i64>) -> Self {
        Self {
            success: true,
            report_type: report_type.to_string(),
            report_no,
            error: None,
        }
    }

    fn failed(report_type: &str, report_no: Option<i64>, error: String) -> Self {
        Self {
            success: false,
            report_type: report_type.to_string(),
            report_no,
            error: Some(error),
        }
    }
}

/// A report's request: one R event per input, numbered from
/// `ticket_number`.
fn report_prepared(
    op: Op,
    request: SignRequest,
    label: EventLabel,
    ticket_number: i64,
    employee_insz: &str,
    request_jsons: Vec<String>,
    context: PendingContext,
) -> Prepared {
    Prepared {
        op,
        request,
        label,
        first_ticket_no: ticket_number,
        events: request_jsons
            .into_iter()
            .map(|request_json| ArchiveSpec {
                order_id: None,
                user_insz: employee_insz.to_string(),
                request_json,
                sent_entries: None,
            })
            .collect(),
        order_id: None,
        user_insz: employee_insz.to_string(),
        context,
    }
}

/// Returns true if no Z-report exists for today AND the local time
/// is past 22:00 - used by the Vue header to show a warning banner
/// reminding the manager to run the end-of-day Z before leaving.
pub fn do_needs_z_report(state: &AppState) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let open = pos_db::queries::booking::open_period(&db).map_err(|e| e.to_string())?;
    let now = pos_fiscal::event::now_in_belgium();
    Ok(z_reminder_due(open.as_ref(), now))
}

/// The Z reminder shows while a booking period is open (something was
/// recorded since the last Z) and it is either past 22:00 in Belgium or
/// the period started on an earlier day.
fn z_reminder_due(open: Option<&BookingPeriod>, now: chrono::DateTime<chrono_tz::Tz>) -> bool {
    let Some(period) = open else {
        return false;
    };
    let today = now.date_naive().to_string();
    now.hour() >= 22 || period.booking_date < today
}

/// List past Z-reports for the history view (newest first).
pub fn do_list_z_reports(state: &AppState) -> Result<Vec<ZReportSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::reports::list_z_reports(&db).map_err(|e| e.to_string())
}

/// Generate the end-of-day Z (M184): the financial Z and the user Z
/// are built together, signed by the FDM in ONE GraphQL operation,
/// and archived atomically (both R events or none).
pub async fn do_generate_z_report(
    state: &AppState,
    employee_insz: String,
) -> Result<ReportResult, String> {
    access::operator_named(state, &employee_insz, Access::Admin)?;
    let op = Op::new(Kind::ReportZ, "z");
    let guard = match signing::begin(state, &op).await {
        Ok(Begin::Fresh(guard)) => guard,
        // A Z sent earlier was signed: the day is closed and printed.
        Ok(Begin::AlreadyDone(recovered)) => {
            let report_no = match &recovered.context {
                PendingContext::ReportZ {
                    report_no,
                    training,
                    ..
                } => (!training).then_some(*report_no),
                _ => None,
            };
            return Ok(ReportResult::done("Z", report_no));
        }
        Err(e) => return Ok(ReportResult::failed("Z", None, e)),
    };
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let signed = z_report_locked(state, &guard, &config, employee_insz, op).await?;
    drop(guard);
    Ok(after_z(state, &config, signed).await)
}

/// Close a business day left without its Z (producers FAQ Q501c): when
/// the open booking period belongs to a day that is over (see
/// `PosConfig::day_end_hour`), make its Z in the name of the system
/// identity. `Ok(true)` when a Z was made. Run at start-up and every
/// minute; new events wait for it (`commands::booking`). Nothing to do
/// in training mode, whose Z closes no period.
///
/// A sign request still awaiting its answer is sent again first (also
/// at start-up). While another event is being signed, the tick is
/// skipped: the next one checks again, inside the lock, so a Z made by
/// hand in between is never followed by an empty one.
///
/// A day without any event has no open period, so it gets no Z: only a
/// day with recorded activity is closed.
pub async fn close_overdue_day(state: &AppState) -> Result<bool, String> {
    let Some(guard) = signing::try_acquire(state) else {
        return Ok(false);
    };
    signing::resolve_open(state, &guard, None).await?;
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let overdue = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let open = pos_db::queries::booking::open_period(&db).map_err(|e| e.to_string())?;
        !config.training_mode
            && open.is_some_and(|period| {
                super::period_overdue(
                    &period,
                    pos_fiscal::event::now_in_belgium(),
                    config.day_end_hour,
                )
            })
    };
    if !overdue {
        return Ok(false);
    }
    let signed = z_report_locked(
        state,
        &guard,
        &config,
        pos_core::user::SYSTEM_INSZ.to_string(),
        Op::new(Kind::ReportZ, "z"),
    )
    .await?;
    drop(guard);
    let result = after_z(state, &config, signed).await;
    if result.success {
        tracing::info!(report_no = ?result.report_no, "automatic Z of a day left open");
        Ok(true)
    } else {
        Err(result.error.unwrap_or_else(|| "automatic Z failed".into()))
    }
}

type ZSigned = Result<(PendingContext, signing::Committed, Option<i64>), ReportResult>;

/// The Z's backup and printouts, once signed.
async fn after_z(
    state: &AppState,
    config: &pos_core::config::PosConfig,
    signed: ZSigned,
) -> ReportResult {
    match signed {
        Ok((context, committed, report_no)) => {
            signing::follow_up(state, config, &context, &committed, signing::Drawer::Leave).await;
            ReportResult::done("Z", report_no)
        }
        Err(refused) => refused,
    }
}

/// The end-of-day Z (M184) in the name of `employee_insz`, who is
/// allowed to make it: the financial Z and the user Z, built together,
/// signed in ONE GraphQL operation and archived atomically with the
/// period's closing (both R events or none).
async fn z_report_locked(
    state: &AppState,
    guard: &SignGuard<'_>,
    config: &pos_core::config::PosConfig,
    employee_insz: String,
    op: Op,
) -> Result<ZSigned, String> {
    let label = EventLabel::R.in_mode(Mode::from(config.training_mode));
    let (
        period,
        turnover_data,
        user_data,
        turnover_ticket,
        user_ticket,
        report_no,
        print_data,
        user_print_data,
    ) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let period = super::booking_period(&db, guard, config)?;
        let events = period_events(&db, &period.id)?;
        let data = turnover_data(config, &events);
        let users = user_data(config, &events);
        // Two sequential ticket numbers - the M184 fixture shows the
        // user Z carrying posFiscalTicketNo = turnover Z + 1.
        let tn = signing::peek(&db, guard, label)?;
        let un = tn + 1;
        let rn = report_queries::next_z_report_number(&db).map_err(|e| e.to_string())?;
        let pd = build_print_data(
            config,
            &period,
            &events,
            &data,
            "Z",
            Some(rn),
            tn,
            &employee_insz,
        );
        let upd = build_user_print_data(
            config,
            &period,
            &events,
            &users,
            "Z",
            Some(rn),
            un,
            &employee_insz,
        );
        (period, data, users, tn, un, rn, pd, upd)
    };

    let turnover_report = build_report_turnover_z(
        config,
        &state.identity,
        turnover_ticket,
        &employee_insz,
        &as_booking(&period),
        report_no,
        &period.booking_date,
        turnover_data,
    );
    let user_report = build_report_user_z(
        config,
        &state.identity,
        user_ticket,
        &employee_insz,
        &as_booking(&period),
        report_no,
        &period.booking_date,
        user_data,
    );
    // M184 - one GraphQL operation signs both reports.
    let request = SignRequest::turnover_z_user_z(
        &turnover_report,
        &user_report,
        Mode::from(config.training_mode),
    )
    .map_err(errors::internal)?;
    let context = PendingContext::ReportZ {
        period_id: period.id.clone(),
        booking_date: period.booking_date.clone(),
        report_no,
        training: config.training_mode,
        print: Box::new(print_data),
        user_print: Box::new(user_print_data),
    };
    let prepared = report_prepared(
        op,
        request,
        label,
        turnover_ticket,
        &employee_insz,
        vec![
            serde_json::to_string(&turnover_report).map_err(|e| e.to_string())?,
            serde_json::to_string(&user_report).map_err(|e| e.to_string())?,
        ],
        context.clone(),
    );
    debug_assert_eq!(user_ticket, turnover_ticket + 1);
    // A training Z takes no Z number: the next real Z carries the
    // same `reportNo` (art. 27 continuity is that of real Zs).
    let shown_no = (!config.training_mode).then_some(report_no);
    Ok(match signing::sign(state, guard, config, prepared).await {
        Ok(committed) => Ok((context, committed, shown_no)),
        Err(failure) => Err(ReportResult::failed("Z", Some(report_no), failure.error)),
    })
}

/// Generate a user X-report (M182, non-resetting) on demand from the
/// Reports menu.
pub async fn do_generate_user_x_report(
    state: &AppState,
    employee_insz: String,
) -> Result<ReportResult, String> {
    access::operator_named(state, &employee_insz, Access::Admin)?;
    let op = Op::new(Kind::ReportUserX, &employee_insz);
    let guard = match signing::begin(state, &op).await {
        Ok(Begin::Fresh(guard)) => guard,
        // Answered late: its follow-up printed it.
        Ok(Begin::AlreadyDone(_)) => return Ok(ReportResult::done("X-USER", None)),
        Err(e) => return Ok(ReportResult::failed("X-USER", None, e)),
    };
    let (config, period, user_data, ticket_number, print_data) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let period = super::booking_period(&db, &guard, &config)?;
        let events = period_events(&db, &period.id)?;
        let users = user_data(&config, &events);
        let tn = signing::peek(
            &db,
            &guard,
            EventLabel::R.in_mode(Mode::from(config.training_mode)),
        )?;
        let pd = build_user_print_data(
            &config,
            &period,
            &events,
            &users,
            "X",
            None,
            tn,
            &employee_insz,
        );
        (config, period, users, tn, pd)
    };

    let report = build_report_user_x(
        &config,
        &state.identity,
        ticket_number,
        &employee_insz,
        &as_booking(&period),
        user_data,
    );
    let request = SignRequest::new(
        SignMutation::ReportUserX,
        &report,
        Mode::from(config.training_mode),
    )
    .map_err(errors::internal)?;
    let context = PendingContext::ReportUserX {
        print: Box::new(print_data),
    };
    let prepared = report_prepared(
        op,
        request,
        EventLabel::R.in_mode(Mode::from(config.training_mode)),
        ticket_number,
        &employee_insz,
        vec![serde_json::to_string(&report).map_err(|e| e.to_string())?],
        context.clone(),
    );
    let committed = match signing::sign(state, &guard, &config, prepared).await {
        Ok(committed) => committed,
        Err(failure) => return Ok(ReportResult::failed("X-USER", None, failure.error)),
    };
    drop(guard);

    signing::follow_up(state, &config, &context, &committed, signing::Drawer::Leave).await;
    Ok(ReportResult::done("X-USER", None))
}

/// Create a timestamped backup of the live database.
pub fn do_backup_database(state: &AppState) -> Result<String, String> {
    access::operator(state, Access::Admin)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let config = state.config.lock().map_err(|e| e.to_string())?;
    backup_database_impl(&db, &config, &state.paths.backups)
}

/// How many engine backups stay in the backup directory (user decision of
/// 2026-10-10, "Keep the 7 newest"). The live `pos.db` keeps the whole
/// fiscal journal, so an older copy carries nothing the database lost.
pub const BACKUPS_KEPT: usize = 7;

/// Copy the live database into the backup directory, check the copy, then
/// keep only the [`BACKUPS_KEPT`] newest engine backups.
///
/// Retention runs only once the new copy is complete and passes SQLite's
/// `quick_check`; a failed copy deletes nothing. It touches only the
/// engine's own `pos-<UTC timestamp>.db` files of that directory, never
/// `pos.db`, the nightly export folder or a file someone else put there
/// (`manual-*.db`, `pre-update-*.db`).
pub(crate) fn backup_database_impl(
    conn: &rusqlite::Connection,
    config: &pos_core::config::PosConfig,
    default_backups: &std::path::Path,
) -> Result<String, String> {
    let backup_dir = match &config.backup_dir {
        Some(dir) => std::path::PathBuf::from(dir),
        None => default_backups.to_path_buf(),
    };
    std::fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let dest_path = fresh_backup_path(&backup_dir, chrono::Utc::now());
    {
        let mut dest = rusqlite::Connection::open(&dest_path).map_err(|e| e.to_string())?;
        let backup = rusqlite::backup::Backup::new(conn, &mut dest).map_err(|e| e.to_string())?;
        backup
            .run_to_completion(100, std::time::Duration::from_millis(50), None)
            .map_err(|e| e.to_string())?;
    }
    verify_backup(&dest_path)?;

    let path_str = dest_path.to_string_lossy().to_string();
    tracing::info!(path = %path_str, "database backup created");
    prune_backups(&backup_dir, &dest_path, BACKUPS_KEPT);
    Ok(path_str)
}

/// `pos-YYYYMMDD-HHMMSS-mmm.db` at `now` (UTC), one millisecond later as
/// long as that name is taken, so a backup never overwrites another one and
/// the names sort in the order the backups were made.
fn fresh_backup_path(
    dir: &std::path::Path,
    now: chrono::DateTime<chrono::Utc>,
) -> std::path::PathBuf {
    let mut at = now;
    loop {
        let path = dir.join(format!("pos-{}.db", at.format("%Y%m%d-%H%M%S-%3f")));
        if !path.exists() {
            return path;
        }
        at += chrono::Duration::milliseconds(1);
    }
}

/// The copy opens and SQLite's `quick_check` finds it sound.
fn verify_backup(path: &std::path::Path) -> Result<(), String> {
    let copy = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("backup check: {e}"))?;
    let verdict: String = copy
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(|e| format!("backup check: {e}"))?;
    if verdict == "ok" {
        Ok(())
    } else {
        Err(format!("backup check: {verdict}"))
    }
}

/// An engine backup name: `pos-` then the UTC timestamp (digits and dashes,
/// with or without milliseconds), then `.db`. `pos.db` is not one.
fn is_engine_backup(name: &str) -> bool {
    name.strip_prefix("pos-")
        .and_then(|rest| rest.strip_suffix(".db"))
        .is_some_and(|stamp| {
            stamp.len() >= 15
                && stamp.starts_with(|c: char| c.is_ascii_digit())
                && stamp.chars().all(|c| c.is_ascii_digit() || c == '-')
        })
}

/// Delete the oldest engine backups of `dir` so that `keep` remain, `fresh`
/// (the backup just made and checked) always among them. Each deletion is
/// logged; a deletion that fails is logged and the rest go on.
fn prune_backups(dir: &std::path::Path, fresh: &std::path::Path, keep: usize) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::warn!(%e, dir = %dir.display(), "backup retention: cannot list the backups");
            return;
        }
    };
    let fresh_name = fresh.file_name();
    let mut older: Vec<std::path::PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.path())
        .filter(|p| {
            p.file_name() != fresh_name
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(is_engine_backup)
        })
        .collect();
    // Newest first: the names are UTC timestamps.
    older.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    for old in older.iter().skip(keep.saturating_sub(1)) {
        match std::fs::remove_file(old) {
            Ok(()) => tracing::info!(
                path = %old.display(),
                kept = keep,
                "backup retention: oldest backup deleted"
            ),
            Err(e) => tracing::warn!(
                %e,
                path = %old.display(),
                "backup retention: could not delete an old backup"
            ),
        }
    }
}

/// Build the printed turnover report from the totals sent to the FDM,
/// so the paper always shows what was signed.
#[expect(
    clippy::too_many_arguments,
    reason = "each argument is a distinct input of one fiscal step; a parameter struct would only rename them"
)]
fn build_print_data(
    config: &pos_core::config::PosConfig,
    period: &BookingPeriod,
    events: &[ReportEvent],
    data: &TurnoverReportData,
    report_type: &str,
    report_no: Option<i64>,
    ticket_number: i64,
    employee_insz: &str,
) -> ReportPrintData {
    let turnover = &data.turnover;
    let payment_totals = print_payments(&turnover.payments, events.iter());
    let (first_datetime, last_datetime) = period_range(&data.pos_devices);
    ReportPrintData {
        report_type: report_type.to_string(),
        report_no,
        restaurant_name: config.restaurant_name.clone(),
        restaurant_address: config.restaurant_address.clone(),
        restaurant_vat: config.restaurant_vat.clone(),
        pos_id: config.pos_id.clone(),
        terminal_id: config.terminal_id.clone(),
        report_date: period.booking_date.clone(),
        first_datetime,
        last_datetime,
        employee_insz: employee_insz.to_string(),
        is_training: config.training_mode,
        event_totals: turnover
            .transactions
            .iter()
            .map(|t| ReportEventTotal {
                label: enum_name(&t.event_label),
                count: t.ticket_count,
                amount_cents: cents_from_eur(t.amount),
            })
            .collect(),
        vat_totals: turnover
            .vats
            .iter()
            .map(|v| ReportVatTotal {
                code: enum_name(&v.label),
                rate: v.rate,
                total_incl_cents: cents_from_eur(v.total_amount),
                vat_cents: cents_from_eur(v.vat_amount),
            })
            .collect(),
        grand_total_cents: payment_totals.iter().map(|p| p.amount_cents).sum(),
        payment_totals,
        drawer_opens: turnover.drawers_open_count,
        fdm_id: data.fdm_devices.first().map(|f| f.fdm_id.clone()),
        fdm_datetime: None,
        short_signature: None,
        ticket_number,
        lang: pos_fiscal::ticket::ReceiptLanguage::from_code(&config.language),
        details: report_details(config, period, data, events),
    }
}

/// The art. 29 content of the printed report beyond its totals.
fn report_details(
    config: &pos_core::config::PosConfig,
    period: &BookingPeriod,
    data: &TurnoverReportData,
    events: &[ReportEvent],
) -> ReportDetails {
    let turnover = &data.turnover;
    let neg = |refund: bool| {
        turnover
            .neg_quantities
            .iter()
            .filter(|n| (n.neg_quantity_reason == NegQuantityReason::Refund) == refund)
            .fold((0, 0), |(count, amount), n| {
                (count + n.ticket_count, amount + cents_from_eur(n.amount))
            })
    };
    let transactions_of = |label: EventLabel| {
        turnover
            .transactions
            .iter()
            .filter(|t| t.event_label == label)
            .fold((0, 0), |(count, amount), t| {
                (count + t.ticket_count, amount + cents_from_eur(t.amount))
            })
    };
    // The public price changes of the sales, split by scope as the
    // printout asks: a change on one line (a free line) or on the whole
    // ticket (an order discount, an online order's). The signed totals
    // (`turnover.priceChanges`) are not split.
    let (line_reductions, event_reductions) = reductions_by_scope(events);
    ReportDetails {
        restaurant_est_no: config.restaurant_est_no.clone(),
        booking_period_id: period.id.clone(),
        created_at: pos_fiscal::event::now_in_belgium()
            .format("%d/%m/%y %H:%M:%S")
            .to_string(),
        fdm_first_event: data
            .fdm_devices
            .first()
            .map(|f| (f.first_total_counter, f.first_fdm_date_time.clone())),
        fdm_last_event: data
            .fdm_devices
            .first()
            .map(|f| (f.last_total_counter, f.last_fdm_date_time.clone())),
        departments: turnover
            .departments
            .iter()
            .map(|d| (d.department_name.clone(), cents_from_eur(d.amount)))
            .collect(),
        corrections_cents: neg(false).1,
        refunds: neg(true),
        training: transactions_of(EventLabel::T),
        line_reductions,
        event_reductions,
        invoices: turnover
            .invoices
            .iter()
            .flatten()
            .map(|i| (i.invoice_no.clone(), cents_from_eur(i.amount)))
            .collect(),
        rounding_cents: turnover
            .payments
            .iter()
            .flat_map(|p| &p.amount)
            .filter(|a| a.amount_type == PaymentLineType::Rounding)
            .map(|a| cents_from_eur(a.total_amount))
            .sum(),
        payment_corrections: payment_corrections(&turnover.payments),
    }
}

/// The reductions of the sales' lines, per scope: `(tickets, cents)` for
/// `LINE` scope changes, then for `EVENT` scope ones. A ticket counts once
/// per scope it has a public change of; the amount is their sum (negative
/// for a discount).
fn reductions_by_scope(events: &[ReportEvent]) -> ((i64, i64), (i64, i64)) {
    let mut line = (0, 0);
    let mut event = (0, 0);
    for sale in events.iter().filter(|e| e.label == EventLabel::N) {
        let (mut line_hit, mut event_hit) = (false, false);
        for l in sale.lines() {
            for vat in l["mainProduct"]["vats"].as_array().into_iter().flatten() {
                for change in vat["priceChanges"].as_array().into_iter().flatten() {
                    if change["type"].as_str() != Some("PUBLIC") {
                        continue;
                    }
                    let amount = pos_fiscal::report_totals::cents(&change["amount"]);
                    if change["scope"].as_str() == Some("LINE") {
                        line.1 += amount;
                        line_hit = true;
                    } else {
                        event.1 += amount;
                        event_hit = true;
                    }
                }
            }
        }
        line.0 += i64::from(line_hit);
        event.0 += i64::from(event_hit);
    }
    (line, event)
}

/// Payment corrections (event F) of the report: their number (each
/// one is a negative and a positive line) and the amount they moved to
/// another method.
fn payment_corrections(payments: &[PaymentTotalInput]) -> (i64, i64) {
    let (lines, moved) = payments
        .iter()
        .flat_map(|p| &p.amount)
        .filter(|a| a.amount_type == PaymentLineType::Payment)
        .fold((0, 0), |(lines, moved), a| {
            (
                lines + a.corrections_count,
                moved + cents_from_eur(a.positive_corrections),
            )
        });
    // Each correction is a negative and a positive line, so the count is even.
    if lines % 2 != 0 {
        tracing::warn!(lines, "payment correction lines should come in pairs");
    }
    (lines / 2, moved)
}

/// Printed payment lines: the signed totals, with the number of
/// payments made with each method.
fn print_payments<'a>(
    payments: &[PaymentTotalInput],
    events: impl Iterator<Item = &'a ReportEvent> + Clone,
) -> Vec<ReportPaymentTotal> {
    // One pass over the sales: the number of PAYMENT lines per (id, name).
    // A cash sale's rounding line is not a second payment.
    let mut counts: std::collections::BTreeMap<(&str, &str), i64> =
        std::collections::BTreeMap::new();
    for line in events
        .filter(|e| e.label == EventLabel::N)
        .filter_map(|e| e.request["financials"].as_array())
        .flatten()
        .filter(|f| f["amountType"] == "PAYMENT")
    {
        if let (Some(id), Some(name)) = (line["id"].as_str(), line["name"].as_str()) {
            *counts.entry((id, name)).or_insert(0) += 1;
        }
    }
    payments
        .iter()
        .map(|p| ReportPaymentTotal {
            method: p.name.clone(),
            count: counts
                .get(&(p.id.as_str(), p.name.as_str()))
                .copied()
                .unwrap_or(0),
            // The payment lines; the rounding is totalled apart.
            amount_cents: p
                .amount
                .iter()
                .filter(|a| a.amount_type == PaymentLineType::Payment)
                .map(|a| cents_from_eur(a.total_amount))
                .sum(),
        })
        .collect()
}

/// First and last POS time of the report's events.
fn period_range(devices: &[PosDeviceInput]) -> (String, String) {
    let parse = |s: &str| chrono::DateTime::parse_from_rfc3339(s).ok();
    // Only parseable timestamps compete: an unparseable one sorts first
    // as `None` and would be printed as the "first POS time".
    let first = devices
        .iter()
        .map(|d| &d.first_pos_date_time)
        .filter(|s| parse(s).is_some())
        .min_by_key(|s| parse(s));
    let last = devices
        .iter()
        .map(|d| &d.last_pos_date_time)
        .filter(|s| parse(s).is_some())
        .max_by_key(|s| parse(s));
    (
        first.cloned().unwrap_or_default(),
        last.cloned().unwrap_or_default(),
    )
}

/// The wire name of a fiscal enum (`EventLabel::N` → "N").
fn enum_name<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Send the formatted report to the thermal printer (best-effort).
pub(crate) async fn print_report(
    config: &pos_core::config::PosConfig,
    mut data: ReportPrintData,
    sign_result: Option<&pos_fiscal::response::SignResult>,
    status_timeout: std::time::Duration,
) {
    // Enrich with FDM signature data if available
    if let Some(sr) = sign_result {
        data.fdm_id = Some(sr.fdm_ref.fdm_id.clone());
        data.fdm_datetime = Some(sr.fdm_ref.fdm_date_time.clone());
        data.short_signature = sr.short_signature.clone();
    }

    if let Some(ref addr) = config.printer_address {
        let report_bytes = pos_print::report::format_report(&data, config.paper_for(addr));
        match pos_print::transport::send_to_printer_with(addr, &report_bytes, status_timeout).await
        {
            Ok(status) => {
                if !status.paper_ok {
                    tracing::warn!(?status.warnings, "printer: paper issue after report print");
                }
            }
            Err(e) => {
                tracing::warn!(%e, "report print failed (non-fatal)");
            }
        }
    }
}

/// The events of the booking period, as signed by the FDM: the input of
/// every report (see `pos_fiscal::report_totals`). A sale carries the
/// negative lines its order registrations already transmitted, even in
/// an earlier period, so they are counted once (FAQ Q303).
fn period_events(
    db: &rusqlite::Connection,
    booking_period_id: &str,
) -> Result<Vec<ReportEvent>, String> {
    let unreadable = |id: i64| format!("archived fiscal event {id} is unreadable");
    archive::events_in_booking_period(db, booking_period_id)
        .map_err(|e| e.to_string())?
        .iter()
        .map(|e| {
            let mut event = ReportEvent::from_archive(e).ok_or_else(|| unreadable(e.id))?;
            if e.event_type == EventLabel::I {
                // An invoice's amount is the sum of the sales it names.
                let mut amount = 0.0;
                for fdm_ref in event.request["fdmRefs"].as_array().into_iter().flatten() {
                    amount += archive::transaction_total_by_fdm_ref(db, fdm_ref)
                        .map_err(|e| e.to_string())?
                        .unwrap_or(0.0);
                }
                event.invoice_amount = Some((amount * 100.0).round() / 100.0);
            }
            if let (EventLabel::N, Some(order_id)) = (e.event_type, e.order_id) {
                for earlier in
                    archive::order_events_before(db, order_id, e.id).map_err(|e| e.to_string())?
                {
                    let earlier = ReportEvent::from_archive(&earlier)
                        .ok_or_else(|| unreadable(earlier.id))?;
                    if earlier.is_order_registration() {
                        event.registered_negatives.extend(earlier.negative_lines());
                    }
                }
            }
            Ok(event)
        })
        .collect()
}

fn as_booking(period: &BookingPeriod) -> pos_fiscal::event::Booking {
    pos_fiscal::event::Booking {
        period_id: period.id.clone(),
        date: period.booking_date.clone(),
    }
}

/// This terminal, for a report whose period has no event yet.
fn this_terminal(config: &pos_core::config::PosConfig) -> Vec<PosDeviceInput> {
    let now = pos_fiscal::event::fdm_now();
    vec![PosDeviceInput {
        pos_id: config.pos_id.clone(),
        terminal_id: Some(config.terminal_id.clone()),
        first_pos_date_time: now.clone(),
        last_pos_date_time: now,
    }]
}

fn pos_devices(
    config: &pos_core::config::PosConfig,
    events: &[ReportEvent],
) -> Vec<PosDeviceInput> {
    let devices = report_totals::pos_devices(events);
    if devices.is_empty() {
        this_terminal(config)
    } else {
        devices
    }
}

fn turnover_data(
    config: &pos_core::config::PosConfig,
    events: &[ReportEvent],
) -> TurnoverReportData {
    TurnoverReportData {
        pos_devices: pos_devices(config, events),
        fdm_devices: report_totals::fdm_devices(events),
        turnover: report_totals::turnover(events),
    }
}

fn user_data(config: &pos_core::config::PosConfig, events: &[ReportEvent]) -> UserReportData {
    UserReportData {
        pos_devices: pos_devices(config, events),
        fdm_devices: report_totals::fdm_devices(events),
        users: report_totals::users(events),
    }
}

/// Build the printed user report from the totals sent to the FDM.
#[expect(
    clippy::too_many_arguments,
    reason = "each argument is a distinct input of one fiscal step; a parameter struct would only rename them"
)]
fn build_user_print_data(
    config: &pos_core::config::PosConfig,
    period: &BookingPeriod,
    events: &[ReportEvent],
    data: &UserReportData,
    report_type: &str,
    report_no: Option<i64>,
    ticket_number: i64,
    employee_insz: &str,
) -> UserReportPrintData {
    let users = data
        .users
        .iter()
        .map(|u| UserReportUser {
            insz: u.employee_id.clone(),
            first_datetime: u.first_pos_date_time.clone(),
            last_datetime: u.last_pos_date_time.clone(),
            total_cents: cents_from_eur(u.total_amount),
            payments: print_payments(
                &u.payments,
                events
                    .iter()
                    .filter(|e| e.request["employeeId"].as_str() == Some(&u.employee_id)),
            ),
            social_events: u
                .social_events
                .iter()
                .map(|s| {
                    let operation = match s.in_out {
                        InOut::In => "WORK_IN",
                        InOut::Out => "WORK_OUT",
                    };
                    (operation.to_string(), s.pos_date_time.clone())
                })
                .collect(),
        })
        .collect();

    UserReportPrintData {
        report_type: report_type.to_string(),
        report_no,
        restaurant_name: config.restaurant_name.clone(),
        restaurant_address: config.restaurant_address.clone(),
        restaurant_vat: config.restaurant_vat.clone(),
        terminal_id: config.terminal_id.clone(),
        report_date: period.booking_date.clone(),
        employee_insz: employee_insz.to_string(),
        is_training: config.training_mode,
        users,
        fdm_id: None,
        fdm_datetime: None,
        short_signature: None,
        ticket_number,
        lang: pos_fiscal::ticket::ReceiptLanguage::from_code(&config.language),
    }
}

/// Send the formatted user report to the thermal printer (best-effort).
pub(crate) async fn print_user_report(
    config: &pos_core::config::PosConfig,
    mut data: UserReportPrintData,
    sign_result: Option<&pos_fiscal::response::SignResult>,
    status_timeout: std::time::Duration,
) {
    if let Some(sr) = sign_result {
        data.fdm_id = Some(sr.fdm_ref.fdm_id.clone());
        data.fdm_datetime = Some(sr.fdm_ref.fdm_date_time.clone());
        data.short_signature = sr.short_signature.clone();
    }

    if let Some(ref addr) = config.printer_address {
        let bytes = pos_print::report::format_user_report(&data, config.paper_for(addr));
        match pos_print::transport::send_to_printer_with(addr, &bytes, status_timeout).await {
            Ok(status) => {
                if !status.paper_ok {
                    tracing::warn!(?status.warnings, "printer: paper issue after user report");
                }
            }
            Err(e) => {
                tracing::warn!(%e, "user report print failed (non-fatal)");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use chrono_tz::Europe::Brussels;

    fn period(date: &str) -> BookingPeriod {
        BookingPeriod {
            id: "bp".into(),
            booking_date: date.into(),
        }
    }

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> chrono::DateTime<chrono_tz::Tz> {
        Brussels.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    #[test]
    fn the_period_range_ignores_unparseable_timestamps() {
        let device = |first: &str, last: &str| PosDeviceInput {
            pos_id: "pos".into(),
            terminal_id: None,
            first_pos_date_time: first.into(),
            last_pos_date_time: last.into(),
        };
        let devices = [
            device("garbage", "2026-07-03T10:00:00+02:00"),
            device("2026-07-03T09:00:00+02:00", "garbage"),
        ];
        assert_eq!(
            period_range(&devices),
            (
                "2026-07-03T09:00:00+02:00".to_string(),
                "2026-07-03T10:00:00+02:00".to_string()
            )
        );
    }

    #[test]
    fn faq_q501_a_service_past_midnight_stays_on_its_day() {
        // Day ends at 05:00: Friday's period is still Friday at 01:30.
        let p = period("2026-07-03");
        assert!(!crate::commands::period_overdue(
            &p,
            at(2026, 7, 4, 1, 30),
            5
        ));
        assert!(!crate::commands::period_overdue(
            &p,
            at(2026, 7, 4, 4, 59),
            5
        ));
        assert!(crate::commands::period_overdue(&p, at(2026, 7, 4, 5, 0), 5));
    }

    #[test]
    fn faq_q501_the_day_end_hour_is_configurable() {
        let p = period("2026-07-03");
        assert!(crate::commands::period_overdue(&p, at(2026, 7, 4, 0, 0), 0));
        assert!(!crate::commands::period_overdue(
            &p,
            at(2026, 7, 4, 11, 59),
            12
        ));
        assert!(crate::commands::period_overdue(
            &p,
            at(2026, 7, 4, 12, 0),
            12
        ));
    }

    #[test]
    fn faq_q501_a_new_period_takes_the_business_day() {
        assert_eq!(
            crate::commands::business_date(at(2026, 7, 4, 1, 30), 5),
            "2026-07-03"
        );
        assert_eq!(
            crate::commands::business_date(at(2026, 7, 4, 5, 0), 5),
            "2026-07-04"
        );
    }

    fn backup_names(dir: &std::path::Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn engine_backup_names_are_recognised_and_nothing_else() {
        assert!(is_engine_backup("pos-20261010-120000.db"));
        assert!(is_engine_backup("pos-20261010-120000-042.db"));
        for other in [
            "pos.db",
            "pos.db-wal",
            "pos-.db",
            "pos-latest.db",
            "pos-20261010-120000.db-journal",
            "pos-20261008-z000042.db.age",
            "manual-20261010-120000.db",
            "pre-update-20261010-120000.db",
        ] {
            assert!(!is_engine_backup(other), "{other}");
        }
    }

    #[test]
    fn a_backup_never_takes_the_name_of_another() {
        let dir = tempfile::tempdir().unwrap();
        let now = chrono::Utc
            .with_ymd_and_hms(2026, 10, 10, 12, 0, 0)
            .unwrap();
        let first = fresh_backup_path(dir.path(), now);
        std::fs::write(&first, "x").unwrap();
        let second = fresh_backup_path(dir.path(), now);
        assert_ne!(first, second);
        assert!(second.file_name() > first.file_name(), "sorts after");
        assert_eq!(
            first.file_name().unwrap(),
            "pos-20261010-120000-000.db",
            "UTC timestamp with milliseconds"
        );
    }

    #[test]
    fn nine_backups_leave_the_seven_newest_and_nothing_else_is_touched() {
        let data = tempfile::tempdir().unwrap();
        let dir = data.path().join("backups");
        std::fs::create_dir_all(&dir).unwrap();
        // What retention must never touch.
        let foreign = [
            "manual-20250101-000000.db",
            "pre-update-20250101-000000.db",
            "notes.txt",
        ];
        for name in foreign {
            std::fs::write(dir.join(name), "keep").unwrap();
        }
        std::fs::create_dir_all(dir.join("pos-20000101-000000.db")).unwrap();
        let live = rusqlite::Connection::open(data.path().join("pos.db")).unwrap();
        live.execute_batch("CREATE TABLE t (x INTEGER); INSERT INTO t VALUES (1);")
            .unwrap();
        let config = pos_core::config::PosConfig {
            backup_dir: None,
            ..Default::default()
        };

        let made: Vec<String> = (0..9)
            .map(|_| {
                let path = backup_database_impl(&live, &config, &dir).unwrap();
                std::path::Path::new(&path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        let mut expected: Vec<String> = made[made.len() - BACKUPS_KEPT..].to_vec();
        expected.extend(foreign.iter().map(ToString::to_string));
        expected.push("pos-20000101-000000.db".into());
        expected.sort();
        assert_eq!(backup_names(&dir), expected);
        assert!(data.path().join("pos.db").is_file(), "pos.db untouched");
        // Every kept copy is a sound database.
        for name in &made[made.len() - BACKUPS_KEPT..] {
            verify_backup(&dir.join(name)).unwrap();
        }
    }

    #[test]
    fn the_fresh_backup_is_kept_even_when_an_older_name_sorts_after_it() {
        let data = tempfile::tempdir().unwrap();
        let dir = data.path().join("backups");
        std::fs::create_dir_all(&dir).unwrap();
        // Seven backups "from the future" (the clock went back since).
        for i in 0..7 {
            std::fs::write(dir.join(format!("pos-2099010{i}-000000.db")), "x").unwrap();
        }
        let live = rusqlite::Connection::open_in_memory().unwrap();
        let config = pos_core::config::PosConfig::default();
        let path = backup_database_impl(&live, &config, &dir).unwrap();
        assert!(
            std::path::Path::new(&path).is_file(),
            "the fresh copy stays"
        );
        let names = backup_names(&dir);
        assert_eq!(names.len(), BACKUPS_KEPT);
        assert!(
            !names.contains(&"pos-20990100-000000.db".to_string()),
            "oldest name went"
        );
    }

    #[test]
    fn a_failed_backup_deletes_nothing() {
        let data = tempfile::tempdir().unwrap();
        let dir = data.path().join("backups");
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..9 {
            std::fs::write(dir.join(format!("pos-2026010{i}-000000.db")), "x").unwrap();
        }
        let live = rusqlite::Connection::open_in_memory().unwrap();
        // A file where the directory should be: the copy cannot be made.
        let blocked = data.path().join("blocked");
        std::fs::write(&blocked, "x").unwrap();
        let config = pos_core::config::PosConfig::default();
        assert!(backup_database_impl(&live, &config, &blocked).is_err());
        assert_eq!(backup_names(&dir).len(), 9);
        // A copy that fails its check deletes nothing either.
        let corrupt = dir.join("pos-20260108-000000.db");
        std::fs::write(&corrupt, vec![0u8; 4096]).unwrap();
        assert!(verify_backup(&corrupt).is_err());
        assert_eq!(backup_names(&dir).len(), 9);
    }

    #[test]
    fn no_reminder_without_an_open_period() {
        assert!(!z_reminder_due(None, at(2026, 7, 3, 23, 30)));
    }

    #[test]
    fn reminder_from_22h_on_the_day_of_the_period() {
        let p = period("2026-07-03");
        assert!(!z_reminder_due(Some(&p), at(2026, 7, 3, 21, 59)));
        assert!(z_reminder_due(Some(&p), at(2026, 7, 3, 22, 0)));
    }

    #[test]
    fn reminder_all_day_when_the_period_started_earlier() {
        // Forgotten Z: the reminder stays up the next morning.
        let p = period("2026-07-03");
        assert!(z_reminder_due(Some(&p), at(2026, 7, 4, 1, 30)));
        assert!(z_reminder_due(Some(&p), at(2026, 7, 4, 11, 0)));
    }

    #[test]
    fn reminder_uses_belgian_time_across_daylight_saving() {
        // 21:30 UTC is 23:30 in summer (CEST) and 22:30 in winter (CET).
        let summer = chrono::Utc.with_ymd_and_hms(2026, 7, 3, 21, 30, 0).unwrap();
        let winter = chrono::Utc.with_ymd_and_hms(2026, 1, 9, 20, 30, 0).unwrap();
        assert!(z_reminder_due(
            Some(&period("2026-07-03")),
            summer.with_timezone(&Brussels)
        ));
        assert!(!z_reminder_due(
            Some(&period("2026-01-09")),
            winter.with_timezone(&Brussels)
        ));
    }
}
