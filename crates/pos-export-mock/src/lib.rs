//! Test receiver of the nightly export (phase X1, decision X1D8): the HTTPS
//! PUT endpoint the POS uploads the encrypted backup, the statistics file and
//! the manifest to. It speaks plain HTTP on a local port (the engine accepts
//! plain HTTP only towards `127.0.0.1`, `localhost`, `[::1]` and `10.0.2.2`).
//!
//! A test tool outside the certified build: never a normal dependency of a
//! shipped crate (dev-dependency only), see `README.md`.
//!
//! The server runs on its own thread with its own tokio runtime, so a test can
//! start a [`Receiver`] from synchronous or asynchronous code, and a slow
//! answer ([`Receiver::set_delay`]) never blocks the test's own runtime.
//!
//! ```no_run
//! let receiver = pos_export_mock::Receiver::start("secret-token");
//! let url = receiver.url(); // http://127.0.0.1:<port>
//! // ... PUT <url>/20261008-z000042/manifest-20261008-z000042.json ...
//! assert_eq!(receiver.put_count(), 0);
//! ```

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};

/// Largest body the mock accepts (the client refuses 512 MiB, X1D8).
const MAX_BODY_BYTES: usize = 1 << 30;

/// One stored upload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Received {
    /// Request path without the leading slash: `<export_id>/<file name>`.
    pub path: String,
    /// The request body.
    pub bytes: Vec<u8>,
    /// Lowercase hex SHA-256 of the body.
    pub sha256: String,
    /// The `Content-Type` header (empty when absent).
    pub content_type: String,
}

/// Called for every PUT with the request path, the body size and the status
/// answered. The binary prints one line from it.
type Observer = Arc<dyn Fn(&str, usize, u16) + Send + Sync>;

struct State_ {
    token: String,
    dir: Option<PathBuf>,
    received: Vec<Received>,
    /// PUTs still to be refused with `fail_status`.
    fail_next: usize,
    fail_status: u16,
    delay: Duration,
    unauthorized: bool,
    put_count: usize,
    observer: Option<Observer>,
}

type Shared = Arc<Mutex<State_>>;

fn lock(state: &Shared) -> MutexGuard<'_, State_> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A running mock receiver. Stops when dropped or on [`Receiver::stop`].
pub struct Receiver {
    addr: SocketAddr,
    state: Shared,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl std::fmt::Debug for Receiver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Receiver")
            .field("addr", &self.addr)
            .finish_non_exhaustive()
    }
}

impl Receiver {
    /// Bind `127.0.0.1:0` and serve; uploads are kept in memory only.
    ///
    /// # Panics
    /// When the local port cannot be bound or the server thread cannot start.
    pub fn start(token: &str) -> Receiver {
        Receiver::start_on("127.0.0.1:0".parse().expect("static address"), token, None)
            .expect("pos-export-mock: cannot start")
    }

    /// Bind `addr` and serve. With `dir`, every accepted body is also written
    /// to `<dir>/<path>` (parent directories are created).
    pub fn start_on(
        addr: SocketAddr,
        token: &str,
        dir: Option<PathBuf>,
    ) -> std::io::Result<Receiver> {
        let listener = std::net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        let state: Shared = Arc::new(Mutex::new(State_ {
            token: token.to_string(),
            dir,
            received: Vec::new(),
            fail_next: 0,
            fail_status: 500,
            delay: Duration::ZERO,
            unauthorized: false,
            put_count: 0,
            observer: None,
        }));
        let (shutdown, stop) = tokio::sync::oneshot::channel::<()>();
        let app = router(Arc::clone(&state));
        let thread = std::thread::Builder::new()
            .name("pos-export-mock".to_string())
            .spawn(move || {
                let Ok(rt) = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                else {
                    return;
                };
                rt.block_on(async move {
                    let Ok(listener) = tokio::net::TcpListener::from_std(listener) else {
                        return;
                    };
                    tokio::select! {
                        _ = axum::serve(listener, app) => {}
                        _ = stop => {}
                    }
                });
                // Drop the connection tasks (a delayed answer included) at once.
                rt.shutdown_background();
            })?;
        Ok(Receiver {
            addr,
            state,
            shutdown: Some(shutdown),
            thread: Some(thread),
        })
    }

