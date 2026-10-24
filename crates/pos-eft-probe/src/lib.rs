//! The logic of `pos-eft-probe`: the four commands against a
//! [`Terminal`], the YES prompt, the masked trace file.
//!
//! Factored out of the binary so the tests drive it against
//! `pos-eft-mock` with a scripted stdin.

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use pos_core::config::{PaymentTerminalConfig, PaymentTerminalKind};
use pos_eft::{EftError, Outcome, PaymentRequest, Terminal, mask_pan};
use tracing::{info, warn};
use tracing_subscriber::Layer;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::layer::SubscriberExt;

/// What to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Reachability only; nothing is sent that could charge.
    Check,
    /// One payment of `amount_cents`, after the operator typed YES.
    Pay { amount_cents: i64 },
    /// The outcome of an earlier reference.
    Status { reference: String },
    /// Stop the payment `reference`.
    Abort { reference: String },
}

/// What the command line asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub terminal: PaymentTerminalConfig,
    pub trace: PathBuf,
    pub command: Command,
}

/// The usage text.
pub const USAGE: &str = "\
usage: pos-eft-probe [terminal] [--trace FILE] [COMMAND]

terminal, either:
  --config POS.JSON            read the payment_terminal section of a pos.json
  --kind ccv_opi --address HOST:4100 [--device-port 4102]
  --kind ccv_payment_api --terminal-id ID --management-system-id ID
        [--base-url URL] [--api-key-file FILE]
  --kind worldline_terminal_api --merchant-id ID --terminal-id ID
        --integrator-id ID [--base-url URL] [--api-key-file FILE]
  [--timeout-seconds N]        longest wait for a payment (30 to 600)

commands:
  check                        reachability only (default)
  pay --amount 0.01            asks to type YES, then charges the card
  status REFERENCE             outcome of an earlier payment
  abort REFERENCE              stop a payment in progress

--trace FILE                   masked wire trace and logs
                               (default eft-trace-<UTC timestamp>.log)
";

/// Read the command line (without the program name). `Err` carries the
/// message to print, with the usage for `--help`.
pub fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut flags: Vec<(String, String)> = Vec::new();
    let mut positional: Vec<String> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--help" || a == "-h" {
            return Err(USAGE.to_string());
        }
        if let Some(name) = a.strip_prefix("--") {
            let value = it
                .next()
                .ok_or_else(|| format!("--{name}: a value is expected"))?;
            flags.push((name.to_string(), value.clone()));
        } else {
            positional.push(a.clone());
        }
    }
    let take = |flags: &mut Vec<(String, String)>, name: &str| -> Option<String> {
        flags
            .iter()
            .position(|(n, _)| n == name)
            .map(|i| flags.remove(i).1)
    };
    let trace = take(&mut flags, "trace")
        .map_or_else(|| PathBuf::from(default_trace_name()), PathBuf::from);
    let amount = take(&mut flags, "amount");
    let config_path = take(&mut flags, "config");
    let terminal = if let Some(path) = config_path {
        if !flags.is_empty() {
            return Err("--config cannot be combined with terminal flags".into());
        }
        let path = PathBuf::from(path);
        let mut config = pos_core::config::PosConfig::load_from_file(&path)
            .map_err(|e| format!("--config: {e}"))?;
        config.resolve_relative_to(path.parent().unwrap_or(Path::new("")));
        config
            .payment_terminal
            .ok_or("--config: the file has no payment_terminal section")?
    } else {
        let kind = take(&mut flags, "kind").ok_or("--config or --kind is required")?;
        let kind = match kind.as_str() {
            "ccv_opi" => PaymentTerminalKind::CcvOpi,
            "ccv_payment_api" => PaymentTerminalKind::CcvPaymentApi,
            "worldline_terminal_api" => PaymentTerminalKind::WorldlineTerminalApi,
            other => return Err(format!("--kind: unknown kind {other:?}")),
        };
        let device_port = take(&mut flags, "device-port")
            .map(|p| {
                p.parse::<u16>()
                    .map_err(|_| format!("--device-port: a port expected, got {p:?}"))
            })
            .transpose()?;
        let timeout_seconds = take(&mut flags, "timeout-seconds")
            .map(|p| {
                p.parse::<u64>()
                    .map_err(|_| format!("--timeout-seconds: a number expected, got {p:?}"))
            })
            .transpose()?
            .unwrap_or(180);
        let config = PaymentTerminalConfig {
            kind,
            base_url: take(&mut flags, "base-url"),
            api_key_file: take(&mut flags, "api-key-file"),
            merchant_id: take(&mut flags, "merchant-id"),
            terminal_id: take(&mut flags, "terminal-id"),
            integrator_id: take(&mut flags, "integrator-id"),
            management_system_id: take(&mut flags, "management-system-id"),
            access_protocol: take(&mut flags, "access-protocol"),
            address: take(&mut flags, "address"),
            device_port,
            timeout_seconds,
        };
        if let Some((name, _)) = flags.first() {
            return Err(format!("--{name}: unknown option"));
        }
        config
    };
    let mut words = positional.into_iter();
    let command = match words.next().as_deref() {
        None | Some("check") => Command::Check,
        Some("pay") => Command::Pay {
            amount_cents: parse_amount(amount.as_deref().ok_or("pay: --amount is required")?)?,
        },
        Some("status") => Command::Status {
            reference: words.next().ok_or("status: a reference is required")?,
        },
        Some("abort") => Command::Abort {
            reference: words.next().ok_or("abort: a reference is required")?,
        },
        Some(other) => return Err(format!("unknown command {other:?}")),
    };
    if let Some(extra) = words.next() {
        return Err(format!("unexpected argument {extra:?}"));
    }
    if amount.is_some() && !matches!(command, Command::Pay { .. }) {
        return Err("--amount only goes with pay".into());
    }
    Ok(Options {
        terminal,
        trace,
        command,
    })
}

