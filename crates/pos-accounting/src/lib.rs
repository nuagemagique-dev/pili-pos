//! Accountant export of the till's signed archive.
//!
//! [`export`] reads a copy of `pos.db` read-only, checks each booking period closed by
//! a Z report against that Z, and writes a folder `export-comptable-<from>-<to>/` of CSV
//! files, a balanced journal per day, a French note (`LISEZMOI.txt`) and a
//! `MANIFEST.sha256`. It is a stand-alone tool outside the certified binary: no shipped
//! crate depends on it (decision V12-D5), and it never writes to the source.
//!
//! The data read is the signed archive only (see [`source`]); the reconciliation of
//! every day with its Z report ([`reconcile`]) is what makes the files trustworthy.

pub mod aggregate;
pub mod csv;
pub mod error;
pub mod files;
pub mod journal;
pub mod parse;
pub mod readme;
pub mod reconcile;
pub mod source;

use std::path::{Path, PathBuf};

use chrono::{Datelike, NaiveDate};
use sha2::{Digest, Sha256};

pub use csv::Decimal;
pub use error::{AccountingError, Result};
pub use journal::{Accounts, Profile};

use aggregate::{Day, load_day};
use source::Source;

/// The period to export, in booking dates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeriodSpec {
    /// `YYYY-MM`.
    Month(String),
    /// Inclusive `YYYY-MM-DD` bounds.
    Range { from: String, to: String },
}

fn date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AccountingError::PeriodInvalid(format!("{value:?} is not a YYYY-MM-DD date")))
}

impl PeriodSpec {
    /// First and last booking date, `YYYY-MM-DD`.
    pub fn bounds(&self) -> Result<(String, String)> {
        match self {
            Self::Month(month) => {
                let first = NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d").map_err(
                    |_| AccountingError::PeriodInvalid(format!("{month:?} is not a YYYY-MM month")),
                )?;
                let next = if first.month() == 12 {
                    NaiveDate::from_ymd_opt(first.year() + 1, 1, 1)
                } else {
                    NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1)
                }
                .ok_or_else(|| AccountingError::PeriodInvalid("month out of range".into()))?;
                let last = next
                    .pred_opt()
                    .ok_or_else(|| AccountingError::PeriodInvalid("month out of range".into()))?;
                Ok((first.to_string(), last.to_string()))
            }
            Self::Range { from, to } => {
                let (f, t) = (date(from)?, date(to)?);
                if f > t {
                    return Err(AccountingError::PeriodInvalid(format!(
                        "{from} is after {to}"
                    )));
                }
                Ok((f.to_string(), t.to_string()))
            }
        }
    }
}

/// What to export.
#[derive(Debug, Clone)]
pub struct ExportRequest {
    /// A `pos.db` file or an SPF export folder holding one.
    pub db: PathBuf,
    pub period: PeriodSpec,
    /// The folder in which `export-comptable-<from>-<to>/` is created.
    pub out_parent: PathBuf,
    /// JSON file overriding account numbers.
    pub accounts: Option<PathBuf>,
    pub profile: Profile,
    pub decimal: Decimal,
    /// Written in `LISEZMOI.txt` (the only value that differs between two runs).
    pub generated_at: String,
}

/// What an export wrote.
#[derive(Debug, Clone)]
pub struct ExportSummary {
    pub folder: PathBuf,
    /// `(booking date, Z number)` of each period exported.
    pub days: Vec<(String, i64)>,
    /// Booking dates of the open periods left out.
    pub left_out: Vec<String>,
    /// File names written, `MANIFEST.sha256` last.
    pub files: Vec<String>,
}

/// Name of the output folder for a period.
pub fn folder_name(from: &str, to: &str) -> String {
    format!("export-comptable-{from}-{to}")
}

fn io(path: &Path, e: &std::io::Error) -> AccountingError {
    AccountingError::Io(format!("{}: {e}", path.display()))
}

