package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** Golden of the training banner in the violet training palette. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w800dp-h120dp-mdpi")
class TrainingBannerScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun banner() {
        val catalog = I18nTestSupport.catalog()
        rule.setContent {
            PiliTheme(training = true) {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Column(Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary)) { TrainingBanner() }
                }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/training_banner.png")
    }
}
