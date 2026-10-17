//! Facade of the reports, shifts, financial and settings commands (engine modules `report`,
//! `social`, `financial`, `settings`, `fdm_monitor`): one method per Tauri command, same
//! parameter names, one engine call each.

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::common::FdmHealth;
use crate::records::reports::{
    FinancialResult, MoneyDirection, OpenShift, PosSettings, ReportResult, SocialResult,
    ZReportSummary,
};

// The closures name the engine function at the call, which `check_command_parity.py` looks for.
#[allow(clippy::redundant_closure)]
#[uniffi::export]
impl PosEngine {
    /// Generate an X-report (mid-day, non-resetting).
    pub async fn generate_x_report(&self, employee_insz: String) -> Result<ReportResult, PosError> {
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::report::do_generate_x_report(&engine, employee_insz).await
            })
            .await?;
        convert::out(&result)
    }

    /// Generate the end-of-day Z (financial Z and user Z, signed together).
    pub async fn generate_z_report(&self, employee_insz: String) -> Result<ReportResult, PosError> {
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::report::do_generate_z_report(&engine, employee_insz).await
            })
            .await?;
        convert::out(&result)
    }

    /// Generate a user X-report (non-resetting) on demand.
    pub async fn generate_user_x_report(
        &self,
        employee_insz: String,
    ) -> Result<ReportResult, PosError> {
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::report::do_generate_user_x_report(&engine, employee_insz)
                    .await
            })
            .await?;
        convert::out(&result)
    }

    /// Whether the end-of-day Z is due (no Z today and past the reminder hour).
    pub async fn needs_z_report(&self) -> Result<bool, PosError> {
        self.blocking(|state| pos_engine::commands::report::do_needs_z_report(state))
            .await
    }

    /// Past Z-reports for the history view, newest first.
    pub async fn list_z_reports(&self) -> Result<Vec<ZReportSummary>, PosError> {
        let rows = self
            .blocking(|state| pos_engine::commands::report::do_list_z_reports(state))
            .await?;
        convert::out(&rows)
    }

    /// Create a timestamped backup of the live database; returns the backup path.
    pub async fn backup_database(&self) -> Result<String, PosError> {
        self.blocking(|state| pos_engine::commands::report::do_backup_database(state))
            .await
    }

    /// Register an employee clock-in (ONSS presence start).
    pub async fn work_in(&self, employee_insz: String) -> Result<SocialResult, PosError> {
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::social::do_work_in(&engine, employee_insz).await
            })
            .await?;
        convert::out(&result)
    }

    /// Register an employee clock-out (ONSS presence end).
    pub async fn work_out(&self, employee_insz: String) -> Result<SocialResult, PosError> {
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::social::do_work_out(&engine, employee_insz).await
            })
            .await?;
        convert::out(&result)
    }

    /// A manager registers another employee's arrival (S event of the
    /// employee, the manager kept for the audit).
    pub async fn clock_in_employee(&self, employee_insz: String) -> Result<SocialResult, PosError> {
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::social::do_clock_in_employee(&engine, employee_insz).await
            })
            .await?;
        convert::out(&result)
    }

    /// A manager registers another employee's departure.
    pub async fn clock_out_employee(
        &self,
        employee_insz: String,
    ) -> Result<SocialResult, PosError> {
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::social::do_clock_out_employee(&engine, employee_insz).await
            })
            .await?;
        convert::out(&result)
    }

    /// End the employee's shift: PIN check, clock-out, session closed.
    pub async fn end_shift(
        &self,
        insz: String,
        pin: Option<String>,
    ) -> Result<SocialResult, PosError> {
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::social::do_end_shift(&engine, insz, pin).await
            })
            .await?;
        convert::out(&result)
    }

    /// Employees currently on shift.
    pub async fn list_open_shifts(&self) -> Result<Vec<OpenShift>, PosError> {
        let rows = self
            .blocking(|state| pos_engine::commands::social::do_list_open_shifts(state))
            .await?;
        convert::out(&rows)
    }

    /// Record a cash movement (money in, money out) or the count of the drawer.
    pub async fn money_in_out(
        &self,
        employee_insz: String,
        amount_cents: i64,
        direction: MoneyDirection,
        reason: String,
    ) -> Result<FinancialResult, PosError> {
        let direction: pos_engine::commands::financial::MoneyDirection = convert::inp(&direction)?;
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::financial::do_money_in_out(
                    &engine,
                    employee_insz,
                    amount_cents,
                    direction,
                    reason,
                )
                .await
            })
            .await?;
        convert::out(&result)
    }

    /// Record a fiscal drawer opening outside of a sale.
    pub async fn open_drawer_fiscal(
        &self,
        employee_insz: String,
    ) -> Result<FinancialResult, PosError> {
        let result = self
            .spawn(move |engine| async move {
                pos_engine::commands::financial::do_open_drawer_fiscal(&engine, employee_insz).await
            })
            .await?;
        convert::out(&result)
    }

    /// The settings the screens show, with the software version of this build.
    pub async fn get_settings(&self) -> Result<PosSettings, PosError> {
        let settings = self
            .blocking(|state| pos_engine::commands::settings::do_get_settings(state))
            .await?;
        convert::out(&settings)
    }

    /// Switch the training mode; returns the new value.
    pub async fn set_training_mode(&self, enabled: bool) -> Result<bool, PosError> {
        self.blocking(move |state| {
            pos_engine::commands::settings::do_set_training_mode(state, enabled)
        })
        .await
    }

    /// The last FDM health the monitor knows (`None` before the first check).
    pub async fn fdm_health(&self) -> Result<Option<FdmHealth>, PosError> {
        let health = self
            .blocking(|state| pos_engine::commands::fdm_monitor::do_fdm_health(state))
            .await?;
        convert::out(&health)
    }

    /// Check the FDM now; the health is stored and emitted as an `FdmStatus` event.
    pub async fn fdm_check_now(&self) -> Result<FdmHealth, PosError> {
        let health = self
            .spawn(|engine| async move {
                pos_engine::commands::fdm_monitor::do_fdm_check_now(&engine).await
            })
            .await?;
        convert::out(&health)
    }
}
