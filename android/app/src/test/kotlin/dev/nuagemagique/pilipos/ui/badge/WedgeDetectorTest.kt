package dev.nuagemagique.pilipos.ui.badge

import android.view.KeyEvent
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** The detector with a key code as its payload, so a replay can be compared with what was held. */
class WedgeDetectorTest {
    private val config = WedgeConfig(maxGapMs = 50L, minLen = 6, maxLen = 32)
    private val detector = WedgeDetector<Int>(config)
    private var clock = 1000L

    @Suppress("LongParameterList")
    private fun key(
        code: Int,
        gap: Long = 5L,
        repeat: Boolean = false,
        ctrl: Boolean = false,
        alt: Boolean = false,
        meta: Boolean = false,
    ): WedgeResult<Int> {
        clock += gap
        return detector.onKey(WedgeKey(code, code, clock, repeat, ctrl, alt, meta))
    }

    private fun digits(count: Int): List<Int> = List(count) { KeyEvent.KEYCODE_1 + it % 9 }

    private fun burst(codes: List<Int>): List<WedgeResult<Int>> = codes.map { key(it) }

    // ── The physical key mapping (L5-D12) ─────────────────────────

    @Test
    fun physicalKeysNotCharacters() {
        assertEquals('0', badgeChar(KeyEvent.KEYCODE_0))
        assertEquals('9', badgeChar(KeyEvent.KEYCODE_9))
        assertEquals('0', badgeChar(KeyEvent.KEYCODE_NUMPAD_0))
        assertEquals('9', badgeChar(KeyEvent.KEYCODE_NUMPAD_9))
        assertEquals('A', badgeChar(KeyEvent.KEYCODE_A))
        assertEquals('F', badgeChar(KeyEvent.KEYCODE_F))
        assertNull(badgeChar(KeyEvent.KEYCODE_G))
        assertNull(badgeChar(KeyEvent.KEYCODE_SPACE))
        assertNull(badgeChar(KeyEvent.KEYCODE_ENTER))
    }

    @Test
    fun theKeypadAndTheDigitRowGiveTheSameToken() {
        val codes =
            listOf(
                KeyEvent.KEYCODE_1,
                KeyEvent.KEYCODE_NUMPAD_2,
                KeyEvent.KEYCODE_3,
                KeyEvent.KEYCODE_NUMPAD_4,
                KeyEvent.KEYCODE_5,
                KeyEvent.KEYCODE_A,
            )
        burst(codes)
        val result = key(KeyEvent.KEYCODE_NUMPAD_ENTER)
        assertEquals("1234" + "5A", (result as WedgeResult.Badge).token)
    }

    // ── A burst ending with Enter is a badge ──────────────────────

    @Test
    fun burstEndingWithEnterIsABadge() {
        val results = burst(digits(8))
        assertTrue(results.all { it is WedgeResult.Hold && it.replay.isEmpty() })
        val end = key(KeyEvent.KEYCODE_ENTER)
        assertEquals("12345678", (end as WedgeResult.Badge).token)
        assertTrue(end.replay.isEmpty())
        assertFalse(detector.pending)
    }

    @Test
    fun hexLettersAreUppercase() {
        burst(
            listOf(KeyEvent.KEYCODE_0, KeyEvent.KEYCODE_4, KeyEvent.KEYCODE_A, KeyEvent.KEYCODE_B, KeyEvent.KEYCODE_C, KeyEvent.KEYCODE_F),
        )
        assertEquals("04ABCF", (key(KeyEvent.KEYCODE_ENTER) as WedgeResult.Badge).token)
    }

    @Test
    fun theLengthLimitsAreInclusive() {
        burst(digits(6))
        assertTrue(key(KeyEvent.KEYCODE_ENTER) is WedgeResult.Badge)
        burst(digits(32))
        assertTrue(key(KeyEvent.KEYCODE_ENTER) is WedgeResult.Badge)
    }

    @Test
    fun aGapOfExactlyTheLimitStillCounts() {
        key(KeyEvent.KEYCODE_1)
        val codes = digits(5)
        codes.forEach { key(it, gap = 50L) }
        assertTrue(key(KeyEvent.KEYCODE_ENTER, gap = 50L) is WedgeResult.Badge)
    }

    // ── Typing is never a badge and is replayed in order ──────────

    @Test
    fun humanTypingIsReplayed() {
        val first = key(KeyEvent.KEYCODE_1, gap = 200L)
        assertTrue(first is WedgeResult.Hold)
        val second = key(KeyEvent.KEYCODE_2, gap = 200L)
        assertEquals(listOf(KeyEvent.KEYCODE_1), second.replay)
        val third = key(KeyEvent.KEYCODE_3, gap = 200L)
        assertEquals(listOf(KeyEvent.KEYCODE_2), third.replay)
        assertEquals(listOf(KeyEvent.KEYCODE_3), detector.flush())
    }

