//! Export status shown to the operator (card X1-T5, decision X1D10).
//!
//! [`ExportStatus`] is the contract between the engine, the Tauri command,
//! the FFI, the Vue screen and the Kotlin screen: camelCase JSON, `i64`
//! counts, RFC 3339 UTC times. Fields are never renamed; adding one is a
//! change of the cert command surface and needs a decision.

use serde::{Deserialize, Serialize};

/// What the operator sees about the nightly export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportStatus {
    /// The export is configured and usable.
    pub enabled: bool,
    /// Why the export is off although a section exists (a configuration
    /// error); `None` when it is simply not configured or working.
    pub problem: Option<String>,
    /// `scheme://host[:port]` of the endpoint, without path or credentials.
    pub endpoint: Option<String>,
    /// Days an export stays in the outbox.
    pub keep_days: i64,
    /// Newest prepared export.
    pub last_export_id: Option<String>,
    /// When the newest export was prepared.
    pub last_prepared_at: Option<String>,
    /// Newest export completely uploaded.
    pub last_uploaded_id: Option<String>,
    /// When it was completely uploaded.
    pub last_uploaded_at: Option<String>,
    /// Exports not completely uploaded yet.
    pub pending: i64,
    /// Earliest scheduled retry of the pending exports.
    pub next_attempt_at: Option<String>,
    /// Last error of a preparation or an upload, cleared by a success.
    pub last_error: Option<String>,
    /// When it happened.
    pub last_error_at: Option<String>,
    /// Exports held in the outbox (sent or not).
    pub kept: i64,
    /// Unsent exports deleted by the retention, since the export exists.
    pub dropped: i64,
}

impl ExportStatus {
    /// The status of an installation without a working export: `enabled`
    /// false, `keep_days` 0, everything else empty. `problem` explains a
    /// configuration error.
    pub fn disabled(problem: Option<String>) -> ExportStatus {
        ExportStatus {
            enabled: false,
            problem,
            endpoint: None,
            keep_days: 0,
            last_export_id: None,
            last_prepared_at: None,
            last_uploaded_id: None,
            last_uploaded_at: None,
            pending: 0,
            next_attempt_at: None,
            last_error: None,
            last_error_at: None,
            kept: 0,
            dropped: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_json_field_names_are_the_contract() {
        let status = ExportStatus::disabled(Some("export.url: bad".into()));
        let value = serde_json::to_value(&status).unwrap();
        let mut keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        let mut expected = vec![
            "enabled",
            "problem",
            "endpoint",
            "keepDays",
            "lastExportId",
            "lastPreparedAt",
            "lastUploadedId",
            "lastUploadedAt",
            "pending",
            "nextAttemptAt",
            "lastError",
            "lastErrorAt",
            "kept",
            "dropped",
        ];
        expected.sort_unstable();
        assert_eq!(keys, expected);
        assert_eq!(value["problem"], "export.url: bad");
        assert!(value["endpoint"].is_null());
    }

    #[test]
    fn a_status_round_trips_and_refuses_unknown_fields() {
        let mut status = ExportStatus::disabled(None);
        status.enabled = true;
        status.keep_days = 30;
        status.pending = 2;
        status.last_export_id = Some("20261008-z000042".into());
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(serde_json::from_str::<ExportStatus>(&json).unwrap(), status);
        let extra = json.replacen('{', "{\"surprise\":1,", 1);
        assert!(serde_json::from_str::<ExportStatus>(&extra).is_err());
    }

    #[test]
    fn disabled_is_empty() {
        let s = ExportStatus::disabled(None);
        assert!(!s.enabled);
        assert_eq!(s.keep_days, 0);
        assert_eq!((s.pending, s.kept, s.dropped), (0, 0, 0));
        assert!(s.problem.is_none() && s.endpoint.is_none() && s.last_error.is_none());
    }
}
