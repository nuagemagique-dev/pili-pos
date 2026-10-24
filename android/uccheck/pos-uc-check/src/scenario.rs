//! Scenario files (format 1, P7.md section 4.2): parsing, `$name.path`
//! references and the `expect` rules.
//!
//! A scenario is data. This module holds no fiscal rule: it substitutes
//! references and checks what a call returned against what the scenario
//! expects, nothing more.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde::Deserialize;
use serde_json::{Map, Value};

/// One scenario file.
#[derive(Debug, Clone, Deserialize)]
pub struct Scenario {
    pub format: u32,
    pub id: String,
    #[serde(default)]
    pub ids: Vec<String>,
    #[serde(default)]
    pub title: String,
    /// A directory of the official UC folders, relative to the repository root.
    #[serde(default)]
    pub fixture: Option<String>,
    #[serde(default, rename = "fixtureDeviations")]
    pub fixture_deviations: Vec<FixtureDeviation>,
    #[serde(default)]
    pub compare: CompareConfig,
    pub steps: Vec<Step>,
}

/// An intended difference between the engine's request and the fixture's.
#[derive(Debug, Clone, Deserialize)]
pub struct FixtureDeviation {
    pub path: String,
    #[serde(default)]
    pub reason: String,
}

/// How the Status calls are compared: `ignore` (default) or `shape`.
#[derive(Debug, Clone, Deserialize)]
pub struct CompareConfig {
    #[serde(default = "default_status")]
    pub status: String,
}

fn default_status() -> String {
    "ignore".to_string()
}

impl Default for CompareConfig {
    fn default() -> Self {
        Self {
            status: default_status(),
        }
    }
}

/// One step: a command with its arguments.
#[derive(Debug, Clone, Deserialize)]
pub struct Step {
    pub cmd: String,
    #[serde(default = "empty_object")]
    pub args: Value,
    #[serde(default)]
    pub save: Option<String>,
    #[serde(default)]
    pub expect: Expect,
    #[serde(default = "one")]
    pub repeat: u32,
}

fn empty_object() -> Value {
    Value::Object(Map::new())
}

fn one() -> u32 {
    1
}

/// What a step must return.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Expect {
    /// Fields of the result (dot paths) that must equal the given values.
    #[serde(default)]
    pub fields: Map<String, Value>,
    /// An error code the call must fail with.
    #[serde(default)]
    pub error: Option<String>,
}

impl Scenario {
    /// Read and validate one scenario file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("scenario {}", path.display()))
    }

    pub fn parse(text: &str) -> anyhow::Result<Self> {
        let scenario: Scenario = serde_json::from_str(text)?;
        if scenario.format != 1 {
            bail!("unsupported scenario format {}", scenario.format);
        }
        if scenario.steps.is_empty() {
            bail!("a scenario needs at least one step");
        }
        if scenario.compare.status != "ignore" && scenario.compare.status != "shape" {
            bail!(
                "compare.status must be ignore or shape, not {}",
                scenario.compare.status
            );
        }
        for step in &scenario.steps {
            if !step.args.is_object() {
                bail!("step {}: args must be an object", step.cmd);
            }
            if step.repeat == 0 {
                bail!("step {}: repeat must be at least 1", step.cmd);
            }
        }
        Ok(scenario)
    }
}

