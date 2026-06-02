//! SCE 2.0 Basic Set v1.7 fixture validation.
//!
//! Walks every UC folder under `docs/20250916_UC_BasicSet_POS_FDM_1.7/` and
//! validates that:
//!
//! 1. The published `*_Response.json` deserializes cleanly into `SignResult`
//!    (or `FdmStatus` for the Q* status queries). This is the most important
//!    check — it proves we can correctly parse responses from a real FDM.
//! 2. The published `*_GQL-Variables.json` carries the field set we expect
//!    for each event type (top-level shape check, no struct round-trip
//!    because our `*Input` types are `Serialize`-only).
//! 3. UC-specific invariants: e.g. M171 training response has no `vatCalc`;
//!    M121 (signOrder, event P) has no `shortSignature` / `verificationUrl`;
//!    M110 (sale, event N) has all three.
//!
//! When this whole file passes, the v1.7 contract is met on the receive side
//! and the dossier can claim the fiscal core "speaks the spec" for Basic Set.

use std::fs;
use std::path::{Path, PathBuf};

use pos_fiscal::response::{FdmStatus, SignResult};
use pos_fiscal::types::{EventLabel, EventOperation};

// ── Fixture locator ────────────────────────────────────────────────

/// Absolute path to the Basic Set v1.7 fixture root, computed from the
/// crate's own manifest dir at compile time so this works no matter where
/// `cargo test` is invoked from.
fn basic_set_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..") // crates/pos-fiscal → crates/
        .join("..") // crates/ → workspace root
        .join("docs")
        .join("20250916_UC_BasicSet_POS_FDM_1.7")
}

/// One discovered fixture set within a single UC folder.
struct UcFixture {
    /// e.g. "M110_signSale" — the folder name, used as test ID.
    uc_id: String,
    /// Absolute path to the `*_GQL-Variables.json` file. None for status
    /// queries (Q*), which carry a `*_Request.json` instead.
    variables_path: Option<PathBuf>,
    /// Absolute path to the `*_Response.json` (or `status_response.json`).
    response_path: PathBuf,
    /// What flavour of response the JSON carries.
    response_kind: ResponseKind,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ResponseKind {
    /// A `signSale` / `signOrder` / `signReport*` / … response. Wrapped
    /// in `data.<mutationName>` and shaped like `SignResult`.
    SignResult,
    /// A `Status` query response, shaped like `FdmStatus`.
    Status,
}

fn discover_fixtures() -> Vec<UcFixture> {
    let root = basic_set_root();
    let mut out = Vec::new();

    let entries = fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("cannot read fixture root {}: {e}", root.display()));

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        // UC folder names start with M (mutation) or Q (query).
        if !(name.starts_with('M') || name.starts_with('Q')) {
            continue;
        }

        // Case 1: single-step UC — fixture files live directly in this
        // folder (M110, M121, …, Q100).
        if let Some((response_path, response_kind)) = find_response_file(&path) {
            let variables_path = find_variables_file(&path);
            out.push(UcFixture {
                uc_id: name.clone(),
                variables_path,
                response_path,
                response_kind,
            });
            continue;
        }

        // Case 2: multi-step UC — fixture triples live in sub-folders
        // like `M112A_1_OriginalSale/`, `M112A_2_PartialRefund/`. Each
        // sub-step is registered as its own fixture so the test matrix
        // covers the whole flow (the cert manual tests them step-by-step).
        let sub_entries = match fs::read_dir(&path) {
            Ok(it) => it,
            Err(_) => continue,
        };
        for sub in sub_entries.flatten() {
            let sub_path = sub.path();
            if !sub_path.is_dir() {
                continue;
            }
            let Some((response_path, response_kind)) = find_response_file(&sub_path) else {
                continue;
            };
            let variables_path = find_variables_file(&sub_path);
            let sub_name = sub.file_name().to_string_lossy().into_owned();
            out.push(UcFixture {
                // Compose "M112A_signSale_RefundPartial/M112A_2_PartialRefund"
                uc_id: format!("{name}/{sub_name}"),
                variables_path,
                response_path,
                response_kind,
            });
        }
    }

    out.sort_by(|a, b| a.uc_id.cmp(&b.uc_id));
    out
}

