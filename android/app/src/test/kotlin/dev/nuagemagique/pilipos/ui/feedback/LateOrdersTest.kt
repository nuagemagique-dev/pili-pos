package dev.nuagemagique.pilipos.ui.feedback

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.launchIn
import kotlinx.coroutines.flow.onEach
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.currentTime
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.pos_ffi.WebshopOrderSummary
import uniffi.pos_ffi.WebshopState
import java.time.Instant

/** The late online orders (O3): the count, the chime schedule and the clock that drives them. */
class LateOrdersTest {
    private val start = Instant.parse("2026-10-09T10:00:00Z")

    private fun order(
        id: Long,
        waitedMinutes: Long,
        hubrise: String? = "new",
        fiscal: String = "pending",
    ) = WebshopOrderSummary(
        id = id,
        hubriseOrderId = "h$id",
        status = "open",
        hubriseStatus = hubrise,
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
        webshop =
            WebshopState(
                orderId = id,
                hubriseOrderId = "h$id",
                channel = "Web",
                channelKind = null,
                serviceType = null,
                paymentMode = null,
                paymentType = null,
                fiscalState = fiscal,
                resolution = "ok",
                attention = emptyList(),
                warnings = emptyList(),
                lines = emptyList(),
            ),
    )

    @Test
    fun anOrderIsLateFromFiveMinutesOfWaiting() {
        val orders = listOf(order(1, 4), order(2, 5), order(3, 12))

        assertEquals(2, lateOrderCount(orders, start))
    }

    @Test
    fun onlyAnUnansweredOrderCanBeLate() {
        val orders = listOf(order(1, 30, hubrise = "accepted"), order(2, 30, fiscal = "signed_n"), order(3, 30, hubrise = "received"))

        assertEquals(1, lateOrderCount(orders, start))
    }

    @Test
    fun theChimeIsDueAtOnceThenEveryTwoMinutes() {
        assertFalse(lateChimeDue(0, null, 0L))
        assertTrue(lateChimeDue(1, null, 0L))
        assertFalse(lateChimeDue(1, 1_000L, 1_000L + LATE_CHIME_EVERY_MS - 1))
        assertTrue(lateChimeDue(2, 1_000L, 1_000L + LATE_CHIME_EVERY_MS))
        assertFalse(lateChimeDue(0, 1_000L, 1_000L + 10 * LATE_CHIME_EVERY_MS))
    }

    @Test
    fun theSamplesFollowTheClockAndTheOrdersBecomeLate() =
        runTest {
            val orders = MutableStateFlow(listOf(order(1, 4)))
            val counts = mutableListOf<Int>()
            val job = lateSamples(orders, { start.plusMillis(currentTime) }).onEach { counts += it.count }.launchIn(backgroundScope)
            runCurrent()
            assertEquals(listOf(0), counts)

            advanceTimeBy(60_000L)
            runCurrent()

            assertEquals(1, counts.last())
            job.cancel()
        }

    @Test
    fun anIdleBoardRunsNoClock() =
        runTest {
            val orders = MutableStateFlow(listOf(order(1, 30, hubrise = "accepted")))
            val samples = mutableListOf<LateSample>()
            lateSamples(orders, { start.plusMillis(currentTime) }).onEach { samples += it }.launchIn(backgroundScope)

            advanceTimeBy(10 * LATE_CHIME_EVERY_MS)
            runCurrent()

            assertEquals(1, samples.size)
            assertEquals(0, samples.single().count)
        }

    @Test
    fun theLateCountsOnlyEmitWhenTheCountChanges() =
        runTest {
            val orders = MutableStateFlow(listOf(order(1, 6)))
            val counts = mutableListOf<Int>()
            lateCounts(orders, { start.plusMillis(currentTime) }).onEach { counts += it }.launchIn(backgroundScope)

            advanceTimeBy(60_000L)
            orders.value = listOf(order(1, 6), order(2, 9))
            runCurrent()
            advanceTimeBy(60_000L)
            orders.value = emptyList()
            runCurrent()

            assertEquals(listOf(1, 2, 0), counts)
        }

    @Test
    fun theLateChimeSoundsAtOnceThenEveryTwoMinutesWhileAnOrderIsLate() =
        runTest {
            val orders = MutableStateFlow(listOf(order(1, 5)))
            val player = CountingPlayer()
            val job = backgroundScope.launchChime(lateSamples(orders, { start.plusMillis(currentTime) }), player)
            runCurrent()
            assertEquals(1, player.plays)

            advanceTimeBy(LATE_CHIME_EVERY_MS - 1)
            runCurrent()
            assertEquals(1, player.plays)
            advanceTimeBy(LATE_TICK_MS)
            runCurrent()
            assertEquals(2, player.plays)
            advanceTimeBy(LATE_CHIME_EVERY_MS)
            runCurrent()
            assertEquals(3, player.plays)

            orders.value = emptyList()
            runCurrent()
            advanceTimeBy(5 * LATE_CHIME_EVERY_MS)
            runCurrent()
            assertEquals(3, player.plays)

            orders.value = listOf(order(2, 8))
            runCurrent()
            assertEquals(4, player.plays)
            job.cancel()
        }

    private class CountingPlayer : OrderChimePlayer {
        var plays = 0

        override fun play() {
            plays += 1
        }
    }

    private fun CoroutineScope.launchChime(
        samples: Flow<LateSample>,
        player: OrderChimePlayer,
    ) = launch { playLateChime(samples, player) }
}
