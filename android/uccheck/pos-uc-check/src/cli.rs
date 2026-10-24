//! Command line of `pos-uc-check`, parsed by hand.

use std::path::PathBuf;

use crate::compare::Options as CompareOptions;
use crate::record::RecordOptions;

pub const USAGE: &str = "\
pos-uc-check: the host side of the P7 end-to-end parity proof

usage:
  pos-uc-check record --scenarios DIR --out DIR [--group G]... [--only STEM] [--repo DIR]
      Play the scenario groups of DIR with the engine on the host (the reference run) and
      write OUT/linux/<group>/<scenario>.json. Prints 'UCREC | ...' lines, ends with
      'UCREC | total | PASS|FAIL | ...'. Exit 1 on FAIL.
  pos-uc-check compare --linux DIR --android DIR [--fixtures] [--no-counters] [--same-side]
                       [--report FILE] [--repo DIR] [--scenarios DIR]
      Compare the journals of two directories (<group>/<scenario>.json). Prints
      'UCCHECK | ...' lines, ends with 'UCCHECK | total | PASS|FAIL | ...'. Exit 1 on FAIL.
      --fixtures    also check the requests against the official fixtures
      --no-counters skip the counters check (live mode)
      --same-side   both directories hold journals of the same side (host run against host run)
      --report FILE write the results as JSON
      --scenarios   scenario files (default <repo>/android/app/src/androidTest/assets/uc)
  --repo DIR defaults to the current directory (the repository root).
Exit: 0 PASS, 1 FAIL or run error, 2 usage error.
";

#[derive(Debug, Clone)]
pub enum Command {
    Record(RecordOptions),
    Compare {
        linux: PathBuf,
        android: PathBuf,
        report: Option<PathBuf>,
        options: CompareOptions,
    },
    Help,
}

/// Parse the arguments after the program name.
pub fn parse(args: &[String]) -> Result<Command, String> {
    let Some((command, rest)) = args.split_first() else {
        return Err("a command is required (record or compare)".to_string());
    };
    match command.as_str() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "record" => parse_record(rest),
        "compare" => parse_compare(rest),
        other => Err(format!("unknown command {other}")),
    }
}

fn value<'a>(flag: &str, rest: &'a [String], i: &mut usize) -> Result<&'a str, String> {
    *i += 1;
    rest.get(*i)
        .map(String::as_str)
        .ok_or_else(|| format!("{flag} needs a value"))
}

fn parse_record(rest: &[String]) -> Result<Command, String> {
    let mut scenarios = None;
    let mut out = None;
    let mut groups = Vec::new();
    let mut only = None;
    let mut repo = PathBuf::from(".");
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--scenarios" => scenarios = Some(PathBuf::from(value("--scenarios", rest, &mut i)?)),
            "--out" => out = Some(PathBuf::from(value("--out", rest, &mut i)?)),
            "--group" => groups.push(value("--group", rest, &mut i)?.to_string()),
            "--only" => only = Some(value("--only", rest, &mut i)?.to_string()),
            "--repo" => repo = PathBuf::from(value("--repo", rest, &mut i)?),
            other => return Err(format!("record: unknown argument {other}")),
        }
        i += 1;
    }
    Ok(Command::Record(RecordOptions {
        scenarios: scenarios.ok_or("record: --scenarios is required")?,
        out: out.ok_or("record: --out is required")?,
        groups,
        only,
        repo,
    }))
}

fn parse_compare(rest: &[String]) -> Result<Command, String> {
    let mut linux = None;
    let mut android = None;
    let mut report = None;
    let mut options = CompareOptions {
        repo: PathBuf::from("."),
        ..CompareOptions::default()
    };
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--linux" => linux = Some(PathBuf::from(value("--linux", rest, &mut i)?)),
            "--android" => android = Some(PathBuf::from(value("--android", rest, &mut i)?)),
            "--report" => report = Some(PathBuf::from(value("--report", rest, &mut i)?)),
            "--repo" => options.repo = PathBuf::from(value("--repo", rest, &mut i)?),
            "--scenarios" => {
                options.scenarios = Some(PathBuf::from(value("--scenarios", rest, &mut i)?));
            }
            "--fixtures" => options.fixtures = true,
            "--no-counters" => options.no_counters = true,
            "--same-side" => options.same_side = true,
            other => return Err(format!("compare: unknown argument {other}")),
        }
        i += 1;
    }
    Ok(Command::Compare {
        linux: linux.ok_or("compare: --linux is required")?,
        android: android.ok_or("compare: --android is required")?,
        report,
        options,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn record_takes_repeated_groups_and_defaults_the_repo() {
        let cmd = parse(&args(&[
            "record",
            "--scenarios",
            "s",
            "--out",
            "o",
            "--group",
            "00_smoke",
            "--group",
            "10_basic",
            "--only",
            "001_SMOKE",
        ]))
        .unwrap();
        let Command::Record(opts) = cmd else {
            panic!("record expected");
        };
        assert_eq!(opts.groups, ["00_smoke", "10_basic"]);
        assert_eq!(opts.only.as_deref(), Some("001_SMOKE"));
        assert_eq!(opts.repo, PathBuf::from("."));
    }

    #[test]
    fn compare_takes_its_flags() {
        let cmd = parse(&args(&[
            "compare",
            "--linux",
            "a",
            "--android",
            "b",
            "--fixtures",
            "--no-counters",
            "--same-side",
            "--report",
            "r.json",
        ]))
        .unwrap();
        let Command::Compare {
            report, options, ..
        } = cmd
        else {
            panic!("compare expected");
        };
        assert!(options.fixtures && options.no_counters && options.same_side);
        assert_eq!(report, Some(PathBuf::from("r.json")));
    }

    #[test]
    fn missing_values_and_unknown_arguments_are_usage_errors() {
        assert!(parse(&[]).is_err());
        assert!(parse(&args(&["frobnicate"])).is_err());
        assert!(parse(&args(&["record", "--out", "o"])).is_err());
        assert!(parse(&args(&["record", "--scenarios"])).is_err());
        assert!(parse(&args(&["compare", "--linux", "a"])).is_err());
        assert!(
            parse(&args(&[
                "compare",
                "--linux",
                "a",
                "--android",
                "b",
                "--bogus"
            ]))
            .is_err()
        );
        assert!(matches!(parse(&args(&["--help"])), Ok(Command::Help)));
    }
}
