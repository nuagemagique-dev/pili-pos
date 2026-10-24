package dev.nuagemagique.pilipos.feature.reports

import uniffi.pos_ffi.Customer
import uniffi.pos_ffi.CustomerLookup
import uniffi.pos_ffi.InvoicePreview
import uniffi.pos_ffi.InvoiceResult
import uniffi.pos_ffi.InvoiceSummary
import uniffi.pos_ffi.InvoiceableSale

/** The Peppol status of a customer that has not been asked about (`unknown`). */
internal const val PEPPOL_UNKNOWN = "unknown"

/** The Peppol status of a customer who can receive e-invoices. */
internal const val PEPPOL_REGISTERED = "registered"

/** The Peppol status of a customer the network does not know. */
internal const val PEPPOL_NOT_REGISTERED = "not_registered"

/** The longest VAT number the form takes (`maxLength` of the Vue keyboard). */
internal const val VAT_MAX_CHARS = 16

/** The longest postal code. */
internal const val POSTAL_MAX_CHARS = 10

/** The length of an ISO 3166-1 alpha-2 country code. */
internal const val COUNTRY_CHARS = 2

/** The longest name, street or city. */
internal const val TEXT_MAX_CHARS = 80

/** The steps of the invoice dialog (`InvoiceStep` of `stores/invoice.ts`). */
internal enum class InvoiceStep {
    /** The operator chooses or types the customer. */
    CUSTOMER,

    /** The engine's preview of the invoice: lines, VAT, total. */
    PREVIEW,

    /** The invoice (event I) is being signed. */
    PROCESSING,

    /** The engine answered: [InvoiceFlow.result]. */
    RESULT,
}

/** The customer form (`CustomerForm` of `stores/invoice.ts`); an [id] edits a known customer. */
internal data class CustomerForm(
    val id: Long? = null,
    val vatNumber: String = "",
    val name: String = "",
    val street: String = "",
    val postalCode: String = "",
    val city: String = "",
    val countryCode: String = "",
) {
    /** True when every field the engine requires is filled (the country is optional: the VAT number gives it). */
    val complete: Boolean
        get() = listOf(vatNumber, name, street, postalCode, city).all { it.isNotBlank() }
}

/**
 * The invoice dialog (`useInvoiceStore` of `stores/invoice.ts`).
 *
 * The engine owns every rule: the VAT number check, the lines and totals read from the archived sales (the UI
 * sends sale ids and a customer id), the numbering, the print and the Peppol queue. A Peppol lookup that fails or
 * is offline gives [PEPPOL_UNKNOWN] and never blocks the invoice.
 */
internal data class InvoiceFlow(
    /** The sales on the invoice (the ticks of the tickets list when the dialog opened). */
    val saleEventIds: List<Long>,
    val step: InvoiceStep = InvoiceStep.CUSTOMER,
    val form: CustomerForm = CustomerForm(),
    val search: String = "",
    val searchResults: List<Customer> = emptyList(),
    /** The answer of the VAT check and Peppol lookup, null until asked or after the number changed. */
    val lookup: CustomerLookup? = null,
    val lookupBusy: Boolean = false,
    /** The screen text of the last refusal of the customer step, or null. */
    val customerError: String? = null,
    /** The customer as saved by the engine. */
    val customer: Customer? = null,
    val preview: InvoicePreview? = null,
    /** The screen texts of the preview's coded problems; the confirmation is blocked while there are any. */
    val problems: List<String> = emptyList(),
    val result: InvoiceResult? = null,
    /** The screen text of the refusal when the invoice was not issued, or null. */
    val resultError: String? = null,
    val busy: Boolean = false,
) {
    /** The status of the chip: the lookup's, else the saved customer's, else unknown. */
    val peppolStatus: String get() = lookup?.peppolStatus ?: customer?.peppolStatus ?: PEPPOL_UNKNOWN

    /** True when the continue button may be pressed. */
    val canContinue: Boolean get() = form.complete && !busy && !lookupBusy

    /** True when the confirm button may be pressed. */
    val canConfirm: Boolean get() = !busy && problems.isEmpty() && preview != null
}

/** The invoices part of the reports screen: the sales to tick, the invoices issued and the dialog. */
internal data class InvoicesState(
    /** The recent sales a cashier can put on an invoice (`list_invoiceable_sales`). */
    val sales: List<InvoiceableSale> = emptyList(),
    /** The invoices issued, newest first (`list_invoices`). */
    val invoices: List<InvoiceSummary> = emptyList(),
    /** The fiscal event ids ticked in the tickets list. */
    val selected: List<Long> = emptyList(),
    /** The dialog, or null when it is closed. */
    val flow: InvoiceFlow? = null,
    /** The invoice whose copy is being signed, or null. */
    val copying: Long? = null,
    /** True for a manager: only managers copy an invoice (the engine refuses the others). */
    val canCopy: Boolean = false,
)

/** The tickets list with the sales that are not in it (a sale the copy list no longer holds), newest first. */
internal fun ticketRowsWith(
    rows: List<TicketRow>,
    sales: List<InvoiceableSale>,
): List<TicketRow> {
    val known = rows.map { it.fiscalEventId }.toSet()
    val extra =
        sales.filter { it.fiscalEventId !in known }.map { sale ->
            TicketRow(
                fiscalEventId = sale.fiscalEventId,
                ticketNumber = sale.ticketNumber,
                tableNumber = null,
                totalCents = sale.totalCents,
                createdAt = sale.posDateTime,
                kind = if (sale.isRefund) TicketKind.REFUND else TicketKind.SALE,
                copyable = null,
                refundable = null,
            )
        }
    return (rows + extra).sortedByDescending { it.fiscalEventId }
}

/** The Vue key of the label of a delivery status (`statusKey` of `InvoiceList.vue`). */
internal fun invoiceStatusKey(status: String?): String =
    when (status) {
        "queued", "accepted", "failed" -> "invoice_status_$status"
        else -> "invoice_status_none"
    }

/** "2026-10-17" (the Brussels date of an invoice) as "17/10/2026" (`issueDay` of `InvoiceList.vue`). */
internal fun issueDay(isoDate: String): String {
    val parts = isoDate.take(ISO_DATE_CHARS).split("-")
    return if (parts.size == ISO_DATE_PARTS && parts.all { it.isNotEmpty() }) "${parts[2]}/${parts[1]}/${parts[0]}" else isoDate
}

private const val ISO_DATE_CHARS = 10
private const val ISO_DATE_PARTS = 3

private const val MILLI = 1000L

/** A quantity in thousandths as the dialog shows it: "2" for 2000, "0,35" for 350 (`quantity` of `InvoiceDialog.vue`). */
internal fun quantityText(milli: Long): String {
    if (milli % MILLI == 0L) return (milli / MILLI).toString()
    val text = java.math.BigDecimal(milli).movePointLeft(MILLI_DIGITS).stripTrailingZeros().toPlainString()
    return text.replace('.', ',')
}

private const val MILLI_DIGITS = 3

private const val HUNDRED = 100L

/** A VAT rate in hundredths of a percent: "21 %" for 2100, "5,5 %" for 550 (`rate` of `InvoiceDialog.vue`). */
internal fun rateText(hundredths: Long): String {
    val text =
        if (hundredths % HUNDRED == 0L) {
            (hundredths / HUNDRED).toString()
        } else {
            java.math.BigDecimal(hundredths).movePointLeft(2).stripTrailingZeros().toPlainString().replace('.', ',')
        }
    return "$text %"
}
