//! The three drivers against `pos-eft-mock`, served in-process: the
//! payment, its refusal, a busy terminal, an abort, a lost answer, a bad
//! key, an amount that is not the one asked, the same payment twice.

use std::net::TcpListener;
use std::time::{Duration, Instant};

use pos_core::config::{PaymentTerminalConfig, PaymentTerminalKind};
use pos_eft_mock::{Behaviour, Mock, Next};
use tempfile::TempDir;

use crate::ccv_cloud::CcvPaymentApi;
use crate::ccv_opi::CcvOpi;
use crate::worldline::WorldlineTerminalApi;
use crate::{CardType, Driver, EftError, Outcome, PaymentRequest, Terminal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Worldline,
    Ccv,
    Opi,
}

const ALL: [Kind; 3] = [Kind::Worldline, Kind::Ccv, Kind::Opi];

struct Rig {
    kind: Kind,
    mock: Mock,
    dir: TempDir,
    config: PaymentTerminalConfig,
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn next(result: Behaviour, delay_ms: u64) -> Next {
    Next {
        result,
        delay_ms,
        ..Next::default()
    }
}

impl Rig {
    fn new(kind: Kind) -> Self {
        let mock = Mock::start();
        let device = free_port();
        mock.set_device_port(device);
        mock.set_default(next(Behaviour::Approve, 30));
        let config = PaymentTerminalConfig {
            kind: match kind {
                Kind::Worldline => PaymentTerminalKind::WorldlineTerminalApi,
                Kind::Ccv => PaymentTerminalKind::CcvPaymentApi,
                Kind::Opi => PaymentTerminalKind::CcvOpi,
            },
            base_url: (kind != Kind::Opi).then(|| mock.url()),
            api_key_file: None,
            merchant_id: Some("WL000D5T0000004".into()),
            terminal_id: Some("26197214".into()),
            integrator_id: Some("9b2e6f1a1c3d4e5f8a7b0c1d2e3f4a5b".into()),
            management_system_id: Some("000103030".into()),
            access_protocol: None,
            address: (kind == Kind::Opi).then(|| mock.opi_addr().to_string()),
            device_port: Some(device),
            timeout_seconds: 60,
        };
        Self {
            kind,
            mock,
            dir: tempfile::tempdir().unwrap(),
            config,
        }
    }

    fn set_key(&mut self, key: &str) {
        let path = self.dir.path().join("api.key");
        std::fs::write(&path, format!("{key}\n")).unwrap();
        self.config.api_key_file = Some(path.display().to_string());
    }

    /// The terminal with a short timeout and a fast poll.
    fn terminal(&self, timeout: Duration) -> Terminal {
        let http = reqwest::Client::new();
        let driver = match self.kind {
            Kind::Worldline => {
                let mut d =
                    WorldlineTerminalApi::new(&self.config, http, "pili-pos/1.0.0").unwrap();
                d.set_timeout(timeout);
                Driver::Worldline(d)
            }
            Kind::Ccv => {
                let mut d = CcvPaymentApi::new(&self.config, http).unwrap();
                d.set_timing(timeout, Duration::from_millis(40));
                Driver::CcvCloud(d)
            }
            Kind::Opi => {
                let mut d = CcvOpi::new(&self.config, "pili-pos/1.0.0").unwrap();
                d.set_timeout(timeout);
                Driver::CcvOpi(d)
            }
        };
        Terminal { driver }
    }

