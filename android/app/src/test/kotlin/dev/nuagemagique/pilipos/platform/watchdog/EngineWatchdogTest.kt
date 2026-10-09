package dev.nuagemagique.pilipos.platform.watchdog

import android.app.Application
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.ClosedReason
import dev.nuagemagique.pilipos.engine.EngineState
import dev.nuagemagique.pilipos.platform.PlatformOps
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import dev.nuagemagique.pilipos.platform.audit.PlatformClock
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.io.File

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class EngineWatchdogTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val base = 1_800_000_000_000L
    private val second = 1_000L
    private val minute = 60_000L

    private class Rig(
        val state: MutableStateFlow<EngineState>,
        val audit: PlatformAudit,
        val budget: RestartBudget,
        val ops: PlatformOps,
        val dir: File,
        val watchdog: EngineWatchdog,
    ) {
        var kills = 0
        var starts = 0
        var startFails = false

        fun events(): List<String> {
            val file = File(dir, "audit.jsonl")
            if (!file.exists()) return emptyList()
            return file.readLines().map { JSONObject(it).let { j -> "${j.getString("kind")}/${j.getString("event")}" } }
        }
    }

    private fun rig(scope: TestScope): Rig {
        val dir = File(tmp.root, "platform")
        val io = UnconfinedTestDispatcher(scope.testScheduler)
        val clock = PlatformClock { base + scope.testScheduler.currentTime }
        val state = MutableStateFlow<EngineState>(EngineState.Closed(ClosedReason.Initial))
        val audit = PlatformAudit(dir, clock, io, { 1L to "t" }, { 1 })
        val budget = RestartBudget(File(dir, "restarts.json"))
        val ops = PlatformOps()
        lateinit var rig: Rig
        val watchdog =
            EngineWatchdog(
                state = state,
                audit = audit,
                budget = budget,
                ops = ops,
                killer = { rig.kills++ },
                starter =
                    {
                        rig.starts++
                        if (rig.startFails) error("service not allowed")
                    },
                clock = clock,
                scope = scope.backgroundScope,
                ioDispatcher = io,
            )
        rig = Rig(state, audit, budget, ops, dir, watchdog)
        return rig
    }

    private fun TestScope.start(rig: Rig) {
        rig.watchdog.onApplicationCreated(ApplicationProvider.getApplicationContext<Application>())
        runCurrent()
    }

    private fun TestScope.step(ms: Long) {
        advanceTimeBy(ms)
        runCurrent()
    }

    @Test
    fun theHookRunsAtOrderTwoHundred() =
        runTest {
            assertEquals(200, rig(this).watchdog.order)
        }

    @Test
    fun aFatalStateKillsTheProcessAfterThreeSeconds() =
        runTest {
            val rig = rig(this)
            start(rig)
            rig.state.value = EngineState.Fatal("panic: boom")
            runCurrent()
            assertEquals(listOf("watchdog/fatal"), rig.events())
            step(2_999)
            assertEquals(0, rig.kills)
            step(2)
            assertEquals(1, rig.kills)
            assertEquals(listOf("watchdog/fatal", "watchdog/restart"), rig.events())
            assertEquals(base + 3 * second, rig.budget.lastRestart())
            val fatal = File(rig.dir, "audit.jsonl").readLines().first().let(::JSONObject)
            assertEquals("panic: boom", fatal.getJSONObject("detail").getString("reason"))
        }

    @Test
    fun theFourthFatalInTenMinutesIsNotRestarted() =
        runTest {
            val rig = rig(this)
            start(rig)
            for (i in 1..3) {
                rig.state.value = EngineState.Fatal("f$i")
                step(4 * second)
                step(minute)
            }
            assertEquals(3, rig.kills)
            rig.state.value = EngineState.Fatal("f4")
            step(10 * second)
            assertEquals(3, rig.kills)
            assertEquals("watchdog/budget_exhausted", rig.events().last())
            assertEquals(3, rig.events().count { it == "watchdog/restart" })
        }

    @Test
    fun theBudgetOpensAgainOnceTheWindowHasPassed() =
        runTest {
            val rig = rig(this)
            start(rig)
            for (i in 1..3) {
                rig.state.value = EngineState.Fatal("f$i")
                step(4 * second)
            }
            assertEquals(3, rig.kills)
            step(10 * minute)
            rig.state.value = EngineState.Fatal("later")
            step(4 * second)
            assertEquals(4, rig.kills)
        }

    @Test
    fun aRestartYoungerThanAMinuteIsAuditedAtStart() =
        runTest {
            val rig = rig(this)
            rig.budget.record(base - 5 * second)
            start(rig)
            assertEquals(listOf("watchdog/restarted"), rig.events())
        }

    @Test
    fun anOlderRestartIsNotReported() =
        runTest {
            val rig = rig(this)
            rig.budget.record(base - 2 * minute)
            start(rig)
            assertEquals(emptyList<String>(), rig.events())
        }

    @Test
    fun aShutdownLastingMoreThanAMinuteReopensTheEngine() =
        runTest {
            val rig = rig(this)
            start(rig)
            rig.state.value = EngineState.Closed(ClosedReason.Shutdown)
            runCurrent()
            step(59 * second)
            assertEquals(0, rig.starts)
            step(2 * second)
            assertEquals(1, rig.starts)
            assertEquals(listOf("watchdog/reopen"), rig.events())
        }

    @Test
    fun aRunningPlatformOperationBlocksTheReopen() =
        runTest {
            val rig = rig(this)
            start(rig)
            val gate = CompletableDeferred<Unit>()
            launch { rig.ops.run("update") { gate.await() } }
            runCurrent()
            rig.state.value = EngineState.Closed(ClosedReason.Shutdown)
            step(5 * minute)
            assertEquals(0, rig.starts)
            assertEquals(emptyList<String>(), rig.events())
            gate.complete(Unit)
            runCurrent()
            assertEquals(1, rig.starts)
            assertEquals(listOf("watchdog/reopen"), rig.events())
        }

    @Test
    fun aStateChangeCancelsTheReopenTimer() =
        runTest {
            val rig = rig(this)
            start(rig)
            rig.state.value = EngineState.Closed(ClosedReason.Shutdown)
            step(30 * second)
            rig.state.value = EngineState.Ready("v")
            step(5 * minute)
            assertEquals(0, rig.starts)
            assertEquals(emptyList<String>(), rig.events())
        }

    @Test
    fun aFailingServiceStartIsAuditedAndSwallowed() =
        runTest {
            val rig = rig(this)
            rig.startFails = true
            start(rig)
            rig.state.value = EngineState.Closed(ClosedReason.Shutdown)
            step(61 * second)
            assertEquals(listOf("watchdog/reopen", "watchdog/reopen_failed"), rig.events())
        }

    @Test
    fun otherStatesDoNothing() =
        runTest {
            val rig = rig(this)
            start(rig)
            for (s in listOf(
                EngineState.Opening,
                EngineState.Ready("v"),
                EngineState.Closed(ClosedReason.NotProvisioned),
                EngineState.Closed(ClosedReason.DeviceIdUnavailable("x")),
            )) {
                rig.state.value = s
                step(5 * minute)
            }
            assertEquals(0, rig.kills)
            assertEquals(0, rig.starts)
            assertEquals(emptyList<String>(), rig.events())
        }

    @Test
    fun theWatchdogKeepsRunningAfterARestartWasHandled() =
        runTest {
            val rig = rig(this)
            start(rig)
            rig.state.value = EngineState.Fatal("one")
            step(4 * second)
            rig.state.value = EngineState.Closed(ClosedReason.Shutdown)
            step(61 * second)
            assertEquals(1, rig.kills)
            assertEquals(1, rig.starts)
        }
}
