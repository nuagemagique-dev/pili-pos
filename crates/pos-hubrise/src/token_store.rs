//! SQLite-backed singleton store for the HubRise connection row.

use rusqlite::{params, Connection, OptionalExtension};

use crate::errors::Result;

#[derive(Debug, Clone)]
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

/// Load the currently persisted HubRise connection, if any.
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
            base_url = excluded.base_url",
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
    fn update_cursor_persists() {
        let conn = test_conn();
        save(&conn, &sample()).unwrap();
        update_cursor(&conn, Some("cur-abc")).unwrap();
        let loaded = load(&conn).unwrap().unwrap();
        assert_eq!(loaded.last_event_cursor, Some("cur-abc".to_string()));
    }
}
