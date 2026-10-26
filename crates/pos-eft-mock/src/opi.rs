//! A CCV terminal speaking OPI on the LAN: channel 0 (the till connects,
//! 4-byte big-endian length then XML) and, for an approved payment, a
//! connection back to the till's channel 1 to print the customer receipt
//! before the answer.

use std::fmt::Write as _;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

use crate::state::{
    Begin, Behaviour, Payment, Phase, Protocol, Shared, amount_text, auth_code, circuit,
    customer_receipt, lock, wait_done,
};

/// What the mock needs of an incoming message.
#[derive(Default, Debug)]
struct Doc {
    name: String,
    attrs: Vec<(String, String)>,
    total_amount: Option<String>,
}

impl Doc {
    fn attr(&self, key: &str) -> &str {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map_or("", |(_, v)| v.as_str())
    }
}

fn attrs_of(e: &BytesStart<'_>) -> Vec<(String, String)> {
    e.attributes()
        .flatten()
        .map(|a| (a.key.local_name().as_ref().to_string(), a.value.to_string()))
        .collect()
}

fn read_doc(xml: &str) -> Option<Doc> {
    let mut reader = Reader::from_str(xml);
    let mut doc = Doc::default();
    let mut in_amount = false;
    loop {
        match reader.read_event().ok()? {
            Event::Start(e) | Event::Empty(e) => {
                let name = e.local_name().as_ref().to_string();
                if doc.name.is_empty() {
                    doc.name = name;
                    doc.attrs = attrs_of(&e);
                } else if name == "TotalAmount" {
                    in_amount = true;
                }
            }
            Event::Text(t) if in_amount => {
                doc.total_amount = Some(
                    t.xml_content(quick_xml::XmlVersion::Implicit1_0)
                        .trim()
                        .to_string(),
                );
                in_amount = false;
            }
            Event::End(_) => in_amount = false,
            Event::Eof => break,
            _ => {}
        }
    }
    (!doc.name.is_empty()).then_some(doc)
}

async fn read_frame(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len).await?;
    let len = usize::try_from(u32::from_be_bytes(len))
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "frame too long"))?;
    if len > 1 << 20 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "frame too long",
        ));
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).await?;
    Ok(String::from_utf8_lossy(&body).into_owned())
}

async fn write_frame(stream: &mut TcpStream, xml: &str) -> std::io::Result<()> {
    let len = u32::try_from(xml.len())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "frame too long"))?;
    let mut frame = len.to_be_bytes().to_vec();
    frame.extend_from_slice(xml.as_bytes());
    stream.write_all(&frame).await?;
    stream.flush().await
}

fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn bare_response(doc: &Doc, root: &str, result: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><{root} RequestType=\"{}\" ApplicationSender=\"{}\" \
         WorkstationID=\"{}\" RequestID=\"{}\" OverallResult=\"{result}\"/>",
        esc(doc.attr("RequestType")),
        esc(doc.attr("ApplicationSender")),
        esc(doc.attr("WorkstationID")),
        esc(doc.attr("RequestID")),
    )
}

/// The `CardServiceResponse` of a payment that reached `phase`.
fn card_response(p: &Payment, phase: Phase) -> String {
    let head = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><CardServiceResponse RequestType=\"CardPayment\" \
         ApplicationSender=\"{}\" WorkstationID=\"{}\" RequestID=\"{}\"",
        esc(&p.sender),
        esc(&p.workstation),
        esc(&p.reference),
    );
    match phase {
        Phase::Approved => format!(
            "{head} OverallResult=\"Success\"><Terminal TerminalID=\"00000001\" TerminalBatch=\"1\" STAN=\"{:06}\"/>\
             <Tender><TotalAmount Currency=\"EUR\" PaymentAmount=\"{amount}\">{amount}</TotalAmount>\
             <Authorisation AcquirerID=\"1\" CardPAN=\"XXXXXXXXXXXX0119\" PANPrint=\"XXXXXXXXXXXX0119\" \
             TimeStamp=\"{}\" ActionCode=\"\" ApprovalCode=\"{:06}\" CardCircuit=\"{}\" ResponseCode=\"0\" \
             ResponseComment=\"Approved\"/></Tender><PrivateData></PrivateData></CardServiceResponse>",
            p.id,
            chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
            auth_code(p),
            circuit(&p.brand),
            amount = amount_text(p.approved_cents),
        ),
        Phase::Declined => format!(
            "{head} OverallResult=\"Failure\" ElmeErrorCode=\"1001\" ElmeErrorText=\"Card refused\"></CardServiceResponse>"
        ),
        Phase::Cancelled => format!("{head} OverallResult=\"Aborted\"></CardServiceResponse>"),
        Phase::InProgress | Phase::Stuck => {
            format!("{head} OverallResult=\"Busy\"></CardServiceResponse>")
        }
    }
}

