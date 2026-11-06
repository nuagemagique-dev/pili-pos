package dev.nuagemagique.pilipos.ui.state

import dev.nuagemagique.pilipos.i18n.AppLanguage
import org.junit.Assert.assertEquals
import org.junit.Test

class UserLanguagesTest {
    private val choices = mapOf("111" to AppLanguage.ZH, "222" to AppLanguage.EN)

    @Test
    fun aUserGetsTheirOwnChoice() {
        assertEquals(AppLanguage.ZH, languageFor(choices, "111", AppLanguage.FR))
        assertEquals(AppLanguage.EN, languageFor(choices, "222", AppLanguage.FR))
    }

    @Test
    fun aUserWithoutAChoiceFollowsTheTerminal() {
        assertEquals(AppLanguage.EN, languageFor(choices, "333", AppLanguage.EN))
    }

    @Test
    fun nobodyLoggedInIsTheTerminalLanguage() {
        assertEquals(AppLanguage.FR, languageFor(choices, null, AppLanguage.FR))
        assertEquals(AppLanguage.ZH, languageFor(choices, "", AppLanguage.ZH))
    }

    @Test
    fun theSelectorsListFrZhEnInThatOrder() {
        assertEquals(listOf("fr", "zh", "en"), LanguageChoices.map { it.code })
    }
}
