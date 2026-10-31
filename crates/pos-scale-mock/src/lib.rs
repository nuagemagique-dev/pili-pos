//! A scale that speaks Dialog 06 over TCP. Tests set what is on it through
//! [`MockScale`]; a running one takes the same changes over HTTP.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use pos_core::money::weighed_amount_cents;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const EOT: u8 = 0x04;
const ENQ: u8 = 0x05;
const STX: u8 = 0x02;
const ETX: u8 = 0x03;
const ESC: u8 = 0x1B;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;

/// The scale asks for the check again after this many weighings.
const CHECK_EVERY: u32 = 50;

#[derive(Debug, Clone, Serialize, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent switches of the simulator, set one by one by the tests"
)]
pub struct ScaleState {
    /// `None`: nothing to answer with, as if switched off (no answers).
    pub off: bool,
    /// Grams on the scale.
    pub load: i64,
    /// Answers "still in motion" this many more times.
    pub motion: u32,
    /// The current load was already weighed.
    pub weighed: bool,
    /// The software check passed.
    pub verified: bool,
    /// The verdict waiting for the next ENQ.
    pub verdict: Option<bool>,
    pub unit_price_cents: Option<i64>,
    pub tare_grams: Option<i64>,
    pub text: Option<String>,
    pub weighings: u32,
    pub weighings_since_check: u32,
    /// The record 09 code of the next NAK, when forced.
    pub forced_status: Option<u8>,
    /// Sent to the price it answers with, in cents.
    pub price_error_cents: i64,
    /// The unit price the record 02 echoes, when wrong on purpose.
    pub echo_price_cents: Option<i64>,
    /// The next answer is damaged.
    pub corrupt_next: bool,
    /// The scale weighs in another unit (record 02 status).
    pub status_byte: Option<u8>,
    /// Requests seen.
    pub received: Vec<String>,
    /// How many check rounds the POS answered.
    pub checks_answered: u32,
    last_status: u8,
    random: u16,
}

#[derive(Clone, Debug)]
pub struct MockScale {
    state: Arc<Mutex<ScaleState>>,
    expected_check: Arc<Mutex<String>>,
}

impl MockScale {
    /// A scale that accepts `check` as the record 10 data, with the check
    /// already done. [`MockScale::require_check`] asks for it again.
    pub fn new(check: impl Into<String>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ScaleState {
                verified: true,
                random: 0xEF,
                ..Default::default()
            })),
            expected_check: Arc::new(Mutex::new(check.into())),
        }
    }

    pub fn with<T>(&self, change: impl FnOnce(&mut ScaleState) -> T) -> T {
        change(&mut self.state.lock().expect("scale state"))
    }

    pub fn state(&self) -> ScaleState {
        self.with(|s| s.clone())
    }

    /// Put `grams` on the scale, as if the last load had been taken off.
    pub fn load(&self, grams: i64) {
        self.with(|s| {
            s.load = grams;
            s.weighed = false;
        });
    }

    /// Ask for the software check, as after power on.
    pub fn require_check(&self) {
        self.with(|s| {
            s.verified = false;
            s.verdict = None;
        });
    }

    /// The Dialog 06 port, on a free local port, for a test.
    pub async fn spawn(&self) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        let scale = self.clone();
        tokio::spawn(async move { scale.accept(listener).await });
        address
    }

    pub async fn serve_dialog(&self, address: SocketAddr) -> std::io::Result<()> {
        let listener = TcpListener::bind(address).await?;
        self.accept(listener).await;
        Ok(())
    }

    async fn accept(&self, listener: TcpListener) {
        while let Ok((stream, _)) = listener.accept().await {
            let scale = self.clone();
            tokio::spawn(async move { scale.talk(stream).await });
        }
    }

    async fn talk(&self, mut stream: TcpStream) {
        let mut pending: Vec<u8> = Vec::new();
        let mut buffer = [0u8; 256];
        loop {
            let Ok(n) = stream.read(&mut buffer).await else {
                return;
            };
            if n == 0 {
                return;
            }
            pending.extend_from_slice(&buffer[..n]);
            while let Some(request) = take_request(&mut pending) {
                let answer = self.answer(&request);
                if !answer.is_empty() && stream.write_all(&answer).await.is_err() {
                    return;
                }
            }
        }
    }

    /// The answer to one request, without its leading EOT.
    pub fn answer(&self, request: &[u8]) -> Vec<u8> {
        let expected = self.expected_check.lock().expect("check").clone();
        let mut state = self.state.lock().expect("scale state");
        let state = &mut *state;
        if state.off {
            return Vec::new();
        }
        state
            .received
            .push(String::from_utf8_lossy(request).into_owned());
        let answer = match request {
            [ENQ] => enq(state),
            [] => Vec::new(), // EOT alone resets the interface
            [STX, body @ .., ETX] => record(state, body, &expected),
            _ => {
                state.last_status = 2;
                vec![NAK]
            }
        };
        if state.corrupt_next && !answer.is_empty() {
            state.corrupt_next = false;
            return vec![0xFF, 0xFE];
        }
        answer
    }

    pub fn control_router(&self) -> Router {
        Router::new()
            .route("/state", get(control_state))
            .route("/load", post(control_load))
            .route("/motion", post(control_motion))
            .route("/check", post(control_check))
            .route("/status", post(control_status))
            .route("/corrupt", post(control_corrupt))
            .route("/power", post(control_power))
            .with_state(self.clone())
    }
}

