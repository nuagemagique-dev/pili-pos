package dev.nuagemagique.pilipos.engine

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.withTimeoutOrNull
import timber.log.Timber
import uniffi.pos_ffi.EventStream
import uniffi.pos_ffi.InternalException
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException
import javax.inject.Inject
import javax.inject.Singleton

/**
 * The engine events as a hot flow.
 *
 * Subscribe before the call that causes an event: there is no replay (FFI.md section 8). The pump that
 * feeds the flow suspends while a collector is slow, so the Rust broadcast channel lags and the stream
 * answers [PosEvent.Resync]; collectors treat `Resync` as "re-fetch the state" (PD107).
 */
@Singleton
class EngineEvents
    @Inject
    constructor() {
        /**
         * The holder whose state decides whether the end of the stream is expected. Member-injected and lazy
         * because the holder itself depends on this class; left unset in tests that only pump events.
         */
        @Inject
        lateinit var holder: dagger.Lazy<EngineHolder>

        /** How long the end of the stream may take to be explained by a shutdown in progress. */
        @Volatile
        internal var endGraceMs: Long = END_GRACE_MS

        private val mutableEvents =
            MutableSharedFlow<PosEvent>(
                replay = 0,
                extraBufferCapacity = EVENT_BUFFER,
                onBufferOverflow = BufferOverflow.SUSPEND,
            )

        /** Every event of the running engine, in order. */
        val events: SharedFlow<PosEvent> = mutableEvents.asSharedFlow()

        /**
         * Reads [stream] until the engine is gone (`next()` answers null) and emits each event, then destroys
         * the stream. [onFatal] receives a Rust panic or an unexpected failure of the stream.
         *
         * A stream that ends while the engine is still [EngineState.Ready] and stays Ready is not a shutdown
         * (a shutdown moves the state off Ready within [endGraceMs]): the engine is marked Fatal so that the
         * platform watchdog restarts the process (PD122, P4 exit finding).
         */
        @Suppress("TooGenericExceptionCaught")
        internal suspend fun pump(
            stream: EventStream,
            onFatal: (reason: String, cause: Throwable) -> Unit,
        ) {
            try {
                while (true) {
                    val event = stream.next() ?: break
                    mutableEvents.emit(event)
                }
                failIfStillReady()
            } catch (e: CancellationException) {
                throw e
            } catch (e: InternalException) {
                onFatal("panic: event pump: ${e.message}", e)
            } catch (e: PosException.EngineClosed) {
                Timber.d(e, "Event stream ended: engine closed")
            } catch (e: Throwable) {
                onFatal("event pump: ${e::class.simpleName}: ${e.message}", e)
            } finally {
                stream.destroy()
            }
        }

        /** Marks the engine Fatal when the stream ended but the engine did not leave Ready within the grace. */
        private suspend fun failIfStillReady() {
            if (!::holder.isInitialized) return
            val engineHolder = holder.get()
            if (engineHolder.state.value !is EngineState.Ready) return
            val left = withTimeoutOrNull(endGraceMs) { engineHolder.state.first { it !is EngineState.Ready } }
            if (left == null) {
                engineHolder.markFatal("event stream ended while ready", null)
            }
        }

        private companion object {
            /** Above the 35 s outer bound of the holder's shutdown. */
            const val END_GRACE_MS = 40_000L

            /** Events held for slow collectors before the pump suspends. */
            const val EVENT_BUFFER = 64
        }
    }
