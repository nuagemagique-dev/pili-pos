package dev.nuagemagique.pilipos.provisioning

import android.annotation.SuppressLint
import android.os.Build
import timber.log.Timber
import javax.inject.Inject

/** Outcome of reading the hardware identifier of the device. */
sealed interface DeviceIdResult {
    /** The trimmed hardware serial, used as the deviceId (1 to 600 characters). */
    data class Available(
        val serial: String,
    ) : DeviceIdResult

    /** No usable serial; [detail] says why (never a secret). */
    data class Unavailable(
        val detail: String,
    ) : DeviceIdResult
}

/**
 * Reads the Sunmi hardware serial through `Build.getSerial()`, which works only for the device owner with
 * READ_PHONE_STATE granted. There is no fallback: no `ro.serialno`, no ANDROID_ID, no MAC address and no
 * generated identifier (DECISIONS section 3, Q13). Without a serial the till does not start.
 */
class DeviceIdReader internal constructor(
    private val serialSource: () -> String?,
) {
    /** Production reader: the platform serial. */
    @Inject
    @SuppressLint("HardwareIds")
    constructor() : this({ Build.getSerial() })

    /** Reads the serial now; never throws. */
    fun read(): DeviceIdResult {
        val raw =
            try {
                serialSource()
            } catch (e: SecurityException) {
                Timber.w("provisioning: the serial is not readable (SecurityException)")
                return DeviceIdResult.Unavailable("permission denied: ${e.message.orEmpty()}".trim())
            }
        val serial = raw?.trim().orEmpty()
        return when {
            serial.isEmpty() -> DeviceIdResult.Unavailable("the serial is empty")
            serial.equals("unknown", ignoreCase = true) -> DeviceIdResult.Unavailable("the serial is unknown")
            serial.length > MAX_LENGTH -> DeviceIdResult.Unavailable("the serial is longer than $MAX_LENGTH characters")
            else -> DeviceIdResult.Available(serial)
        }
    }

    private companion object {
        const val MAX_LENGTH = 600
    }
}
