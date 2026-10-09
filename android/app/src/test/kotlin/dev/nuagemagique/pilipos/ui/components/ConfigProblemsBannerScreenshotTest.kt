package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Alignment
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

/** Golden of the config problems banner along the bottom of a screen. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w960dp-h300dp-mdpi")
class ConfigProblemsBannerScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun twoProblems() {
        val catalog = I18nTestSupport.catalog()
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary), contentAlignment = Alignment.BottomCenter) {
                        ConfigProblemsBanner(listOf("fdm_url: empty", "pos_id: longer than 14 characters"))
                    }
                }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/config_problems.png")
    }
}
