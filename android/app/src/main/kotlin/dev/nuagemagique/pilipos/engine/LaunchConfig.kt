package dev.nuagemagique.pilipos.engine

import java.io.File

/**
 * Everything the engine needs to start, produced by provisioning (P4-T6) and consumed by the engine
 * service (P4-T5). This is the contract between the two cards: the names are relied upon.
 *
 * @property dataDir absolute directory of the engine databases and journals
 * @property configBundle absolute path of the verified configuration bundle
 * @property deviceId the hardware-bound device identifier
 * @property logDir absolute directory of the engine logs
 */
data class LaunchConfig(
    val dataDir: File,
    val configBundle: File,
    val deviceId: String,
    val logDir: File,
)

/** Outcome of asking provisioning whether the engine can start. */
sealed interface LaunchResult {
    /** The device is provisioned and the engine can start with [config]. */
    data class Ready(
        val config: LaunchConfig,
    ) : LaunchResult

    /** No configuration bundle is installed yet. */
    data object NotProvisioned : LaunchResult

    /** The stored device identifier is not the one of this device. */
    data class DeviceMismatch(
        val stored: String,
        val actual: String,
    ) : LaunchResult

    /** The device identifier could not be read; [detail] says why. */
    data class DeviceIdUnavailable(
        val detail: String,
    ) : LaunchResult
}

/** Source of the [LaunchResult]; implemented by provisioning (P4-T6), faked in tests. */
fun interface LaunchConfigSource {
    suspend fun launchConfig(): LaunchResult
}