fn find_response_file(dir: &Path) -> Option<(PathBuf, ResponseKind)> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if !name.ends_with(".json") {
            continue;
        }
        if name.contains("response") {
            let kind = if name.starts_with("status") {
                ResponseKind::Status
            } else {
                ResponseKind::SignResult
            };
            return Some((entry.path(), kind));
        }
    }
    None
}

fn find_variables_file(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let fname = entry.file_name();
        let name = fname.to_string_lossy();
        if name.ends_with("_GQL-Variables.json") {
            return Some(entry.path());
        }
    }
    None
}

// ── Per-UC validators ──────────────────────────────────────────────

/// Deserialize the response under `data.<key>`. For mutations the
/// envelope has exactly one inner key. For some queries — Q110 in
/// particular — the envelope can contain extra sibling keys (e.g.
/// `messagesFR/NL/DE`) alongside the primary `status` payload; we
/// fall back to whichever named candidate is present.
fn extract_data_payload<'a>(
    json: &'a serde_json::Value,
    fallback_keys: &[&str],
) -> &'a serde_json::Value {
    let data = json
        .get("data")
        .unwrap_or_else(|| panic!("response JSON has no top-level `data` field: {json}"));
    let obj = data
        .as_object()
        .unwrap_or_else(|| panic!("data is not an object: {json}"));
    // Mutation envelopes have a single inner key — accept that path first.
    if obj.len() == 1 {
        return obj.values().next().expect("len==1");
    }
    // Otherwise pick the named candidate (e.g. "status" for Q110).
    for key in fallback_keys {
        if let Some(v) = obj.get(*key) {
            return v;
        }
    }
    panic!(
        "data has multiple keys ({:?}) and none of {:?} matched",
        obj.keys().collect::<Vec<_>>(),
        fallback_keys
    );
}

fn parse_sign_result(path: &Path) -> SignResult {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let envelope: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", path.display()));
    // Sign mutations always have exactly one inner key — no fallback needed.
    let payload = extract_data_payload(&envelope, &[]);
    serde_json::from_value(payload.clone()).unwrap_or_else(|e| {
        panic!(
            "{} response did not deserialize into SignResult: {e}\npayload was: {payload}",
            path.display()
        )
    })
}

fn parse_fdm_status(path: &Path) -> FdmStatus {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let envelope: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", path.display()));
    // Q100 has a single "Status" key; Q110 wraps the same payload under
    // "status" alongside per-language `messagesFR/NL/DE` siblings.
    let payload = extract_data_payload(&envelope, &["Status", "status"]);
    serde_json::from_value(payload.clone()).unwrap_or_else(|e| {
        panic!(
            "{} status did not deserialize into FdmStatus: {e}\npayload was: {payload}",
            path.display()
        )
    })
}

fn read_variables(path: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", path.display()))
}

/// Generic check that the `data` payload of `Variables.json` carries
/// the SCE 2.0 common fields every event must include. The `*Input`
/// structs flatten `CommonFields`, so these are the keys we'd expect
/// to round-trip through any of our builders.
fn assert_common_fields_present(uc_id: &str, vars: &serde_json::Value) {
    let data = vars.get("data").unwrap_or_else(|| {
        panic!("{uc_id}: Variables.json has no `data` envelope")
    });
    let required: &[&str] = &[
        "posId",
        "posFiscalTicketNo",
        "posDateTime",
        "deviceId",
        "vatNo",
        "estNo",
        "posSwVersion",
        "bookingPeriodId",
        "bookingDate",
        "ticketMedium",
        "employeeId",
    ];
    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|f| data.get(*f).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "{uc_id}: Variables.json missing common fields: {missing:?}\nfull data keys: {:?}",
        data.as_object()
            .map(|o| o.keys().collect::<Vec<_>>())
            .unwrap_or_default()
    );
}

