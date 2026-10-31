//! The probe's commands against `pos-eft-mock` (OPI), with a scripted
//! stdin: nothing leaves 127.0.0.1.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use std::io::Cursor;
use std::net::TcpListener;
use std::path::Path;

use pos_core::config::{PaymentTerminalConfig, PaymentTerminalKind};
use pos_eft::{Terminal, mask_pan};
use pos_eft_mock::{Behaviour, Mock, Next};
use pos_eft_probe::{
    Command, EXIT_FAILED, EXIT_NOT_CONFIRMED, EXIT_OK, describe, run, trace_subscriber,
};

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn rig() -> (Mock, PaymentTerminalConfig) {
    let mock = Mock::start();
    let device = free_port();
    mock.set_device_port(device);
    mock.set_default(Next {
        result: Behaviour::Approve,
        delay_ms: 20,
        ..Next::default()
    });
    let config = PaymentTerminalConfig {
        kind: PaymentTerminalKind::CcvOpi,
        base_url: None,
        api_key_file: None,
        merchant_id: None,
        terminal_id: None,
        integrator_id: None,
        management_system_id: None,
        access_protocol: None,
        address: Some(mock.opi_addr().to_string()),
        device_port: Some(device),
        timeout_seconds: 60,
    };
    (mock, config)
}

/// Run `command` with `stdin` as the operator's answer, tracing to
/// `trace`. Returns the exit code and what was printed.
async fn drive(
    config: &PaymentTerminalConfig,
    command: Command,
    stdin: &str,
    trace: &Path,
) -> (u8, String) {
    let _guard = tracing::subscriber::set_default(trace_subscriber(trace).unwrap());
    let terminal = Terminal::new(config, reqwest::Client::new(), "pos-eft-probe/test").unwrap();
    let mut out = Vec::new();
    let code = run(
        &command,
        &terminal,
        &describe(config),
        &mut Cursor::new(stdin.as_bytes().to_vec()),
        &mut out,
    )
    .await
    .unwrap();
    (code, String::from_utf8(out).unwrap())
}

#[tokio::test(flavor = "current_thread")]
async fn check_reports_a_reachable_terminal_and_sends_no_payment() {
    let (mock, config) = rig();
    let dir = tempfile::tempdir().unwrap();
    let (code, out) = drive(&config, Command::Check, "", &dir.path().join("t.log")).await;
    assert_eq!(code, EXIT_OK, "{out}");
    assert!(out.contains("Reachable"), "{out}");
    assert!(mock.payments().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn check_reports_a_closed_port_as_failed() {
    let (_mock, mut config) = rig();
    config.address = Some(format!("127.0.0.1:{}", free_port()));
    let dir = tempfile::tempdir().unwrap();
    let (code, out) = drive(&config, Command::Check, "", &dir.path().join("t.log")).await;
    assert_eq!(code, EXIT_FAILED, "{out}");
    assert!(out.contains("Not reachable"), "{out}");
}

#[tokio::test(flavor = "current_thread")]
async fn pay_without_yes_sends_nothing() {
    let (mock, config) = rig();
    let dir = tempfile::tempdir().unwrap();
    for answer in ["", "no\n", "yes\n", "y\n"] {
        let (code, out) = drive(
            &config,
            Command::Pay { amount_cents: 1 },
            answer,
            &dir.path().join("t.log"),
        )
        .await;
        assert_eq!(code, EXIT_NOT_CONFIRMED, "{answer:?}: {out}");
        assert!(out.contains("Reference: "), "{out}");
        assert!(out.contains("Nothing was sent"), "{out}");
    }
    assert!(mock.payments().is_empty());
    assert!(mock.requests().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn pay_with_yes_is_approved_and_the_trace_is_masked() {
    let (mock, config) = rig();
    let dir = tempfile::tempdir().unwrap();
    let trace = dir.path().join("trace.log");
    let (code, out) = drive(&config, Command::Pay { amount_cents: 1 }, "YES\n", &trace).await;
    assert_eq!(code, EXIT_OK, "{out}");
    assert!(out.contains("\"outcome\": \"approved\""), "{out}");
    let payments = mock.payments();
    assert_eq!(payments.len(), 1);
    assert_eq!(payments[0].amount_cents, 1);
    assert_eq!(payments[0].outcome, "approved");

    let log = std::fs::read_to_string(&trace).unwrap();
    assert!(log.contains("pos_eft::wire"), "{log}");
    assert!(log.contains("CardServiceRequest"), "{log}");
    assert!(log.contains("CardServiceResponse"), "{log}");
    assert!(log.contains("pay confirmed by the operator"), "{log}");
    // No run of 13 to 19 digits survives in the file.
    assert_eq!(mask_pan(&log), log);
    assert!(!out.is_empty() && mask_pan(&out) == out);
}

#[tokio::test(flavor = "current_thread")]
async fn a_declined_payment_is_a_failure_with_the_reason() {
    let (mock, config) = rig();
    mock.next(Next {
        result: Behaviour::Decline,
        delay_ms: 10,
        ..Next::default()
    });
    let dir = tempfile::tempdir().unwrap();
    let (code, out) = drive(
        &config,
        Command::Pay { amount_cents: 1 },
        "YES\n",
        &dir.path().join("t.log"),
    )
    .await;
    assert_eq!(code, EXIT_FAILED, "{out}");
    assert!(out.contains("declined"), "{out}");
}
