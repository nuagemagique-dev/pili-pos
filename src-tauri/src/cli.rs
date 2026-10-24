//! Offline command-line entry points of the `pili-pos` binary.
//!
//! Without arguments the binary starts the kiosk UI. The commands below
//! run without a display and exit; they are used by the image build and
//! by `deploy/install.sh`:
//!
//! - `pili-pos --version`
//! - `pili-pos --init-db [--seed <file.sql>]` - create or migrate the
//!   database at `paths::db_path()` and optionally load a catalogue seed.
//! - `pili-pos --check-config` - read `pos.json` as the kiosk would and
//!   list what the FDM would refuse; exits 1 when anything is wrong.

use std::path::PathBuf;

use anyhow::bail;

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    RunUi,
    Version,
    InitDb { seed: Option<PathBuf> },
    CheckConfig,
}

pub fn parse(args: &[String]) -> anyhow::Result<Command> {
    match args {
        [] => Ok(Command::RunUi),
        [flag] if flag == "--version" => Ok(Command::Version),
        [flag] if flag == "--init-db" => Ok(Command::InitDb { seed: None }),
        [flag] if flag == "--check-config" => Ok(Command::CheckConfig),
        [flag, seed_flag, seed] if flag == "--init-db" && seed_flag == "--seed" => {
            Ok(Command::InitDb {
                seed: Some(PathBuf::from(seed)),
            })
        }
        _ => bail!(
            "usage: pili-pos [--version | --check-config | --init-db [--seed <file.sql>]] (got: {})",
            args.join(" ")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_supported_commands() {
        assert_eq!(parse(&[]).unwrap(), Command::RunUi);
        assert_eq!(parse(&args(&["--version"])).unwrap(), Command::Version);
        assert_eq!(
            parse(&args(&["--check-config"])).unwrap(),
            Command::CheckConfig
        );
        assert_eq!(
            parse(&args(&["--init-db"])).unwrap(),
            Command::InitDb { seed: None }
        );
        assert_eq!(
            parse(&args(&["--init-db", "--seed", "seed.sql"])).unwrap(),
            Command::InitDb {
                seed: Some(PathBuf::from("seed.sql"))
            }
        );
    }

    #[test]
    fn rejects_unknown_arguments() {
        assert!(parse(&args(&["--seed", "seed.sql"])).is_err());
        assert!(parse(&args(&["--init-db", "seed.sql"])).is_err());
        assert!(parse(&args(&["--check-config", "pos.json"])).is_err());
    }
}
