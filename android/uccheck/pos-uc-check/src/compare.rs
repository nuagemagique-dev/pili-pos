//! `pos-uc-check compare`: the checks of P7.md section 4.5 between the
//! journals of the reference run and the ones of the Android run.
//!
//! The comparison reads requests and tickets. It computes no amount and
//! holds no fiscal rule: `business_view` and `diff` are the ones the
//! Linux fixture tests use (`pos-engine-testkit`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Context;
use pos_engine_testkit::compare::{business_view, diff};
use serde_json::{Value, json};

use crate::journal::{self, FdmCall, Journal};
use crate::scenario::Scenario;
use crate::ticket::normalise_ticket;
use crate::volatile::strip_volatile;

/// The checks, in the order they are printed.
pub const CHECKS: [&str; 9] = [
    "steps",
    "fdm-ops",
    "fdm-business",
    "fdm-counters",
    "fdm-full",
    "sw-version",
    "tickets",
    "fixture",
    "status-shape",
];

#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Check the requests against the official fixtures.
    pub fixtures: bool,
    /// Skip the counters check (live mode: the simulator's counters are not ours).
    pub no_counters: bool,
    /// Both directories hold journals of the same side (a host run against a second host run).
    pub same_side: bool,
    /// The repository root, for the fixture directories and the scenario files.
    pub repo: PathBuf,
    /// Where the scenario files are; default `<repo>/android/app/src/androidTest/assets/uc`.
    pub scenarios: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pass,
    Fail,
    Skip,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Pass => "PASS",
            Status::Fail => "FAIL",
            Status::Skip => "SKIP",
        }
    }
}

/// One check's outcome with what differs (empty on PASS and SKIP).
#[derive(Debug, Clone)]
pub struct Outcome {
    pub status: Status,
    pub details: Vec<String>,
}

