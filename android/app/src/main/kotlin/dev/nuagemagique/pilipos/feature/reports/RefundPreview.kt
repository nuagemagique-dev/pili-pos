package dev.nuagemagique.pilipos.feature.reports

import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.ui.format.CashDisplay
import dev.nuagemagique.pilipos.ui.format.LineAmount
import kotlinx.coroutines.CancellationException
import uniffi.pos_ffi.OrderLine
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.VatLine
import uniffi.pos_ffi.VatSplitRow

/** A ticked refund line with the amount the engine gives it for the quantity that is ticked. */
internal data class PreviewLine(
    val selection: RefundSelection,
    val amount: LineAmount,
)

/**
 * What the refund confirmation shows for the ticked lines (UI-PD307). Nothing here is computed in Kotlin: every
 * amount is an engine value (`vatSplit`, `cashDueCents`) or the display sum of engine values
 * ([CashDisplay.sumCents]).
 *
 * [totalCents] and [cashCents] are null when no preview total can be shown: nothing is ticked, a ticked line has
 * VAT code X (the split leaves it out, so its amount is not known here), or the helpers failed. The refund
 * result of the engine then gives the amount.
 */
internal data class RefundPreview(
    /** The ticked lines, in the order they were ticked, each with its engine amount. */
    val lines: List<PreviewLine> = emptyList(),
    /** The VAT summary of the ticked lines: the `vatSplit` rows of the selection. */
    val vatRows: List<VatSplitRow> = emptyList(),
    /** The sum of the line totals ([CashDisplay.sumCents]), or null when no preview total is shown. */
    val totalCents: Long? = null,
    /** The cash to pay back: [totalCents] through `EngineHelpers.cashDueCents` (the engine's 5-cent rounding). */
    val cashCents: Long? = null,
) {
    /** True when a line is ticked but its amount is not known here (VAT code X): the screen explains why. */
    val totalUnavailable: Boolean get() = lines.isNotEmpty() && cashCents == null
}

/** The VAT lines of [selected]: the unit price of the line and the quantity that is ticked. */
private fun vatLinesOf(selected: List<RefundSelection>): List<VatLine> =
    selected.map { VatLine(it.line.vatCode, it.line.unitPriceCents, it.quantity.toLong()) }

/**
 * The amount of one line for [quantity] units: the single `vatSplit` row of that line, as
 * `CartAmounts.lineAmount` does. A line the split leaves out (code X) shows quantity and unit price instead.
 */
private suspend fun amountOf(
    helpers: EngineHelpers,
    vatCode: VatCode,
    unitPriceCents: Long,
    quantity: Int,
): LineAmount {
    val row = helpers.vatSplit(listOf(VatLine(vatCode, unitPriceCents, quantity.toLong()))).firstOrNull()
    return if (row != null) LineAmount.Total(row.totalInclCents) else LineAmount.PerUnit(quantity, unitPriceCents)
}

/**
 * The amount of every line of the sale for its full quantity, by line id (the list of the lines step). A failure
 * of the helpers gives the amounts it could not read as missing.
 */
internal suspend fun lineAmounts(
    helpers: EngineHelpers,
    lines: List<OrderLine>,
): Map<Long, LineAmount> {
    val amounts = LinkedHashMap<Long, LineAmount>()
    for (line in lines) {
        val amount = attemptOrNull { amountOf(helpers, line.vatCode, line.unitPriceCents, line.quantity) }
        if (amount != null) amounts[line.id] = amount
    }
    return amounts
}

/**
 * The preview of a refund of [selected]: the engine amount of each ticked line, the VAT summary of the
 * selection, and, when every line has a known amount, the sum of the line totals ([CashDisplay.sumCents]) and
 * the cash to pay back (`EngineHelpers.cashDueCents` of that sum with a single cash part).
 *
 * A failure of the helpers gives a preview without total instead of an exception: the refund itself is signed by
 * the engine and does not depend on the preview.
 */
internal suspend fun refundPreview(
    helpers: EngineHelpers,
    selected: List<RefundSelection>,
): RefundPreview {
    if (selected.isEmpty()) return RefundPreview()
    return attemptOrNull { buildPreview(helpers, selected) } ?: RefundPreview()
}

private suspend fun buildPreview(
    helpers: EngineHelpers,
    selected: List<RefundSelection>,
): RefundPreview {
    val lines = selected.map { PreviewLine(it, amountOf(helpers, it.line.vatCode, it.line.unitPriceCents, it.quantity)) }
    val vatRows = helpers.vatSplit(vatLinesOf(selected))
    val totals = lines.mapNotNull { (it.amount as? LineAmount.Total)?.cents }
    // A line with VAT code X has no row: no total is shown, the engine's refund result gives the amount.
    if (totals.size != lines.size) return RefundPreview(lines, vatRows)
    val total = CashDisplay.sumCents(totals)
    val cash = helpers.cashDueCents(total, listOf(PaymentPart(PaymentMethod.CASH, null)))
    return RefundPreview(lines, vatRows, total, cash)
}

/** The answer of [block], or null when it failed (a cancellation is rethrown). */
private suspend fun <T> attemptOrNull(block: suspend () -> T): T? = attempt(block).getOrNull()

/** Runs [block]; the answer or the failure it threw (a cancellation is rethrown). */
@Suppress("TooGenericExceptionCaught")
private suspend fun <T> attempt(block: suspend () -> T): Result<T> =
    try {
        Result.success(block())
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        Result.failure(e)
    }
