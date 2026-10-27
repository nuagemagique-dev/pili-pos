//! `pos-export-mock --listen ADDR --token-file FILE [--dir DIR]`: the test
//! receiver of the nightly export as a standalone process, for manual runs and
//! for the emulator (`http://10.0.2.2:9330`). Prints one line per PUT on
//! stdout: path, size, status. Never prints the token.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a standalone tool whose output is the console"
)]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: pos-export-mock [--listen ADDR] --token-file FILE [--dir DIR]\n  \
--listen ADDR      address to bind (default 127.0.0.1:9330)\n  \
--token-file FILE  file holding the bearer token (trimmed)\n  \
--dir DIR          also write every accepted body to DIR/<export_id>/<file>\n  \
-h, --help         this text";

struct Options {
    listen: SocketAddr,
    token_file: PathBuf,
    dir: Option<PathBuf>,
}

enum Parsed {
    Run(Options),
    Help,
}

fn parse(args: impl IntoIterator<Item = String>) -> Result<Parsed, String> {
    let mut listen: SocketAddr = "127.0.0.1:9330".parse().map_err(|e| format!("{e}"))?;
    let mut token_file = None;
    let mut dir = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "-h" | "--help" => return Ok(Parsed::Help),
            "--listen" => {
                let v = value("--listen")?;
                listen = v
                    .parse()
                    .map_err(|e| format!("--listen {v}: not an address ({e})"))?;
            }
            "--token-file" => token_file = Some(PathBuf::from(value("--token-file")?)),
            "--dir" => dir = Some(PathBuf::from(value("--dir")?)),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let token_file = token_file.ok_or("--token-file is required")?;
    Ok(Parsed::Run(Options {
        listen,
        token_file,
        dir,
    }))
}

fn run(options: &Options) -> Result<(), String> {
    let token = std::fs::read_to_string(&options.token_file)
        .map_err(|e| format!("cannot read {}: {e}", options.token_file.display()))?;
    let token = token.trim();
    if token.is_empty() {
        return Err(format!("{} is empty", options.token_file.display()));
    }
    let receiver = pos_export_mock::Receiver::start_on(options.listen, token, options.dir.clone())
        .map_err(|e| format!("cannot listen on {}: {e}", options.listen))?;
    receiver.set_observer(|path, size, status| println!("PUT /{path} {size} bytes -> {status}"));
    println!("pos-export-mock listening on {}", receiver.url());
    if let Some(dir) = &options.dir {
        println!("writing accepted bodies under {}", dir.display());
    }
    println!("Press Ctrl+C to stop");
    // The server runs on its own thread; keep this one parked until killed.
    loop {
        std::thread::park();
    }
}

fn main() -> ExitCode {
    match parse(std::env::args().skip(1)) {
        Ok(Parsed::Help) => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Parsed::Run(options)) => match run(&options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("pos-export-mock: {message}");
                ExitCode::from(1)
            }
        },
        Err(message) => {
            eprintln!("pos-export-mock: {message}\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(std::string::ToString::to_string).collect()
    }

    #[test]
    fn defaults_and_all_options() {
        let Ok(Parsed::Run(o)) = parse(args(&["--token-file", "t"])) else {
            panic!("expected options");
        };
        assert_eq!(o.listen.to_string(), "127.0.0.1:9330");
        assert_eq!(o.token_file, PathBuf::from("t"));
        assert!(o.dir.is_none());

        let Ok(Parsed::Run(o)) = parse(args(&[
            "--listen",
            "0.0.0.0:9444",
            "--token-file",
            "t",
            "--dir",
            "d",
        ])) else {
            panic!("expected options");
        };
        assert_eq!(o.listen.to_string(), "0.0.0.0:9444");
        assert_eq!(o.dir, Some(PathBuf::from("d")));
    }

    #[test]
    fn errors() {
        assert!(parse(args(&[])).is_err());
        assert!(parse(args(&["--token-file"])).is_err());
        assert!(parse(args(&["--token-file", "t", "--listen", "nope"])).is_err());
        assert!(parse(args(&["--token-file", "t", "--wat"])).is_err());
        assert!(matches!(parse(args(&["--help"])), Ok(Parsed::Help)));
    }
}