/// Run the export. Nothing is written unless every day reconciles with its Z.
#[expect(
    clippy::too_many_lines,
    reason = "one linear pipeline (read, reconcile, render, write); each step is a call already"
)]
pub fn export(req: &ExportRequest) -> Result<ExportSummary> {
    let (from, to) = req.period.bounds()?;
    let final_dir = req.out_parent.join(folder_name(&from, &to));
    if final_dir.exists() {
        return Err(AccountingError::OutExists(final_dir));
    }
    let accounts = match &req.accounts {
        Some(path) => Accounts::load(path)?,
        None => Accounts::default(),
    };

    let source = Source::open(&req.db)?;
    let periods = source.closed_periods(&from, &to)?;
    let left_out = source.open_periods(&from, &to)?;
    if periods.is_empty() {
        return Err(if left_out.is_empty() {
            AccountingError::EmptyRange { from, to }
        } else {
            AccountingError::PeriodOpen(left_out.join(", "))
        });
    }

    let mut days: Vec<Day> = Vec::with_capacity(periods.len());
    for period in &periods {
        let day = load_day(&source, period)?;
        reconcile::reconcile(&day)?;
        days.push(day);
    }
    let entries = journal::entries(&days, &accounts)?;

    let dec = req.decimal;
    let mut contents: Vec<(&str, Vec<u8>)> = vec![
        ("jours.csv", files::jours(&days, dec).render()),
        ("tva.csv", files::tva(&days, dec).render()),
        ("paiements.csv", files::paiements(&days, dec).render()),
        (
            "remboursements.csv",
            files::remboursements(&days, dec).render(),
        ),
        ("factures.csv", files::factures(&days, dec).render()),
        ("bons_cadeaux.csv", files::bons_cadeaux(&days, dec).render()),
        (
            "mouvements_caisse.csv",
            files::mouvements_caisse(&days, dec).render(),
        ),
        ("rapports_z.csv", files::rapports_z(&days, dec).render()),
        ("journal.csv", journal::render(&entries, req.profile)),
    ];
    let profile_name = match req.profile {
        Profile::Generic => "generic",
        Profile::Winbooks => "winbooks",
        Profile::Yuki => "yuki",
    };
    let source_name = source
        .db_path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let note = readme::render(&readme::Note {
        generated_at: &req.generated_at,
        from: &from,
        to: &to,
        source_name: &source_name,
        source_sha256: &source.sha256,
        manifest_checked: source.manifest_checked,
        days: &days,
        left_out: &left_out,
        decimal: dec,
        profile: profile_name,
    });
    contents.push(("LISEZMOI.txt", note.into_bytes()));
    contents.sort_by(|a, b| a.0.cmp(b.0));
    let mut manifest = String::new();
    for (name, bytes) in &contents {
        manifest.push_str(&hex::encode(Sha256::digest(bytes)));
        manifest.push_str("  ");
        manifest.push_str(name);
        manifest.push('\n');
    }

    std::fs::create_dir_all(&req.out_parent).map_err(|e| io(&req.out_parent, &e))?;
    let mut partial_name = final_dir.as_os_str().to_os_string();
    partial_name.push(".partial");
    let partial = PathBuf::from(partial_name);
    if partial.exists() {
        // A leftover of an interrupted run of this tool.
        std::fs::remove_dir_all(&partial).map_err(|e| io(&partial, &e))?;
    }
    let written = write_folder(&partial, &contents, &manifest);
    if let Err(e) = written {
        let _ = std::fs::remove_dir_all(&partial);
        return Err(e);
    }
    if final_dir.exists() {
        let _ = std::fs::remove_dir_all(&partial);
        return Err(AccountingError::OutExists(final_dir));
    }
    std::fs::rename(&partial, &final_dir).map_err(|e| io(&final_dir, &e))?;

    let mut names: Vec<String> = contents.iter().map(|(n, _)| (*n).to_string()).collect();
    names.push("MANIFEST.sha256".to_string());
    Ok(ExportSummary {
        folder: final_dir,
        days: days
            .iter()
            .map(|d| (d.booking_date.clone(), d.z_report_no))
            .collect(),
        left_out,
        files: names,
    })
}

fn write_folder(dir: &Path, contents: &[(&str, Vec<u8>)], manifest: &str) -> Result<()> {
    std::fs::create_dir(dir).map_err(|e| io(dir, &e))?;
    for (name, bytes) in contents {
        let path = dir.join(name);
        std::fs::write(&path, bytes).map_err(|e| io(&path, &e))?;
    }
    let path = dir.join("MANIFEST.sha256");
    std::fs::write(&path, manifest).map_err(|e| io(&path, &e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_month_spans_its_first_and_last_day() {
        let (f, t) = PeriodSpec::Month("2026-02".into()).bounds().unwrap();
        assert_eq!((f.as_str(), t.as_str()), ("2026-02-01", "2026-02-28"));
        let (f, t) = PeriodSpec::Month("2028-02".into()).bounds().unwrap();
        assert_eq!((f.as_str(), t.as_str()), ("2028-02-01", "2028-02-29"));
        let (f, t) = PeriodSpec::Month("2026-12".into()).bounds().unwrap();
        assert_eq!((f.as_str(), t.as_str()), ("2026-12-01", "2026-12-31"));
    }

    #[test]
    fn bad_periods_are_refused() {
        for p in [
            PeriodSpec::Month("2026-13".into()),
            PeriodSpec::Month("October".into()),
            PeriodSpec::Range {
                from: "2026-10-05".into(),
                to: "2026-10-01".into(),
            },
            PeriodSpec::Range {
                from: "2026-10-01".into(),
                to: "soon".into(),
            },
        ] {
            assert_eq!(p.bounds().unwrap_err().code(), "period_invalid", "{p:?}");
        }
    }

    #[test]
    fn the_folder_name_carries_the_bounds() {
        assert_eq!(
            folder_name("2026-10-01", "2026-10-31"),
            "export-comptable-2026-10-01-2026-10-31"
        );
    }
}
