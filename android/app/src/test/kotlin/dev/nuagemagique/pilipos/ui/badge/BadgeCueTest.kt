package dev.nuagemagique.pilipos.ui.badge

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import kotlin.math.abs

class BadgeCueTest {
    private val ok = cueSamples(OkCue)
    private val refused = cueSamples(RefusedCue)

    private fun perNote(ms: Int): Int = CUE_SAMPLE_RATE * ms / 1000

    private fun periods(values: List<Short>): Int = values.zipWithNext().count { (a, b) -> a < 0 && b >= 0 }

    private fun peak(values: List<Short>): Int = values.maxOf { abs(it.toInt()) }

    @Test
    fun theOkCueIsAHighTwoTone() {
        assertEquals(perNote(120) + perNote(200), ok.size)
        val first = ok.toList().subList(0, perNote(120))
        val second = ok.toList().subList(perNote(120), ok.size)
        // 120 ms of 880 Hz is 105.6 periods, 200 ms of 1318.5 Hz is 263.7 periods.
        assertTrue(periods(first) in 104..107)
        assertTrue(periods(second) in 262..265)
    }

    @Test
    fun theRefusedCueIsOneLowTone() {
        assertEquals(perNote(350), refused.size)
        // 350 ms of 196 Hz is 68.6 periods.
        assertTrue(periods(refused.toList()) in 67..70)
    }

    @Test
    fun theLowToneIsLowerThanTheOkTones() {
        val lowPeriodsPerSecond = periods(refused.toList()) * 1000.0 / 350
        val highPeriodsPerSecond = periods(ok.toList().subList(0, perNote(120))) * 1000.0 / 120
        assertTrue(lowPeriodsPerSecond < highPeriodsPerSecond / 3)
    }

    @Test
    fun eachNoteStartsAtZeroAndFadesOut() {
        for (samples in listOf(ok, refused)) {
            assertEquals(0, samples.first().toInt())
            val values = samples.toList()
            val head = peak(values.subList(0, values.size / 10))
            val tail = peak(values.subList(values.size * 9 / 10, values.size))
            assertTrue("head $head tail $tail", tail < head)
        }
    }

    @Test
    fun noSampleExceedsTheStartGain() {
        val limit = (0.3 * Short.MAX_VALUE).toInt()
        assertTrue(peak(ok.toList()) <= limit)
        assertTrue(peak(refused.toList()) <= limit)
    }
}