    /// Base URL without trailing slash: `http://127.0.0.1:<port>`.
    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// The address the receiver listens on.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Stop serving and release the port. Idempotent; also done on drop.
    pub fn stop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }

    /// Answer `status` to the next `n` PUTs that pass the token check (they
    /// are counted but not stored). Replaces a previous setting.
    pub fn fail_next(&self, n: usize, status: u16) {
        let mut state = lock(&self.state);
        state.fail_next = n;
        state.fail_status = status;
    }

    /// Sleep this long before answering every PUT (zero: no delay).
    pub fn set_delay(&self, delay: Duration) {
        lock(&self.state).delay = delay;
    }

    /// Answer 401 to every request, whatever its token, until
    /// [`Receiver::accept_tokens`].
    pub fn reject_all(&self) {
        lock(&self.state).unauthorized = true;
    }

    /// Go back to checking the token (undo [`Receiver::reject_all`]).
    pub fn accept_tokens(&self) {
        lock(&self.state).unauthorized = false;
    }

    /// Stored uploads, in arrival order.
    pub fn received(&self) -> Vec<Received> {
        lock(&self.state).received.clone()
    }

    /// The body of the latest upload to `path`, if any.
    pub fn file(&self, path: &str) -> Option<Vec<u8>> {
        lock(&self.state)
            .received
            .iter()
            .rev()
            .find(|r| r.path == path)
            .map(|r| r.bytes.clone())
    }

    /// Every PUT request seen, whatever the answer.
    pub fn put_count(&self) -> usize {
        lock(&self.state).put_count
    }

    /// Call `observer(path, size, status)` for every PUT, after the answer is
    /// decided. Replaces a previous observer.
    pub fn set_observer(&self, observer: impl Fn(&str, usize, u16) + Send + Sync + 'static) {
        lock(&self.state).observer = Some(Arc::new(observer));
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        self.stop();
    }
}

fn router(state: Shared) -> Router {
    Router::new()
        .fallback(handle)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}

/// The request path without its leading slash, or `None` when it is not an
/// acceptable relative path: empty, an empty segment, `.`, `..`, a backslash,
/// a percent escape or a control character.
fn clean_path(uri: &Uri) -> Option<String> {
    let path = uri.path().strip_prefix('/')?;
    if path.is_empty() {
        return None;
    }
    let bad = path.split('/').any(|segment| {
        segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains(['\\', '%'])
            || segment.chars().any(char::is_control)
    });
    (!bad).then(|| path.to_string())
}

fn status(code: u16) -> StatusCode {
    StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

/// True when the `Authorization` header is exactly `Bearer <token>`.
fn authorized(headers: &HeaderMap, token: &str) -> bool {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|presented| presented == token)
}

async fn handle(
    State(state): State<Shared>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let path = uri.path().trim_start_matches('/').to_string();
    if method != Method::PUT {
        let unauthorized = lock(&state).unauthorized;
        return if unauthorized {
            StatusCode::UNAUTHORIZED
        } else {
            StatusCode::METHOD_NOT_ALLOWED
        }
        .into_response();
    }

    let delay = {
        let mut guard = lock(&state);
        guard.put_count += 1;
        guard.delay
    };
    if !delay.is_zero() {
        tokio::time::sleep(delay).await;
    }

    let (code, observer) = {
        let mut guard = lock(&state);
        let code = decide(&mut guard, &uri, &headers, &body);
        (code, guard.observer.clone())
    };
    if let Some(observer) = observer {
        observer(&path, body.len(), code.as_u16());
    }
    code.into_response()
}

