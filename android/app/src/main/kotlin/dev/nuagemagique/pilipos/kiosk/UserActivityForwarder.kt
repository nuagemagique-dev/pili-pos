package dev.nuagemagique.pilipos.kiosk

import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineHolder
import dev.nuagemagique.pilipos.engine.EngineState
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.launch
import timber.log.Timber
import java.util.concurrent.atomic.AtomicLong
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Tells the engine that the user touched the screen (PD110). The engine owns the idle timer and the
 * lock rule (P1b); this class only bounds the traffic to one `userActivity()` call per second.
 *
 * Signals go into a conflated channel, so a burst of touches costs one pending signal. One consumer in
 * the application scope calls the engine while it is [EngineState.Ready], then waits one second.
 * Failures are logged at debug level and never shown.
 */
@Singleton
class UserActivityForwarder
    @Inject
    constructor(
        private val holder: EngineHolder,
        private val gateway: EngineGateway,
        @ApplicationScope scope: CoroutineScope,
    ) {
        private val signals = Channel<Unit>(Channel.CONFLATED)
        private val forwarded = AtomicLong(0)

        /** Number of `userActivity()` calls the engine accepted since the process started. */
        val forwardedCount: Long
            get() = forwarded.get()

        init {
            scope.launch {
                signals.receiveAsFlow().collect {
                    if (holder.state.value is EngineState.Ready) {
                        forward()
                        delay(MIN_INTERVAL_MS)
                    }
                }
            }
        }

        /** Records one user interaction; cheap and safe to call from the main thread for every event. */
        fun onInteraction() {
            signals.trySend(Unit)
        }

        @Suppress("TooGenericExceptionCaught")
        private suspend fun forward() {
            try {
                gateway.call { it.userActivity() }
                forwarded.incrementAndGet()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Timber.d(e, "kiosk: userActivity not forwarded")
            }
        }

        private companion object {
            const val MIN_INTERVAL_MS = 1_000L
        }
    }