// ── The actual test ────────────────────────────────────────────────

/// Single roll-up test that walks every UC folder and accumulates results.
/// Reporting as one test (vs one #[test] per UC) keeps the output compact
/// while still failing loudly with a precise per-UC diff.
#[test]
fn basic_set_v1_7_fixtures_deserialize_and_match_schema() {
    let fixtures = discover_fixtures();
    assert!(
        !fixtures.is_empty(),
        "no fixtures found under {}",
        basic_set_root().display()
    );

    // Track per-UC outcomes for the dossier.
    let mut passed: Vec<String> = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();
    // EventLabel doesn't derive Hash, so we de-dupe with a Vec instead
    // of a HashSet. The set is tiny (<= 8 elements).
    let mut event_labels_seen: Vec<EventLabel> = Vec::new();

    for fx in &fixtures {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            match fx.response_kind {
                ResponseKind::SignResult => {
                    let result = parse_sign_result(&fx.response_path);

                    // ── Event-type invariants (per spec POS v2.2 §253-376) ──
                    let label = result.fdm_ref.event_label;
                    if !event_labels_seen.contains(&label) {
                        event_labels_seen.push(label);
                    }

                    // Only N events (real fiscal sales) carry the
                    // short signature + customer verification URL.
                    if label == EventLabel::N {
                        assert!(
                            result.short_signature.is_some(),
                            "{}: N event without shortSignature",
                            fx.uc_id
                        );
                        assert!(
                            result.verification_url.is_some(),
                            "{}: N event without verificationUrl",
                            fx.uc_id
                        );
                    } else {
                        assert!(
                            result.short_signature.is_none(),
                            "{}: {:?} event must not carry shortSignature",
                            fx.uc_id,
                            label
                        );
                        assert!(
                            result.verification_url.is_none(),
                            "{}: {:?} event must not carry verificationUrl",
                            fx.uc_id,
                            label
                        );
                    }

                    // Training sales (M171/M171B/M172) explicitly omit
                    // vatCalc in v1.7 — the FDM does not produce a VAT
                    // breakdown for non-fiscal events (changelog
                    // entry "M171: correction in response FDM (no vatCalc)").
                    if label == EventLabel::T
                        && (result.event_operation == EventOperation::Sale
                            || result.event_operation == EventOperation::Copy)
                    {
                        assert!(
                            result.vat_calc.is_none(),
                            "{}: training {:?} must omit vatCalc (v1.6+)",
                            fx.uc_id,
                            result.event_operation
                        );
                    }

                    // shortSignature, when present, must be 40 hex chars
                    // (SHA-1 in hex per the FDM spec).
                    if let Some(ref sig) = result.short_signature {
                        assert_eq!(
                            sig.len(),
                            40,
                            "{}: shortSignature must be 40-char hex SHA-1, got {} chars",
                            fx.uc_id,
                            sig.len()
                        );
                        assert!(
                            sig.chars().all(|c| c.is_ascii_hexdigit()),
                            "{}: shortSignature contains non-hex chars",
                            fx.uc_id
                        );
                    }
                }
                ResponseKind::Status => {
                    let status = parse_fdm_status(&fx.response_path);
                    // The status query must always identify the device
                    // and tell us whether it is initialised.
                    assert!(
                        !status.device.fdm_id.is_empty(),
                        "{}: status response carries empty fdmId",
                        fx.uc_id
                    );
                }
            }

            // ── Variables.json schema check (mutation UCs only) ────
            if let Some(ref vp) = fx.variables_path {
                let vars = read_variables(vp);
                assert_common_fields_present(&fx.uc_id, &vars);
            }
        }));

        match outcome {
            Ok(()) => passed.push(fx.uc_id.clone()),
            Err(e) => {
                let msg = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "<non-string panic>".to_string());
                failed.push((fx.uc_id.clone(), msg));
            }
        }
    }

    // Always print the matrix so the dossier capture is easy.
    eprintln!("\n── Basic Set v1.7 fixture coverage ──");
    eprintln!("Total UCs discovered: {}", fixtures.len());
    eprintln!("Event labels exercised: {:?}", event_labels_seen);
    eprintln!("Passed ({}):", passed.len());
    for uc in &passed {
        eprintln!("  ✓ {uc}");
    }
    if !failed.is_empty() {
        eprintln!("Failed ({}):", failed.len());
        for (uc, err) in &failed {
            eprintln!("  ✗ {uc}\n    {err}");
        }
        panic!("{} of {} Basic Set fixtures failed", failed.len(), fixtures.len());
    }
}

