//! Remote support (`RustDesk`) is started from the POS by the logged-in
//! manager, confirmed by their PIN, one session at a time. It survives
//! a logout, shows at the login screen, may be stopped by anyone and
//! ends after two hours (`commands/support.rs`). A shell script stands
//! in for `RustDesk`.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;

use common::*;

use pos_core::user::UserRole;
use pos_engine::commands::access::{ERR_FORBIDDEN, ERR_NOT_LOGGED_IN, ERR_NOT_THE_OPERATOR};
use pos_engine::commands::auth::{do_logout, do_set_pin};
use pos_engine::commands::errors;
use pos_engine::commands::support::{
    MAX_SESSION, do_remote_support_running, do_remote_support_status, do_start_remote_support,
    do_stop_remote_support,
};

const MANAGER: &str = "94060242576";
const SERVER: &str = "84022899837";
const PIN: &str = "4321";

/// A stand-in for `RustDesk`: `script` runs as the program.
fn fake_program(dir: &tempfile::TempDir, script: &str) -> PathBuf {
    let path = dir.path().join("AppRun");
    std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The manager logged in, with a PIN, and a support program installed.
async fn app_with(script: &str) -> (TestApp, tempfile::TempDir) {
    let app = TestApp::new().await;
    let dir = tempfile::tempdir().unwrap();
    app.state
        .support
        .set_program(Some(fake_program(&dir, script)));
    app.login_as(MANAGER, UserRole::Admin);
    do_set_pin(&app.state, MANAGER.into(), PIN.into(), None).unwrap();
    (app, dir)
}

#[tokio::test]
async fn a_manager_starts_and_stops_a_session() {
    let (app, _dir) = app_with("sleep 30").await;
    let idle = do_remote_support_status(&app.state).unwrap();
    assert!(idle.available);
    assert!(!idle.running);

    let running = do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).unwrap();
    assert!(running.running);
    assert_eq!(running.started_by.as_deref(), Some("Test 94060242576"));
    assert!(running.started_at.is_some());
    assert!(do_remote_support_status(&app.state).unwrap().running);

    let stopped = do_stop_remote_support(&app.state).unwrap();
    assert!(!stopped.running);
    assert!(!do_remote_support_status(&app.state).unwrap().running);
}

#[tokio::test]
async fn the_pin_confirms_the_start() {
    let (app, _dir) = app_with("sleep 30").await;
    assert_eq!(
        do_start_remote_support(&app.state, MANAGER.into(), "0000".into()).unwrap_err(),
        errors::PIN_WRONG
    );
    assert!(!do_remote_support_status(&app.state).unwrap().running);
}

#[tokio::test]
async fn a_manager_without_pin_cannot_start_it() {
    let app = TestApp::new().await;
    let dir = tempfile::tempdir().unwrap();
    app.state
        .support
        .set_program(Some(fake_program(&dir, "sleep 30")));
    app.login_as(MANAGER, UserRole::Admin);
    assert_eq!(
        do_start_remote_support(&app.state, MANAGER.into(), "1234".into()).unwrap_err(),
        errors::PIN_SETUP_REQUIRED
    );
}

#[tokio::test]
async fn only_the_logged_in_manager_starts_it() {
    let (app, _dir) = app_with("sleep 30").await;
    for role in [UserRole::Cashier, UserRole::Waiter] {
        app.login_as(SERVER, role);
        assert_eq!(
            do_start_remote_support(&app.state, SERVER.into(), PIN.into()).unwrap_err(),
            ERR_FORBIDDEN,
            "{role:?}"
        );
    }
    app.login_as(MANAGER, UserRole::Admin);
    assert_eq!(
        do_start_remote_support(&app.state, SERVER.into(), PIN.into()).unwrap_err(),
        ERR_NOT_THE_OPERATOR
    );
    app.logout();
    assert_eq!(
        do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_remote_support_status(&app.state).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
}

#[tokio::test]
async fn every_role_sees_that_a_session_runs() {
    let (app, _dir) = app_with("sleep 30").await;
    do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).unwrap();
    for role in [UserRole::Cashier, UserRole::Waiter] {
        app.login_as(SERVER, role);
        assert!(
            do_remote_support_status(&app.state).unwrap().running,
            "{role:?}"
        );
    }
    app.login_as(MANAGER, UserRole::Admin);
    do_stop_remote_support(&app.state).unwrap();
}

