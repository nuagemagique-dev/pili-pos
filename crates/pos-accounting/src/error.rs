//! Errors of the accountant export, each with a stable code the command line prints.

use std::path::PathBuf;

/// Why an export did not happen. Nothing is written when one is returned.
#[derive(Debug, thiserror::Error)]
pub enum AccountingError {
    /// The database file cannot be opened or read.
    #[error("cannot read the database: {0}")]
    DbUnreadable(String),
    /// The file is a SQLite database but not a POS archive (or an old one without migration 4).
    #[error("not a POS archive: {0}")]
    NotAPiliDb(String),
    /// A file listed in `MANIFEST.sha256` of an export folder is missing or changed.
    #[error("the export folder does not match its MANIFEST.sha256: {0}")]
    ManifestMismatch(String),
    /// The requested period is not a valid month or date range.
    #[error("invalid period: {0}")]
    PeriodInvalid(String),
    /// Every booking period of the range is still open (no Z yet).
    #[error("no closed booking period in the range, still open: {0}")]
    PeriodOpen(String),
    /// No booking period at all in the range.
    #[error("no booking period in the range {from} to {to}")]
    EmptyRange { from: String, to: String },
    /// A day's detail does not add up to its Z report.
    #[error(
        "day {day} (Z {z_report_no}): {field} is {found} in the signed tickets and {expected} in the Z report"
    )]
    ReconciliationFailed {
        day: String,
        z_report_no: i64,
        field: String,
        expected: String,
        found: String,
    },
    /// The archive holds a signed event the tool cannot read.
    #[error("unreadable signed event {event_id}: {detail}")]
    EventUnreadable { event_id: i64, detail: String },
    /// The accounts file is not valid, or an account needed by the journal is missing.
    #[error("accounts: {0}")]
    Accounts(String),
    /// The journal of a day does not balance (an internal inconsistency).
    #[error("journal of day {day} does not balance: debit {debit}, credit {credit}")]
    JournalUnbalanced {
        day: String,
        debit: String,
        credit: String,
    },
    /// The output folder already exists: the tool never overwrites.
    #[error("the output folder already exists: {0}")]
    OutExists(PathBuf),
    /// Writing the output failed.
    #[error("cannot write the output: {0}")]
    Io(String),
}

impl AccountingError {
    /// Stable code of the error (printed by the command line).
    pub fn code(&self) -> &'static str {
        match self {
            Self::DbUnreadable(_) => "db_unreadable",
            Self::NotAPiliDb(_) => "not_a_pili_db",
            Self::ManifestMismatch(_) => "manifest_mismatch",
            Self::PeriodInvalid(_) => "period_invalid",
            Self::PeriodOpen(_) => "period_open",
            Self::EmptyRange { .. } => "empty_range",
            Self::ReconciliationFailed { .. } => "reconciliation_failed",
            Self::EventUnreadable { .. } => "event_unreadable",
            Self::Accounts(_) => "accounts_invalid",
            Self::JournalUnbalanced { .. } => "journal_unbalanced",
            Self::OutExists(_) => "out_exists",
            Self::Io(_) => "io_error",
        }
    }

    /// Exit code of the command line: 2 when the books do not reconcile, 1 otherwise.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::ReconciliationFailed { .. } => 2,
            _ => 1,
        }
    }
}

pub type Result<T> = std::result::Result<T, AccountingError>;
