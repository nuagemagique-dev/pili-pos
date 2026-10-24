//! The upload client against the mock receiver (card X1-T4, decision X1D8).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::Duration;

use pos_export::upload::{MAX_UPLOAD_BYTES, UploadError, Uploader};
use pos_export_mock::Receiver;
use sha2::{Digest, Sha256};

const EXPORT_ID: &str = "20261008-z000042";

/// A random token, generated per test run (nothing secret is committed).
fn token() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("random bytes");
    format!("tok-{}", hex::encode(bytes))
}

fn client(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .expect("client")
}

fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("write");
    path
}

async fn put(
    uploader: &Uploader,
    path: &Path,
    name: &str,
    sha256: &str,
    token: &str,
) -> Result<(), UploadError> {
    uploader
        .put_file(
            EXPORT_ID,
            name,
            path,
            sha256,
            token,
            "application/octet-stream",
        )
        .await
}

#[tokio::test]
async fn uploads_a_file_and_the_receiver_stores_the_exact_bytes() {
    let token = token();
    let receiver = Receiver::start(&token);
    let uploader = Uploader::new(client(Duration::from_secs(20)), &receiver.url());
    let dir = tempfile::tempdir().unwrap();
    let body: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
    let file = write_file(dir.path(), "pos.db.age", &body);
    let name = format!("pos-{EXPORT_ID}.db.age");

    put(&uploader, &file, &name, &sha(&body), &token)
        .await
        .expect("upload");

    let got = receiver.received();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].path, format!("{EXPORT_ID}/{name}"));
    assert_eq!(got[0].bytes, body);
    assert_eq!(got[0].sha256, sha(&body));
    assert_eq!(got[0].content_type, "application/octet-stream");
    assert_eq!(receiver.file(&format!("{EXPORT_ID}/{name}")).unwrap(), body);
}

#[tokio::test]
async fn the_manifest_goes_out_as_json_and_a_trailing_slash_in_the_url_is_fine() {
    let token = token();
    let receiver = Receiver::start(&token);
    let uploader = Uploader::new(
        client(Duration::from_secs(20)),
        &format!("{}/", receiver.url()),
    );
    let dir = tempfile::tempdir().unwrap();
    let body = br#"{"format":"pili-export/1"}"#;
    let file = write_file(dir.path(), "m.json", body);
    let name = format!("manifest-{EXPORT_ID}.json");

    uploader
        .put_file(
            EXPORT_ID,
            &name,
            &file,
            &sha(body),
            &token,
            "application/json",
        )
        .await
        .expect("upload");

    let got = receiver.received();
    assert_eq!(got[0].path, format!("{EXPORT_ID}/{name}"));
    assert_eq!(got[0].content_type, "application/json");
}

#[tokio::test]
async fn a_wrong_token_is_unauthorized_and_nothing_is_stored() {
    let token = token();
    let receiver = Receiver::start(&token);
    let uploader = Uploader::new(client(Duration::from_secs(20)), &receiver.url());
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "a", b"data");

    let err = put(&uploader, &file, "a", &sha(b"data"), "not-the-token")
        .await
        .unwrap_err();
    assert_eq!(err, UploadError::Unauthorized(401));
    assert_eq!(err.to_string(), "unauthorized (HTTP 401)");
    assert!(receiver.received().is_empty());
}

#[tokio::test]
async fn a_receiver_that_refuses_everything_is_unauthorized() {
    let token = token();
    let receiver = Receiver::start(&token);
    receiver.set_unauthorized(true);
    let uploader = Uploader::new(client(Duration::from_secs(20)), &receiver.url());
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "a", b"data");

    assert_eq!(
        put(&uploader, &file, "a", &sha(b"data"), &token).await,
        Err(UploadError::Unauthorized(401))
    );
    receiver.set_unauthorized(false);
    assert_eq!(
        put(&uploader, &file, "a", &sha(b"data"), &token).await,
        Ok(())
    );
}

#[tokio::test]
async fn forbidden_is_unauthorized_too() {
    let token = token();
    let receiver = Receiver::start(&token);
    receiver.fail_next(1, 403);
    let uploader = Uploader::new(client(Duration::from_secs(20)), &receiver.url());
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "a", b"data");

    assert_eq!(
        put(&uploader, &file, "a", &sha(b"data"), &token).await,
        Err(UploadError::Unauthorized(403))
    );
}