    fn t(&self) -> Terminal {
        self.terminal(Duration::from_secs(20))
    }
}

fn request(cents: i64) -> PaymentRequest {
    PaymentRequest {
        reference: uuid::Uuid::new_v4().to_string(),
        amount_cents: cents,
        description: "Tokyo Sushi Bar, table 4".into(),
        language: "fr".into(),
    }
}

#[tokio::test]
async fn an_approved_payment_carries_brand_type_reference_and_receipt() {
    for kind in ALL {
        let rig = Rig::new(kind);
        let req = request(1250);
        let out = rig.t().pay(&req).await.unwrap();
        let Outcome::Approved(a) = out else {
            panic!("{kind:?}: {out:?}")
        };
        assert_eq!(a.amount_cents, 1250, "{kind:?}");
        assert_eq!(a.brand.as_deref(), Some("Bancontact"), "{kind:?}");
        assert_eq!(a.card_type, CardType::Debit, "{kind:?}");
        assert!(a.terminal_reference.is_some(), "{kind:?}");
        assert_eq!(
            a.masked_pan.as_deref(),
            Some("XXXXXXXXXXXX0119"),
            "{kind:?}"
        );
        assert!(a.authorisation_code.is_some(), "{kind:?}");
        let receipt = a.customer_receipt.unwrap_or_default();
        assert!(
            receipt.contains("Cardholder Receipt"),
            "{kind:?}: {receipt}"
        );
        assert!(
            receipt.contains("Total-EFT EUR:             12.50"),
            "{kind:?}"
        );
        let payments = rig.mock.payments();
        assert_eq!(payments.len(), 1, "{kind:?}");
        assert_eq!(payments[0].outcome, "approved");
        if kind != Kind::Opi {
            assert_eq!(payments[0].reference, req.reference, "{kind:?}");
        }
    }
}

#[tokio::test]
async fn a_credit_brand_is_credit() {
    for kind in ALL {
        let rig = Rig::new(kind);
        rig.mock.next(Next {
            brand: "Visa".into(),
            delay_ms: 10,
            ..Next::default()
        });
        let Outcome::Approved(a) = rig.t().pay(&request(500)).await.unwrap() else {
            panic!("{kind:?}")
        };
        assert_eq!(a.brand.as_deref(), Some("Visa"), "{kind:?}");
        assert_eq!(a.card_type, CardType::Credit, "{kind:?}");
    }
}

#[tokio::test]
async fn a_refused_payment_is_declined() {
    for kind in ALL {
        let rig = Rig::new(kind);
        rig.mock.next(next(Behaviour::Decline, 10));
        let out = rig.t().pay(&request(900)).await.unwrap();
        assert!(matches!(out, Outcome::Declined { .. }), "{kind:?}: {out:?}");
    }
}

#[tokio::test]
async fn a_busy_terminal_is_an_error_before_any_charge() {
    for kind in ALL {
        let rig = Rig::new(kind);
        rig.mock.next(next(Behaviour::Busy, 10));
        let out = rig.t().pay(&request(900)).await;
        assert!(matches!(out, Err(EftError::Busy)), "{kind:?}: {out:?}");
        assert!(rig.mock.payments().is_empty(), "{kind:?}");
    }
}

#[tokio::test]
async fn a_second_payment_while_one_runs_finds_the_terminal_busy() {
    for kind in ALL {
        let rig = Rig::new(kind);
        rig.mock.next(next(Behaviour::Approve, 800));
        let first = request(700);
        let terminal = rig.t();
        let (first_out, second_out) = tokio::join!(terminal.pay(&first), async {
            tokio::time::sleep(Duration::from_millis(250)).await;
            rig.t().pay(&request(300)).await
        });
        assert!(matches!(first_out, Ok(Outcome::Approved(_))), "{kind:?}");
        assert!(
            matches!(second_out, Err(EftError::Busy)),
            "{kind:?}: {second_out:?}"
        );
        assert_eq!(rig.mock.payments().len(), 1, "{kind:?}");
    }
}

#[tokio::test]
async fn an_abort_during_the_payment_cancels_it() {
    for kind in ALL {
        let rig = Rig::new(kind);
        rig.mock.next(next(Behaviour::Approve, 5000));
        let req = request(1250);
        let terminal = rig.t();
        let started = Instant::now();
        let (out, aborted) = tokio::join!(terminal.pay(&req), async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let first = terminal.abort(&req.reference).await;
            // Idempotent: asking again is still fine.
            let second = terminal.abort(&req.reference).await;
            (first, second)
        });
        assert_eq!(out.unwrap(), Outcome::Cancelled, "{kind:?}");
        assert!(
            aborted.0.is_ok() && aborted.1.is_ok(),
            "{kind:?}: {aborted:?}"
        );
        assert!(started.elapsed() < Duration::from_secs(4), "{kind:?}");
        assert_eq!(rig.mock.payments()[0].outcome, "cancelled", "{kind:?}");
        // Nothing charged: the status says so too.
        let status = rig.t().status(&req.reference).await.unwrap();
        assert!(
            matches!(
                status,
                Outcome::Cancelled | Outcome::Unknown { .. } | Outcome::Declined { .. }
            ),
            "{kind:?}: {status:?}"
        );
    }
}

