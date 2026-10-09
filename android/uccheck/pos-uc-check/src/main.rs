use std::process::ExitCode;

use pos_uc_check::cli::{self, Command};
use pos_uc_check::{compare, record};

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match cli::parse(&args) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("pos-uc-check: {message}\n\n{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };
    match command {
        Command::Help => {
            print!("{}", cli::USAGE);
            ExitCode::SUCCESS
        }
        Command::Record(options) => match record::record(&options).await {
            Ok(recorded) => {
                let (lines, pass) = record::summary_lines(&recorded);
                for line in lines {
                    println!("{line}");
                }
                exit(pass)
            }
            Err(e) => {
                eprintln!("pos-uc-check record: {e:#}");
                println!(
                    "UCREC | total | FAIL | error: {}",
                    one_line(&format!("{e:#}"))
                );
                ExitCode::FAILURE
            }
        },
        Command::Compare {
            linux,
            android,
            report,
            options,
        } => match compare::compare_dirs(&linux, &android, &options) {
            Ok(results) => {
                let (lines, pass) = compare::summary_lines(&results);
                for line in &lines {
                    println!("{line}");
                }
                for result in &results {
                    for detail in result.detail_lines() {
                        eprintln!("{detail}");
                    }
                }
                if let Some(path) = report {
                    let generated =
                        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                    let body = compare::report(&results, &generated);
                    let written = serde_json::to_string_pretty(&body)
                        .map_err(anyhow::Error::from)
                        .and_then(|text| {
                            if let Some(parent) =
                                path.parent().filter(|p| !p.as_os_str().is_empty())
                            {
                                std::fs::create_dir_all(parent)?;
                            }
                            std::fs::write(&path, text + "\n").map_err(anyhow::Error::from)
                        });
                    if let Err(e) = written {
                        eprintln!("pos-uc-check compare: cannot write the report: {e:#}");
                        return ExitCode::FAILURE;
                    }
                }
                exit(pass)
            }
            Err(e) => {
                eprintln!("pos-uc-check compare: {e:#}");
                println!(
                    "UCCHECK | total | FAIL | error: {}",
                    one_line(&format!("{e:#}"))
                );
                ExitCode::FAILURE
            }
        },
    }
}

fn exit(pass: bool) -> ExitCode {
    if pass {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
