package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.posSettings
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.PosSettings

@RunWith(RobolectricTestRunner::class)
class ConfigProblemsBannerTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val problems = listOf("fdm_url: empty", "pos_id: too long")

    private val settingsFlow = MutableStateFlow<PosSettings?>(null)
    private val languageFlow = MutableStateFlow(AppLanguage.FR)
    private val store =
        mockk<PosSettingsStore> {
            every { settings } returns settingsFlow
            every { language } returns languageFlow
        }

    @Test
    fun noProblemDrawsNothing() {
        compose.setContent { PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { ConfigProblemsBanner(emptyList()) } } }
        compose.onNodeWithTag("config_problems").assertDoesNotExist()
    }

    @Test
    fun theHeadlineAndEveryProblemAreShown() {
        compose.setContent { PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { ConfigProblemsBanner(problems) } } }
        compose.onNodeWithTag("config_problems").assertIsDisplayed()
        compose.onNodeWithText(catalog.text("config_problems", AppLanguage.FR)).assertIsDisplayed()
        compose.onNodeWithTag("config_problem_0").assertIsDisplayed()
        compose.onNodeWithTag("config_problem_1").assertIsDisplayed()
        compose.onNodeWithText("• fdm_url: empty").assertIsDisplayed()
    }

    @Test
    fun theOverlayOrderIsSixty() {
        assertEquals(60, ConfigProblemsOverlay(store, catalog).order)
    }

    @Test
    fun theOverlayShowsNothingWhileSettingsAreMissingOrClean() {
        val overlay = ConfigProblemsOverlay(store, catalog)
        compose.setContent { PiliTheme { overlay.Content() } }
        compose.onNodeWithTag("config_problems").assertDoesNotExist()

        settingsFlow.value = posSettings()
        compose.waitForIdle()
        compose.onNodeWithTag("config_problems").assertDoesNotExist()
    }

    @Test
    fun theOverlayFollowsTheSettingsAndTheLanguageWithoutAnExternalProvider() {
        val overlay = ConfigProblemsOverlay(store, catalog)
        compose.setContent { PiliTheme { overlay.Content() } }

        settingsFlow.value = posSettings().copy(configProblems = problems)
        compose.waitForIdle()
        compose.onNodeWithTag("config_problems").assertIsDisplayed()
        compose.onNodeWithText(catalog.text("config_problems", AppLanguage.FR)).assertIsDisplayed()

        languageFlow.value = AppLanguage.EN
        compose.waitForIdle()
        compose.onNodeWithText(catalog.text("config_problems", AppLanguage.EN)).assertIsDisplayed()

        settingsFlow.value = posSettings()
        compose.waitForIdle()
        compose.onNodeWithTag("config_problems").assertDoesNotExist()
    }

    @Test
    fun aButtonAboveTheBannerStaysClickable() {
        var clicks by mutableIntStateOf(0)
        settingsFlow.value = posSettings().copy(configProblems = problems)
        val overlay = ConfigProblemsOverlay(store, catalog)
        compose.setContent {
            PiliTheme {
                Box(Modifier.fillMaxSize()) {
                    Button(onClick = { clicks++ }, modifier = Modifier.testTag("under")) { Text("under") }
                    overlay.Content()
                }
            }
        }
        compose.onNodeWithTag("under").performClick()
        assertEquals(1, clicks)
    }
}
