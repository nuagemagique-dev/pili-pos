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

/// Timeout for reading DLE EOT status response.
const STATUS_READ_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("Connection failed: {0}")]
    Connect(String),
    #[error("Write failed: {0}")]
    Write(String),
    #[error("Connection timeout")]
    Timeout,
    #[error("Printer not configured")]
    NotConfigured,
}

/// Post-print health status queried via DLE EOT. If the printer
/// doesn't support status queries (or the read times out), all
/// fields default to `true` / empty (optimistic).
#[derive(Debug, Clone, Serialize)]
pub struct PrinterStatus {
    pub paper_ok: bool,
    pub cover_closed: bool,
    pub warnings: Vec<String>,
}

impl Default for PrinterStatus {
    fn default() -> Self {
        Self {
            paper_ok: true,
            cover_closed: true,
            warnings: Vec::new(),
        }
    }
}

/// TCP transport for sending ESC/POS data to a network printer.
///
/// The Epson TM-m30III exposes a raw TCP socket on port 9100.
/// Data sent to this port is printed directly with no protocol overhead.
pub struct TcpPrinter {
    address: String,
}

impl TcpPrinter {
    /// Create a new TCP printer transport.
    /// `address` format: "ip:port" or just "ip" (defaults to port 9100).
    pub fn new(address: &str) -> Self {
        let address = if address.contains(':') {
            address.to_string()
        } else {
            format!("{address}:{DEFAULT_PRINTER_PORT}")
        };
        Self { address }
    }

    /// Send raw ESC/POS bytes to the printer and query its health
    /// status via DLE EOT. The print data is sent first; if the
    /// status query fails or times out, an optimistic default is
    /// returned (print still happened, just no health feedback).
    pub async fn send(&self, data: &[u8]) -> Result<PrinterStatus, TransportError> {
        let stream = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(&self.address))
            .await
            .map_err(|_| TransportError::Timeout)?
            .map_err(|e| TransportError::Connect(e.to_string()))?;

        let mut stream = stream;

        tokio::time::timeout(WRITE_TIMEOUT, stream.write_all(data))
            .await
            .map_err(|_| TransportError::Timeout)?
            .map_err(|e| TransportError::Write(e.to_string()))?;

        tokio::time::timeout(WRITE_TIMEOUT, stream.flush())
            .await
            .map_err(|_| TransportError::Timeout)?
            .map_err(|e| TransportError::Write(e.to_string()))?;

        // DLE EOT status query - best-effort. If the printer doesn't
        // respond (older models, unidirectional TCP) we return a
        // default "everything OK" status so the checkout isn't blocked.
        let status = query_printer_status(&mut stream).await;

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

/// Query the printer's real-time status via ESC/POS DLE EOT command.
/// Returns a default (optimistic) status on timeout or read error.
async fn query_printer_status(stream: &mut TcpStream) -> PrinterStatus {
    let mut status = PrinterStatus::default();

    // DLE EOT 1 = paper sensor status
    // DLE EOT 3 = error status (cover, paper feed)
    for &(cmd_n, label) in &[(1u8, "paper"), (3u8, "error")] {
        let cmd = [0x10u8, 0x04, cmd_n]; // DLE EOT n
        if stream.write_all(&cmd).await.is_err() {
            return status; // Unidirectional - give up.
        }
        let _ = stream.flush().await;

        let mut buf = [0u8; 1];
        let read_result =
            tokio::time::timeout(STATUS_READ_TIMEOUT, stream.read_exact(&mut buf)).await;

        match read_result {
            Ok(Ok(_)) => {
                let byte = buf[0];
                match label {
                    "paper" => {
                        // Bit 5: paper near-end, Bit 6: paper end
                        if byte & (1 << 6) != 0 {
                            status.paper_ok = false;
                            status.warnings.push("paper_end".to_string());
                        } else if byte & (1 << 5) != 0 {
                            status.warnings.push("paper_near_end".to_string());
                        }
                    }
                    "error" => {
                        // Bit 2: cover open, Bit 5: paper feed error
                        if byte & (1 << 2) != 0 {
                            status.cover_closed = false;
                            status.warnings.push("cover_open".to_string());
                        }
                        if byte & (1 << 5) != 0 {
                            status.paper_ok = false;
                            status.warnings.push("feed_error".to_string());
                        }
                    }
                    _ => {}
                }
            }
            _ => {
                // Timeout or read error - printer may not support
                // DLE EOT. Return what we have so far.
                return status;
            }
        }
    }

    status
}

#[cfg(test)]
mod tests {
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

        let received = server.await.unwrap();
        assert!(
            received.starts_with(&data),
            "printer must receive the exact ESC/POS bytes, got {received:?}"
        );
    }
}
