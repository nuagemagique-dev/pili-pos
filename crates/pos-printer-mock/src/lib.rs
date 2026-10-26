//! A network thermal printer for development, demos and tests, standing
//! in for the Epson TM-m30III the POS prints to (raw TCP, port 9100).
//!
//! - Every connection is one job. The job is kept with its ESC/POS
//!   bytes rendered as the paper shows them (`pos_print::render`),
//!   whether it opened the cash drawer, and whether it came out.
//! - The real-time status queries (DLE EOT n) are answered as an Epson
//!   printer answers them, from switchable states: paper near end or
//!   out, cover open, error. A printer stopped by one of them keeps
//!   the job in its buffer: the job is recorded as not printed.
//! - "Unplugged" closes the port: connections are refused, as when the
//!   printer is off or the cable is out.
//!
//! A small HTTP server shows the jobs and switches the states:
//! `GET /` (the jobs as printed), `GET /jobs`, `GET /jobs/{id}`,
//! `DELETE /jobs`, `GET` / `POST /control`.

#![expect(
    clippy::print_stderr,
    reason = "the mock reports a failed outbox write or re-bind on the console"
)]

use std::fmt::Write as _;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use pos_print::transport::{
    DLE_EOT, OFFLINE_COVER_OPEN, OFFLINE_ERROR, OFFLINE_PAPER_END, PAPER_END, PAPER_NEAR_END,
    STATUS_FIXED,
};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

/// A connection silent this long is over.
const IDLE: Duration = Duration::from_secs(2);

/// ESC p: open the cash drawer.
const DRAWER_KICK: [u8; 2] = [0x1B, b'p'];

/// The paper roll.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Paper {
    #[default]
    Ok,
    NearEnd,
    End,
}

/// The printer's switchable states.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Control {
    pub paper: Paper,
    pub cover_open: bool,
    pub error: bool,
    pub unplugged: bool,
}

impl Control {
    /// Start-up states: `MOCK_PRINTER_PAPER` (`ok`, `near_end`, `end`),
    /// `MOCK_PRINTER_COVER_OPEN`, `MOCK_PRINTER_ERROR`,
    /// `MOCK_PRINTER_UNPLUGGED` (`1` or `true`).
    pub fn from_env() -> Self {
        let flag = |key: &str| {
            std::env::var(key).is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        };
        Self {
            paper: match std::env::var("MOCK_PRINTER_PAPER").as_deref() {
                Ok("near_end") => Paper::NearEnd,
                Ok("end") => Paper::End,
                _ => Paper::Ok,
            },
            cover_open: flag("MOCK_PRINTER_COVER_OPEN"),
            error: flag("MOCK_PRINTER_ERROR"),
            unplugged: flag("MOCK_PRINTER_UNPLUGGED"),
        }
    }

    /// Stopped: a job waits in the buffer instead of printing.
    pub fn offline(&self) -> bool {
        self.cover_open || self.error || self.paper == Paper::End
    }

    /// The answer to DLE EOT n (Epson ESC/POS real-time status).
    pub fn status_byte(&self, n: u8) -> u8 {
        let mut byte = STATUS_FIXED;
        match n {
            // Printer status: bit 3 offline.
            1 if self.offline() => byte |= 0x08,
            // Offline cause.
            2 => {
                if self.cover_open {
                    byte |= OFFLINE_COVER_OPEN;
                }
                if self.paper == Paper::End {
                    byte |= OFFLINE_PAPER_END;
                }
                if self.error {
                    byte |= OFFLINE_ERROR;
                }
            }
            // Error cause: bit 6 automatically recoverable error.
            3 if self.error => byte |= 0x40,
            // Roll paper sensor.
            4 => match self.paper {
                Paper::Ok => {}
                Paper::NearEnd => byte |= PAPER_NEAR_END,
                Paper::End => byte |= PAPER_NEAR_END | PAPER_END,
            },
            _ => {}
        }
        byte
    }
}

/// One connection's worth of printing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: usize,
    pub at: String,
    /// ESC/POS bytes received, status queries left out.
    pub bytes: usize,
    /// Came out of the printer (false: stopped, kept in the buffer).
    pub printed: bool,
    pub drawer_kick: bool,
    /// The job as the paper shows it.
    pub text: String,
}

