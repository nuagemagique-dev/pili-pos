//! OPI (Open Payment Initiative, XML) over the shop's LAN, as CCV
//! terminals speak it.
//!
//! **Read this before using it in production.** CCV's own OPI-NL profile
//! (the one a Belgian CCV terminal speaks) is not public. The shapes
//! below follow the public OPI samples: the OPI-DE minimal request that
//! works on a real terminal, the IFSF / Diebold Nixdorf Italian profile
//! (response, DeviceRequest, abort), and the attributes CCV's Android
//! SDK documents (`ElmeErrorCode`, `CaptureReference`, result states).
//! What a real Belgian terminal sends (the Bancontact `CardCircuit`
//! value, extra attributes, whether a `Login` is needed first) must be
//! learned from a trace of that terminal, and CCV certifies every
//! integration before it may take payments.
//!
//! Two TCP channels, as OPI-NL "dual socket":
//!
//! * **Channel 0**: the till connects to the terminal (`address`, usually
//!   port 4100), sends a `CardServiceRequest` `RequestType="CardPayment"`
//!   and waits for the `CardServiceResponse`. Every message is an XML
//!   document preceded by its length as 4 bytes, big endian.
//! * **Channel 1**: the till listens on `device_port` (4102 by default);
//!   the terminal connects back and sends `DeviceRequest`s (text for the
//!   cashier display, receipt lines to print). Each is answered with a
//!   `DeviceResponse` `OverallResult="Success"`; the lines for
//!   `OutDeviceTarget="PrinterReceipt"` are the customer receipt kept
//!   with the approval (`Printer` is the merchant copy).
//!
//! `RequestID` is a 9 digit number derived from the till reference, so
//! the same payment always carries the same id: that is how
//! `RepeatLastMessage` (sent by [`CcvOpi::status`]) is matched to the
//! payment, and the answer is accepted only if its `RequestID` equals it.
//! `AbortRequest` goes on a new channel 0 connection.
//!
//! Not in the public samples, so not sent: the per-payment screen
//! language (the terminal uses its own), `POPID`, the card circuits
//! collection.

use std::collections::HashSet;
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pos_core::config::PaymentTerminalConfig;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::{Instant, timeout, timeout_at};
use tracing::{debug, info, warn};

use crate::util::{cents_from_text, truncate};
use crate::xml::{Elem, escape, parse};
use crate::{Approval, EftError, Outcome, PaymentRequest, amount_text, brand, receipt};

const DEFAULT_DEVICE_PORT: u16 = 4102;
/// Largest message accepted from the terminal.
const MAX_FRAME: usize = 1 << 20;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Connect timeout of [`CcvOpi::check`].
const CHECK_TIMEOUT: Duration = Duration::from_secs(3);

/// A CCV terminal on the LAN, spoken to in OPI.
pub struct CcvOpi {
    address: String,
    workstation_id: String,
    software: String,
    device_port: u16,
    timeout: Duration,
    /// Till references the till asked to stop.
    aborted: Mutex<HashSet<String>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// FNV-1a, 64 bits: stable across builds, unlike the std hasher.
fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The OPI `RequestID` of a payment: 9 digits (CCV allows at most 9),
/// never starting with 0, the same for the same reference.
pub(crate) fn request_id(reference: &str) -> String {
    (100_000_000 + fnv1a(reference) % 900_000_000).to_string()
}

/// A decoded frame body: UTF-8, else Latin-1 (OPI-DE uses ISO-8859-1).
fn decode(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes)
        .unwrap_or_else(|e| e.into_bytes().iter().map(|&b| char::from(b)).collect())
}

async fn read_frame<R: AsyncReadExt + Unpin>(stream: &mut R) -> io::Result<String> {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len).await?;
    let len = u32::from_be_bytes(len) as usize;
    if len > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("frame of {len} bytes"),
        ));
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).await?;
    Ok(decode(body))
}

async fn write_frame<W: AsyncWriteExt + Unpin>(stream: &mut W, xml: &str) -> io::Result<()> {
    let bytes = xml.as_bytes();
    let len = u32::try_from(bytes.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "message too long"))?;
    let mut frame = Vec::with_capacity(4 + bytes.len());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(bytes);
    stream.write_all(&frame).await?;
    stream.flush().await
}

