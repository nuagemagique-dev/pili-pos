package dev.nuagemagique.pilipos.ui.state

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** The hint of the manager PIN the engine kept for an order (UX audit P8), as `pinGrant.test.ts`. */
class KeptPinTest {
    @Test
    fun noGrantAnsweredMeansNoHint() {
        assertNull(KeptPin.hintAfter(7L, null, 1_000L))
        assertNull(KeptPin.hintAfter(7L, 0L, 1_000L))
    }

    @Test
    fun theHintRunsFromTheAnswerForTheTimeTheEngineSaid() {
        assertEquals(PinGrantHint(7L, 121_000L), KeptPin.hintAfter(7L, 120_000L, 1_000L))
    }

    @Test
    fun itTriesWithoutThePinOnlyForTheSameOrderBeforeTheEnd() {
        val hint = KeptPin.hintAfter(7L, 120_000L, 0L)
        assertTrue(KeptPin.triesWithoutPin(hint, 7L, 0L))
        assertTrue(KeptPin.triesWithoutPin(hint, 7L, 119_999L))
        assertFalse(KeptPin.triesWithoutPin(hint, 7L, 120_000L))
        assertFalse(KeptPin.triesWithoutPin(hint, 8L, 1_000L))
        assertFalse(KeptPin.triesWithoutPin(hint, null, 1_000L))
        assertFalse(KeptPin.triesWithoutPin(null, 7L, 1_000L))
    }

    @Test
    fun theToastSaysWholeMinutesAtLeastOne() {
        assertEquals(2, KeptPin.minutesKept(120_000L))
        assertEquals(2, KeptPin.minutesKept(119_000L))
        assertEquals(1, KeptPin.minutesKept(60_000L))
        assertEquals(1, KeptPin.minutesKept(500L))
    }
}
