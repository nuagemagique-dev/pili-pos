package dev.nuagemagique.pilipos.engine

/**
 * Lifecycle of the in-process fiscal engine, published by [EngineHolder].
 *
 * The state never exposes the engine object: calls go through [EngineGateway].
 */
sealed interface EngineState {
    /** No engine is running; [reason] says why. */
    data class Closed(
        val reason: ClosedReason,
    ) : EngineState

    /** The engine is being opened (database, migrations, lock). */
    data object Opening : EngineState

    /** The engine runs and answers calls; [posSwVersion] is the software version the engine reports. */
    data class Ready(
        val posSwVersion: String,
    ) : EngineState

    /**
     * The engine failed in a way that cannot be repaired in this process (open failure or a Rust panic).
     * There is no in-process reopen: the process is restarted (P6).
     */
    data class Fatal(
        val reason: String,
    ) : EngineState
}

/** Why the engine is [EngineState.Closed]. */
sealed interface ClosedReason {
    /** Nothing was tried yet in this process. */
    data object Initial : ClosedReason

    /** No configuration bundle is installed: the engine is not started (no silent TEST identity). */
    data object NotProvisioned : ClosedReason

    /** The stored device identifier is not the one of this device. */
    data class DeviceMismatch(
        val stored: String,
        val actual: String,
    ) : ClosedReason

    /** The device identifier could not be read; [detail] says why. */
    data class DeviceIdUnavailable(
        val detail: String,
    ) : ClosedReason

    /** The engine was shut down cleanly. */
    data object Shutdown : ClosedReason
}

/** Outcome of [EngineHolder.close]. */
sealed interface CloseResult {
    /** The engine is closed (or was not running); its lock is released. */
    data object Closed : CloseResult

    /** The engine did not shut down (its own 30 s bound or the outer timeout); it keeps running. */
    data class StillOpen(
        val detail: String,
    ) : CloseResult
}
