//! The host reference run end to end: the smoke group of the repository
//! (`android/app/src/androidTest/assets/uc/00_smoke`) recorded with the
//! engine on the host, recorded twice, and compared with itself.

#![expect(
    clippy::assert_is_empty,
    reason = "assert!(!x.is_empty()) states the check directly"
)]

use std::path::{Path, PathBuf};

use pos_uc_check::compare::{self, Options};
use pos_uc_check::journal::Journal;
use pos_uc_check::record::{self, RecordOptions};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root")
}

fn options(scenarios: &Path, out: &Path) -> RecordOptions {
    RecordOptions {
        scenarios: scenarios.to_path_buf(),
        out: out.to_path_buf(),
        groups: vec![],
        only: None,
        repo: repo(),
    }
}

fn smoke_dir() -> PathBuf {
    repo().join("android/app/src/androidTest/assets/uc")
}

/// The options for the smoke group only: the other groups hold the UC scenarios.
fn smoke_options(out: &Path) -> RecordOptions {
    let mut opts = options(&smoke_dir(), out);
    opts.groups = vec!["00_smoke".to_string()];
    opts
}

fn operations(journal: &Journal) -> Vec<&str> {
    journal.fdm.iter().map(|c| c.operation.as_str()).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_smoke_group_records_a_passing_journal_with_a_sale_and_an_x_report() {
    let out = tempfile::tempdir().unwrap();
    let recorded = record::record(&smoke_options(out.path())).await.unwrap();
    let (lines, pass) = record::summary_lines(&recorded);
    assert!(pass, "{lines:?}");
    assert!(
        lines[0].starts_with("UCREC | 00_smoke/001_SMOKE | PASS | 5 steps,"),
        "{lines:?}"
    );
    assert_eq!(
        lines.last().unwrap(),
        "UCREC | total | PASS | 0 failed (1 scenarios)"
    );

    let journal =
        Journal::load(&out.path().join("linux/00_smoke/001_SMOKE.json")).expect("the journal");
    assert_eq!(journal.verdict, "PASS");
    assert_eq!(journal.side, "linux");
    assert!(
        journal.pos_sw_version.starts_with("pili-pos/"),
        "{}",
        journal.pos_sw_version
    );
    let ops = operations(&journal);
    assert!(ops.contains(&"signSale"), "{ops:?}");
    assert!(ops.contains(&"signReportTurnoverX"), "{ops:?}");
    // The clock-in the login queued is settled before the sale.
    assert_eq!(ops[0], "signWorkIn", "{ops:?}");
    assert!(!journal.tickets.is_empty());
    assert!(journal.tickets.iter().any(|t| t.contains("Perrier")));
    assert!(journal.steps.iter().all(|s| s.ok && s.unmet.is_none()));
    // Every sign request carries the journal's posSwVersion.
    for call in &journal.fdm {
        assert_eq!(
            call.request["variables"]["data"]["posSwVersion"],
            journal.pos_sw_version.as_str()
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_recordings_of_the_smoke_group_compare_equal_on_the_same_side() {
    let (first, second) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    for out in [&first, &second] {
        let recorded = record::record(&smoke_options(out.path())).await.unwrap();
        assert!(record::summary_lines(&recorded).1);
    }
    let opts = Options {
        same_side: true,
        fixtures: true,
        repo: repo(),
        ..Options::default()
    };
    let results = compare::compare_dirs(
        &first.path().join("linux"),
        &second.path().join("linux"),
        &opts,
    )
    .unwrap();
    let (lines, pass) = compare::summary_lines(&results);
    assert!(
        pass,
        "{lines:?} {:?}",
        results
            .iter()
            .flat_map(compare::ScenarioResult::detail_lines)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        lines.last().unwrap(),
        "UCCHECK | total | PASS | 0 failed (1 scenarios)"
    );

    // Without --same-side two reference journals are not a valid pair: the
    // Android side must carry pili-pos-android/.
    let strict = Options {
        repo: repo(),
        ..Options::default()
    };
    let results = compare::compare_dirs(
        &first.path().join("linux"),
        &second.path().join("linux"),
        &strict,
    )
    .unwrap();
    assert!(!compare::summary_lines(&results).1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn references_repeat_expected_errors_and_unmet_expectations_are_recorded() {
    let scenarios = tempfile::tempdir().unwrap();
    let group = scenarios.path().join("00_t");
    std::fs::create_dir_all(&group).unwrap();
    std::fs::write(
        group.join("001_OK.json"),
        r#"{
          "format": 1, "id": "T", "ids": ["T"], "title": "t",
          "steps": [
            { "cmd": "login", "args": { "insz": "94060242576", "pin": "9999" }, "expect": { "error": "pin_wrong" } },
            { "cmd": "login", "args": { "insz": "94060242576", "pin": "1111" } },
            { "cmd": "create_order", "args": { "input": { "order_type": "takeaway", "table_number": null, "user_insz": "94060242576" } }, "save": "order" },
            { "cmd": "add_order_line", "args": { "input": { "order_id": "$order.id", "menu_item_id": 12007, "quantity": 1, "modifiers_json": null } }, "repeat": 2 },
            { "cmd": "get_order", "args": { "orderId": "$order.id" }, "expect": { "fields": { "status": "open" } } },
            { "cmd": "sleep_ms", "args": { "ms": 10 } }
          ]
        }"#,
    )
    .unwrap();
    std::fs::write(
        group.join("002_BAD.json"),
        r#"{
          "format": 1, "id": "B", "ids": ["B"], "title": "b",
          "steps": [
            { "cmd": "get_order", "args": { "orderId": 1 }, "expect": { "fields": { "status": "closed" } } },
            { "cmd": "logout" }
          ]
        }"#,
    )
    .unwrap();
    let out = tempfile::tempdir().unwrap();
    let recorded = record::record(&options(scenarios.path(), out.path()))
        .await
        .unwrap();
    assert_eq!(recorded.len(), 2);

    let ok = Journal::load(&out.path().join("linux/00_t/001_OK.json")).unwrap();
    assert_eq!(ok.verdict, "PASS");
    // login (refused), login, create_order, add_order_line twice, get_order, sleep_ms.
    assert_eq!(ok.steps.len(), 7);
    // An expected error is a step that met its expect: ok, with the error text kept.
    assert!(ok.steps[0].ok);
    assert_eq!(ok.steps[0].error.as_deref(), Some("pin_wrong"));
    assert!(ok.steps[1].ok);

    // The unmet expectation fails the scenario and stops it there.
    let bad = Journal::load(&out.path().join("linux/00_t/002_BAD.json")).unwrap();
    assert_eq!(bad.verdict, "FAIL");
    assert_eq!(bad.steps.len(), 1);
    assert!(!bad.steps[0].ok);
    assert!(bad.steps[0].unmet.as_deref().unwrap().contains("status"));
    let (lines, pass) = record::summary_lines(&recorded);
    assert!(!pass);
    assert_eq!(
        lines.last().unwrap(),
        "UCREC | total | FAIL | 1 failed (2 scenarios)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unknown_group_or_an_empty_selection_is_an_error() {
    let out = tempfile::tempdir().unwrap();
    let mut opts = options(&smoke_dir(), out.path());
    opts.groups = vec!["99_nothing".to_string()];
    assert!(record::record(&opts).await.is_err());
    opts.groups = vec!["00_smoke".to_string()];
    opts.only = Some("999_NOPE".to_string());
    assert!(record::record(&opts).await.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_locked_fdm_refuses_the_sale_and_the_sale_goes_through_after_the_reset() {
    let scenarios = tempfile::tempdir().unwrap();
    let group = scenarios.path().join("00_fdm");
    std::fs::create_dir_all(&group).unwrap();
    std::fs::write(
        group.join("001_V_FDM_LOCKED.json"),
        r#"{
          "format": 1, "id": "V_FDM_LOCKED", "ids": ["V_FDM_LOCKED"], "title": "locked", "compare": { "status": "shape" },
          "steps": [
            { "cmd": "login", "args": { "insz": "94060242576", "pin": "1111" } },
            { "cmd": "create_order", "args": { "input": { "order_type": "takeaway", "table_number": null, "user_insz": "94060242576" } }, "save": "order" },
            { "cmd": "add_order_line", "args": { "input": { "order_id": "$order.id", "menu_item_id": 10006, "quantity": 1, "modifiers_json": null } } },
            { "cmd": "fdm_control", "args": { "errors": ["FDM_LOCKED"] } },
            { "cmd": "checkout", "args": { "orderId": "$order.id", "payments": [{ "method": "cash" }] }, "expect": { "error": "FDM_LOCKED" } },
            { "cmd": "check_fdm_status", "args": {} },
            { "cmd": "fdm_control_reset", "args": {} },
            { "cmd": "checkout", "args": { "orderId": "$order.id", "payments": [{ "method": "cash" }] } }
          ]
        }"#,
    )
    .unwrap();
    let out = tempfile::tempdir().unwrap();
    let recorded = record::record(&options(scenarios.path(), out.path()))
        .await
        .unwrap();
    assert!(record::summary_lines(&recorded).1);
    let journal = Journal::load(&out.path().join("linux/00_fdm/001_V_FDM_LOCKED.json")).unwrap();
    assert_eq!(journal.verdict, "PASS");
    // The locked FDM refused the first sale (the call is logged with its error
    // answer); the retry after the reset was signed.
    let sales: Vec<_> = journal
        .fdm
        .iter()
        .filter(|c| c.operation == "signSale")
        .collect();
    assert_eq!(sales.len(), 2, "{:?}", operations(&journal));
    assert!(sales[0].response["data"]["signSale"]["fdmRef"].is_null());
    assert!(sales[1].response["data"]["signSale"]["fdmRef"].is_object());
    assert!(!journal.status_calls.is_empty());
}
