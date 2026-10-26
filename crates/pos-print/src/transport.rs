use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Default port for Epson TM-m30III raw TCP printing.
pub const DEFAULT_PRINTER_PORT: u16 = 9100;

/// Connection timeout for printer.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Write timeout for printer data.
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

/// Default wait for one DLE EOT status answer (Linux). A shell whose
/// network path answers late (the Android emulator, a slow Wi-Fi hop)
/// passes a longer wait through [`TcpPrinter::with_status_timeout`] or
/// [`send_to_printer_with`].
pub const DEFAULT_STATUS_READ_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("Connection failed: {0}")]
    Connect(String),
    #[error("Write failed: {0}")]
    Write(String),
    #[error("Connection timeout")]
    ConnectTimeout,
    #[error("Write timeout")]
    WriteTimeout,
    #[error("Printer not configured")]
    NotConfigured,
}

/// Post-print health status queried via DLE EOT. If the printer
/// doesn't support status queries (or the read times out), all
/// fields default to `true` / empty (optimistic) and `status_known`
/// is `false`.
#[derive(Debug, Clone, Serialize)]
pub struct PrinterStatus {
    /// `true` only when the printer answered both DLE EOT queries.
    /// `false` means the other fields are the optimistic default, not
    /// something the printer said (no answer, or an answer after the
    /// read timeout).
    pub status_known: bool,
    pub paper_ok: bool,
    pub cover_closed: bool,
    /// No error stops the printer (cutter jam, head too hot, ...).
    pub error_free: bool,
    pub warnings: Vec<String>,
}

impl Default for PrinterStatus {
    fn default() -> Self {
        Self {
            paper_ok: true,
            cover_closed: true,
            error_free: true,
            warnings: Vec::new(),
            status_known: false,
        }
    }
}

impl PrinterStatus {
    /// Nothing keeps the job from coming out of the printer.
    pub fn ready(&self) -> bool {
        self.paper_ok && self.cover_closed && self.error_free
    }

    /// Read the answer to DLE EOT 2, the offline cause (Epson ESC/POS:
    /// bit 2 cover open, bit 5 stopped at paper end, bit 6 error).
    pub fn read_offline_cause(&mut self, byte: u8) {
        if byte & OFFLINE_COVER_OPEN != 0 {
            self.cover_closed = false;
            self.warnings.push("cover_open".to_string());
        }
        if byte & OFFLINE_PAPER_END != 0 && self.paper_ok {
            self.paper_ok = false;
            self.warnings.push("paper_end".to_string());
        }
        if byte & OFFLINE_ERROR != 0 {
            self.error_free = false;
            self.warnings.push("error".to_string());
        }
    }

    /// Read the answer to DLE EOT 4, the roll paper sensor (Epson
    /// ESC/POS: bits 2-3 near end, bits 5-6 paper end).
    pub fn read_paper_sensor(&mut self, byte: u8) {
        if byte & PAPER_END != 0 {
            if self.paper_ok {
                self.paper_ok = false;
                self.warnings.push("paper_end".to_string());
            }
        } else if byte & PAPER_NEAR_END != 0 {
            self.warnings.push("paper_near_end".to_string());
        }
    }
}

/// DLE EOT n: transmit real-time status n.
pub const DLE_EOT: [u8; 2] = [0x10, 0x04];
/// DLE EOT 2 answer: cover open.
pub const OFFLINE_COVER_OPEN: u8 = 0x04;
/// DLE EOT 2 answer: printing stopped at paper end.
pub const OFFLINE_PAPER_END: u8 = 0x20;
/// DLE EOT 2 answer: an error occurred.
pub const OFFLINE_ERROR: u8 = 0x40;
/// DLE EOT 4 answer: roll paper near end (both sensor bits).
pub const PAPER_NEAR_END: u8 = 0x0C;
/// DLE EOT 4 answer: roll paper end (both sensor bits).
pub const PAPER_END: u8 = 0x60;
/// The bits every status byte carries (bits 1 and 4 fixed to 1).
pub const STATUS_FIXED: u8 = 0x12;

/// `address` as `host:port`: a missing port is the default 9100, a bare IPv6 address gets its
/// brackets (`::1` is `[::1]:9100`), `host:port`, `[v6]:port` and `v4:port` stay as written.
fn normalize_address(address: &str) -> String {
    let address = address.trim();
    if address.parse::<SocketAddr>().is_ok() {
        return address.to_string();
    }
    if let Ok(ip) = address.parse::<IpAddr>() {
        return SocketAddr::new(ip, DEFAULT_PRINTER_PORT).to_string();
    }
    if address.starts_with('[') && address.ends_with(']') {
        return format!("{address}:{DEFAULT_PRINTER_PORT}");
    }
    match address.matches(':').count() {
        0 => format!("{address}:{DEFAULT_PRINTER_PORT}"),
        // host:port
        1 => address.to_string(),
        // An IPv6 form that did not parse: leave it to the connect error.
        _ => format!("[{address}]:{DEFAULT_PRINTER_PORT}"),
    }
}

