//! SQLite-backed singleton store for the `HubRise` connection row.

use rusqlite::{Connection, OptionalExtension, params};

use crate::errors::Result;

#[derive(Clone)]
pub struct StoredConnection {
    pub access_token: String,
    pub account_id: String,
    pub account_name: String,
    pub location_id: String,
    pub location_name: String,
    pub catalog_id: String,
    pub catalog_name: Option<String>,
    pub customer_list_id: Option<String>,
    pub customer_list_name: Option<String>,
    pub scope: String,
    pub base_url: String,
    pub last_event_cursor: Option<String>,
}

/// What a secret prints as in a `Debug` output.
const REDACTED: &str = "<redacted>";

impl std::fmt::Debug for StoredConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredConnection")
            .field("access_token", &REDACTED)
            .field("account_id", &self.account_id)
            .field("account_name", &self.account_name)
            .field("location_id", &self.location_id)
            .field("location_name", &self.location_name)
            .field("catalog_id", &self.catalog_id)
            .field("catalog_name", &self.catalog_name)
            .field("customer_list_id", &self.customer_list_id)
            .field("customer_list_name", &self.customer_list_name)
            .field("scope", &self.scope)
            .field("base_url", &self.base_url)
            .field("last_event_cursor", &self.last_event_cursor)
            .finish()
    }
}

/// Load the currently persisted `HubRise` connection, if any.
pub fn load(conn: &Connection) -> Result<Option<StoredConnection>> {
    let row = conn
        .query_row(
            "SELECT access_token, account_id, account_name, location_id, location_name,
                    catalog_id, catalog_name, customer_list_id, customer_list_name,
                    scope, base_url, last_event_cursor
             FROM hubrise_connection WHERE id = 1",
            [],
            |r| {
                Ok(StoredConnection {
                    access_token: r.get(0)?,
                    account_id: r.get(1)?,
                    account_name: r.get(2)?,
                    location_id: r.get(3)?,
                    location_name: r.get(4)?,
                    catalog_id: r.get(5)?,
                    catalog_name: r.get(6)?,
                    customer_list_id: r.get(7)?,
                    customer_list_name: r.get(8)?,
                    scope: r.get(9)?,
                    base_url: r.get(10)?,
                    last_event_cursor: r.get(11)?,
                })
            },
        )
        .optional()?;
    Ok(row)
}

/// Insert or update the singleton connection row.
pub fn save(conn: &Connection, c: &StoredConnection) -> Result<()> {
    conn.execute(
        "INSERT INTO hubrise_connection
            (id, access_token, account_id, account_name, location_id, location_name,
             catalog_id, catalog_name, customer_list_id, customer_list_name,
             scope, base_url)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(id) DO UPDATE SET
            access_token = excluded.access_token,
            account_id = excluded.account_id,
            account_name = excluded.account_name,
            location_id = excluded.location_id,
            location_name = excluded.location_name,
            catalog_id = excluded.catalog_id,
            catalog_name = excluded.catalog_name,
            customer_list_id = excluded.customer_list_id,
            customer_list_name = excluded.customer_list_name,
            scope = excluded.scope,
            base_url = excluded.base_url,
            auth_error_at = NULL,
            last_error = NULL",
        params![
            c.access_token,
            c.account_id,
            c.account_name,
            c.location_id,
            c.location_name,
            c.catalog_id,
            c.catalog_name,
            c.customer_list_id,
            c.customer_list_name,
            c.scope,
            c.base_url,
        ],
    )?;
    Ok(())
}

/// Delete the stored connection (e.g. on admin-triggered disconnect).
pub fn clear(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM hubrise_connection", [])?;
    Ok(())
}

/// Persist the latest event cursor returned by the poller.
pub fn update_cursor(conn: &Connection, cursor: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE hubrise_connection SET last_event_cursor = ?1, last_poll_at = datetime('now') WHERE id = 1",
        params![cursor],
    )?;
    Ok(())
}

/// `HubRise` refused the token (401): the connection was revoked or
/// blocked. Shown until a request succeeds again.
pub fn set_auth_error(conn: &Connection, message: &str) -> Result<()> {
    conn.execute(
        "UPDATE hubrise_connection
         SET auth_error_at = COALESCE(auth_error_at, datetime('now')), last_error = ?1
         WHERE id = 1",
        params![message],
    )?;
    Ok(())
}

