package dev.nuagemagique.pilipos.nightly

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** Goldens of the nightly export screen (French, the default language of the till). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "fr-w960dp-h540dp-mdpi")
class NightlyScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private fun shoot(
        state: NightlyUiState,
        file: String,
    ) {
        rule.setContent { PiliTheme { NightlyContent(state, onRetry = {}, onBack = {}) } }
        rule.onRoot().captureRoboImage("src/test/screenshots/nightly/$file")
    }

    @Test
    fun upToDate() = shoot(NightlyUiState.Loaded(exportStatus()), "nightly_ok.png")

    @Test
    fun pendingWithAnError() = shoot(NightlyUiState.Loaded(pendingStatus()), "nightly_pending.png")

    @Test
    fun disabled() = shoot(NightlyUiState.Loaded(disabledStatus()), "nightly_disabled.png")
}