/// TCP transport for sending ESC/POS data to a network printer.
///
/// The Epson TM-m30III exposes a raw TCP socket on port 9100.
/// Data sent to this port is printed directly with no protocol overhead.
pub struct TcpPrinter {
    address: String,
    status_timeout: Duration,
}

impl TcpPrinter {
    /// Create a new TCP printer transport.
    /// `address` format: "ip:port" or just "ip" (defaults to port 9100).
    pub fn new(address: &str) -> Self {
        Self {
            address: normalize_address(address),
            status_timeout: DEFAULT_STATUS_READ_TIMEOUT,
        }
    }

    /// Wait `timeout` for each DLE EOT answer instead of
    /// [`DEFAULT_STATUS_READ_TIMEOUT`].
    pub fn with_status_timeout(mut self, timeout: Duration) -> Self {
        self.status_timeout = timeout;
        self
    }

    /// Send raw ESC/POS bytes to the printer and query its health
    /// status via DLE EOT. The print data is sent first; if the
    /// status query fails or times out, an optimistic default is
    /// returned (print still happened, just no health feedback).
    pub async fn send(&self, data: &[u8]) -> Result<PrinterStatus, TransportError> {
        let stream = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(&self.address))
            .await
            .map_err(|_| TransportError::ConnectTimeout)?
            .map_err(|e| TransportError::Connect(e.to_string()))?;

        let mut stream = stream;

        tokio::time::timeout(WRITE_TIMEOUT, stream.write_all(data))
            .await
            .map_err(|_| TransportError::WriteTimeout)?
            .map_err(|e| TransportError::Write(e.to_string()))?;

        tokio::time::timeout(WRITE_TIMEOUT, stream.flush())
            .await
            .map_err(|_| TransportError::WriteTimeout)?
            .map_err(|e| TransportError::Write(e.to_string()))?;

        // DLE EOT status query - best-effort. If the printer doesn't
        // respond (older models, unidirectional TCP) we return a
        // default "everything OK" status so the checkout isn't blocked.
        let status = query_printer_status(&mut stream, self.status_timeout).await;

        // Shutdown the write half gracefully
        let _ = stream.shutdown().await;

        Ok(status)
    }

    /// Check if the printer is reachable by attempting a TCP connection.
    pub async fn ping(&self) -> bool {
        tokio::time::timeout(Duration::from_secs(2), TcpStream::connect(&self.address))
            .await
            .map(|r| r.is_ok())
            .unwrap_or(false)
    }

    pub fn address(&self) -> &str {
        &self.address
    }
}

/// Send ESC/POS data to a printer at the given address.
/// Convenience function that creates a one-shot connection.
pub async fn send_to_printer(address: &str, data: &[u8]) -> Result<PrinterStatus, TransportError> {
    let printer = TcpPrinter::new(address);
    printer.send(data).await
}

/// [`send_to_printer`] waiting `status_timeout` for each DLE EOT answer.
pub async fn send_to_printer_with(
    address: &str,
    data: &[u8],
    status_timeout: Duration,
) -> Result<PrinterStatus, TransportError> {
    TcpPrinter::new(address)
        .with_status_timeout(status_timeout)
        .send(data)
        .await
}

/// Query the printer's real-time status via ESC/POS DLE EOT: the
/// offline cause (n = 2) and the roll paper sensor (n = 4).
/// Returns a default (optimistic) status, `status_known` false, on
/// timeout or read error; `status_known` is set only after both answers.
async fn query_printer_status(stream: &mut TcpStream, read_timeout: Duration) -> PrinterStatus {
    let mut status = PrinterStatus::default();

    for n in [2u8, 4] {
        if stream
            .write_all(&[DLE_EOT[0], DLE_EOT[1], n])
            .await
            .is_err()
        {
            return status; // Unidirectional - give up.
        }
        let _ = stream.flush().await;

        let mut buf = [0u8; 1];
        match tokio::time::timeout(read_timeout, stream.read_exact(&mut buf)).await {
            Ok(Ok(_)) if n == 2 => status.read_offline_cause(buf[0]),
            Ok(Ok(_)) => status.read_paper_sensor(buf[0]),
            // Timeout or read error - printer may not support
            // DLE EOT. Return what we have so far.
            _ => return status,
        }
    }

    status.status_known = true;
    status
}

