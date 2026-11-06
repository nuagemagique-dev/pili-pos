package dev.nuagemagique.pilipos.feature.checkout

import dev.nuagemagique.pilipos.ui.state.HeldPart
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.pos_ffi.PaymentMethod

/** `reconcileHeld` of `utils/heldParts.ts` (UX audit P4). */
class HeldPartsRulesTest {
    private fun cash(cents: Long) = HeldPart(PaymentMethod.CASH, cents, 0L)

    @Test
    fun partsThatFitAreKept() {
        val parts = listOf(cash(500), cash(700))

        val result = reconcileHeld(parts, totalCents = 2_000L, lockedCents = 300L)

        assertEquals(parts, result.parts)
        assertFalse(result.dropped)
    }

    @Test
    fun partsThatFillTheTotalExactlyAreKept() {
        val result = reconcileHeld(listOf(cash(1_200), cash(800)), totalCents = 2_000L, lockedCents = 0L)

        assertEquals(2, result.parts.size)
        assertFalse(result.dropped)
    }

    @Test
    fun oneOverTheRestDropsEveryPartTakenByHand() {
        val result = reconcileHeld(listOf(cash(500), cash(1_600)), totalCents = 2_000L, lockedCents = 0L)

        assertTrue(result.parts.isEmpty())
        assertTrue(result.dropped)
    }

    @Test
    fun theApprovedCardPartsCountAgainstTheTotal() {
        val result = reconcileHeld(listOf(cash(900)), totalCents = 2_000L, lockedCents = 1_200L)

        assertTrue(result.dropped)
    }

    @Test
    fun noPartsIsNeverADrop() {
        val result = reconcileHeld(emptyList(), totalCents = 0L, lockedCents = 0L)

        assertTrue(result.parts.isEmpty())
        assertFalse(result.dropped)
    }
}