#[tokio::test]
async fn one_session_at_a_time() {
    let (app, _dir) = app_with("sleep 30").await;
    do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).unwrap();
    assert_eq!(
        do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).unwrap_err(),
        errors::SUPPORT_RUNNING
    );
    do_stop_remote_support(&app.state).unwrap();
    // Once stopped, a new one may start.
    assert!(
        do_start_remote_support(&app.state, MANAGER.into(), PIN.into())
            .unwrap()
            .running
    );
    do_stop_remote_support(&app.state).unwrap();
}

#[tokio::test]
async fn a_session_ended_by_the_program_is_no_longer_running() {
    let (app, _dir) = app_with("exit 0").await;
    do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).unwrap();
    let mut running = true;
    for _ in 0..200 {
        running = do_remote_support_status(&app.state).unwrap().running;
        if !running {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(!running, "the banner goes away when RustDesk exits");
    // And a new session can start.
    assert!(do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).is_ok());
}

#[tokio::test]
async fn the_session_gets_a_private_home() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("home.txt");
    // Written aside then renamed: the shell creates the file before
    // echo fills it, and a busy machine would let us read it empty.
    let (app, _program) = app_with(&format!(
        "echo \"$HOME\" > {m}.tmp && mv {m}.tmp {m}",
        m = marker.display()
    ))
    .await;
    do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).unwrap();
    for _ in 0..200 {
        if marker.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let home = std::fs::read_to_string(&marker).unwrap();
    assert!(home.trim().ends_with("pili-pos-support"), "{home}");
}

#[tokio::test]
async fn without_the_program_the_feature_says_so() {
    let app = TestApp::new().await;
    app.login_as(MANAGER, UserRole::Admin);
    do_set_pin(&app.state, MANAGER.into(), PIN.into(), None).unwrap();
    let status = do_remote_support_status(&app.state).unwrap();
    assert!(!status.available);
    assert_eq!(
        do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).unwrap_err(),
        errors::SUPPORT_UNAVAILABLE
    );
}

#[tokio::test]
async fn a_program_that_cannot_run_is_reported() {
    let app = TestApp::new().await;
    app.state
        .support
        .set_program(Some(PathBuf::from("/nonexistent/AppRun")));
    app.login_as(MANAGER, UserRole::Admin);
    do_set_pin(&app.state, MANAGER.into(), PIN.into(), None).unwrap();
    let err = do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).unwrap_err();
    assert!(err.starts_with("support_start_failed|detail="), "{err}");
    assert!(!do_remote_support_status(&app.state).unwrap().running);
}

#[tokio::test]
async fn stopping_when_nothing_runs_is_harmless() {
    let (app, _dir) = app_with("sleep 30").await;
    assert!(!do_stop_remote_support(&app.state).unwrap().running);
}

#[tokio::test]
async fn a_session_outlives_a_logout_and_shows_at_the_login_screen() {
    let (app, _dir) = app_with("sleep 30").await;
    do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).unwrap();
    do_logout(&app.state).unwrap();

    let seen = do_remote_support_running(&app.state).unwrap();
    assert!(seen.running);
    assert!(seen.started_at.is_some());
}

#[tokio::test]
async fn anyone_stops_a_session_even_logged_out() {
    let (app, _dir) = app_with("sleep 30").await;
    do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).unwrap();
    app.login_as(SERVER, UserRole::Waiter);
    assert!(!do_stop_remote_support(&app.state).unwrap().running);

    app.login_as(MANAGER, UserRole::Admin);
    do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).unwrap();
    app.logout();
    assert!(!do_stop_remote_support(&app.state).unwrap().running);
    assert!(!do_remote_support_running(&app.state).unwrap().running);
}

#[tokio::test]
async fn a_session_ends_after_the_time_limit() {
    let (app, _dir) = app_with("sleep 30").await;
    do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).unwrap();
    app.state
        .support
        .age_session(MAX_SESSION - chrono::TimeDelta::minutes(1));
    assert!(do_remote_support_running(&app.state).unwrap().running);

    app.state.support.age_session(chrono::TimeDelta::minutes(1));
    assert!(!do_remote_support_running(&app.state).unwrap().running);
    assert!(!do_remote_support_status(&app.state).unwrap().running);
}

#[tokio::test]
async fn the_reaper_ends_an_expired_session_without_anyone_asking() {
    let (app, _dir) = app_with("sleep 30").await;
    do_start_remote_support(&app.state, MANAGER.into(), PIN.into()).unwrap();
    pos_engine::commands::support::reap_expired(&app.state);
    assert!(app.state.support.holds_session(), "not expired yet");

    app.state.support.age_session(MAX_SESSION);
    assert!(app.state.support.holds_session(), "nobody asked");
    pos_engine::commands::support::reap_expired(&app.state);
    assert!(!app.state.support.holds_session());
}
