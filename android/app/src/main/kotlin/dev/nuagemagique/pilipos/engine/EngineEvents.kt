package dev.nuagemagique.pilipos.engine

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
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

        private companion object {
            /** Events held for slow collectors before the pump suspends. */
            const val EVENT_BUFFER = 64
        }
    }