/// The next complete request of the stream: from an EOT to the following
/// ETX, or an EOT ENQ, or a lone EOT before another request.
fn take_request(pending: &mut Vec<u8>) -> Option<Vec<u8>> {
    let start = pending.iter().position(|&b| b == EOT)?;
    pending.drain(..start);
    match pending.get(1) {
        None => None,
        Some(&ENQ) => {
            pending.drain(..2);
            Some(vec![ENQ])
        }
        Some(&STX) => {
            let end = pending.iter().position(|&b| b == ETX)?;
            let request = pending[1..=end].to_vec();
            pending.drain(..=end);
            Some(request)
        }
        Some(&EOT) => {
            pending.drain(..1);
            Some(Vec::new())
        }
        Some(_) => {
            pending.drain(..1);
            Some(vec![0xFF])
        }
    }
}

fn check_request(state: &ScaleState) -> Vec<u8> {
    let mut out = vec![STX, b'1', b'1', ESC, b'2'];
    out.extend_from_slice(format!("{:02X}", state.random).as_bytes());
    out.push(ETX);
    out
}

fn nak(state: &mut ScaleState, code: u8) -> Vec<u8> {
    state.last_status = code;
    vec![NAK]
}

fn enq(state: &mut ScaleState) -> Vec<u8> {
    if let Some(ok) = state.verdict.take() {
        state.verified = ok;
        return vec![STX, b'1', b'1', ESC, if ok { b'1' } else { b'0' }, ETX];
    }
    if !state.verified {
        return check_request(state);
    }
    if let Some(code) = state.forced_status.take() {
        return nak(state, code);
    }
    let Some(unit_price) = state.unit_price_cents else {
        return nak(state, 22);
    };
    if state.motion > 0 {
        state.motion -= 1;
        return nak(state, 20);
    }
    if state.load < 0 {
        return nak(state, 31);
    }
    if state.load == 0 || state.load < 40 {
        return nak(state, 30);
    }
    if state.load > 15_000 {
        return nak(state, 32);
    }
    if state.weighed {
        return nak(state, 21);
    }
    state.weighed = true;
    state.weighings += 1;
    state.weighings_since_check += 1;
    let selling = weighed_amount_cents(state.load, unit_price) + state.price_error_cents;
    let echo = state.echo_price_cents.unwrap_or(unit_price);
    let width = if unit_price > 99_999 || echo > 99_999 {
        6
    } else {
        5
    };
    let status = state.status_byte.unwrap_or(0x33);
    let mut out = vec![STX, b'0', b'2', ESC, status, ESC];
    out.extend_from_slice(format!("{:05}", state.load).as_bytes());
    out.push(ESC);
    out.extend_from_slice(format!("{echo:0width$}").as_bytes());
    out.push(ESC);
    out.extend_from_slice(format!("{selling:06}").as_bytes());
    out.push(ETX);
    state.unit_price_cents = None;
    if state.weighings_since_check >= CHECK_EVERY {
        state.verified = false;
        state.weighings_since_check = 0;
    }
    out
}

