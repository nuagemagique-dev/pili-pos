package dev.nuagemagique.pilipos.platform.about

import org.json.JSONArray
import org.json.JSONObject

/**
 * What identifies this installation (PD209): the build, the hashes of the installed files, the device and
 * the storage. A null value is a field that could not be read (the engine is not ready, the entry is not in
 * the APK); [soProblem] says why the shared library hash is missing.
 *
 * @property applicationId the package name
 * @property posSwVersion the software version the engine reports, null when the engine library is not loaded
 * @property engineRef the engine reference, `pili-pos <version>, <freeze tag> (<short sha>)`
 * @property apkSha1 lowercase hex SHA-1 of the installed `base.apk`
 * @property apkSha256 lowercase hex SHA-256 of the installed `base.apk`
 * @property soSha1 lowercase hex SHA-1 of the `lib/arm64-v8a/libpos_ffi.so` entry of that APK
 * @property soProblem why [soSha1] is null
 * @property signerSha256 lowercase hex SHA-256 of each signing certificate
 * @property deviceId the serial recorded by the provisioning import
 * @property posId the POS id of the settings, null while the engine is not ready
 * @property terminalId the terminal id of the settings, null while the engine is not ready
 * @property freeInternalBytes free bytes of the internal storage the app data lives on
 * @property backupsCount number of `*.db` files in `engine/backups`
 * @property backupsBytes their total size in bytes
 * @property generatedAt ISO 8601 instant in the Europe/Brussels zone
 */
@Suppress("LongParameterList")
data class IdentityReport(
    val applicationId: String,
    val versionName: String,
    val versionCode: Long,
    val posSwVersion: String?,
    val engineRef: String,
    val apkSha1: String?,
    val apkSha256: String?,
    val soSha1: String?,
    val soProblem: String?,
    val signerSha256: List<String>,
    val deviceId: String?,
    val posId: String?,
    val terminalId: String?,
    val androidRelease: String,
    val sdkInt: Int,
    val fingerprint: String,
    val manufacturer: String,
    val model: String,
    val freeInternalBytes: Long,
    val backupsCount: Int,
    val backupsBytes: Long,
    val generatedAt: String,
) {
    /** True when less than [LOW_FREE_BYTES] are free. */
    val lowFreeSpace: Boolean get() = freeInternalBytes < LOW_FREE_BYTES

    /** True when the backups take more than half of the free space. */
    val backupsOverHalfOfFree: Boolean get() = backupsBytes > freeInternalBytes / 2

    /**
     * The report as one JSON object with a stable key order, written by the export as `identity.json`.
     * A missing value is a JSON null, never an absent key.
     */
    fun toJson(): JSONObject =
        JSONObject()
            .put("applicationId", applicationId)
            .put("versionName", versionName)
            .put("versionCode", versionCode)
            .put("posSwVersion", orNull(posSwVersion))
            .put("engineRef", engineRef)
            .put("apkSha1", orNull(apkSha1))
            .put("apkSha256", orNull(apkSha256))
            .put("soSha1", orNull(soSha1))
            .put("soProblem", orNull(soProblem))
            .put("signerSha256", JSONArray(signerSha256))
            .put("deviceId", orNull(deviceId))
            .put("posId", orNull(posId))
            .put("terminalId", orNull(terminalId))
            .put("androidRelease", androidRelease)
            .put("sdkInt", sdkInt)
            .put("fingerprint", fingerprint)
            .put("manufacturer", manufacturer)
            .put("model", model)
            .put("freeInternalBytes", freeInternalBytes)
            .put("backupsCount", backupsCount)
            .put("backupsBytes", backupsBytes)
            .put("generatedAt", generatedAt)

    private fun orNull(value: String?): Any = value ?: JSONObject.NULL

    companion object {
        /** The free space under which the About screen warns: 2 GiB. */
        const val LOW_FREE_BYTES: Long = 2L * 1024L * 1024L * 1024L
    }
}
