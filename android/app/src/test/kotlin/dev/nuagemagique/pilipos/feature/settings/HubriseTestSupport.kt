@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.feature.settings

import dev.nuagemagique.pilipos.engine.DisposableEngine
import io.mockk.coEvery
import uniffi.pos_ffi.Adoption
import uniffi.pos_ffi.ApplyOutcome
import uniffi.pos_ffi.Backlog
import uniffi.pos_ffi.CallbackEvent
import uniffi.pos_ffi.Channel
import uniffi.pos_ffi.ConnectionHealth
import uniffi.pos_ffi.HubriseConnectionInfo
import uniffi.pos_ffi.HubriseStatus
import uniffi.pos_ffi.SyncState

internal fun connection(connected: Boolean): HubriseConnectionInfo =
    HubriseConnectionInfo(
        connected = connected,
        accountId = if (connected) "acc" else null,
        accountName = if (connected) "Account" else null,
        locationId = if (connected) "loc" else null,
        locationName = if (connected) "Location" else null,
        catalogId = null,
        catalogName = null,
        customerListId = null,
        customerListName = null,
    )

internal fun syncState(
    appliedAt: String? = null,
    rejectedAt: String? = null,
): SyncState = SyncState(source = "hubrise", managesMenu = true, appliedAt = appliedAt, rejectedAt = rejectedAt, issues = emptyList())

internal fun health(
    connected: Boolean = true,
    awaiting: Boolean = false,
    lastCatalogSyncAt: String? = null,
    catalog: SyncState? = null,
): HubriseStatus =
    HubriseStatus(
        health =
            ConnectionHealth(
                connected = connected,
                lastPollAt = null,
                authErrorAt = null,
                lastError = null,
                callbackRegisteredAt = null,
                lastCatalogSyncAt = lastCatalogSyncAt,
            ),
        outbox = Backlog(pending = 0L, parked = 0L),
        outboxError = null,
        parkedEvents = 0L,
        staleOrders = 0L,
        catalog = catalog,
        catalogAwaitingAdoption = awaiting,
    )

internal fun adoption(
    id: Long,
    name: String = "Item $id",
): Adoption = Adoption(menuItemId = id, key = "k$id", name = name, priceCents = 100L * id)

internal fun channel(
    name: String = "Deliveroo",
    kind: String = ChannelKind.PLATFORM,
    payment: String = ChannelPayment.APP,
    charge: String = ChargePolicy.EXCLUDE,
): Channel = Channel(channel = name, kind = kind, paymentType = payment, chargePolicy = charge)

internal fun callbackEvent(type: String = "update"): CallbackEvent =
    CallbackEvent(
        id = "e1",
        resourceType = "catalog",
        eventType = type,
        createdAt = "2026-10-09T09:00:00Z",
        previousState = null,
        newState = null,
        extra = "",
    )

internal val applied: ApplyOutcome = ApplyOutcome.Applied(hash = "h", activeItems = 3L, deactivatedItems = 0L)

/** Stubs every read the HubRise panel makes; a test overrides the one it cares about afterwards. */
internal fun stubHubriseReads(engine: DisposableEngine) {
    coEvery { engine.hubriseGetConnectionStatus() } returns connection(true)
    coEvery { engine.hubriseStatus() } returns health()
    coEvery { engine.hubriseListChannels() } returns listOf(channel())
    coEvery { engine.hubriseAdoptionPreview() } returns emptyList()
}
