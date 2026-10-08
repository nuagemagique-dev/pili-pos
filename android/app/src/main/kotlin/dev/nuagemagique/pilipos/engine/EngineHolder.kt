package dev.nuagemagique.pilipos.engine

import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.di.IoDispatcher
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.TimeoutCancellationException
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.withTimeoutOrNull
import timber.log.Timber
import uniffi.pos_ffi.Disposable
import uniffi.pos_ffi.EngineOptions
import uniffi.pos_ffi.InternalException
import uniffi.pos_ffi.PosEngineInterface
import uniffi.pos_ffi.PosException
import java.io.File
import java.io.IOException
import java.util.concurrent.atomic.AtomicBoolean
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Owns the one engine of the process: opens it from the launch configuration, publishes its [state],
 * pumps its events and shuts it down.
 *
 * There is no in-process reopen after [EngineState.Fatal]: the process is restarted (P6). The engine
 * object never leaves this class except through [readyEngine], which only [EngineGateway] uses.
 */
@Singleton
class EngineHolder
    @Inject
    constructor(
        private val launchConfigSource: LaunchConfigSource,
        private val natives: EngineNatives,
        private val factory: EngineFactory,
        private val engineEvents: EngineEvents,
        @IoDispatcher private val ioDispatcher: CoroutineDispatcher,
        @ApplicationScope private val applicationScope: CoroutineScope,
    ) {
        private val mutableState = MutableStateFlow<EngineState>(EngineState.Closed(ClosedReason.Initial))

        /** The lifecycle state of the engine. */
        val state: StateFlow<EngineState> = mutableState.asStateFlow()

        private val lifecycle = Mutex()
        private val loggingInstalled = AtomicBoolean(false)

        @Volatile private var engine: PosEngineInterface? = null

        @Volatile private var pump: Job? = null

        /**
         * Opens the engine when the device is provisioned. Idempotent: a [EngineState.Ready],
         * [EngineState.Opening] or [EngineState.Fatal] engine is not touched. Without a configuration the
         * engine is never started (no silent TEST identity).
         *
         * @return the state after the attempt
         */
        suspend fun open(): EngineState =
            lifecycle.withLock {
                val current = mutableState.value
                if (current is EngineState.Ready || current is EngineState.Opening || current is EngineState.Fatal) {
                    current
                } else {
                    when (val launch = launchConfigSource.launchConfig()) {
                        is LaunchResult.Ready -> openEngine(launch.config)
                        LaunchResult.NotProvisioned -> closedWith(ClosedReason.NotProvisioned)
                        is LaunchResult.DeviceMismatch -> closedWith(ClosedReason.DeviceMismatch(launch.stored, launch.actual))
                        is LaunchResult.DeviceIdUnavailable -> closedWith(ClosedReason.DeviceIdUnavailable(launch.detail))
                    }
                }
            }

        /**
         * Shuts the engine down. When it is not [EngineState.Ready] nothing is called. A refusal of the engine
         * (its own 30 s bound) or the outer timeout leaves it running: [CloseResult.StillOpen].
         */
        suspend fun close(): CloseResult =
            lifecycle.withLock {
                val running = engine
                if (mutableState.value !is EngineState.Ready || running == null) {
                    CloseResult.Closed
                } else {
                    shutdownRunning(running)
                }
            }

        /**
         * Records an unrecoverable failure: the state becomes [EngineState.Fatal] and a best-effort shutdown
         * is launched in the application scope (its errors are ignored). Idempotent.
         */
        fun markFatal(
            reason: String,
            cause: Throwable?,
        ) {
            var updated = false
            while (!updated) {
                val current = mutableState.value
                if (current is EngineState.Fatal) return
                updated = mutableState.compareAndSet(current, EngineState.Fatal(reason))
            }
            Timber.e(cause, "Engine fatal: %s", reason)
            val dying = engine
            engine = null
            if (dying != null) {
                applicationScope.launch(ioDispatcher) { bestEffortShutdown(dying) }
            }
        }

        /** The engine while [EngineState.Ready], for [EngineGateway] only. */
        internal fun readyEngine(): PosEngineInterface? = if (mutableState.value is EngineState.Ready) engine else null

        private fun closedWith(reason: ClosedReason): EngineState {
            val closed = EngineState.Closed(reason)
            mutableState.value = closed
            return closed
        }

        @Suppress("TooGenericExceptionCaught")
        private suspend fun openEngine(config: LaunchConfig): EngineState {
            mutableState.value = EngineState.Opening
            // Once the engine is open it must be published (or shut down): the caller's cancellation waits.
            return withContext(NonCancellable + ioDispatcher) {
                try {
                    prepare(config)
                    val opened = factory.open(engineOptions(config))
                    engine = opened
                    val stream = opened.events()
                    pump = applicationScope.launch(ioDispatcher) { engineEvents.pump(stream, ::markFatal) }
                    val ready = EngineState.Ready(natives.posSwVersion())
                    if (mutableState.compareAndSet(EngineState.Opening, ready)) ready else mutableState.value
                } catch (e: Throwable) {
                    markFatal("open: ${describe(e)}", e)
                    mutableState.value
                }
            }
        }

        private suspend fun prepare(config: LaunchConfig) {
            ensureDirectory(config.logDir)
            ensureDirectory(config.dataDir)
            if (!loggingInstalled.get()) {
                natives.initLogging(config.logDir.absolutePath, LOG_FILTER)
                loggingInstalled.set(true)
            }
            natives.ensureInitialized()
        }

        private suspend fun shutdownRunning(running: PosEngineInterface): CloseResult {
            var refusal: String? = null
            try {
                withTimeout(SHUTDOWN_TIMEOUT_MS) { withContext(ioDispatcher) { running.shutdown() } }
            } catch (e: TimeoutCancellationException) {
                refusal = "shutdown did not finish within ${SHUTDOWN_TIMEOUT_MS / MS_PER_SECOND} s: ${e.message}"
            } catch (e: CancellationException) {
                throw e
            } catch (e: PosException.EngineClosed) {
                Timber.d(e, "Engine already closed")
            } catch (e: PosException) {
                refusal = describe(e)
            } catch (e: InternalException) {
                markFatal("panic: ${e.message}", e)
                refusal = "panic: ${e.message}"
            }
            return if (refusal == null) {
                finishClose(running)
                CloseResult.Closed
            } else {
                Timber.w("Engine still open: %s", refusal)
                CloseResult.StillOpen(refusal)
            }
        }

        private suspend fun finishClose(closed: PosEngineInterface) {
            val current = mutableState.value
            if (current is EngineState.Ready) {
                mutableState.compareAndSet(current, EngineState.Closed(ClosedReason.Shutdown))
            }
            engine = null
            val job = pump
            pump = null
            if (job != null && withTimeoutOrNull(PUMP_DRAIN_MS) { job.join() } == null) {
                job.cancelAndJoin()
            }
            withContext(ioDispatcher) { (closed as? Disposable)?.destroy() }
        }

        @Suppress("TooGenericExceptionCaught")
        private suspend fun bestEffortShutdown(dying: PosEngineInterface) {
            try {
                dying.shutdown()
            } catch (e: Throwable) {
                Timber.w("Best-effort engine shutdown failed: %s", e.message)
            }
        }

        private companion object {
            const val LOG_FILTER = "info"
            const val MS_PER_SECOND = 1_000L

            /** The engine bounds its own shutdown at 30 s; the outer bound leaves it a margin. */
            const val SHUTDOWN_TIMEOUT_MS = 35_000L
            const val PUMP_DRAIN_MS = 2_000L
        }
    }

private fun engineOptions(config: LaunchConfig) =
    EngineOptions(
        dataDir = config.dataDir.absolutePath,
        configBundle = config.configBundle.absolutePath,
        deviceId = config.deviceId,
        timers = null,
    )

private fun ensureDirectory(dir: File) {
    if (!dir.isDirectory && !dir.mkdirs() && !dir.isDirectory) {
        throw IOException("cannot create directory ${dir.absolutePath}")
    }
}

private fun describe(e: Throwable): String =
    when (e) {
        is PosException.Coded -> "PosException.Coded: ${e.code}"
        is PosException.Fdm -> "PosException.Fdm: ${e.detail}"
        is PosException.Internal -> "PosException.Internal: ${e.detail}"
        is PosException.EngineClosed -> "PosException.EngineClosed"
        else -> "${e::class.simpleName}: ${e.message}"
    }
