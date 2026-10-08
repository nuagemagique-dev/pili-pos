package dev.nuagemagique.pilipos.engine

import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.di.IoDispatcher
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.withContext
import uniffi.pos_ffi.InternalException
import uniffi.pos_ffi.PosEngineInterface
import javax.inject.Inject
import javax.inject.Singleton

/** The engine is not [EngineState.Ready]: [state] is what the holder reported when the call was refused. */
class EngineUnavailableException(
    val state: EngineState,
) : Exception("Engine unavailable: $state")

/**
 * The only way for the rest of the app to call the engine.
 *
 * Every call runs on the I/O dispatcher. [uniffi.pos_ffi.PosException] passes through unchanged (the callers
 * show it); an [InternalException] is a Rust panic, which makes the engine [EngineState.Fatal].
 */
@Singleton
class EngineGateway
    @Inject
    constructor(
        private val holder: EngineHolder,
        @IoDispatcher private val ioDispatcher: CoroutineDispatcher,
        @ApplicationScope scope: CoroutineScope,
    ) {
        /** True while the engine is [EngineState.Ready]. */
        val ready: StateFlow<Boolean> =
            holder.state
                .map { it is EngineState.Ready }
                .stateIn(scope, SharingStarted.Eagerly, holder.state.value is EngineState.Ready)

        /**
         * Runs [block] against the ready engine on the I/O dispatcher.
         *
         * @throws EngineUnavailableException when the engine is not ready
         */
        suspend fun <T> call(block: suspend (PosEngineInterface) -> T): T {
            val engine = holder.readyEngine() ?: throw EngineUnavailableException(holder.state.value)
            return try {
                withContext(ioDispatcher) { block(engine) }
            } catch (e: InternalException) {
                holder.markFatal("panic: ${e.message}", e)
                throw e
            }
        }
    }
