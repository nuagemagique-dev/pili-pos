package dev.nuagemagique.pilipos.ui.components

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AmountDialogEntryStateTest {
    private fun type(vararg keys: NumPadKey): AmountEntryState = keys.fold(AmountEntryState()) { s, k -> s.press(k) }

    private fun d(c: Char) = NumPadKey.Digit(c)

    @Test
    fun digitsAreReadAsCentsLikeAPaymentTerminal() {
        assertEquals(2000L, type(d('2'), d('0'), d('0'), d('0')).cents)
        assertEquals(5L, type(d('5')).cents)
        assertEquals(0L, AmountEntryState().cents)
    }

    @Test
    fun noLeadingZero() {
        assertEquals("", type(d('0'), d('0')).digits)
        assertEquals("105", type(d('0'), d('1'), d('0'), d('5')).digits)
    }

    @Test
    fun atMostSixDigits() {
        val state = type(d('1'), d('2'), d('3'), d('4'), d('5'), d('6'), d('7'), d('8'))
        assertEquals("123456", state.digits)
        assertEquals(123456L, state.cents)
    }

    @Test
    fun backspaceErasesTheLastDigitAndClearAllOfThem() {
        assertEquals("12", type(d('1'), d('2'), d('3'), NumPadKey.Backspace).digits)
        assertEquals("", type(NumPadKey.Backspace).digits)
        assertEquals("", type(d('1'), d('2'), NumPadKey.Clear).digits)
    }

    @Test
    fun validWhenAboveZeroOrZeroWithAllowZero() {
        assertFalse(AmountEntryState().isValid(allowZero = false))
        assertTrue(AmountEntryState().isValid(allowZero = true))
        assertTrue(type(d('1')).isValid(allowZero = false))
        assertTrue(type(d('1')).isValid(allowZero = true))
    }
}