/// Send the customer receipt to the till's channel 1 and wait for its
/// `DeviceResponse`. A till that does not listen is not an error here.
async fn print_receipt(peer: SocketAddr, device_port: u16, p: &Payment) {
    if device_port == 0 {
        return;
    }
    let target = SocketAddr::new(peer.ip(), device_port);
    let Ok(Ok(mut stream)) = timeout(Duration::from_secs(2), TcpStream::connect(target)).await
    else {
        tracing::warn!(%target, "pos-eft-mock: the till does not listen on channel 1");
        return;
    };
    let lines: String = customer_receipt(p)
        .iter()
        .fold(String::new(), |mut acc, l| {
            let _ = write!(acc, "<TextLine>{}</TextLine>", esc(l));
            acc
        });
    let request = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><DeviceRequest RequestType=\"Output\" \
         ApplicationSender=\"{}\" WorkstationID=\"{}\" TerminalID=\"00000001\" RequestID=\"{}\" SequenceID=\"1\">\
         <Output OutDeviceTarget=\"PrinterReceipt\" Immediate=\"true\">{lines}\
         <TextLine PaperCut=\"true\"></TextLine></Output></DeviceRequest>",
        esc(&p.sender),
        esc(&p.workstation),
        esc(&p.reference),
    );
    if write_frame(&mut stream, &request).await.is_ok() {
        let _ = timeout(Duration::from_secs(2), read_frame(&mut stream)).await;
    }
}

/// Accept connections on `listener` until the task is dropped.
pub(crate) async fn serve(listener: TcpListener, shared: Shared) {
    while let Ok((stream, peer)) = listener.accept().await {
        tokio::spawn(connection(stream, peer, Arc::clone(&shared)));
    }
}

async fn connection(mut stream: TcpStream, peer: SocketAddr, shared: Shared) {
    while let Ok(xml) = read_frame(&mut stream).await {
        let _log = lock(&shared).log(
            Protocol::Opi,
            "TCP",
            "opi:channel0",
            serde_json::Value::String(xml.clone()),
        );
        let Some(doc) = read_doc(&xml) else {
            let _ = write_frame(
                &mut stream,
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><ServiceResponse OverallResult=\"ParsingError\"/>",
            )
            .await;
            continue;
        };
        let reply = match (doc.name.as_str(), doc.attr("RequestType")) {
            ("CardServiceRequest", "CardPayment") => match payment(&shared, &doc, peer).await {
                Some(r) => r,
                // The answer is lost: the connection is cut.
                None => return,
            },
            ("CardServiceRequest", "AbortRequest") => abort(&shared, &doc),
            ("CardServiceRequest", "RepeatLastMessage") => repeat(&shared, &doc),
            ("ServiceRequest", _) => bare_response(&doc, "ServiceResponse", "Success"),
            _ => bare_response(&doc, "ServiceResponse", "FormatError"),
        };
        if write_frame(&mut stream, &reply).await.is_err() {
            return;
        }
    }
}

async fn payment(shared: &Shared, doc: &Doc, peer: SocketAddr) -> Option<String> {
    let id = doc.attr("RequestID");
    if id.is_empty() || id.len() > 9 || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Some(bare_response(doc, "CardServiceResponse", "ValidationError"));
    }
    let cents = doc.total_amount.as_deref().and_then(|t| {
        let (whole, frac) = t.split_once('.').unwrap_or((t, "00"));
        let whole: i64 = whole.parse().ok()?;
        let frac: i64 = format!("{frac:0<2}").get(..2)?.parse().ok()?;
        Some(whole * 100 + frac)
    });
    let Some(cents) = cents.filter(|c| *c > 0) else {
        return Some(bare_response(
            doc,
            "CardServiceResponse",
            "MissingMandatoryData",
        ));
    };
    let device_port;
    let begun = {
        let mut inner = lock(shared);
        device_port = inner.device_port;
        let begun = inner.begin(
            Protocol::Opi,
            id,
            "opi",
            cents,
            doc.attr("ApplicationSender"),
            doc.attr("WorkstationID"),
        );
        if matches!(begun, Begin::Created(_) | Begin::Existing(_)) {
            inner.opi_last = Some(id.to_string());
        }
        begun
    };
    let index = match begun {
        Begin::Busy => return Some(bare_response(doc, "CardServiceResponse", "Busy")),
        Begin::Existing(i) | Begin::Created(i) => i,
    };
    let (payment, phase) = wait_done(shared, index).await;
    if matches!(payment.behaviour, Behaviour::Timeout | Behaviour::Unknown)
        && phase != Phase::Cancelled
    {
        return None;
    }
    if phase == Phase::Approved {
        print_receipt(peer, device_port, &payment).await;
    }
    Some(card_response(&payment, phase))
}

fn abort(shared: &Shared, doc: &Doc) -> String {
    let mut inner = lock(shared);
    let target = inner.payments.iter().position(|p| {
        p.protocol == Protocol::Opi && p.phase(std::time::Instant::now()) == Phase::InProgress
    });
    let stopped = target.is_some_and(|i| inner.abort(i));
    bare_response(
        &Doc {
            name: doc.name.clone(),
            attrs: doc.attrs.clone(),
            total_amount: None,
        },
        "CardServiceResponse",
        if stopped { "Aborted" } else { "Failure" },
    )
}

fn repeat(shared: &Shared, doc: &Doc) -> String {
    let inner = lock(shared);
    let last = inner
        .opi_last
        .as_deref()
        .and_then(|r| inner.find(Protocol::Opi, r))
        .map(|i| inner.payments[i].clone());
    match last {
        Some(p) => {
            let phase = p.phase(std::time::Instant::now());
            card_response(&p, phase)
        }
        None => bare_response(doc, "ServiceResponse", "Failure"),
    }
}
