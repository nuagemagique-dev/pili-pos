package dev.nuagemagique.pilipos.platform.watchdog

import android.app.Application
import dev.nuagemagique.pilipos.di.AppLifecycleHook
import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.di.IoDispatcher
import dev.nuagemagique.pilipos.engine.ClosedReason
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineState
import dev.nuagemagique.pilipos.platform.PlatformOps
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import dev.nuagemagique.pilipos.platform.audit.PlatformClock
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

/**
 * The crash watchdog (PD207). It watches [EngineState] from the application scope.
 *
 * - [EngineState.Fatal]: audit, wait [KILL_DELAY_MS] (the logs flush, the fatal screen is seen), record the
 *   restart and kill the own process; the system restarts the persistent HOME app and the sticky engine service.
 *   [RestartBudget] allows 3 restarts per 10 minutes; beyond that the watchdog audits `budget_exhausted` and the
 *   shell keeps its fatal screen. The restart is a real process death only (PD118).
 * - `Closed(Shutdown)` for more than [REOPEN_DELAY_MS] while no platform operation runs: audit and start the
 *   engine service again. While an operation (update, export) runs, it waits for the operation to end first.
 * - At start: a restart recorded less than [RESTARTED_WINDOW_MS] ago is audited as `restarted`.
 */
@Singleton
class EngineWatchdog
    @Suppress("LongParameterList")
    internal constructor(
        private val state: StateFlow<EngineState>,
        private val audit: PlatformAudit,
        private val budget: RestartBudget,
        private val ops: PlatformOps,
        private val killer: ProcessKiller,
        private val starter: EngineStarter,
        private val clock: PlatformClock,
        private val scope: CoroutineScope,
        private val ioDispatcher: CoroutineDispatcher,
    ) : AppLifecycleHook {
        @Suppress("LongParameterList")
        @Inject
        constructor(
            gateway: EngineGateway,
            audit: PlatformAudit,
            budget: RestartBudget,
            ops: PlatformOps,
            killer: ProcessKiller,
            starter: EngineStarter,
            clock: PlatformClock,
            @ApplicationScope scope: CoroutineScope,
            @IoDispatcher ioDispatcher: CoroutineDispatcher,
        ) : this(gateway.state, audit, budget, ops, killer, starter, clock, scope, ioDispatcher)

        override val order: Int = ORDER

        override fun onApplicationCreated(app: Application) {
            scope.launch {
                reportRestart()
                state.collectLatest { handle(it) }
            }
        }

        private suspend fun reportRestart() {
            val last = withContext(ioDispatcher) { budget.lastRestart() } ?: return
            val age = clock.nowMillis() - last
            if (age in 0 until RESTARTED_WINDOW_MS) {
                audit.append("watchdog", "restarted", mapOf("ageMs" to age.toString()))
            }
        }

        private suspend fun handle(current: EngineState) {
            when {
                current is EngineState.Fatal -> onFatal(current.reason)
                current is EngineState.Closed && current.reason == ClosedReason.Shutdown -> onShutdown()
            }
        }

        private suspend fun onFatal(reason: String) {
            audit.append("watchdog", "fatal", mapOf("reason" to reason))
            val allowed = withContext(ioDispatcher) { budget.allow(clock.nowMillis()) }
            if (!allowed) {
                audit.append("watchdog", "budget_exhausted")
                return
            }
            delay(KILL_DELAY_MS)
            withContext(ioDispatcher) { budget.record(clock.nowMillis()) }
            audit.append("watchdog", "restart")
            killer.kill()
        }

        private suspend fun onShutdown() {
            delay(REOPEN_DELAY_MS)
            ops.busy.first { it.isEmpty() }
            audit.append("watchdog", "reopen")
            try {
                starter.start()
            } catch (e: CancellationException) {
                throw e
            } catch (
                @Suppress("TooGenericExceptionCaught") e: Exception,
            ) {
                Timber.w(e, "EngineWatchdog: engine service start failed")
                audit.append("watchdog", "reopen_failed", mapOf("error" to (e.message ?: e.javaClass.simpleName)))
            }
        }

        companion object {
            /** Position among the lifecycle hooks. */
            const val ORDER = 200

            /** Wait between the fatal audit and the kill. */
            const val KILL_DELAY_MS = 3_000L

            /** How long the engine may stay cleanly shut down before it is reopened. */
            const val REOPEN_DELAY_MS = 60_000L

            /** A restart younger than this at start is reported as `restarted`. */
            const val RESTARTED_WINDOW_MS = 60_000L
        }
    }