/// The receipt lines the terminal asked the till to print.
#[derive(Default)]
struct Printed {
    customer: Vec<String>,
    merchant: Vec<String>,
}

/// The till's side of channel 1 while a payment runs.
struct DeviceChannel {
    task: JoinHandle<()>,
    printed: Arc<Mutex<Printed>>,
}

impl Drop for DeviceChannel {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl DeviceChannel {
    fn customer_receipt(&self) -> Option<String> {
        let printed = lock(&self.printed);
        let text = printed.customer.join("\n");
        let text = text.trim().to_string();
        (!text.is_empty()).then_some(text)
    }
}

async fn serve_device(mut stream: TcpStream, printed: Arc<Mutex<Printed>>, software: String) {
    while let Ok(xml) = read_frame(&mut stream).await {
        let Ok(root) = parse(&xml) else {
            warn!("opi: unreadable device request");
            continue;
        };
        if !root.name.eq_ignore_ascii_case("DeviceRequest") {
            continue;
        }
        let request_type = root.attr("RequestType").unwrap_or("Output");
        let mut outputs = Vec::new();
        root.find_all("Output", &mut outputs);
        let mut results = String::new();
        for output in &outputs {
            let target = output.attr("OutDeviceTarget").unwrap_or("");
            let lines: Vec<String> = output
                .children
                .iter()
                .filter(|c| c.name.eq_ignore_ascii_case("TextLine"))
                .map(|c| c.text.trim_end().to_string())
                .collect();
            match target {
                "PrinterReceipt" => {
                    let mut p = lock(&printed);
                    p.customer.extend(lines);
                }
                "Printer" | "JournalPrinter" => {
                    let mut p = lock(&printed);
                    p.merchant.extend(lines);
                }
                _ => debug!(target, "opi: display text from the terminal"),
            }
            results.push_str(&format!(
                "<Output OutDeviceTarget=\"{}\" OutResult=\"Success\"/>",
                escape(target)
            ));
        }
        let overall = if request_type.eq_ignore_ascii_case("Output") {
            "Success"
        } else {
            // Input requests (signature, identification) are not served.
            "DeviceUnavailable"
        };
        let response = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><DeviceResponse RequestType=\"{}\" \
             ApplicationSender=\"{}\" WorkstationID=\"{}\" RequestID=\"{}\" OverallResult=\"{overall}\">{results}</DeviceResponse>",
            escape(request_type),
            escape(&truncate(&software, 60)),
            escape(root.attr("WorkstationID").unwrap_or("")),
            escape(root.attr("RequestID").unwrap_or("")),
        );
        if write_frame(&mut stream, &response).await.is_err() {
            break;
        }
    }
}

/// What reading one terminal message concluded.
enum Reading {
    /// Not the answer to this payment (another RequestID, a display
    /// message).
    NotOurs,
    Done(Result<Outcome, EftError>),
}

impl CcvOpi {
    pub fn new(config: &PaymentTerminalConfig, software: &str) -> Result<Self, EftError> {
        let address = config
            .address
            .as_deref()
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .ok_or_else(|| EftError::Config("payment_terminal.address: required".into()))?
            .to_string();
        Ok(Self {
            address,
            workstation_id: truncate(
                config
                    .terminal_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .unwrap_or("POS1"),
                16,
            ),
            software: truncate(software, 60),
            device_port: config.device_port.unwrap_or(DEFAULT_DEVICE_PORT),
            timeout: crate::timeout(config),
            aborted: Mutex::new(HashSet::new()),
        })
    }

