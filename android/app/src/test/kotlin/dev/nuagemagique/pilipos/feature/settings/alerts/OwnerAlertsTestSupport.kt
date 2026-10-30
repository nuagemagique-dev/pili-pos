@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.feature.settings.alerts

import dev.nuagemagique.pilipos.engine.DisposableEngine
import io.mockk.coEvery
import uniffi.pos_ffi.AlertKinds
import uniffi.pos_ffi.AlertLogRow
import uniffi.pos_ffi.AlertTestResult
import uniffi.pos_ffi.OwnerAlerts

internal fun allKinds(on: Boolean = true): AlertKinds =
    AlertKinds(
        fdm = on,
        printer = on,
        terminal = on,
        scale = on,
        zMissing = on,
        onlineWaiting = on,
    )

@Suppress("LongParameterList")
internal fun ownerAlerts(
    enabled: Boolean = false,
    chatId: String = "",
    token: Boolean = true,
    label: String = "",
    language: String = "fr",
    reminder: Long = 60L,
): OwnerAlerts =
    OwnerAlerts(
        enabled = enabled,
        chatId = chatId,
        label = label,
        language = language,
        kinds = allKinds(),
        zDeadlineHour = 5L,
        onlineWaitMinutes = 5L,
        reminderMinutes = reminder,
        tokenConfigured = token,
    )

internal fun logRow(
    id: Long,
    transition: String = "raised",
    kind: String = "fdm",
    outcome: String = "sent",
): AlertLogRow =
    AlertLogRow(
        id = id,
        kind = kind,
        transition = transition,
        detail = null,
        outcome = outcome,
        httpStatus = null,
        createdAt = "2026-10-17 09:00:00",
    )

internal val testSent = AlertTestResult(sent = true, detail = "ok")

/** Stubs the reads of the alerts panel: off, a token on the till, an empty log. */
internal fun stubAlerts(
    engine: DisposableEngine,
    settings: OwnerAlerts = ownerAlerts(),
    log: List<AlertLogRow> = emptyList(),
) {
    coEvery { engine.getOwnerAlerts() } returns settings
    coEvery { engine.listOwnerAlertLog(ALERT_LOG_ROWS) } returns log
}