fn record(state: &mut ScaleState, body: &[u8], expected: &str) -> Vec<u8> {
    let fields: Vec<&[u8]> = body.split(|&b| b == ESC).collect();
    match fields.first().copied() {
        Some(b"01" | b"03" | b"04" | b"05") => price(state, &fields),
        Some(b"08") => {
            let code = state.last_status;
            let mut out = vec![STX, b'0', b'9', ESC];
            out.extend_from_slice(format!("{code:02}").as_bytes());
            out.push(ETX);
            out
        }
        Some(b"10") => {
            let got = fields.get(1).copied().unwrap_or_default();
            state.verdict = Some(got == expected.as_bytes());
            state.checks_answered += 1;
            vec![ACK]
        }
        Some(b"20") => vec![ACK],
        _ => nak(state, 10),
    }
}

fn price(state: &mut ScaleState, fields: &[&[u8]]) -> Vec<u8> {
    if !state.verified {
        // "If POS transmits a unit price and the checksum check has not
        // been carried out, the scale will respond by transmitting record
        // 11 (32H, Z)."
        return check_request(state);
    }
    let digits = |field: &[u8]| -> Option<i64> {
        std::str::from_utf8(field)
            .ok()
            .filter(|t| t.bytes().all(|b| b.is_ascii_digit()) && !t.is_empty())?
            .parse()
            .ok()
    };
    let Some(unit_price) = fields
        .get(1)
        .filter(|f| matches!(f.len(), 5 | 6))
        .and_then(|f| digits(f))
    else {
        return nak(state, 11);
    };
    let (mut tare, mut text) = (None, None);
    match fields[0] {
        b"01" => {
            // The record closes with a separator: an empty last field.
            if fields.len() != 3 || !fields[2].is_empty() {
                return nak(state, 2);
            }
        }
        b"03" | b"05" => {
            let Some(t) = fields
                .get(2)
                .filter(|f| f.len() == 4)
                .and_then(|f| digits(f))
            else {
                return nak(state, 12);
            };
            tare = Some(t);
            if fields[0] == b"05" {
                let Some(t) = fields.get(3).filter(|f| f.len() == 13) else {
                    return nak(state, 13);
                };
                text = Some(String::from_utf8_lossy(t).into_owned());
            }
        }
        _ => {
            let Some(t) = fields.get(2).filter(|f| f.len() == 13) else {
                return nak(state, 13);
            };
            text = Some(String::from_utf8_lossy(t).into_owned());
        }
    }
    state.unit_price_cents = Some(unit_price);
    state.tare_grams = tare;
    state.text = text;
    vec![ACK]
}

#[derive(Deserialize)]
struct LoadBody {
    grams: i64,
}

#[derive(Deserialize)]
struct MotionBody {
    polls: u32,
}

#[derive(Deserialize)]
struct CheckBody {
    pending: bool,
}

#[derive(Deserialize)]
struct StatusBody {
    code: u8,
}

#[derive(Deserialize)]
struct PowerBody {
    on: bool,
}

async fn control_state(State(scale): State<MockScale>) -> Json<ScaleState> {
    Json(scale.state())
}

async fn control_load(State(scale): State<MockScale>, Json(body): Json<LoadBody>) -> &'static str {
    scale.load(body.grams);
    "ok"
}

async fn control_motion(
    State(scale): State<MockScale>,
    Json(body): Json<MotionBody>,
) -> &'static str {
    scale.with(|s| s.motion = body.polls);
    "ok"
}

async fn control_check(
    State(scale): State<MockScale>,
    Json(body): Json<CheckBody>,
) -> &'static str {
    if body.pending {
        scale.require_check();
    } else {
        scale.with(|s| s.verified = true);
    }
    "ok"
}

async fn control_status(
    State(scale): State<MockScale>,
    Json(body): Json<StatusBody>,
) -> &'static str {
    scale.with(|s| s.forced_status = Some(body.code));
    "ok"
}

async fn control_corrupt(State(scale): State<MockScale>) -> &'static str {
    scale.with(|s| s.corrupt_next = true);
    "ok"
}

async fn control_power(
    State(scale): State<MockScale>,
    Json(body): Json<PowerBody>,
) -> &'static str {
    scale.with(|s| s.off = !body.on);
    "ok"
}
