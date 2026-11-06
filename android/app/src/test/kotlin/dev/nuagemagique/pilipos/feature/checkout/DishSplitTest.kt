package dev.nuagemagique.pilipos.feature.checkout

import dev.nuagemagique.pilipos.ui.state.DishUnits
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.pos_ffi.PaymentMethod

/** `utils/dishSplit.ts` of the Vue UI (UX audit P3). */
class DishSplitTest {
    // Three Sushi Mix (29,99 net after a 0,01 discount), a soup of 4,60, two offered teas.
    private val lines = listOf(DishLine(1, 3, 2_999L), DishLine(2, 1, 460L), DishLine(3, 2, 0L))

    private fun part(vararg units: DishUnits) = TakenPart(PaymentMethod.CARD, 0L, 0L, units = units.toList())

    @Test
    fun theLinesCarryTheirNetAfterTheDiscounts() {
        val cart =
            listOf(
                cartLine(1, quantity = 3, unitPriceCents = 1_000L, discountCents = -1L),
                cartLine(3, quantity = 2, unitPriceCents = 250L, discountCents = -500L),
            )

        assertEquals(listOf(DishLine(1, 3, 2_999L), DishLine(3, 2, 0L)), dishLinesOf(cart))
    }

    @Test
    fun theUnitsCheckedAreAddedUpLineByLine() {
        assertEquals(1_460L, pickedCents(lines, emptyMap(), mapOf(1L to 1, 2L to 1)))
        assertEquals(2_999L, pickedCents(lines, emptyMap(), mapOf(1L to 3)))
    }

    @Test
    fun theUnitsPaidAreNotTakenAgain() {
        val paid = paidUnits(listOf(part(DishUnits(1, 1))))

        assertEquals(2, unitsLeft(lines[0], paid))
        assertEquals(999L + 1_000L, pickedCents(lines, paid, mapOf(1L to 2)))
        assertEquals(1_000L, pickedCents(lines, paid, mapOf(1L to 1)))
    }

    @Test
    fun everyPartTogetherPaysTheTotalToTheCent() {
        val picks = listOf(mapOf(1L to 1), mapOf(1L to 1), mapOf(1L to 1, 2L to 1, 3L to 2))
        val taken = mutableListOf<TakenPart>()
        val sums = mutableListOf<Long>()
        for (pick in picks) {
            sums += pickedCents(lines, paidUnits(taken), pick)
            taken += part(*pickedUnits(lines, pick).toTypedArray())
        }

        assertEquals(2_999L + 460L, sums.sum())
    }

    @Test
    fun aCheckIsClampedToTheUnitsLeftAndAnEmptyOneIsRemoved() {
        assertEquals(mapOf(1L to 1), withUnits(emptyMap(), 1, left = 1, count = 3))
        assertEquals(emptyMap<Long, Int>(), withUnits(mapOf(1L to 1), 1, left = 1, count = 0))
        assertEquals(emptyMap<Long, Int>(), withUnits(emptyMap(), 1, left = 3, count = -2))
    }

    @Test
    fun aTapChecksEveryUnitLeftThenClearsThem() {
        val once = toggledUnits(emptyMap(), 1, left = 3)
        assertEquals(mapOf(1L to 3), once)
        assertEquals(emptyMap<Long, Int>(), toggledUnits(once, 1, left = 3))
        assertEquals(mapOf(1L to 3), toggledUnits(mapOf(1L to 1), 1, left = 3))
    }

    @Test
    fun itKnowsWhenEveryUnitLeftIsChecked() {
        assertTrue(allLeftPicked(lines, emptyMap(), mapOf(1L to 3, 2L to 1, 3L to 2)))
        assertFalse(allLeftPicked(lines, emptyMap(), mapOf(1L to 3, 2L to 1)))
        val paid = paidUnits(listOf(part(DishUnits(1, 3))))
        assertTrue(allLeftPicked(lines, paid, mapOf(2L to 1, 3L to 2)))
    }

    @Test
    fun theUnitsCheckedAreListedInCartOrder() {
        assertEquals(listOf(DishUnits(1, 2), DishUnits(2, 1)), pickedUnits(lines, mapOf(2L to 1, 1L to 2)))
    }

    @Test
    fun keptPartsStillFitOnlyWhenTheirLinesAndUnitsAreStillInTheCart() {
        assertTrue(unitsFit(lines, listOf(listOf(DishUnits(1, 2)), listOf(DishUnits(1, 1)))))
        assertFalse(unitsFit(lines, listOf(listOf(DishUnits(9, 1)))))
        assertFalse(unitsFit(lines, listOf(listOf(DishUnits(1, 2)), listOf(DishUnits(1, 2)))))
    }
}
