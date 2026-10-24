//! V12-D5: the accountant export is outside the certified binary.
//!
//! No shipped crate may depend on `pos-accounting`, and the tool itself depends on none
//! of the POS crates at run time (it reads the signed archive as plain SQLite).

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn manifest(relative: &str) -> String {
    std::fs::read_to_string(root().join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
}

#[test]
fn no_shipped_crate_names_pos_accounting() {
    for relative in [
        "crates/pos-engine/Cargo.toml",
        "src-tauri/Cargo.toml",
        "android/rust/pos-ffi/Cargo.toml",
        "android/rust/Cargo.toml",
        "crates/pos-core/Cargo.toml",
        "crates/pos-db/Cargo.toml",
        "crates/pos-fiscal/Cargo.toml",
        "crates/pos-print/Cargo.toml",
        "crates/pos-export/Cargo.toml",
    ] {
        assert!(
            !manifest(relative).contains("pos-accounting"),
            "{relative} names pos-accounting"
        );
    }
}

#[test]
fn the_android_workspace_and_its_lock_do_not_know_the_tool() {
    for relative in ["android/rust/Cargo.lock", "android/uccheck/Cargo.lock"] {
        if root().join(relative).exists() {
            assert!(
                !manifest(relative).contains("pos-accounting"),
                "{relative} names pos-accounting"
            );
        }
    }
}

#[test]
fn the_tool_has_no_pos_crate_as_a_run_time_dependency() {
    let text = manifest("crates/pos-accounting/Cargo.toml");
    let runtime = text
        .split("[dev-dependencies]")
        .next()
        .unwrap()
        .split("[dependencies]")
        .nth(1)
        .expect("a [dependencies] section");
    assert!(
        !runtime.contains("pos-"),
        "run-time dependencies: {runtime}"
    );
}

#[test]
fn the_tool_is_not_published() {
    assert!(manifest("crates/pos-accounting/Cargo.toml").contains("publish = false"));
}
