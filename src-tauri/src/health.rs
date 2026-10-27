//! The answer of the local healthcheck (`127.0.0.1:9999/health`), kept apart
//! from the listener so it can be tested.

/// The whole HTTP response. `outbox_pending` is the unsent `HubRise` outbox
/// count, or `None` when the database did not answer: a POS whose database is
/// broken is not "ok", so the monitor gets a 503 and `"status":"degraded"`
/// instead of a healthy answer with a made-up zero.
#[must_use]
pub fn http_response(uptime_secs: u64, outbox_pending: Option<i64>) -> String {
    let (status_line, body) = match outbox_pending {
        Some(pending) => (
            "200 OK",
            format!(r#"{{"status":"ok","uptime_secs":{uptime_secs},"outbox_pending":{pending}}}"#),
        ),
        None => (
            "503 Service Unavailable",
            format!(r#"{{"status":"degraded","uptime_secs":{uptime_secs}}}"#),
        ),
    };
    format!(
        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(response: &str) -> (&str, &str) {
        let (head, body) = response.split_once("\r\n\r\n").unwrap();
        (head, body)
    }

    #[test]
    fn a_working_database_answers_200_ok_with_the_count() {
        let response = http_response(42, Some(3));
        let (head, body) = split(&response);
        assert!(head.starts_with("HTTP/1.1 200 OK\r\n"));
        assert_eq!(
            body,
            r#"{"status":"ok","uptime_secs":42,"outbox_pending":3}"#
        );
        assert!(head.contains(&format!("Content-Length: {}", body.len())));
    }

    #[test]
    fn a_database_that_does_not_answer_is_503_degraded() {
        let response = http_response(7, None);
        let (head, body) = split(&response);
        assert!(head.starts_with("HTTP/1.1 503 Service Unavailable\r\n"));
        assert_eq!(body, r#"{"status":"degraded","uptime_secs":7}"#);
        assert!(!body.contains("outbox_pending"));
        assert!(head.contains(&format!("Content-Length: {}", body.len())));
    }
}