    #[cfg(test)]
    pub(crate) fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// A `CardServiceRequest` of `request_type`.
    pub(crate) fn card_request(
        &self,
        request_type: &str,
        request_id: &str,
        amount_cents: Option<i64>,
    ) -> String {
        let now = chrono::Utc::now()
            .with_timezone(&chrono_tz::Europe::Brussels)
            .format("%Y-%m-%dT%H:%M:%S%:z");
        let amount = amount_cents.map_or_else(String::new, |c| {
            format!(
                "<TotalAmount Currency=\"EUR\">{}</TotalAmount>",
                amount_text(c)
            )
        });
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><CardServiceRequest RequestType=\"{}\" \
             ApplicationSender=\"{}\" WorkstationID=\"{}\" RequestID=\"{}\">\
             <POSdata><POSTimeStamp>{now}</POSTimeStamp></POSdata>{amount}</CardServiceRequest>",
            escape(request_type),
            escape(&self.software),
            escape(&self.workstation_id),
            escape(request_id),
        )
    }

    async fn connect(&self) -> Result<TcpStream, EftError> {
        match timeout(CONNECT_TIMEOUT, TcpStream::connect(&self.address)).await {
            Ok(Ok(stream)) => {
                let _ = stream.set_nodelay(true);
                Ok(stream)
            }
            Ok(Err(e)) => Err(EftError::Unreachable(format!(
                "cannot connect to the terminal at {}: {e}",
                self.address
            ))),
            Err(_) => Err(EftError::Unreachable(format!(
                "the terminal at {} does not answer",
                self.address
            ))),
        }
    }

    /// Listen for the terminal's channel 1.
    async fn open_device_channel(&self) -> Result<DeviceChannel, EftError> {
        let address = SocketAddr::from(([0, 0, 0, 0], self.device_port));
        // The previous payment's listener is dropped by its aborted task a
        // moment after the payment returned: give it a little time before
        // calling the port taken.
        let mut attempts = 0;
        let listener = loop {
            match TcpListener::bind(address).await {
                Ok(l) => break l,
                Err(e) if e.kind() == io::ErrorKind::AddrInUse && attempts < 10 => {
                    attempts += 1;
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
                    warn!(port = self.device_port, "opi: device port in use");
                    return Err(EftError::Busy);
                }
                Err(e) => {
                    return Err(EftError::Config(format!(
                        "cannot listen on the device port {}: {e}",
                        self.device_port
                    )));
                }
            }
        };
        let printed = Arc::new(Mutex::new(Printed::default()));
        let shared = printed.clone();
        let software = self.software.clone();
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            while let Ok((stream, _)) = listener.accept().await {
                connections.spawn(serve_device(stream, shared.clone(), software.clone()));
            }
        });
        Ok(DeviceChannel { task, printed })
    }

    pub async fn pay(&self, request: &PaymentRequest) -> Result<Outcome, EftError> {
        let id = request_id(&request.reference);
        let device = self.open_device_channel().await?;
        let mut stream = self.connect().await?;
        info!(
            reference = %request.reference,
            request_id = %id,
            amount_cents = request.amount_cents,
            "opi payment sent"
        );
        let xml = self.card_request("CardPayment", &id, Some(request.amount_cents));
        if let Err(e) = write_frame(&mut stream, &xml).await {
            return Ok(Outcome::Unknown {
                detail: format!("the request may not have reached the terminal ({e})"),
            });
        }
        let deadline = Instant::now() + self.timeout;
        loop {
            let xml = match timeout_at(deadline, read_frame(&mut stream)).await {
                Ok(Ok(xml)) => xml,
                Ok(Err(e)) => {
                    return Ok(Outcome::Unknown {
                        detail: format!("connection to the terminal lost ({e})"),
                    });
                }
                Err(_) => {
                    return Ok(Outcome::Unknown {
                        detail: format!(
                            "no answer from the terminal within {} s",
                            self.timeout.as_secs()
                        ),
                    });
                }
            };
            let root = match parse(&xml) {
                Ok(r) => r,
                Err(e) => {
                    return Ok(Outcome::Unknown {
                        detail: format!("unreadable answer from the terminal ({e})"),
                    });
                }
            };
            let by_till = lock(&self.aborted).contains(&request.reference);
            match interpret(&root, &id, Some(request.amount_cents), by_till, false) {
                Reading::NotOurs => debug!("opi: message that is not the payment's answer"),
                Reading::Done(Ok(Outcome::Approved(mut approval))) => {
                    approval.customer_receipt = device.customer_receipt();
                    // The circuit does not say debit or credit; the receipt may.
                    if approval.card_type == crate::CardType::Unknown
                        && let Some(t) = approval
                            .customer_receipt
                            .as_deref()
                            .and_then(receipt::account_type)
                    {
                        approval.card_type = t;
                    }
                    return Ok(Outcome::Approved(approval));
                }
                Reading::Done(result) => return result,
            }
        }
    }

    pub async fn status(&self, reference: &str) -> Result<Outcome, EftError> {
        let id = request_id(reference);
        let mut stream = self.connect().await?;
        let repeat_id = request_id(&format!("{reference}#repeat"));
        let xml = self.card_request("RepeatLastMessage", &repeat_id, None);
        if let Err(e) = write_frame(&mut stream, &xml).await {
            return Ok(Outcome::Unknown {
                detail: format!("RepeatLastMessage not delivered ({e})"),
            });
        }
        let wait = self.timeout.min(Duration::from_secs(30));
        let xml = match timeout(wait, read_frame(&mut stream)).await {
            Ok(Ok(xml)) => xml,
            Ok(Err(e)) => {
                return Ok(Outcome::Unknown {
                    detail: format!("connection to the terminal lost ({e})"),
                });
            }
            Err(_) => {
                return Ok(Outcome::Unknown {
                    detail: "no answer to RepeatLastMessage".into(),
                });
            }
        };
        let Ok(root) = parse(&xml) else {
            return Ok(Outcome::Unknown {
                detail: "unreadable answer to RepeatLastMessage".into(),
            });
        };
        let by_till = lock(&self.aborted).contains(reference);
        match interpret(&root, &id, None, by_till, true) {
            Reading::NotOurs => Ok(Outcome::Unknown {
                detail: describe_other(&root),
            }),
            // A busy terminal is not an error of the status call.
            Reading::Done(Err(EftError::Busy)) => Ok(Outcome::Unknown {
                detail: "the terminal is busy".into(),
            }),
            Reading::Done(result) => result,
        }
    }

    /// Reachability check: a TCP connection to the terminal's channel 0
    /// address, closed at once. No message is sent, so nothing can be
    /// charged and the terminal's payment state is untouched. It proves
    /// the terminal listens, not that it is ready for a card.
    pub async fn check(&self) -> Result<(), EftError> {
        match timeout(CHECK_TIMEOUT, TcpStream::connect(&self.address)).await {
            Ok(Ok(mut stream)) => {
                let _ = stream.shutdown().await;
                Ok(())
            }
            Ok(Err(e)) => Err(EftError::Unreachable(format!(
                "cannot connect to the terminal at {}: {e}",
                self.address
            ))),
            Err(_) => Err(EftError::Unreachable(format!(
                "the terminal at {} does not answer",
                self.address
            ))),
        }
    }

    pub async fn abort(&self, reference: &str) -> Result<(), EftError> {
        lock(&self.aborted).insert(reference.to_string());
        let mut stream = self.connect().await?;
        let xml = self.card_request(
            "AbortRequest",
            &request_id(&format!("{reference}#abort")),
            None,
        );
        write_frame(&mut stream, &xml)
            .await
            .map_err(|e| EftError::Unreachable(format!("abort not delivered: {e}")))?;
        // The answer only says whether the abort took; the payment's own
        // connection carries the final result either way.
        match timeout(Duration::from_secs(5), read_frame(&mut stream)).await {
            Ok(Ok(xml)) => debug!(
                "opi: abort answered {}",
                parse(&xml)
                    .ok()
                    .and_then(|r| r.attr("OverallResult").map(str::to_string))
                    .unwrap_or_default()
            ),
            Ok(Err(_)) | Err(_) => debug!("opi: no answer to the abort"),
        }
        Ok(())
    }
}

