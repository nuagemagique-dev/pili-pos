//! SCE 2.0 Reports v1.3 fixture validation.
//!
//! Walks `docs/20260821_UC_Reports_1.3/` and validates:
//!
//! 1. **M180–M184**: every report response deserializes cleanly into
//!    `SignResult`. M184 is the spec-mandated "combined Z" emission -
//!    a single GraphQL mutation that aliases both `signReportTurnoverZ`
//!    and `signReportUserZ`; the test pulls both `SignResult`s out of
//!    one envelope and checks counters increment monotonically across
//!    them (`totalCounter` for Z+1 == `totalCounter` for Z, +1).
//! 2. **InputReport/T1–T8**: the staging sales/orders that have to be
//!    fed to the FDM before a meaningful Z can be emitted. Each
//!    staging response is just a normal `SignResult`; we validate the
//!    shape so we know the test campaign's prerequisite events also
//!    parse with our deserializer.
//! 3. **v1.3 schema drift**:
//!    - X-reports must NOT carry `reportBookingDate` (changelog entry).
//!    - Z-reports MUST carry `reportBookingDate`.
//!
//!    The matching `inputs::ReportTurnoverXInput` / `ReportUserXInput`
//!    structs have no `report_booking_date` field, and the Z variants
//!    have it as required - this test asserts that the fixtures agree.
//!
//! When this passes, the v1.3 contract is met for reports.

use std::fs;
use std::path::{Path, PathBuf};

use pos_fiscal::response::SignResult;
use pos_fiscal::types::{EventLabel, EventOperation};

fn reports_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("20260821_UC_Reports_1.3")
}

/// Re-used envelope unwrapper from the Basic Set test. Mutation
/// envelopes have exactly one inner key, except for M184 which has
/// two (`signReportTurnoverZ` + `signReportUserZ`) - that one is
/// handled separately below.
fn extract_single_payload(json: &serde_json::Value) -> &serde_json::Value {
    let data = json
        .get("data")
        .unwrap_or_else(|| panic!("response JSON has no top-level `data`: {json}"));
    let obj = data
        .as_object()
        .unwrap_or_else(|| panic!("data is not an object: {json}"));
    assert_eq!(
        obj.len(),
        1,
        "expected exactly one inner key under data, got {} ({:?})",
        obj.len(),
        obj.keys().collect::<Vec<_>>()
    );
    obj.values().next().expect("len==1")
}

fn read_json(path: &Path) -> serde_json::Value {
    let raw =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", path.display()))
}

fn parse_sign_result(path: &Path) -> SignResult {
    let envelope = read_json(path);
    let payload = extract_single_payload(&envelope);
    serde_json::from_value(payload.clone()).unwrap_or_else(|e| {
        panic!(
            "{} did not deserialize into SignResult: {e}\npayload: {payload}",
            path.display()
        )
    })
}

/// Parse the dual-key envelope used by M184 (combined Z). Returns
/// `(turnover_z_result, user_z_result)`.
fn parse_combined_z(path: &Path) -> (SignResult, SignResult) {
    let envelope = read_json(path);
    let data = envelope
        .get("data")
        .unwrap_or_else(|| panic!("M184 envelope missing data: {envelope}"));
    let turnover_v = data
        .get("signReportTurnoverZ")
        .unwrap_or_else(|| panic!("M184 missing signReportTurnoverZ payload"));
    let user_v = data
        .get("signReportUserZ")
        .unwrap_or_else(|| panic!("M184 missing signReportUserZ payload"));
    let turnover: SignResult = serde_json::from_value(turnover_v.clone())
        .unwrap_or_else(|e| panic!("M184 signReportTurnoverZ parse: {e}"));
    let user: SignResult = serde_json::from_value(user_v.clone())
        .unwrap_or_else(|e| panic!("M184 signReportUserZ parse: {e}"));
    (turnover, user)
}

