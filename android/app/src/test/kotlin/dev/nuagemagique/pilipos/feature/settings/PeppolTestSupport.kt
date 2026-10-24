package dev.nuagemagique.pilipos.feature.settings

import uniffi.pos_ffi.Delivery
import uniffi.pos_ffi.PeppolDeliveries

internal fun peppolDelivery(
    id: Long,
    status: String = DELIVERY_PARKED,
    reason: String? = "recipient not on Peppol",
    attempts: Long = 3L,
): Delivery =
    Delivery(
        id = id,
        fiscalEventId = 40L + id,
        invoiceNo = "2026000$id",
        customerEndpointId = "0208:0308357159",
        apVendor = "storecove",
        apMessageId = null,
        status = status,
        errorMsg = if (status == DELIVERY_PARKED) reason else null,
        attempts = attempts,
        nextAttemptAt = null,
        lastAttemptAt = "2026-10-17 09:00:00",
        parkedAt = if (status == DELIVERY_PARKED) "2026-10-17 09:00:00" else null,
        acceptedAt = null,
        createdAt = "2026-10-17 08:00:00",
        lookupStatus = "not_registered",
    )

internal fun peppolPanel(
    configured: Boolean = true,
    deliveries: List<Delivery> = listOf(peppolDelivery(9L), peppolDelivery(10L, status = "queued")),
    queued: Long = 2L,
    accepted: Long = 14L,
    failed: Long = 1L,
): PeppolDeliveries =
    PeppolDeliveries(
        configured = configured,
        provider = if (configured) "storecove" else null,
        senderId = if (configured) "0208:0123456789" else null,
        queued = queued,
        accepted = accepted,
        failed = failed,
        deliveries = deliveries,
    )
