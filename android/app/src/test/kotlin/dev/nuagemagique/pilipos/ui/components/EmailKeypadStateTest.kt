package dev.nuagemagique.pilipos.ui.components

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class EmailKeypadStateTest {
    private fun typed(text: String): EmailKeypadState = EmailKeypadState().append(text)

    @Test
    fun startsEmptyAndTyping() {
        val state = EmailKeypadState()
        assertEquals("", state.buffer)
        assertEquals(EmailKeypadStep.Type, state.step)
        assertFalse(state.isValid)
        assertFalse(state.showsError)
    }

    @Test
    fun validatesTheShapeOfVue() {
        assertTrue(typed("a@b.be").isValid)
        assertTrue(typed("first.last+tag@sub.example.com").isValid)
        assertFalse(typed("a").isValid)
        assertFalse(typed("a@b").isValid)
        assertFalse(typed("a@b.").isValid)
        assertFalse(typed("@b.be").isValid)
        assertFalse(typed("a@@b.be").isValid)
        assertFalse(typed("a b@c.be").isValid)
    }

    @Test
    fun theErrorShowsOnlyForSomethingTypedThatIsNotValid() {
        assertFalse(typed("").showsError)
        assertTrue(typed("abc").showsError)
        assertFalse(typed("a@b.be").showsError)
    }

    @Test
    fun appendStopsAtSixtyCharacters() {
        val full = typed("a".repeat(EMAIL_MAX_LENGTH))
        assertEquals(EMAIL_MAX_LENGTH, full.buffer.length)
        assertEquals(full, full.append("b"))
        // A chunk that would cross the limit is refused whole, one that reaches it exactly is taken.
        val nearly = typed("a".repeat(EMAIL_MAX_LENGTH - 2))
        assertEquals(nearly, nearly.append(".com"))
        assertEquals(EMAIL_MAX_LENGTH, nearly.append(".c").buffer.length)
    }

    @Test
    fun backspaceAndClearEditTheBuffer() {
        val state = typed("ab@c.be")
        assertEquals("ab@c.b", state.backspace().buffer)
        assertEquals("", state.clear().buffer)
        assertEquals("", EmailKeypadState().backspace().buffer)
    }

    @Test
    fun confirmStepNeedsAValidAddress() {
        assertEquals(EmailKeypadStep.Type, typed("abc").goConfirm().step)
        val confirm = typed("a@b.be").goConfirm()
        assertEquals(EmailKeypadStep.Confirm, confirm.step)
        assertEquals("a@b.be", confirm.buffer)
    }

    @Test
    fun editGoesBackToTypingKeepingTheBuffer() {
        val back = typed("a@b.be").goConfirm().goBackToType()
        assertEquals(EmailKeypadStep.Type, back.step)
        assertEquals("a@b.be", back.buffer)
    }
}
