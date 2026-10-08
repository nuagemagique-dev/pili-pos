package dev.nuagemagique.pilipos.engine

import io.mockk.coEvery
import io.mockk.every
import io.mockk.justRun
import io.mockk.mockk
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import uniffi.pos_ffi.Disposable
import uniffi.pos_ffi.EngineOptions
import uniffi.pos_ffi.EventStream
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.PosEngineInterface
import uniffi.pos_ffi.SettlementPreview
import uniffi.pos_ffi.VatLine
import uniffi.pos_ffi.VatSplitRow
import java.io.File

/** A mocked engine that can also be destroyed, as the real `PosEngine` can. */
interface DisposableEngine :
    PosEngineInterface,
    Disposable

/** Records the order of the native calls, shared by the fakes of one test. */
class CallLog {
    val entries: MutableList<String> = mutableListOf()

    fun add(entry: String) {
        entries += entry
    }
}

/** [EngineNatives] that records its calls and never loads a library. */
class FakeNatives(
    private val log: CallLog = CallLog(),
) : EngineNatives {
    var version: String = "pili-pos-android/test"
    var ensureFailure: Throwable? = null
    var loggingDirs: MutableList<String> = mutableListOf()
    var preview: SettlementPreview = mockk()
    var cashDue: Long = 0L
    var vatRows: List<VatSplitRow> = emptyList()
    var qr: List<List<Boolean>> = emptyList()
    var threadNames: MutableList<String> = mutableListOf()

    override suspend fun initLogging(
        logDir: String,
        filter: String,
    ) {
        log.add("initLogging:$filter")
        loggingDirs += logDir
    }

    override fun ensureInitialized() {
        log.add("ensureInitialized")
        ensureFailure?.let { throw it }
    }

    override suspend fun posSwVersion(): String {
        threadNames += Thread.currentThread().name
        return version
    }

    override suspend fun settlePreview(
        totalCents: Long,
        parts: List<PaymentPart>,
    ): SettlementPreview {
        threadNames += Thread.currentThread().name
        return preview
    }

    override suspend fun cashDueCents(
        totalCents: Long,
        parts: List<PaymentPart>,
    ): Long {
        threadNames += Thread.currentThread().name
        return cashDue
    }

    override suspend fun vatSplit(lines: List<VatLine>): List<VatSplitRow> {
        threadNames += Thread.currentThread().name
        return vatRows
    }

    override suspend fun qrMatrix(text: String): List<List<Boolean>> {
        threadNames += Thread.currentThread().name
        return qr
    }
}

/** [EngineFactory] that hands out [engine] and records how it was called. */
class FakeFactory(
    private val log: CallLog,
    var engine: PosEngineInterface,
    private val onOpen: () -> Unit = {},
) : EngineFactory {
    var openCount: Int = 0
    var lastOptions: EngineOptions? = null
    var failure: Throwable? = null

    override fun open(options: EngineOptions): PosEngineInterface {
        log.add("factory.open")
        openCount += 1
        lastOptions = options
        onOpen()
        failure?.let { throw it }
        return engine
    }
}

/** [LaunchConfigSource] answering a settable result. */
class FakeLaunchSource(
    var result: LaunchResult,
) : LaunchConfigSource {
    var calls: Int = 0

    override suspend fun launchConfig(): LaunchResult {
        calls += 1
        return result
    }
}

/** A mocked engine whose event stream ends at once and whose shutdown succeeds. */
fun mockEngine(
    log: CallLog = CallLog(),
    stream: EventStream = mockStream(),
): DisposableEngine {
    val engine = mockk<DisposableEngine>()
    coEvery { engine.events() } coAnswers {
        log.add("events")
        stream
    }
    coEvery { engine.shutdown() } coAnswers { log.add("shutdown") }
    every { engine.destroy() } answers { log.add("destroy") }
    return engine
}

/** A mocked stream that ends immediately; tests override `next()` when they need events. */
fun mockStream(): EventStream {
    val stream = mockk<EventStream>()
    coEvery { stream.next() } returns null
    justRun { stream.destroy() }
    return stream
}

/** A provisioned launch result under [root]. */
fun readyLaunch(root: File): LaunchResult.Ready =
    LaunchResult.Ready(
        LaunchConfig(
            dataDir = File(root, "data"),
            configBundle = File(root, "bundle/pos.json"),
            deviceId = "SN-0001",
            logDir = File(root, "logs"),
        ),
    )

/** Everything an engine test needs, wired the way Hilt wires it. */
@OptIn(ExperimentalCoroutinesApi::class)
class EngineFixture(
    testScope: TestScope,
    root: File,
    launch: LaunchResult? = null,
    val dispatcher: CoroutineDispatcher = UnconfinedTestDispatcher(testScope.testScheduler),
) {
    val log = CallLog()
    val stream: EventStream = mockStream()
    val engine: DisposableEngine = mockEngine(log, stream)
    val source = FakeLaunchSource(launch ?: readyLaunch(root))
    val natives = FakeNatives(log)
    val events = EngineEvents()
    lateinit var holder: EngineHolder
    val factory = FakeFactory(log, engine) { stateAtOpen = holder.state.value }
    var stateAtOpen: EngineState? = null

    init {
        holder = EngineHolder(source, natives, factory, events, dispatcher, testScope.backgroundScope)
    }

    /** A gateway over [holder] on the fixture dispatcher. */
    fun gateway(testScope: TestScope): EngineGateway = EngineGateway(holder, dispatcher, testScope.backgroundScope)
}