impl Outcome {
    fn pass() -> Self {
        Self {
            status: Status::Pass,
            details: Vec::new(),
        }
    }
    fn skip(why: &str) -> Self {
        Self {
            status: Status::Skip,
            details: vec![why.to_string()],
        }
    }
    fn from_problems(problems: Vec<String>) -> Self {
        if problems.is_empty() {
            Self::pass()
        } else {
            Self {
                status: Status::Fail,
                details: problems,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScenarioResult {
    pub group: String,
    pub scenario: String,
    pub ids: Vec<String>,
    pub checks: BTreeMap<&'static str, Outcome>,
}

impl ScenarioResult {
    pub fn passed(&self) -> bool {
        self.checks.values().all(|o| o.status != Status::Fail)
    }

    pub fn line(&self) -> String {
        let checks: Vec<String> = CHECKS
            .iter()
            .filter_map(|c| {
                self.checks
                    .get(c)
                    .map(|o| format!("{c}:{}", o.status.as_str()))
            })
            .collect();
        format!(
            "UCCHECK | {}/{} | {} | {}",
            self.group,
            self.scenario,
            if self.passed() { "PASS" } else { "FAIL" },
            checks.join(" ")
        )
    }

    /// What differs, one line per problem, for the people who read a FAIL.
    pub fn detail_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for check in CHECKS {
            if let Some(outcome) = self.checks.get(check)
                && outcome.status == Status::Fail
            {
                for detail in &outcome.details {
                    lines.push(format!(
                        "  {}/{} {check}: {detail}",
                        self.group, self.scenario
                    ));
                }
            }
        }
        lines
    }
}

// ── The request and response shapes ────────────────────────────────

/// The data objects of a request as the mock logged it: `variables.data`
/// for a sign operation, or every object of `variables` but `isTraining`
/// (the Z report signs the turnover and the user report in one operation:
/// `turnover` and `user`), sorted by name.
fn request_parts(variables: &Value) -> Vec<(String, &Value)> {
    if let Some(data) = variables.get("data") {
        return vec![("data".to_string(), data)];
    }
    let mut parts: Vec<(String, &Value)> = variables
        .as_object()
        .map(|map| {
            map.iter()
                .filter(|(k, v)| k.as_str() != "isTraining" && v.is_object())
                .map(|(k, v)| (k.clone(), v))
                .collect()
        })
        .unwrap_or_default();
    if parts.is_empty() {
        parts.push(("variables".to_string(), variables));
    }
    parts.sort_by(|a, b| a.0.cmp(&b.0));
    parts
}

/// The results of an answer: `(result name, fdmRef)` for every result
/// that has an `fdmRef`, sorted by name (a Z answers with two).
fn answer_refs(response: &Value) -> Vec<(String, &Value)> {
    let mut refs: Vec<(String, &Value)> = response["data"]
        .as_object()
        .map(|map| {
            map.iter()
                .filter_map(|(name, result)| result.get("fdmRef").map(|r| (name.clone(), r)))
                .collect()
        })
        .unwrap_or_default();
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    refs
}

/// (result, eventLabel, eventCounter, totalCounter) of every result of
/// the answer; empty for an answer without an `fdmRef` (an error).
fn counters(call: &FdmCall) -> Vec<(String, Value, Value, Value)> {
    answer_refs(&call.response)
        .into_iter()
        .map(|(name, fdm_ref)| {
            (
                name,
                fdm_ref["eventLabel"].clone(),
                fdm_ref["eventCounter"].clone(),
                fdm_ref["totalCounter"].clone(),
            )
        })
        .collect()
}

/// The business content of the `variables` of a request: `business_view`
/// of each data object, with the points in time left out (an `fdmRef`
/// names the referenced ticket by its FDM time too).
fn variables_view(variables: &Value) -> Result<Value, String> {
    let parts = request_parts(variables);
    let mut views = Vec::new();
    for (name, data) in &parts {
        let view = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| business_view(data)))
            .map_err(|_| "business_view could not read the request".to_string())?;
        views.push((name.clone(), strip_volatile(&view)));
    }
    if let [(name, view)] = views.as_slice()
        && name == "data"
    {
        return Ok(view.clone());
    }
    Ok(Value::Object(views.into_iter().collect()))
}

/// A view with every `fdmRef` reduced to its presence: the reference
/// names the FDM's own answer of an earlier request (its id, counters),
/// which an official fixture cannot share with a run against another FDM.
fn fdm_ref_as_presence(view: &Value) -> Value {
    match view {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    if k == "fdmRef" && !v.is_null() {
                        (k.clone(), Value::Bool(true))
                    } else {
                        (k.clone(), fdm_ref_as_presence(v))
                    }
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

// ── The checks ─────────────────────────────────────────────────────

fn check_steps(linux: &Journal, android: &Journal) -> Outcome {
    let mut problems = Vec::new();
    if linux.verdict != "PASS" {
        problems.push(format!(
            "the {} journal has verdict {}",
            linux.side, linux.verdict
        ));
    }
    if android.verdict != "PASS" {
        problems.push(format!(
            "the {} journal has verdict {}",
            android.side, android.verdict
        ));
    }
    if linux.steps.len() != android.steps.len() {
        problems.push(format!(
            "{} steps against {}",
            linux.steps.len(),
            android.steps.len()
        ));
    }
    for (i, (l, a)) in linux.steps.iter().zip(&android.steps).enumerate() {
        if l.cmd != a.cmd {
            problems.push(format!("step {i}: command {} against {}", l.cmd, a.cmd));
        } else if l.ok != a.ok {
            problems.push(format!(
                "step {i} ({}): ok {} against {} (errors {:?} against {:?})",
                l.cmd, l.ok, a.ok, l.error, a.error
            ));
        }
    }
    Outcome::from_problems(problems)
}

fn operations(journal: &Journal) -> Vec<&str> {
    journal.fdm.iter().map(|c| c.operation.as_str()).collect()
}

fn check_ops(linux: &Journal, android: &Journal) -> Outcome {
    let (l, a) = (operations(linux), operations(android));
    if l == a {
        Outcome::pass()
    } else {
        Outcome::from_problems(vec![format!("operations {l:?} against {a:?}")])
    }
}

fn check_business(linux: &Journal, android: &Journal) -> Outcome {
    if linux.fdm.len() != android.fdm.len() {
        return Outcome::from_problems(vec![format!(
            "{} FDM calls against {}",
            linux.fdm.len(),
            android.fdm.len()
        )]);
    }
    let mut problems = Vec::new();
    for (i, (l, a)) in linux.fdm.iter().zip(&android.fdm).enumerate() {
        let views = variables_view(&l.request["variables"])
            .and_then(|lv| variables_view(&a.request["variables"]).map(|av| (lv, av)));
        match views {
            Err(e) => problems.push(format!("call {i} ({}): {e}", l.operation)),
            Ok((lv, av)) => {
                let mut found = Vec::new();
                diff("", &lv, &av, &mut found);
                for (path, ours, theirs) in found {
                    problems.push(format!(
                        "call {i} ({}): {path} is {ours} against {theirs}",
                        l.operation
                    ));
                }
            }
        }
    }
    Outcome::from_problems(problems)
}

fn check_counters(linux: &Journal, android: &Journal) -> Outcome {
    if linux.fdm.len() != android.fdm.len() {
        return Outcome::from_problems(vec![format!(
            "{} FDM calls against {}",
            linux.fdm.len(),
            android.fdm.len()
        )]);
    }
    let mut problems = Vec::new();
    for (i, (l, a)) in linux.fdm.iter().zip(&android.fdm).enumerate() {
        let (lc, ac) = (counters(l), counters(a));
        if lc != ac {
            problems.push(format!(
                "call {i} ({}): counters {lc:?} against {ac:?}",
                l.operation
            ));
        }
    }
    Outcome::from_problems(problems)
}

fn check_full(linux: &Journal, android: &Journal) -> Outcome {
    if linux.fdm.len() != android.fdm.len() {
        return Outcome::from_problems(vec![format!(
            "{} FDM calls against {}",
            linux.fdm.len(),
            android.fdm.len()
        )]);
    }
    let mut problems = Vec::new();
    for (i, (l, a)) in linux.fdm.iter().zip(&android.fdm).enumerate() {
        let mut found = Vec::new();
        diff(
            "",
            &strip_volatile(&l.request),
            &strip_volatile(&a.request),
            &mut found,
        );
        for (path, ours, theirs) in found {
            problems.push(format!(
                "call {i} ({}): {path} is {ours} against {theirs}",
                l.operation
            ));
        }
    }
    Outcome::from_problems(problems)
}

fn check_sw_version(linux: &Journal, android: &Journal, same_side: bool) -> Outcome {
    let mut problems = Vec::new();
    if same_side {
        if linux.pos_sw_version != android.pos_sw_version {
            problems.push(format!(
                "posSwVersion {} against {}",
                linux.pos_sw_version, android.pos_sw_version
            ));
        }
    } else {
        if !android.pos_sw_version.starts_with("pili-pos-android/") {
            problems.push(format!(
                "the android posSwVersion {} does not start with pili-pos-android/",
                android.pos_sw_version
            ));
        }
        if !linux.pos_sw_version.starts_with("pili-pos/") {
            problems.push(format!(
                "the reference posSwVersion {} does not start with pili-pos/",
                linux.pos_sw_version
            ));
        }
    }
    for journal in [linux, android] {
        for (i, call) in journal.fdm.iter().enumerate() {
            for (part, data) in request_parts(&call.request["variables"]) {
                let carried = data["posSwVersion"].as_str().unwrap_or("<none>");
                if carried != journal.pos_sw_version {
                    problems.push(format!(
                        "{} call {i} ({}, {part}) carries posSwVersion {carried}, the journal says {}",
                        journal.side, call.operation, journal.pos_sw_version
                    ));
                }
            }
        }
    }
    Outcome::from_problems(problems)
}

fn check_tickets(linux: &Journal, android: &Journal) -> Outcome {
    if linux.tickets.len() != android.tickets.len() {
        return Outcome::from_problems(vec![format!(
            "{} tickets against {}",
            linux.tickets.len(),
            android.tickets.len()
        )]);
    }
    let mut problems = Vec::new();
    for (i, (l, a)) in linux.tickets.iter().zip(&android.tickets).enumerate() {
        let (ln, an) = (normalise_ticket(l), normalise_ticket(a));
        if ln != an {
            let line = ln
                .lines()
                .zip(an.lines())
                .position(|(x, y)| x != y)
                .unwrap_or_else(|| ln.lines().count().min(an.lines().count()));
            problems.push(format!(
                "ticket {i} differs at line {}: {:?} against {:?}",
                line + 1,
                ln.lines().nth(line).unwrap_or("<end>"),
                an.lines().nth(line).unwrap_or("<end>")
            ));
        }
    }
    Outcome::from_problems(problems)
}

/// A path as the fixture deviations spell it: `[n]` as `.n`, and the
/// `mainProduct` level (which `business_view` flattens) left out, so
/// `transaction.transactionLines.0.mainProduct.productId` and
/// `transaction.transactionLines[0].productId` are the same path.
pub fn normalise_path(path: &str) -> String {
    let dotted = path.replace('[', ".").replace(']', "");
    dotted
        .split('.')
        .filter(|s| !s.is_empty() && *s != "mainProduct")
        .collect::<Vec<_>>()
        .join(".")
}

/// The fixtures of a `fixture` directory: `(file name, operation, variables)`,
/// in order. A directory with a `*GQL-Variables.json` of its own is one
/// fixture; otherwise each sub-directory that has one is (multi-step UCs).
pub fn load_fixtures(dir: &Path) -> anyhow::Result<Vec<(String, String, Value)>> {
    fn variables_in(dir: &Path) -> anyhow::Result<Option<(String, String, Value)>> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .with_context(|| format!("reading {}", dir.display()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().ends_with("GQL-Variables.json"))
            })
            .collect();
        files.sort();
        let Some(file) = files.first() else {
            return Ok(None);
        };
        let text = std::fs::read_to_string(file)?;
        let variables: Value =
            serde_json::from_str(&text).with_context(|| format!("fixture {}", file.display()))?;
        let name = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        // The operation: the mutation named by the sibling query, else the name's prefix.
        let query = name.replace("GQL-Variables.json", "GQL-Query.graphql");
        let operation = std::fs::read_to_string(dir.join(&query))
            .ok()
            .and_then(|q| {
                regex::Regex::new(r"\b(sign[A-Za-z]+)\s*\(")
                    .ok()?
                    .captures(&q)
                    .map(|c| c[1].to_string())
            })
            .or_else(|| {
                regex::Regex::new(r"(sign[A-Za-z]+)")
                    .ok()?
                    .captures(&name)
                    .map(|c| c[1].to_string())
            })
            .unwrap_or_default();
        Ok(Some((name, operation, variables)))
    }

    if let Some(one) = variables_in(dir)? {
        return Ok(vec![one]);
    }
    let mut subdirs: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    subdirs.sort();
    let mut out = Vec::new();
    for sub in subdirs {
        if let Some(one) = variables_in(&sub)? {
            out.push(one);
        }
    }
    Ok(out)
}

fn check_fixture(android: &Journal, scenario: &Scenario, opts: &Options) -> Outcome {
    let Some(fixture) = &scenario.fixture else {
        return Outcome::skip("the scenario names no fixture");
    };
    let dir = opts.repo.join(fixture);
    let fixtures = match load_fixtures(&dir) {
        Ok(f) if !f.is_empty() => f,
        Ok(_) => {
            return Outcome::from_problems(vec![format!("no GQL-Variables.json under {fixture}")]);
        }
        Err(e) => return Outcome::from_problems(vec![format!("{e:#}")]),
    };

    let allowed: Vec<(String, &str)> = scenario
        .fixture_deviations
        .iter()
        .map(|d| (normalise_path(&d.path), d.reason.as_str()))
        .collect();
    let mut used: BTreeSet<String> = BTreeSet::new();
    let mut problems = Vec::new();
    let mut taken: BTreeMap<String, usize> = BTreeMap::new();

    // The fixtures are the last requests of their operation: a scenario may sign
    // set-up requests first (the sale a refund refers to) that no fixture covers.
    let mut wanted: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, operation, _) in &fixtures {
        *wanted.entry(operation.as_str()).or_insert(0) += 1;
    }
    for (name, operation, variables) in &fixtures {
        let nth = taken.entry(operation.clone()).or_insert(0);
        let made = android
            .fdm
            .iter()
            .filter(|c| &c.operation == operation)
            .count();
        let skipped = made.saturating_sub(wanted.get(operation.as_str()).copied().unwrap_or(0));
        let call = android
            .fdm
            .iter()
            .filter(|c| &c.operation == operation)
            .nth(skipped + *nth);
        *nth += 1;
        let Some(call) = call else {
            problems.push(format!("{name}: no {operation} request to compare with"));
            continue;
        };
        let views = variables_view(&call.request["variables"])
            .and_then(|ours| variables_view(variables).map(|theirs| (ours, theirs)));
        let (ours, theirs) = match views {
            Ok((ours, theirs)) => (fdm_ref_as_presence(&ours), fdm_ref_as_presence(&theirs)),
            Err(e) => {
                problems.push(format!("{name}: {e}"));
                continue;
            }
        };
        let mut found = Vec::new();
        diff("", &ours, &theirs, &mut found);
        for (path, ours, theirs) in found {
            let normal = normalise_path(&path);
            if allowed.iter().any(|(p, _)| *p == normal) {
                used.insert(normal);
            } else {
                problems.push(format!(
                    "{name}: {path} is {ours} against the fixture's {theirs}"
                ));
            }
        }
    }
    for (path, _) in &allowed {
        if !used.contains(path) {
            problems.push(format!("the listed deviation {path} does not occur"));
        }
    }
    Outcome::from_problems(problems)
}