/// The scenario files of one group directory, in lexical order.
pub fn scenario_files(group_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(group_dir)
        .with_context(|| format!("reading {}", group_dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    Ok(files)
}

/// The group directories of a scenarios root, in lexical order.
pub fn group_dirs(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .with_context(|| format!("reading {}", root.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    Ok(dirs)
}

// ── References ─────────────────────────────────────────────────────

/// Is `s` a whole-string reference `$name.path`? Returns the name and
/// the path segments. A string that merely starts with `$` (a price, a
/// label) is not one.
fn parse_reference(s: &str) -> Option<(&str, Vec<&str>)> {
    let rest = s.strip_prefix('$')?;
    let mut parts = rest.split('.');
    let name = parts.next()?;
    let valid_name = !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !valid_name {
        return None;
    }
    let path: Vec<&str> = parts.collect();
    let valid_path = path
        .iter()
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
    valid_path.then_some((name, path))
}

/// The value at a dot path (`lines.0.id`): object keys, array indexes.
pub fn lookup<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = match current {
            Value::Object(map) => map.get(*segment)?,
            Value::Array(items) => items.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

/// Replace every reference string of `value` by the saved value it names.
/// A reference to a name no earlier step saved, or to a path the saved
/// result does not have, is an error.
pub fn substitute(value: &Value, saved: &HashMap<String, Value>) -> anyhow::Result<Value> {
    match value {
        Value::String(s) => match parse_reference(s) {
            None => Ok(value.clone()),
            Some((name, path)) => {
                let base = saved
                    .get(name)
                    .with_context(|| format!("reference {s}: nothing was saved as {name}"))?;
                lookup(base, &path)
                    .cloned()
                    .with_context(|| format!("reference {s}: no such path in the saved result"))
            }
        },
        Value::Array(items) => items
            .iter()
            .map(|v| substitute(v, saved))
            .collect::<anyhow::Result<Vec<_>>>()
            .map(Value::Array),
        Value::Object(map) => map
            .iter()
            .map(|(k, v)| substitute(v, saved).map(|v| (k.clone(), v)))
            .collect::<anyhow::Result<Map<_, _>>>()
            .map(Value::Object),
        other => Ok(other.clone()),
    }
}

// ── Expect ─────────────────────────────────────────────────────────

/// The code of an engine error string: the part before the first `|`
/// (`pin_locked|seconds=30` is `pin_locked`).
pub fn error_code(error: &str) -> &str {
    error.split('|').next().unwrap_or(error)
}

/// Does `text` carry `code` as a whole word (letters, digits and
/// underscores around it are not allowed)? An engine error is
/// `code|k=v`; an FDM refusal is free text that names the FDM's code
/// (`FDM error: GraphQL error: FDM_LOCKED: ...`), as the screen's
/// `translateFdmError` finds it.
pub fn names_code(text: &str, code: &str) -> bool {
    if code.is_empty() {
        return false;
    }
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.match_indices(code).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + code.len()..].chars().next();
        !before.is_some_and(is_word) && !after.is_some_and(is_word)
    })
}

/// Check what a call returned against `expect`. `Ok(())` when it is met,
/// else the reason it is not.
///
/// An expected error `CODE` is met by an error whose code (the part
/// before the first `|`) is `CODE` or that names `CODE` as a whole word,
/// and by a result with `success: false` whose `error` names it so.
pub fn check_expect(expect: &Expect, outcome: &Result<Value, String>) -> Result<(), String> {
    match (&expect.error, outcome) {
        (Some(code), Err(error)) => {
            if error_code(error) == code || names_code(error, code) {
                Ok(())
            } else {
                Err(format!("expected the error {code}, got {error}"))
            }
        }
        (Some(code), Ok(result)) => match result_failure(result) {
            Some(error) if names_code(&error, code) => Ok(()),
            Some(error) => Err(format!("expected the error {code}, got {error}")),
            None => Err(format!("expected the error {code}, the call succeeded")),
        },
        (None, Err(error)) => Err(format!("the call failed: {error}")),
        (None, Ok(result)) => {
            if let Some(error) = result_failure(result) {
                return Err(format!("success is false: {error}"));
            }
            check_fields(&expect.fields, result)
        }
    }
}

/// A result object with `success: false`: its `error` text (empty when
/// the object has none).
fn result_failure(result: &Value) -> Option<String> {
    let success = result.get("success")?;
    if success.as_bool() == Some(false) {
        return Some(
            result
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        );
    }
    None
}

fn check_fields(fields: &Map<String, Value>, result: &Value) -> Result<(), String> {
    for (path, want) in fields {
        let segments: Vec<&str> = path.split('.').collect();
        match lookup(result, &segments) {
            Some(got) if got == want => {}
            Some(got) => return Err(format!("field {path}: expected {want}, got {got}")),
            None => return Err(format!("field {path}: absent from the result")),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn saved() -> HashMap<String, Value> {
        let mut saved = HashMap::new();
        saved.insert(
            "order".to_string(),
            json!({ "id": 7, "lines": [{ "id": 11 }, { "id": 12 }], "note": null }),
        );
        saved
    }

    #[test]
    fn a_whole_string_reference_is_replaced_by_the_saved_value() {
        let args = json!({ "orderId": "$order.id", "nested": { "line": "$order.lines.1.id" } });
        let got = substitute(&args, &saved()).unwrap();
        assert_eq!(got, json!({ "orderId": 7, "nested": { "line": 12 } }));
    }

    #[test]
    fn a_reference_can_name_an_object_and_a_null() {
        let got = substitute(&json!(["$order.lines.0", "$order.note"]), &saved()).unwrap();
        assert_eq!(got, json!([{ "id": 11 }, null]));
    }

    #[test]
    fn strings_that_are_not_whole_references_stay_as_they_are() {
        let args = json!({ "a": "$5 coffee", "b": "cost $order.id", "c": "$", "d": "$order..id" });
        assert_eq!(substitute(&args, &saved()).unwrap(), args);
    }

    #[test]
    fn a_reference_to_an_unsaved_name_or_path_is_an_error() {
        let unsaved = substitute(&json!("$nobody.id"), &saved()).unwrap_err();
        assert!(
            unsaved.to_string().contains("nothing was saved"),
            "{unsaved}"
        );
        let no_path = substitute(&json!("$order.lines.5.id"), &saved()).unwrap_err();
        assert!(no_path.to_string().contains("no such path"), "{no_path}");
    }

    #[test]
    fn the_default_expect_wants_a_call_without_error_and_success_not_false() {
        let expect = Expect::default();
        assert!(check_expect(&expect, &Ok(json!({ "success": true }))).is_ok());
        assert!(check_expect(&expect, &Ok(json!({ "id": 1 }))).is_ok());
        assert!(check_expect(&expect, &Ok(Value::Null)).is_ok());
        let failed = check_expect(&expect, &Ok(json!({ "success": false, "error": "FDM_X" })));
        assert!(failed.unwrap_err().contains("success is false"));
        let errored = check_expect(&expect, &Err("pin_wrong".to_string()));
        assert!(errored.unwrap_err().contains("pin_wrong"));
    }

    #[test]
    fn expected_fields_use_dot_paths_and_must_equal() {
        let expect = Expect {
            fields: json!({ "ticket.total": 12.5, "lines.0.id": 3 })
                .as_object()
                .unwrap()
                .clone(),
            error: None,
        };
        let ok = json!({ "ticket": { "total": 12.5 }, "lines": [{ "id": 3 }] });
        assert!(check_expect(&expect, &Ok(ok)).is_ok());
        let wrong = json!({ "ticket": { "total": 13.0 }, "lines": [{ "id": 3 }] });
        assert!(
            check_expect(&expect, &Ok(wrong))
                .unwrap_err()
                .contains("ticket.total")
        );
        let absent = json!({ "ticket": { "total": 12.5 } });
        assert!(
            check_expect(&expect, &Ok(absent))
                .unwrap_err()
                .contains("absent")
        );
    }

    #[test]
    fn an_expected_error_matches_the_code_before_the_first_pipe() {
        let expect = Expect {
            fields: Map::new(),
            error: Some("pin_locked".to_string()),
        };
        assert!(check_expect(&expect, &Err("pin_locked|seconds=30".to_string())).is_ok());
        assert!(check_expect(&expect, &Err("pin_wrong".to_string())).is_err());
        // A longer code that merely starts with the expected one is another code.
        assert!(check_expect(&expect, &Err("pin_locked_more".to_string())).is_err());
        assert!(check_expect(&expect, &Ok(json!({ "success": true }))).is_err());
    }

    #[test]
    fn an_expected_error_also_matches_a_result_with_success_false() {
        let expect = Expect {
            fields: Map::new(),
            error: Some("FDM_LOCKED".to_string()),
        };
        let locked = json!({ "success": false, "error": "FDM_LOCKED: the FDM is locked" });
        assert!(check_expect(&expect, &Ok(locked)).is_ok());
        // The engine words an FDM refusal as free text that names the code.
        let worded = json!({
            "success": false,
            "error": "FDM error: GraphQL error: FDM_LOCKED: Le FDM est verrouillé"
        });
        assert!(check_expect(&expect, &Ok(worded)).is_ok());
        // Another code that merely contains the expected one is another code.
        let longer = json!({ "success": false, "error": "FDM_LOCKED_BY_SPF" });
        assert!(check_expect(&expect, &Ok(longer)).is_err());
        let other = json!({ "success": false, "error": "BUFFER_FULL" });
        assert!(check_expect(&expect, &Ok(other)).is_err());
    }

    #[test]
    fn a_scenario_is_validated_when_it_is_parsed() {
        let good = r#"{"format":1,"id":"X","steps":[{"cmd":"logout"}]}"#;
        let scenario = Scenario::parse(good).unwrap();
        assert_eq!(scenario.steps[0].repeat, 1);
        assert_eq!(scenario.compare.status, "ignore");
        assert!(Scenario::parse(r#"{"format":2,"id":"X","steps":[{"cmd":"a"}]}"#).is_err());
        assert!(Scenario::parse(r#"{"format":1,"id":"X","steps":[]}"#).is_err());
        let bad_repeat = r#"{"format":1,"id":"X","steps":[{"cmd":"a","repeat":0}]}"#;
        assert!(Scenario::parse(bad_repeat).is_err());
        let bad_status =
            r#"{"format":1,"id":"X","compare":{"status":"all"},"steps":[{"cmd":"a"}]}"#;
        assert!(Scenario::parse(bad_status).is_err());
        let bad_args = r#"{"format":1,"id":"X","steps":[{"cmd":"a","args":[1]}]}"#;
        assert!(Scenario::parse(bad_args).is_err());
    }
}