#[tokio::test]
async fn a_lost_answer_is_unknown_and_status_finds_the_approval() {
    for kind in ALL {
        let rig = Rig::new(kind);
        rig.mock.next(next(Behaviour::Timeout, 50));
        let req = request(1250);
        let terminal = rig.t();
        let out = terminal.pay(&req).await.unwrap();
        assert!(matches!(out, Outcome::Unknown { .. }), "{kind:?}: {out:?}");
        tokio::time::sleep(Duration::from_millis(100)).await;
        let Outcome::Approved(a) = terminal.status(&req.reference).await.unwrap() else {
            panic!("{kind:?}: status did not find the approval")
        };
        assert_eq!(a.amount_cents, 1250, "{kind:?}");
        assert_eq!(a.brand.as_deref(), Some("Bancontact"), "{kind:?}");
        assert!(a.terminal_reference.is_some(), "{kind:?}");
    }
}

#[tokio::test]
async fn a_payment_still_running_at_the_timeout_is_unknown_then_approved() {
    for kind in ALL {
        let rig = Rig::new(kind);
        rig.mock.next(next(Behaviour::Approve, 1200));
        let req = request(800);
        let terminal = rig.terminal(Duration::from_millis(400));
        let started = Instant::now();
        let out = terminal.pay(&req).await.unwrap();
        assert!(matches!(out, Outcome::Unknown { .. }), "{kind:?}: {out:?}");
        assert!(started.elapsed() < Duration::from_millis(1100), "{kind:?}");
        // Still in progress right after the timeout, approved a bit later.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        let status = terminal.status(&req.reference).await.unwrap();
        assert!(
            matches!(status, Outcome::Approved(_)),
            "{kind:?}: {status:?}"
        );
    }
}

#[tokio::test]
async fn a_terminal_that_never_answers_stays_unknown() {
    for kind in ALL {
        let rig = Rig::new(kind);
        rig.mock.next(next(Behaviour::Unknown, 20));
        let req = request(800);
        let terminal = rig.terminal(Duration::from_millis(600));
        let out = terminal.pay(&req).await.unwrap();
        assert!(matches!(out, Outcome::Unknown { .. }), "{kind:?}: {out:?}");
        let status = terminal.status(&req.reference).await.unwrap();
        assert!(
            matches!(status, Outcome::Unknown { .. }),
            "{kind:?}: {status:?}"
        );
    }
}

#[tokio::test]
async fn a_payment_the_terminal_never_saw_has_no_outcome() {
    for kind in ALL {
        let rig = Rig::new(kind);
        let status = rig
            .t()
            .status(&uuid::Uuid::new_v4().to_string())
            .await
            .unwrap();
        assert!(
            matches!(status, Outcome::Unknown { .. }),
            "{kind:?}: {status:?}"
        );
    }
}

#[tokio::test]
async fn a_bad_api_key_is_a_rejected_request() {
    for kind in [Kind::Worldline, Kind::Ccv] {
        let mut rig = Rig::new(kind);
        rig.mock.set_api_key(Some("the-real-key"));
        rig.set_key("a-wrong-key");
        let out = rig.t().pay(&request(500)).await;
        assert!(
            matches!(out, Err(EftError::Rejected(_))),
            "{kind:?}: {out:?}"
        );
        assert!(rig.mock.payments().is_empty(), "{kind:?}");
        let status = rig.t().status("whatever").await;
        assert!(
            matches!(status, Err(EftError::Rejected(_))),
            "{kind:?}: {status:?}"
        );
        rig.set_key("the-real-key");
        let out = rig.t().pay(&request(500)).await.unwrap();
        assert!(matches!(out, Outcome::Approved(_)), "{kind:?}: {out:?}");
    }
}

#[tokio::test]
async fn an_amount_other_than_the_one_asked_is_unknown() {
    for kind in ALL {
        let rig = Rig::new(kind);
        rig.mock.next(Next {
            approved_cents: Some(1000),
            delay_ms: 10,
            ..Next::default()
        });
        let out = rig.t().pay(&request(1250)).await.unwrap();
        assert!(matches!(out, Outcome::Unknown { .. }), "{kind:?}: {out:?}");
    }
}

