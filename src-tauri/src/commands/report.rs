use chrono::Timelike;
use pos_db::queries::reports::{self as report_queries, ZReportSummary};
use pos_print::report::{
    ReportEventTotal, ReportPaymentTotal, ReportPrintData, ReportVatTotal, UserReportPrintData,
    UserReportUser,
};
use serde::Serialize;
use tauri::State;

use pos_db::queries::booking::BookingPeriod;
use pos_fiscal::archive::{self, ArchiveParams};
use pos_fiscal::counter::FiscalCounter;
use pos_fiscal::event::{
    TurnoverReportData, UserReportData, build_report_turnover_x, build_report_turnover_z,
    build_report_user_x, build_report_user_z,
};
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::inputs::*;
use pos_fiscal::report_totals::{self, ReportEvent};
use pos_fiscal::types::*;

use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportResult {
    pub success: bool,
    pub report_type: String,
    pub report_no: Option<i64>,
    pub error: Option<String>,
}

/// Generate an X-report (mid-day, non-resetting).
#[tauri::command]
pub async fn generate_x_report(
    state: State<'_, AppState>,
    employee_insz: String,
) -> Result<ReportResult, String> {
    do_generate_x_report(state.inner(), employee_insz).await
}

pub async fn do_generate_x_report(
    state: &AppState,
    employee_insz: String,
) -> Result<ReportResult, String> {
    let (config, period, turnover_data, ticket_number, print_data) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let period = super::booking_period(&db)?;
        let events = period_events(&db, &period.id)?;
        let data = turnover_data(&config, &events);
        let tn = FiscalCounter::peek_next(&db, EventLabel::R.in_mode(config.training_mode))
            .map_err(|e| e.to_string())?;
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
        ticket_number,
        &employee_insz,
        &as_booking(&period),
        turnover_data,
    );
    let request_json = serde_json::to_string(&report).map_err(|e| e.to_string())?;

    let fdm = FdmClient::new(&config.fdm_url);
    let sign_result = match fdm
        .sign_report_turnover_x(report, config.training_mode)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            return Ok(ReportResult {
                success: false,
                report_type: "X".to_string(),
                report_no: None,
                error: Some(format!("FDM error: {e}")),
            });
        }
    };

    super::commit_signed_event(
        state,
        &config,
        super::SignedEvent {
            label: EventLabel::R.in_mode(config.training_mode),
            ticket_number,
            order_id: None,
            user_insz: &employee_insz,
            request_json: &request_json,
            sign_result: &sign_result,
            sent_entries: None,
        },
    )?;

    // Auto-print report on thermal printer
    print_report(&config, print_data, Some(&sign_result)).await;

    Ok(ReportResult {
        success: true,
        report_type: "X".to_string(),
        report_no: None,
        error: None,
    })
}

/// Returns true if no Z-report exists for today AND the local time
/// is past 22:00 — used by the Vue header to show a warning banner
/// reminding the manager to run the end-of-day Z before leaving.
#[tauri::command]
pub fn needs_z_report(state: State<'_, AppState>) -> Result<bool, String> {
    do_needs_z_report(state.inner())
}

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
#[tauri::command]
pub fn list_z_reports(state: State<'_, AppState>) -> Result<Vec<ZReportSummary>, String> {
    do_list_z_reports(state.inner())
}

pub fn do_list_z_reports(state: &AppState) -> Result<Vec<ZReportSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::reports::list_z_reports(&db).map_err(|e| e.to_string())
}

/// Generate the end-of-day Z (M184): the financial Z and the user Z
/// are built together, signed by the FDM in ONE GraphQL operation,
/// and archived atomically (both R events or none).
#[tauri::command]
pub async fn generate_z_report(
    state: State<'_, AppState>,
    employee_insz: String,
) -> Result<ReportResult, String> {
    do_generate_z_report(state.inner(), employee_insz).await
}

