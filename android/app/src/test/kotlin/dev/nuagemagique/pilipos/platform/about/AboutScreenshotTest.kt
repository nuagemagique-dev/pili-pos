package dev.nuagemagique.pilipos.platform.about

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.ui.theme.PiliDensity
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** Golden of the About screen at the design width (960 px window, density 0.5 = the 1920 px till). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w960dp-h540dp-mdpi")
class AboutScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun aboutScreen() {
        rule.setContent { PiliTheme { PiliDensity { AboutContent(AboutUiState.Loaded(sampleReport()), onRefresh = {}) } } }
        rule.onRoot().captureRoboImage("src/test/screenshots/platform/about.png")
    }
}