/// The printer: its states and the jobs it received.
pub struct Printer {
    control: Mutex<Control>,
    unplugged: watch::Sender<bool>,
    jobs: Mutex<Vec<Job>>,
    next_id: AtomicUsize,
    outbox: Option<PathBuf>,
}

impl std::fmt::Debug for Printer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Printer").finish_non_exhaustive()
    }
}

impl Printer {
    /// A printer in the given states. With an `outbox`, each job is also
    /// written there as `<id>.txt` (rendered) and `<id>.bin` (raw).
    pub fn new(control: Control, outbox: Option<PathBuf>) -> Arc<Self> {
        let (unplugged, _) = watch::channel(control.unplugged);
        Arc::new(Self {
            control: Mutex::new(control),
            unplugged,
            jobs: Mutex::new(Vec::new()),
            next_id: AtomicUsize::new(1),
            outbox,
        })
    }

    /// States from the environment, outbox from `MOCK_PRINTER_OUTBOX`.
    pub fn from_env() -> Arc<Self> {
        Self::new(
            Control::from_env(),
            std::env::var("MOCK_PRINTER_OUTBOX").ok().map(PathBuf::from),
        )
    }

    pub fn control(&self) -> Control {
        self.control
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Switch the states, all of them at once.
    pub fn set_control(&self, control: Control) {
        let unplugged = control.unplugged;
        *self
            .control
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = control;
        self.unplugged.send_replace(unplugged);
    }

    pub fn jobs(&self) -> Vec<Job> {
        self.jobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn clear(&self) {
        self.jobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    /// Keep what a connection sent, status queries left out. A
    /// connection that printed nothing (a reachability check, a status
    /// query alone) is not a job.
    fn record(&self, raw: &[u8]) {
        let bytes = without_status_queries(raw);
        if bytes.is_empty() {
            return;
        }
        let job = Job {
            id: self.next_id.fetch_add(1, Ordering::SeqCst),
            at: chrono::Local::now().to_rfc3339(),
            bytes: bytes.len(),
            printed: !self.control().offline(),
            drawer_kick: bytes.windows(2).any(|w| w == DRAWER_KICK),
            text: pos_print::render::render_plain_text(&bytes),
        };
        if let Some(dir) = &self.outbox {
            let written = std::fs::create_dir_all(dir)
                .and_then(|()| std::fs::write(dir.join(format!("{:04}.txt", job.id)), &job.text))
                .and_then(|()| std::fs::write(dir.join(format!("{:04}.bin", job.id)), &bytes));
            if let Err(e) = written {
                eprintln!(
                    "printer mock: cannot write job {} to {}: {e}",
                    job.id,
                    dir.display()
                );
            }
        }
        self.jobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(job);
    }
}

/// The bytes with every complete DLE EOT n taken out.
fn without_status_queries(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i..].starts_with(&DLE_EOT) && i + 2 < raw.len() {
            i += 3;
        } else {
            out.push(raw[i]);
            i += 1;
        }
    }
    out
}

async fn handle(printer: Arc<Printer>, mut stream: TcpStream) {
    let mut data = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut scanned = 0;
    loop {
        match tokio::time::timeout(IDLE, stream.read(&mut chunk)).await {
            Ok(Ok(0) | Err(_)) | Err(_) => break,
            Ok(Ok(n)) => {
                data.extend_from_slice(&chunk[..n]);
                // Answer each complete DLE EOT n, in order.
                while let Some(at) = data[scanned..].windows(2).position(|w| w == DLE_EOT) {
                    let at = scanned + at;
                    let Some(&n) = data.get(at + 2) else { break };
                    let byte = printer.control().status_byte(n);
                    let _ = stream.write_all(&[byte]).await;
                    scanned = at + 3;
                }
            }
        }
    }
    printer.record(&data);
}

/// Listen for print jobs on `addr` (port 0 picks one) and return the
/// address listened on. While the printer is unplugged the port is
/// closed; plugging it back opens the same port again.
pub async fn bind(addr: SocketAddr, printer: Arc<Printer>) -> std::io::Result<SocketAddr> {
    let listener = TcpListener::bind(addr).await?;
    let addr = listener.local_addr()?;
    tokio::spawn(serve(listener, addr, printer));
    Ok(addr)
}

async fn serve(first: TcpListener, addr: SocketAddr, printer: Arc<Printer>) {
    let mut unplugged = printer.unplugged.subscribe();
    let mut listener = Some(first);
    loop {
        if *unplugged.borrow_and_update() {
            listener = None;
            if unplugged.changed().await.is_err() {
                return;
            }
            continue;
        }
        let current = match listener.take() {
            Some(l) => l,
            None => match TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("printer mock: cannot listen on {addr} again: {e}");
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
            },
        };
        loop {
            tokio::select! {
                accepted = current.accept() => {
                    if let Ok((stream, _)) = accepted {
                        tokio::spawn(handle(Arc::clone(&printer), stream));
                    }
                }
                changed = unplugged.changed() => {
                    if changed.is_err() {
                        return;
                    }
                    if *unplugged.borrow() {
                        break; // drop the listener: connections refused
                    }
                }
            }
        }
    }
}

