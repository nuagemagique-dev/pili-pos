//! Runs a [`Session`] over TCP, with the clock the session does not have:
//! the simulator, or a serial device server on the LAN in front of the
//! scale. One conversation at a time: a weighing holds the link until it
//! ends.

use std::sync::Mutex as StdMutex;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, Notify};

use crate::frame::{CheckValue, Decoder, EOT, PriceDigits};
use crate::session::{CheckEvent, Failure, Output, Progress, Reading, Session, WeighRequest};

#[derive(Debug, Clone)]
pub struct DriverConfig {
    pub digits: PriceDigits,
    pub check_values: Vec<CheckValue>,
    /// The whole weighing, from the price to the reading.
    pub weigh_timeout: Duration,
    /// Silence before a request goes again.
    pub answer_timeout: Duration,
    /// Between two `EOT ENQ` (the spec's example waits 100 ms).
    pub poll_interval: Duration,
}

impl DriverConfig {
    pub fn new(
        digits: PriceDigits,
        check_values: Vec<CheckValue>,
        weigh_timeout: Duration,
    ) -> Self {
        Self {
            digits,
            check_values,
            weigh_timeout,
            answer_timeout: Duration::from_millis(1000),
            poll_interval: Duration::from_millis(150),
        }
    }
}

/// The end of a weighing, with the check's steps for the audit log (they
/// count even when the weighing failed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeighOutcome {
    pub result: Result<Reading, Failure>,
    pub checks: Vec<CheckEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeOutcome {
    pub result: Result<(), Failure>,
    pub checks: Vec<CheckEvent>,
}

pub struct ScaleDriver {
    config: DriverConfig,
    /// `host:port`.
    address: String,
    /// Held for a whole conversation; empty until connected or after a
    /// lost link.
    link: Mutex<Option<TcpStream>>,
    cancel: Notify,
    busy: StdMutex<bool>,
}

impl ScaleDriver {
    pub fn new(config: DriverConfig, address: String) -> Self {
        Self {
            config,
            address,
            link: Mutex::new(None),
            cancel: Notify::new(),
            busy: StdMutex::new(false),
        }
    }

    /// True while a weighing holds the link.
    pub fn is_busy(&self) -> bool {
        self.busy.lock().map(|b| *b).unwrap_or(false)
    }

    /// Ends the running weighing, if any, with [`Failure::Cancelled`].
    pub fn cancel(&self) {
        self.cancel.notify_waiters();
    }

    async fn connect(&self) -> Option<TcpStream> {
        let connect = TcpStream::connect(&self.address);
        let stream = tokio::time::timeout(self.config.answer_timeout, connect)
            .await
            .ok()?
            .ok()?;
        let _ = stream.set_nodelay(true);
        Some(stream)
    }

    /// Weigh once. `on_progress` is told where it stands.
    pub async fn weigh(
        &self,
        request: WeighRequest,
        on_progress: impl Fn(Progress) + Send,
    ) -> WeighOutcome {
        let session = Session::weighing(
            request,
            self.config.digits,
            self.config.check_values.clone(),
        );
        let (result, checks) = self
            .run(session, self.config.weigh_timeout, on_progress)
            .await;
        WeighOutcome {
            result: result.and_then(|r| r.ok_or(Failure::Unreachable)),
            checks,
        }
    }

    /// Whether the scale answers; runs the software check if it asks.
    pub async fn probe(&self) -> ProbeOutcome {
        let session = Session::probe(self.config.digits, self.config.check_values.clone());
        let limit = self.config.answer_timeout * 4;
        let (result, checks) = self.run(session, limit, |_| {}).await;
        ProbeOutcome {
            result: result.map(|_| ()),
            checks,
        }
    }

    async fn run(
        &self,
        mut session: Session,
        limit: Duration,
        on_progress: impl Fn(Progress) + Send,
    ) -> (Result<Option<Reading>, Failure>, Vec<CheckEvent>) {
        let mut slot = self.link.lock().await;
        if let Ok(mut busy) = self.busy.lock() {
            *busy = true;
        }
        let mut checks = Vec::new();
        let result = self
            .converse(&mut slot, &mut session, limit, &on_progress, &mut checks)
            .await;
        if let Ok(mut busy) = self.busy.lock() {
            *busy = false;
        }
        (result, checks)
    }

    async fn converse(
        &self,
        slot: &mut Option<TcpStream>,
        session: &mut Session,
        limit: Duration,
        on_progress: &(impl Fn(Progress) + Send),
        checks: &mut Vec<CheckEvent>,
    ) -> Result<Option<Reading>, Failure> {
        if slot.is_none() {
            *slot = self.connect().await;
        }
        let Some(link) = slot.as_mut() else {
            return Err(Failure::Unreachable);
        };
        // Registered before the first await that can be cancelled.
        let cancelled = self.cancel.notified();
        tokio::pin!(cancelled);
        cancelled.as_mut().enable();
        let deadline = tokio::time::Instant::now() + limit;
        let mut decoder = Decoder::default();
        let mut buffer = [0u8; 128];
        let mut outputs = session.start();
        loop {
            for output in std::mem::take(&mut outputs) {
                match output {
                    Output::Send(bytes) => {
                        if link.write_all(&bytes).await.is_err() {
                            *slot = None;
                            return Err(Failure::Unreachable);
                        }
                    }
                    Output::SendLater(bytes) => {
                        tokio::select! {
                            () = tokio::time::sleep(self.config.poll_interval) => {}
                            () = &mut cancelled => {
                                let _ = link.write_all(&[EOT]).await;
                                return Err(Failure::Cancelled);
                            }
                            () = tokio::time::sleep_until(deadline) => {
                                let _ = link.write_all(&[EOT]).await;
                                return Err(Failure::TimedOut);
                            }
                        }
                        if link.write_all(&bytes).await.is_err() {
                            *slot = None;
                            return Err(Failure::Unreachable);
                        }
                    }
                    Output::Progress(progress) => on_progress(progress),
                    Output::Check(event) => checks.push(event),
                    Output::Done(reading) => return Ok(Some(reading)),
                    Output::Connected => return Ok(None),
                    Output::Failed(failure) => {
                        if failure == Failure::Unreachable {
                            *slot = None;
                        }
                        return Err(failure);
                    }
                }
            }
            tokio::select! {
                () = &mut cancelled => {
                    let _ = link.write_all(&[EOT]).await;
                    return Err(Failure::Cancelled);
                }
                () = tokio::time::sleep_until(deadline) => {
                    let _ = link.write_all(&[EOT]).await;
                    return Err(Failure::TimedOut);
                }
                read = tokio::time::timeout(self.config.answer_timeout, link.read(&mut buffer)) => {
                    match read {
                        Err(_) => outputs = session.on_silence(),
                        Ok(Ok(0)) | Ok(Err(_)) => {
                            *slot = None;
                            return Err(Failure::Unreachable);
                        }
                        Ok(Ok(n)) => {
                            for message in decoder.push(&buffer[..n]) {
                                outputs.extend(session.on_message(message));
                            }
                        }
                    }
                }
            }
        }
    }
}
