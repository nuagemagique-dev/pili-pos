package dev.nuagemagique.pilipos.feature.reports

import uniffi.pos_ffi.SalesProduct
import uniffi.pos_ffi.SalesReport

/** How the by-product table is sorted. */
internal enum class SalesSort {
    AMOUNT,
    QUANTITY,
}

/** The date field the keypad types into. */
internal enum class SalesField {
    FROM,
    TO,
}

/**
 * The sales tab (`SalesReportPanel.vue`): the non-fiscal sales report of a range of booking dates. The report is
 * the engine's; nothing is computed here (PD208), the gap with the Z and the length of the hour bars come with it.
 */
internal data class SalesState(
    val preset: SalesPreset = SalesPreset.TODAY,
    /** The range shown, `YYYY-MM-DD` both; before the first answer it is the calendar date of the day. */
    val from: String = "",
    val to: String = "",
    val report: SalesReport? = null,
    val loading: Boolean = false,
    /** The screen text of the engine's refusal, or null. */
    val error: String? = null,
    /** The business date now, once the engine said it: what "today" is. */
    val businessDate: String? = null,
    val sort: SalesSort = SalesSort.AMOUNT,
    /** The custom range: the digits typed so far, per field. */
    val field: SalesField = SalesField.FROM,
    val fromDigits: String = "",
    val toDigits: String = "",
    val printing: Boolean = false,
) {
    /** The typed start date, or null while it is not a real date. */
    val typedFrom: String? get() = SalesRange.parseTyped(fromDigits)

    /** The typed end date, or null while it is not a real date. */
    val typedTo: String? get() = SalesRange.parseTyped(toDigits)

    /** True when both typed dates are real and nothing is loading: the Show button works. */
    val canApply: Boolean get() = typedFrom != null && typedTo != null && !loading

    /** The by-product rows in the chosen order (the biggest first, the name breaking ties). */
    val products: List<SalesProduct>
        get() {
            val rows = report?.byProduct.orEmpty()
            return when (sort) {
                SalesSort.AMOUNT -> rows.sortedWith(compareByDescending<SalesProduct> { it.amountCents }.thenBy { it.name })
                SalesSort.QUANTITY -> rows.sortedWith(compareByDescending<SalesProduct> { it.quantityMilli }.thenBy { it.name })
            }
        }
}
