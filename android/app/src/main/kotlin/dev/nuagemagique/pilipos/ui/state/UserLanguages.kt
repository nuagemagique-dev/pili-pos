package dev.nuagemagique.pilipos.ui.state

import dev.nuagemagique.pilipos.i18n.AppLanguage

/** The languages in the order of the selectors: "FR · 中文 · EN" (UX audit R5). */
internal val LanguageChoices: List<AppLanguage> = listOf(AppLanguage.FR, AppLanguage.ZH, AppLanguage.EN)

/**
 * The language of the screens for the user [userId] (`languageFor` of `utils/userLanguage.ts`): their own choice,
 * else the terminal language. Nobody logged in (a null or empty id) is the terminal language.
 */
internal fun languageFor(
    choices: Map<String, AppLanguage>,
    userId: String?,
    terminal: AppLanguage,
): AppLanguage = if (userId.isNullOrEmpty()) terminal else choices[userId] ?: terminal
