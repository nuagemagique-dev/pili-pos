package dev.nuagemagique.pilipos.ui.feedback

import androidx.compose.ui.test.junit4.v2.createComposeRule
import dev.nuagemagique.pilipos.ui.state.LineAdd
import dev.nuagemagique.pilipos.ui.state.OrderStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asFlow
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/** The beep on each add (C9): the overlay, the switch and the generated sound. */
@RunWith(RobolectricTestRunner::class)
class AddBeepOverlayTest {
    @get:Rule
    val compose = createComposeRule()

    private class FakePlayer : AddBeepPlayer {
        var plays = 0

        override fun play() {
            plays += 1
        }
    }

    private val player = FakePlayer()
    private val lastAdd = MutableStateFlow<LineAdd?>(null)
    private val beepOn = MutableStateFlow(true)

    private fun add(seq: Long) = LineAdd(seq, lineId = 5, itemId = 1, units = 1)

    private fun show() {
        val orders = mockk<OrderStore> { every { lastAdd } returns this@AddBeepOverlayTest.lastAdd }
        val settings = mockk<PosSettingsStore> { every { addBeep } returns this@AddBeepOverlayTest.beepOn }
        val overlay = AddBeepOverlay(orders, settings, player)
        compose.setContent { overlay.Content() }
        compose.waitForIdle()
    }

    private fun addOne(seq: Long) {
        lastAdd.value = add(seq)
        compose.waitForIdle()
    }

    @Test
    fun theOverlayIsBelowEveryVisibleOne() {
        val overlay = AddBeepOverlay(mockk(), mockk(), player)
        assertEquals(ADD_BEEP_OVERLAY_ORDER, overlay.order)
    }

    @Test
    fun everyAddBeepsOnce() {
        show()
        addOne(1)
        addOne(2)
        assertEquals(2, player.plays)
    }

    @Test
    fun anAddAlreadyThereWhenTheOverlayStartsMakesNoSound() {
        lastAdd.value = add(7)
        show()
        assertEquals(0, player.plays)
        addOne(8)
        assertEquals(1, player.plays)
    }

    @Test
    fun theSwitchOffSilencesTheBeepAndOnBringsItBack() {
        show()
        beepOn.value = false
        addOne(1)
        assertEquals(0, player.plays)

        beepOn.value = true
        addOne(2)
        assertEquals(1, player.plays)
    }

    @Test
    fun playOnAddReadsTheSwitchAtEachAdd() =
        runTest {
            var on = true
            val adds = listOf(add(1), add(2), add(3))
            playOnAdd(adds.asFlow(), { on.also { on = !on } }, player)
            // On, off, on.
            assertEquals(2, player.plays)
        }

    @Test
    fun theBeepIsAShortSineAt880Hz() {
        val samples = beepSamples()
        assertEquals(BEEP_SAMPLE_RATE * ADD_BEEP_MS / 1000, samples.size)
        assertTrue(samples.any { it.toInt() != 0 })
        // The gain falls: the end is much quieter than the start.
        val head = samples.take(200).maxOf { kotlin.math.abs(it.toInt()) }
        val tail = samples.takeLast(200).maxOf { kotlin.math.abs(it.toInt()) }
        assertTrue(tail < head / 4)
        assertEquals(880.0, ADD_BEEP_HZ, 0.0)
    }
}
