package dev.nuagemagique.pilipos.ui.badge

import android.os.Handler
import android.os.Looper
import android.view.KeyEvent
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import java.util.concurrent.TimeUnit

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class KeyWedgeSourceTest {
    private var now = 10_000L
    private val source = KeyWedgeSource({ now }, Handler(Looper.getMainLooper()))

    /** What reached the view hierarchy: (action, key code), in order. */
    private val delivered = mutableListOf<Pair<Int, Int>>()

    init {
        source.attach { event ->
            delivered += event.action to event.keyCode
            true
        }
    }

    private fun down(
        code: Int,
        gap: Long = 5L,
    ): Boolean {
        now += gap
        return source.dispatch(KeyEvent(now, now, KeyEvent.ACTION_DOWN, code, 0))
    }

    private fun up(code: Int): Boolean = source.dispatch(KeyEvent(now, now, KeyEvent.ACTION_UP, code, 0))

    private fun typeBadge(codes: List<Int>) {
        codes.forEach {
            down(it)
            up(it)
        }
    }

    private val sixDigits =
        listOf(KeyEvent.KEYCODE_1, KeyEvent.KEYCODE_2, KeyEvent.KEYCODE_3, KeyEvent.KEYCODE_4, KeyEvent.KEYCODE_5, KeyEvent.KEYCODE_6)

    private fun collect(block: () -> Unit): List<String> {
        val scans = mutableListOf<String>()
        runTest(UnconfinedTestDispatcher()) {
            val job = launch { source.scans.toList(scans) }
            block()
            job.cancel()
        }
        return scans
    }

    @Test
    fun readerOffPassesEveryKey() {
        // Not started: the source never takes a key.
        assertFalse(down(KeyEvent.KEYCODE_1))
        assertFalse(up(KeyEvent.KEYCODE_1))
        assertFalse(down(KeyEvent.KEYCODE_ENTER))
        assertTrue(delivered.isEmpty())
    }

    @Test
    fun aBadgeIsSwallowedAndHandedOver() {
        source.start()
        val scans =
            collect {
                typeBadge(sixDigits)
                assertTrue(down(KeyEvent.KEYCODE_ENTER))
                assertTrue(up(KeyEvent.KEYCODE_ENTER))
            }
        assertEquals(listOf("123456"), scans)
        // Nothing of the scan reached the view hierarchy.
        assertTrue(delivered.isEmpty())
    }

    @Test
    fun theUpOfASwallowedKeyIsSwallowedToo() {
        source.start()
        assertTrue(down(KeyEvent.KEYCODE_7))
        assertTrue(up(KeyEvent.KEYCODE_7))
        // A second up for a key that was not held goes through.
        assertFalse(up(KeyEvent.KEYCODE_7))
    }

    @Test
    fun heldEventsReplayInOrder() {
        source.start()
        // Three keys, then a letter that is not a badge key: the held keys come first, then the letter passes.
        typeBadge(listOf(KeyEvent.KEYCODE_1, KeyEvent.KEYCODE_2, KeyEvent.KEYCODE_3))
        val passed = !down(KeyEvent.KEYCODE_G)
        assertTrue(passed)
        assertEquals(
            listOf(
                KeyEvent.ACTION_DOWN to KeyEvent.KEYCODE_1,
                KeyEvent.ACTION_UP to KeyEvent.KEYCODE_1,
                KeyEvent.ACTION_DOWN to KeyEvent.KEYCODE_2,
                KeyEvent.ACTION_UP to KeyEvent.KEYCODE_2,
                KeyEvent.ACTION_DOWN to KeyEvent.KEYCODE_3,
                KeyEvent.ACTION_UP to KeyEvent.KEYCODE_3,
            ),
            delivered,
        )
    }

    @Test
    fun aShortBurstWithEnterIsReplayedAndTheEnterPasses() {
        source.start()
        typeBadge(listOf(KeyEvent.KEYCODE_1, KeyEvent.KEYCODE_2))
        assertFalse(down(KeyEvent.KEYCODE_ENTER))
        assertEquals(listOf(KeyEvent.KEYCODE_1, KeyEvent.KEYCODE_1, KeyEvent.KEYCODE_2, KeyEvent.KEYCODE_2), delivered.map { it.second })
    }

    @Test
    fun theHandlerReleasesTheHeldKeysAfterTheGap() {
        source.start()
        source.configure(WedgeConfig(maxGapMs = 50L, minLen = 6, maxLen = 32))
        assertTrue(down(KeyEvent.KEYCODE_4))
        assertTrue(delivered.isEmpty())
        shadowOf(Looper.getMainLooper()).idleFor(60, TimeUnit.MILLISECONDS)
        assertEquals(listOf(KeyEvent.ACTION_DOWN to KeyEvent.KEYCODE_4, KeyEvent.ACTION_UP to KeyEvent.KEYCODE_4), delivered)
    }

    @Test
    fun aNewTimingReleasesTheHeldKeysThenAppliesTheNewGap() {
        source.start()
        assertTrue(down(KeyEvent.KEYCODE_4))
        assertTrue(delivered.isEmpty())

        source.configure(WedgeConfig(maxGapMs = 150L, minLen = 6, maxLen = 32))

        // The held key left at once, before the new gap is applied.
        assertEquals(listOf(KeyEvent.ACTION_DOWN to KeyEvent.KEYCODE_4, KeyEvent.ACTION_UP to KeyEvent.KEYCODE_4), delivered)
        assertTrue(down(KeyEvent.KEYCODE_5))
        shadowOf(Looper.getMainLooper()).idleFor(60, TimeUnit.MILLISECONDS)
        assertEquals("still held after 60 ms (gap 150 ms)", 2, delivered.size)
        shadowOf(Looper.getMainLooper()).idleFor(100, TimeUnit.MILLISECONDS)
        assertEquals(
            listOf(
                KeyEvent.ACTION_DOWN to KeyEvent.KEYCODE_4,
                KeyEvent.ACTION_UP to KeyEvent.KEYCODE_4,
                KeyEvent.ACTION_DOWN to KeyEvent.KEYCODE_5,
                KeyEvent.ACTION_UP to KeyEvent.KEYCODE_5,
            ),
            delivered,
        )
    }

    @Test
    fun theReplayIsNotSeenByTheDetectorAgain() {
        source.start()
        val scans =
            collect {
                // Six digits then a gap: they are released, and released keys must not be held again.
                typeBadge(sixDigits)
                shadowOf(Looper.getMainLooper()).idleFor(60, TimeUnit.MILLISECONDS)
                assertFalse(down(KeyEvent.KEYCODE_ENTER))
            }
        assertTrue(scans.isEmpty())
        assertEquals(12, delivered.size)
    }

    @Test
    fun aBurstThatIsTooSlowIsTypingNotABadge() {
        source.start()
        val scans =
            collect {
                sixDigits.forEach {
                    down(it, gap = 120L)
                    up(it)
                }
                down(KeyEvent.KEYCODE_ENTER, gap = 120L)
            }
        assertTrue(scans.isEmpty())
    }

    @Test
    fun stoppingTheSourceReleasesWhatIsHeldAndPassesEverything() {
        source.start()
        assertTrue(down(KeyEvent.KEYCODE_8))
        source.stop()
        assertEquals(listOf(KeyEvent.ACTION_DOWN to KeyEvent.KEYCODE_8, KeyEvent.ACTION_UP to KeyEvent.KEYCODE_8), delivered)
        assertFalse(down(KeyEvent.KEYCODE_9))
    }

    @Test
    fun aModifierIsNeverTaken() {
        source.start()
        assertFalse(down(KeyEvent.KEYCODE_SHIFT_LEFT))
        assertFalse(up(KeyEvent.KEYCODE_SHIFT_LEFT))
    }

    @Test
    fun detachingTheActivityReleasesTheHeldKeysIntoNothing() {
        source.start()
        assertTrue(down(KeyEvent.KEYCODE_2))
        source.detach()
        assertTrue(delivered.isEmpty())
    }
}
