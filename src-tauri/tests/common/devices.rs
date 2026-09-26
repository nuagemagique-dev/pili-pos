//! Fake peripherals for the delivery tests: a thermal printer that
//! records every job, and the Scaleway TEM email API (wiremock).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use pos_core::config::DigitalTicketConfig;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

use moyun_pos::commands::delivery::MailState;
use pos_mail::tem::TemClient;

use super::harness::TestApp;

/// A TCP "printer" on 127.0.0.1. It answers the DLE EOT status queries
/// with "all fine" and keeps the bytes of every connection.
pub struct PrinterSink {
    pub addr: String,
    jobs: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl PrinterSink {
    pub async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind printer sink");
        let addr = listener.local_addr().unwrap().to_string();
        let jobs = Arc::new(Mutex::new(Vec::new()));
        let recorded = jobs.clone();
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let recorded = recorded.clone();
                tokio::spawn(async move {
                    let mut data = Vec::new();
                    let mut chunk = [0u8; 4096];
                    let mut answered = 0;
                    loop {
                        match tokio::time::timeout(Duration::from_secs(2), stream.read(&mut chunk))
                            .await
                        {
                            Ok(Ok(0)) | Ok(Err(_)) | Err(_) => break,
                            Ok(Ok(n)) => {
                                data.extend_from_slice(&chunk[..n]);
                                let asked = data.windows(2).filter(|w| w == &[0x10, 0x04]).count();
                                while answered < asked {
                                    let _ = stream.write_all(&[0x12]).await;
                                    answered += 1;
                                }
                            }
                        }
                    }
                    recorded.lock().unwrap().push(data);
                });
            }
        });
        Self { addr, jobs }
    }

    /// Printed documents (the drawer kick and status queries alone are
    /// a handful of bytes).
    fn documents(&self) -> Vec<String> {
        self.jobs
            .lock()
            .unwrap()
            .iter()
            .filter(|j| j.len() > 64)
            .map(|j| String::from_utf8_lossy(j).into_owned())
            .collect()
    }

    /// Jobs that carried a receipt: every document but the bar /
    /// kitchen slips.
    pub fn receipts(&self) -> Vec<String> {
        self.documents()
            .into_iter()
            .filter(|d| !is_bar_slip(d))
            .collect()
    }

    /// Bar / kitchen order slips, in print order.
    pub fn bar_slips(&self) -> Vec<String> {
        self.documents()
            .into_iter()
            .filter(|d| is_bar_slip(d))
            .collect()
    }

    /// Wait until `n` bar slips arrived, then a little longer to catch
    /// extra ones.
    pub async fn settle_slips(&self, n: usize) -> Vec<String> {
        for _ in 0..40 {
            if self.bar_slips().len() >= n {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
        self.bar_slips()
    }

    /// Jobs that opened the cash drawer (ESC p 0 25 250).
    pub fn drawer_kicks(&self) -> usize {
        const KICK: [u8; 5] = [0x1B, 0x70, 0x00, 0x19, 0xFA];
        self.jobs
            .lock()
            .unwrap()
            .iter()
            .filter(|j| j.windows(5).any(|w| w == KICK))
            .count()
    }

    /// Wait until `n` receipts arrived (jobs are recorded when the
    /// connection closes), then a little longer to catch extra ones.
    pub async fn settle(&self, n: usize) -> Vec<String> {
        for _ in 0..40 {
            if self.receipts().len() >= n {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
        self.receipts()
    }
}

/// The title of the bar / kitchen slip, in FR, NL or DE.
fn is_bar_slip(document: &str) -> bool {
    ["BON DE COMMANDE", "BESTELBON", "BESTELLSCHEIN"]
        .iter()
        .any(|title| document.contains(title))
}

/// An address nothing listens on: every print fails at connect.
pub const DEAD_PRINTER: &str = "127.0.0.1:1";

pub const TEM_MESSAGE_ID: &str = "tem-message-1";

/// TEM accepts every email.
pub async fn tem_accepts(tem: &MockServer) {
    Mock::given(method("POST"))
        .and(path_regex(r"/emails$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "emails": [{ "id": "e-1", "message_id": TEM_MESSAGE_ID }]
        })))
        .mount(tem)
        .await;
}

/// TEM answers `status` to the next `times` sends.
pub async fn tem_fails(tem: &MockServer, status: u16, times: u64) {
    Mock::given(method("POST"))
        .and(path_regex(r"/emails$"))
        .respond_with(ResponseTemplate::new(status).set_body_string("unavailable"))
        .up_to_n_times(times)
        .mount(tem)
        .await;
}

/// The reachability probe (`GET …/emails`) answers `status`.
pub async fn tem_probe(tem: &MockServer, status: u16) {
    Mock::given(method("GET"))
        .and(path_regex(r"/emails$"))
        .respond_with(ResponseTemplate::new(status).set_body_json(json!({ "emails": [] })))
        .mount(tem)
        .await;
}

/// Emails TEM received, as JSON bodies.
pub async fn tem_sent(tem: &MockServer) -> Vec<serde_json::Value> {
    tem.received_requests()
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r.method.as_str() == "POST")
        .map(|r| serde_json::from_slice(&r.body).expect("TEM body is JSON"))
        .collect()
}

impl TestApp {
    /// Turn on the digital ticket, sending through `tem`.
    pub fn enable_mail(&mut self, tem: &MockServer, allow_digital_only: bool) {
        let client = TemClient::new(
            reqwest::Client::new(),
            tem.uri(),
            "fr-par",
            "project-1",
            "secret-key",
            "tickets@example.be",
            "The FPS Finances Experience",
            Duration::from_secs(5),
        );
        self.state.mail = MailState::new(Some(client));
        self.state.config.lock().unwrap().digital_ticket = Some(DigitalTicketConfig {
            region: "fr-par".into(),
            project_id: "project-1".into(),
            from_email: "tickets@example.be".into(),
            from_name: "The FPS Finances Experience".into(),
            api_base: Some(tem.uri()),
            api_key_file: "/unused/in/tests".into(),
            allow_digital_only,
        });
    }

    pub fn set_printer(&self, addr: &str) {
        self.state.config.lock().unwrap().printer_address = Some(addr.to_string());
    }

    /// The `ticketMedium` of the archived request of event `id`.
    pub fn signed_ticket_medium(&self, fiscal_event_id: i64) -> String {
        let conn = self.state.db.lock().unwrap();
        conn.query_row(
            "SELECT json_extract(event_data_json, '$.ticketMedium') FROM fiscal_events WHERE id = ?1",
            [fiscal_event_id],
            |r| r.get(0),
        )
        .expect("archived event")
    }

    pub fn deliveries(&self) -> Vec<pos_db::queries::deliveries::TicketDelivery> {
        let conn = self.state.db.lock().unwrap();
        pos_db::queries::deliveries::list_recent(&conn, 100).unwrap()
    }

    pub fn original_printed(&self, fiscal_event_id: i64) -> bool {
        let conn = self.state.db.lock().unwrap();
        pos_db::queries::prints::original_printed(&conn, fiscal_event_id).unwrap()
    }
}

/// Serve the admission image's mock FDM (`pos-fdm-mock`) on a free
/// local port and point the app at it: every event is signed like the
/// real mock signs it, with no canned responses to mount.
impl TestApp {
    pub async fn use_admission_mock(&self) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, pos_fdm_mock::app()).await.unwrap();
        });
        self.state.config.lock().unwrap().fdm_url = format!("http://{addr}/graphql");
    }
}
