//! `pili-accounting`: accountant export from a copy of the till's `pos.db`.

use std::path::PathBuf;
use std::process::ExitCode;

use pos_accounting::{AccountingError, Decimal, ExportRequest, PeriodSpec, Profile, export};

const USAGE: &str = "\
pili-accounting: accountant export from a copy of the till's pos.db

USAGE:
    pili-accounting --db <pos.db or export folder> (--month YYYY-MM | --from YYYY-MM-DD --to YYYY-MM-DD)
                    [--out <folder>] [--accounts <accounts.json>]
                    [--profile generic|winbooks|yuki] [--decimal comma|point] [--lang fr]

Writes <out>/export-comptable-<from>-<to>/ (never overwrites an existing folder).
Exit codes: 0 done, 1 error, 2 RECONCILIATION_FAILED (a day does not equal its Z report).";

struct Args {
    db: PathBuf,
    period: PeriodSpec,
    out: PathBuf,
    accounts: Option<PathBuf>,
    profile: Profile,
    decimal: Decimal,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let (mut db, mut month, mut from, mut to) = (None, None, None, None);
    let (mut out, mut accounts) = (None, None);
    let mut profile = Profile::Generic;
    let mut decimal = Decimal::Comma;
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match flag.as_str() {
            "--db" => db = Some(PathBuf::from(value("--db")?)),
            "--month" => month = Some(value("--month")?),
            "--from" => from = Some(value("--from")?),
            "--to" => to = Some(value("--to")?),
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            "--accounts" => accounts = Some(PathBuf::from(value("--accounts")?)),
            "--profile" => {
                let name = value("--profile")?;
                profile = Profile::parse(&name)
                    .ok_or_else(|| format!("unknown profile {name:?} (generic, winbooks, yuki)"))?;
            }
            "--decimal" => {
                decimal = match value("--decimal")?.as_str() {
                    "comma" => Decimal::Comma,
                    "point" => Decimal::Point,
                    other => return Err(format!("unknown decimal {other:?} (comma, point)")),
                };
            }
            "--lang" => {
                let lang = value("--lang")?;
                if lang != "fr" {
                    return Err(format!("language {lang:?} is not available (fr)"));
                }
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    let period = match (month, from, to) {
        (Some(m), None, None) => PeriodSpec::Month(m),
        (None, Some(from), Some(to)) => PeriodSpec::Range { from, to },
        _ => return Err("give --month YYYY-MM, or --from and --to".to_string()),
    };
    Ok(Args {
        db: db.ok_or("--db is required")?,
        period,
        out: out.unwrap_or_else(|| PathBuf::from(".")),
        accounts,
        profile,
        decimal,
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let args = match parse_args(&args) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("ERROR usage: {e}\n\n{USAGE}");
            return ExitCode::from(1);
        }
    };
    let request = ExportRequest {
        db: args.db,
        period: args.period,
        out_parent: args.out,
        accounts: args.accounts,
        profile: args.profile,
        decimal: args.decimal,
        generated_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
    };
    match export(&request) {
        Ok(summary) => {
            println!("OK {}", summary.folder.display());
            for (date, z) in &summary.days {
                println!("  {date}  Z {z}  reconciled");
            }
            for date in &summary.left_out {
                println!("  {date}  left out: no Z yet");
            }
            ExitCode::SUCCESS
        }
        Err(e @ AccountingError::ReconciliationFailed { .. }) => {
            eprintln!("RECONCILIATION_FAILED {e}");
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("ERROR {}: {e}", e.code());
            ExitCode::from(1)
        }
    }
}