/// Process exit codes.
pub const EXIT_OK: u8 = 0;
/// Declined, cancelled, unknown, unreachable, rejected.
pub const EXIT_FAILED: u8 = 1;
/// The operator did not type YES: nothing was sent.
pub const EXIT_NOT_CONFIRMED: u8 = 2;

/// `amount` ("0.01", "12,5", "3") in euro cents; at most two decimals,
/// more than zero.
pub fn parse_amount(text: &str) -> Result<i64, String> {
    let bad = || format!("amount: a positive euro amount such as 0.01 expected, got {text:?}");
    let t = text.trim();
    let (whole, frac) = t.split_once(['.', ',']).unwrap_or((t, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if (whole.is_empty() && frac.is_empty()) || !digits(whole) || !digits(frac) || frac.len() > 2 {
        return Err(bad());
    }
    let whole: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().map_err(|_| bad())?
    };
    let frac: i64 = format!("{frac:0<2}").parse().map_err(|_| bad())?;
    let cents = whole
        .checked_mul(100)
        .and_then(|c| c.checked_add(frac))
        .ok_or_else(bad)?;
    if cents <= 0 {
        return Err(bad());
    }
    Ok(cents)
}

/// Where the terminal is, in words, for the operator to check before
/// typing YES.
pub fn describe(config: &PaymentTerminalConfig) -> String {
    let kind = config.kind.as_str();
    match config.kind {
        PaymentTerminalKind::CcvOpi => format!(
            "{kind} at {} (channel 1 on port {})",
            config.address.as_deref().unwrap_or("?"),
            config.device_port.unwrap_or(4102)
        ),
        _ => format!(
            "{kind} through {} (terminal {})",
            config
                .base_url
                .as_deref()
                .unwrap_or("the vendor's production API"),
            config.terminal_id.as_deref().unwrap_or("?")
        ),
    }
}

/// The subscriber that writes the masked wire trace and the info logs
/// of `pos-eft` and of the probe to `path`, without colours. Other
/// crates' events (an HTTP stack may log headers) are not written.
pub fn trace_subscriber(
    path: &Path,
) -> io::Result<impl tracing::Subscriber + Send + Sync + 'static> {
    let file = std::fs::File::create(path)?;
    let filter = Targets::new()
        .with_default(LevelFilter::OFF)
        .with_target("pos_eft", LevelFilter::INFO)
        .with_target("pos_eft::wire", LevelFilter::TRACE)
        .with_target("pos_eft_probe", LevelFilter::INFO);
    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(std::sync::Mutex::new(file))
        .with_filter(filter);
    Ok(tracing_subscriber::registry().with(layer))
}

/// A UTC timestamp for the default trace file name (`20261013T101500Z`).
pub fn default_trace_name() -> String {
    format!(
        "eft-trace-{}.log",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ")
    )
}

fn show_outcome(out: &mut dyn Write, outcome: &Outcome) -> io::Result<()> {
    let json = serde_json::to_string_pretty(outcome).unwrap_or_else(|e| e.to_string());
    writeln!(out, "Outcome: {}", mask_pan(&json))
}

fn show_error(out: &mut dyn Write, what: &str, e: &EftError) -> io::Result<()> {
    warn!("{what}: {e}");
    writeln!(out, "{what}: {e}")
}

