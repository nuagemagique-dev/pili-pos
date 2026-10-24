use pos_db::queries::reports::ZReportSummary;
use tauri::State;

use pos_engine::commands::report::ReportResult;
use pos_engine::commands::sales_report::SalesReport;

use super::Managed;

/// Generate an X-report (mid-day, non-resetting).
#[tauri::command]
pub async fn generate_x_report(
    state: State<'_, Managed>,
    employee_insz: String,
) -> Result<ReportResult, String> {
    pos_engine::commands::report::do_generate_x_report(state.inner(), employee_insz).await
}

/// Returns true if no Z-report exists for today AND the local time
/// is past 22:00 - used by the Vue header to show a warning banner
/// reminding the manager to run the end-of-day Z before leaving.
#[tauri::command]
pub fn needs_z_report(state: State<'_, Managed>) -> Result<bool, String> {
    pos_engine::commands::report::do_needs_z_report(state.inner())
}

/// List past Z-reports for the history view (newest first).
#[tauri::command]
pub fn list_z_reports(state: State<'_, Managed>) -> Result<Vec<ZReportSummary>, String> {
    pos_engine::commands::report::do_list_z_reports(state.inner())
}

/// Generate the end-of-day Z (M184): the financial Z and the user Z
/// are built together, signed by the FDM in ONE GraphQL operation,
/// and archived atomically (both R events or none).
#[tauri::command]
pub async fn generate_z_report(
    state: State<'_, Managed>,
    employee_insz: String,
) -> Result<ReportResult, String> {
    pos_engine::commands::report::do_generate_z_report(state.inner(), employee_insz).await
}

/// Generate a user X-report (M182, non-resetting) on demand from the
/// Reports menu.
#[tauri::command]
pub async fn generate_user_x_report(
    state: State<'_, Managed>,
    employee_insz: String,
) -> Result<ReportResult, String> {
    pos_engine::commands::report::do_generate_user_x_report(state.inner(), employee_insz).await
}

/// Create a timestamped backup of the live database.
#[tauri::command]
pub fn backup_database(state: State<'_, Managed>) -> Result<String, String> {
    pos_engine::commands::report::do_backup_database(state.inner())
}

/// The non-fiscal sales report of the booking dates `from..=to` (`YYYY-MM-DD`, at most 92 days).
#[tauri::command]
pub fn sales_report(
    state: State<'_, Managed>,
    from_date: String,
    to_date: String,
) -> Result<SalesReport, String> {
    pos_engine::commands::sales_report::do_sales_report(state.inner(), from_date, to_date)
}

/// Print the non-fiscal sales report of the range on the ticket printer.
#[tauri::command]
pub async fn print_sales_report(
    state: State<'_, Managed>,
    from_date: String,
    to_date: String,
) -> Result<(), String> {
    pos_engine::commands::sales_report::do_print_sales_report(state.inner(), from_date, to_date)
        .await
}
