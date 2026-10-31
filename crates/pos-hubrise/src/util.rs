//! Small helpers shared by the poller, the outbox and the inventory sync.

use std::sync::{Mutex, MutexGuard, PoisonError};

use rusqlite::Connection;

/// Lock the database connection, recovering from a poisoned mutex. A
/// panic elsewhere while the lock was held rolled its open transaction
/// back, so the connection stays usable; failing here would kill the
/// background task for good.
pub(crate) fn lock_db(db: &Mutex<Connection>) -> MutexGuard<'_, Connection> {
    db.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `value` as one URL path segment: every byte but an unreserved
/// character is percent-encoded, so an id from the wire cannot add
/// segments (`/`) or a query (`?`).
pub(crate) fn path_segment(value: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn a_poisoned_lock_is_recovered() {
        let db = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        let other = Arc::clone(&db);
        let _ = std::thread::spawn(move || {
            let _guard = other.lock().unwrap();
            panic!("poison the mutex");
        })
        .join();
        assert!(db.is_poisoned());
        let conn = lock_db(&db);
        conn.execute_batch("CREATE TABLE t (x)").unwrap();
    }

    #[test]
    fn path_segments_cannot_escape() {
        assert_eq!(path_segment("abc-1_2.3~"), "abc-1_2.3~");
        assert_eq!(path_segment("a/b?c#d e"), "a%2Fb%3Fc%23d%20e");
        assert_eq!(path_segment("../x"), "..%2Fx");
    }
}
