package dev.nuagemagique.pilipos.nightly

import uniffi.pos_ffi.ExportStatus

/** An [ExportStatus] with sensible defaults: enabled, nothing pending, one export uploaded. */
@Suppress("LongParameterList")
internal fun exportStatus(
    enabled: Boolean = true,
    problem: String? = null,
    endpoint: String? = "https://exports.example.test:8443",
    keepDays: Long = 30,
    lastExportId: String? = "2026-10-08",
    lastPreparedAt: String? = "2026-10-09T01:00:00Z",
    lastUploadedId: String? = "2026-10-08",
    lastUploadedAt: String? = "2026-10-09T01:05:00Z",
    pending: Long = 0,
    nextAttemptAt: String? = null,
    lastError: String? = null,
    lastErrorAt: String? = null,
    kept: Long = 12,
    dropped: Long = 0,
): ExportStatus =
    ExportStatus(
        enabled = enabled,
        problem = problem,
        endpoint = endpoint,
        keepDays = keepDays,
        lastExportId = lastExportId,
        lastPreparedAt = lastPreparedAt,
        lastUploadedId = lastUploadedId,
        lastUploadedAt = lastUploadedAt,
        pending = pending,
        nextAttemptAt = nextAttemptAt,
        lastError = lastError,
        lastErrorAt = lastErrorAt,
        kept = kept,
        dropped = dropped,
    )

/** Not configured: the section is absent, so nothing else is known. */
internal fun disabledStatus(): ExportStatus =
    exportStatus(
        enabled = false,
        endpoint = null,
        lastExportId = null,
        lastPreparedAt = null,
        lastUploadedId = null,
        lastUploadedAt = null,
        kept = 0,
    )

/** Waiting exports with a failed attempt behind them. */
internal fun pendingStatus(): ExportStatus =
    exportStatus(
        pending = 3,
        lastUploadedAt = "2026-10-06T01:05:00Z",
        nextAttemptAt = "2026-10-09T10:30:00Z",
        lastError = "upload refused: HTTP 503",
        lastErrorAt = "2026-10-09T10:00:00Z",
        dropped = 2,
    )
