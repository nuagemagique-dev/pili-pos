package dev.nuagemagique.pilipos.ui.feedback

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import kotlin.math.abs

class OrderChimeSamplesTest {
    private val samples = chimeSamples()
    private val perNote = CHIME_SAMPLE_RATE * CHIME_NOTE_MS / 1000

    private fun note(index: Int): List<Short> = samples.toList().subList(index * perNote, (index + 1) * perNote)

    /** Upward zero crossings: one per period of a sine. */
    private fun periods(values: List<Short>): Int = values.zipWithNext().count { (a, b) -> a < 0 && b >= 0 }

    private fun peak(values: List<Short>): Int = values.maxOf { abs(it.toInt()) }

    @Test
    fun theChimeIsTwoNotesOfOneHundredFiftyMilliseconds() {
        assertEquals(2 * 6615, samples.size)
        assertEquals(300, samples.size * 1000 / CHIME_SAMPLE_RATE)
    }

    @Test
    fun theFirstNoteIsC5AndTheSecondE5() {
        // 150 ms of 523.25 Hz is 78.5 periods and of 659.25 Hz 98.9 periods.
        assertTrue(periods(note(0)) in 77..79)
        assertTrue(periods(note(1)) in 97..99)
    }

    @Test
    fun eachNoteStartsAtZeroAndPeaksNearTheStartGain() {
        assertEquals(0, note(0).first().toInt())
        assertEquals(0, note(1).first().toInt())
        val limit = (CHIME_START_GAIN * Short.MAX_VALUE).toInt()
        assertTrue(peak(note(0)) in (limit * 9 / 10)..limit)
        assertTrue(peak(note(1)) in (limit * 9 / 10)..limit)
    }

    @Test
    fun eachNoteFadesOutExponentially() {
        for (index in 0..1) {
            val values = note(index)
            val start = peak(values.subList(0, perNote / 10))
            val middle = peak(values.subList(perNote * 45 / 100, perNote * 55 / 100))
            val end = peak(values.subList(perNote * 9 / 10, perNote))
            assertTrue("start $start middle $middle", middle < start)
            assertTrue("middle $middle end $end", end < middle)
            // Exponential: the middle is about the geometric mean of the ends, far below the linear mean.
            assertTrue(end < start / 20)
        }
    }

    @Test
    fun theSamplesStayWithinTheStartGain() {
        val limit = (CHIME_START_GAIN * Short.MAX_VALUE).toInt()
        assertTrue(samples.all { abs(it.toInt()) <= limit })
    }
}