#[cfg(test)]
mod tests {
    #[test]
    fn printer_addresses_are_normalised() {
        use super::normalize_address as n;
        assert_eq!(n("192.168.1.5"), "192.168.1.5:9100");
        assert_eq!(n("192.168.1.5:9101"), "192.168.1.5:9101");
        assert_eq!(n("printer.lan"), "printer.lan:9100");
        assert_eq!(n("printer.lan:9101"), "printer.lan:9101");
        assert_eq!(n("::1"), "[::1]:9100");
        assert_eq!(n("fe80::1"), "[fe80::1]:9100");
        assert_eq!(n("[::1]"), "[::1]:9100");
        assert_eq!(n("[::1]:9101"), "[::1]:9101");
    }

    use super::*;

    #[test]
    fn address_default_port() {
        let p = TcpPrinter::new("192.168.1.200");
        assert_eq!(p.address(), "192.168.1.200:9100");
    }

    #[test]
    fn address_custom_port() {
        let p = TcpPrinter::new("192.168.1.200:9101");
        assert_eq!(p.address(), "192.168.1.200:9101");
    }

    #[test]
    fn printer_status_default_is_optimistic() {
        let status = PrinterStatus::default();
        assert!(status.paper_ok);
        assert!(status.cover_closed);
        assert!(status.warnings.is_empty());
        assert!(status.ready());
        assert!(!status.status_known);
    }

    #[test]
    fn an_all_fine_printer_answers_the_fixed_bits_only() {
        let mut status = PrinterStatus::default();
        status.read_offline_cause(STATUS_FIXED);
        status.read_paper_sensor(STATUS_FIXED);
        assert!(status.ready());
        assert!(status.warnings.is_empty());
    }

    #[test]
    fn the_offline_cause_reports_cover_paper_end_and_error() {
        let mut status = PrinterStatus::default();
        status.read_offline_cause(STATUS_FIXED | OFFLINE_COVER_OPEN);
        assert!(!status.cover_closed && status.paper_ok && status.error_free);

        let mut status = PrinterStatus::default();
        status.read_offline_cause(STATUS_FIXED | OFFLINE_ERROR);
        assert!(!status.error_free && !status.ready());
        assert_eq!(status.warnings, ["error"]);
    }

    #[test]
    fn paper_end_is_reported_once_by_both_bytes() {
        let mut status = PrinterStatus::default();
        status.read_offline_cause(STATUS_FIXED | OFFLINE_PAPER_END);
        status.read_paper_sensor(STATUS_FIXED | PAPER_END);
        assert!(!status.paper_ok);
        assert_eq!(status.warnings, ["paper_end"]);
    }

    #[test]
    fn paper_near_end_is_a_warning_only() {
        let mut status = PrinterStatus::default();
        status.read_paper_sensor(STATUS_FIXED | PAPER_NEAR_END);
        assert!(status.ready());
        assert_eq!(status.warnings, ["paper_near_end"]);
    }

    #[test]
    fn the_feed_button_is_not_a_paper_problem() {
        // Paper being fed with the button (DLE EOT 2 bit 3) stops
        // nothing. The feed button bit of DLE EOT 1 used to be read as
        // paper end.
        let mut status = PrinterStatus::default();
        status.read_offline_cause(STATUS_FIXED | 0x08); // paper fed by the button
        assert!(status.ready());
    }

