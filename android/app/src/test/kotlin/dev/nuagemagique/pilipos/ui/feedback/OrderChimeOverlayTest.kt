package dev.nuagemagique.pilipos.ui.feedback

import androidx.compose.ui.test.junit4.v2.createComposeRule
import dev.nuagemagique.pilipos.ui.state.WebshopOrdersStore
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asFlow
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class OrderChimeOverlayTest {
    @get:Rule
    val compose = createComposeRule()

    private class FakePlayer : OrderChimePlayer {
        var plays = 0

        override fun play() {
            plays += 1
        }
    }

    private val player = FakePlayer()
    private val loaded = MutableStateFlow(true)
    private val pending = MutableStateFlow(0)

    private fun show() {
        val store =
            mockk<WebshopOrdersStore> {
                every { loaded } returns this@OrderChimeOverlayTest.loaded
                every { pendingCount } returns this@OrderChimeOverlayTest.pending
            }
        val overlay = OrderChimeOverlay(store, player)
        compose.setContent { overlay.Content() }
        compose.waitForIdle()
    }

    private fun setPending(value: Int) {
        pending.value = value
        compose.waitForIdle()
    }

    @Test
    fun theOverlayIsBelowEveryVisibleOne() {
        assertEquals(10, OrderChimeOverlay(mockk(), player).order)
    }

    @Test
    fun theFirstValueMakesNoSound() {
        pending.value = 3
        show()
        assertEquals(0, player.plays)
    }

    @Test
    fun eachIncreasePlaysExactlyOnce() {
        show()
        setPending(1)
        assertEquals(1, player.plays)
        setPending(2)
        assertEquals(2, player.plays)
        setPending(5)
        assertEquals(3, player.plays)
    }

    @Test
    fun aDecreaseAndAnEqualValueMakeNoSound() {
        pending.value = 4
        show()
        setPending(2)
        assertEquals(0, player.plays)
        setPending(2)
        assertEquals(0, player.plays)
        setPending(0)
        assertEquals(0, player.plays)
        setPending(1)
        assertEquals(1, player.plays)
    }

    @Test
    fun nothingPlaysWhileTheBoardIsNotLoaded() {
        loaded.value = false
        show()
        setPending(2)
        assertEquals(0, player.plays)
    }

    @Test
    fun theOrdersAlreadyWaitingWhenTheBoardLoadsAreNotAnnounced() {
        loaded.value = false
        show()
        setPending(2)
        loaded.value = true
        compose.waitForIdle()
        assertEquals(0, player.plays)
        setPending(3)
        assertEquals(1, player.plays)
    }

    @Test
    fun theStartingPointIsTakenAgainWhenTheBoardIsReloaded() {
        pending.value = 1
        show()
        loaded.value = false
        compose.waitForIdle()
        pending.value = 0
        loaded.value = true
        compose.waitForIdle()
        assertEquals(0, player.plays)
        setPending(1)
        assertEquals(1, player.plays)
    }

    @Test
    fun playOnIncreaseSkipsTheFirstValueAndEverythingThatDoesNotRise() =
        runTest {
            val fake = FakePlayer()

            playOnIncrease(listOf(2, 3, 3, 1, 1, 4).asFlow(), fake)

            assertEquals(2, fake.plays)
        }
}
