package dev.nuagemagique.pilipos.ui.feedback

import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.ErrorTexts
import javax.inject.Inject
import javax.inject.Singleton

/** Default lifetime of an error toast, in milliseconds (5000 or 6000 in the Vue callers). */
const val ERROR_TOAST_TTL_MS: Long = 5000L

/**
 * `describeError` plus `toast.show(..., "error")` of the Vue UI: turns a failed engine call into the text of the
 * till and, on request, into an error toast.
 *
 * The language is a parameter (the caller reads it from the settings). Never throws: an unknown failure gives the
 * unexpected-error text of [ErrorTexts].
 */
@Singleton
class ErrorPresenter
    @Inject
    constructor(
        private val toasts: ToastStore,
        private val errors: ErrorTexts,
    ) {
        /** The text of [error]; with a [prefix] the Vue form `prefix : text`. */
        fun text(
            error: Throwable,
            lang: AppLanguage,
            prefix: String? = null,
        ): String = withPrefix(prefix, errors.describe(error, lang))

        /** The text of a raw error string (the `error` of a `*Result` record), prefixed like [text]. */
        fun textRaw(
            raw: String?,
            lang: AppLanguage,
            prefix: String? = null,
        ): String = withPrefix(prefix, errors.describe(raw, lang))

        /** Shows [error] as an error toast of [ttlMs]; returns the toast id. */
        fun show(
            error: Throwable,
            lang: AppLanguage,
            prefix: String? = null,
            ttlMs: Long = ERROR_TOAST_TTL_MS,
        ): Long = toasts.show(text(error, lang, prefix), ToastVariant.Error, ttlMs)

        /** Shows the raw error string [raw] as an error toast of [ttlMs]; returns the toast id. */
        fun showRaw(
            raw: String?,
            lang: AppLanguage,
            prefix: String? = null,
            ttlMs: Long = ERROR_TOAST_TTL_MS,
        ): Long = toasts.show(textRaw(raw, lang, prefix), ToastVariant.Error, ttlMs)

        private fun withPrefix(
            prefix: String?,
            text: String,
        ): String = if (prefix.isNullOrEmpty()) text else "$prefix : $text"
    }
