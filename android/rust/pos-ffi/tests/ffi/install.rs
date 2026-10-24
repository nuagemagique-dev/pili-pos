//! Tests of the installation helpers: `install_init_db` and `check_config_bundle`.

use std::path::{Path, PathBuf};

use pos_ffi::PosError;
use pos_ffi::install::{check_config_bundle, install_init_db};

use crate::support::open_seeded;

fn seed_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../src-tauri/seed_spf_test.sql")
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// Whether the SQLite file holds the seed's manager name (text is stored as is in the pages).
fn holds_seed(db: &Path) -> bool {
    let bytes = std::fs::read(db).expect("read the database");
    assert!(bytes.starts_with(b"SQLite format 3\0"), "an SQLite file");
    bytes.windows(11).any(|w| w == b"Gerant Test")
}

#[tokio::test]
async fn install_init_db_creates_the_database_and_applies_the_seed() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");

    install_init_db(s(&data), Some(s(&seed_path())))
        .await
        .expect("init");
    let db = data.join("pos.db");
    assert!(db.is_file());
    assert!(holds_seed(&db), "the seed is applied");
    assert!(data.join("engine.lock").is_file());

    // A second run without a seed migrates again and keeps the data.
    install_init_db(s(&data), None).await.expect("init again");
    assert!(holds_seed(&db));
}

#[tokio::test]
async fn install_init_db_without_a_seed_makes_an_empty_database() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("nested").join("data");
    install_init_db(s(&data), None).await.expect("init");
    assert!(data.join("pos.db").is_file());
    assert!(!holds_seed(&data.join("pos.db")));
}

#[tokio::test]
async fn install_init_db_refuses_relative_paths() {
    let dir = tempfile::tempdir().unwrap();
    let err = install_init_db("relative/data".into(), None)
        .await
        .unwrap_err();
    assert!(matches!(err, PosError::Internal { .. }), "{err:?}");
    let err = install_init_db(s(dir.path()), Some("seed.sql".into()))
        .await
        .unwrap_err();
    assert!(matches!(err, PosError::Internal { .. }), "{err:?}");
}

#[tokio::test]
async fn install_init_db_reports_a_missing_seed_as_internal() {
    let dir = tempfile::tempdir().unwrap();
    let err = install_init_db(
        s(&dir.path().join("data")),
        Some(s(&dir.path().join("none.sql"))),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, PosError::Internal { .. }), "{err:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn install_init_db_is_refused_while_an_engine_is_open_on_the_directory() {
    let t = open_seeded().await;
    let data = t.data_dir();

    let err = install_init_db(s(&data), None).await.unwrap_err();
    let PosError::Internal { detail } = err else {
        panic!("an internal error");
    };
    assert!(detail.contains("engine already open"), "{detail}");

    // Once the engine is shut down and dropped, the lock is free again.
    t.engine.shutdown().await.expect("shutdown");
    let crate::support::TestEngine { engine, dir, .. } = t;
    drop(engine);
    install_init_db(s(&dir.path().join("data")), None)
        .await
        .expect("init after the engine is gone");
}

#[tokio::test(flavor = "multi_thread")]
async fn install_init_db_accepts_the_seed_twice_while_no_fiscal_event_exists() {
    // The refusal once fiscal events exist belongs to the engine and is tested there; here the
    // facade must not add a refusal of its own.
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    install_init_db(s(&data), Some(s(&seed_path())))
        .await
        .unwrap();
    install_init_db(s(&data), Some(s(&seed_path())))
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn check_config_bundle_accepts_the_support_bundle() {
    let t = open_seeded().await;
    let problems = check_config_bundle(s(&t.bundle_json()))
        .await
        .expect("check");
    assert_eq!(problems, Vec::<String>::new());
    t.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn check_config_bundle_reports_a_missing_token_file() {
    let dir = tempfile::tempdir().unwrap();
    let pos_json = dir.path().join("pos.json");
    std::fs::write(
        &pos_json,
        r#"{
            "pos_id": "CPOS0031234567", "terminal_id": "POS-2-ANDROID", "device_id": "d",
            "restaurant_name": "R", "restaurant_address": "A", "restaurant_vat": "BE0499999960",
            "restaurant_est_no": "2123456791", "fdm_url": "http://192.168.1.20/graphql",
            "fdm_token_file": "fdm.token"
        }"#,
    )
    .unwrap();
    let problems = check_config_bundle(s(&pos_json)).await.expect("check");
    assert!(!problems.is_empty());
    assert!(
        problems.iter().any(|p| p.contains("fdm_token_file")),
        "{problems:?}"
    );
    // Each problem is a coded error: the setting's code, the English text as detail.
    assert!(
        problems
            .iter()
            .all(|p| p.starts_with("config_fdm_files|detail=fdm_token_file:")),
        "{problems:?}"
    );

    // With the file next to it, the problem is gone.
    std::fs::write(dir.path().join("fdm.token"), "token").unwrap();
    let problems = check_config_bundle(s(&pos_json)).await.expect("check");
    assert_eq!(problems, Vec::<String>::new());
}

#[tokio::test]
async fn check_config_bundle_refuses_a_relative_path_and_reports_a_missing_file() {
    let err = check_config_bundle("pos.json".into()).await.unwrap_err();
    assert!(matches!(err, PosError::Internal { .. }), "{err:?}");

    let dir = tempfile::tempdir().unwrap();
    let problems = check_config_bundle(s(&dir.path().join("pos.json")))
        .await
        .unwrap();
    assert_eq!(problems.len(), 1, "{problems:?}");
}