#[tokio::test]
async fn the_same_payment_twice_is_one_charge() {
    for kind in ALL {
        let rig = Rig::new(kind);
        let req = request(1250);
        let terminal = rig.t();
        let first = terminal.pay(&req).await.unwrap();
        let second = terminal.pay(&req).await.unwrap();
        assert!(matches!(first, Outcome::Approved(_)), "{kind:?}: {first:?}");
        assert!(
            matches!(second, Outcome::Approved(_)),
            "{kind:?}: {second:?}"
        );
        assert_eq!(rig.mock.payments().len(), 1, "{kind:?}");
        if let (Outcome::Approved(a), Outcome::Approved(b)) = (first, second) {
            assert_eq!(a.terminal_reference, b.terminal_reference, "{kind:?}");
        }
    }
}

#[tokio::test]
async fn an_unreachable_terminal_is_an_error_before_any_charge() {
    for kind in ALL {
        let mut rig = Rig::new(kind);
        let closed = {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap()
        };
        if kind == Kind::Opi {
            rig.config.address = Some(closed.to_string());
        } else {
            rig.config.base_url = Some(format!("http://{closed}"));
        }
        let out = rig.t().pay(&request(500)).await;
        assert!(
            matches!(out, Err(EftError::Unreachable(_))),
            "{kind:?}: {out:?}"
        );
        let status = rig.t().status("x").await;
        assert!(
            matches!(status, Err(EftError::Unreachable(_))),
            "{kind:?}: {status:?}"
        );
    }
}

#[tokio::test]
async fn the_request_carries_the_reference_amount_and_language() {
    let rig = Rig::new(Kind::Worldline);
    let mut req = request(10_411);
    req.language = "nl".into();
    rig.t().pay(&req).await.unwrap();
    let logged = &rig.mock.requests()[0];
    let r = &logged.body["SaleToPOIServiceRequest"];
    assert_eq!(r["Header"]["ExchangeIdentification"], req.reference);
    assert_eq!(
        r["Header"]["SalesSystemInfo"]["IntegratorId"],
        "9b2e6f1a1c3d4e5f8a7b0c1d2e3f4a5b"
    );
    assert_eq!(
        r["Header"]["SalesSystemInfo"]["SaleSoftware"][0]["Status"]["VersionNumber"],
        "1.0.0"
    );
    assert_eq!(
        r["ServiceRequest"]["Environment"]["Cardholder"]["Language"],
        "NL"
    );
    let details =
        &r["ServiceRequest"]["PaymentRequest"]["PaymentTransaction"]["TransactionDetails"];
    assert_eq!(details["TotalAmount"].to_string(), "104.11");

    let rig = Rig::new(Kind::Ccv);
    let mut req = request(1250);
    req.language = "de".into();
    rig.t().pay(&req).await.unwrap();
    let logged = &rig.mock.requests()[0];
    assert_eq!(logged.body["language"], "deu");
    assert_eq!(logged.body["merchantOrderReference"], req.reference);
    assert_eq!(logged.body["details"]["accessProtocol"], "OPI_NL");
    assert_eq!(logged.body["amount"].to_string(), "12.5");

    let rig = Rig::new(Kind::Opi);
    rig.t().pay(&request(1250)).await.unwrap();
    let body = rig.mock.requests()[0].body.as_str().unwrap().to_string();
    assert!(body.contains("RequestType=\"CardPayment\""), "{body}");
    assert!(
        body.contains("<TotalAmount Currency=\"EUR\">12.50</TotalAmount>"),
        "{body}"
    );
    assert!(body.contains("WorkstationID=\"26197214\""), "{body}");
}

#[test]
fn a_terminal_needs_its_configuration() {
    let mut config = Rig::new(Kind::Worldline).config;
    config.integrator_id = None;
    let err = Terminal::new(&config, reqwest::Client::new(), "pili-pos/1.0.0");
    assert!(matches!(err, Err(EftError::Config(_))));
}