pub async fn do_generate_z_report(
    state: &AppState,
    employee_insz: String,
) -> Result<ReportResult, String> {
    let (
        config,
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
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let period = super::booking_period(&db)?;
        let events = period_events(&db, &period.id)?;
        let data = turnover_data(&config, &events);
        let users = user_data(&config, &events);
        // Two sequential ticket numbers — the M184 fixture shows the
        // user Z carrying posFiscalTicketNo = turnover Z + 1.
        // Peeked, not consumed: both numbers are committed with the
        // archive once the FDM has signed (see commit_signed_event).
        let tn = FiscalCounter::peek_next(&db, EventLabel::R.in_mode(config.training_mode))
            .map_err(|e| e.to_string())?;
        let un = tn + 1;
        let rn = report_queries::next_z_report_number(&db).map_err(|e| e.to_string())?;
        let pd = build_print_data(
            &config,
            &period,
            &events,
            &data,
            "Z",
            Some(rn),
            tn,
            &employee_insz,
        );
        let upd = build_user_print_data(
            &config,
            &period,
            &events,
            &users,
            "Z",
            Some(rn),
            un,
            &employee_insz,
        );
        (config, period, data, users, tn, un, rn, pd, upd)
    };

    let turnover_report = build_report_turnover_z(
        &config,
        turnover_ticket,
        &employee_insz,
        &as_booking(&period),
        report_no,
        &period.booking_date,
        turnover_data,
    );
    let user_report = build_report_user_z(
        &config,
        user_ticket,
        &employee_insz,
        &as_booking(&period),
        report_no,
        &period.booking_date,
        user_data,
    );
    let turnover_json = serde_json::to_string(&turnover_report).map_err(|e| e.to_string())?;
    let user_json = serde_json::to_string(&user_report).map_err(|e| e.to_string())?;

    // M184 — one GraphQL operation signs both reports.
    let fdm = FdmClient::new(&config.fdm_url);
    let (turnover_result, user_result) = match fdm
        .sign_report_turnover_z_user_z(turnover_report, user_report, config.training_mode)
        .await
    {
        Ok(signed) => signed,
        Err(e) => {
            return Ok(ReportResult {
                success: false,
                report_type: "Z".to_string(),
                report_no: Some(report_no),
                error: Some(format!("FDM error: {e}")),
            });
        }
    };
    let label = EventLabel::R.in_mode(config.training_mode);

    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        // Both R events + the z_reports row commit atomically: a crash
        // between the two archives can never leave a half-recorded Z.
        let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
        let committed = FiscalCounter::next(&tx, label).map_err(|e| e.to_string())?;
        // Graceful error, not a panic — see `commands/mod.rs::commit_signed_event`.
        if committed != turnover_ticket {
            return Err(format!(
                "counter drifted between peek and commit for {label:?} (peeked \
                 {turnover_ticket}, committed {committed}) — likely a concurrent \
                 report generation; retry"
            ));
        }
        let fiscal_event_id = archive::archive_event(
            &tx,
            ArchiveParams {
                event_type: label,
                sequence_number: turnover_ticket,
                order_id: None,
                pos_id: &config.pos_id,
                terminal_id: &config.terminal_id,
                user_insz: &employee_insz,
                request_json: &turnover_json,
                sign_result: Some(&turnover_result),
            },
        )
        .map_err(|e| e.to_string())?;
        let committed = FiscalCounter::next(&tx, label).map_err(|e| e.to_string())?;
        // Graceful error, not a panic — see `commands/mod.rs::commit_signed_event`.
        if committed != user_ticket {
            return Err(format!(
                "counter drifted between peek and commit for {label:?} (peeked \
                 {user_ticket}, committed {committed}) — likely a concurrent \
                 report generation; retry"
            ));
        }
        archive::archive_event(
            &tx,
            ArchiveParams {
                event_type: label,
                sequence_number: user_ticket,
                order_id: None,
                pos_id: &config.pos_id,
                terminal_id: &config.terminal_id,
                user_insz: &employee_insz,
                request_json: &user_json,
                sign_result: Some(&user_result),
            },
        )
        .map_err(|e| e.to_string())?;
        // A training Z is an exercise: it neither takes a Z number nor
        // closes the real booking period.
        if !config.training_mode {
            report_queries::save_z_report(
                &tx,
                &period.booking_date,
                report_no,
                &turnover_json,
                Some(fiscal_event_id),
            )
            .map_err(|e| e.to_string())?;
            pos_db::queries::booking::close_period(&tx, &period.id, fiscal_event_id)
                .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;

        // Auto-backup after Z-report — best-effort, failure never blocks
        // the Z itself. Uses SQLite's built-in backup API which is safe
        // for WAL-mode databases.
        if let Err(e) = backup_database_impl(&db, &config) {
            tracing::warn!(%e, "post-Z backup failed (non-fatal)");
        }
    }

    // Auto-print: financial Z first, then the user Z as a second
    // sequential ticket.
    print_report(&config, print_data, Some(&turnover_result)).await;
    print_user_report(&config, user_print_data, Some(&user_result)).await;

    Ok(ReportResult {
        success: true,
        report_type: "Z".to_string(),
        report_no: Some(report_no),
        error: None,
    })
}

