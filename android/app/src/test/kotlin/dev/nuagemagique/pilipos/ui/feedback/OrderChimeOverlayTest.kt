package dev.nuagemagique.pilipos.ui.feedback

import androidx.compose.ui.test.junit4.v2.createComposeRule
import dev.nuagemagique.pilipos.ui.state.SessionStore
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
import uniffi.pos_ffi.User
import uniffi.pos_ffi.WebshopOrderSummary
import java.time.Instant

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
    private val orders = MutableStateFlow<List<WebshopOrderSummary>>(emptyList())
    private val user = MutableStateFlow<User?>(null)
    private val start = Instant.parse("2026-10-09T10:00:00Z")

    private fun show() {
        val store =
            mockk<WebshopOrdersStore> {
                every { loaded } returns this@OrderChimeOverlayTest.loaded
                every { pendingCount } returns this@OrderChimeOverlayTest.pending
                every { orders } returns this@OrderChimeOverlayTest.orders
            }
        val session = mockk<SessionStore> { every { currentUser } returns this@OrderChimeOverlayTest.user }
        val overlay = OrderChimeOverlay(store, player, session)
        overlay.now = { start.plusMillis(compose.mainClock.currentTime) }
        compose.setContent { overlay.Content() }
        compose.waitForIdle()
    }

    private fun setPending(value: Int) {
        pending.value = value
        compose.waitForIdle()
    }

    @Test
    fun theOverlayIsBelowEveryVisibleOne() {
        assertEquals(10, OrderChimeOverlay(mockk(), player, mockk()).order)
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

    private fun lateOrder(waitedMinutes: Long) =
        WebshopOrderSummary(
            id = 1,
            hubriseOrderId = "h1",
            status = "open",
            hubriseStatus = "new",
            source = "web",
            serviceType = "collection",
            totalCents = 2_000,
            createdAt = start.minusSeconds(waitedMinutes * 60).toString(),
            items = emptyList(),
            customerNotes = null,
            customerName = "Client",
            confirmedTime = null,
            customerAddress1 = null,
            customerAddress2 = null,
            customerPostalCode = null,
            customerCity = null,
            webshop = null,
        )

    @Test
    fun aLateOrderChimesAtOnceThenEveryTwoMinutesForALoggedInOperator() {
        compose.mainClock.autoAdvance = false
        orders.value = listOf(lateOrder(6))
        user.value = mockk(relaxed = true)
        show()
        compose.mainClock.advanceTimeBy(1L)
        assertEquals(1, player.plays)

        compose.mainClock.advanceTimeBy(LATE_CHIME_EVERY_MS - LATE_TICK_MS)
        assertEquals(1, player.plays)
        compose.mainClock.advanceTimeBy(LATE_TICK_MS + 1L)
        assertEquals(2, player.plays)
    }

    @Test
    fun theLateChimeIsSilentWithNobodyLoggedIn() {
        compose.mainClock.autoAdvance = false
        orders.value = listOf(lateOrder(6))
        show()
        compose.mainClock.advanceTimeBy(2 * LATE_CHIME_EVERY_MS)

        assertEquals(0, player.plays)
    }
}
