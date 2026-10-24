package dev.nuagemagique.pilipos.core

import java.io.File
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.TimeUnit

/**
 * One mock process (pos-fdm-mock or pos-printer-mock) started for a test. [stop] destroys the
 * process and its descendants; callers use it in a finally block or in @AfterClass.
 */
class MockProcess(
    val name: String,
    private val process: Process,
    val log: File,
    /** The ports the mock listens on, in the order of the start function. */
    val ports: List<Int>,
) : AutoCloseable {
    val isAlive: Boolean get() = process.isAlive

    fun stop() {
        Mocks.forget(this)
        val tree = descendantHandles(process)
        process.destroy()
        tree.forEach { destroyHandle(it, force = false) }
        if (!process.waitFor(5, TimeUnit.SECONDS)) {
            tree.forEach { destroyHandle(it, force = true) }
            process.destroyForcibly()
            process.waitFor(5, TimeUnit.SECONDS)
        }
    }

    override fun close() = stop()
}

// java.lang.ProcessHandle is not part of the Android API surface that unit tests compile against
// (android.jar), but the host JVM that runs them has it: the descendants are reached by reflection.
private val processHandleClass: Class<*> = Class.forName("java.lang.ProcessHandle")

private fun descendantHandles(process: Process): List<Any> {
    val stream = Process::class.java.getMethod("descendants").invoke(process) as java.util.stream.Stream<*>
    return stream.iterator().asSequence().filterNotNull().toList()
}

private fun destroyHandle(
    handle: Any,
    force: Boolean,
) {
    processHandleClass.getMethod(if (force) "destroyForcibly" else "destroy").invoke(handle)
}

/** Starts the mocks built by the Gradle task cargoBuildMocks (paths come from system properties). */
object Mocks {
    private const val STARTUP_TIMEOUT_MS = 10_000L

    /** The repository root (system property pili.repoRoot). */
    val repoRoot: File get() = File(requiredProperty("pili.repoRoot"))

    // Mocks still running; one JVM shutdown hook stops them, so a crashed test JVM leaves no process.
    private val running: MutableSet<MockProcess> = ConcurrentHashMap.newKeySet()

    init {
        Runtime.getRuntime().addShutdownHook(
            Thread {
                running.toList().forEach { runCatching { it.stop() } }
            },
        )
    }

    internal fun forget(mock: MockProcess) {
        running.remove(mock)
    }

    private val logDir: File
        get() = File(System.getProperty("user.dir"), "build/mock-logs").also { it.mkdirs() }

    /** A port that was free a moment ago (bound on port 0, then released). */
    fun freePort(): Int = ServerSocket(0).use { it.localPort }

    /** pos-fdm-mock on a free port; [MockProcess.ports] is `[port]`. */
    fun startFdm(): MockProcess {
        val port = freePort()
        return start("pos-fdm-mock", requiredProperty("pili.fdmMock"), mapOf("PORT" to port.toString()), listOf(port))
    }

    /** pos-printer-mock on two free ports; [MockProcess.ports] is `[tcpPort, httpPort]`. */
    fun startPrinter(): MockProcess {
        val tcp = freePort()
        var http = freePort()
        while (http == tcp) http = freePort()
        return start(
            "pos-printer-mock",
            requiredProperty("pili.printerMock"),
            mapOf("PORT" to tcp.toString(), "HTTP_PORT" to http.toString()),
            listOf(tcp, http),
        )
    }

    /** Whether something accepts TCP connections on 127.0.0.1:[port]. */
    fun accepts(
        port: Int,
        timeoutMs: Int = 300,
    ): Boolean =
        try {
            Socket().use { it.connect(InetSocketAddress("127.0.0.1", port), timeoutMs) }
            true
        } catch (_: java.io.IOException) {
            false
        }

    private fun requiredProperty(name: String): String =
        System.getProperty(name) ?: error("system property $name is not set (run the tests through Gradle)")

    private fun start(
        name: String,
        binary: String,
        env: Map<String, String>,
        ports: List<Int>,
    ): MockProcess {
        require(File(binary).canExecute()) { "$name binary not found or not executable: $binary" }
        val log = File(logDir, "$name-${ports.first()}.log")
        val builder =
            ProcessBuilder(binary)
                .redirectErrorStream(true)
                .redirectOutput(log)
        builder.environment().putAll(env)
        val mock = MockProcess(name, builder.start(), log, ports)
        running.add(mock)
        try {
            val deadline = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(STARTUP_TIMEOUT_MS)
            for (port in ports) {
                while (!accepts(port)) {
                    check(mock.isAlive) { "$name exited before listening on $port: ${log.readText().take(2000)}" }
                    check(System.nanoTime() < deadline) { "$name did not listen on $port within 10 s" }
                    Thread.sleep(50)
                }
            }
            return mock
        } catch (t: Throwable) {
            mock.stop()
            throw t
        }
    }
}