#[tokio::test]
async fn a_server_error_is_http_then_the_next_try_succeeds() {
    let token = token();
    let receiver = Receiver::start(&token);
    receiver.fail_next(1, 503);
    let uploader = Uploader::new(client(Duration::from_secs(20)), &receiver.url());
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "a", b"data");

    let err = put(&uploader, &file, "a", &sha(b"data"), &token)
        .await
        .unwrap_err();
    assert_eq!(err, UploadError::Http(503));
    assert_eq!(err.to_string(), "HTTP 503");
    assert!(receiver.received().is_empty());

    put(&uploader, &file, "a", &sha(b"data"), &token)
        .await
        .expect("second try");
    assert_eq!(receiver.received().len(), 1);
    assert_eq!(receiver.put_count(), 2);
}

#[tokio::test]
async fn a_wrong_hash_is_refused_by_the_receiver_with_400() {
    let token = token();
    let receiver = Receiver::start(&token);
    let uploader = Uploader::new(client(Duration::from_secs(20)), &receiver.url());
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "a", b"data");

    assert_eq!(
        put(&uploader, &file, "a", &sha(b"other data"), &token).await,
        Err(UploadError::Http(400))
    );
    assert!(receiver.received().is_empty());
}

#[tokio::test]
async fn a_slow_answer_past_the_client_timeout_is_a_transport_error() {
    let token = token();
    let receiver = Receiver::start(&token);
    receiver.set_delay(Duration::from_secs(5));
    let uploader = Uploader::new(client(Duration::from_millis(300)), &receiver.url());
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "a", b"data");

    let started = std::time::Instant::now();
    let err = put(&uploader, &file, "a", &sha(b"data"), &token)
        .await
        .unwrap_err();
    assert!(started.elapsed() < Duration::from_secs(4), "{err}");
    match &err {
        UploadError::Transport(reason) => {
            assert!(
                !reason.contains("http://") && !reason.contains("127.0.0.1"),
                "the URL must not show: {reason}"
            );
        }
        other => panic!("expected Transport, got {other:?}"),
    }
    assert!(err.to_string().starts_with("connection failed: "));
}

#[tokio::test]
async fn a_closed_port_is_a_transport_error() {
    let token = token();
    let closed = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    };
    let uploader = Uploader::new(client(Duration::from_secs(5)), &format!("http://{closed}"));
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "a", b"data");

    let err = put(&uploader, &file, "a", &sha(b"data"), &token)
        .await
        .unwrap_err();
    assert!(matches!(err, UploadError::Transport(_)), "{err:?}");
    assert!(!err.to_string().contains(&closed.to_string()), "{err}");
}

#[tokio::test]
async fn a_stopped_receiver_is_a_transport_error() {
    let token = token();
    let mut receiver = Receiver::start(&token);
    let uploader = Uploader::new(client(Duration::from_secs(5)), &receiver.url());
    receiver.stop();
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "a", b"data");

    assert!(matches!(
        put(&uploader, &file, "a", &sha(b"data"), &token).await,
        Err(UploadError::Transport(_))
    ));
}

#[tokio::test]
async fn a_file_above_the_limit_is_too_large_and_never_read_or_sent() {
    let token = token();
    let receiver = Receiver::start(&token);
    let uploader = Uploader::new(client(Duration::from_secs(5)), &receiver.url());
    let dir = tempfile::tempdir().unwrap();
    // Sparse file: the length is set, no block is written.
    let path = dir.path().join("huge");
    let size = MAX_UPLOAD_BYTES + 1;
    std::fs::File::create(&path)
        .unwrap()
        .set_len(size as u64)
        .unwrap();

    let err = put(&uploader, &path, "huge", &"0".repeat(64), &token)
        .await
        .unwrap_err();
    assert_eq!(err, UploadError::TooLarge(size));
    assert_eq!(err.to_string(), format!("file too large: {size} bytes"));
    assert_eq!(receiver.put_count(), 0);
}

#[tokio::test]
async fn a_missing_file_is_an_io_error() {
    let token = token();
    let receiver = Receiver::start(&token);
    let uploader = Uploader::new(client(Duration::from_secs(5)), &receiver.url());
    let dir = tempfile::tempdir().unwrap();

    let err = put(
        &uploader,
        &dir.path().join("absent"),
        "absent",
        &"0".repeat(64),
        &token,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, UploadError::Io(_)), "{err:?}");
    assert!(err.to_string().starts_with("file unreadable: "));
    assert_eq!(receiver.put_count(), 0);
}

