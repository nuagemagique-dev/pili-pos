package dev.nuagemagique.pilipos.soak

import android.util.Log
import org.json.JSONObject
import java.io.File
import java.io.IOException

/**
 * Where the driver reports: one logcat line per event (tag [TAG]) and one JSON line per event appended to
 * `<externalFilesDir>/soak/driver.jsonl`, which the host sampler reads (the log ring of the device is too small to
 * carry a 24 hour run). Every JSON line has a millisecond epoch `ts` and a `type`.
 */
internal class SoakLog(private val dir: File) {
    private val file = File(dir, FILE_NAME)

    init {
        if (!dir.isDirectory && !dir.mkdirs()) throw AssertionError("cannot create ${dir.path}")
    }

    /** The stop file: the host creates it to end the run cleanly. */
    val stopFile: File get() = File(dir, "stop")

    /** Logs [text] to logcat only. */
    fun line(text: String) {
        Log.i(TAG, text)
    }

    /** Appends one JSON line of [type] with [fields], and logs [text] (the `SOAK_*` line) when given. */
    @Synchronized
    fun event(
        type: String,
        text: String?,
        fields: Map<String, Any?> = emptyMap(),
    ) {
        if (text != null) line(text)
        val json = JSONObject()
        json.put("ts", System.currentTimeMillis())
        json.put("type", type)
        for ((key, value) in fields) json.put(key, value ?: JSONObject.NULL)
        try {
            file.appendText(json.toString() + "\n")
        } catch (e: IOException) {
            Log.w(TAG, "driver.jsonl not written: ${e.message}")
        }
    }

    companion object {
        /** The logcat tag of every soak line. */
        const val TAG = "PiliSoak"

        /** The JSON lines file. */
        const val FILE_NAME = "driver.jsonl"
    }
}
