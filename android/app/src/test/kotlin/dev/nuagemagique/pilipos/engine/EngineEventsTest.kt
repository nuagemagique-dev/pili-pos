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
import kotlinx.coroutines.test.advanceTimeBy
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

    @Test
    fun aStreamEndingWhileReadyMakesTheEngineFatal() =
        runTest {
            val f = EngineFixture(this, tmp.root, wireStreamEnd = true)
            f.holder.open()
            // The pump runs in the background scope: advance the virtual clock past the grace period.
            advanceTimeBy(f.events.endGraceMs - 1)
            runCurrent()
            assertTrue("still within the grace period", f.holder.state.value is EngineState.Ready)
            advanceTimeBy(2)
            runCurrent()
            assertEquals(EngineState.Fatal("event stream ended while ready"), f.holder.state.value)
            assertNull(f.holder.readyEngine())
            verify(exactly = 1) { f.stream.destroy() }
        }

    @Test
    fun aStreamEndingDuringAShutdownInProgressStaysQuiet() =
        runTest {
            val f = EngineFixture(this, tmp.root, wireStreamEnd = true)
            val streamEnded = CompletableDeferred<Unit>()
            val release = CompletableDeferred<Unit>()
            val stream = mockk<EventStream>()
            coEvery { stream.next() } coAnswers {
                streamEnded.await()
                null
            }
            justRun { stream.destroy() }
            coEvery { f.engine.events() } returns stream
            coEvery { f.engine.shutdown() } coAnswers { release.await() }
            f.holder.open()
            val closing = launch { f.holder.close() }
            runCurrent()
            // The engine closes its stream while the shutdown call is still running: state is still Ready.
            streamEnded.complete(Unit)
            runCurrent()
            assertTrue(f.holder.state.value is EngineState.Ready)
            release.complete(Unit)
            closing.join()
            advanceUntilIdle()
            assertEquals(EngineState.Closed(ClosedReason.Shutdown), f.holder.state.value)
        }

    @Test
    fun aStreamEndingWhileNotReadyEndsQuietly() =
        runTest {
            val f = EngineFixture(this, tmp.root, wireStreamEnd = true)
            f.holder.open()
            f.holder.markFatal("earlier", null)
            advanceUntilIdle()
            assertEquals(EngineState.Fatal("earlier"), f.holder.state.value)
            // A pump started after the engine left Ready (closed here) must not report anything.
            val closed = EngineFixture(this, tmp.root, LaunchResult.NotProvisioned, wireStreamEnd = true)
            closed.holder.open()
            val stream = streamOf(1)
            closed.events.pump(stream) { _, _ -> error("onFatal must not be called") }
            assertEquals(EngineState.Closed(ClosedReason.NotProvisioned), closed.holder.state.value)
            verify(exactly = 1) { stream.destroy() }
        }

    @Test
    fun aPumpWithoutAHolderEndsQuietly() =
        runTest {
            val events = EngineEvents()
            val stream = streamOf(0)
            events.pump(stream) { _, _ -> error("onFatal must not be called") }
            verify(exactly = 1) { stream.destroy() }
        }
}
