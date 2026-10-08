package dev.nuagemagique.pilipos.engine

import app.cash.turbine.test
import io.mockk.coEvery
import io.mockk.every
import io.mockk.justRun
import io.mockk.mockk
import io.mockk.verify
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.pos_ffi.EventStream
import uniffi.pos_ffi.InternalException
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException

@OptIn(ExperimentalCoroutinesApi::class)
class EngineEventsTest {
    @get:Rule
    val tmp = TemporaryFolder()

    /** A stream that yields `Resync(1)` to `Resync(count)` then ends; [reads] counts the `next()` calls. */
    private fun streamOf(
        count: Int,
        reads: IntArray = IntArray(1),
    ): EventStream {
        val stream = mockk<EventStream>()
        coEvery { stream.next() } coAnswers {
            reads[0] += 1
            if (reads[0] <= count) PosEvent.Resync(reads[0].toLong()) else null
        }
        justRun { stream.destroy() }
        return stream
    }

    @Test
    fun eventsAreDeliveredInOrder() =
        runTest {
            val events = EngineEvents()
            val stream = streamOf(5)
            events.events.test {
                val pump = launch { events.pump(stream) { _, _ -> } }
                val received = (1..5).map { (awaitItem() as PosEvent.Resync).missed }
                assertEquals(listOf(1L, 2L, 3L, 4L, 5L), received)
                pump.join()
            }
        }

    @Test
    fun streamEndDestroysTheStream() =
        runTest {
            val events = EngineEvents()
            val stream = streamOf(0)
            events.pump(stream) { _, _ -> }
            verify(exactly = 1) { stream.destroy() }
        }

    @Test
    fun pumpWithoutCollectorsDoesNotBlock() =
        runTest {
            val events = EngineEvents()
            val reads = IntArray(1)
            events.pump(streamOf(300, reads)) { _, _ -> }
            assertEquals(301, reads[0])
        }

    @Test
    fun slowCollectorSuspendsThePumpAndNoEventIsLost() =
        runTest {
            val events = EngineEvents()
            val reads = IntArray(1)
            val total = 200
            val received = mutableListOf<Long>()
            val gate = CompletableDeferred<Unit>()
            backgroundScope.launch {
                events.events.collect {
                    gate.await()
                    received += (it as PosEvent.Resync).missed
                }
            }
            runCurrent()
            val pump = launch { events.pump(streamOf(total, reads)) { _, _ -> } }
            runCurrent()
            // One event held by the collector, 64 buffered, the next one suspends the pump.
            assertTrue("pump read ${reads[0]} events while the collector was stalled", reads[0] in 65..67)
            gate.complete(Unit)
            pump.join()
            advanceUntilIdle()
            assertEquals((1..total).map { it.toLong() }, received)
        }

    @Test
    fun internalExceptionInThePumpIsFatalAndTheStreamIsDestroyed() =
        runTest {
            val events = EngineEvents()
            val stream = mockk<EventStream>()
            coEvery { stream.next() } throws InternalException("panic in next")
            justRun { stream.destroy() }
            var reported: String? = null
            events.pump(stream) { reason, _ -> reported = reason }
            assertEquals("panic: event pump: panic in next", reported)
            verify(exactly = 1) { stream.destroy() }
        }

    @Test
    fun unexpectedFailureInThePumpIsFatal() =
        runTest {
            val events = EngineEvents()
            val stream = mockk<EventStream>()
            coEvery { stream.next() } throws IllegalStateException("bad")
            justRun { stream.destroy() }
            var reported: String? = null
            events.pump(stream) { reason, _ -> reported = reason }
            assertEquals("event pump: IllegalStateException: bad", reported)
        }

    @Test
    fun engineClosedEndsThePumpQuietly() =
        runTest {
            val events = EngineEvents()
            val stream = mockk<EventStream>()
            coEvery { stream.next() } throws PosException.EngineClosed()
            every { stream.destroy() } returns Unit
            var reported = false
            events.pump(stream) { _, _ -> reported = true }
            assertEquals(false, reported)
            verify(exactly = 1) { stream.destroy() }
        }

    @Test
    fun cancellingThePumpDestroysTheStream() =
        runTest {
            val events = EngineEvents()
            val stream = mockk<EventStream>()
            coEvery { stream.next() } coAnswers {
                delay(1_000_000)
                null
            }
            justRun { stream.destroy() }
            val pump = launch { events.pump(stream) { _, _ -> } }
            runCurrent()
            pump.cancel()
            pump.join()
            verify(exactly = 1) { stream.destroy() }
        }

    @Test
    fun theHolderFeedsTheEventFlowFromTheEngineStream() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            val stream = streamOf(2)
            coEvery { f.engine.events() } returns stream
            f.events.events.test {
                f.holder.open()
                assertEquals(1L, (awaitItem() as PosEvent.Resync).missed)
                assertEquals(2L, (awaitItem() as PosEvent.Resync).missed)
            }
        }

    @Test
    fun aPanicInTheHolderPumpMakesTheEngineFatal() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            val stream = mockk<EventStream>()
            coEvery { stream.next() } throws InternalException("pump panic")
            justRun { stream.destroy() }
            coEvery { f.engine.events() } returns stream
            f.holder.open()
            advanceUntilIdle()
            assertEquals(EngineState.Fatal("panic: event pump: pump panic"), f.holder.state.value)
            assertNull(f.holder.readyEngine())
        }
}
