//! An FDM that loses answers: a raw TCP front for `pos_fdm_mock::app()`
//! that records every request and, on demand, closes the connection
//! without answering (after the mock signed, or before it saw the
//! request), or holds an answer back.
//!
//! A real lost answer would take the client's 10 s timeout; closing
//! the socket gives the same "maybe signed" outcome at once.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// What the front does with one sign request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fate {
    Answer,
    /// The mock signs it, the answer is lost.
    LoseAnswer,
    /// The connection drops before the mock sees it.
    LoseRequest,
    /// The answer comes back after this long.
    Delay(Duration),
}

#[derive(Debug, Clone)]
pub struct Sent {
    /// The sign mutation's first field (`signSale`, ...), else "other".
    pub op: String,
    pub body: String,
    pub fate: Fate,
    /// The mock's answer, when it saw the request.
    pub answer: Option<serde_json::Value>,
}

impl Sent {
    pub fn variables(&self) -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(&self.body).unwrap()["variables"].clone()
    }
}

type Rule = Box<dyn Fn(&str, usize) -> Fate + Send + Sync>;

pub struct LossyFdm {
    pub url: String,
    sent: Arc<Mutex<Vec<Sent>>>,
    rule: Arc<Mutex<Rule>>,
}

const OPS: [&str; 16] = [
    "signReportTurnoverZ",
    "signReportTurnoverX",
    "signReportUserZ",
    "signReportUserX",
    "signSale",
    "signOrder",
    "signPreBill",
    "signCostCenterChange",
    "signMoneyInOut",
    "signDrawerOpen",
    "signPaymentCorrection",
    "signWorkIn",
    "signWorkOut",
    "signInvoice",
    "signInvoiceCreditNote",
    "signCopy",
];

fn op_of(body: &str) -> String {
    let query = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v["query"].as_str().map(str::to_string))
        .unwrap_or_default();
    OPS.iter()
        .find(|op| query.contains(&format!("{op}(")))
        .map(|op| op.to_string())
        .unwrap_or_else(|| "other".into())
}

impl LossyFdm {
    /// Every request answered until [`LossyFdm::rule`] says otherwise.
    pub async fn start() -> Self {
        let inner = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let inner_url = format!("http://{}/graphql", inner.local_addr().unwrap());
        tokio::spawn(async move {
            axum::serve(inner, pos_fdm_mock::app()).await.unwrap();
        });
        let front = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/graphql", front.local_addr().unwrap());
        let sent: Arc<Mutex<Vec<Sent>>> = Arc::default();
        let rule: Arc<Mutex<Rule>> = Arc::new(Mutex::new(Box::new(|_, _| Fate::Answer)));
        let http = reqwest::Client::new();
        {
            let sent = sent.clone();
            let rule = rule.clone();
            tokio::spawn(async move {
                loop {
                    let Ok((socket, _)) = front.accept().await else {
                        return;
                    };
                    tokio::spawn(serve(
                        socket,
                        inner_url.clone(),
                        http.clone(),
                        sent.clone(),
                        rule.clone(),
                    ));
                }
            });
        }
        Self { url, sent, rule }
    }

    /// Decide each sign request's fate from its op and its index among
    /// sign requests (0 = the first one sent).
    pub fn rule(&self, rule: impl Fn(&str, usize) -> Fate + Send + Sync + 'static) {
        *self.rule.lock().unwrap() = Box::new(rule);
    }

    /// Lose the answer to the first `op` request, answer the rest.
    pub fn lose_first(&self, op: &'static str) {
        let lost = Arc::new(Mutex::new(false));
        self.rule(move |o, _| {
            let mut lost = lost.lock().unwrap();
            if o == op && !*lost {
                *lost = true;
                Fate::LoseAnswer
            } else {
                Fate::Answer
            }
        });
    }

    pub fn sent(&self) -> Vec<Sent> {
        self.sent.lock().unwrap().clone()
    }

    pub fn sent_op(&self, op: &str) -> Vec<Sent> {
        self.sent().into_iter().filter(|s| s.op == op).collect()
    }
}

async fn read_request(socket: &mut TcpStream) -> Option<String> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let header_end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        let n = socket.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).to_ascii_lowercase();
    let length: usize = head
        .lines()
        .find_map(|l| l.strip_prefix("content-length:"))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    while buf.len() < header_end + length {
        let n = socket.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    Some(String::from_utf8_lossy(&buf[header_end..header_end + length]).into_owned())
}

async fn serve(
    mut socket: TcpStream,
    inner_url: String,
    http: reqwest::Client,
    sent: Arc<Mutex<Vec<Sent>>>,
    rule: Arc<Mutex<Rule>>,
) {
    while let Some(body) = read_request(&mut socket).await {
        let op = op_of(&body);
        let fate = if op == "other" {
            Fate::Answer
        } else {
            let index = sent
                .lock()
                .unwrap()
                .iter()
                .filter(|s| s.op != "other")
                .count();
            (rule.lock().unwrap())(&op, index)
        };
        let answer = if fate == Fate::LoseRequest {
            None
        } else {
            let text = http
                .post(&inner_url)
                .header("content-type", "application/json")
                .body(body.clone())
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            Some(text)
        };
        sent.lock().unwrap().push(Sent {
            op,
            body,
            fate,
            answer: answer.as_deref().map(|a| serde_json::from_str(a).unwrap()),
        });
        match fate {
            Fate::LoseAnswer | Fate::LoseRequest => return,
            Fate::Delay(d) => tokio::time::sleep(d).await,
            Fate::Answer => {}
        }
        let answer = answer.unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{answer}",
            answer.len()
        );
        if socket.write_all(response.as_bytes()).await.is_err() {
            return;
        }
    }
}

impl super::harness::TestApp {
    /// Sign against `fdm` instead of the wiremock FDM.
    pub fn use_lossy_fdm(&self, fdm: &LossyFdm) {
        self.state.config.lock().unwrap().fdm_url = fdm.url.clone();
    }
}
