package dev.nuagemagique.pilipos.ui.components

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class PinPadStateTest {
    private fun PinPadState.type(text: String): String? {
        var out: String? = null
        text.forEach { out = apply(NumPadKey.Digit(it)) ?: out }
        return out
    }

    @Test
    fun verifyIsReadyOnExactlyFourDigits() {
        val state = PinPadState(PinMode.Verify)
        assertNull(state.type("123"))
        assertFalse(state.isReady)
        assertEquals("1234", state.apply(NumPadKey.Digit('4')))
        assertTrue(state.isReady)
        assertTrue(state.busy)
    }

    @Test
    fun aFifthDigitIsIgnored() {
        val state = PinPadState(PinMode.Verify)
        state.type("1234")
        state.failed("no")
        state.type("123")
        assertEquals("1234", state.apply(NumPadKey.Digit('4')))
        assertNull(state.apply(NumPadKey.Digit('9')))
        assertEquals("1234", state.digits)
    }

    @Test
    fun keysAreIgnoredWhileBusySoAPinIsNeverSubmittedTwice() {
        val state = PinPadState(PinMode.Verify)
        state.type("1234")
        assertNull(state.apply(NumPadKey.Backspace))
        assertNull(state.apply(NumPadKey.Clear))
        assertNull(state.apply(NumPadKey.Digit('1')))
        assertEquals("1234", state.digits)
    }

    @Test
    fun backspaceAndClearEditTheDigits() {
        val state = PinPadState(PinMode.Verify)
        state.type("12")
        state.apply(NumPadKey.Backspace)
        assertEquals("1", state.digits)
        state.type("9")
        assertEquals("19", state.digits)
        state.apply(NumPadKey.Clear)
        assertEquals("", state.digits)
        state.apply(NumPadKey.Backspace)
        assertEquals("", state.digits)
    }

    @Test
    fun verifyFailureShowsTheMessageAndClears() {
        val state = PinPadState(PinMode.Verify)
        state.type("1234")
        state.failed("PIN incorrect")
        assertEquals("PIN incorrect", state.error)
        assertEquals("", state.digits)
        assertFalse(state.busy)
        assertEquals(PinStep.Enter, state.step)
        state.type("1234")
        assertNull(state.error)
    }

    @Test
    fun successClearsAndFreesThePad() {
        val state = PinPadState(PinMode.Verify)
        state.type("1234")
        state.succeeded()
        assertEquals("", state.digits)
        assertFalse(state.busy)
        state.apply(NumPadKey.Digit('5'))
        assertEquals("5", state.digits)
    }

    @Test
    fun chooseGoesToConfirmThenSubmitsOnAMatch() {
        val state = PinPadState(PinMode.Choose, "mismatch")
        assertNull(state.type("4321"))
        assertEquals(PinStep.Confirm, state.step)
        assertEquals("", state.digits)
        assertFalse(state.isReady)
        assertEquals("4321", state.type("4321"))
        assertTrue(state.isReady)
        assertTrue(state.busy)
        assertNull(state.error)
    }

    @Test
    fun chooseMismatchSetsTheErrorAndStartsOverAtEnter() {
        val state = PinPadState(PinMode.Choose, "mismatch")
        state.type("4321")
        assertNull(state.type("4322"))
        assertEquals("mismatch", state.error)
        assertEquals(PinStep.Enter, state.step)
        assertEquals("", state.digits)
        assertFalse(state.busy)
        assertNull(state.type("1111"))
        assertNull(state.error)
        assertEquals(PinStep.Confirm, state.step)
        assertEquals("1111", state.type("1111"))
    }

    @Test
    fun chooseFailureRestartsAtEnter() {
        val state = PinPadState(PinMode.Choose, "mismatch")
        state.type("4321")
        state.type("4321")
        state.failed("boom")
        assertEquals("boom", state.error)
        assertEquals(PinStep.Enter, state.step)
        assertEquals("", state.digits)
        assertFalse(state.busy)
    }
}