/// Find the lone `*Response.json` file in a fixture folder.
fn find_response(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let fname = entry.file_name();
        let name = fname.to_string_lossy().to_lowercase();
        let is_json = Path::new(&name)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json"));
        if is_json && name.contains("response") {
            return Some(entry.path());
        }
    }
    None
}

fn find_variables(dir: &Path) -> Option<PathBuf> {
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

// ── Per-UC tests ───────────────────────────────────────────────────

#[test]
fn m180_turnover_x_response_parses() {
    let path = reports_root()
        .join("M180_signReportTurnoverX")
        .join("signReportTurnoverX_Response.json");
    let r = parse_sign_result(&path);
    assert_eq!(r.event_operation, EventOperation::ReportTurnoverX);
    assert_eq!(r.fdm_ref.event_label, EventLabel::R);
    // Reports are non-N events - no shortSignature/verificationUrl.
    assert!(r.short_signature.is_none());
    assert!(r.verification_url.is_none());
}

#[test]
fn m181_turnover_z_response_parses() {
    let path = reports_root()
        .join("M181_signReportTurnoverZ")
        .join("signReportTurnoverZ_Response.json");
    let r = parse_sign_result(&path);
    assert_eq!(r.event_operation, EventOperation::ReportTurnoverZ);
    assert_eq!(r.fdm_ref.event_label, EventLabel::R);
}

#[test]
fn m182_user_x_response_parses() {
    let path = reports_root()
        .join("M182_signReportUserX")
        .join("signReportUserX_Response.json");
    let r = parse_sign_result(&path);
    assert_eq!(r.event_operation, EventOperation::ReportUserX);
    assert_eq!(r.fdm_ref.event_label, EventLabel::R);
}

#[test]
fn m183_user_z_response_parses() {
    let path = reports_root()
        .join("M183_signReportUserZ")
        .join("signReportUserZ_Response.json");
    let r = parse_sign_result(&path);
    assert_eq!(r.event_operation, EventOperation::ReportUserZ);
    assert_eq!(r.fdm_ref.event_label, EventLabel::R);
}

/// M184: spec-mandated combined Z emission. Both mutations are sent
/// in one GraphQL operation; the FDM signs and increments counters
/// in sequence, so `totalCounter` should differ by exactly 1 between
/// the two responses. This is the canonical end-of-day flow the cert
/// manual expects, and it's the test our `trigger_end_of_day_z_report`
/// command in Tauri has to produce in real life.
#[test]
fn m184_combined_z_response_parses_and_counters_increment() {
    let path = reports_root()
        .join("M184_signReportTurnoverZUserZ")
        .join("signReportTurnoverZUserZ_Response.json");
    let (turnover, user) = parse_combined_z(&path);

    assert_eq!(turnover.event_operation, EventOperation::ReportTurnoverZ);
    assert_eq!(turnover.fdm_ref.event_label, EventLabel::R);
    assert_eq!(user.event_operation, EventOperation::ReportUserZ);
    assert_eq!(user.fdm_ref.event_label, EventLabel::R);

    // Counters must increment monotonically across the dual emission.
    assert_eq!(
        user.fdm_ref.total_counter,
        turnover.fdm_ref.total_counter + 1,
        "UserZ totalCounter must be TurnoverZ totalCounter + 1"
    );
    // Both must reference the same FDM (single FDM in the test setup).
    assert_eq!(turnover.fdm_ref.fdm_id, user.fdm_ref.fdm_id);
}

// ── v1.3 schema drift assertions ───────────────────────────────────

/// X-reports must NOT carry `reportBookingDate` in their Variables
/// (changelog v1.3: "Removed `reportBookingDate` from X-reports").
/// Our `inputs::ReportTurnoverXInput` / `ReportUserXInput` already
/// omit it; this test catches anyone who tries to re-add the field
/// later by checking the published fixtures.
#[test]
fn x_reports_omit_report_booking_date() {
    for uc in ["M180_signReportTurnoverX", "M182_signReportUserX"] {
        let dir = reports_root().join(uc);
        let vp = find_variables(&dir).unwrap_or_else(|| panic!("{uc}: no Variables.json found"));
        let vars = read_json(&vp);
        let data = vars
            .get("data")
            .unwrap_or_else(|| panic!("{uc}: Variables.json has no data envelope"));
        assert!(
            data.get("reportBookingDate").is_none(),
            "{uc}: X-report Variables.json must not carry reportBookingDate (v1.3)"
        );
    }
}

/// Z-reports MUST carry `reportBookingDate` - it identifies which
/// calendar day the Z is closing. Our `inputs::ReportTurnoverZInput`
/// / `ReportUserZInput` have it as required.
#[test]
fn z_reports_include_report_booking_date() {
    for uc in ["M181_signReportTurnoverZ", "M183_signReportUserZ"] {
        let dir = reports_root().join(uc);
        let vp = find_variables(&dir).unwrap_or_else(|| panic!("{uc}: no Variables.json found"));
        let vars = read_json(&vp);
        let data = vars
            .get("data")
            .unwrap_or_else(|| panic!("{uc}: Variables.json has no data envelope"));
        assert!(
            data.get("reportBookingDate").is_some(),
            "{uc}: Z-report Variables.json must carry reportBookingDate"
        );
    }

    // M184: combined Z carries TWO report payloads under separate
    // variable names (`turnover` and `user`), each must include
    // reportBookingDate.
    let m184 = reports_root()
        .join("M184_signReportTurnoverZUserZ")
        .join("M184_signReportTurnoverZUserZ_GQL-Variables.json");
    let vars = read_json(&m184);
    let turnover = vars.get("turnover").expect("M184 variables: turnover");
    let user = vars.get("user").expect("M184 variables: user");
    assert!(
        turnover.get("reportBookingDate").is_some(),
        "M184 turnover.reportBookingDate missing"
    );
    assert!(
        user.get("reportBookingDate").is_some(),
        "M184 user.reportBookingDate missing"
    );
}

// ── InputReport staging fixtures (T1–T8) ───────────────────────────

/// Walks the `InputReport/Ti` folders and ensures each staging
/// response parses. These are the sale/order/refund/pre-bill events
/// that have to be in the FDM before a meaningful Z is produced. If
/// we can read these responses, we can also replay them in a
/// rehearsal harness against the real FDM (Phase 11).
#[test]
fn input_report_staging_fixtures_parse() {
    let input_root = reports_root().join("InputReport");
    let entries = fs::read_dir(&input_root)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", input_root.display()));

    let mut found = 0_usize;
    let mut labels: Vec<EventLabel> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let fname = entry.file_name();
        let name = fname.to_string_lossy();
        if !name.starts_with('T') {
            continue;
        }
        let Some(rp) = find_response(&path) else {
            continue;
        };

        let r = parse_sign_result(&rp);
        if !labels.contains(&r.fdm_ref.event_label) {
            labels.push(r.fdm_ref.event_label);
        }
        // N events must be fully signed; P events must not be.
        match r.fdm_ref.event_label {
            EventLabel::N => {
                assert!(
                    r.short_signature.is_some(),
                    "{name}: staged N event without shortSignature"
                );
                assert!(
                    r.verification_url.is_some(),
                    "{name}: staged N event without verificationUrl"
                );
            }
            EventLabel::P => {
                assert!(
                    r.short_signature.is_none(),
                    "{name}: staged P event with shortSignature"
                );
            }
            _ => {}
        }
        found += 1;
    }

    assert!(
        found >= 8,
        "expected 8 InputReport staging fixtures, found {found}"
    );
    // The published staging set exercises both N (sales/refunds) and
    // P (orders/pre-bills) - the two event flavours a Z aggregates.
    assert!(labels.contains(&EventLabel::N), "staging missing N events");
    assert!(labels.contains(&EventLabel::P), "staging missing P events");
}
