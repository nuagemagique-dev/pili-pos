package dev.nuagemagique.pilipos.engine

import app.cash.turbine.test
import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.pos_ffi.InternalException
import uniffi.pos_ffi.PosException
import java.util.concurrent.Executors

class EngineGatewayTest {
    @get:Rule
    val tmp = TemporaryFolder()

    @Test
    fun callBeforeReadyThrowsUnavailableCarryingTheState() =
        runTest {
            val f = EngineFixture(this, tmp.root, LaunchResult.NotProvisioned)
            f.holder.open()
            val gateway = f.gateway(this)
            val failure = runCatching { gateway.call { 1 } }.exceptionOrNull()
            assertEquals(EngineState.Closed(ClosedReason.NotProvisioned), (failure as EngineUnavailableException).state)
        }

    @Test
    fun callRunsTheBlockOnTheInjectedDispatcher() =
        runTest {
            val executor = Executors.newSingleThreadExecutor { Thread(it, "fake-io") }
            try {
                val f = EngineFixture(this, tmp.root, dispatcher = executor.asCoroutineDispatcher())
                f.holder.open()
                val gateway = f.gateway(this)
                val thread = gateway.call { Thread.currentThread().name }
                assertEquals("fake-io", thread)
            } finally {
                executor.shutdown()
            }
        }

    @Test
    fun callPassesTheReadyEngineToTheBlock() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.holder.open()
            val seen = f.gateway(this).call { it }
            assertSame(f.engine, seen)
        }

    @Test
    fun internalExceptionMakesTheEngineFatalAndIsRethrown() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.holder.open()
            val gateway = f.gateway(this)
            val failure =
                runCatching { gateway.call<Unit> { throw InternalException("rust panic") } }.exceptionOrNull()
            assertTrue(failure is InternalException)
            assertEquals(EngineState.Fatal("panic: rust panic"), f.holder.state.value)
            assertFalse(gateway.ready.first { !it })
            val next = runCatching { gateway.call { 1 } }.exceptionOrNull()
            assertTrue(next is EngineUnavailableException)
        }

    @Test
    fun codedExceptionPassesThroughAndTheStateStaysReady() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.holder.open()
            val gateway = f.gateway(this)
            val failure =
                runCatching { gateway.call<Unit> { throw PosException.Coded("pin_locked", mapOf("seconds" to "30")) } }.exceptionOrNull()
            assertEquals("pin_locked", (failure as PosException.Coded).code)
            assertEquals(mapOf("seconds" to "30"), failure.params)
            assertTrue(f.holder.state.value is EngineState.Ready)
        }

    @Test
    fun engineClosedPassesThroughWithoutChangingTheState() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.holder.open()
            val failure = runCatching { f.gateway(this).call<Unit> { throw PosException.EngineClosed() } }.exceptionOrNull()
            assertTrue(failure is PosException.EngineClosed)
            assertTrue(f.holder.state.value is EngineState.Ready)
        }

    @Test
    fun readyFollowsTheHolderState() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            val gateway = f.gateway(this)
            gateway.ready.test {
                assertFalse(awaitItem())
                f.holder.open()
                assertTrue(awaitItem())
                f.holder.close()
                assertFalse(awaitItem())
            }
        }
}
