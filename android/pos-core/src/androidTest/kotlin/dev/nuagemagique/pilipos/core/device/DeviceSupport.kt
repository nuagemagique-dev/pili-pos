package dev.nuagemagique.pilipos.core.device

import android.content.Context
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import uniffi.pos_ffi.EngineOptions
import uniffi.pos_ffi.PosEngine
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.installInitDb
import java.io.File
import java.net.InetSocketAddress
import java.net.Socket
import java.util.UUID
import java.util.concurrent.CopyOnWriteArrayList

/** Seeded manager of src-tauri/seed_spf_test.sql (role admin). */
const val MANAGER_INSZ = "94060242576"
const val MANAGER_PIN = "1111"

/** Name of the seed asset that the Gradle task copyAndroidTestSeedAssets puts into the test APK. */
const val SEED_ASSET = "seed_spf_test.sql"

const val DEVICE_ID = "EMULATOR-TEST-0001"

/**
 * The dispatcher of every FFI call of these tests: never the main thread. One place for the detekt rule
 * InjectDispatcher, which tests have no use for.
 */
@Suppress("InjectDispatcher")
val ioDispatcher: CoroutineDispatcher = Dispatchers.IO

private const val SCRIPT = "scripts/android/run_android_tests.sh"
private const val SOCKET_TIMEOUT_MS = 3_000

/**
 * The host mocks as the device sees them. The ports are instrumentation arguments
 * (-Pandroid.testInstrumentationRunnerArguments.<key>=<value>); the defaults are the ones of
 * scripts/android/run_android_tests.sh. The mocks are reached through `adb reverse`, never directly.
 */
object DeviceEnv {
    private fun port(
        key: String,
        default: Int,
    ): Int = InstrumentationRegistry.getArguments().getString(key)?.toInt() ?: default

    val fdmPort: Int get() = port("fdmPort", 19280)
    val printerPort: Int get() = port("printerPort", 19290)
    val printerHttpPort: Int get() = port("printerHttpPort", 19291)

    /** The context of the app under test (the test APK itself for a library module). */
    val context: Context get() = InstrumentationRegistry.getInstrumentation().targetContext

    /** A fresh, unique directory under filesDir; the caller deletes it. */
    fun newWorkDir(prefix: String): File = File(context.filesDir, "device-tests/$prefix-${UUID.randomUUID()}").also { it.mkdirs() }

    /**
     * Copies the seed asset (copied at build time from src-tauri/seed_spf_test.sql) into cacheDir, where
     * the engine can read it as a plain file.
     */
    fun seedFile(): File {
        val seed = File(context.cacheDir, SEED_ASSET)
        InstrumentationRegistry.getInstrumentation().context.assets.open(SEED_ASSET).use { input ->
            seed.outputStream().use { input.copyTo(it) }
        }
        return seed
    }
}

/** One plain HTTP/1.0 exchange with 127.0.0.1:[port]; returns the status line and the body. */
private fun httpExchange(
    port: Int,
    method: String,
    path: String,
): Pair<String, String> =
    Socket().use { socket ->
        socket.soTimeout = SOCKET_TIMEOUT_MS
        socket.connect(InetSocketAddress("127.0.0.1", port), SOCKET_TIMEOUT_MS)
        socket.getOutputStream().apply {
            write("$method $path HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n".toByteArray())
            flush()
        }
        val response = socket.getInputStream().readBytes().toString(Charsets.UTF_8)
        response.substringBefore("\r\n") to response.substringAfter("\r\n\r\n", "")
    }

/** The raw JSON of the printer mock's GET /jobs (through the device port). */
fun printerJobsJson(): String = httpExchange(DeviceEnv.printerHttpPort, "GET", "/jobs").second

/** The printed jobs of the printer mock (text of each job as the paper shows it). */
fun printerJobTexts(): List<String> =
    Regex("\"text\"\\s*:\\s*\"((?:[^\"\\\\]|\\\\.)*)\"").findAll(printerJobsJson())
        .map { it.groupValues[1].replace("\\n", "\n").replace("\\\"", "\"") }.toList()

/** Empties the printer mock (DELETE /jobs), so a test counts only its own jobs. */
fun clearPrinterJobs() {
    val (status, _) = httpExchange(DeviceEnv.printerHttpPort, "DELETE", "/jobs")
    check(status.contains(" 2")) { "DELETE /jobs answered: $status" }
}

/** Fails (never skips) when the FDM mock does not answer through the device port. */
fun requireFdmMock() {
    val port = DeviceEnv.fdmPort
    val status =
        try {
            httpExchange(port, "GET", "/graphql").first
        } catch (e: java.io.IOException) {
            throw AssertionError(
                "FDM mock unreachable at 127.0.0.1:$port from the device. These tests need the host mocks and " +
                    "adb reverse: run them with $SCRIPT (${e.message})",
                e,
            )
        }
    if (!status.startsWith("HTTP/")) {
        throw AssertionError("FDM mock at 127.0.0.1:$port did not answer HTTP ($status). Run the tests with $SCRIPT")
    }
}

