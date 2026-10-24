package dev.nuagemagique.pilipos.i18n

import java.util.Locale

/**
 * The three UI languages of the till, as the Vue UI knows them (`pos.language`).
 *
 * [code] is the persisted value and the resource qualifier; [languageTag] is the BCP 47 tag the UI sets on
 * text runs so that TalkBack and the font fallback pick the simplified Chinese glyphs (Q19).
 */
enum class AppLanguage(
    val code: String,
    val languageTag: String,
) {
    FR("fr", "fr"),
    EN("en", "en"),
    ZH("zh", "zh-Hans"),
    ;

    /** The locale whose resources hold the texts of this language. */
    val locale: Locale get() = Locale.forLanguageTag(code)

    companion object {
        /** The language of [code] (case and surrounding blanks ignored); null, blank or unknown gives [FR]. */
        fun fromCode(code: String?): AppLanguage {
            val wanted = code?.trim()?.lowercase()
            return entries.firstOrNull { it.code == wanted } ?: FR
        }
    }
}