/// Generate a user X-report (M182, non-resetting) on demand from the
/// Reports menu.
#[tauri::command]
pub async fn generate_user_x_report(
    state: State<'_, AppState>,
    employee_insz: String,
) -> Result<ReportResult, String> {
    do_generate_user_x_report(state.inner(), employee_insz).await
}

pub async fn do_generate_user_x_report(
    state: &AppState,
    employee_insz: String,
) -> Result<ReportResult, String> {
    let (config, period, user_data, ticket_number, print_data) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let period = super::booking_period(&db)?;
        let events = period_events(&db, &period.id)?;
        let users = user_data(&config, &events);
        let tn = FiscalCounter::peek_next(&db, EventLabel::R.in_mode(config.training_mode))
            .map_err(|e| e.to_string())?;
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
        ticket_number,
        &employee_insz,
        &as_booking(&period),
        user_data,
    );
    let request_json = serde_json::to_string(&report).map_err(|e| e.to_string())?;

    let fdm = FdmClient::new(&config.fdm_url);
    let sign_result = match fdm.sign_report_user_x(report, config.training_mode).await {
        Ok(result) => result,
        Err(e) => {
            return Ok(ReportResult {
                success: false,
                report_type: "X-USER".to_string(),
                report_no: None,
                error: Some(format!("FDM error: {e}")),
            });
        }
    };

    super::commit_signed_event(
        state,
        &config,
        super::SignedEvent {
            label: EventLabel::R.in_mode(config.training_mode),
            ticket_number,
            order_id: None,
            user_insz: &employee_insz,
            request_json: &request_json,
            sign_result: &sign_result,
            sent_entries: None,
        },
    )?;

    print_user_report(&config, print_data, Some(&sign_result)).await;

    Ok(ReportResult {
        success: true,
        report_type: "X-USER".to_string(),
        report_no: None,
        error: None,
    })
}

/// Create a timestamped backup of the live database.
#[tauri::command]
pub fn backup_database(state: State<'_, AppState>) -> Result<String, String> {
    do_backup_database(state.inner())
}

pub fn do_backup_database(state: &AppState) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let config = state.config.lock().map_err(|e| e.to_string())?;
    backup_database_impl(&db, &config)
}

fn backup_database_impl(
    conn: &rusqlite::Connection,
    config: &pos_core::config::PosConfig,
) -> Result<String, String> {
    let backup_dir = match &config.backup_dir {
        Some(dir) => std::path::PathBuf::from(dir),
        None => crate::paths::backups_dir(),
    };
    std::fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("pos-{timestamp}.db");
    let dest_path = backup_dir.join(&filename);

    let mut dest = rusqlite::Connection::open(&dest_path).map_err(|e| e.to_string())?;
    let backup = rusqlite::backup::Backup::new(conn, &mut dest).map_err(|e| e.to_string())?;
    backup
        .run_to_completion(100, std::time::Duration::from_millis(50), None)
        .map_err(|e| e.to_string())?;

    let path_str = dest_path.to_string_lossy().to_string();
    tracing::info!(path = %path_str, "database backup created");
    Ok(path_str)
}

