//! The nightly export records (phase X1, decision X1D10): the FFI mirror of
//! `pos_engine::commands::export::ExportStatus`, the value of `export_status` and `export_run_now`.

/// What the operator sees about the nightly export.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportStatus {
    /// The export is configured and usable.
    pub enabled: bool,
    /// Why the export is off although a section exists (a configuration error).
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
