package dev.nuagemagique.pilipos.provisioning

import android.content.res.Resources
import androidx.annotation.StringRes
import dev.nuagemagique.pilipos.R

/**
 * A reason an import or a launch was refused. [code] is the suffix of the string resource
 * `prov_err_<code>`; [params] fill the `{name}` placeholders of that text.
 *
 * The engine reports bundle problems as `code` or `code|k=v|k2=v2` (FFI.md section 9); [parse] turns such
 * a string into a problem. The app adds the codes of the failures it detects itself.
 */
data class ProvisioningProblem(
    val code: String,
    val params: Map<String, String>,
    @StringRes val textRes: Int,
) {
    /** The text in the language of [resources], placeholders replaced (never `getString` format arguments). */
    fun render(resources: Resources): String {
        val shown = if (code == CONFIG_INVALID) params + ("detail" to configLines(resources)) else params
        return shown.entries.fold(resources.getString(textRes)) { text, (key, value) -> text.replace("{$key}", value) }
    }

    /**
     * The problems of `pos.json`, one per line: the engine reports each as `config_<setting>|detail=<English text>`
     * (the `detail` param holds them, one per line). The setting is said in the language of [resources] (the
     * `err_config_*` texts of the Vue dictionary), the English text stays as its detail; a line that is not a
     * known code is shown as it is.
     */
    private fun configLines(resources: Resources): String =
        params["detail"]
            .orEmpty()
            .lines()
            .filter { it.isNotBlank() }
            .joinToString("\n") { "• " + describeConfigProblem(resources, it) }

    companion object {
        /** Every code that has a text, engine codes first. */
        val ALL_CODES: List<String> get() = TEXTS.keys.toList()

        private val TEXTS: Map<String, Int> =
            linkedMapOf(
                "bundle_signature_invalid" to R.string.prov_err_bundle_signature_invalid,
                "bundle_manifest_invalid" to R.string.prov_err_bundle_manifest_invalid,
                "bundle_path_invalid" to R.string.prov_err_bundle_path_invalid,
                "bundle_file_missing" to R.string.prov_err_bundle_file_missing,
                "bundle_size_mismatch" to R.string.prov_err_bundle_size_mismatch,
                "bundle_hash_mismatch" to R.string.prov_err_bundle_hash_mismatch,
                "bundle_unlisted_file" to R.string.prov_err_bundle_unlisted_file,
                "minisign_key_invalid" to R.string.prov_err_minisign_key_invalid,
                "minisign_signature_invalid" to R.string.prov_err_minisign_signature_invalid,
                "key_missing" to R.string.prov_err_key_missing,
                "not_device_owner" to R.string.prov_err_not_device_owner,
                "device_id_unavailable" to R.string.prov_err_device_id_unavailable,
                "config_invalid" to R.string.prov_err_config_invalid,
                "engine_open" to R.string.prov_err_engine_open,
                "copy_failed" to R.string.prov_err_copy_failed,
                "too_large" to R.string.prov_err_too_large,
                "symlink" to R.string.prov_err_symlink,
                "busy" to R.string.prov_err_busy,
                "unexpected" to R.string.prov_err_unexpected,
            )

        /** A problem of a known [code]; an unknown code becomes `unexpected` carrying the code as detail. */
        fun of(
            code: String,
            vararg params: Pair<String, String>,
        ): ProvisioningProblem {
            val res = TEXTS[code] ?: return unexpected(code)
            return ProvisioningProblem(code, params.toMap(), res)
        }

        /** The catch-all problem, [detail] shown to the technician. */
        fun unexpected(detail: String): ProvisioningProblem = of("unexpected", "detail" to detail.take(MAX_DETAIL))

        /** Parses the engine form `code|k=v|k2=v2`. */
        fun parse(raw: String): ProvisioningProblem {
            val parts = raw.trim().split('|')
            val code = parts.first()
            if (code !in TEXTS) return unexpected(raw)
            val params =
                parts.drop(1).mapNotNull { part ->
                    val at = part.indexOf('=')
                    if (at <= 0) null else part.substring(0, at) to part.substring(at + 1)
                }
            return ProvisioningProblem(code, params.toMap(), requireNotNull(TEXTS[code]))
        }

        private const val MAX_DETAIL = 400
        private const val CONFIG_INVALID = "config_invalid"

        // The code names the string (`err_config_*`) only at run time: the lookup by name is the point here.
        @Suppress("DiscouragedApi")
        private fun describeConfigProblem(
            resources: Resources,
            line: String,
        ): String {
            val code = line.substringBefore('|')
            val detail = line.substringAfter("|detail=", missingDelimiterValue = "")
            val id = resources.getIdentifier("err_$code", "string", resources.getResourcePackageName(R.string.prov_title))
            if (!code.startsWith("config_") || id == 0) return line
            return resources.getString(id).replace("{detail}", detail)
        }
    }
}
