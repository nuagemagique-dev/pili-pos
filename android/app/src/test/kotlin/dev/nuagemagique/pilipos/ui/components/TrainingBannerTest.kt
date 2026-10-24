package dev.nuagemagique.pilipos.ui.components

import androidx.compose.ui.test.assertHeightIsEqualTo
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class TrainingBannerTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    private fun show(language: AppLanguage) {
        compose.setContent { PiliTheme(training = true) { ProvideUiText(language, catalog) { TrainingBanner() } } }
    }

    @Test
    fun showsTheVueTextInFrench() {
        show(AppLanguage.FR)
        compose.onNodeWithTag("training_banner").assertIsDisplayed()
        compose.onNodeWithText(catalog.text("training_banner", AppLanguage.FR)).assertIsDisplayed()
    }

    @Test
    fun followsTheAppLanguage() {
        show(AppLanguage.EN)
        compose.onNodeWithText(catalog.text("training_banner", AppLanguage.EN)).assertIsDisplayed()
    }

    @Test
    fun theBandIsFortyEightDpHigh() {
        show(AppLanguage.FR)
        compose.onNodeWithTag("training_banner").assertHeightIsEqualTo(48.dp)
    }
}
