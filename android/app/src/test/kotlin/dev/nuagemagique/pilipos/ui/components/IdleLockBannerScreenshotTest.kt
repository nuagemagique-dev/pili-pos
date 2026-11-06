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

/** Golden of the idle lock countdown banner (UX audit L4) as it sits under the header. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w960dp-h120dp-mdpi")
class IdleLockBannerScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun thirtySecondsLeft() {
        val catalog = I18nTestSupport.catalog()
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary), contentAlignment = Alignment.TopCenter) {
                        IdleLockBannerContent(30)
                    }
                }
            }
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/components/idle_lock_banner.png")
    }
}