/// The status of one PUT; stores the upload when it is accepted.
fn decide(state: &mut State_, uri: &Uri, headers: &HeaderMap, body: &Bytes) -> StatusCode {
    if state.unauthorized || !authorized(headers, &state.token) {
        return StatusCode::UNAUTHORIZED;
    }
    if state.fail_next > 0 {
        state.fail_next -= 1;
        return status(state.fail_status);
    }
    let Some(path) = clean_path(uri) else {
        return StatusCode::BAD_REQUEST;
    };
    let sha256 = hex::encode(Sha256::digest(body));
    let declared = headers
        .get("x-content-sha256")
        .and_then(|v| v.to_str().ok());
    if declared != Some(sha256.as_str()) {
        return StatusCode::BAD_REQUEST;
    }
    if let Some(dir) = &state.dir {
        let target = dir.join(&path);
        let written = target
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&target, body));
        if written.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    state.received.push(Received {
        path,
        bytes: body.to_vec(),
        sha256,
        content_type,
    });
    StatusCode::CREATED
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Instant;

    const TOKEN: &str = "test-token-0123";

    fn sha(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    /// Raw HTTP/1.1 request; returns (status, whole response text).
    fn request(
        receiver: &Receiver,
        method: &str,
        path: &str,
        headers: &[(&str, String)],
        body: &[u8],
    ) -> (u16, String) {
        let mut stream = TcpStream::connect(receiver.addr()).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut head =
            format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
        for (name, value) in headers {
            let _ = write!(head, "{name}: {value}\r\n");
        }
        let _ = write!(head, "Content-Length: {}\r\n\r\n", body.len());
        stream.write_all(head.as_bytes()).unwrap();
        stream.write_all(body).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let code = response
            .split_whitespace()
            .nth(1)
            .and_then(|c| c.parse().ok())
            .unwrap_or(0);
        (code, response)
    }

    fn put(receiver: &Receiver, path: &str, body: &[u8], token: Option<&str>) -> u16 {
        let mut headers = vec![
            ("Content-Type", "application/octet-stream".to_string()),
            ("X-Content-SHA256", sha(body)),
        ];
        if let Some(token) = token {
            headers.push(("Authorization", format!("Bearer {token}")));
        }
        request(receiver, "PUT", path, &headers, body).0
    }

    #[test]
    fn stores_a_valid_put_and_answers_201() {
        let receiver = Receiver::start(TOKEN);
        assert!(receiver.url().starts_with("http://127.0.0.1:"));
        assert_eq!(put(&receiver, "/e1/a.bin", b"hello", Some(TOKEN)), 201);
        let got = receiver.received();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].path, "e1/a.bin");
        assert_eq!(got[0].bytes, b"hello");
        assert_eq!(got[0].sha256, sha(b"hello"));
        assert_eq!(got[0].content_type, "application/octet-stream");
        assert_eq!(receiver.file("e1/a.bin").unwrap(), b"hello");
        assert_eq!(receiver.file("e1/other"), None);
        assert_eq!(receiver.put_count(), 1);
    }

    #[test]
    fn same_path_twice_keeps_both_and_file_returns_the_latest() {
        let receiver = Receiver::start(TOKEN);
        assert_eq!(put(&receiver, "/e1/a", b"one", Some(TOKEN)), 201);
        assert_eq!(put(&receiver, "/e1/a", b"two", Some(TOKEN)), 201);
        assert_eq!(receiver.received().len(), 2);
        assert_eq!(receiver.file("e1/a").unwrap(), b"two");
    }

    #[test]
    fn missing_or_wrong_token_is_401_and_nothing_is_stored() {
        let receiver = Receiver::start(TOKEN);
        assert_eq!(put(&receiver, "/e1/a", b"x", None), 401);
        assert_eq!(put(&receiver, "/e1/a", b"x", Some("wrong")), 401);
        let (code, _) = request(
            &receiver,
            "PUT",
            "/e1/a",
            &[
                ("Authorization", TOKEN.to_string()),
                ("X-Content-SHA256", sha(b"x")),
            ],
            b"x",
        );
        assert_eq!(code, 401, "scheme Bearer is required");
        assert_eq!(receiver.received().len(), 0);
        assert_eq!(receiver.put_count(), 3);
    }

    #[test]
    fn hash_mismatch_or_missing_is_400() {
        let receiver = Receiver::start(TOKEN);
        let auth = ("Authorization", format!("Bearer {TOKEN}"));
        let wrong = ("X-Content-SHA256", sha(b"other"));
        assert_eq!(
            request(&receiver, "PUT", "/e1/a", &[auth.clone(), wrong], b"x").0,
            400
        );
        let upper = ("X-Content-SHA256", sha(b"x").to_uppercase());
        assert_eq!(
            request(&receiver, "PUT", "/e1/a", &[auth.clone(), upper], b"x").0,
            400
        );
        assert_eq!(request(&receiver, "PUT", "/e1/a", &[auth], b"x").0, 400);
        assert_eq!(receiver.received().len(), 0);
    }

    #[test]
    fn bad_paths_are_400() {
        let receiver = Receiver::start(TOKEN);
        for path in [
            "/",
            "/e1/../x",
            "/../x",
            "/e1//a",
            "/e1/",
            "/e1/./a",
            "/e1/%2e%2e/a",
            "/e1/a%2Fb",
        ] {
            assert_eq!(put(&receiver, path, b"x", Some(TOKEN)), 400, "{path}");
        }
        assert_eq!(receiver.received().len(), 0);
    }

    #[test]
    fn other_methods_are_405() {
        let receiver = Receiver::start(TOKEN);
        let auth = vec![("Authorization", format!("Bearer {TOKEN}"))];
        for method in ["GET", "POST", "DELETE", "HEAD"] {
            assert_eq!(
                request(&receiver, method, "/e1/a", &auth, b"").0,
                405,
                "{method}"
            );
        }
        assert_eq!(receiver.put_count(), 0);
    }

    #[test]
    fn fail_next_answers_the_status_n_times_then_recovers() {
        let receiver = Receiver::start(TOKEN);
        receiver.fail_next(2, 503);
        assert_eq!(put(&receiver, "/e1/a", b"x", Some(TOKEN)), 503);
        assert_eq!(put(&receiver, "/e1/a", b"x", Some(TOKEN)), 503);
        assert_eq!(receiver.received().len(), 0);
        assert_eq!(put(&receiver, "/e1/a", b"x", Some(TOKEN)), 201);
        assert_eq!(receiver.received().len(), 1);
        assert_eq!(receiver.put_count(), 3);
        // A refused token does not consume a pending failure.
        receiver.fail_next(1, 500);
        assert_eq!(put(&receiver, "/e1/a", b"x", Some("wrong")), 401);
        assert_eq!(put(&receiver, "/e1/a", b"x", Some(TOKEN)), 500);
    }

    #[test]
    fn set_delay_postpones_the_answer() {
        let receiver = Receiver::start(TOKEN);
        receiver.set_delay(Duration::from_millis(400));
        let started = Instant::now();
        assert_eq!(put(&receiver, "/e1/a", b"x", Some(TOKEN)), 201);
        assert!(started.elapsed() >= Duration::from_millis(400));
        receiver.set_delay(Duration::ZERO);
        let started = Instant::now();
        assert_eq!(put(&receiver, "/e1/b", b"x", Some(TOKEN)), 201);
        assert!(started.elapsed() < Duration::from_millis(300));
    }

    #[test]
    fn set_unauthorized_answers_401_to_everything_until_cleared() {
        let receiver = Receiver::start(TOKEN);
        receiver.reject_all();
        assert_eq!(put(&receiver, "/e1/a", b"x", Some(TOKEN)), 401);
        let auth = vec![("Authorization", format!("Bearer {TOKEN}"))];
        assert_eq!(request(&receiver, "GET", "/e1/a", &auth, b"").0, 401);
        assert_eq!(receiver.received().len(), 0);
        receiver.accept_tokens();
        assert_eq!(put(&receiver, "/e1/a", b"x", Some(TOKEN)), 201);
    }

    #[test]
    fn dir_receives_the_bodies() {
        let dir = std::env::temp_dir().join(format!(
            "pos-export-mock-{}-{}",
            std::process::id(),
            &sha(TOKEN.as_bytes())[..8]
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let receiver =
            Receiver::start_on("127.0.0.1:0".parse().unwrap(), TOKEN, Some(dir.clone())).unwrap();
        assert_eq!(put(&receiver, "/e1/a.bin", b"on disk", Some(TOKEN)), 201);
        assert_eq!(std::fs::read(dir.join("e1/a.bin")).unwrap(), b"on disk");
        // A refused body is not written.
        assert_eq!(put(&receiver, "/e2/a.bin", b"no", Some("wrong")), 401);
        assert!(!dir.join("e2").exists());
        drop(receiver);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn observer_sees_every_put_with_its_status() {
        let receiver = Receiver::start(TOKEN);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);
        receiver.set_observer(move |path, size, code| {
            sink.lock().unwrap().push((path.to_string(), size, code));
        });
        assert_eq!(put(&receiver, "/e1/a", b"abc", Some(TOKEN)), 201);
        assert_eq!(put(&receiver, "/e1/a", b"abc", None), 401);
        let seen = seen.lock().unwrap().clone();
        assert_eq!(
            seen,
            vec![("e1/a".to_string(), 3, 201), ("e1/a".to_string(), 3, 401)]
        );
    }

    #[test]
    fn stop_releases_the_port_and_is_idempotent() {
        let mut receiver = Receiver::start(TOKEN);
        let addr = receiver.addr();
        assert!(TcpStream::connect(addr).is_ok());
        receiver.stop();
        receiver.stop();
        assert!(TcpStream::connect(addr).is_err());
    }

    #[test]
    fn stop_does_not_wait_for_a_delayed_answer() {
        let mut receiver = Receiver::start(TOKEN);
        receiver.set_delay(Duration::from_secs(30));
        let addr = receiver.addr();
        let client = std::thread::spawn(move || {
            let mut stream = TcpStream::connect(addr).unwrap();
            let head = format!(
                "PUT /e1/a HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: 1\r\n\r\nx"
            );
            let _ = stream.write_all(head.as_bytes());
            let mut sink = String::new();
            let _ = stream.read_to_string(&mut sink);
        });
        std::thread::sleep(Duration::from_millis(300));
        let started = Instant::now();
        receiver.stop();
        assert!(started.elapsed() < Duration::from_secs(5));
        client.join().unwrap();
    }

    #[test]
    fn large_bodies_above_the_axum_default_are_accepted() {
        let receiver = Receiver::start(TOKEN);
        let body = vec![7u8; 5 * 1024 * 1024];
        assert_eq!(put(&receiver, "/e1/big", &body, Some(TOKEN)), 201);
        assert_eq!(receiver.file("e1/big").unwrap().len(), body.len());
    }
}