/// The key paths of a JSON value: `a.b`, arrays folded into `[]`.
fn shape_of(value: &Value, prefix: &str, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let path = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                out.insert(path.clone());
                shape_of(v, &path, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                shape_of(item, &format!("{prefix}[]"), out);
            }
        }
        _ => {}
    }
}

/// The key paths of some JSON values, as one set: the union over the values.
/// An answer whose list is empty carries no key below it, so comparing one
/// whole-answer string per call would make a call without messages differ
/// from a call with messages; the union over all the calls of a side is the
/// set of keys that side can answer.
fn shapes<'a>(values: impl Iterator<Item = &'a Value>) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for value in values {
        shape_of(value, "", &mut keys);
    }
    keys
}

/// The shapes of what the scenario's `check_fdm_status` steps returned
/// (the engine caches a status answer for 20 s, so a step may be served
/// without a call to the FDM; its result keeps the same shape).
fn step_shapes(journal: &Journal) -> BTreeSet<String> {
    shapes(
        journal
            .steps
            .iter()
            .filter(|s| s.cmd == "check_fdm_status" && s.ok)
            .map(|s| &s.result),
    )
}

/// The shapes of the Status answers the mock logged.
fn call_shapes(journal: &Journal) -> BTreeSet<String> {
    shapes(journal.status_calls.iter().map(|c| &c.response))
}

