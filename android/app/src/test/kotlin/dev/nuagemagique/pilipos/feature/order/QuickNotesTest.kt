package dev.nuagemagique.pilipos.feature.order

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** The rules of the quick note chips, the same as `utils/quickNotes.ts`. */
class QuickNotesTest {
    @Test
    fun aChipGoesAloneOnAnEmptyNote() {
        assertEquals("Sans wasabi", appendQuickNote("", "Sans wasabi"))
        assertEquals("Sans wasabi", appendQuickNote("   ", "Sans wasabi"))
    }

    @Test
    fun aChipIsAppendedWithACommaAndASpace() {
        assertEquals("Sans wasabi, Bien cuit", appendQuickNote("Sans wasabi", "Bien cuit"))
    }

    @Test
    fun aTypedSeparatorIsTidiedBeforeAppending() {
        assertEquals("sans wasabi, Bien cuit", appendQuickNote("sans wasabi,", "Bien cuit"))
        assertEquals("sans wasabi, Bien cuit", appendQuickNote("sans wasabi ,  ", "Bien cuit"))
    }

    @Test
    fun aChipAlreadyInTheNoteDoesNothingIgnoringCase() {
        assertNull(appendQuickNote("Sans wasabi, Bien cuit", "bien cuit"))
        assertTrue(noteHasChip("Sans wasabi, Bien cuit", "Sans wasabi"))
        // A part of a text is not the chip.
        assertFalse(noteHasChip("Bien cuit pas trop", "Bien cuit"))
        assertEquals("Bien cuit pas trop, Bien cuit", appendQuickNote("Bien cuit pas trop", "Bien cuit"))
    }

    @Test
    fun theSixtyCharacterLimitStillApplies() {
        val forty = "x".repeat(40)
        assertEquals("$forty, Sans oignon", appendQuickNote(forty, "Sans oignon"))
        assertNull(appendQuickNote(forty, "y".repeat(19)))
        assertEquals(NOTE_MAX, appendQuickNote(forty, "y".repeat(18))?.length)
        assertNull(appendQuickNote("", "z".repeat(NOTE_MAX + 1)))
    }

    @Test
    fun aBlankChipDoesNothing() {
        assertNull(appendQuickNote("Bien cuit", "  "))
    }

    @Test
    fun anItemMovesUpOrDownClampedToTheEnds() {
        val list = listOf("A", "B", "C")
        assertEquals(listOf("B", "A", "C"), moveQuickNote(list, 1, -1))
        assertEquals(listOf("A", "C", "B"), moveQuickNote(list, 1, 1))
        assertEquals(list, moveQuickNote(list, 0, -1))
        assertEquals(list, moveQuickNote(list, 2, 1))
        assertEquals(list, moveQuickNote(list, 5, 1))
    }

    @Test
    fun anItemIsRemoved() {
        assertEquals(listOf("A", "C"), removeQuickNote(listOf("A", "B", "C"), 1))
    }

    @Test
    fun aTrimmedItemIsAddedAtTheEnd() {
        assertEquals(listOf("A", "B", "C", "D"), addQuickNote(listOf("A", "B", "C"), "  D "))
    }

    @Test
    fun aBlankRepeatedTooLongItemOrAFullListIsRefused() {
        val list = listOf("A", "B", "C")
        assertNull(addQuickNote(list, "  "))
        assertNull(addQuickNote(list, "b"))
        assertNull(addQuickNote(list, "x".repeat(NOTE_MAX + 1)))
        assertEquals(4, addQuickNote(list, "x".repeat(NOTE_MAX))?.size)
        assertNull(addQuickNote(list, "D", maxCount = 3))
    }
}
