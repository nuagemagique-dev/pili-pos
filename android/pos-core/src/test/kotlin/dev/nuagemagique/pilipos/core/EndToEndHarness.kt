package dev.nuagemagique.pilipos.core

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import uniffi.pos_ffi.EngineOptions
import uniffi.pos_ffi.PosEngine
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.installInitDb
import java.io.File
import java.net.HttpURLConnection
import java.net.URI
import java.util.concurrent.CopyOnWriteArrayList

/** Seeded manager of src-tauri/seed_spf_test.sql (role admin). */
const val MANAGER_INSZ = "94060242576"
const val MANAGER_PIN = "1111"

/**
 * One engine on a fresh temporary data directory, seeded like the SPF test machine, pointed at a
 * pos-fdm-mock and a pos-printer-mock process. The events of the engine are collected in a
 * coroutine into [events]. [close] shuts the engine down (30 s bound) and releases it.
 */
class EndToEnd private constructor(
    val engine: PosEngine,
    val dataDir: File,
    private val printerHttpPort: Int,
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

    /** The raw JSON of the printer mock's GET /jobs. */
    fun printerJobsJson(): String {
        val conn = URI("http://127.0.0.1:$printerHttpPort/jobs").toURL().openConnection() as HttpURLConnection
        try {
            conn.connectTimeout = 3_000
            conn.readTimeout = 3_000
            return conn.inputStream.bufferedReader().use { it.readText() }
        } finally {
            conn.disconnect()
        }
    }

    /** The printed jobs of the printer mock (text of each job as the paper shows it). */
    fun printerJobTexts(): List<String> =
        Regex("\"text\"\\s*:\\s*\"((?:[^\"\\\\]|\\\\.)*)\"").findAll(printerJobsJson())
            .map { it.groupValues[1].replace("\\n", "\n").replace("\\\"", "\"") }.toList()

    /** Waits until the printer mock holds at least [count] jobs and returns their texts. */
    suspend fun awaitPrinterJobs(
        count: Int,
        timeoutMs: Long = 10_000,
    ): List<String> {
        var texts = emptyList<String>()
        try {
            withTimeout(timeoutMs) {
                while (true) {
                    texts = withContext(Dispatchers.IO) { printerJobTexts() }
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
        kotlinx.coroutines.runBlocking(Dispatchers.IO) {
            runCatching { withTimeout(35_000) { engine.shutdown() } }
            runCatching { withTimeout(10_000) { collector.cancelAndJoin() } }
            runCatching { engine.destroy() }
        }
        scope.coroutineContext[Job]?.cancel()
    }

    companion object {
        /** Opens an engine under [dataDir] (seeded) against the two mocks. */
        suspend fun open(
            dataDir: File,
            bundleDir: File,
            fdm: MockProcess,
            printer: MockProcess,
        ): EndToEnd {
            bundleDir.mkdirs()
            val posJson = File(bundleDir, "pos.json")
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
                  "fdm_url": "http://127.0.0.1:${fdm.ports.single()}/graphql",
                  "printer_address": "127.0.0.1:${printer.ports[0]}",
                  "table_count": 25
                }
                """.trimIndent(),
            )
            val seed = File(Mocks.repoRoot, "src-tauri/seed_spf_test.sql")
            require(seed.isFile) { "seed not found: $seed" }
            installInitDb(dataDir.absolutePath, seed.absolutePath)
            val engine =
                withContext(Dispatchers.IO) {
                    PosEngine.open(
                        EngineOptions(
                            dataDir = dataDir.absolutePath,
                            configBundle = posJson.absolutePath,
                            deviceId = "EMULATOR-TEST-0001",
                            timers = null,
                        ),
                    )
                }
            val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
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
            return EndToEnd(engine, dataDir, printer.ports[1], scope, collector, events)
        }
    }
}
