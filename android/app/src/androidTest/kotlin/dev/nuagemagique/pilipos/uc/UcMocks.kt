package dev.nuagemagique.pilipos.uc

import org.json.JSONObject
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL

/**
 * The mock commands of the UC vocabulary (`fdm_control`, `fdm_control_reset`, `printer_control`, `sleep_ms`).
 * They never reach the engine: they talk to the host mocks through the adb reverse ports.
 *
 * @param fdmPort the device port of `pos-fdm-mock` (default 19380)
 * @param printerHttpPort the device port of the printer mock's HTTP side (default 19391)
 */
internal class UcMocks(
    private val fdmPort: Int,
    private val printerHttpPort: Int,
) {
    /** POSTs [args] (the fields of the mock's `Control` struct) to `pos-fdm-mock`; it replaces every setting. */
    fun fdmControl(args: JSONObject) {
        post("http://127.0.0.1:$fdmPort/mock/control", args)
    }

    /** Back to the defaults: the control struct with every field absent. */
    fun fdmControlReset() {
        post("http://127.0.0.1:$fdmPort/mock/control", JSONObject())
    }

    /** POSTs [args] to the control endpoint of the printer mock. */
    fun printerControl(args: JSONObject) {
        post("http://127.0.0.1:$printerHttpPort/control", args)
    }

    /** Waits [ms] milliseconds (at most [MAX_SLEEP_MS]). */
    fun sleep(ms: Long) {
        if (ms < 0 || ms > MAX_SLEEP_MS) throw UcStepException("sleep_ms: $ms is not between 0 and $MAX_SLEEP_MS")
        Thread.sleep(ms)
    }

    private fun post(
        url: String,
        body: JSONObject,
    ) {
        val connection = URL(url).openConnection() as HttpURLConnection
        try {
            connection.requestMethod = "POST"
            connection.connectTimeout = TIMEOUT_MS
            connection.readTimeout = TIMEOUT_MS
            connection.doOutput = true
            connection.setRequestProperty("Content-Type", "application/json")
            connection.outputStream.use { it.write(body.toString().toByteArray(Charsets.UTF_8)) }
            val code = connection.responseCode
            if (code !in HTTP_OK_RANGE) throw UcStepException("POST $url answered HTTP $code")
            connection.inputStream.use { it.readBytes() }
        } catch (e: IOException) {
            throw UcStepException("POST $url failed: ${e.message}", e)
        } finally {
            connection.disconnect()
        }
    }

    private companion object {
        const val TIMEOUT_MS = 5_000
        const val MAX_SLEEP_MS = 10_000L
        val HTTP_OK_RANGE = 200..299
    }
}