/// The HTTP side: the jobs and the switches.
pub fn http_app(printer: Arc<Printer>) -> Router {
    Router::new()
        .route("/", get(page))
        .route("/jobs", get(list_jobs).delete(clear_jobs))
        .route("/jobs/{id}", get(job_text))
        .route("/control", get(get_control).post(set_control))
        .with_state(printer)
}

async fn list_jobs(State(printer): State<Arc<Printer>>) -> Json<Vec<Job>> {
    Json(printer.jobs())
}

async fn clear_jobs(State(printer): State<Arc<Printer>>) -> StatusCode {
    printer.clear();
    StatusCode::NO_CONTENT
}

async fn job_text(State(printer): State<Arc<Printer>>, Path(id): Path<usize>) -> impl IntoResponse {
    match printer.jobs().into_iter().find(|j| j.id == id) {
        Some(job) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            job.text,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn get_control(State(printer): State<Arc<Printer>>) -> Json<Control> {
    Json(printer.control())
}

async fn set_control(
    State(printer): State<Arc<Printer>>,
    Json(control): Json<Control>,
) -> Json<Control> {
    printer.set_control(control);
    Json(printer.control())
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The jobs as printed, newest first, refreshed every 3 s.
async fn page(State(printer): State<Arc<Printer>>) -> Html<String> {
    let control = printer.control();
    let mut html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta http-equiv=\"refresh\" content=\"3\"><title>Printer mock</title>\
         <style>body{{font-family:sans-serif;background:#eee;margin:16px}}\
         pre{{background:#fff;width:48ch;padding:12px;font-size:13px;overflow-x:auto}}\
         .held{{opacity:.5}}</style></head><body>\
         <h1>Printer mock</h1><p>States: <code>{}</code></p>",
        escape(&serde_json::to_string(&control).unwrap_or_default())
    );
    for job in printer.jobs().iter().rev() {
        let _ = write!(
            html,
            "<h2>Job {} <small>{}{}{}</small></h2><pre class=\"{}\">{}</pre>",
            job.id,
            escape(&job.at),
            if job.drawer_kick {
                ", drawer opened"
            } else {
                ""
            },
            if job.printed {
                ""
            } else {
                ", held (printer stopped)"
            },
            if job.printed { "" } else { "held" },
            escape(&job.text),
        );
    }
    html.push_str("</body></html>");
    Html(html)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use pos_print::transport::{TransportError, send_to_printer};
    use tower::ServiceExt;

    async fn printer(control: Control) -> (Arc<Printer>, String) {
        let printer = Printer::new(control, None);
        let addr = bind("127.0.0.1:0".parse().unwrap(), Arc::clone(&printer))
            .await
            .unwrap();
        (printer, addr.to_string())
    }

    /// Jobs are recorded when the connection closes.
    async fn settled(printer: &Printer, n: usize) -> Vec<Job> {
        for _ in 0..40 {
            if printer.jobs().len() >= n {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        printer.jobs()
    }

    const RECEIPT: &[u8] = b"\x1B@Bonjour\nTotal 12,00\n";

    #[tokio::test]
    async fn a_ready_printer_prints_and_says_so() {
        let (printer, addr) = printer(Control::default()).await;
        let status = send_to_printer(&addr, RECEIPT).await.unwrap();
        assert!(status.ready(), "{status:?}");
        assert_eq!(status.warnings.len(), 0);
        let jobs = settled(&printer, 1).await;
        assert_eq!(jobs.len(), 1);
        assert!(jobs[0].printed);
        assert!(!jobs[0].drawer_kick);
        assert_eq!(jobs[0].bytes, RECEIPT.len(), "status queries left out");
        assert!(jobs[0].text.contains("Total 12,00"), "{}", jobs[0].text);
    }

    #[tokio::test]
    async fn each_state_reaches_the_pos_as_an_epson_printer_reports_it() {
        let cases = [
            (
                Control {
                    paper: Paper::NearEnd,
                    ..Control::default()
                },
                true,
                "paper_near_end",
            ),
            (
                Control {
                    paper: Paper::End,
                    ..Control::default()
                },
                false,
                "paper_end",
            ),
            (
                Control {
                    cover_open: true,
                    ..Control::default()
                },
                false,
                "cover_open",
            ),
            (
                Control {
                    error: true,
                    ..Control::default()
                },
                false,
                "error",
            ),
        ];
        for (control, ready, warning) in cases {
            let (printer, addr) = printer(control.clone()).await;
            let status = send_to_printer(&addr, RECEIPT).await.unwrap();
            assert_eq!(status.ready(), ready, "{control:?}: {status:?}");
            assert_eq!(status.warnings, [warning], "{control:?}");
            let jobs = settled(&printer, 1).await;
            assert_eq!(jobs[0].printed, ready, "{control:?}: held when stopped");
        }
    }

    #[tokio::test]
    async fn the_drawer_kick_is_noted() {
        let (printer, addr) = printer(Control::default()).await;
        send_to_printer(&addr, &pos_print::drawer::kick_drawer_bytes())
            .await
            .unwrap();
        assert!(settled(&printer, 1).await[0].drawer_kick);
    }

    #[tokio::test]
    async fn a_reachability_check_is_not_a_job() {
        let (printer, addr) = printer(Control::default()).await;
        assert!(pos_print::transport::TcpPrinter::new(&addr).ping().await);
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(printer.jobs().is_empty());
    }

    #[tokio::test]
    async fn unplugged_refuses_connections_until_plugged_back() {
        let (printer, addr) = printer(Control::default()).await;
        printer.set_control(Control {
            unplugged: true,
            ..Control::default()
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        let err = send_to_printer(&addr, RECEIPT).await.unwrap_err();
        assert!(matches!(err, TransportError::Connect(_)), "{err:?}");

        printer.set_control(Control::default());
        let mut sent = false;
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if send_to_printer(&addr, RECEIPT).await.is_ok() {
                sent = true;
                break;
            }
        }
        assert!(sent, "plugged back on the same port");
        assert_eq!(settled(&printer, 1).await.len(), 1);
    }

    #[tokio::test]
    async fn the_http_side_switches_states_and_lists_jobs() {
        let (printer, addr) = printer(Control::default()).await;
        let app = http_app(Arc::clone(&printer));

        let response = app
            .clone()
            .oneshot(
                Request::post("/control")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"paper":"end"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(printer.control().paper, Paper::End);

        let status = send_to_printer(&addr, RECEIPT).await.unwrap();
        assert!(!status.paper_ok);
        settled(&printer, 1).await;

        let response = app
            .clone()
            .oneshot(Request::get("/jobs").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let jobs: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(jobs[0]["printed"], false);

        let response = app
            .clone()
            .oneshot(Request::get("/jobs/1").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&body).contains("Bonjour"));

        let response = app
            .clone()
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8_lossy(&body);
        assert!(html.contains("held (printer stopped)"), "{html}");

        let response = app
            .oneshot(Request::delete("/jobs").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(printer.jobs().is_empty());
    }

    #[test]
    fn status_queries_are_taken_out_of_the_job() {
        let raw = [b'a', 0x10, 0x04, 2, b'b', 0x10, 0x04, 4, 0x10, 0x04];
        // A query cut at the end of the stream stays: it was not asked.
        assert_eq!(without_status_queries(&raw), [b'a', b'b', 0x10, 0x04]);
    }
}
