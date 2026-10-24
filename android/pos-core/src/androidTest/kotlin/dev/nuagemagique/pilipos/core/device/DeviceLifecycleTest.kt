package dev.nuagemagique.pilipos.core.device

import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import uniffi.pos_ffi.EngineOptions
import uniffi.pos_ffi.PosEngine
import uniffi.pos_ffi.PosException
import java.io.File

/**
 * Lifecycle of the engine on the device: open, second open on the same directory, shutdown, reopen, a call
 * and the event stream after shutdown. Each test has its own directory under filesDir, removed afterwards.
 */
@RunWith(AndroidJUnit4::class)
class DeviceLifecycleTest {
    private lateinit var root: File
    private lateinit var dataDir: File
    private lateinit var posJson: File
    private val opened = mutableListOf<PosEngine>()

    @Before
    fun setUp() {
        requireFdmMock()
        root = DeviceEnv.newWorkDir("lifecycle")
        dataDir = File(root, "data")
        posJson = writePosJson(File(root, "bundle"))
    }

    @After
    fun tearDown() {
        // Always shut the engines down and release their native objects, then remove the directory.
        runBlocking(ioDispatcher) {
            for (engine in opened) {
                runCatching { withTimeout(TIMEOUT_MS) { engine.shutdown() } }
                runCatching { engine.destroy() }
            }
            opened.clear()
        }
        if (::root.isInitialized) root.deleteRecursively()
    }

    /** Opens an engine on the IO dispatcher and remembers it for the cleanup. */
    private suspend fun open(options: EngineOptions = engineOptions(dataDir, posJson)): PosEngine =
        withContext(ioDispatcher) { PosEngine.open(options) }.also { opened += it }

    private fun lifecycle(body: suspend () -> Unit) = runBlocking(ioDispatcher) { withTimeout(TIMEOUT_MS * 2) { body() } }

    @Test
    fun openOnAFreshDirectoryWritesTheEngineFiles() =
        lifecycle {
            open()
            assertTrue("the data directory exists", dataDir.isDirectory)
            assertTrue("the data directory is not empty", dataDir.list().orEmpty().isNotEmpty())
        }

    @Test
    fun aSecondOpenOnTheSameDataDirIsRefused() =
        lifecycle {
            val first = open()
            val error = runCatching { open() }.exceptionOrNull()
            assertTrue("expected PosException, got $error", error is PosException)
            val detail = error.toString()
            assertTrue("detail was: $detail", detail.contains("engine already open"))
            first.shutdown()
        }

    @Test
    fun openThenShutdownThenReopenOnTheSameDirectory() =
        lifecycle {
            val first = open()
            first.shutdown()
            // shutdown released the engine lock: the same directory opens again.
            val second = open()
            second.shutdown()
            // A second shutdown is Ok.
            second.shutdown()
        }

    @Test
    fun aRelativeDataDirIsRefused() =
        lifecycle {
            val error = runCatching { open(EngineOptions("relative/data", posJson.absolutePath, DEVICE_ID, null)) }.exceptionOrNull()
            assertTrue("expected PosException, got $error", error is PosException)
            assertTrue("detail was: $error", error.toString().contains("absolute"))
        }

    @Test
    fun eventsAfterShutdownAreEngineClosedAndAnOpenStreamEnds() =
        lifecycle {
            val engine = open()
            val stream = engine.events()
            engine.shutdown()
            assertNull("next() after shutdown", withTimeout(10_000) { stream.next() })
            // And stays closed.
            assertNull(withTimeout(10_000) { stream.next() })
            stream.destroy()
            val error = runCatching { engine.events() }.exceptionOrNull()
            assertTrue("expected PosException.EngineClosed, got $error", error is PosException.EngineClosed)
        }

    @Test
    fun aCallAfterShutdownIsEngineClosed() =
        lifecycle {
            val engine = open()
            engine.shutdown()
            val error = runCatching { engine.getSettings() }.exceptionOrNull()
            assertTrue("expected EngineClosed, got $error", error is PosException.EngineClosed)
            val again = runCatching { engine.userActivity() }.exceptionOrNull()
            assertTrue("expected EngineClosed, got $again", again is PosException.EngineClosed)
        }

    @Test
    fun aRelativeBundlePathIsRefused() =
        lifecycle {
            val error = runCatching { open(EngineOptions(dataDir.absolutePath, "bundle/pos.json", null, null)) }.exceptionOrNull()
            assertTrue("expected PosException, got $error", error is PosException)
            assertEquals(true, error.toString().contains("absolute"))
        }

    private companion object {
        const val TIMEOUT_MS = 60_000L
    }
}
