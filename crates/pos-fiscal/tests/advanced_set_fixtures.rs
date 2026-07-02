//! SCE 2.0 Advanced Set v1.5 fixture validation.
//!
//! Walks `docs/20250923_UC_AdvancedSet_POS_FDM_1.5/` and validates that
//! every published response deserializes into `SignResult`. The Advanced
//! Set covers composite products, price changes, table split/merge, MPV/SPV
//! vouchers, foreign currency, customer accounts, etc. — the "optional but
//! restaurant-realistic" flows in the cert manual.
//!
//! Approach mirrors `uc_fixtures.rs` (Basic Set):
//! - Walk top-level UCnnn folders.
//! - Single-step UCs have fixture files directly inside.
//! - Multi-step UCs have sub-folders like `UC240_1_Order/` that each carry
//!   a fixture triple; the loader recurses one level deeper.
//!
//! The roll-up test produces a coverage matrix to stderr so the
//! `ADVANCED_SET_COVERAGE.md` dossier artifact can be regenerated from the
//! same source-of-truth.

use std::fs;
use std::path::{Path, PathBuf};

use pos_fiscal::response::SignResult;
use pos_fiscal::types::EventLabel;

fn advanced_set_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("20250923_UC_AdvancedSet_POS_FDM_1.5")
}

struct UcFixture {
    uc_id: String,
    response_path: PathBuf,
    /// Some Advanced Set UCs (the M184-like combined emissions) carry
    /// multiple top-level keys under `data`. We don't enforce the
    /// single-key invariant here.
    is_dual_envelope: bool,
}

fn find_response(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let fname = entry.file_name();
        let name = fname.to_string_lossy().to_lowercase();
        if name.ends_with(".json") && name.contains("response") {
            return Some(entry.path());
        }
    }
    None
}

fn read_json(path: &Path) -> serde_json::Value {
    let raw =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", path.display()))
}

fn discover_fixtures() -> Vec<UcFixture> {
    let root = advanced_set_root();
    let mut out = Vec::new();

    let entries = fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("cannot read fixture root {}: {e}", root.display()));

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        // Folder names start with UC.
        if !name.starts_with("UC") {
            continue;
        }

        // Single-step UC?
        if let Some(rp) = find_response(&path) {
            out.push(UcFixture {
                uc_id: name.clone(),
                response_path: rp,
                is_dual_envelope: false,
            });
            continue;
        }

        // Multi-step UC: scan sub-folders for fixtures.
        let Ok(sub_entries) = fs::read_dir(&path) else {
            continue;
        };
        for sub in sub_entries.flatten() {
            let sub_path = sub.path();
            if !sub_path.is_dir() {
                continue;
            }
            let Some(rp) = find_response(&sub_path) else {
                continue;
            };
            let sub_name = sub.file_name().to_string_lossy().into_owned();
            out.push(UcFixture {
                uc_id: format!("{name}/{sub_name}"),
                response_path: rp,
                is_dual_envelope: false,
            });
        }
    }

    out.sort_by(|a, b| a.uc_id.cmp(&b.uc_id));
    out
}

/// Best-effort parse: try the single-payload form first; if `data`
/// has multiple keys (combined emission, like M184's pattern), parse
/// each value as `SignResult` and return them all. Both branches must
/// yield a non-empty list of `SignResult`s.
fn parse_response_flexible(path: &Path) -> (Vec<SignResult>, bool) {
    let envelope = read_json(path);
    let data = envelope
        .get("data")
        .unwrap_or_else(|| panic!("{}: response has no `data`", path.display()));
    let obj = data
        .as_object()
        .unwrap_or_else(|| panic!("{}: data not an object", path.display()));

    let mut results = Vec::new();
    let dual = obj.len() > 1;
    for (key, val) in obj.iter() {
        let parsed: SignResult = serde_json::from_value(val.clone()).unwrap_or_else(|e| {
            panic!(
                "{}: payload `{key}` did not parse into SignResult: {e}\nvalue: {val}",
                path.display()
            )
        });
        results.push(parsed);
    }
    (results, dual)
}

/// Roll-up over every Advanced Set fixture. Records per-UC outcomes so
/// the cert dossier captures coverage at a glance. Any fixture that fails
/// to deserialize blocks the test and prints the offending path + raw
/// payload so the gap can be triaged.
#[test]
fn advanced_set_v1_5_fixtures_deserialize() {
    let fixtures = discover_fixtures();
    assert!(
        !fixtures.is_empty(),
        "no fixtures found under {}",
        advanced_set_root().display()
    );

    let mut passed: Vec<String> = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();
    let mut event_labels_seen: Vec<EventLabel> = Vec::new();
    let mut dual_envelope_uc_count = 0_usize;

    for fx in &fixtures {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let (results, dual) = parse_response_flexible(&fx.response_path);
            assert!(!results.is_empty(), "{}: zero SignResults parsed", fx.uc_id);
            for r in &results {
                if !event_labels_seen.contains(&r.fdm_ref.event_label) {
                    event_labels_seen.push(r.fdm_ref.event_label);
                }
            }
            dual
        }));

        match outcome {
            Ok(dual) => {
                passed.push(fx.uc_id.clone());
                if dual {
                    dual_envelope_uc_count += 1;
                }
            }
            Err(e) => {
                let msg = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "<non-string panic>".to_string());
                failed.push((fx.uc_id.clone(), msg));
            }
        }
        // Avoid unused-variable warning for is_dual_envelope across the
        // discovery side — keep the field for documentation purposes.
        let _ = fx.is_dual_envelope;
    }

    eprintln!("\n── Advanced Set v1.5 fixture coverage ──");
    eprintln!("Total fixtures discovered: {}", fixtures.len());
    eprintln!("Event labels seen: {event_labels_seen:?}");
    eprintln!("Dual-envelope UCs: {dual_envelope_uc_count}");
    eprintln!("Passed ({}):", passed.len());
    for uc in &passed {
        eprintln!("  ✓ {uc}");
    }
    if !failed.is_empty() {
        eprintln!("Failed ({}):", failed.len());
        for (uc, err) in &failed {
            eprintln!("  ✗ {uc}\n    {}", err.lines().next().unwrap_or(err));
        }
        panic!(
            "{} of {} Advanced Set fixtures failed",
            failed.len(),
            fixtures.len()
        );
    }
}
