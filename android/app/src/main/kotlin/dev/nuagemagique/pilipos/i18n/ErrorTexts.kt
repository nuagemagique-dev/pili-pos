package dev.nuagemagique.pilipos.i18n

import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.engine.EngineUnavailableException
import uniffi.pos_ffi.InternalException
import uniffi.pos_ffi.PosException
import javax.inject.Inject
import javax.inject.Singleton

/** A backend refusal split back: the [code] and its named [vars] (`pin_locked|seconds=30`). */
data class CodedError(
    val code: String,
    val vars: Map<String, String>,
)

private val CODE = Regex("[a-z][a-z_]*")

/**
 * `parseErrorCode` of `ui/src/utils/errorMessages.ts`: `code|name=value|...` split back, or null when [text] is
 * not a code (the code must be lowercase letters and underscores). Parts without a name are ignored.
 */
fun parseErrorCode(text: String): CodedError? {
    val parts = text.trim().split("|")
    val code = parts.first()
    if (code.isEmpty() || !CODE.matches(code)) return null
    val vars = LinkedHashMap<String, String>()
    for (part in parts.drop(1)) {
        val at = part.indexOf('=')
        if (at > 0) vars[part.substring(0, at)] = part.substring(at + 1)
    }
    return CodedError(code, vars)
}

/**
 * The screen text of an engine refusal, in the language of the till (`describeError` of the Vue UI).
 *
 * Pure presentation: it only looks texts up. A text that starts with `FDM` is an FDM failure (French text, plus
 * the Chinese line in Chinese); a known `err_<code>` gets its template filled from the code's values; anything
 * else is the unexpected-error text with the raw detail.
 */
@Singleton
class ErrorTexts
    @Inject
    constructor(
        private val catalog: MessageCatalog,
        private val fdm: FdmTexts,
    ) {
        /** `describeError`: the text of a raw engine error string ([PosException] detail, `*Result.error`). */
        fun describe(
            raw: String?,
            lang: AppLanguage,
        ): String {
            val text = raw.orEmpty().trim()
            val coded = parseErrorCode(text)
            return when {
                text.isEmpty() -> catalog.text("err_failed", lang)
                isFdm(text) -> describeFdm(text, lang)
                coded != null -> describeCoded(coded.code, coded.vars, text, lang)
                else -> unexpected(text, lang)
            }
        }

        /** The text of a failed engine call, by the kind of [error]. */
        fun describe(
            error: Throwable,
            lang: AppLanguage,
        ): String =
            when (error) {
                is PosException.Coded -> describeCoded(error.code, error.params, rawOf(error), lang)
                is PosException.Fdm -> describeFdm(error.detail.trim(), lang)
                is PosException.Internal -> unexpected(error.detail, lang)
                is PosException.EngineClosed, is EngineUnavailableException ->
                    catalog.text(R.string.engine_unavailable, lang)
                is InternalException -> unexpected(error.message.orEmpty(), lang)
                else -> unexpected(error.javaClass.name, lang)
            }

        // The FFI rule (PD54): a text starting with FDM is an FDM failure. The Vue test is `^FDM\b`, which a code
        // such as FDM_ANSWER_PENDING|label=N does not match (underscore is a word character); the till follows the
        // FFI rule so that those POS check refusals get their message instead of the unexpected-error text.
        private fun isFdm(text: String): Boolean = text.startsWith("FDM")

        private fun describeFdm(
            text: String,
            lang: AppLanguage,
        ): String {
            val shown = fdm.describe(text, lang) ?: return text
            return if (shown.chinese == null) shown.french else shown.french + "\n" + shown.chinese
        }

        private fun describeCoded(
            code: String,
            vars: Map<String, String>,
            raw: String,
            lang: AppLanguage,
        ): String {
            val key = "err_$code"
            return if (catalog.has(key)) catalog.text(key, lang, vars) else unexpected(raw, lang)
        }

        private fun unexpected(
            detail: String,
            lang: AppLanguage,
        ): String = catalog.text("err_unexpected", lang, mapOf("detail" to detail))

        private fun rawOf(error: PosException.Coded): String =
            (listOf(error.code) + error.params.map { (name, value) -> "$name=$value" }).joinToString("|")
    }
