package dev.nuagemagique.pilipos.platform.update

import org.json.JSONException
import org.json.JSONObject
import timber.log.Timber
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.nio.file.Files
import java.nio.file.StandardCopyOption

/**
 * The note an update leaves before the install starts (PD215): `filesDir/platform/update/pending.json`. The
 * new process reads it to write the "done" audit row ([UpdateCompletionHook]).
 */
data class UpdateMarker(
    val fromVersionCode: Long,
    val toVersionCode: Long,
    val version: String,
    val sha1: String,
    /** Epoch milliseconds of the start. */
    val startedAt: Long,
)

/** The marker as audit detail (the `started` row). */
fun UpdateMarker.auditDetail(): Map<String, String> =
    mapOf("from" to fromVersionCode.toString(), "to" to toVersionCode.toString(), "version" to version, "sha1" to sha1)

/** What [UpdateMarkerStore.read] found. */
sealed interface MarkerRead {
    /** There is no marker. */
    data object Missing : MarkerRead

    /** The marker exists but is not what [UpdateMarkerStore.write] wrote. */
    data class Corrupt(
        val reason: String,
    ) : MarkerRead

    /** A valid marker. */
    data class Found(
        val marker: UpdateMarker,
    ) : MarkerRead
}

/**
 * Reads, writes and deletes the marker in [dir]. [write] is durable: the bytes go to a temporary file that is
 * fsynced, then renamed over `pending.json`, so that a reader never sees half a marker.
 */
class UpdateMarkerStore(
    private val dir: File,
) {
    private val file: File get() = File(dir, FILE_NAME)

    /** Writes [marker] and syncs it to disk; throws [IOException] when that fails. */
    fun write(marker: UpdateMarker) {
        dir.mkdirs()
        val json =
            JSONObject()
                .put("fromVersionCode", marker.fromVersionCode)
                .put("toVersionCode", marker.toVersionCode)
                .put("version", marker.version)
                .put("sha1", marker.sha1)
                .put("startedAt", marker.startedAt)
        val temp = File(dir, "$FILE_NAME.tmp")
        FileOutputStream(temp).use { out ->
            out.write(json.toString().toByteArray(Charsets.UTF_8))
            out.flush()
            out.fd.sync()
        }
        Files.move(temp.toPath(), file.toPath(), StandardCopyOption.REPLACE_EXISTING)
    }

    /** The marker, [MarkerRead.Missing] when there is none. */
    fun read(): MarkerRead {
        if (!file.isFile) return MarkerRead.Missing
        return try {
            val json = JSONObject(file.readText(Charsets.UTF_8))
            MarkerRead.Found(
                UpdateMarker(
                    fromVersionCode = json.getLong("fromVersionCode"),
                    toVersionCode = json.getLong("toVersionCode"),
                    version = json.getString("version"),
                    sha1 = json.getString("sha1"),
                    startedAt = json.getLong("startedAt"),
                ),
            )
        } catch (e: JSONException) {
            MarkerRead.Corrupt("not a marker: ${e.message}")
        } catch (e: IOException) {
            MarkerRead.Corrupt("unreadable: ${e.message}")
        }
    }

    /** Deletes the marker (and a leftover temporary file); never throws. */
    fun delete() {
        try {
            File(dir, "$FILE_NAME.tmp").delete()
            if (file.exists() && !file.delete()) Timber.w("UpdateMarkerStore: pending.json could not be deleted")
        } catch (
            @Suppress("TooGenericExceptionCaught") e: SecurityException,
        ) {
            Timber.w(e, "UpdateMarkerStore: delete failed")
        }
    }

    /** True when a marker file exists. */
    fun exists(): Boolean = file.exists()

    companion object {
        /** Name of the marker file. */
        const val FILE_NAME = "pending.json"
    }
}