/// What a message that is not the awaited answer says, for the log.
fn describe_other(root: &Elem) -> String {
    match root.attr("OverallResult") {
        Some(r) => format!(
            "the terminal's last message is not this payment (result {r}, RequestID {})",
            root.attr("RequestID").unwrap_or("?")
        ),
        None => "the terminal's last message is not this payment".into(),
    }
}

fn first_attr<'a>(elem: &'a Elem, names: &[&str]) -> Option<&'a str> {
    names
        .iter()
        .find_map(|n| elem.attr(n).map(str::trim).filter(|v| !v.is_empty()))
}

/// One message from channel 0 read as the answer to payment `id`.
/// `strict` demands the `RequestID` to match (a repeated message);
/// otherwise a missing one is accepted.
#[cfg(test)]
pub(crate) fn interpret_response(
    root: &Elem,
    id: &str,
    expected: Option<i64>,
    aborted_by_till: bool,
    strict: bool,
) -> Result<Outcome, EftError> {
    match interpret(root, id, expected, aborted_by_till, strict) {
        Reading::Done(r) => r,
        Reading::NotOurs => Ok(Outcome::Unknown {
            detail: describe_other(root),
        }),
    }
}

fn interpret(
    root: &Elem,
    id: &str,
    expected: Option<i64>,
    aborted_by_till: bool,
    strict: bool,
) -> Reading {
    let mut candidates = Vec::new();
    root.find_all("CardServiceResponse", &mut candidates);
    let Some(response) = candidates
        .into_iter()
        .find(|c| c.attr("RequestType").is_none_or(|t| t == "CardPayment"))
    else {
        return Reading::NotOurs;
    };
    match response.attr("RequestID") {
        Some(got) if got != id => return Reading::NotOurs,
        None if strict => return Reading::NotOurs,
        _ => {}
    }
    let result = response.attr("OverallResult").unwrap_or("");
    let authorisation = response.find("Authorisation");
    let reason = || {
        let text = first_attr(response, &["ElmeErrorText"])
            .or_else(|| authorisation.and_then(|a| first_attr(a, &["ResponseComment"])))
            .or_else(|| first_attr(response, &["ElmeErrorCode"]))
            .or_else(|| authorisation.and_then(|a| first_attr(a, &["ReturnCode", "ActionCode"])))
            .unwrap_or("Failure");
        text.to_string()
    };
    Reading::Done(match result {
        "Success" => Ok(approval(response, authorisation, expected)),
        "Failure" => Ok(Outcome::Declined { reason: reason() }),
        "Aborted" => Ok(if aborted_by_till {
            Outcome::Cancelled
        } else {
            Outcome::Declined {
                reason: "stopped at the terminal".into(),
            }
        }),
        "Busy" => Err(EftError::Busy),
        "DeviceUnavailable" => Err(EftError::Unreachable(
            "the terminal's payment device is unavailable".into(),
        )),
        "FormatError"
        | "ParsingError"
        | "ValidationError"
        | "MissingMandatoryData"
        | "Loggedout" => Err(EftError::Rejected(format!(
            "the terminal refused the request: {result}{}",
            first_attr(response, &["ElmeErrorText"])
                .map(|t| format!(" ({t})"))
                .unwrap_or_default()
        ))),
        other => Ok(Outcome::Unknown {
            detail: format!(
                "terminal result {}",
                if other.is_empty() { "missing" } else { other }
            ),
        }),
    })
}

