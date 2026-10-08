package dev.nuagemagique.pilipos.provisioning

import dev.nuagemagique.pilipos.di.IoDispatcher
import dev.nuagemagique.pilipos.engine.LaunchConfig
import dev.nuagemagique.pilipos.engine.LaunchConfigSource
import dev.nuagemagique.pilipos.engine.LaunchResult
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.withContext
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Holds the installed configuration bundle and the device binding, and tells the engine how to open.
 *
 * The engine starts only when a record and `pos.json` exist and the serial of this device is the one
 * recorded at import. A different serial keeps the engine closed: only a new signed import records the new
 * serial (PD108). The deviceId is always the serial read now, never a stored copy.
 */
@Singleton
class ProvisioningStore internal constructor(
    private val backing: Backing?,
) : LaunchConfigSource {
    internal class Backing(
        val paths: ProvisioningPaths,
        val deviceIds: DeviceIdReader,
        val records: ProvisioningRecordStore,
        val io: CoroutineDispatcher,
    )

    @Inject
    constructor(
        paths: ProvisioningPaths,
        deviceIds: DeviceIdReader,
        records: ProvisioningRecordStore,
        @IoDispatcher io: CoroutineDispatcher,
    ) : this(Backing(paths, deviceIds, records, io))

    /** A store without storage: the device is never provisioned. */
    constructor() : this(null)

    private val empty = MutableStateFlow<ProvisioningRecord?>(null)

    /** The record of the installed bundle: filled by [launchConfig] and [refresh], and by every import. */
    val record: StateFlow<ProvisioningRecord?> get() = backing?.records?.state ?: empty

    /** Re-reads the record file. */
    suspend fun refresh(): ProvisioningRecord? = backing?.let { b -> withContext(b.io) { b.records.load() } }

    override suspend fun launchConfig(): LaunchResult {
        val b = backing ?: return LaunchResult.NotProvisioned
        return withContext(b.io) { resolve(b) }
    }

    private fun resolve(b: Backing): LaunchResult {
        val stored = b.records.load()
        if (stored == null || !b.paths.posJson.isFile) return LaunchResult.NotProvisioned
        return when (val id = b.deviceIds.read()) {
            is DeviceIdResult.Unavailable -> {
                Timber.w("provisioning: the device serial is unavailable")
                LaunchResult.DeviceIdUnavailable(id.detail)
            }
            is DeviceIdResult.Available ->
                if (id.serial != stored.deviceId) {
                    Timber.w("provisioning: the bundle belongs to another device")
                    LaunchResult.DeviceMismatch(stored = stored.deviceId, actual = id.serial)
                } else {
                    b.paths.dataDir.mkdirs()
                    b.paths.logDir.mkdirs()
                    LaunchResult.Ready(LaunchConfig(b.paths.dataDir, b.paths.posJson, id.serial, b.paths.logDir))
                }
        }
    }
}
