package dev.nuagemagique.pilipos.nightly

import uniffi.pos_ffi.ExportStatus

/** The state line of the export, as the Vue panel derives it. */
internal enum class NightlyKind { Invalid, Disabled, Pending, Ok }

/** Invalid when a configuration problem is reported, disabled when not enabled, else pending or up to date. */
internal fun ExportStatus.kind(): NightlyKind =
    when {
        problem != null -> NightlyKind.Invalid
        !enabled -> NightlyKind.Disabled
        pending > 0 -> NightlyKind.Pending
        else -> NightlyKind.Ok
    }

/** "Retry now" is offered when the export is enabled and its configuration is valid. */
internal fun ExportStatus.canRetry(): Boolean = kind() == NightlyKind.Pending || kind() == NightlyKind.Ok