fn approval(response: &Elem, authorisation: Option<&Elem>, expected: Option<i64>) -> Outcome {
    let tender_amount = response.find("TotalAmount");
    let amount = tender_amount
        .map(Elem::text)
        .filter(|t| !t.is_empty())
        .and_then(cents_from_text)
        .or_else(|| {
            tender_amount
                .and_then(|t| t.attr("PaymentAmount"))
                .and_then(cents_from_text)
        });
    let amount_cents = match (amount, expected) {
        (Some(a), Some(e)) if a != e => {
            return Outcome::Unknown {
                detail: format!("approved amount {a} cents differs from the {e} cents requested"),
            };
        }
        (Some(a), _) => a,
        (None, Some(e)) => e,
        (None, None) => {
            return Outcome::Unknown {
                detail: "success without an amount".into(),
            };
        }
    };
    if let Some(currency) = tender_amount.and_then(|t| t.attr("Currency"))
        && !currency.eq_ignore_ascii_case("EUR")
    {
        return Outcome::Unknown {
            detail: format!("approved in {currency}"),
        };
    }
    let circuit = authorisation.and_then(|a| first_attr(a, &["CardCircuit"]));
    let normalized = circuit.and_then(brand::normalize);
    let terminal = response.find("Terminal");
    let stan = terminal.and_then(|t| first_attr(t, &["STAN"]));
    let terminal_id = terminal.and_then(|t| first_attr(t, &["TerminalID"]));
    let terminal_reference = authorisation
        .and_then(|a| first_attr(a, &["CaptureReference"]))
        .map(str::to_string)
        .or_else(|| match (terminal_id, stan) {
            (Some(t), Some(s)) => Some(format!("{t}/{s}")),
            (None, Some(s)) => Some(s.to_string()),
            _ => None,
        })
        .or_else(|| {
            authorisation
                .and_then(|a| first_attr(a, &["ReceiptNumber"]))
                .map(str::to_string)
        });
    Outcome::Approved(Approval {
        amount_cents,
        card_type: circuit
            .and_then(receipt::explicit_card_type)
            .unwrap_or_else(|| brand::card_type_of(normalized.as_deref())),
        brand: normalized,
        terminal_reference,
        authorisation_code: authorisation
            .and_then(|a| first_attr(a, &["ApprovalCode"]))
            .map(str::to_string),
        masked_pan: authorisation
            .and_then(|a| first_attr(a, &["PANPrint", "CardPAN"]))
            .map(str::to_string),
        customer_receipt: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardType;

    // Source: https://www.sebeto.com/intranet/ftpscambio/BuonoChiaro/Aevi%20Pay-OPI-Interface%201.16.pdf
    // (Italian OPI profile, sections 4.2 and 6.2.1) and the OPI-DE gist
    // https://gist.github.com/t-oster/a1bed2bb82d566bbdb90706d521a8f83
    const RESPONSE: &str =
        include_str!("../tests/fixtures/opi_card_service_response_italian_profile.xml");
    const DEVICE_REQUEST: &str =
        include_str!("../tests/fixtures/opi_device_request_italian_profile.xml");
    const DE_MINIMAL: &str = include_str!("../tests/fixtures/opi_card_request_de_minimal.xml");

    fn config() -> PaymentTerminalConfig {
        PaymentTerminalConfig {
            kind: pos_core::config::PaymentTerminalKind::CcvOpi,
            base_url: None,
            api_key_file: None,
            merchant_id: None,
            terminal_id: Some("TILL-7".into()),
            integrator_id: None,
            management_system_id: None,
            access_protocol: None,
            address: Some("127.0.0.1:4100".into()),
            device_port: None,
            timeout_seconds: 60,
        }
    }

    #[test]
    fn request_ids_are_nine_digits_and_stable() {
        let a = request_id("6f1f0a52-0000-4000-8000-000000000001");
        assert_eq!(a.len(), 9);
        assert!(!a.starts_with('0'));
        assert_eq!(a, request_id("6f1f0a52-0000-4000-8000-000000000001"));
        assert_ne!(a, request_id("6f1f0a52-0000-4000-8000-000000000002"));
    }

    #[test]
    fn the_request_has_the_shape_of_the_public_samples() {
        let opi = CcvOpi::new(&config(), "pili-pos/1.0.0").unwrap();
        let xml = opi.card_request("CardPayment", "123456789", Some(1250));
        let root = parse(&xml).unwrap();
        assert_eq!(root.name, "CardServiceRequest");
        assert_eq!(root.attr("RequestType"), Some("CardPayment"));
        assert_eq!(root.attr("WorkstationID"), Some("TILL-7"));
        assert_eq!(root.attr("RequestID"), Some("123456789"));
        assert_eq!(root.attr("ApplicationSender"), Some("pili-pos/1.0.0"));
        let amount = root.child("TotalAmount").unwrap();
        assert_eq!(amount.attr("Currency"), Some("EUR"));
        assert_eq!(amount.text(), "12.50");
        assert!(
            root.child("POSdata")
                .unwrap()
                .child("POSTimeStamp")
                .is_some()
        );
        // The same elements as the minimal request of the OPI-DE sample.
        let sample = parse(DE_MINIMAL).unwrap();
        assert_eq!(sample.attr("RequestType"), root.attr("RequestType"));
        assert!(sample.child("POSdata").is_some());
        // Without a terminal id the workstation is POS1.
        let mut c = config();
        c.terminal_id = None;
        let xml = CcvOpi::new(&c, "x")
            .unwrap()
            .card_request("AbortRequest", "1", None);
        assert!(xml.contains("WorkstationID=\"POS1\""));
        assert!(!xml.contains("TotalAmount"));
    }

    #[test]
    fn the_public_response_example_is_an_approval() {
        let root = parse(RESPONSE).unwrap();
        let out = interpret_response(&root, "1", Some(100_000), false, true).unwrap();
        let Outcome::Approved(a) = out else {
            panic!("{out:?}")
        };
        assert_eq!(a.amount_cents, 100_000);
        assert_eq!(a.brand.as_deref(), Some("Mastercard"));
        assert_eq!(a.card_type, CardType::Unknown);
        assert_eq!(a.masked_pan.as_deref(), Some("361709*****900"));
        assert_eq!(a.terminal_reference.as_deref(), Some("832632/000178"));
        // Another request id is not this payment's answer.
        let other = interpret_response(&root, "2", Some(100_000), false, true).unwrap();
        assert!(matches!(other, Outcome::Unknown { .. }));
        // Another amount is never an approval.
        let wrong = interpret_response(&root, "1", Some(500), false, true).unwrap();
        assert!(matches!(wrong, Outcome::Unknown { .. }), "{wrong:?}");
    }

    fn answer(result: &str) -> Elem {
        parse(&format!(
            "<CardServiceResponse RequestType=\"CardPayment\" RequestID=\"1\" OverallResult=\"{result}\" \
             ElmeErrorText=\"why\"/>"
        ))
        .unwrap()
    }

    #[test]
    fn overall_results_map_to_outcomes() {
        let run =
            |r: &str, by_till: bool| interpret_response(&answer(r), "1", Some(1), by_till, false);
        assert!(
            matches!(run("Failure", false), Ok(Outcome::Declined { reason }) if reason == "why")
        );
        assert_eq!(run("Aborted", true).unwrap(), Outcome::Cancelled);
        assert!(matches!(
            run("Aborted", false),
            Ok(Outcome::Declined { .. })
        ));
        assert!(matches!(run("Busy", false), Err(EftError::Busy)));
        assert!(matches!(
            run("DeviceUnavailable", false),
            Err(EftError::Unreachable(_))
        ));
        assert!(matches!(
            run("FormatError", false),
            Err(EftError::Rejected(_))
        ));
        assert!(matches!(
            run("TimedOut", false),
            Ok(Outcome::Unknown { .. })
        ));
        assert!(matches!(
            run("PartialFailure", false),
            Ok(Outcome::Unknown { .. })
        ));
    }

    #[test]
    fn a_repeated_message_wrapped_in_another_element_is_found() {
        let wrapped = parse(&format!(
            "<ServiceResponse RequestType=\"RepeatLastMessage\" OverallResult=\"Success\"><PrivateData>{RESPONSE}</PrivateData></ServiceResponse>"
        ))
        .unwrap();
        assert!(matches!(
            interpret_response(&wrapped, "1", None, false, true),
            Ok(Outcome::Approved(_))
        ));
    }

    #[tokio::test]
    async fn the_device_request_example_is_answered_and_its_receipt_kept() {
        let printed = Arc::new(Mutex::new(Printed::default()));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = {
            let printed = printed.clone();
            tokio::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                serve_device(stream, printed, "pili-pos/1.0.0".into()).await;
            })
        };
        let mut client = TcpStream::connect(addr).await.unwrap();
        write_frame(&mut client, DEVICE_REQUEST).await.unwrap();
        let reply = parse(&read_frame(&mut client).await.unwrap()).unwrap();
        assert_eq!(reply.name, "DeviceResponse");
        assert_eq!(reply.attr("OverallResult"), Some("Success"));
        assert_eq!(reply.attr("RequestID"), Some("1"));
        drop(client);
        server.await.unwrap();
        let p = lock(&printed);
        assert_eq!(
            p.customer.first().map(String::as_str),
            Some("CUSTOMER RECEIPT")
        );
        assert_eq!(p.customer.len(), 4);
        assert!(p.merchant.is_empty());
    }
}
