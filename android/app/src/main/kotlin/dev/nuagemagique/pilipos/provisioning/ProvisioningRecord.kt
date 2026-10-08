package dev.nuagemagique.pilipos.provisioning

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import org.json.JSONException
import org.json.JSONObject
import timber.log.Timber
import java.io.File
import java.io.FileOutputStream
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import javax.inject.Inject
import javax.inject.Singleton

/**
 * What the last successful import recorded.
 *
 * @property deviceId the serial of the device the bundle was imported on (the engine deviceId)
 * @property manifestSha256 lowercase hex SHA-256 of the exact bytes of `MANIFEST.json`
 * @property trustedComment the signed trusted comment of the manifest
 * @property importedAt ISO-8601 UTC instant of the import
 */
data class ProvisioningRecord(
    val deviceId: String,
    val manifestSha256: String,
    val trustedComment: String,
    val importedAt: String,
)

/** Reads and writes `provisioning/record.json` and publishes the current record. */
@Singleton
class ProvisioningRecordStore
    @Inject
    constructor(
        private val paths: ProvisioningPaths,
    ) {
        private val current = MutableStateFlow<ProvisioningRecord?>(null)

        /** The record last read or written; null until the first [load] or [write], or when none exists. */
        val state: StateFlow<ProvisioningRecord?> get() = current

        /** Reads the record file; a missing or corrupt file reads as absent (the corrupt case is logged). */
        fun load(): ProvisioningRecord? {
            val record = parse(paths.recordFile)
            current.value = record
            return record
        }

        /** Writes [record] atomically (temporary file, then rename) and publishes it. */
        fun write(record: ProvisioningRecord) {
            val target = paths.recordFile
            target.parentFile?.mkdirs()
            val temp = File(target.parentFile, "${target.name}.tmp")
            val json =
                JSONObject()
                    .put("format", FORMAT)
                    .put("deviceId", record.deviceId)
                    .put("manifestSha256", record.manifestSha256)
                    .put("trustedComment", record.trustedComment)
                    .put("importedAt", record.importedAt)
            FileOutputStream(temp).use { out ->
                out.write(json.toString().toByteArray(Charsets.UTF_8))
                out.fd.sync()
            }
            Files.move(temp.toPath(), target.toPath(), StandardCopyOption.ATOMIC_MOVE)
            current.value = record
        }

        private fun parse(file: File): ProvisioningRecord? {
            if (!file.isFile) return null
            return try {
                val json = JSONObject(file.readText(Charsets.UTF_8))
                require(json.optInt("format", 0) == FORMAT) { "unknown record format" }
                ProvisioningRecord(
                    deviceId = json.getString("deviceId").also { require(it.isNotEmpty()) { "empty deviceId" } },
                    manifestSha256 = json.getString("manifestSha256"),
                    trustedComment = json.getString("trustedComment"),
                    importedAt = json.getString("importedAt"),
                )
            } catch (e: JSONException) {
                Timber.w("provisioning: record.json is corrupt (%s), read as absent", e.javaClass.simpleName)
                null
            } catch (e: IllegalArgumentException) {
                Timber.w("provisioning: record.json is invalid (%s), read as absent", e.message)
                null
            } catch (e: java.io.IOException) {
                Timber.w("provisioning: record.json is unreadable (%s), read as absent", e.javaClass.simpleName)
                null
            }
        }

        private companion object {
            const val FORMAT = 1
        }
    }
