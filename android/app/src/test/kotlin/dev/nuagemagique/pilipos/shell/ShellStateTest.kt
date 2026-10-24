package dev.nuagemagique.pilipos.shell

import dev.nuagemagique.pilipos.engine.ClosedReason
import dev.nuagemagique.pilipos.engine.EngineState
import org.junit.Assert.assertEquals
import org.junit.Test

class ShellStateTest {
    private val idle = ShellProgress()

    private fun map(
        engine: EngineState,
        owner: Boolean = true,
        progress: ShellProgress = idle,
    ) = mapShellState(owner, engine, progress)

    @Test
    fun notDeviceOwnerWinsOverEveryEngineState() {
        val states =
            listOf(
                EngineState.Closed(ClosedReason.NotProvisioned),
                EngineState.Closed(ClosedReason.Initial),
                EngineState.Opening,
                EngineState.Ready("1.0.0"),
                EngineState.Fatal("boom"),
            )
        states.forEach { assertEquals(ShellState.NeedsDeviceOwner, map(it, owner = false)) }
    }

    @Test
    fun notProvisionedCarriesTheImportProgress() {
        assertEquals(
            ShellState.NotProvisioned(importing = false, problem = null),
            map(EngineState.Closed(ClosedReason.NotProvisioned)),
        )
        assertEquals(
            ShellState.NotProvisioned(importing = true, problem = null),
            map(EngineState.Closed(ClosedReason.NotProvisioned), progress = ShellProgress(importing = true)),
        )
    }

    @Test
    fun initialIsOpeningUntilTheFirstOpenReturnedThenNotProvisioned() {
        assertEquals(ShellState.Opening, map(EngineState.Closed(ClosedReason.Initial)))
        assertEquals(
            ShellState.NotProvisioned(importing = false, problem = null),
            map(EngineState.Closed(ClosedReason.Initial), progress = ShellProgress(firstOpenDone = true)),
        )
    }

    @Test
    fun mismatchAndDeviceIdAreMapped() {
        assertEquals(
            ShellState.DeviceMismatch("A", "B", importing = false, problem = null),
            map(EngineState.Closed(ClosedReason.DeviceMismatch("A", "B"))),
        )
        assertEquals(
            ShellState.DeviceIdUnavailable("no serial"),
            map(EngineState.Closed(ClosedReason.DeviceIdUnavailable("no serial"))),
        )
    }

    @Test
    fun openingReadyFatalAndShutdownAreMapped() {
        assertEquals(ShellState.Opening, map(EngineState.Opening))
        assertEquals(ShellState.Ready("2.1.0"), map(EngineState.Ready("2.1.0")))
        assertEquals(ShellState.Fatal("boom"), map(EngineState.Fatal("boom")))
        assertEquals(ShellState.Opening, map(EngineState.Closed(ClosedReason.Shutdown)))
    }
}
