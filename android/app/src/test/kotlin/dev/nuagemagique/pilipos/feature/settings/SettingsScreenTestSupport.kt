@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.feature.settings

import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.SettingsEntry
import uniffi.pos_ffi.Adoption
import uniffi.pos_ffi.Issue
import uniffi.pos_ffi.IssueCode
import uniffi.pos_ffi.SwitchChange

/** An [AppNavigator] that records what it was asked. */
internal class RecordingNavigator : AppNavigator {
    val calls: MutableList<String> = mutableListOf()

    override fun navigate(route: String) {
        calls += "navigate:$route"
    }

    override fun back() {
        calls += "back"
    }

    override fun toLogin() {
        calls += "toLogin"
    }

    override fun replaceAll(route: String) {
        calls += "replaceAll:$route"
    }
}

/** Real string ids for the rows of a render test (the rows of [testRegistry] carry placeholder ids). */
internal val shownOpen = SettingsEntry("a.open", R.string.ui_tables_none, "settings-open", 10, managerOnly = false)
internal val shownAbout = SettingsEntry("platform.about", R.string.ui_tables_signed_out, "settings-about", 900, managerOnly = true)
internal val shownExport = SettingsEntry("platform.export", R.string.ui_settings_rows_title, "settings-export", 910, managerOnly = true)

/** The state of a manager's screen: three employees, two deliveries, the mail service up. */
internal fun managerState(
    rows: List<SettingsEntry> = listOf(shownOpen, shownAbout, shownExport),
    hubrise: HubriseSwitchState = HubriseSwitchState(),
): SettingsUiState =
    SettingsUiState(
        rows = rows,
        isManager = true,
        currentInsz = ADMIN_INSZ,
        info = engineSettings(),
        users = listOf(admin, cashier, waiter),
        deliveries = listOf(delivery(1L), delivery(2L).copy(status = "failed", attempts = 3L, lastError = "smtp 550")),
        mailStatus = mail(),
        hubrise = hubrise,
    )

/** The state of a non-manager's screen: only the rows open to every role. */
internal fun workerState(rows: List<SettingsEntry> = listOf(shownOpen)): SettingsUiState =
    SettingsUiState(rows = rows, isManager = false, currentInsz = CASHIER_INSZ, info = engineSettings())

internal fun switchChange(
    enabled: Boolean,
    source: String = "admin",
    by: String? = "Alice",
    at: String = "2026-10-09 09:05:00",
): SwitchChange = SwitchChange(enabled = enabled, source = source, changedBy = by, changedAt = at)

internal fun hubriseOn(
    open: Long = 0L,
    log: List<SwitchChange> = emptyList(),
): HubriseSwitchState = HubriseSwitchState(enabled = true, visible = true, onlineOrdersOpen = open, log = log)

/** A connected HubRise panel for a manager, with one channel and a catalog synced at 09:05 UTC. */
internal fun connectedState(
    manager: Boolean = true,
    adoptions: List<Adoption> = emptyList(),
    refused: Boolean = false,
): HubriseUiState =
    HubriseUiState(
        active = true,
        isManager = manager,
        status = connection(true),
        health =
            health(
                awaiting = adoptions.isNotEmpty(),
                lastCatalogSyncAt = "2026-10-09 09:05:00",
                catalog =
                    if (refused) {
                        syncState(rejectedAt = "2026-10-09 09:10:00").copy(
                            issues = listOf(Issue(IssueCode.EMPTY_KEY, ""), Issue(IssueCode.MISSING_NAME, "sushi-42")),
                        )
                    } else {
                        syncState(appliedAt = "2026-10-09 09:05:00")
                    },
            ),
        adoptions = adoptions,
        adopted = adoptions.map { it.menuItemId }.toSet(),
        channels = listOf(channel("Deliveroo"), channel("Webshop", ChannelKind.WEBSHOP, ChannelPayment.ONLINE, ChargePolicy.INCLUDE)),
    )
