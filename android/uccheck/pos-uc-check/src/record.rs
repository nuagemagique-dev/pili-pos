//! `pos-uc-check record`: play the scenario groups with the engine on the
//! host (the reference run) and write one journal per scenario.
//!
//! A group is a directory of scenarios that share one fresh database
//! (P7D7): each group gets a fresh data directory, a fresh engine and
//! fresh mocks. Between two scenarios of a group the host clears the
//! mocks' request log and job list over HTTP, as the Android host script
//! does.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::bail;
use serde_json::Value;

use crate::host::Host;
use crate::journal::{FdmCall, Journal, JournalStep};
use crate::scenario::{Scenario, check_expect, group_dirs, scenario_files, substitute};

/// Calls the FDM mock keeps: a full log means calls were dropped.
const FDM_LOG_SIZE: usize = 100;

#[derive(Debug, Clone)]
pub struct RecordOptions {
    pub scenarios: PathBuf,
    pub out: PathBuf,
    /// Group directory names to run; empty: all of them.
    pub groups: Vec<String>,
    /// A scenario file stem to run; none: all of the group.
    pub only: Option<String>,
    /// The repository root (root Cargo.toml and the seed).
    pub repo: PathBuf,
}

/// One scenario's outcome, for the output lines.
#[derive(Debug, Clone)]
pub struct RecordedScenario {
    pub group: String,
    pub scenario: String,
    pub verdict: String,
    pub steps: usize,
    pub fdm_calls: usize,
    pub tickets: usize,
}

impl RecordedScenario {
    pub fn line(&self) -> String {
        format!(
            "UCREC | {}/{} | {} | {} steps, {} fdm calls, {} tickets",
            self.group, self.scenario, self.verdict, self.steps, self.fdm_calls, self.tickets
        )
    }
}

/// Play the selected groups. Returns every scenario's outcome; the
/// journals are written under `out/linux/<group>/`.
pub async fn record(opts: &RecordOptions) -> anyhow::Result<Vec<RecordedScenario>> {
    let mut dirs = group_dirs(&opts.scenarios)?;
    if !opts.groups.is_empty() {
        for wanted in &opts.groups {
            if !dirs
                .iter()
                .any(|d| d.file_name().is_some_and(|n| n == wanted.as_str()))
            {
                bail!("no group {wanted} in {}", opts.scenarios.display());
            }
        }
        dirs.retain(|d| {
            d.file_name()
                .is_some_and(|n| opts.groups.iter().any(|g| n == g.as_str()))
        });
    }

    let mut recorded = Vec::new();
    for dir in dirs {
        let group = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut files = scenario_files(&dir)?;
        if let Some(only) = &opts.only {
            files.retain(|f| f.file_stem().is_some_and(|s| s == only.as_str()));
        }
        if files.is_empty() {
            continue;
        }

        // Fresh database, fresh engine, fresh mocks for the group.
        let host = Host::start(&opts.repo).await?;
        let mut outcome = Ok(());
        for file in files {
            let stem = file
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let result: anyhow::Result<RecordedScenario> = async {
                let scenario = Scenario::load(&file)?;
                let journal = run_scenario(&host, &group, &stem, &scenario).await?;
                journal.write(&opts.out)?;
                Ok(RecordedScenario {
                    group: group.clone(),
                    scenario: stem.clone(),
                    verdict: journal.verdict.clone(),
                    steps: journal.steps.len(),
                    fdm_calls: journal.fdm.len(),
                    tickets: journal.tickets.len(),
                })
            }
            .await;
            match result {
                Ok(done) => recorded.push(done),
                Err(e) => {
                    outcome = Err(e.context(format!("{group}/{stem}")));
                    break;
                }
            }
        }
        host.stop().await;
        outcome?;
    }
    if recorded.is_empty() {
        bail!(
            "no scenario to run in {} (groups {:?}, only {:?})",
            opts.scenarios.display(),
            opts.groups,
            opts.only
        );
    }
    Ok(recorded)
}

/// Play one scenario on a running host and build its journal.
pub async fn run_scenario(
    host: &Host,
    group: &str,
    stem: &str,
    scenario: &Scenario,
) -> anyhow::Result<Journal> {
    host.reset_mocks().await?;

    let mut saved: HashMap<String, Value> = HashMap::new();
    let mut steps = Vec::new();
    let mut failed = false;
    'steps: for step in &scenario.steps {
        for _ in 0..step.repeat {
            let recorded = match substitute(&step.args, &saved) {
                Err(e) => JournalStep {
                    cmd: step.cmd.clone(),
                    ok: false,
                    error: Some(format!("uc_bad_args|detail={e}")),
                    result: Value::Null,
                    unmet: Some(format!("{e:#}")),
                },
                Ok(args) => {
                    let outcome = host.call(&step.cmd, &args).await;
                    let unmet = check_expect(&step.expect, &outcome).err();
                    if let (Some(name), Ok(value)) = (&step.save, &outcome) {
                        saved.insert(name.clone(), value.clone());
                    }
                    match outcome {
                        Ok(result) => JournalStep {
                            cmd: step.cmd.clone(),
                            ok: true,
                            error: None,
                            result,
                            unmet,
                        },
                        Err(error) => JournalStep {
                            cmd: step.cmd.clone(),
                            ok: false,
                            error: Some(error),
                            result: Value::Null,
                            unmet,
                        },
                    }
                }
            };
            let stop = recorded.unmet.is_some();
            steps.push(recorded);
            if stop {
                failed = true;
                break 'steps;
            }
        }
    }

    host.settle().await?;
    let calls = host.fdm_requests().await?;
    if calls.len() >= FDM_LOG_SIZE {
        bail!(
            "{group}/{stem}: the FDM mock log is full ({FDM_LOG_SIZE} calls); split the scenario"
        );
    }
    let mut fdm = Vec::new();
    let mut status_calls = Vec::new();
    for call in calls {
        let entry = FdmCall {
            operation: call["operation"].as_str().unwrap_or_default().to_string(),
            request: call["request"].clone(),
            response: call["response"].clone(),
        };
        if entry.operation == "Status" {
            status_calls.push(entry);
        } else {
            fdm.push(entry);
        }
    }
    let tickets = host
        .printer_jobs()
        .await?
        .iter()
        .map(|job| job["text"].as_str().unwrap_or_default().to_string())
        .collect();

    let ids = if scenario.ids.is_empty() && scenario.id != "SMOKE" {
        vec![scenario.id.clone()]
    } else {
        scenario.ids.clone()
    };
    Ok(Journal {
        format: 1,
        side: "linux".to_string(),
        group: group.to_string(),
        scenario: stem.to_string(),
        ids,
        pos_sw_version: host.sw_version.clone(),
        steps,
        fdm,
        status_calls,
        tickets,
        verdict: if failed { "FAIL" } else { "PASS" }.to_string(),
    })
}

/// The lines `record` prints: one per scenario, then the total.
pub fn summary_lines(recorded: &[RecordedScenario]) -> (Vec<String>, bool) {
    let failed = recorded.iter().filter(|r| r.verdict != "PASS").count();
    let mut lines: Vec<String> = recorded.iter().map(RecordedScenario::line).collect();
    lines.push(format!(
        "UCREC | total | {} | {failed} failed ({} scenarios)",
        if failed == 0 { "PASS" } else { "FAIL" },
        recorded.len()
    ));
    (lines, failed == 0)
}
