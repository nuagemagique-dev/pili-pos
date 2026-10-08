package dev.nuagemagique.pilipos.smoke

import uniffi.pos_ffi.EngineOptions
import java.io.File

/** The value of an earlier step, or a failure that names the step it depends on. */
internal fun <T : Any> need(
    value: T?,
    step: String,
): T = checkNotNull(value) { "needs the step $step" }

internal fun engineOptions(
    data: File,
    posJson: File,
) = EngineOptions(
    dataDir = data.absolutePath,
    configBundle = posJson.absolutePath,
    deviceId = SMOKE_DEVICE_ID,
    timers = null,
)

/** Writes the provisioning bundle pos.json (the host mocks as seen through adb reverse). */
internal fun writePosJson(
    dir: File,
    fdmPort: Int,
    printerPort: Int,
): File {
    dir.mkdirs()
    val file = File(dir, "pos.json")
    file.writeText(
        """
        {
          "pos_id": "CPOS0031234567",
          "terminal_id": "POS-2-ANDROID",
          "device_id": "pos-json-device",
          "restaurant_name": "Restaurant Test",
          "restaurant_address": "1 Rue du Test, 4000 Liege",
          "restaurant_vat": "BE0000000097",
          "restaurant_est_no": "2000000042",
          "fdm_url": "http://127.0.0.1:$fdmPort/graphql",
          "printer_address": "127.0.0.1:$printerPort",
          "table_count": 25
        }
        """.trimIndent(),
    )
    return file
}

internal const val SMOKE_DEVICE_ID = "EMULATOR-SMOKE-0001"