/** Fails (never skips) when the printer mock does not answer through the device ports. */
fun requirePrinterMock() {
    val port = DeviceEnv.printerHttpPort
    try {
        httpExchange(port, "GET", "/jobs")
        Socket().use { it.connect(InetSocketAddress("127.0.0.1", DeviceEnv.printerPort), SOCKET_TIMEOUT_MS) }
    } catch (e: java.io.IOException) {
        throw AssertionError(
            "printer mock unreachable at 127.0.0.1:$port/${DeviceEnv.printerPort} from the device. " +
                "Run the tests with $SCRIPT (${e.message})",
            e,
        )
    }
}

/** Writes the provisioning bundle pos.json (the host mocks as seen through adb reverse). */
fun writePosJson(dir: File): File {
    dir.mkdirs()
    val file = File(dir, "pos.json")
    file.writeText(
        """
        {
          "pos_id": "CPOS0031234567",
          "terminal_id": "POS-2-ANDROID",
          "device_id": "pos-json-device",
          "restaurant_name": "Restaurant Test",
          "restaurant_address": "1 Rue du Test, 4000 Liege",
          "restaurant_vat": "BE0000000097",
          "restaurant_est_no": "2000000042",
          "fdm_url": "http://127.0.0.1:${DeviceEnv.fdmPort}/graphql",
          "printer_address": "127.0.0.1:${DeviceEnv.printerPort}",
          "table_count": 25
        }
        """.trimIndent(),
    )
    return file
}

fun engineOptions(
    dataDir: File,
    posJson: File,
) = EngineOptions(
    dataDir = dataDir.absolutePath,
    configBundle = posJson.absolutePath,
    deviceId = DEVICE_ID,
    timers = null,
)

/**
 * One engine on a fresh data directory under filesDir, seeded like the SPF test machine and pointed at the
 * host mocks. The events of the engine are collected into [events]. [close] shuts the engine down, releases
 * it and removes the directory. Every call is made off the main thread.
 */
class DeviceEngine private constructor(
    val engine: PosEngine,
    private val root: File,
    private val scope: CoroutineScope,
    private val collector: Job,
    /** Every event received since the engine opened, in order (filled by the collector). */
    val events: CopyOnWriteArrayList<PosEvent>,
) : AutoCloseable {
    private var closed = false

    /** The first event of type [T] seen so far or within [timeoutMs], else a failure. */
    suspend inline fun <reified T : PosEvent> awaitEvent(timeoutMs: Long = 15_000): T {
        val deadline = System.nanoTime() + timeoutMs * 1_000_000
        while (true) {
            events.filterIsInstance<T>().firstOrNull()?.let { return it }
            if (System.nanoTime() > deadline) {
                throw AssertionError("no ${T::class.simpleName} event within $timeoutMs ms, saw: ${events.toList()}")
            }
            delay(25)
        }
    }

    /** Waits until the printer mock holds at least [count] jobs and returns their texts. */
    suspend fun awaitPrinterJobs(
        count: Int,
        timeoutMs: Long = 10_000,
    ): List<String> {
        var texts = emptyList<String>()
        try {
            withTimeout(timeoutMs) {
                while (true) {
                    texts = withContext(ioDispatcher) { printerJobTexts() }
                    if (texts.size >= count) return@withTimeout
                    delay(50)
                }
            }
        } catch (e: kotlinx.coroutines.TimeoutCancellationException) {
            throw AssertionError("printer mock holds ${texts.size} job(s), expected $count", e)
        }
        return texts
    }

    override fun close() {
        if (closed) return
        closed = true
        runBlocking(ioDispatcher) {
            runCatching { withTimeout(35_000) { engine.shutdown() } }
            runCatching { withTimeout(10_000) { collector.cancelAndJoin() } }
            runCatching { engine.destroy() }
        }
        scope.coroutineContext[Job]?.cancel()
        root.deleteRecursively()
    }

    companion object {
        /** Opens a seeded engine on a fresh directory; the directory is removed when the open fails. */
        suspend fun open(): DeviceEngine {
            val root = DeviceEnv.newWorkDir("e2e")
            try {
                val dataDir = File(root, "data")
                val posJson = writePosJson(File(root, "bundle"))
                val seed = DeviceEnv.seedFile()
                val engine =
                    withContext(ioDispatcher) {
                        installInitDb(dataDir.absolutePath, seed.absolutePath)
                        PosEngine.open(engineOptions(dataDir, posJson))
                    }
                val scope = CoroutineScope(SupervisorJob() + ioDispatcher)
                val stream = engine.events()
                val events = CopyOnWriteArrayList<PosEvent>()
                val collector =
                    scope.launch {
                        try {
                            while (true) {
                                val event = stream.next() ?: break
                                events += event
                            }
                        } finally {
                            stream.destroy()
                        }
                    }
                return DeviceEngine(engine, root, scope, collector, events)
            } catch (t: Throwable) {
                root.deleteRecursively()
                throw t
            }
        }
    }
}