fn check_status_shape(linux: &Journal, android: &Journal) -> Outcome {
    let (ls, a_s) = (step_shapes(linux), step_shapes(android));
    let (lc, ac) = (call_shapes(linux), call_shapes(android));
    let mut problems = Vec::new();
    let no_steps = ls.is_empty() && a_s.is_empty();
    if no_steps && (lc.is_empty() || ac.is_empty()) {
        problems.push(format!(
            "no Status answer to compare ({} Status calls against {}, no check_fdm_status step)",
            linux.status_calls.len(),
            android.status_calls.len()
        ));
    }
    // The Status calls of the monitor depend on timing: they are compared
    // only when both sides made one.
    for (what, l, a, applies) in [
        ("check_fdm_status result", &ls, &a_s, !no_steps),
        ("Status answer", &lc, &ac, !lc.is_empty() && !ac.is_empty()),
    ] {
        if !applies {
            continue;
        }
        for shape in l.difference(a) {
            problems.push(format!("a {what} only the reference has: {shape}"));
        }
        for shape in a.difference(l) {
            problems.push(format!("a {what} only the other side has: {shape}"));
        }
    }
    Outcome::from_problems(problems)
}

// ── One pair ───────────────────────────────────────────────────────

/// Run every check on one scenario's two journals. `meta` is the scenario
/// file when it could be read: without it the fixture and status-shape
/// checks are skipped.
pub fn compare_pair(
    linux: &Journal,
    android: &Journal,
    meta: Option<&Scenario>,
    opts: &Options,
) -> ScenarioResult {
    let mut checks: BTreeMap<&'static str, Outcome> = BTreeMap::new();
    checks.insert("steps", check_steps(linux, android));
    checks.insert("fdm-ops", check_ops(linux, android));
    checks.insert("fdm-business", check_business(linux, android));
    checks.insert(
        "fdm-counters",
        if opts.no_counters {
            Outcome::skip("--no-counters")
        } else {
            check_counters(linux, android)
        },
    );
    checks.insert("fdm-full", check_full(linux, android));
    checks.insert(
        "sw-version",
        check_sw_version(linux, android, opts.same_side),
    );
    checks.insert("tickets", check_tickets(linux, android));
    checks.insert(
        "fixture",
        match (opts.fixtures, meta) {
            (false, _) => Outcome::skip("without --fixtures"),
            (true, None) => Outcome::skip("the scenario file was not found"),
            (true, Some(scenario)) => check_fixture(android, scenario, opts),
        },
    );
    checks.insert(
        "status-shape",
        match meta {
            Some(scenario) if scenario.compare.status == "shape" => {
                check_status_shape(linux, android)
            }
            Some(_) => Outcome::skip("compare.status is ignore"),
            None => Outcome::skip("the scenario file was not found"),
        },
    );
    ScenarioResult {
        group: linux.group.clone(),
        scenario: linux.scenario.clone(),
        ids: if linux.ids.is_empty() {
            android.ids.clone()
        } else {
            linux.ids.clone()
        },
        checks,
    }
}

// ── Two directories ────────────────────────────────────────────────

fn failure(group: &str, scenario: &str, why: String) -> ScenarioResult {
    let mut checks = BTreeMap::new();
    checks.insert("steps", Outcome::from_problems(vec![why]));
    ScenarioResult {
        group: group.to_string(),
        scenario: scenario.to_string(),
        ids: Vec::new(),
        checks,
    }
}

/// Compare every journal of `linux_dir` with the one of the same
/// group and scenario in `android_dir`; a journal on one side only is a
/// FAIL.
pub fn compare_dirs(
    linux_dir: &Path,
    android_dir: &Path,
    opts: &Options,
) -> anyhow::Result<Vec<ScenarioResult>> {
    let linux_list = journal::list(linux_dir)?;
    let android_list = journal::list(android_dir)?;
    let all: BTreeSet<&(String, String)> = linux_list.iter().chain(&android_list).collect();
    let scenarios_root = opts
        .scenarios
        .clone()
        .unwrap_or_else(|| opts.repo.join("android/app/src/androidTest/assets/uc"));

    let mut results = Vec::new();
    for (group, scenario) in all {
        let l = linux_list.contains(&(group.clone(), scenario.clone()));
        let a = android_list.contains(&(group.clone(), scenario.clone()));
        if !l || !a {
            let side = if l { "android" } else { "reference" };
            results.push(failure(group, scenario, format!("no {side} journal")));
            continue;
        }
        let load = |root: &Path| Journal::load(&root.join(group).join(format!("{scenario}.json")));
        let (linux, android) = match (load(linux_dir), load(android_dir)) {
            (Ok(l), Ok(a)) => (l, a),
            (Err(e), _) | (_, Err(e)) => {
                results.push(failure(group, scenario, format!("{e:#}")));
                continue;
            }
        };
        let meta =
            Scenario::load(&scenarios_root.join(group).join(format!("{scenario}.json"))).ok();
        results.push(compare_pair(&linux, &android, meta.as_ref(), opts));
    }
    Ok(results)
}

/// The lines `compare` prints: one per scenario, then the total.
pub fn summary_lines(results: &[ScenarioResult]) -> (Vec<String>, bool) {
    let failed = results.iter().filter(|r| !r.passed()).count();
    let mut lines: Vec<String> = results.iter().map(ScenarioResult::line).collect();
    lines.push(format!(
        "UCCHECK | total | {} | {failed} failed ({} scenarios)",
        if failed == 0 { "PASS" } else { "FAIL" },
        results.len()
    ));
    (lines, failed == 0)
}