    @Test
    fun aGapBreaksTheBurstAndTheNewKeyStartsOver() {
        burst(digits(4))
        val late = key(KeyEvent.KEYCODE_9, gap = 80L)
        assertEquals(digits(4), late.replay)
        assertTrue(late is WedgeResult.Hold)
        assertEquals(listOf(KeyEvent.KEYCODE_9), detector.flush())
    }

    @Test
    fun aShortBurstIsReplayedWithItsEnter() {
        burst(digits(3))
        val enter = key(KeyEvent.KEYCODE_ENTER)
        assertTrue(enter is WedgeResult.Pass)
        assertEquals(digits(3), enter.replay)
    }

    @Test
    fun aKeyThatIsNotABadgeKeyBreaksTheBurstAndKeepsTheOrder() {
        burst(digits(3))
        val other = key(KeyEvent.KEYCODE_G)
        assertTrue(other is WedgeResult.Pass)
        assertEquals(digits(3), other.replay)
        assertFalse(detector.pending)
    }

    @Test
    fun anOverlongBurstIsReplayedWholeAndItsTailIsTyping() {
        burst(digits(32))
        val over = key(KeyEvent.KEYCODE_5)
        assertTrue(over is WedgeResult.Pass)
        assertEquals(digits(32), over.replay)
        // The rest of the burst, at the same pace, is typing too: never the tail of a short badge.
        val tail = burst(digits(7))
        assertTrue(tail.all { it is WedgeResult.Pass && it.replay.isEmpty() })
        val enter = key(KeyEvent.KEYCODE_ENTER)
        assertTrue(enter is WedgeResult.Pass)
        assertTrue(enter.replay.isEmpty())
    }

    @Test
    fun anEnterAloneIsNotHeld() {
        val enter = key(KeyEvent.KEYCODE_ENTER)
        assertTrue(enter is WedgeResult.Pass)
        assertTrue(enter.replay.isEmpty())
    }

    // ── Neutral keys ──────────────────────────────────────────────

    @Test
    fun shiftHeldByTheReaderDoesNotBreakTheBurst() {
        key(KeyEvent.KEYCODE_1)
        assertTrue(key(KeyEvent.KEYCODE_SHIFT_LEFT) is WedgeResult.Neutral)
        burst(digits(5))
        assertTrue(key(KeyEvent.KEYCODE_ENTER) is WedgeResult.Badge)
    }

    @Test
    fun aKeyRepeatInsideABurstIsSwallowedAndNotCounted() {
        burst(digits(3))
        val repeat = key(KeyEvent.KEYCODE_3, repeat = true)
        assertTrue(repeat is WedgeResult.Hold)
        burst(digits(3))
        assertEquals("123123", (key(KeyEvent.KEYCODE_ENTER) as WedgeResult.Badge).token)
    }

    @Test
    fun aKeyRepeatWithNothingHeldIsLeftAlone() {
        assertTrue(key(KeyEvent.KEYCODE_3, repeat = true) is WedgeResult.Neutral)
    }

    @Test
    fun aChordIsAShortcutNotABadgeKey() {
        burst(digits(2))
        val chord = key(KeyEvent.KEYCODE_C, ctrl = true)
        assertTrue(chord is WedgeResult.Pass)
        assertEquals(digits(2), chord.replay)
        val altEnter = key(KeyEvent.KEYCODE_ENTER, alt = true)
        assertTrue(altEnter is WedgeResult.Pass)
    }

    // ── Configuration ─────────────────────────────────────────────

    @Test
    fun theConfiguredGapIsHonoured() {
        detector.configure(WedgeConfig(maxGapMs = 150L, minLen = 6, maxLen = 32))
        key(KeyEvent.KEYCODE_1)
        digits(5).forEach { key(it, gap = 100L) }
        assertTrue(key(KeyEvent.KEYCODE_ENTER, gap = 100L) is WedgeResult.Badge)
    }

    @Test
    fun theConfiguredLengthsAreHonoured() {
        detector.configure(WedgeConfig(maxGapMs = 50L, minLen = 4, maxLen = 4))
        burst(digits(4))
        assertTrue(key(KeyEvent.KEYCODE_ENTER) is WedgeResult.Badge)
        burst(digits(5))
        val enter = key(KeyEvent.KEYCODE_ENTER)
        assertTrue(enter is WedgeResult.Pass)
    }

    @Test
    fun theDefaultsAreTheVueOnes() {
        val defaults = WedgeConfig()
        assertEquals(50L, defaults.maxGapMs)
        assertEquals(6, defaults.minLen)
        assertEquals(32, defaults.maxLen)
    }
}
