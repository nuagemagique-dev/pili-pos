package dev.nuagemagique.pilipos.feature.reports

import uniffi.pos_ffi.Customer
import uniffi.pos_ffi.CustomerLookup
import uniffi.pos_ffi.InvoicePreview
import uniffi.pos_ffi.InvoiceResult
import uniffi.pos_ffi.InvoiceSummary
import uniffi.pos_ffi.InvoiceableSale
import uniffi.pos_ffi.PreviewLine
import uniffi.pos_ffi.PreviewVat

internal fun customer(
    id: Long = 7L,
    vat: String = "BE0308357159",
    name: String = "ACME SPRL",
    status: String = PEPPOL_REGISTERED,
): Customer =
    Customer(
        id = id,
        vatNumber = vat,
        name = name,
        street = "RUE DU PONT 1",
        postalCode = "4000",
        city = "LIEGE",
        countryCode = "BE",
        peppolStatus = status,
        peppolCheckedAt = "2026-10-17 10:00:00",
    )

internal fun invoiceableSale(
    id: Long,
    totalCents: Long = 1_500L,
    invoicedBy: String? = null,
    isRefund: Boolean = false,
): InvoiceableSale =
    InvoiceableSale(
        fiscalEventId = id,
        ticketNumber = id,
        posDateTime = "2026-10-17T12:30:00+02:00",
        totalCents = totalCents,
        fdmId = "FDM0000001",
        eventCounter = id,
        totalCounter = id + 20L,
        isRefund = isRefund,
        invoicedBy = invoicedBy,
    )

internal fun lookup(
    status: String = PEPPOL_REGISTERED,
    known: Customer? = null,
    vat: String = "BE0308357159",
    country: String = "BE",
    deliverable: Boolean = country == "BE",
): CustomerLookup =
    CustomerLookup(
        vatNumber = vat,
        countryCode = country,
        customer = known,
        peppolStatus = status,
        peppolChecked = status != PEPPOL_UNKNOWN,
        deliverable = deliverable,
    )

internal fun invoicePreview(problems: List<String> = emptyList()): InvoicePreview =
    InvoicePreview(
        sales = listOf(invoiceableSale(11L, 2_400L), invoiceableSale(12L, 1_500L)),
        lines =
            listOf(
                PreviewLine(description = "Maki saumon x8", quantityMilli = 2_000L, totalCents = 2_400L, vatLabel = "B"),
                PreviewLine(description = "Coca-Cola", quantityMilli = 1_000L, totalCents = 1_500L, vatLabel = "A"),
            ),
        vat =
            listOf(
                PreviewVat(label = "A", rateHundredths = 2_100L, baseCents = 1_240L, vatCents = 260L, totalCents = 1_500L),
                PreviewVat(label = "B", rateHundredths = 1_200L, baseCents = 2_143L, vatCents = 257L, totalCents = 2_400L),
            ),
        totalCents = 3_900L,
        problems = problems,
    )

internal fun invoiceResult(
    success: Boolean = true,
    error: String? = null,
    printed: Boolean = true,
    delivery: String? = "queued",
): InvoiceResult =
    InvoiceResult(
        success = success,
        fiscalEventId = if (success) 40L else null,
        invoiceNo = if (success) "20260002" else null,
        printed = printed,
        deliveryStatus = delivery,
        error = error,
    )

internal fun invoiceSummary(
    id: Long,
    status: String? = "queued",
    fiscalEventId: Long = id + 40L,
    name: String = "ACME SPRL",
): InvoiceSummary =
    InvoiceSummary(
        id = id,
        invoiceNo = "2026000$id",
        fiscalEventId = fiscalEventId,
        issueDate = "2026-10-17",
        customerName = name,
        customerVatNumber = "BE0308357159",
        totalCents = 3_900L,
        saleEventIds = listOf(11L, 12L),
        deliveryStatus = status,
        deliveryId = if (status == null) null else id,
    )
