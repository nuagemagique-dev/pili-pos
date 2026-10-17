package dev.nuagemagique.pilipos.ui.format

import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.engine.EngineNatives
import dev.nuagemagique.pilipos.engine.FakeNatives
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Test
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.VatLine
import uniffi.pos_ffi.VatSplitRow

@OptIn(ExperimentalCoroutinesApi::class)
class CartAmountsTest {
    private val received = mutableListOf<List<VatLine>>()

    private fun amounts(
        scope: kotlinx.coroutines.test.TestScope,
        rows: List<VatSplitRow>,
    ): CartAmounts {
        val natives =
            object : EngineNatives by FakeNatives() {
                override suspend fun vatSplit(lines: List<VatLine>): List<VatSplitRow> {
                    received += lines
                    return rows
                }
            }
        return CartAmounts(EngineHelpers(natives, UnconfinedTestDispatcher(scope.testScheduler)))
    }

    private fun line(
        id: Long,
        quantity: Int,
        unitPriceCents: Long,
        vat: VatCode,
        discountCents: Long = 0,
    ): CartLine =
        CartLine(
            id = id,
            orderId = 1,
            menuItemId = 3,
            quantity = quantity,
            unitPriceCents = unitPriceCents,
            vatCode = vat,
            name = "Sushi",
            modifiersJson = null,
            createdAt = "2026-10-09 10:00:00",
            departmentId = "d",
            departmentName = "Dept",
            nameFr = null,
            nameZh = null,
            code = null,
            unsentQuantity = 0,
            discountCents = discountCents,
            priceChanges = emptyList(),
            kitchenNote = null,
        )

    @Test
    fun vatRowsPassesEveryLineUnchangedAndReturnsTheEngineRows() =
        runTest {
            val rows = listOf(VatSplitRow(VatCode.B, 12, 1250, 134), VatSplitRow(VatCode.C, 6, 500, 28))
            val lines = listOf(line(1, 2, 500, VatCode.B), line(2, 1, 250, VatCode.C))

            val result = amounts(this, rows).vatRows(lines)

            assertEquals(rows, result)
            assertEquals(listOf(listOf(VatLine(VatCode.B, 500, 2), VatLine(VatCode.C, 250, 1))), received)
        }

    @Test
    fun vatRowsOfNoLinesAsksTheEngineWithNoLine() =
        runTest {
            assertEquals(emptyList<VatSplitRow>(), amounts(this, emptyList()).vatRows(emptyList()))
            assertEquals(listOf(emptyList<VatLine>()), received)
        }

    @Test
    fun oneRowGivesTheTotalOfThatRow() =
        runTest {
            val row = VatSplitRow(VatCode.A, 21, 1500, 260)

            val amount = amounts(this, listOf(row)).lineAmount(line(1, 3, 500, VatCode.A))

            assertEquals(LineAmount.Total(1500), amount)
            assertEquals(listOf(listOf(VatLine(VatCode.A, 500, 3))), received)
        }

    @Test
    fun noRowGivesThePriceOfOneUnitAndTheQuantity() =
        runTest {
            val amount = amounts(this, emptyList()).lineAmount(line(1, 4, 750, VatCode.X))

            assertEquals(LineAmount.PerUnit(quantity = 4, unitCents = 750), amount)
            assertEquals(listOf(listOf(VatLine(VatCode.X, 750, 4))), received)
        }

    @Test
    fun aDiscountIsSentAsOneMoreNegativeLineOfTheSameVatCode() =
        runTest {
            amounts(this, listOf(VatSplitRow(VatCode.A, 21, 1000, 174))).lineAmount(line(1, 2, 1500, VatCode.A, discountCents = -2000))

            assertEquals(listOf(listOf(VatLine(VatCode.A, 1500, 2), VatLine(VatCode.A, -2000, 1))), received)
        }

    @Test
    fun theVatRowsOfDiscountedLinesCarryTheirDiscountsToo() =
        runTest {
            val lines = listOf(line(1, 1, 1000, VatCode.B, discountCents = -500), line(2, 3, 200, VatCode.C))

            amounts(this, emptyList()).vatRows(lines)

            assertEquals(
                listOf(listOf(VatLine(VatCode.B, 1000, 1), VatLine(VatCode.B, -500, 1), VatLine(VatCode.C, 200, 3))),
                received,
            )
        }
}