/// A request went through: no authentication error any more.
pub fn clear_auth_error(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE hubrise_connection SET auth_error_at = NULL WHERE id = 1 AND auth_error_at IS NOT NULL",
        [],
    )?;
    Ok(())
}

/// The last transport error, for the status screen (`None` clears it).
pub fn set_last_error(conn: &Connection, message: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE hubrise_connection SET last_error = ?1 WHERE id = 1",
        params![message],
    )?;
    Ok(())
}

/// The passive callback is registered (see `callbacks`).
pub fn mark_callback_registered(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE hubrise_connection SET callback_registered_at = datetime('now') WHERE id = 1",
        [],
    )?;
    Ok(())
}

/// What the status screen shows about the connection.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ConnectionHealth {
    pub connected: bool,
    pub last_poll_at: Option<String>,
    pub auth_error_at: Option<String>,
    pub last_error: Option<String>,
    pub callback_registered_at: Option<String>,
    pub last_catalog_sync_at: Option<String>,
}

pub fn health(conn: &Connection) -> Result<ConnectionHealth> {
    let row = conn
        .query_row(
            "SELECT last_poll_at, auth_error_at, last_error, callback_registered_at,
                    last_catalog_sync_at
             FROM hubrise_connection WHERE id = 1",
            [],
            |r| {
                Ok(ConnectionHealth {
                    connected: true,
                    last_poll_at: r.get(0)?,
                    auth_error_at: r.get(1)?,
                    last_error: r.get(2)?,
                    callback_registered_at: r.get(3)?,
                    last_catalog_sync_at: r.get(4)?,
                })
            },
        )
        .optional()?;
    Ok(row.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pos_db::schema::run_migrations;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn sample() -> StoredConnection {
        StoredConnection {
            access_token: "tok-123".to_string(),
            account_id: "acc-1".to_string(),
            account_name: "Test Account".to_string(),
            location_id: "loc-1".to_string(),
            location_name: "Liège".to_string(),
            catalog_id: "cat-1".to_string(),
            catalog_name: Some("Menu".to_string()),
            customer_list_id: None,
            customer_list_name: None,
            scope: "location[orders.write]".to_string(),
            base_url: "https://api.hubrise.com/v1".to_string(),
            last_event_cursor: None,
        }
    }

    #[test]
    fn load_returns_none_on_empty_db() {
        let conn = test_conn();
        assert!(load(&conn).unwrap().is_none());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let conn = test_conn();
        save(&conn, &sample()).unwrap();
        let loaded = load(&conn).unwrap().expect("should exist");
        assert_eq!(loaded.access_token, "tok-123");
        assert_eq!(loaded.location_name, "Liège");
    }

    #[test]
    fn save_upserts_on_second_call() {
        let conn = test_conn();
        let mut s = sample();
        save(&conn, &s).unwrap();
        s.access_token = "tok-456".to_string();
        save(&conn, &s).unwrap();
        let loaded = load(&conn).unwrap().unwrap();
        assert_eq!(loaded.access_token, "tok-456");
    }

    #[test]
    fn clear_removes_connection() {
        let conn = test_conn();
        save(&conn, &sample()).unwrap();
        clear(&conn).unwrap();
        assert!(load(&conn).unwrap().is_none());
    }

    #[test]
    fn an_auth_error_shows_until_cleared() {
        let conn = test_conn();
        save(&conn, &sample()).unwrap();
        set_auth_error(&conn, "blocked").unwrap();
        let h = health(&conn).unwrap();
        assert!(h.auth_error_at.is_some());
        assert_eq!(h.last_error.as_deref(), Some("blocked"));
        clear_auth_error(&conn).unwrap();
        assert!(health(&conn).unwrap().auth_error_at.is_none());
    }

    #[test]
    fn health_without_connection_is_disconnected() {
        let conn = test_conn();
        assert!(!health(&conn).unwrap().connected);
    }

    #[test]
    fn update_cursor_persists() {
        let conn = test_conn();
        save(&conn, &sample()).unwrap();
        update_cursor(&conn, Some("cur-abc")).unwrap();
        let loaded = load(&conn).unwrap().unwrap();
        assert_eq!(loaded.last_event_cursor, Some("cur-abc".to_string()));
    }

    #[test]
    fn debug_output_hides_the_access_token() {
        let mut c = sample();
        c.access_token = "tok-very-secret".into();
        let out = format!("{c:?}");
        assert!(!out.contains("tok-very-secret"));
        assert!(out.contains("<redacted>"));
    }
}
