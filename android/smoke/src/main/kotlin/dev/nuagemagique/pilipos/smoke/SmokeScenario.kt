package dev.nuagemagique.pilipos.smoke

import android.content.Context
import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeout
import uniffi.pos_ffi.AddOrderLineInput
import uniffi.pos_ffi.CreateOrderInput
import uniffi.pos_ffi.OrderStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.PosEngine
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.initLogging
import uniffi.pos_ffi.installInitDb
import uniffi.pos_ffi.posSwVersion
import uniffi.pos_ffi.uniffiEnsureInitialized
import java.io.File
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicBoolean

/**
 * The scripted scenario of the :smoke app: the real engine (libpos_ffi.so through the minified
 * bindings of :pos-core) against the host FDM and printer mocks. Every step logs one line
 * `SMOKE | <step> | PASS|FAIL | <detail>` (logcat tag [TAG]); the last line is always
 * `SMOKE | total | PASS|FAIL | <n> failed (<m> steps)`. A step that needs the result of an earlier
 * step fails with a message naming it. Amounts only come from FFI values, nothing here computes money.
 * [run] must be called on a background dispatcher: it makes blocking FFI calls.
 */
class SmokeScenario(
    private val context: Context,
    private val scope: CoroutineScope,
    private val fdmPort: Int,
    private val printerPort: Int,
    private val onLine: (String) -> Unit,
) {
    private var steps = 0
    private var failed = 0

    private var engine: PosEngine? = null
    private var dataDir: File? = null
    private var bundleDir: File? = null
    private var collector: Job? = null
    private val events = CopyOnWriteArrayList<PosEvent>()
    private val streamEnded = AtomicBoolean(false)

    suspend fun run() {
        step("ensure-initialized") {
            uniffiEnsureInitialized()
            "bindings and checksums verified"
        }
        step("version") {
            val version = posSwVersion()
            check(version == EXPECTED_VERSION) { "got $version, expected $EXPECTED_VERSION" }
            version
        }
        step("logging") { loggingStep() }
        step("install-db") { installDbStep() }
        step("open") { openStep() }
        step("events") { eventsStep() }
        step("login") {
            val user = need(engine, "open").login(MANAGER_INSZ, MANAGER_PIN)
            check(user.insz == MANAGER_INSZ) { "logged in as another user" }
            "manager ${user.insz}"
        }
        step("fdm-status") { fdmStatusStep() }
        step("sale") { saleStep() }
        step("z-report") {
            val e = need(engine, "open")
            val report = e.generateZReport(MANAGER_INSZ)
            check(report.success) { "Z report refused: ${report.error}" }
            check(!e.needsZReport()) { "a Z report is still needed" }
            "Z report ${report.reportNo ?: "(no number)"}"
        }
        step("shutdown", LONG_STEP_MS) { shutdownStep() }

        val verdict = if (failed == 0) "PASS" else "FAIL"
        emit("total", verdict, "$failed failed ($steps steps)")
    }

    private fun emit(
        name: String,
        verdict: String,
        detail: String,
    ) {
        val clean = detail.replace('\n', ' ').replace('|', '/').take(MAX_DETAIL)
        val line = "SMOKE | $name | $verdict | $clean"
        Log.i(TAG, line)
        onLine(line)
    }

    private suspend fun step(
        name: String,
        timeoutMs: Long = STEP_MS,
        body: suspend () -> String,
    ) {
        steps++
        @Suppress("TooGenericExceptionCaught")
        try {
            val detail = withTimeout(timeoutMs) { body() }
            emit(name, "PASS", detail)
        } catch (t: Throwable) {
            failed++
            emit(name, "FAIL", "${t.javaClass.name}: ${t.message}")
        }
    }

    private suspend fun loggingStep(): String {
        val logDir = File(context.filesDir, "logs").apply { mkdirs() }
        initLogging(logDir.absolutePath, "info")
        var found: String? = null
        val deadline = System.nanoTime() + LOG_WAIT_NS
        while (found == null && System.nanoTime() < deadline) {
            found = logDir.list()?.firstOrNull { it.startsWith("pili-pos.log") }
            if (found == null) delay(POLL_MS)
        }
        return checkNotNull(found) { "no pili-pos.log.* file in $logDir" }
    }

    private suspend fun installDbStep(): String {
        val root = File(context.filesDir, "smoke")
        check(!root.exists() || root.deleteRecursively()) { "cannot delete the previous run in $root" }
        val seed = File(context.cacheDir, SEED_ASSET)
        context.assets.open(SEED_ASSET).use { input -> seed.outputStream().use { input.copyTo(it) } }
        val data = File(root, "data")
        installInitDb(data.absolutePath, seed.absolutePath)
        dataDir = data
        bundleDir = File(root, "bundle")
        return "seed ${seed.length()} bytes installed under ${data.name}"
    }

    private suspend fun openStep(): String {
        val data = need(dataDir, "install-db")
        val posJson = writePosJson(need(bundleDir, "install-db"), fdmPort, printerPort)
        engine = PosEngine.open(engineOptions(data, posJson))
        return "engine open on 127.0.0.1:$fdmPort and 127.0.0.1:$printerPort"
    }

    private suspend fun eventsStep(): String {
        val stream = need(engine, "open").events()
        collector =
            scope.launch {
                try {
                    while (true) {
                        val event = stream.next() ?: break
                        events += event
                    }
                    streamEnded.set(true)
                } finally {
                    stream.destroy()
                }
            }
        return "collecting the event stream"
    }

    private suspend fun fdmStatusStep(): String {
        need(engine, "open")
        val deadline = System.nanoTime() + FDM_WAIT_NS
        while (System.nanoTime() < deadline) {
            events.filterIsInstance<PosEvent.FdmStatus>().firstOrNull()?.let { return "FdmStatus event: ${it.health}" }
            delay(POLL_MS)
        }
        error("no FdmStatus event within ${FDM_WAIT_NS / NANOS_PER_MS} ms, saw ${events.size} event(s)")
    }

    private suspend fun saleStep(): String {
        val e = need(engine, "open")
        val item = e.getMenuItems(null, "fr").first { it.active && !it.openPrice && it.priceCents > 0 }
        val order = e.createOrder(CreateOrderInput(OrderType.TAKEAWAY, null, MANAGER_INSZ))
        e.addOrderLine(AddOrderLineInput(order.id, item.id, 1, null, null))
        val priced = e.getOrder(order.id)
        val result = e.checkout(order.id, listOf(PaymentPart(PaymentMethod.CASH, null)), null)
        check(result.success) { "checkout refused: ${result.error}" }
        val ticket = checkNotNull(result.ticket) { "no ticket" }
        val ref = checkNotNull(ticket.fdmRef) { "no FDM reference on the ticket" }
        check(ref.fdmId.isNotBlank()) { "blank FDM id" }
        val eventId = checkNotNull(result.fiscalEventId) { "no fiscal event id" }
        val closed = e.getOrder(order.id)
        check(closed.status == OrderStatus.CLOSED) { "order is ${closed.status}" }
        return "order ${order.id} total ${priced.totalCents} cents, fiscal event $eventId, fdm ${ref.fdmId}"
    }

    private suspend fun shutdownStep(): String {
        val first = need(engine, "open")
        val data = need(dataDir, "install-db")
        first.shutdown()
        collector?.join()
        check(streamEnded.get()) { "the event stream did not end with null" }
        val second = PosEngine.open(engineOptions(data, File(need(bundleDir, "install-db"), "pos.json")))
        second.shutdown()
        second.destroy()
        first.destroy()
        engine = null
        return "stream ended, reopened on the same data dir, closed again"
    }

    private companion object {
        const val TAG = "PiliSmoke"
        const val EXPECTED_VERSION = "pili-pos-android/1.0.0-dev"
        const val SEED_ASSET = "seed_spf_test.sql"

        /** Seeded manager of src-tauri/seed_spf_test.sql (role admin). */
        const val MANAGER_INSZ = "94060242576"
        const val MANAGER_PIN = "1111"

        const val MAX_DETAIL = 200
        const val STEP_MS = 60_000L
        const val LONG_STEP_MS = 120_000L
        const val POLL_MS = 50L
        const val NANOS_PER_MS = 1_000_000L
        const val LOG_WAIT_NS = 3_000_000_000L
        const val FDM_WAIT_NS = 15_000_000_000L
    }
}
