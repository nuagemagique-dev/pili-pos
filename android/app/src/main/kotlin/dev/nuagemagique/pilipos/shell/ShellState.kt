package dev.nuagemagique.pilipos.shell

import dev.nuagemagique.pilipos.engine.ClosedReason
import dev.nuagemagique.pilipos.engine.EngineState
import dev.nuagemagique.pilipos.provisioning.ProvisioningProblem

/** What the shell shows: one screen per state. */
sealed interface ShellState {
    /** The app is not the device owner (PD108): nothing can be provisioned or opened. */
    data object NeedsDeviceOwner : ShellState

    /** No configuration bundle is installed; [importing] while an import runs, [problem] after a failed one. */
    data class NotProvisioned(
        val importing: Boolean,
        val problem: ProvisioningProblem?,
    ) : ShellState

    /** The installed bundle belongs to the device [stored], this device is [actual]. */
    data class DeviceMismatch(
        val stored: String,
        val actual: String,
        val importing: Boolean,
        val problem: ProvisioningProblem?,
    ) : ShellState

    /** The device identifier cannot be read; [detail] says why. */
    data class DeviceIdUnavailable(
        val detail: String,
    ) : ShellState

    /** The engine is being opened. */
    data object Opening : ShellState

    /** The engine runs; the navigation host is shown. */
    data class Ready(
        val posSwVersion: String,
    ) : ShellState

    /** The engine failed for good; the process must restart. */
    data class Fatal(
        val reason: String,
    ) : ShellState
}

/**
 * The shell side of an import and of the first open: whether an import runs, the problem of the last failed
 * import and whether the first [dev.nuagemagique.pilipos.engine.EngineHolder.open] of this screen returned.
 */
internal data class ShellProgress(
    val importing: Boolean = false,
    val problem: ProvisioningProblem? = null,
    val firstOpenDone: Boolean = false,
)

/**
 * Maps the device owner flag, the engine state and the import progress to a [ShellState].
 *
 * - not the device owner: [ShellState.NeedsDeviceOwner] whatever the engine says (PD108, every build);
 * - `Closed(NotProvisioned)`: [ShellState.NotProvisioned]; `Closed(Initial)` too once the first open returned
 *   (before that the open is about to run, so [ShellState.Opening], no flash of the provisioning screen);
 * - `Closed(DeviceMismatch)`, `Closed(DeviceIdUnavailable)`: the matching state;
 * - `Opening`, `Ready`, `Fatal`: the matching state;
 * - `Closed(Shutdown)`: [ShellState.Opening], the shell always reopens (an import closes, then opens again).
 */
internal fun mapShellState(
    isDeviceOwner: Boolean,
    engine: EngineState,
    progress: ShellProgress,
): ShellState =
    when {
        !isDeviceOwner -> ShellState.NeedsDeviceOwner
        else ->
            when (engine) {
                EngineState.Opening -> ShellState.Opening
                is EngineState.Ready -> ShellState.Ready(engine.posSwVersion)
                is EngineState.Fatal -> ShellState.Fatal(engine.reason)
                is EngineState.Closed -> mapClosed(engine.reason, progress)
            }
    }

private fun mapClosed(
    reason: ClosedReason,
    progress: ShellProgress,
): ShellState =
    when (reason) {
        ClosedReason.NotProvisioned -> ShellState.NotProvisioned(progress.importing, progress.problem)
        ClosedReason.Initial ->
            if (progress.firstOpenDone) {
                ShellState.NotProvisioned(progress.importing, progress.problem)
            } else {
                ShellState.Opening
            }
        is ClosedReason.DeviceMismatch ->
            ShellState.DeviceMismatch(reason.stored, reason.actual, progress.importing, progress.problem)
        is ClosedReason.DeviceIdUnavailable -> ShellState.DeviceIdUnavailable(reason.detail)
        ClosedReason.Shutdown -> ShellState.Opening
    }
