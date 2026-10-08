package dev.nuagemagique.pilipos.engine

import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.verify
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.delay
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.pos_ffi.InternalException
import uniffi.pos_ffi.PosException
import java.io.File

@OptIn(ExperimentalCoroutinesApi::class)
class EngineHolderTest {
    @get:Rule
    val tmp = TemporaryFolder()

    @Test
    fun startsClosedWithInitialReason() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            assertEquals(EngineState.Closed(ClosedReason.Initial), f.holder.state.value)
            assertNull(f.holder.readyEngine())
        }

    @Test
    fun notProvisionedStaysClosedAndNeverCallsTheFactory() =
        runTest {
            val f = EngineFixture(this, tmp.root, LaunchResult.NotProvisioned)
            val result = f.holder.open()
            assertEquals(EngineState.Closed(ClosedReason.NotProvisioned), result)
            assertEquals(result, f.holder.state.value)
            assertEquals(0, f.factory.openCount)
            assertTrue(f.log.entries.isEmpty())
        }

    @Test
    fun deviceMismatchIsMapped() =
        runTest {
            val f = EngineFixture(this, tmp.root, LaunchResult.DeviceMismatch(stored = "A", actual = "B"))
            assertEquals(EngineState.Closed(ClosedReason.DeviceMismatch("A", "B")), f.holder.open())
            assertEquals(0, f.factory.openCount)
        }

    @Test
    fun deviceIdUnavailableIsMapped() =
        runTest {
            val f = EngineFixture(this, tmp.root, LaunchResult.DeviceIdUnavailable("no serial"))
            assertEquals(EngineState.Closed(ClosedReason.DeviceIdUnavailable("no serial")), f.holder.open())
            assertEquals(0, f.factory.openCount)
        }

    @Test
    fun openCallsTheNativesInOrderAndPublishesReady() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            val result = f.holder.open()
            assertEquals(EngineState.Ready("pili-pos-android/test"), result)
            assertEquals(result, f.holder.state.value)
            assertEquals(listOf("initLogging:info", "ensureInitialized", "factory.open", "events"), f.log.entries)
            assertEquals(EngineState.Opening, f.stateAtOpen)
            assertSame(f.engine, f.holder.readyEngine())
        }

    @Test
    fun openPassesAbsolutePathsTheDeviceIdAndDefaultTimers() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.holder.open()
            val options = checkNotNull(f.factory.lastOptions)
            assertEquals(File(tmp.root, "data").absolutePath, options.dataDir)
            assertEquals(File(tmp.root, "bundle/pos.json").absolutePath, options.configBundle)
            assertEquals("SN-0001", options.deviceId)
            assertNull(options.timers)
            assertEquals(listOf(File(tmp.root, "logs").absolutePath), f.natives.loggingDirs)
            assertTrue(File(tmp.root, "data").isDirectory)
            assertTrue(File(tmp.root, "logs").isDirectory)
        }

    @Test
    fun concurrentOpensCallTheFactoryOnce() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            val results = (1..5).map { async { f.holder.open() } }.awaitAll()
            assertEquals(1, f.factory.openCount)
            assertEquals(1, f.source.calls)
            assertTrue(results.all { it is EngineState.Ready })
        }

    @Test
    fun openIsIdempotentOnceReady() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            val first = f.holder.open()
            assertEquals(first, f.holder.open())
            assertEquals(1, f.factory.openCount)
        }

    @Test
    fun openThrowingPosExceptionIsFatal() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.factory.failure = PosException.Internal("engine already open: /data")
            val result = f.holder.open()
            assertEquals(EngineState.Fatal("open: PosException.Internal: engine already open: /data"), result)
            assertNull(f.holder.readyEngine())
        }

    @Test
    fun openThrowingInternalExceptionIsFatal() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.factory.failure = InternalException("panic in open")
            val result = f.holder.open()
            assertEquals(EngineState.Fatal("open: InternalException: panic in open"), result)
        }

    @Test
    fun missingNativeLibraryIsFatal() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.natives.ensureFailure = UnsatisfiedLinkError("libpos_ffi.so")
            val result = f.holder.open()
            assertTrue(result is EngineState.Fatal)
            assertEquals(0, f.factory.openCount)
        }

    @Test
    fun failureAfterTheEngineOpenedShutsItDown() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.events() } throws PosException.EngineClosed()
            val result = f.holder.open()
            assertTrue(result is EngineState.Fatal)
            coVerify(exactly = 1) { f.engine.shutdown() }
        }

    @Test
    fun openAfterFatalStaysFatalAndDoesNotCallTheFactory() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.factory.failure = PosException.Internal("boom")
            val fatal = f.holder.open()
            val again = f.holder.open()
            assertEquals(fatal, again)
            assertEquals(1, f.factory.openCount)
        }

    @Test
    fun openCanBeRetriedAfterNotProvisioned() =
        runTest {
            val f = EngineFixture(this, tmp.root, LaunchResult.NotProvisioned)
            f.holder.open()
            f.source.result = readyLaunch(tmp.root)
            assertTrue(f.holder.open() is EngineState.Ready)
            assertEquals(1, f.factory.openCount)
        }

    @Test
    fun closeShutsDownThenDestroysAndPublishesShutdown() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.holder.open()
            assertEquals(CloseResult.Closed, f.holder.close())
            assertEquals(EngineState.Closed(ClosedReason.Shutdown), f.holder.state.value)
            assertEquals(listOf("initLogging:info", "ensureInitialized", "factory.open", "events", "shutdown", "destroy"), f.log.entries)
            verify(exactly = 1) { f.stream.destroy() }
            assertNull(f.holder.readyEngine())
        }

    @Test
    fun closeWhoseShutdownThrowsInternalIsStillOpen() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.holder.open()
            coEvery { f.engine.shutdown() } throws PosException.Internal("shutdown timed out")
            val result = f.holder.close()
            assertTrue(result is CloseResult.StillOpen)
            assertTrue((result as CloseResult.StillOpen).detail.contains("shutdown timed out"))
            assertTrue(f.holder.state.value is EngineState.Ready)
            assertSame(f.engine, f.holder.readyEngine())
            verify(exactly = 0) { f.engine.destroy() }
        }

    @Test
    fun closeThatHangsPastTheOuterTimeoutIsStillOpen() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.holder.open()
            coEvery { f.engine.shutdown() } coAnswers { delay(60_000) }
            val result = f.holder.close()
            assertTrue(result is CloseResult.StillOpen)
            assertTrue(f.holder.state.value is EngineState.Ready)
        }

    @Test
    fun closeThenCloseAgainMakesNoSecondCall() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.holder.open()
            f.holder.close()
            assertEquals(CloseResult.Closed, f.holder.close())
            coVerify(exactly = 1) { f.engine.shutdown() }
        }

    @Test
    fun closeWhenNotReadyCallsNothing() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            assertEquals(CloseResult.Closed, f.holder.close())
            assertTrue(f.log.entries.isEmpty())
        }

    @Test
    fun reopenAfterACleanCloseWorksAndLoggingIsInstalledOnce() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.holder.open()
            f.holder.close()
            assertTrue(f.holder.open() is EngineState.Ready)
            assertEquals(2, f.factory.openCount)
            assertEquals(1, f.log.entries.count { it.startsWith("initLogging") })
        }

    @Test
    fun markFatalIsIdempotentAndShutsDownOnce() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.holder.open()
            f.holder.markFatal("panic: first", RuntimeException("x"))
            f.holder.markFatal("panic: second", null)
            advanceUntilIdle()
            assertEquals(EngineState.Fatal("panic: first"), f.holder.state.value)
            coVerify(exactly = 1) { f.engine.shutdown() }
            assertNull(f.holder.readyEngine())
        }

    @Test
    fun markFatalIgnoresAFailingBestEffortShutdown() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.holder.open()
            coEvery { f.engine.shutdown() } throws PosException.Internal("cannot")
            f.holder.markFatal("panic: x", null)
            advanceUntilIdle()
            assertEquals(EngineState.Fatal("panic: x"), f.holder.state.value)
        }

    @Test
    fun closeWhenFatalCallsNothing() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            f.factory.failure = PosException.Internal("boom")
            f.holder.open()
            assertEquals(CloseResult.Closed, f.holder.close())
        }
}
