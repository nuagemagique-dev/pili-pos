//! `pos-eft-probe`: one manual test of the card terminal link, see the
//! README. Never sends a payment without the operator typing YES.

use std::io::{self, BufRead};
use std::process::ExitCode;

use pos_eft::Terminal;
use pos_eft_probe::{EXIT_FAILED, Options, describe, parse_args, run, trace_subscriber};

const SOFTWARE: &str = concat!("pos-eft-probe/", env!("CARGO_PKG_VERSION"));

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Options {
        terminal: config,
        trace,
        command,
    } = match parse_args(&args) {
        Ok(o) => o,
        Err(message) => {
            eprintln!("{}", message.trim_end());
            return ExitCode::from(2);
        }
    };
    match trace_subscriber(&trace) {
        Ok(subscriber) => {
            if tracing::subscriber::set_global_default(subscriber).is_err() {
                eprintln!("pos-eft-probe: cannot start the trace");
                return ExitCode::from(EXIT_FAILED);
            }
        }
        Err(e) => {
            eprintln!(
                "pos-eft-probe: cannot write the trace {}: {e}",
                trace.display()
            );
            return ExitCode::from(EXIT_FAILED);
        }
    }
    let terminal = match Terminal::new(&config, reqwest::Client::new(), SOFTWARE) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("pos-eft-probe: {e}");
            return ExitCode::from(EXIT_FAILED);
        }
    };
    println!("Trace: {}", trace.display());
    let stdin = io::stdin();
    let mut input: Box<dyn BufRead> = Box::new(stdin.lock());
    let mut out = io::stdout();
    match run(
        &command,
        &terminal,
        &describe(&config),
        &mut input,
        &mut out,
    )
    .await
    {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("pos-eft-probe: {e}");
            ExitCode::from(EXIT_FAILED)
        }
    }
}