/// Run `command` on `terminal` (described by `target`). `input` is where
/// the operator's YES comes from; `out` is where everything is
/// printed. Returns the process exit code.
pub async fn run(
    command: &Command,
    terminal: &Terminal,
    target: &str,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
) -> io::Result<u8> {
    match command {
        Command::Check => match terminal.check().await {
            Ok(()) => {
                info!("check ok: {target}");
                writeln!(out, "Reachable: {target}")?;
                Ok(EXIT_OK)
            }
            Err(e) => {
                show_error(out, "Not reachable", &e)?;
                Ok(EXIT_FAILED)
            }
        },
        Command::Pay { amount_cents } => {
            let reference = uuid::Uuid::new_v4().to_string();
            writeln!(out, "Reference: {reference}")?;
            writeln!(out, "Terminal:  {target}")?;
            writeln!(
                out,
                "Amount:    EUR {}",
                pos_eft::amount_text(*amount_cents)
            )?;
            writeln!(
                out,
                "A real card will be charged on the terminal. Type YES to send the payment:"
            )?;
            out.flush()?;
            let mut line = String::new();
            input.read_line(&mut line)?;
            if line.trim() != "YES" {
                info!(%reference, "pay not confirmed, nothing sent");
                writeln!(out, "Not confirmed. Nothing was sent.")?;
                return Ok(EXIT_NOT_CONFIRMED);
            }
            info!(%reference, amount_cents, "pay confirmed by the operator");
            let request = PaymentRequest {
                reference: reference.clone(),
                amount_cents: *amount_cents,
                description: "pos-eft-probe".into(),
                language: "fr".into(),
            };
            match terminal.pay(&request).await {
                Ok(outcome) => {
                    info!(%reference, outcome = %mask_pan(&format!("{outcome:?}")), "pay answered");
                    show_outcome(out, &outcome)?;
                    Ok(if matches!(outcome, Outcome::Approved(_)) {
                        EXIT_OK
                    } else {
                        EXIT_FAILED
                    })
                }
                Err(e) => {
                    show_error(out, "Payment not made", &e)?;
                    Ok(EXIT_FAILED)
                }
            }
        }
        Command::Status { reference } => match terminal.status(reference).await {
            Ok(outcome) => {
                info!(%reference, outcome = %mask_pan(&format!("{outcome:?}")), "status answered");
                show_outcome(out, &outcome)?;
                Ok(EXIT_OK)
            }
            Err(e) => {
                show_error(out, "Status failed", &e)?;
                Ok(EXIT_FAILED)
            }
        },
        Command::Abort { reference } => match terminal.abort(reference).await {
            Ok(()) => {
                info!(%reference, "abort sent");
                writeln!(out, "Abort sent for {reference}")?;
                Ok(EXIT_OK)
            }
            Err(e) => {
                show_error(out, "Abort failed", &e)?;
                Ok(EXIT_FAILED)
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_are_read_in_cents() {
        assert_eq!(parse_amount("0.01"), Ok(1));
        assert_eq!(parse_amount("12,5"), Ok(1250));
        assert_eq!(parse_amount("3"), Ok(300));
        assert_eq!(parse_amount(".5"), Ok(50));
        for bad in ["0", "0.00", "-1", "1.234", "abc", "", "1.2.3"] {
            assert!(parse_amount(bad).is_err(), "{bad}");
        }
    }
}

#[cfg(test)]
mod arg_tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn flags_build_an_opi_terminal_and_check_is_the_default() {
        let o = parse_args(&args(&[
            "--kind",
            "ccv_opi",
            "--address",
            "192.168.1.50:4100",
            "--device-port",
            "4102",
            "--trace",
            "t.log",
        ]))
        .unwrap();
        assert_eq!(o.command, Command::Check);
        assert_eq!(o.trace, PathBuf::from("t.log"));
        assert_eq!(o.terminal.kind, PaymentTerminalKind::CcvOpi);
        assert_eq!(o.terminal.address.as_deref(), Some("192.168.1.50:4100"));
        assert_eq!(o.terminal.device_port, Some(4102));
        assert!(o.terminal.problems().is_empty());
        assert!(describe(&o.terminal).contains("192.168.1.50:4100"));
    }

    #[test]
    fn pay_needs_an_amount_and_the_others_a_reference() {
        let base = ["--kind", "ccv_opi", "--address", "127.0.0.1:1"];
        let with = |extra: &[&str]| {
            let mut a = args(&base);
            a.extend(args(extra));
            parse_args(&a)
        };
        assert_eq!(
            with(&["pay", "--amount", "0.01"]).unwrap().command,
            Command::Pay { amount_cents: 1 }
        );
        assert!(with(&["pay"]).is_err());
        assert!(with(&["pay", "--amount", "0"]).is_err());
        assert_eq!(
            with(&["status", "abc"]).unwrap().command,
            Command::Status {
                reference: "abc".into()
            }
        );
        assert!(with(&["abort"]).is_err());
        assert!(with(&["check", "--amount", "1"]).is_err());
        assert!(with(&["frobnicate"]).is_err());
        assert!(with(&["--bogus", "1"]).is_err());
    }

    #[test]
    fn a_missing_terminal_or_a_help_request_is_an_error_with_text() {
        assert!(parse_args(&args(&["check"])).is_err());
        assert!(
            parse_args(&args(&["--help"]))
                .unwrap_err()
                .contains("usage")
        );
        assert!(parse_args(&args(&["--kind", "nope"])).is_err());
        assert!(parse_args(&args(&["--config", "/nonexistent/pos.json"])).is_err());
    }
}