    /// Bind then immediately drop a listener: the OS refuses new
    /// connections on that port, giving a genuinely unreachable
    /// target without depending on external network access.
    fn refused_port() -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        addr.to_string()
    }

    #[tokio::test]
    async fn send_to_an_unreachable_printer_returns_err_quickly() {
        let printer = TcpPrinter::new(&refused_port());
        let start = std::time::Instant::now();
        let err = printer.send(b"\x1B@test").await.unwrap_err();
        let elapsed = start.elapsed();

        assert!(
            matches!(err, TransportError::Connect(_)),
            "expected Connect error, got {err:?}"
        );
        assert!(
            elapsed < Duration::from_secs(1),
            "an unreachable printer must fail fast, took {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn ping_returns_false_quickly_for_an_unreachable_printer() {
        let printer = TcpPrinter::new(&refused_port());
        let start = std::time::Instant::now();
        assert!(!printer.ping().await);
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "ping must fail fast against an unreachable printer"
        );
    }

    #[tokio::test]
    async fn send_to_printer_convenience_fn_also_fails_fast_when_unreachable() {
        let addr = refused_port();
        let start = std::time::Instant::now();
        let err = send_to_printer(&addr, b"data").await.unwrap_err();
        assert!(matches!(err, TransportError::Connect(_)));
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn send_delivers_the_exact_bytes_to_a_real_socket() {
        // A minimal fake printer: accept one connection, read until
        // the client closes its write half, and hand back everything
        // it received. Never responds to the DLE EOT status query,
        // exercising the "printer doesn't support status queries"
        // optimistic-default path at the same time.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = Vec::new();
            let _ = stream.read_to_end(&mut buf).await;
            buf
        });

        let printer = TcpPrinter::new(&addr.to_string());
        let data = b"\x1B@Hello receipt".to_vec();
        let status = printer.send(&data).await.unwrap();
        // No status response was sent by our fake server, so the
        // optimistic default must be preserved.
        assert!(status.paper_ok);
        assert!(status.cover_closed);
        assert!(!status.status_known);

        let received = server.await.unwrap();
        assert!(
            received.starts_with(&data),
            "printer must receive the exact ESC/POS bytes, got {received:?}"
        );
    }

    /// A fake printer: reads the print data (up to the first DLE EOT),
    /// then answers each DLE EOT n query after `delay` with the bytes
    /// of `answers` (`None`: never answers). Returns its address.
    async fn fake_printer(delay: Duration, answers: Option<[u8; 2]>) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut seen = Vec::new();
            let mut answered = 0;
            let mut buf = [0u8; 256];
            loop {
                let Ok(n) = stream.read(&mut buf).await else {
                    return;
                };
                if n == 0 {
                    return;
                }
                seen.extend_from_slice(&buf[..n]);
                while seen.len() >= 3 && seen.windows(3).any(|w| w[..2] == DLE_EOT) {
                    let at = seen.windows(3).position(|w| w[..2] == DLE_EOT).unwrap();
                    seen.drain(..at + 3);
                    if let Some(a) = answers {
                        tokio::time::sleep(delay).await;
                        if stream.write_all(&[a[answered.min(1)]]).await.is_err() {
                            return;
                        }
                        answered += 1;
                    }
                }
            }
        });
        addr.to_string()
    }

    const FINE: [u8; 2] = [STATUS_FIXED, STATUS_FIXED];
    const COVER_OPEN: [u8; 2] = [STATUS_FIXED | OFFLINE_COVER_OPEN, STATUS_FIXED];

    #[tokio::test]
    async fn a_printer_answering_both_queries_gives_a_known_status() {
        let addr = fake_printer(Duration::ZERO, Some(FINE)).await;
        let status = send_to_printer(&addr, b"\x1B@x").await.unwrap();
        assert!(status.status_known);
        assert!(status.ready());
        assert!(status.warnings.is_empty());
    }

    #[tokio::test]
    async fn a_silent_printer_is_unknown_yet_ready_within_the_default_wait() {
        let addr = fake_printer(Duration::ZERO, None).await;
        let start = std::time::Instant::now();
        let status = send_to_printer(&addr, b"\x1B@x").await.unwrap();
        assert!(!status.status_known);
        assert!(status.ready());
        assert!(
            start.elapsed() < Duration::from_millis(1500),
            "took {:?}",
            start.elapsed()
        );
    }

    #[tokio::test]
    async fn a_late_answer_is_lost_at_the_default_wait() {
        let addr = fake_printer(Duration::from_millis(800), Some(COVER_OPEN)).await;
        let status = send_to_printer(&addr, b"\x1B@x").await.unwrap();
        assert!(!status.status_known);
        assert!(status.cover_closed, "the optimistic default is kept");
        assert!(status.ready());
    }

    #[tokio::test]
    async fn a_late_answer_is_read_with_a_two_second_wait() {
        let addr = fake_printer(Duration::from_millis(800), Some(COVER_OPEN)).await;
        let status = TcpPrinter::new(&addr)
            .with_status_timeout(Duration::from_secs(2))
            .send(b"\x1B@x")
            .await
            .unwrap();
        assert!(status.status_known);
        assert!(!status.cover_closed);
        assert!(!status.ready());
        assert_eq!(status.warnings, ["cover_open"]);
    }

    #[tokio::test]
    async fn send_to_printer_with_uses_its_timeout() {
        let addr = fake_printer(Duration::from_millis(800), Some(COVER_OPEN)).await;
        let late = send_to_printer_with(&addr, b"\x1B@x", Duration::from_millis(100))
            .await
            .unwrap();
        assert!(!late.status_known);

        let addr = fake_printer(Duration::from_millis(800), Some(COVER_OPEN)).await;
        let patient = send_to_printer_with(&addr, b"\x1B@x", Duration::from_secs(2))
            .await
            .unwrap();
        assert!(patient.status_known);
        assert!(!patient.cover_closed);
    }

    #[test]
    fn the_default_status_wait_is_500_ms() {
        assert_eq!(DEFAULT_STATUS_READ_TIMEOUT, Duration::from_millis(500));
    }
}
