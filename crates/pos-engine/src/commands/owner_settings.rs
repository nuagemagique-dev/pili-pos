//! The owner settings: typed access to the `owner_settings` store
//! (migration 60, V12-D2), shared by the owner alerts and the v1.2
//! features that follow (courses, gift vouchers).
//!
//! One JSON document per key. Each key has a Rust type with
//! `serde(deny_unknown_fields)`; an absent key reads as the type's
//! `Default` (the feature is off). A stored document that no longer
//! parses (written by a later build, say) also reads as the default and
//! is logged: a setting never stops the till.
//!
//! [`set`] writes the document and its log row in one transaction.

use pos_db::queries::owner_settings;
use rusqlite::Connection;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::commands::errors;

/// The key of the owner alerts.
pub const KEY_ALERTS: &str = "alerts";

/// The settings stored under `key`, or the default when none are stored
/// or the stored document does not parse.
pub fn get<T: DeserializeOwned + Default>(db: &Connection, key: &str) -> Result<T, String> {
    let Some(json) = owner_settings::get(db, key).map_err(errors::internal)? else {
        return Ok(T::default());
    };
    match serde_json::from_str(&json) {
        Ok(value) => Ok(value),
        Err(e) => {
            tracing::warn!(key, %e, "owner setting does not parse, reading the default");
            Ok(T::default())
        }
    }
}

/// Store `value` under `key` and log the change (`changed_by`: INSZ of
/// the manager). Returns whether anything changed.
pub fn set<T: Serialize>(
    db: &Connection,
    key: &str,
    value: &T,
    changed_by: Option<&str>,
) -> Result<bool, String> {
    let json = serde_json::to_string(value).map_err(errors::internal)?;
    owner_settings::set(db, key, &json, changed_by).map_err(errors::internal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Demo {
        #[serde(default)]
        on: bool,
        #[serde(default)]
        n: i64,
    }

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        pos_db::migrate::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn an_absent_key_reads_the_default() {
        assert_eq!(get::<Demo>(&db(), "demo").unwrap(), Demo::default());
    }

    #[test]
    fn a_stored_value_reads_back_and_is_logged_once() {
        let conn = db();
        let v = Demo { on: true, n: 3 };
        assert!(set(&conn, "demo", &v, Some("00000000029")).unwrap());
        assert!(!set(&conn, "demo", &v, Some("00000000029")).unwrap());
        assert_eq!(get::<Demo>(&conn, "demo").unwrap(), v);
        assert_eq!(owner_settings::history(&conn, "demo", 10).unwrap().len(), 1);
    }

    #[test]
    fn a_document_with_an_unknown_field_reads_the_default() {
        let conn = db();
        owner_settings::set(&conn, "demo", r#"{"on":true,"future":1}"#, None).unwrap();
        assert_eq!(get::<Demo>(&conn, "demo").unwrap(), Demo::default());
        owner_settings::set(&conn, "demo", "not json", None).unwrap();
        assert_eq!(get::<Demo>(&conn, "demo").unwrap(), Demo::default());
    }
}