/// The report of P7.md section 4.5.
pub fn report(results: &[ScenarioResult], generated: &str) -> Value {
    let scenarios: Vec<Value> = results
        .iter()
        .map(|r| {
            let checks: serde_json::Map<String, Value> = CHECKS
                .iter()
                .filter_map(|c| {
                    r.checks
                        .get(c)
                        .map(|o| ((*c).to_string(), json!(o.status.as_str())))
                })
                .collect();
            let mut entry = json!({
                "group": r.group,
                "scenario": r.scenario,
                "ids": r.ids,
                "checks": checks,
                "verdict": if r.passed() { "PASS" } else { "FAIL" },
            });
            let details = r.detail_lines();
            if !details.is_empty() {
                entry["details"] = json!(details);
            }
            entry
        })
        .collect();
    json!({
        "format": 1,
        "generated": generated,
        "reference": "engine on the host",
        "scenarios": scenarios,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::JournalStep;

    fn sale_call(unit_price: f64, event_counter: i64, sw: &str) -> FdmCall {
        FdmCall {
            operation: "signSale".to_string(),
            request: json!({
                "query": "mutation { signSale }",
                "variables": { "data": {
                    "posId": "CPOS0031234567",
                    "posSwVersion": sw,
                    "posDateTime": "2026-10-10T01:36:36.176+02:00",
                    "bookingDate": "2026-10-09",
                    "ticketMedium": "PAPER",
                    "transaction": {
                        "transactionLines": [{
                            "lineTotal": unit_price * 2.0,
                            "lineType": "SINGLE_PRODUCT",
                            "mainProduct": {
                                "departmentId": "12",
                                "productId": "12007",
                                "quantity": 2.0,
                                "quantityType": "PIECE",
                                "unitPrice": unit_price,
                                "vats": [{ "label": "C", "price": unit_price * 2.0 }],
                            },
                        }],
                        "transactionTotal": unit_price * 2.0,
                    },
                    "financials": [{ "amount": unit_price * 2.0, "amountType": "PAYMENT", "type": "CASH" }],
                } },
            }),
            response: json!({ "data": { "signSale": {
                "fdmRef": {
                    "eventLabel": "N",
                    "eventCounter": event_counter,
                    "totalCounter": event_counter + 1,
                    "fdmDateTime": "2026-10-09T23:36:36Z",
                },
                "digitalSignature": "c2ln",
            } } }),
        }
    }

    fn journal(side: &str, sw: &str) -> Journal {
        Journal {
            format: 1,
            side: side.to_string(),
            group: "00_smoke".to_string(),
            scenario: "001_SMOKE".to_string(),
            ids: vec![],
            pos_sw_version: sw.to_string(),
            steps: vec![
                JournalStep {
                    cmd: "login".to_string(),
                    ok: true,
                    error: None,
                    result: Value::Null,
                    unmet: None,
                },
                JournalStep {
                    cmd: "checkout".to_string(),
                    ok: true,
                    error: None,
                    result: Value::Null,
                    unmet: None,
                },
            ],
            fdm: vec![sale_call(2.5, 1, sw)],
            status_calls: vec![],
            tickets: vec!["TICKET\n10.10.2026   01:36\nTOTAL 5,00".to_string()],
            verdict: "PASS".to_string(),
        }
    }

    fn pair() -> (Journal, Journal) {
        (
            journal("linux", "pili-pos/1.1.0"),
            journal("android", "pili-pos-android/1.0.0-dev"),
        )
    }

    fn status(result: &ScenarioResult, check: &str) -> Status {
        result.checks[check].status
    }

    fn run(linux: &Journal, android: &Journal) -> ScenarioResult {
        compare_pair(linux, android, None, &Options::default())
    }

    #[test]
    fn two_equal_runs_pass_every_check_that_applies() {
        let (linux, android) = pair();
        let result = run(&linux, &android);
        assert!(result.passed(), "{:?}", result.detail_lines());
        for check in [
            "steps",
            "fdm-ops",
            "fdm-business",
            "fdm-counters",
            "fdm-full",
            "sw-version",
            "tickets",
        ] {
            assert_eq!(status(&result, check), Status::Pass, "{check}");
        }
        assert_eq!(status(&result, "fixture"), Status::Skip);
        assert_eq!(status(&result, "status-shape"), Status::Skip);
        assert!(
            result
                .line()
                .starts_with("UCCHECK | 00_smoke/001_SMOKE | PASS | steps:PASS fdm-ops:PASS")
        );
    }

    #[test]
    fn volatile_values_may_differ_between_the_runs() {
        let (linux, mut android) = pair();
        let data = &mut android.fdm[0].request["variables"]["data"];
        data["posDateTime"] = json!("2026-10-11T17:59:00.001+02:00");
        data["bookingDate"] = json!("2026-10-11");
        data["posId"] = json!("CPOS0039999999");
        android.fdm[0].response["data"]["signSale"]["fdmRef"]["fdmDateTime"] =
            json!("2026-10-11T15:59:00Z");
        let result = run(&linux, &android);
        assert!(result.passed(), "{:?}", result.detail_lines());
    }

    #[test]
    fn a_changed_unit_price_fails_the_business_and_full_checks() {
        let (linux, mut android) = pair();
        android.fdm[0] = sale_call(2.6, 1, &android.pos_sw_version);
        let result = run(&linux, &android);
        assert!(!result.passed());
        assert_eq!(status(&result, "fdm-business"), Status::Fail);
        assert_eq!(status(&result, "fdm-full"), Status::Fail);
        assert!(
            result
                .detail_lines()
                .iter()
                .any(|l| l.contains("unitPrice")),
            "{:?}",
            result.detail_lines()
        );
        assert_eq!(status(&result, "fdm-counters"), Status::Pass);
    }

    #[test]
    fn a_changed_event_counter_fails_only_the_counters_check() {
        let (linux, mut android) = pair();
        android.fdm[0] = sale_call(2.5, 2, &android.pos_sw_version);
        let result = run(&linux, &android);
        assert_eq!(status(&result, "fdm-counters"), Status::Fail);
        assert_eq!(status(&result, "fdm-business"), Status::Pass);
        // --no-counters (live mode) skips the check.
        let opts = Options {
            no_counters: true,
            ..Options::default()
        };
        let live = compare_pair(&linux, &android, None, &opts);
        assert_eq!(status(&live, "fdm-counters"), Status::Skip);
        assert!(live.passed());
    }

    #[test]
    fn a_different_operation_list_fails_the_ops_check() {
        let (linux, mut android) = pair();
        android.fdm[0].operation = "signOrder".to_string();
        assert_eq!(status(&run(&linux, &android), "fdm-ops"), Status::Fail);
        android.fdm.push(sale_call(2.5, 2, &android.pos_sw_version));
        let result = run(&linux, &android);
        assert_eq!(status(&result, "fdm-business"), Status::Fail);
    }

    #[test]
    fn an_extra_ticket_fails_the_tickets_check() {
        let (linux, mut android) = pair();
        android.tickets.push("ANOTHER".to_string());
        let result = run(&linux, &android);
        assert_eq!(status(&result, "tickets"), Status::Fail);
        assert!(result.detail_lines()[0].contains("1 tickets against 2"));
    }

    #[test]
    fn a_ticket_that_differs_after_normalisation_fails_and_dates_do_not() {
        let (linux, mut android) = pair();
        android.tickets[0] = "TICKET\n11.10.2026   17:59\nTOTAL 5,00".to_string();
        assert_eq!(status(&run(&linux, &android), "tickets"), Status::Pass);
        android.tickets[0] = "TICKET\n11.10.2026   17:59\nTOTAL 5,50".to_string();
        let result = run(&linux, &android);
        assert_eq!(status(&result, "tickets"), Status::Fail);
        assert!(
            result.detail_lines()[0].contains("line 3"),
            "{:?}",
            result.detail_lines()
        );
    }

    #[test]
    fn a_failed_step_or_a_different_outcome_fails_the_steps_check() {
        let (linux, mut android) = pair();
        android.steps[1].ok = false;
        android.steps[1].error = Some("FDM_LOCKED".to_string());
        assert_eq!(status(&run(&linux, &android), "steps"), Status::Fail);
        let (linux, mut android) = pair();
        android.verdict = "FAIL".to_string();
        assert_eq!(status(&run(&linux, &android), "steps"), Status::Fail);
        let (linux, mut android) = pair();
        android.steps.pop();
        assert_eq!(status(&run(&linux, &android), "steps"), Status::Fail);
        let (linux, mut android) = pair();
        android.steps[0].cmd = "logout".to_string();
        assert_eq!(status(&run(&linux, &android), "steps"), Status::Fail);
    }

    #[test]
    fn the_software_version_is_checked_per_side_and_per_request() {
        let (linux, mut android) = pair();
        android.pos_sw_version = "pili-pos/1.1.0".to_string();
        assert_eq!(status(&run(&linux, &android), "sw-version"), Status::Fail);

        // A request that carries another value than its journal says.
        let (linux, mut android) = pair();
        android.fdm[0].request["variables"]["data"]["posSwVersion"] = json!("pili-pos/1.1.0");
        assert_eq!(status(&run(&linux, &android), "sw-version"), Status::Fail);

        // Same-side mode: both journals must carry the same value, whatever it is.
        let linux = journal("linux", "pili-pos/1.1.0");
        let second = journal("linux", "pili-pos/1.1.0");
        let same = Options {
            same_side: true,
            ..Options::default()
        };
        assert!(compare_pair(&linux, &second, None, &same).passed());
        let other = journal("linux", "pili-pos/1.2.0");
        assert_eq!(
            status(&compare_pair(&linux, &other, None, &same), "sw-version"),
            Status::Fail
        );
        // Without --same-side two reference journals are not a valid pair.
        assert_eq!(status(&run(&linux, &second), "sw-version"), Status::Fail);
    }

    fn status_call(extra_key: bool) -> FdmCall {
        let mut response =
            json!({ "data": { "status": { "fdmId": "X", "messages": [{ "code": "A" }] } } });
        if extra_key {
            response["data"]["status"]["bufferCapacityUsed"] = json!(1.2);
        }
        FdmCall {
            operation: "Status".to_string(),
            request: json!({}),
            response,
        }
    }

    fn shape_scenario() -> Scenario {
        Scenario::parse(
            r#"{"format":1,"id":"Q100","ids":["Q100"],"compare":{"status":"shape"},"steps":[{"cmd":"check_fdm_status"}]}"#,
        )
        .unwrap()
    }

    #[test]
    fn the_status_shape_is_compared_only_when_the_scenario_asks_for_it() {
        let (mut linux, mut android) = pair();
        linux.status_calls = vec![status_call(false), status_call(false)];
        android.status_calls = vec![status_call(false)];
        let scenario = shape_scenario();
        let ok = compare_pair(&linux, &android, Some(&scenario), &Options::default());
        assert_eq!(status(&ok, "status-shape"), Status::Pass);

        android.status_calls = vec![status_call(true)];
        let bad = compare_pair(&linux, &android, Some(&scenario), &Options::default());
        assert_eq!(status(&bad, "status-shape"), Status::Fail);

        android.status_calls = vec![];
        let none = compare_pair(&linux, &android, Some(&scenario), &Options::default());
        assert_eq!(status(&none, "status-shape"), Status::Fail);

        // A check_fdm_status step is the stable source: the engine caches a Status answer.
        for journal in [&mut linux, &mut android] {
            journal.status_calls = vec![];
            journal.steps.push(JournalStep {
                cmd: "check_fdm_status".to_string(),
                ok: true,
                error: None,
                result: json!({ "connected": true, "messages": [] }),
                unmet: None,
            });
        }
        let by_step = compare_pair(&linux, &android, Some(&scenario), &Options::default());
        assert_eq!(
            status(&by_step, "status-shape"),
            Status::Pass,
            "{:?}",
            by_step.detail_lines()
        );
        android.steps.last_mut().unwrap().result =
            json!({ "connected": true, "messages": [], "error": null });
        let drift = compare_pair(&linux, &android, Some(&scenario), &Options::default());
        assert_eq!(status(&drift, "status-shape"), Status::Fail);

        let ignore = Scenario::parse(r#"{"format":1,"id":"X","steps":[{"cmd":"a"}]}"#).unwrap();
        let skipped = compare_pair(&linux, &android, Some(&ignore), &Options::default());
        assert_eq!(status(&skipped, "status-shape"), Status::Skip);
    }

    #[test]
    fn an_answer_without_messages_has_the_keys_of_one_with_messages_when_another_call_shows_them() {
        let empty = FdmCall {
            operation: "Status".to_string(),
            request: json!({}),
            response: json!({ "data": { "status": { "fdmId": "X", "messages": [] } } }),
        };
        let (mut linux, mut android) = pair();
        linux.status_calls = vec![status_call(false)];
        android.status_calls = vec![empty, status_call(false)];
        let scenario = shape_scenario();
        let both = compare_pair(&linux, &android, Some(&scenario), &Options::default());
        assert_eq!(status(&both, "status-shape"), Status::Pass);
        // The empty answer alone has fewer keys than the reference's: still a difference.
        android.status_calls.remove(1);
        let alone = compare_pair(&linux, &android, Some(&scenario), &Options::default());
        assert_eq!(status(&alone, "status-shape"), Status::Fail);
    }

    #[test]
    fn deviation_paths_are_compared_in_one_spelling() {
        assert_eq!(
            normalise_path("transaction.transactionLines.0.mainProduct.productId"),
            normalise_path("transaction.transactionLines[0].productId")
        );
        assert_eq!(normalise_path("costCenter.type"), "costCenter.type");
        assert_eq!(normalise_path("fdmRef"), "fdmRef");
    }

    /// A fixture directory with one variables file for signSale.
    fn fixture_dir(unit_price: f64, with_center: bool) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let call = sale_call(unit_price, 1, "x");
        let mut data = call.request["variables"]["data"].clone();
        if with_center {
            data["costCenter"] = json!({ "type": "TABLE" });
        }
        std::fs::write(
            dir.path().join("M110_GQL-Variables.json"),
            serde_json::to_string(&json!({ "data": data })).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.path().join("M110_GQL-Query.graphql"),
            "mutation($data: SaleInput!) { signSale(data: $data) { posId } }",
        )
        .unwrap();
        dir
    }

    fn fixture_scenario(dir: &Path, deviations: &str) -> Scenario {
        Scenario::parse(&format!(
            r#"{{"format":1,"id":"M110","ids":["M110"],"fixture":"{}","fixtureDeviations":{deviations},"steps":[{{"cmd":"a"}}]}}"#,
            dir.display()
        ))
        .unwrap()
    }

    fn with_fixtures() -> Options {
        Options {
            fixtures: true,
            repo: PathBuf::from("/"),
            ..Options::default()
        }
    }

    #[test]
    fn a_request_equal_to_its_fixture_passes_the_fixture_check() {
        let (linux, android) = pair();
        let dir = fixture_dir(2.5, false);
        let scenario = fixture_scenario(dir.path(), "[]");
        let result = compare_pair(&linux, &android, Some(&scenario), &with_fixtures());
        assert_eq!(
            status(&result, "fixture"),
            Status::Pass,
            "{:?}",
            result.detail_lines()
        );
    }

    #[test]
    fn a_difference_from_the_fixture_fails_unless_it_is_listed() {
        let (linux, android) = pair();
        let dir = fixture_dir(2.5, true);
        // The fixture names a cost center the engine's request does not carry.
        let unlisted = fixture_scenario(dir.path(), "[]");
        let bad = compare_pair(&linux, &android, Some(&unlisted), &with_fixtures());
        assert_eq!(status(&bad, "fixture"), Status::Fail);
        assert!(
            bad.detail_lines()[0].contains("costCenter.type"),
            "{:?}",
            bad.detail_lines()
        );

        let listed = fixture_scenario(
            dir.path(),
            r#"[{"path":"costCenter.type","reason":"table sale"}]"#,
        );
        let ok = compare_pair(&linux, &android, Some(&listed), &with_fixtures());
        assert_eq!(
            status(&ok, "fixture"),
            Status::Pass,
            "{:?}",
            ok.detail_lines()
        );
    }

    #[test]
    fn a_listed_deviation_that_does_not_occur_fails_the_fixture_check() {
        let (linux, android) = pair();
        let dir = fixture_dir(2.5, false);
        let scenario = fixture_scenario(
            dir.path(),
            r#"[{"path":"transaction.transactionLines.0.mainProduct.productId","reason":"r"}]"#,
        );
        let result = compare_pair(&linux, &android, Some(&scenario), &with_fixtures());
        assert_eq!(status(&result, "fixture"), Status::Fail);
        assert!(
            result.detail_lines()[0].contains("does not occur"),
            "{:?}",
            result.detail_lines()
        );
    }

    #[test]
    fn a_fixture_with_no_matching_request_fails_and_the_check_is_opt_in() {
        let (linux, mut android) = pair();
        android.fdm[0].operation = "signOrder".to_string();
        let dir = fixture_dir(2.5, false);
        let scenario = fixture_scenario(dir.path(), "[]");
        let result = compare_pair(&linux, &android, Some(&scenario), &with_fixtures());
        assert_eq!(status(&result, "fixture"), Status::Fail);
        // Without --fixtures the check is skipped, with a scenario that has no fixture too.
        let off = compare_pair(&linux, &android, Some(&scenario), &Options::default());
        assert_eq!(status(&off, "fixture"), Status::Skip);
        let none = Scenario::parse(r#"{"format":1,"id":"X","steps":[{"cmd":"a"}]}"#).unwrap();
        let skipped = compare_pair(&linux, &android, Some(&none), &with_fixtures());
        assert_eq!(status(&skipped, "fixture"), Status::Skip);
    }

    #[test]
    fn multi_step_fixtures_match_the_requests_of_their_operation_in_order() {
        let dir = tempfile::tempdir().unwrap();
        for (sub, price) in [("1_Original", 2.5), ("2_Refund", 3.0)] {
            let step = dir.path().join(sub);
            std::fs::create_dir_all(&step).unwrap();
            let call = sale_call(price, 1, "x");
            std::fs::write(
                step.join(format!("{sub}_GQL-Variables.json")),
                serde_json::to_string(&json!({ "data": call.request["variables"]["data"] }))
                    .unwrap(),
            )
            .unwrap();
            std::fs::write(
                step.join(format!("{sub}_GQL-Query.graphql")),
                "mutation { signSale(data: $data) }",
            )
            .unwrap();
        }
        let loaded = load_fixtures(dir.path()).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].1, "signSale");

        let (linux, mut android) = pair();
        android.fdm.push(sale_call(3.0, 2, &android.pos_sw_version));
        let scenario = fixture_scenario(dir.path(), "[]");
        let result = compare_pair(&linux, &android, Some(&scenario), &with_fixtures());
        assert_eq!(
            status(&result, "fixture"),
            Status::Pass,
            "{:?}",
            result.detail_lines()
        );
    }

    #[test]
    fn a_fixture_is_matched_with_the_last_requests_of_its_operation() {
        let dir = fixture_dir(3.0, false);
        let (linux, mut android) = pair();
        // A set-up sale signed first, that the fixture does not cover.
        android
            .fdm
            .insert(0, sale_call(2.5, 1, &android.pos_sw_version));
        android.fdm[1] = sale_call(3.0, 2, &android.pos_sw_version);
        let scenario = fixture_scenario(dir.path(), "[]");
        let result = compare_pair(&linux, &android, Some(&scenario), &with_fixtures());
        assert_eq!(
            status(&result, "fixture"),
            Status::Pass,
            "{:?}",
            result.detail_lines()
        );
    }

    /// A Z report: one operation that signs the turnover and the user report.
    fn z_call(sw: &str, user_counter: i64) -> FdmCall {
        FdmCall {
            operation: "signReportTurnoverZ".to_string(),
            request: json!({
                "query": "mutation { signReportTurnoverZ signReportUserZ }",
                "variables": {
                    "isTraining": false,
                    "turnover": { "posSwVersion": sw, "ticketMedium": "PAPER", "turnover": { "vats": [] } },
                    "user": { "posSwVersion": sw, "ticketMedium": "PAPER", "users": [] },
                },
            }),
            response: json!({ "data": {
                "signReportTurnoverZ": { "fdmRef": { "eventLabel": "R", "eventCounter": 2, "totalCounter": 13 } },
                "signReportUserZ": { "fdmRef": { "eventLabel": "R", "eventCounter": user_counter, "totalCounter": 14 } },
            } }),
        }
    }

    #[test]
    fn a_z_report_with_two_data_objects_and_two_results_is_compared_whole() {
        let (mut linux, mut android) = pair();
        linux.fdm = vec![z_call(&linux.pos_sw_version, 3)];
        android.fdm = vec![z_call(&android.pos_sw_version, 3)];
        let result = run(&linux, &android);
        assert!(result.passed(), "{:?}", result.detail_lines());

        android.fdm = vec![z_call(&android.pos_sw_version, 4)];
        let result = run(&linux, &android);
        assert_eq!(status(&result, "fdm-counters"), Status::Fail);
        assert!(
            result.detail_lines()[0].contains("signReportUserZ"),
            "{:?}",
            result.detail_lines()
        );

        // The software version is checked in each data object.
        android.fdm = vec![z_call("pili-pos/1.1.0", 3)];
        assert_eq!(status(&run(&linux, &android), "sw-version"), Status::Fail);
    }

    #[test]
    fn the_time_of_a_referenced_ticket_is_not_a_business_difference() {
        let (mut linux, mut android) = pair();
        for (journal, time) in [
            (&mut linux, "2026-10-09T23:42:46Z"),
            (&mut android, "2026-10-11T08:00:00Z"),
        ] {
            journal.fdm[0].request["variables"]["data"]["fdmRef"] = json!({
                "fdmId": "MCK00000001",
                "fdmDateTime": time,
                "eventLabel": "N",
                "eventCounter": 1,
                "totalCounter": 2,
            });
        }
        let result = run(&linux, &android);
        assert!(result.passed(), "{:?}", result.detail_lines());
        // The referenced counter is a business field.
        android.fdm[0].request["variables"]["data"]["fdmRef"]["eventCounter"] = json!(2);
        assert_eq!(status(&run(&linux, &android), "fdm-business"), Status::Fail);
    }

    #[test]
    fn a_fixture_compares_the_presence_of_a_referenced_ticket_only() {
        let (linux, mut android) = pair();
        android.fdm[0].request["variables"]["data"]["fdmRef"] = json!({
            "fdmId": "MCK00000001", "fdmDateTime": "2026-10-09T23:42:46Z",
            "eventLabel": "N", "eventCounter": 1, "totalCounter": 2,
        });
        let dir = fixture_dir(2.5, false);
        // The fixture references another FDM's ticket.
        let path = dir.path().join("M110_GQL-Variables.json");
        let mut fixture: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        fixture["data"]["fdmRef"] = json!({
            "fdmId": "SPF01987654", "fdmDateTime": "2024-07-29T13:00:00Z",
            "eventLabel": "N", "eventCounter": 7, "totalCounter": 9,
        });
        std::fs::write(&path, serde_json::to_string(&fixture).unwrap()).unwrap();
        let scenario = fixture_scenario(dir.path(), "[]");
        let result = compare_pair(&linux, &android, Some(&scenario), &with_fixtures());
        assert_eq!(
            status(&result, "fixture"),
            Status::Pass,
            "{:?}",
            result.detail_lines()
        );

        // A reference the fixture does not have is a difference to list.
        fixture["data"].as_object_mut().unwrap().remove("fdmRef");
        std::fs::write(&path, serde_json::to_string(&fixture).unwrap()).unwrap();
        let result = compare_pair(&linux, &android, Some(&scenario), &with_fixtures());
        assert_eq!(status(&result, "fixture"), Status::Fail);
        let listed = fixture_scenario(
            dir.path(),
            r#"[{"path":"fdmRef","reason":"every refund references its ticket"}]"#,
        );
        let result = compare_pair(&linux, &android, Some(&listed), &with_fixtures());
        assert_eq!(
            status(&result, "fixture"),
            Status::Pass,
            "{:?}",
            result.detail_lines()
        );
    }

    fn write_dirs() -> (tempfile::TempDir, tempfile::TempDir) {
        let (linux, android) = pair();
        let (l, a) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        // Journal::write puts files under <root>/<side>/<group>/, the compare reads <root>/<group>/.
        for (journal, root) in [(&linux, l.path()), (&android, a.path())] {
            let dir = root.join(&journal.group);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join(format!("{}.json", journal.scenario)),
                serde_json::to_string(journal).unwrap(),
            )
            .unwrap();
        }
        (l, a)
    }

    #[test]
    fn two_directories_are_compared_pair_by_pair_and_a_missing_journal_fails() {
        let (l, a) = write_dirs();
        let opts = Options {
            repo: PathBuf::from("/nonexistent"),
            ..Options::default()
        };
        let results = compare_dirs(l.path(), a.path(), &opts).unwrap();
        assert_eq!(results.len(), 1);
        let (lines, pass) = summary_lines(&results);
        assert!(pass);
        assert_eq!(
            lines.last().unwrap(),
            "UCCHECK | total | PASS | 0 failed (1 scenarios)"
        );

        std::fs::remove_file(a.path().join("00_smoke/001_SMOKE.json")).unwrap();
        let results = compare_dirs(l.path(), a.path(), &opts).unwrap();
        let (lines, pass) = summary_lines(&results);
        assert!(!pass);
        assert!(lines[0].contains("FAIL"), "{lines:?}");
        assert!(results[0].detail_lines()[0].contains("no android journal"));
        assert_eq!(
            lines.last().unwrap(),
            "UCCHECK | total | FAIL | 1 failed (1 scenarios)"
        );
    }

    #[test]
    fn the_report_has_the_format_of_the_spec() {
        let (linux, android) = pair();
        let result = run(&linux, &android);
        let body = report(&[result], "2026-10-10T00:00:00Z");
        assert_eq!(body["format"], 1);
        assert_eq!(body["reference"], "engine on the host");
        assert_eq!(body["generated"], "2026-10-10T00:00:00Z");
        let entry = &body["scenarios"][0];
        assert_eq!(entry["group"], "00_smoke");
        assert_eq!(entry["scenario"], "001_SMOKE");
        assert_eq!(entry["verdict"], "PASS");
        assert_eq!(entry["checks"]["steps"], "PASS");
        assert_eq!(entry["checks"]["fixture"], "SKIP");
        assert!(entry.get("details").is_none());
    }
}