// ── Targeted hand-written checks for the tricky UCs ────────────────

/// M110: standard direct sale (N). Most important fixture — anchors the
/// "happy path" and the v1.7 footer field shape.
#[test]
fn m110_sign_sale_response_shape() {
    let p = basic_set_root().join("M110_signSale").join("signSale_Response.json");
    let r = parse_sign_result(&p);
    assert_eq!(r.event_operation, EventOperation::Sale);
    assert_eq!(r.fdm_ref.event_label, EventLabel::N);
    assert!(r.short_signature.is_some());
    assert!(r.verification_url.is_some());
    assert!(r.vat_calc.is_some(), "N event must include VAT calc");
    assert_eq!(r.fdm_ref.event_counter, 15);
    assert_eq!(r.fdm_ref.total_counter, 59);
}

/// M121: signOrder (P event). Spec POS v2.2: no shortSignature, no
/// verificationUrl, no vatCalc. v1.7 also omits the `footer` field
/// entirely — proves our `#[serde(default)]` on footer is needed.
#[test]
fn m121_sign_order_response_omits_n_only_fields() {
    let p = basic_set_root().join("M121_signOrder").join("signOrder_Response.json");
    let r = parse_sign_result(&p);
    assert_eq!(r.event_operation, EventOperation::Order);
    assert_eq!(r.fdm_ref.event_label, EventLabel::P);
    assert!(r.short_signature.is_none());
    assert!(r.verification_url.is_none());
    assert!(r.vat_calc.is_none());
    assert!(r.footer.is_empty(), "absent footer must default to vec![]");
}

/// M171: training sale (T event). v1.7 changelog: "M171: correction in
/// response FDM (no vatCalc)". This is the spec test for our training
/// flow — the FDM signs the event so we get a digitalSignature, but
/// since training is non-fiscal there is no VAT breakdown.
#[test]
fn m171_training_sale_has_no_vat_calc() {
    let p = basic_set_root()
        .join("M171_signSale_Training")
        .join("signSale_Response.json");
    let r = parse_sign_result(&p);
    assert_eq!(r.event_operation, EventOperation::Sale);
    assert_eq!(r.fdm_ref.event_label, EventLabel::T);
    assert!(r.vat_calc.is_none(), "training event must omit vatCalc");
    assert!(r.short_signature.is_none(), "training event has no shortSignature");
    assert!(r.verification_url.is_none(), "training event has no verificationUrl");
    assert!(!r.digital_signature.is_empty(), "training event still signs");
}

/// Q100: status query. Single check — confirms our `FdmStatus` shape
/// matches what the FDM emits for the introspection query the POS uses
/// to gate transactions when buffer/errors trip thresholds.
#[test]
fn q100_status_response_shape() {
    let p = basic_set_root().join("Q100_Status").join("status_response.json");
    let s = parse_fdm_status(&p);
    assert!(!s.device.fdm_id.is_empty());
    assert!(!s.device.fdm_sw_version.is_empty());
}
