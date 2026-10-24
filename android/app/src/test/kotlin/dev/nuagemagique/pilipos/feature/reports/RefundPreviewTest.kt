package dev.nuagemagique.pilipos.feature.reports

import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.engine.EngineNatives
import dev.nuagemagique.pilipos.engine.FakeNatives
import dev.nuagemagique.pilipos.ui.format.LineAmount
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.pos_ffi.OrderLine
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.VatLine
import uniffi.pos_ffi.VatSplitRow

/** The engine seams of the refund preview (UI-PD307): the rows of `vatSplit` and the rounding of `cashDueCents`. */
private class PreviewNatives : EngineNatives by FakeNatives() {
    val splits = mutableListOf<List<VatLine>>()
    val dues = mutableListOf<Pair<Long, List<PaymentPart>>>()
    var failure: Throwable? = null

    /** A fake engine: one row per VAT code that has a rate (not X). */
    override suspend fun vatSplit(lines: List<VatLine>): List<VatSplitRow> {
        splits += lines
        failure?.let { throw it }
        return lines
            .filter { it.vatCode != VatCode.X }
            .groupBy { it.vatCode }
            .map { (code, group) -> VatSplitRow(code, RATE, group.sumOf { it.unitPriceCents * it.quantity }, VAT) }
    }

    /** A fake engine rounding: 5 cents less than the total, so that a test can tell the engine's answer from the sum. */
    override suspend fun cashDueCents(
        totalCents: Long,
        parts: List<PaymentPart>,
    ): Long {
        dues += totalCents to parts
        return totalCents - ROUNDING
    }

    private companion object {
        const val RATE = 12L
        const val VAT = 1L
        const val ROUNDING = 5L
    }
}

@OptIn(ExperimentalCoroutinesApi::class)
class RefundPreviewTest {
    private val natives = PreviewNatives()

    private fun TestScope.helpers() = EngineHelpers(natives, UnconfinedTestDispatcher(testScheduler))

    private fun line(
        id: Long,
        quantity: Int,
        unitPriceCents: Long,
        vat: VatCode = VatCode.B,
    ): OrderLine =
        orderLine(id, quantity).also {
            it.unitPriceCents = unitPriceCents
            it.vatCode = vat
        }

    private fun selection(
        line: OrderLine,
        quantity: Int,
    ) = RefundSelection(line, quantity)

    @Test
    fun theLineAmountIsTheSingleVatSplitRowOfThatLineForTheTickedQuantity() =
        runTest {
            val preview = refundPreview(helpers(), listOf(selection(line(1L, 3, 500L), 2)))

            assertEquals(listOf<LineAmount>(LineAmount.Total(1_000L)), preview.lines.map { it.amount })
            assertEquals(listOf(VatLine(VatCode.B, 500L, 2L)), natives.splits.first())
        }

    @Test
    fun theTotalIsTheSumOfTheEngineLineTotalsAndTheCashAmountIsTheEnginesRounding() =
        runTest {
            val selected = listOf(selection(line(1L, 3, 500L), 2), selection(line(2L, 1, 450L), 1))

            val preview = refundPreview(helpers(), selected)

            assertEquals(1_450L, preview.totalCents)
            // 1 445 is what the fake engine's cashDueCents answers: the screen shows it as it is.
            assertEquals(1_445L, preview.cashCents)
            assertEquals(listOf(1_450L to listOf(PaymentPart(PaymentMethod.CASH, null))), natives.dues)
            assertFalse(preview.totalUnavailable)
        }

    @Test
    fun theVatSummaryIsTheVatSplitOfTheWholeSelection() =
        runTest {
            val selected = listOf(selection(line(1L, 3, 500L), 2), selection(line(2L, 1, 450L), 1))

            val preview = refundPreview(helpers(), selected)

            assertEquals(listOf(VatLine(VatCode.B, 500L, 2L), VatLine(VatCode.B, 450L, 1L)), natives.splits.last())
            assertEquals(listOf(VatCode.B), preview.vatRows.map { it.vatCode })
            assertEquals(1_450L, preview.vatRows.single().totalInclCents)
        }

    @Test
    fun aLineWithVatCodeXShowsItsQuantityAndUnitPriceAndNoTotalIsShown() =
        runTest {
            val preview = refundPreview(helpers(), listOf(selection(line(1L, 4, 300L, VatCode.X), 3)))

            assertEquals(listOf<LineAmount>(LineAmount.PerUnit(3, 300L)), preview.lines.map { it.amount })
            assertNull(preview.totalCents)
            assertNull(preview.cashCents)
            assertTrue(preview.vatRows.isEmpty())
            assertTrue(preview.totalUnavailable)
            // The rounding is never asked for a total that is not shown.
            assertTrue(natives.dues.isEmpty())
        }

    @Test
    fun oneLineWithVatCodeXAmongOthersAlsoHidesTheTotal() =
        runTest {
            val selected = listOf(selection(line(1L, 3, 500L), 2), selection(line(2L, 1, 300L, VatCode.X), 1))

            val preview = refundPreview(helpers(), selected)

            assertEquals(listOf<LineAmount>(LineAmount.Total(1_000L), LineAmount.PerUnit(1, 300L)), preview.lines.map { it.amount })
            assertNull(preview.totalCents)
            assertNull(preview.cashCents)
            assertEquals(1, preview.vatRows.size)
            assertTrue(natives.dues.isEmpty())
        }

    @Test
    fun nothingTickedGivesAnEmptyPreviewWithoutAskingTheEngine() =
        runTest {
            val preview = refundPreview(helpers(), emptyList())

            assertEquals(RefundPreview(), preview)
            assertFalse(preview.totalUnavailable)
            assertTrue(natives.splits.isEmpty())
        }

    @Test
    fun aFailingHelperGivesAPreviewWithoutTotalInsteadOfAnException() =
        runTest {
            natives.failure = IllegalStateException("ffi down")

            val preview = refundPreview(helpers(), listOf(selection(line(1L, 1, 500L), 1)))

            assertEquals(RefundPreview(), preview)
        }

    @Test
    fun aSumThatOverflowsGivesAPreviewWithoutTotal() =
        runTest {
            val huge = Long.MAX_VALUE
            val selected = listOf(selection(line(1L, 1, huge), 1), selection(line(2L, 1, huge), 1))

            val preview = refundPreview(helpers(), selected)

            assertEquals(RefundPreview(), preview)
            assertTrue(natives.dues.isEmpty())
        }

    @Test
    fun everyLineOfTheSaleGetsItsAmountForItsFullQuantity() =
        runTest {
            val lines = listOf(line(1L, 3, 500L), line(2L, 2, 300L, VatCode.X))

            val amounts = lineAmounts(helpers(), lines)

            assertEquals(mapOf<Long, LineAmount>(1L to LineAmount.Total(1_500L), 2L to LineAmount.PerUnit(2, 300L)), amounts)
        }

    @Test
    fun aFailingHelperLeavesTheLineAmountsOut() =
        runTest {
            natives.failure = IllegalStateException("ffi down")

            assertTrue(lineAmounts(helpers(), listOf(line(1L, 3, 500L))).isEmpty())
        }
}
