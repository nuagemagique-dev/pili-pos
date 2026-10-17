package dev.nuagemagique.pilipos.ui.format

import dev.nuagemagique.pilipos.engine.EngineHelpers
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.VatLine
import uniffi.pos_ffi.VatSplitRow
import javax.inject.Inject
import javax.inject.Singleton

/** What a cart line shows as its price. */
sealed interface LineAmount {
    /** The total of the line, VAT included, as the engine computed it. */
    data class Total(
        val cents: Long,
    ) : LineAmount

    /** A line the VAT split leaves out (code X): its unit price and quantity are shown, no product is made (PD208). */
    data class PerUnit(
        val quantity: Int,
        val unitCents: Long,
    ) : LineAmount
}

/**
 * The amounts of a cart. The UI computes no money (PD208): the VAT rows and the line totals come from the engine
 * helper `vatSplit`, and nothing here adds, multiplies or divides a cents value.
 */
@Singleton
class CartAmounts
    @Inject
    constructor(
        private val helpers: EngineHelpers,
    ) {
        /** The VAT summary rows of [lines], in the engine order (A, B, C, D, only the codes present). */
        suspend fun vatRows(lines: List<CartLine>): List<VatSplitRow> = helpers.vatSplit(lines.flatMap(::vatLines))

        /** The net price of [line] (its discounts taken off): [LineAmount.Total] from a one-line VAT split, [LineAmount.PerUnit] when it gives no row. */
        suspend fun lineAmount(line: CartLine): LineAmount {
            val row = helpers.vatSplit(vatLines(line)).firstOrNull()
            return if (row != null) {
                LineAmount.Total(row.totalInclCents)
            } else {
                LineAmount.PerUnit(line.quantity, line.unitPriceCents)
            }
        }

        private fun vatLine(line: CartLine): VatLine = VatLine(line.vatCode, line.unitPriceCents, line.quantity.toLong())

        /**
         * The line, and its price changes as one more line of quantity 1 and a negative unit price (a till discount
         * is a negative amount of the line's VAT code), so the split gives the net amount without any arithmetic here.
         */
        private fun vatLines(line: CartLine): List<VatLine> =
            if (line.discountCents == 0L) {
                listOf(vatLine(line))
            } else {
                listOf(vatLine(line), VatLine(line.vatCode, line.discountCents, 1L))
            }
    }