#[tokio::test]
async fn a_reachable_terminal_checks_ok_and_charges_nothing() {
    for kind in ALL {
        let rig = Rig::new(kind);
        rig.t()
            .check()
            .await
            .unwrap_or_else(|e| panic!("{kind:?}: {e}"));
        assert!(rig.mock.payments().is_empty(), "{kind:?}");
    }
}

#[tokio::test]
async fn a_closed_port_checks_unreachable() {
    for kind in ALL {
        let mut rig = Rig::new(kind);
        let closed = format!("127.0.0.1:{}", free_port());
        match kind {
            Kind::Opi => rig.config.address = Some(closed),
            _ => rig.config.base_url = Some(format!("http://{closed}")),
        }
        let out = rig.t().check().await;
        assert!(
            matches!(out, Err(EftError::Unreachable(_))),
            "{kind:?}: {out:?}"
        );
    }
}

#[tokio::test]
async fn a_bad_api_key_fails_the_check_as_rejected() {
    for kind in [Kind::Worldline, Kind::Ccv] {
        let mut rig = Rig::new(kind);
        rig.mock.set_api_key(Some("the-real-key"));
        rig.set_key("a-wrong-key");
        let out = rig.t().check().await;
        assert!(
            matches!(out, Err(EftError::Rejected(_))),
            "{kind:?}: {out:?}"
        );
        rig.set_key("the-real-key");
        rig.t().check().await.unwrap();
        assert!(rig.mock.payments().is_empty(), "{kind:?}");
    }
}

#[tokio::test]
async fn a_worldline_terminal_not_connected_fails_the_check() {
    let rig = Rig::new(Kind::Worldline);
    rig.mock.set_terminals_online(false);
    let out = rig.t().check().await;
    let Err(EftError::Unreachable(why)) = out else {
        panic!("{out:?}")
    };
    assert!(why.contains("not connected"), "{why}");
    rig.mock.set_terminals_online(true);
    rig.t().check().await.unwrap();
    assert!(rig.mock.payments().is_empty());
}

#[tokio::test]
async fn the_worldline_check_falls_back_to_v1_when_v2_has_no_terminal_list() {
    let rig = Rig::new(Kind::Worldline);
    rig.mock.set_merchant_v1_only(true);
    rig.t().check().await.unwrap();
    let paths: Vec<String> = rig.mock.requests().iter().map(|r| r.path.clone()).collect();
    assert!(
        paths.iter().any(|p| p.starts_with("/api/v2/merchants/")),
        "{paths:?}"
    );
    assert!(
        paths.iter().any(|p| p.starts_with("/api/v1/merchants/")),
        "{paths:?}"
    );
    assert!(rig.mock.payments().is_empty());
}

#[tokio::test]
async fn an_unlisted_worldline_terminal_id_fails_the_check() {
    let mut rig = Rig::new(Kind::Worldline);
    rig.config.terminal_id = Some("99999999".into());
    let out = rig.t().check().await;
    assert!(matches!(out, Err(EftError::Unreachable(_))), "{out:?}");
}

/// A tracing writer into a shared buffer.
#[derive(Clone, Default)]
struct Sink(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Sink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn the_wire_trace_shows_bodies_but_never_keys_or_queries() {
    for kind in ALL {
        let mut rig = Rig::new(kind);
        if kind != Kind::Opi {
            rig.mock.set_api_key(Some("secret-key-123"));
            rig.set_key("secret-key-123");
        }
        let sink = Sink::default();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .with_writer({
                let sink = sink.clone();
                move || sink.clone()
            })
            .finish();
        let guard = tracing::subscriber::set_default(subscriber);
        let req = request(700);
        rig.t().pay(&req).await.unwrap();
        rig.t().status(&req.reference).await.unwrap();
        drop(guard);
        let trace = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
        assert!(trace.contains("pos_eft::wire"), "{kind:?}: {trace}");
        assert!(trace.contains("recv"), "{kind:?}");
        assert!(!trace.contains("secret-key-123"), "{kind:?}");
        assert!(!trace.contains("Bearer"), "{kind:?}");
        assert!(!trace.contains("merchantOrderReference="), "{kind:?}");
        let marker = match kind {
            Kind::Worldline => "payments/sync",
            Kind::Ccv => "/api/v1/payment",
            Kind::Opi => "CardServiceRequest",
        };
        assert!(trace.contains(marker), "{kind:?}: {trace}");
    }
}
