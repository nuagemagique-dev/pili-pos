package dev.nuagemagique.pilipos.ui.format

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import uniffi.pos_ffi.DiscountKind

class CashDisplayTest {
    @Test
    fun changeIsWhatExceedsTheDue() {
        assertEquals(250L, CashDisplay.changeCents(tenderedCents = 2000L, dueCents = 1750L))
        assertEquals(1L, CashDisplay.changeCents(tenderedCents = 1001L, dueCents = 1000L))
    }

    @Test
    fun changeIsZeroWhenExactOrShort() {
        assertEquals(0L, CashDisplay.changeCents(tenderedCents = 1750L, dueCents = 1750L))
        assertEquals(0L, CashDisplay.changeCents(tenderedCents = 500L, dueCents = 1750L))
        assertEquals(0L, CashDisplay.changeCents(tenderedCents = 0L, dueCents = 0L))
    }

    @Test
    fun theVueExampleSplitsSixtyThirtyInFour() {
        assertEquals(listOf(1508L, 1508L, 1507L, 1507L), CashDisplay.equalShares(6030L, 4))
    }

    @Test
    fun sharesAlwaysAddUpToTheTotal() {
        for (total in listOf(0L, 1L, 2L, 99L, 6030L, 12345L)) {
            for (parts in 1..7) {
                val shares = CashDisplay.equalShares(total, parts)
                assertEquals(parts, shares.size)
                assertEquals(total, shares.sum())
                assertEquals(true, shares.max() - shares.min() <= 1L)
                assertEquals(shares.sortedDescending(), shares)
            }
        }
    }

    @Test
    fun aSingleShareIsTheWholeTotal() {
        assertEquals(listOf(6030L), CashDisplay.equalShares(6030L, 1))
    }

    @Test
    fun aZeroTotalGivesZeroShares() {
        assertEquals(listOf(0L, 0L, 0L), CashDisplay.equalShares(0L, 3))
    }

    @Test
    fun moreShareThanCentsGivesSomeZeroShares() {
        assertEquals(listOf(1L, 1L, 0L, 0L), CashDisplay.equalShares(2L, 4))
    }

    @Test
    fun sharesRefuseNoPartsAndANegativeTotal() {
        assertThrows(IllegalArgumentException::class.java) { CashDisplay.equalShares(100L, 0) }
        assertThrows(IllegalArgumentException::class.java) { CashDisplay.equalShares(-1L, 2) }
    }

    @Test
    fun quickAmountsOfAnOddDue() {
        // Vue: due 12,35 -> ceil to 15 and 20, then 20 again (dropped), 50, 100.
        assertEquals(listOf(1500L, 2000L, 5000L), CashDisplay.quickAmountsCents(1235L))
    }

    @Test
    fun quickAmountsAtAnExactMultipleSkipTheDueItself() {
        // Vue: due 10,00 -> 10 and 10 are not above it, so 20, 50, 100.
        assertEquals(listOf(2000L, 5000L, 10000L), CashDisplay.quickAmountsCents(1000L))
        // Due 5,00 -> 5 is not above it; 10, 20, 50.
        assertEquals(listOf(1000L, 2000L, 5000L), CashDisplay.quickAmountsCents(500L))
    }

    @Test
    fun quickAmountsAboveTheSmallNotes() {
        assertEquals(listOf(1000L, 2000L, 5000L), CashDisplay.quickAmountsCents(505L))
        assertEquals(listOf(5000L, 10000L, 20000L), CashDisplay.quickAmountsCents(4500L))
        assertEquals(listOf(50000L), CashDisplay.quickAmountsCents(49999L))
    }

    @Test
    fun quickAmountsOfAZeroDue() {
        assertEquals(listOf(2000L, 5000L, 10000L), CashDisplay.quickAmountsCents(0L))
    }

    @Test
    fun quickAmountsAboveTheLargestNoteAreEmpty() {
        assertEquals(emptyList<Long>(), CashDisplay.quickAmountsCents(50000L))
        assertEquals(emptyList<Long>(), CashDisplay.quickAmountsCents(75000L))
    }

    @Test
    fun quickAmountsRefuseANegativeDue() {
        assertThrows(IllegalArgumentException::class.java) { CashDisplay.quickAmountsCents(-5L) }
    }

    @Test
    fun sumOfTheLineTotals() {
        assertEquals(0L, CashDisplay.sumCents(emptyList()))
        assertEquals(1250L, CashDisplay.sumCents(listOf(1250L)))
        assertEquals(3060L, CashDisplay.sumCents(listOf(1508L, 1508L, 44L)))
        assertEquals(-250L, CashDisplay.sumCents(listOf(500L, -750L)))
    }

    @Test
    fun sumFailsOnOverflowInsteadOfWrapping() {
        assertThrows(ArithmeticException::class.java) { CashDisplay.sumCents(listOf(Long.MAX_VALUE, 1L)) }
    }

    @Test
    fun aPercentIsRoundedHalfUp() {
        assertEquals(2600L, CashDisplay.totalAfterDiscountCents(5200L, DiscountKind.PERCENT, 50L))
        // 10 % of 5,05 is 50,5 cents: rounds to 51.
        assertEquals(454L, CashDisplay.totalAfterDiscountCents(505L, DiscountKind.PERCENT, 10L))
        // 5 % of 0,10 is 0,5 cent: rounds up to 1.
        assertEquals(9L, CashDisplay.totalAfterDiscountCents(10L, DiscountKind.PERCENT, 5L))
        // 15 % of 0,03 is 0,45 cent: rounds down to 0.
        assertEquals(3L, CashDisplay.totalAfterDiscountCents(3L, DiscountKind.PERCENT, 15L))
    }

    @Test
    fun anAmountIsTakenOffAndOfferedGivesZero() {
        assertEquals(1900L, CashDisplay.totalAfterDiscountCents(2400L, DiscountKind.AMOUNT, 500L))
        assertEquals(0L, CashDisplay.totalAfterDiscountCents(2400L, DiscountKind.AMOUNT, 2400L))
        assertEquals(0L, CashDisplay.totalAfterDiscountCents(2400L, DiscountKind.FREE, 0L))
    }

    @Test
    fun aValueOutOfRangeLeavesTheTotal() {
        assertEquals(2400L, CashDisplay.totalAfterDiscountCents(2400L, DiscountKind.PERCENT, 0L))
        assertEquals(2400L, CashDisplay.totalAfterDiscountCents(2400L, DiscountKind.PERCENT, 100L))
        assertEquals(2400L, CashDisplay.totalAfterDiscountCents(2400L, DiscountKind.AMOUNT, 2401L))
        assertEquals(2400L, CashDisplay.totalAfterDiscountCents(2400L, DiscountKind.AMOUNT, 0L))
    }

    @Test
    fun theSubtotalPutsTheDiscountBack() {
        assertEquals(2400L, CashDisplay.subtotalBeforeDiscountCents(1200L, -1200L))
        assertEquals(1200L, CashDisplay.subtotalBeforeDiscountCents(1200L, 0L))
    }
}
