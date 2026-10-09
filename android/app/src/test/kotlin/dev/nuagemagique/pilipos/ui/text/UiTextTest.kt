package dev.nuagemagique.pilipos.ui.text

import androidx.activity.ComponentActivity
import androidx.compose.material3.LocalTextStyle
import androidx.compose.runtime.Composable
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class UiTextTest {
    @get:Rule
    val compose = createAndroidComposeRule<ComponentActivity>()

    private val catalog = I18nTestSupport.catalog()

    private fun <T> read(
        language: AppLanguage,
        block: @Composable () -> T,
    ): T {
        var out: T? = null
        compose.setContent { ProvideUiText(language, catalog) { out = block() } }
        compose.waitForIdle()
        @Suppress("UNCHECKED_CAST")
        return out as T
    }

    @Test
    fun vueTextFollowsTheAppLanguage() {
        assertEquals("Sortir", read(AppLanguage.FR) { vueText("logout") })
    }

    @Test
    fun vueTextInEnglish() {
        assertEquals("Logout", read(AppLanguage.EN) { vueText("logout") })
    }

    @Test
    fun vueTextInChinese() {
        assertEquals("退出", read(AppLanguage.ZH) { vueText("logout") })
    }

    @Test
    fun vueTextFillsThePlaceholders() {
        val text = read(AppLanguage.EN) { vueText("support_banner", "name" to "Alice") }
        assertTrue(text, text.contains("Alice"))
        assertTrue(text, !text.contains("{name}"))
    }

    @Test
    fun anUnknownKeyGivesTheKey() {
        assertEquals("no_such_key", read(AppLanguage.FR) { vueText("no_such_key") })
    }

    @Test
    fun uiTextReadsTheHandWrittenStringsInFrench() {
        assertEquals("La caisse démarre, veuillez patienter.", read(AppLanguage.FR) { uiText(R.string.engine_unavailable) })
    }

    @Test
    fun uiTextReadsTheHandWrittenStringsInChinese() {
        assertEquals("收银机正在启动，请稍候。", read(AppLanguage.ZH) { uiText(R.string.engine_unavailable) })
    }

    @Test
    fun uiTextFillsThePlaceholders() {
        assertEquals("Version 1.2", read(AppLanguage.EN) { uiText(R.string.shell_version, "version" to "1.2") })
    }

    @Test
    fun theTextRunsCarryTheHanLanguageTag() {
        assertEquals(listOf("zh-Hans"), read(AppLanguage.ZH) { LocalTextStyle.current.localeList!!.map { it.toLanguageTag() } })
    }

    @Test
    fun theTextRunsCarryTheFrenchLanguageTag() {
        assertEquals(listOf("fr"), read(AppLanguage.FR) { LocalTextStyle.current.localeList!!.map { it.toLanguageTag() } })
    }

    @Test
    fun theLanguageLocalIsProvided() {
        assertEquals(AppLanguage.EN, read(AppLanguage.EN) { LocalUiLanguage.current })
    }

    @Test
    fun withoutAProviderTheLanguageIsFrenchAndTheCatalogFails() {
        var language: AppLanguage? = null
        var failure: Throwable? = null
        compose.setContent {
            language = LocalUiLanguage.current
            failure = runCatching { LocalMessageCatalog.current }.exceptionOrNull()
        }
        compose.waitForIdle()
        assertEquals(AppLanguage.FR, language)
        assertTrue(failure is IllegalStateException)
    }
}