#[tokio::test]
async fn an_invalid_token_value_is_refused_before_any_request() {
    let receiver = Receiver::start("whatever");
    let uploader = Uploader::new(client(Duration::from_secs(5)), &receiver.url());
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "a", b"data");

    let bad = "line one\nline two";
    let err = put(&uploader, &file, "a", &sha(b"data"), bad)
        .await
        .unwrap_err();
    assert!(matches!(err, UploadError::Transport(_)), "{err:?}");
    assert!(!err.to_string().contains("line one"));
    assert_eq!(receiver.put_count(), 0);
}

#[tokio::test]
async fn no_error_text_contains_the_token() {
    let token = token();
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "a", b"data");
    let mut errors: Vec<UploadError> = Vec::new();

    // Unauthorized and Http, from a real receiver (the token is the right one
    // for the 503, a wrong one for the 401).
    let receiver = Receiver::start(&token);
    let uploader = Uploader::new(client(Duration::from_secs(5)), &receiver.url());
    receiver.fail_next(1, 503);
    errors.push(
        put(&uploader, &file, "a", &sha(b"data"), &token)
            .await
            .unwrap_err(),
    );
    // A wrong token that embeds the real one must still not echo back.
    let wrong = format!("{token}-extra");
    errors.push(
        put(&uploader, &file, "a", &sha(b"data"), &wrong)
            .await
            .unwrap_err(),
    );

    // Transport: closed port, with the token in the path-free request.
    let closed = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    };
    let dead = Uploader::new(
        client(Duration::from_secs(5)),
        &format!("http://{closed}/p?k=secret"),
    );
    errors.push(
        put(&dead, &file, "a", &sha(b"data"), &token)
            .await
            .unwrap_err(),
    );

    // Io and TooLarge.
    errors.push(
        put(
            &uploader,
            &dir.path().join("absent"),
            "a",
            &sha(b"data"),
            &token,
        )
        .await
        .unwrap_err(),
    );
    errors.push(UploadError::TooLarge(MAX_UPLOAD_BYTES + 1));

    assert_eq!(errors.len(), 5);
    for err in &errors {
        for text in [err.to_string(), format!("{err:?}")] {
            assert!(!text.contains(&token), "token leaked: {text}");
            assert!(!text.contains("Bearer"), "header leaked: {text}");
            assert!(!text.contains("secret"), "query leaked: {text}");
        }
    }
}

/// What the client puts on the wire: request line and headers, read by a bare
/// TCP server.
#[tokio::test]
async fn the_request_carries_the_contract_headers() {
    let token = token();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut seen = Vec::new();
        let mut buf = [0u8; 4096];
        // Read until the headers are complete and the 4-byte body arrived.
        loop {
            let n = stream.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            seen.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&seen).to_string();
            if text
                .find("\r\n\r\n")
                .is_some_and(|end| seen.len() >= end + 4 + 4)
            {
                break;
            }
        }
        stream
            .write_all(b"HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .unwrap();
        String::from_utf8_lossy(&seen).to_string()
    });

    let uploader = Uploader::new(
        client(Duration::from_secs(10)),
        &format!("http://{addr}/base"),
    );
    let dir = tempfile::tempdir().unwrap();
    let file = write_file(dir.path(), "a", b"data");
    uploader
        .put_file(
            EXPORT_ID,
            "stats.sqlite",
            &file,
            &sha(b"data"),
            &token,
            "application/octet-stream",
        )
        .await
        .expect("upload");

    let request = server.join().unwrap();
    let lower = request.to_ascii_lowercase();
    assert!(
        request.starts_with(&format!("PUT /base/{EXPORT_ID}/stats.sqlite HTTP/1.1\r\n")),
        "{request}"
    );
    assert!(lower.contains(&format!("authorization: bearer {token}\r\n")));
    assert!(lower.contains("content-type: application/octet-stream\r\n"));
    assert!(lower.contains(&format!("x-content-sha256: {}\r\n", sha(b"data"))));
    assert!(lower.contains("user-agent: pili-export/1\r\n"));
    assert!(lower.contains("content-length: 4\r\n"));
    assert!(request.ends_with("data"));
}
