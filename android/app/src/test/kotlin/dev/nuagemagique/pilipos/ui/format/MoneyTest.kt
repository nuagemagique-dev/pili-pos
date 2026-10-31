package dev.nuagemagique.pilipos.ui.format

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The expected texts are what Node prints for `(c / 100).toLocaleString('fr-BE', { minimumFractionDigits: 2,
 * maximumFractionDigits: 2 })`, with the group separator U+202F written as an escape.
 */
class MoneyTest {
    @Test
    fun matchesTheNodeFrBeOutput() {
        val expected =
            mapOf(
                0L to "0,00",
                5L to "0,05",
                99L to "0,99",
                100L to "1,00",
                1250L to "12,50",
                -305L to "-3,05",
                99999L to "999,99",
                100000L to "1\u202F000,00",
                123456789L to "1\u202F234\u202F567,89",
            )
        expected.forEach { (cents, text) -> assertEquals("cents=$cents", text, formatCents(cents)) }
    }

    @Test
    fun aWeightPrintsThreeDecimalsAndItsUnit() {
        assertEquals("0,278 kg", formatWeight(278L))
        assertEquals("1,005 kg", formatWeight(1005L))
        assertEquals("2,000 kg", formatWeight(2000L))
    }

    @Test
    fun negativeAmountsKeepTheGroupingAndTheAsciiMinus() {
        assertEquals("-1\u202F234\u202F567,89", formatCents(-123456789L))
        assertEquals("-0,05", formatCents(-5L))
        assertEquals("-1,00", formatCents(-100L))
    }

    @Test
    fun theLargestLongIsExact() {
        assertEquals("92\u202F233\u202F720\u202F368\u202F547\u202F758,07", formatCents(Long.MAX_VALUE))
    }

    @Test
    fun theSmallestLongDoesNotCrashAndIsExact() {
        assertEquals("-92\u202F233\u202F720\u202F368\u202F547\u202F758,08", formatCents(Long.MIN_VALUE))
    }

    @Test
    fun theSeparatorIsTheNarrowNoBreakSpaceAndTheMinusIsAscii() {
        val text = formatCents(-100000L)
        assertEquals('-', text.first())
        assertTrue(text.contains('\u202F'))
        assertTrue(!text.contains(' '))
    }

    @Test
    fun formatEuroAppendsASpaceAndTheEuroSign() {
        assertEquals("12,50 €", formatEuro(1250L))
        assertEquals("0,00 €", formatEuro(0L))
        assertEquals("-3,05 €", formatEuro(-305L))
        assertEquals("1\u202F000,00 €", formatEuro(100000L))
    }

    @Test
    fun theDecimalSeparatorIsAComma() {
        assertEquals(2, formatCents(7L).length - formatCents(7L).indexOf(',') - 1)
        assertTrue(formatCents(7L).contains(','))
        assertTrue(!formatCents(7L).contains('.'))
    }
}
