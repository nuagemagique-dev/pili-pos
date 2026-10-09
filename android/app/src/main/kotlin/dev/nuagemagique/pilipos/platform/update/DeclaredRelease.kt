package dev.nuagemagique.pilipos.platform.update

import org.json.JSONException
import org.json.JSONObject

/**
 * One declared Android release (PD211): the content of `declared.json`.
 *
 * `{"format":1,"platform":"android","package":"dev.nuagemagique.pilipos","version":"X.Y.Z","versionCode":N,
 * "asset":"pili-pos-android-X.Y.Z.apk","sha1":"..","sha256":"..","signerSha256":"..","declared_at":".."}`
 */
data class DeclaredRelease(
    val packageName: String,
    val version: String,
    val versionCode: Long,
    val asset: String,
    val sha1: String,
    val sha256: String,
    val signerSha256: String,
    val declaredAt: String,
)

/**
 * Strict reader of `declared.json`. Anything that is not exactly a version 1 Android descriptor is refused
 * with [UpdateException]: an unknown `format`, a platform other than `android`, a missing or mistyped field,
 * a hash that is not lowercase hex of the right length, an asset name outside `pili-pos-android-X.Y.Z.apk`
 * or one that does not belong to `version`, or a document above [MAX_BYTES]. Unknown extra keys are ignored
 * (the whole document is signed).
 */
object DeclaredParser {
    /** Size limit of the descriptor: 64 KiB. */
    const val MAX_BYTES: Int = 64 * 1024

    /** The only `format` understood. */
    const val FORMAT: Int = 1

    /** The only `platform` understood. */
    const val PLATFORM: String = "android"

    private val ASSET = Regex("^pili-pos-android-[0-9]+\\.[0-9]+\\.[0-9]+\\.apk$")
    private val VERSION = Regex("^[0-9]+\\.[0-9]+\\.[0-9]+$")
    private val SHA1 = Regex("^[0-9a-f]{40}$")
    private val SHA256 = Regex("^[0-9a-f]{64}$")

    /** Parses [bytes]; throws [UpdateException] (`bad_descriptor` or `too_large`). */
    fun parse(bytes: ByteArray): DeclaredRelease {
        refuseUnless(bytes.size <= MAX_BYTES, UpdateRefusalCode.TOO_LARGE) { "declared.json is ${bytes.size} bytes" }
        val json =
            try {
                JSONObject(String(bytes, Charsets.UTF_8))
            } catch (e: JSONException) {
                refuse(UpdateRefusalCode.BAD_DESCRIPTOR, "not a JSON object", e)
            }
        val format = json.opt("format")
        bad(format is Int && format == FORMAT, "unknown format")
        bad(json.opt("platform") == PLATFORM, "platform is not android")
        val version = text(json, "version")
        bad(VERSION.matches(version), "version is not X.Y.Z")
        val asset = text(json, "asset")
        bad(ASSET.matches(asset), "asset name is not pili-pos-android-X.Y.Z.apk")
        bad(asset == "pili-pos-android-$version.apk", "asset does not belong to version $version")
        return DeclaredRelease(
            packageName = text(json, "package"),
            version = version,
            versionCode = versionCode(json),
            asset = asset,
            sha1 = hex(json, "sha1", SHA1),
            sha256 = hex(json, "sha256", SHA256),
            signerSha256 = hex(json, "signerSha256", SHA256),
            declaredAt = text(json, "declared_at"),
        )
    }

    private fun versionCode(json: JSONObject): Long {
        val value = json.opt("versionCode")
        val code =
            when (value) {
                is Int -> value.toLong()
                is Long -> value
                else -> 0L
            }
        bad(code > 0L, "versionCode is missing, not an integer or not positive")
        return code
    }

    private fun text(
        json: JSONObject,
        key: String,
    ): String {
        val value = json.opt(key)
        bad(value is String && value.isNotBlank(), "$key is missing or empty")
        return value as String
    }

    private fun hex(
        json: JSONObject,
        key: String,
        pattern: Regex,
    ): String {
        val value = text(json, key)
        bad(pattern.matches(value), "$key is not lowercase hex of the right length")
        return value
    }

    private fun bad(
        ok: Boolean,
        detail: String,
    ) = refuseUnless(ok, UpdateRefusalCode.BAD_DESCRIPTOR) { detail }
}
