package dev.nuagemagique.pilipos.core

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import org.junit.AfterClass
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.BeforeClass
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.pos_ffi.EngineOptions
import uniffi.pos_ffi.PosEngine
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.initLogging
import uniffi.pos_ffi.uniffiEnsureInitialized
import java.io.File

/**
 * Lifecycle of the engine over the real host library: open, second open on the same directory,
 * shutdown, reopen, events after shutdown. No engine command is called here (later cards).
 */
class LifecycleTest {
    companion object {
        private lateinit var fdm: MockProcess

        @BeforeClass
        @JvmStatic
        fun startMocks() {
            fdm = Mocks.startFdm()
        }

        @AfterClass
        @JvmStatic
        fun stopMocks() {
            if (::fdm.isInitialized) fdm.stop()
        }

        private const val TIMEOUT_MS = 60_000L
    }

    @get:Rule
    val tmp = TemporaryFolder()

    private lateinit var base: File
    private lateinit var dataDir: File
    private lateinit var posJson: File
    private val opened = mutableListOf<PosEngine>()

    @Before
    fun setUp() {
        base = tmp.newFolder("work")
        dataDir = File(base, "data")
        val bundle = File(base, "bundle").also { it.mkdirs() }
        posJson = File(bundle, "pos.json")
        posJson.writeText(
            """
            {
              "pos_id": "CPOS0031234567",
              "terminal_id": "POS-2-ANDROID",
              "device_id": "pos-json-device",
              "restaurant_name": "Restaurant Test",
              "restaurant_address": "1 Rue du Test, 4000 Liege",
              "restaurant_vat": "BE0000000097",
              "restaurant_est_no": "2000000042",
              "fdm_url": "http://127.0.0.1:${fdm.ports.single()}/graphql"
            }
            """.trimIndent(),
        )
    }

    private fun options(data: String = dataDir.absolutePath) =
        EngineOptions(
            dataDir = data,
            configBundle = posJson.absolutePath,
            deviceId = "EMULATOR-TEST-0001",
            timers = null,
        )

    /** Opens an engine on the IO dispatcher and remembers it for the cleanup. */
    private suspend fun open(data: String = dataDir.absolutePath): PosEngine =
        withContext(Dispatchers.IO) { PosEngine.open(options(data)) }.also { opened += it }

    /** Always shut the engines down and release their native objects. */
    private fun cleanup() =
        runBlocking(Dispatchers.IO) {
            for (engine in opened) {
                runCatching { withTimeout(TIMEOUT_MS) { engine.shutdown() } }
                runCatching { engine.destroy() }
            }
            opened.clear()
        }

    private fun lifecycle(body: suspend () -> Unit) {
        try {
            runBlocking(Dispatchers.IO) { withTimeout(TIMEOUT_MS * 2) { body() } }
        } finally {
            cleanup()
        }
    }

    @Test
    fun initLoggingCreatesTheDailyLogFile() =
        lifecycle {
            uniffiEnsureInitialized()
            val logs = File(base, "logs")
            initLogging(logs.absolutePath, "info")
            // A second call is Ok and changes nothing (the subscriber is process-wide).
            initLogging(File(base, "logs-again").absolutePath, "info")
            val files = logs.listFiles { f -> f.name.startsWith("pili-pos.log.") }.orEmpty()
            assertTrue("a log file pili-pos.log.<date> under $logs, found ${logs.list()?.toList()}", files.isNotEmpty())
        }

    @Test
    fun openThenShutdownThenReopenOnTheSameDirectory() =
        lifecycle {
            val first = open()
            first.shutdown()
            // shutdown released engine.lock: the same directory opens again.
            val second = open()
            second.shutdown()
            // A second shutdown is Ok.
            second.shutdown()
        }

    @Test
    fun aSecondOpenOnTheSameDataDirIsRefused() =
        lifecycle {
            val first = open()
            val error = runCatching { open() }.exceptionOrNull()
            assertTrue("expected PosException.Internal, got $error", error is PosException.Internal)
            val detail = (error as PosException.Internal).detail
            assertTrue("detail was: $detail", detail.contains("engine already open"))
            first.shutdown()
        }

    @Test
    fun eventsAfterShutdownAreEngineClosed() =
        lifecycle {
            val engine = open()
            engine.shutdown()
            val error = runCatching { engine.events() }.exceptionOrNull()
            assertTrue("expected PosException.EngineClosed, got $error", error is PosException.EngineClosed)
        }

    @Test
    fun theEventStreamEndsWithTheEngine() =
        lifecycle {
            val engine = open()
            val stream = engine.events()
            engine.shutdown()
            assertNull("next() after shutdown", withTimeout(10_000) { stream.next() })
            // And stays closed.
            assertNull(withTimeout(10_000) { stream.next() })
            stream.destroy()
        }

    @Test
    fun aRelativeDataDirIsRefused() =
        lifecycle {
            val error = runCatching { open("relative/data") }.exceptionOrNull()
            assertTrue("expected PosException.Internal, got $error", error is PosException.Internal)
            assertTrue((error as PosException.Internal).detail.contains("absolute"))
        }

    @Test
    fun aRelativeBundlePathIsRefused() =
        lifecycle {
            val error =
                runCatching {
                    withContext(Dispatchers.IO) {
                        PosEngine.open(EngineOptions(dataDir.absolutePath, "bundle/pos.json", null, null))
                    }.also { opened += it }
                }.exceptionOrNull()
            assertTrue("expected PosException.Internal, got $error", error is PosException.Internal)
            assertEquals(true, (error as PosException.Internal).detail.contains("absolute"))
        }
}
