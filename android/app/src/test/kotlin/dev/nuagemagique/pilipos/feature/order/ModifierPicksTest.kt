package dev.nuagemagique.pilipos.feature.order

import androidx.compose.ui.graphics.Color
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** The pure rules of the options dialog and of the category colour. */
class ModifierPicksTest {
    private val single = group(1, "Sauce", min = 1, max = 1, modifier(1, "A"), modifier(2, "B"))
    private val upToTwo = group(2, "Extras", min = 0, max = 2, modifier(3, "C"), modifier(4, "D"), modifier(5, "E"))
    private val unlimited = group(3, "Libre", min = 2, max = null, modifier(6, "F"), modifier(7, "G"), modifier(8, "H"))
    private val range = group(4, "Plage", min = 1, max = 2, modifier(9, "I"), modifier(10, "J"), modifier(11, "K"))

    @Test
    fun aTapOnAPickedOptionRemovesIt() {
        val picks = ModifierPicks().toggle(upToTwo, 3).toggle(upToTwo, 3)

        assertEquals(emptySet<Long>(), picks.ids)
    }

    @Test
    fun aSingleChoiceReplacesTheOnePicked() {
        val picks = ModifierPicks().toggle(single, 1).toggle(single, 2)

        assertEquals(setOf(2L), picks.ids)
    }

    @Test
    fun aSingleChoiceKeepsThePicksOfOtherGroups() {
        val picks = ModifierPicks().toggle(upToTwo, 3).toggle(single, 1).toggle(single, 2)

        assertEquals(setOf(3L, 2L), picks.ids)
    }

    @Test
    fun aPickAboveTheMaximumIsRefused() {
        val picks = ModifierPicks().toggle(upToTwo, 3).toggle(upToTwo, 4).toggle(upToTwo, 5)

        assertEquals(setOf(3L, 4L), picks.ids)
    }

    @Test
    fun aGroupWithoutMaximumTakesEveryOption() {
        val picks = ModifierPicks().toggle(unlimited, 6).toggle(unlimited, 7).toggle(unlimited, 8)

        assertEquals(3, picks.countIn(unlimited))
    }

    @Test
    fun validNeedsTheMinimumOfEveryGroupAndStaysWithinTheMaximum() {
        assertFalse(ModifierPicks().isValid(listOf(single)))
        assertTrue(ModifierPicks().isValid(listOf(upToTwo)))
        assertFalse(ModifierPicks().toggle(unlimited, 6).isValid(listOf(unlimited)))
        assertTrue(ModifierPicks().toggle(unlimited, 6).toggle(unlimited, 7).isValid(listOf(unlimited)))
        assertTrue(ModifierPicks().toggle(range, 9).isValid(listOf(range)))
        assertTrue(ModifierPicks().toggle(single, 1).toggle(upToTwo, 3).isValid(listOf(single, upToTwo)))
    }

    @Test
    fun noGroupsIsValid() {
        assertTrue(ModifierPicks().isValid(emptyList()))
    }

    @Test
    fun aCategoryColourIsReadFromSixHexDigits() {
        assertEquals(Color(red = 229, green = 57, blue = 53), parseCategoryColor("#E53935"))
        assertEquals(Color(red = 0, green = 0, blue = 0), parseCategoryColor(" #000000 "))
    }

    @Test
    fun aMissingOrMalformedCategoryColourIsNull() {
        assertNull(parseCategoryColor(null))
        assertNull(parseCategoryColor(""))
        assertNull(parseCategoryColor("E53935"))
        assertNull(parseCategoryColor("#E5393"))
        assertNull(parseCategoryColor("#GG0000"))
    }
}