/// Build the printed turnover report from the totals sent to the FDM,
/// so the paper always shows what was signed.
#[allow(clippy::too_many_arguments)]
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
                amount_cents: cents(t.amount),
            })
            .collect(),
        vat_totals: turnover
            .vats
            .iter()
            .map(|v| ReportVatTotal {
                code: enum_name(&v.label),
                rate: v.rate,
                total_incl_cents: cents(v.total_amount),
                vat_cents: cents(v.vat_amount),
            })
            .collect(),
        grand_total_cents: payment_totals.iter().map(|p| p.amount_cents).sum(),
        payment_totals,
        drawer_opens: turnover.drawers_open_count,
        fdm_id: None,
        fdm_datetime: None,
        short_signature: None,
        ticket_number,
    }
}

/// Printed payment lines: the signed totals, with the number of
/// payments made with each method.
fn print_payments<'a>(
    payments: &[PaymentTotalInput],
    events: impl Iterator<Item = &'a ReportEvent> + Clone,
) -> Vec<ReportPaymentTotal> {
    payments
        .iter()
        .map(|p| ReportPaymentTotal {
            method: p.name.clone(),
            count: events
                .clone()
                .filter(|e| e.label == EventLabel::N)
                .filter_map(|e| e.request["financials"].as_array())
                .flatten()
                .filter(|f| f["id"].as_str() == Some(&p.id) && f["name"].as_str() == Some(&p.name))
                .count() as i64,
            amount_cents: p.amount.iter().map(|a| cents(a.total_amount)).sum(),
        })
        .collect()
}

/// First and last POS time of the report's events.
fn period_range(devices: &[PosDeviceInput]) -> (String, String) {
    let parse = |s: &str| chrono::DateTime::parse_from_rfc3339(s).ok();
    let first = devices
        .iter()
        .map(|d| &d.first_pos_date_time)
        .min_by_key(|s| parse(s));
    let last = devices
        .iter()
        .map(|d| &d.last_pos_date_time)
        .max_by_key(|s| parse(s));
    (
        first.cloned().unwrap_or_default(),
        last.cloned().unwrap_or_default(),
    )
}

fn cents(amount: f64) -> i64 {
    (amount * 100.0).round() as i64
}

/// The wire name of a fiscal enum (`EventLabel::N` → "N").
fn enum_name<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Send the formatted report to the thermal printer (best-effort).
async fn print_report(
    config: &pos_core::config::PosConfig,
    mut data: ReportPrintData,
    sign_result: Option<&pos_fiscal::response::SignResult>,
) {
    // Enrich with FDM signature data if available
    if let Some(sr) = sign_result {
        data.fdm_id = Some(sr.fdm_ref.fdm_id.clone());
        data.fdm_datetime = Some(sr.fdm_ref.fdm_date_time.clone());
        data.short_signature = sr.short_signature.clone();
    }

    if let Some(ref addr) = config.printer_address {
        let report_bytes = pos_print::report::format_report(&data);
        match pos_print::transport::send_to_printer(addr, &report_bytes).await {
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
/// every report (see `pos_fiscal::report_totals`).
fn period_events(
    db: &rusqlite::Connection,
    booking_period_id: &str,
) -> Result<Vec<ReportEvent>, String> {
    archive::events_in_booking_period(db, booking_period_id)
        .map_err(|e| e.to_string())?
        .iter()
        .map(|e| {
            ReportEvent::from_archive(e)
                .ok_or_else(|| format!("archived fiscal event {} is unreadable", e.id))
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
    let now = pos_fiscal::event::now_in_belgium().to_rfc3339();
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
#[allow(clippy::too_many_arguments)]
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
            total_cents: cents(u.total_amount),
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
    }
}

/// Send the formatted user report to the thermal printer (best-effort).
async fn print_user_report(
    config: &pos_core::config::PosConfig,
    mut data: UserReportPrintData,
    sign_result: Option<&pos_fiscal::response::SignResult>,
) {
    if let Some(sr) = sign_result {
        data.fdm_id = Some(sr.fdm_ref.fdm_id.clone());
        data.fdm_datetime = Some(sr.fdm_ref.fdm_date_time.clone());
        data.short_signature = sr.short_signature.clone();
    }

    if let Some(ref addr) = config.printer_address {
        let bytes = pos_print::report::format_user_report(&data);
        match pos_print::transport::send_to_printer(addr, &bytes).await {
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
