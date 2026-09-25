use chrono::Timelike;
use pos_db::queries::reports::{self as report_queries, ZReportSummary};
use pos_print::report::{
    ReportEventTotal, ReportPaymentTotal, ReportPrintData, ReportVatTotal, UserReportPrintData,
    UserReportUser,
};
use serde::Serialize;
use tauri::State;

use pos_fiscal::archive::{self, ArchiveParams};
use pos_fiscal::counter::FiscalCounter;
use pos_fiscal::event::{
    TurnoverReportData, UserReportData, build_report_turnover_x, build_report_turnover_z,
    build_report_user_x, build_report_user_z,
};
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::inputs::*;
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
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let (config, bp_id, turnover_data, ticket_number, print_data) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = state
            .booking_period_id
            .lock()
            .map_err(|e| e.to_string())?
            .clone();
        let data = build_turnover_data(&db, &config, &today)?;
        let tn = FiscalCounter::peek_next(&db, EventLabel::R.in_mode(config.training_mode))
            .map_err(|e| e.to_string())?;
        let pd = build_print_data(&db, &config, &today, "X", None, tn, &employee_insz)?;
        (config, bp, data, tn, pd)
    };

    let report = build_report_turnover_x(
        &config,
        ticket_number,
        &employee_insz,
        &bp_id,
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
    let has_z = pos_db::queries::reports::has_z_report_today(&db).map_err(|e| e.to_string())?;
    if has_z {
        return Ok(false);
    }
    let hour = chrono::Local::now().hour();
    Ok(hour >= 22)
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
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let (
        config,
        bp_id,
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
        let bp = state
            .booking_period_id
            .lock()
            .map_err(|e| e.to_string())?
            .clone();
        let data = build_turnover_data(&db, &config, &today)?;
        let users = build_user_report_data(&db, &config, &today)?;
        // Two sequential ticket numbers — the M184 fixture shows the
        // user Z carrying posFiscalTicketNo = turnover Z + 1.
        // Peeked, not consumed: both numbers are committed with the
        // archive once the FDM has signed (see commit_signed_event).
        let tn = FiscalCounter::peek_next(&db, EventLabel::R.in_mode(config.training_mode))
            .map_err(|e| e.to_string())?;
        let un = tn + 1;
        let rn = report_queries::next_z_report_number(&db).map_err(|e| e.to_string())?;
        let pd = build_print_data(&db, &config, &today, "Z", Some(rn), tn, &employee_insz)?;
        let upd = build_user_print_data(&db, &config, &today, "Z", Some(rn), un, &employee_insz)?;
        (config, bp, data, users, tn, un, rn, pd, upd)
    };

    let turnover_report = build_report_turnover_z(
        &config,
        turnover_ticket,
        &employee_insz,
        &bp_id,
        report_no,
        &today,
        turnover_data,
    );
    let user_report = build_report_user_z(
        &config,
        user_ticket,
        &employee_insz,
        &bp_id,
        report_no,
        &today,
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
        debug_assert_eq!(committed, turnover_ticket);
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
        debug_assert_eq!(committed, user_ticket);
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
        report_queries::save_z_report(
            &tx,
            &today,
            report_no,
            &turnover_json,
            Some(fiscal_event_id),
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;

        // Auto-backup after Z-report — best-effort, failure never blocks
        // the Z itself. Uses SQLite's built-in backup API which is safe
        // for WAL-mode databases.
        if let Err(e) = backup_database_impl(&db, &config) {
            tracing::warn!(%e, "post-Z backup failed (non-fatal)");
        }
    }

    // Start new booking period after Z-report
    {
        let mut bp = state.booking_period_id.lock().map_err(|e| e.to_string())?;
        *bp = uuid::Uuid::new_v4().to_string();
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
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let (config, bp_id, user_data, ticket_number, print_data) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = state
            .booking_period_id
            .lock()
            .map_err(|e| e.to_string())?
            .clone();
        let users = build_user_report_data(&db, &config, &today)?;
        let tn = FiscalCounter::peek_next(&db, EventLabel::R.in_mode(config.training_mode))
            .map_err(|e| e.to_string())?;
        let pd = build_user_print_data(&db, &config, &today, "X", None, tn, &employee_insz)?;
        (config, bp, users, tn, pd)
    };

    let report = build_report_user_x(&config, ticket_number, &employee_insz, &bp_id, user_data);
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

/// Build the data struct needed by the thermal report printer.
fn build_print_data(
    db: &rusqlite::Connection,
    config: &pos_core::config::PosConfig,
    today: &str,
    report_type: &str,
    report_no: Option<i64>,
    ticket_number: i64,
    employee_insz: &str,
) -> Result<ReportPrintData, String> {
    let event_totals = report_queries::get_event_totals(db, today).map_err(|e| e.to_string())?;
    let vat_totals = report_queries::get_vat_totals(db, today).map_err(|e| e.to_string())?;
    let payment_totals =
        report_queries::get_payment_totals(db, today).map_err(|e| e.to_string())?;
    let (first_dt, last_dt) =
        report_queries::get_datetime_range(db, today).map_err(|e| e.to_string())?;
    let drawer_opens = report_queries::count_drawer_opens(db, today).map_err(|e| e.to_string())?;

    let grand_total_cents: i64 = payment_totals.iter().map(|p| p.amount_cents).sum();

    let vat_rows: Vec<ReportVatTotal> = vat_totals
        .iter()
        .map(|vt| {
            let (rate, code) = match vt.vat_code.as_str() {
                "A" => (21.0, "A"),
                "B" => (12.0, "B"),
                "C" => (6.0, "C"),
                "D" => (0.0, "D"),
                _ => (0.0, "X"),
            };
            let total_incl = vt.total_cents;
            let vat_cents = if rate > 0.0 {
                total_incl
                    - ((total_incl as f64 * 10000.0 / (10000.0 + rate * 100.0)).round() as i64)
            } else {
                0
            };
            ReportVatTotal {
                code: code.to_string(),
                rate,
                total_incl_cents: total_incl,
                vat_cents,
            }
        })
        .collect();

    Ok(ReportPrintData {
        report_type: report_type.to_string(),
        report_no,
        restaurant_name: config.restaurant_name.clone(),
        restaurant_address: config.restaurant_address.clone(),
        restaurant_vat: config.restaurant_vat.clone(),
        pos_id: config.pos_id.clone(),
        terminal_id: config.terminal_id.clone(),
        report_date: today.to_string(),
        first_datetime: first_dt,
        last_datetime: last_dt,
        employee_insz: employee_insz.to_string(),
        is_training: config.training_mode,
        event_totals: event_totals
            .iter()
            .map(|et| ReportEventTotal {
                label: et.event_label.clone(),
                count: et.ticket_count,
                amount_cents: et.amount_cents,
            })
            .collect(),
        vat_totals: vat_rows,
        payment_totals: payment_totals
            .iter()
            .map(|pt| ReportPaymentTotal {
                method: pt.method.clone(),
                count: pt.count,
                amount_cents: pt.amount_cents,
            })
            .collect(),
        grand_total_cents,
        drawer_opens,
        fdm_id: None,
        fdm_datetime: None,
        short_signature: None,
        ticket_number,
    })
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

/// Build TurnoverReportData from current DB state.
fn build_turnover_data(
    db: &rusqlite::Connection,
    config: &pos_core::config::PosConfig,
    booking_date: &str,
) -> Result<TurnoverReportData, String> {
    let event_totals =
        pos_db::queries::reports::get_event_totals(db, booking_date).map_err(|e| e.to_string())?;
    let vat_totals =
        pos_db::queries::reports::get_vat_totals(db, booking_date).map_err(|e| e.to_string())?;
    let department_totals = pos_db::queries::reports::get_department_totals(db, booking_date)
        .map_err(|e| e.to_string())?;
    let payment_totals = pos_db::queries::reports::get_payment_totals(db, booking_date)
        .map_err(|e| e.to_string())?;
    let (first_dt, last_dt) = pos_db::queries::reports::get_datetime_range(db, booking_date)
        .map_err(|e| e.to_string())?;
    let drawers_count = pos_db::queries::reports::count_drawer_opens(db, booking_date)
        .map_err(|e| e.to_string())?;

    let transactions: Vec<EventTotalInput> = event_totals
        .iter()
        .map(|et| {
            let label = match et.event_label.as_str() {
                "N" => EventLabel::N,
                "P" => EventLabel::P,
                "F" => EventLabel::F,
                "S" => EventLabel::S,
                "I" => EventLabel::I,
                "R" => EventLabel::R,
                "C" => EventLabel::C,
                "T" => EventLabel::T,
                _ => EventLabel::N,
            };
            EventTotalInput {
                event_label: label,
                ticket_count: et.ticket_count,
                amount: et.amount_cents as f64 / 100.0,
            }
        })
        .collect();

    let vats: Vec<VatTotalInput> = vat_totals
        .iter()
        .map(|vt| {
            let (label, rate) = match vt.vat_code.as_str() {
                "A" => (VatLabel::A, 21.0),
                "B" => (VatLabel::B, 12.0),
                "C" => (VatLabel::C, 6.0),
                "D" => (VatLabel::D, 0.0),
                _ => (VatLabel::X, 0.0),
            };
            let total = vt.total_cents as f64 / 100.0;
            let taxable = if rate > 0.0 {
                total / (1.0 + rate / 100.0)
            } else {
                total
            };
            let vat_amount = total - taxable;
            VatTotalInput {
                label,
                rate,
                taxable_amount: taxable,
                vat_amount,
                total_amount: total,
            }
        })
        .collect();

    let payments: Vec<PaymentTotalInput> = payment_totals
        .iter()
        .map(|pt| {
            let ptype = match pt.method.as_str() {
                "cash" => PaymentType::Cash,
                "card" => PaymentType::CardDebit,
                _ => PaymentType::Other,
            };
            PaymentTotalInput {
                id: pt.method.clone(),
                name: pt.method.clone(),
                payment_type: ptype,
                amount: vec![PaymentTotalAmountInput {
                    amount_type: PaymentLineType::Payment,
                    normal_amount: pt.amount_cents as f64 / 100.0,
                    negative_corrections: 0.0,
                    positive_corrections: 0.0,
                    corrections_count: 0,
                    total_amount: pt.amount_cents as f64 / 100.0,
                    normal_foreign: None,
                    corrections_foreign: None,
                }],
            }
        })
        .collect();

    let pos_devices = vec![PosDeviceInput {
        pos_id: config.pos_id.clone(),
        terminal_id: Some(config.terminal_id.clone()),
        first_pos_date_time: first_dt.clone(),
        last_pos_date_time: last_dt.clone(),
    }];

    let fdm_devices = build_fdm_devices(db, booking_date, &first_dt, &last_dt)?;

    Ok(TurnoverReportData {
        pos_devices,
        fdm_devices,
        turnover: TurnoverInput {
            transactions,
            departments: department_totals
                .iter()
                .map(|d| DepartmentTotalInput {
                    department_id: d.department_id.clone(),
                    department_name: d.department_name.clone(),
                    amount: d.total_cents as f64 / 100.0,
                })
                .collect(),
            vats,
            payments,
            drawers_open_count: drawers_count,
            neg_quantities: vec![],
            price_changes: vec![],
            invoices: None,
        },
    })
}

/// The `fdmDevices` block of every X/Z report, read from the FDM
/// references archived that day (the FDM owns those numbers; the POS
/// replays them). Empty-identity fallback only when the day has no
/// signed event yet.
fn build_fdm_devices(
    db: &rusqlite::Connection,
    booking_date: &str,
    fallback_first_dt: &str,
    fallback_last_dt: &str,
) -> Result<Vec<FdmDeviceInput>, String> {
    let range = report_queries::get_fdm_ref_range(db, booking_date).map_err(|e| e.to_string())?;
    Ok(match range {
        Some(r) => vec![FdmDeviceInput {
            fdm_id: r.fdm_id,
            first_fdm_date_time: r.first_datetime,
            first_total_counter: r.first_total_counter,
            last_fdm_date_time: r.last_datetime,
            last_total_counter: r.last_total_counter,
        }],
        None => vec![FdmDeviceInput {
            fdm_id: String::new(),
            first_fdm_date_time: fallback_first_dt.to_string(),
            first_total_counter: 0,
            last_fdm_date_time: fallback_last_dt.to_string(),
            last_total_counter: 0,
        }],
    })
}

/// Map a local payment method string onto the fiscal PaymentType.
fn payment_type_of(method: &str) -> PaymentType {
    match method {
        "cash" => PaymentType::Cash,
        "card" => PaymentType::CardDebit,
        _ => PaymentType::Other,
    }
}

/// Build the `users` block of the user reports (M182/M183/M184) from
/// the day's archived events, orders and payments.
fn build_user_report_data(
    db: &rusqlite::Connection,
    config: &pos_core::config::PosConfig,
    booking_date: &str,
) -> Result<UserReportData, String> {
    let activity =
        report_queries::get_user_activity(db, booking_date).map_err(|e| e.to_string())?;
    let payment_totals =
        report_queries::get_user_payment_totals(db, booking_date).map_err(|e| e.to_string())?;
    let social =
        report_queries::get_user_social_events(db, booking_date).map_err(|e| e.to_string())?;
    let (first_dt, last_dt) =
        report_queries::get_datetime_range(db, booking_date).map_err(|e| e.to_string())?;

    let users: Vec<UserItemInput> = activity
        .iter()
        .map(|ua| UserItemInput {
            employee_id: ua.user_insz.clone(),
            total_amount: ua.total_cents as f64 / 100.0,
            first_pos_date_time: ua.first_datetime.clone(),
            last_pos_date_time: ua.last_datetime.clone(),
            social_events: social
                .iter()
                .filter(|s| s.user_insz == ua.user_insz)
                .map(|s| InOutItemInput {
                    pos_date_time: s.pos_datetime.clone(),
                    in_out: if s.operation == "WORK_IN" {
                        InOut::In
                    } else {
                        InOut::Out
                    },
                })
                .collect(),
            payments: payment_totals
                .iter()
                .filter(|p| p.user_insz == ua.user_insz)
                .map(|p| PaymentTotalInput {
                    id: p.method.clone(),
                    name: p.method.clone(),
                    payment_type: payment_type_of(&p.method),
                    amount: vec![PaymentTotalAmountInput {
                        amount_type: PaymentLineType::Payment,
                        normal_amount: p.amount_cents as f64 / 100.0,
                        negative_corrections: 0.0,
                        positive_corrections: 0.0,
                        corrections_count: 0,
                        total_amount: p.amount_cents as f64 / 100.0,
                        normal_foreign: None,
                        corrections_foreign: None,
                    }],
                })
                .collect(),
        })
        .collect();

    Ok(UserReportData {
        pos_devices: vec![PosDeviceInput {
            pos_id: config.pos_id.clone(),
            terminal_id: Some(config.terminal_id.clone()),
            first_pos_date_time: first_dt.clone(),
            last_pos_date_time: last_dt.clone(),
        }],
        fdm_devices: build_fdm_devices(db, booking_date, &first_dt, &last_dt)?,
        users,
    })
}

/// Build the data struct for the printed user report.
fn build_user_print_data(
    db: &rusqlite::Connection,
    config: &pos_core::config::PosConfig,
    today: &str,
    report_type: &str,
    report_no: Option<i64>,
    ticket_number: i64,
    employee_insz: &str,
) -> Result<UserReportPrintData, String> {
    let activity = report_queries::get_user_activity(db, today).map_err(|e| e.to_string())?;
    let payment_totals =
        report_queries::get_user_payment_totals(db, today).map_err(|e| e.to_string())?;
    let social = report_queries::get_user_social_events(db, today).map_err(|e| e.to_string())?;

    let users: Vec<UserReportUser> = activity
        .iter()
        .map(|ua| UserReportUser {
            insz: ua.user_insz.clone(),
            first_datetime: ua.first_datetime.clone(),
            last_datetime: ua.last_datetime.clone(),
            total_cents: ua.total_cents,
            payments: payment_totals
                .iter()
                .filter(|p| p.user_insz == ua.user_insz)
                .map(|p| ReportPaymentTotal {
                    method: p.method.clone(),
                    count: p.count,
                    amount_cents: p.amount_cents,
                })
                .collect(),
            social_events: social
                .iter()
                .filter(|s| s.user_insz == ua.user_insz)
                .map(|s| (s.operation.clone(), s.pos_datetime.clone()))
                .collect(),
        })
        .collect();

    Ok(UserReportPrintData {
        report_type: report_type.to_string(),
        report_no,
        restaurant_name: config.restaurant_name.clone(),
        restaurant_address: config.restaurant_address.clone(),
        restaurant_vat: config.restaurant_vat.clone(),
        terminal_id: config.terminal_id.clone(),
        report_date: today.to_string(),
        employee_insz: employee_insz.to_string(),
        is_training: config.training_mode,
        users,
        fdm_id: None,
        fdm_datetime: None,
        short_signature: None,
        ticket_number,
    })
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
